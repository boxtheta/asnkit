// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Checks a value against the constraints of its type: the ranges of
//! sizes and numbers, the permitted alphabets of strings, and WITH
//! COMPONENT and WITH COMPONENTS (X.680 51.8).
//!
//! A constraint with an extension marker is not checked.

use alloc::string::ToString;
use alloc::vec::Vec;

use crate::types::{Checked, Inner, Kind, Part, Range, StrKind, TypeId, Types};
use crate::value::Value;
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn fail<T>(what: &'static str) -> Result<T> {
    Err(Error::constraints(what))
}

/// If a value is in one of the values or ranges of the root of a
/// constraint.
fn in_range(range: Option<Range>, parts: &Option<Vec<Part>>, value: i128) -> bool {
    let Some(range) = range else {
        return true;
    };

    // The extension marker means it is not known what is in range.
    if range.extensible {
        return true;
    }

    match parts {
        Some(parts) => parts.iter().any(|(lower, upper)| {
            lower.is_none_or(|lower| value >= lower) && upper.is_none_or(|upper| value <= upper)
        }),
        None => {
            range.min.is_none_or(|min| value >= min) && range.max.is_none_or(|max| value <= max)
        }
    }
}

/// If an integer of more than 128 bits, in two's complement, is in one
/// of the values or ranges of the root of a constraint. It is beyond any
/// bound on its side, which is no more than 128 bits, and within any on
/// the other.
fn big_in_range(range: Option<Range>, parts: &Option<Vec<Part>>, bytes: &[u8]) -> bool {
    let Some(range) = range else {
        return true;
    };

    if range.extensible {
        return true;
    }

    let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);
    let unbounded = |lower: Option<i128>, upper: Option<i128>| match negative {
        true => lower.is_none(),
        false => upper.is_none(),
    };

    match parts {
        Some(parts) => parts.iter().any(|(lower, upper)| unbounded(*lower, *upper)),
        None => unbounded(range.min, range.max),
    }
}

fn in_default_alphabet(kind: StrKind, character: char) -> bool {
    let code = u32::from(character);

    match kind {
        StrKind::Numeric => code == 32 || (48..=57).contains(&code),
        StrKind::Printable => {
            character.is_ascii_alphanumeric() || " '()+,-./:=?".contains(character)
        }
        StrKind::Ia5 => code < 128,
        StrKind::Bmp => code < 0x10000 && !(0xd800..=0xdfff).contains(&code),
        StrKind::Visible => (32..127).contains(&code),
        _ => true,
    }
}

/// Checks `value` as the type `id`.
pub fn check(types: &Types, id: TypeId, value: &Value) -> Result<()> {
    check_at(types, id, value, 0)
}

fn check_at(types: &Types, id: TypeId, value: &Value, depth: usize) -> Result<()> {
    if depth > MAX_DEPTH {
        return fail("The value is nested too deeply.");
    }

    check_kind(types, id, value, depth)?;

    let node = types.node(id);

    match node
        .effective_inner
        .and_then(|holder| types.node(holder).inner.as_ref())
    {
        Some(inner) if !satisfies_inner(types, node.concrete, inner, value, depth + 1)? => {
            match inner {
                Inner::Element(_) => fail("Expected elements that satisfy WITH COMPONENT."),
                _ => fail("Expected a value that satisfies WITH COMPONENTS."),
            }
        }
        _ => Ok(()),
    }
}

/// Checks `value` as the type `id` but for its inner type constraint.
fn check_kind(types: &Types, id: TypeId, value: &Value, depth: usize) -> Result<()> {
    let node = types.node(id);
    let kind = &types.node(node.concrete).kind;

    match (kind, value) {
        (Kind::Integer(named), value) => {
            #[cfg(feature = "bigint")]
            if let Some(range) = &node.effective_big_range {
                let number = match value {
                    Value::Text(name) => {
                        match named.iter().find(|(candidate, _)| candidate == name) {
                            Some((_, number)) => num_bigint::BigInt::from(*number),
                            None => {
                                return Err(Error::encode(
                                    "Expected an integer or a named number.",
                                ));
                            }
                        }
                    }
                    other => match crate::bigint::to_big(other) {
                        Some(number) => number,
                        None => return Err(Error::encode("Expected data of type int or str.")),
                    },
                };

                return if range.allows(&number) {
                    Ok(())
                } else {
                    fail("Expected an integer in the range.")
                };
            }

            let number = match value {
                Value::Integer(number) => *number,
                Value::BigInteger(bytes) => {
                    return if big_in_range(node.effective_range, &node.effective_range_parts, bytes)
                    {
                        Ok(())
                    } else {
                        fail("Expected an integer in the range.")
                    };
                }
                Value::Text(name) => match named.iter().find(|(candidate, _)| candidate == name) {
                    Some((_, number)) => *number,
                    None => return Err(Error::encode("Expected an integer or a named number.")),
                },
                _ => return Err(Error::encode("Expected data of type int or str.")),
            };

            if !in_range(node.effective_range, &node.effective_range_parts, number) {
                return fail("Expected an integer in the range.");
            }

            Ok(())
        }
        (Kind::Str(kind), Value::Text(text)) => check_string(*kind, text, node, true),
        // Its text is made only for a SIZE, of which it is the length.
        (Kind::ObjectIdentifier, Value::ObjectIdentifier(oid)) => match node.effective_size {
            Some(_) => check_string(StrKind::Utf8, &oid.to_string(), node, false),
            None => Ok(()),
        },
        (Kind::OctetString, Value::Bytes(bytes)) => {
            if !in_range(
                node.effective_size,
                &node.effective_size_parts,
                bytes.len() as i128,
            ) {
                return fail("Expected a number of bytes in the range.");
            }

            Ok(())
        }
        (Kind::BitString(_), Value::Bits(_, bits)) => {
            if !in_range(
                node.effective_size,
                &node.effective_size_parts,
                *bits as i128,
            ) {
                return fail("Expected a number of bits in the range.");
            }

            Ok(())
        }
        (Kind::SequenceOf(element) | Kind::SetOf(element), Value::List(items)) => {
            if !in_range(
                node.effective_size,
                &node.effective_size_parts,
                items.len() as i128,
            ) {
                return fail("Expected a list of a length in the range.");
            }

            for (index, item) in items.iter().enumerate() {
                check_at(types, *element, item, depth + 1)
                    .map_err(|error| error.in_element(index))?;
            }

            Ok(())
        }
        (
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members),
            Value::Map(map),
        ) => {
            for member in members.all() {
                if let Some((_, value)) = map.iter().find(|(name, _)| name == &member.name) {
                    check_at(types, member.ty, value, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?;
                }
            }

            Ok(())
        }
        (Kind::Choice(members), Value::Choice(name, inner)) => {
            match members.all().find(|member| &member.name == name) {
                Some(member) => check_at(types, member.ty, inner, depth + 1)
                    .map_err(|error| error.in_member(&member.name)),
                // An alternative that is not known is one of a later version.
                None if members.additions.is_some() => Ok(()),
                None => fail("Expected the name of an alternative of the CHOICE."),
            }
        }
        // A value that is of another kind is the business of the codec.
        _ => Ok(()),
    }
}

fn check_string(
    kind: StrKind,
    text: &str,
    node: &crate::types::TypeNode,
    has_alphabet: bool,
) -> Result<()> {
    if !in_range(
        node.effective_size,
        &node.effective_size_parts,
        text.chars().count() as i128,
    ) {
        return fail("Expected a number of characters in the range.");
    }

    if !has_alphabet {
        return Ok(());
    }

    match &node.effective_alphabet {
        Some(alphabet) => {
            if text.chars().any(|character| !alphabet.contains(&character)) {
                return fail("Expected a character in the permitted alphabet.");
            }
        }
        None => {
            if text
                .chars()
                .any(|character| !in_default_alphabet(kind, character))
            {
                return fail("Expected a character in the permitted alphabet.");
            }
        }
    }

    Ok(())
}

/// If `value` of the type `id` satisfies the constraints of a component
/// or of the elements.
fn satisfies(
    types: &Types,
    id: TypeId,
    checked: &Checked,
    value: &Value,
    depth: usize,
) -> Result<bool> {
    if depth > MAX_DEPTH {
        return fail("The value is nested too deeply.");
    }

    let concrete = types.node(id).concrete;
    let size = |size: usize| in_range(checked.size, &checked.size_parts, size as i128);
    let is_in = match (&types.node(concrete).kind, value) {
        #[cfg(feature = "bigint")]
        (Kind::Integer(named), value) if checked.big_range.is_some() => {
            let range = checked.big_range.as_ref().unwrap();
            let number = match value {
                Value::Text(name) => named
                    .iter()
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, number)| num_bigint::BigInt::from(*number)),
                other => crate::bigint::to_big(other),
            };

            // A value that is of another kind is the business of the codec.
            number.is_none_or(|number| range.allows(&number))
        }
        (Kind::Integer(_), Value::Integer(number)) => {
            in_range(checked.range, &checked.range_parts, *number)
        }
        (Kind::Integer(named), Value::Text(name)) => {
            match named.iter().find(|(candidate, _)| candidate == name) {
                Some((_, number)) => in_range(checked.range, &checked.range_parts, *number),
                None => true,
            }
        }
        (Kind::Integer(_), Value::BigInteger(bytes)) => {
            big_in_range(checked.range, &checked.range_parts, bytes)
        }
        (Kind::Str(_), Value::Text(text)) => {
            size(text.chars().count())
                && checked.alphabet.as_ref().is_none_or(|alphabet| {
                    text.chars()
                        .all(|character| alphabet.binary_search(&character).is_ok())
                })
        }
        (Kind::OctetString, Value::Bytes(bytes)) => size(bytes.len()),
        (Kind::BitString(_), Value::Bits(_, bits)) => size(*bits),
        (Kind::SequenceOf(_) | Kind::SetOf(_), Value::List(items)) => size(items.len()),
        // A value that is of another kind is the business of the codec.
        _ => true,
    };

    if !is_in {
        return Ok(false);
    }

    match &checked.inner {
        Some(inner) => satisfies_inner(types, concrete, inner, value, depth + 1),
        None => Ok(true),
    }
}

/// If `value` of the type `concrete` satisfies an inner type constraint:
/// any of the alternatives of WITH COMPONENTS, or of WITH COMPONENT with
/// every element.
fn satisfies_inner(
    types: &Types,
    concrete: TypeId,
    inner: &Inner,
    value: &Value,
    depth: usize,
) -> Result<bool> {
    match (inner, &types.node(concrete).kind, value) {
        (
            Inner::Element(alternatives),
            Kind::SequenceOf(element) | Kind::SetOf(element),
            Value::List(items),
        ) => {
            'alternatives: for checked in alternatives {
                for item in items {
                    if !satisfies(types, *element, checked, item, depth)? {
                        continue 'alternatives;
                    }
                }

                return Ok(true);
            }

            Ok(false)
        }
        (
            Inner::Components(alternatives),
            Kind::Sequence(members)
            | Kind::Set(members)
            | Kind::Choice(members)
            | Kind::External(members),
            Value::Map(_) | Value::Choice(..),
        ) => {
            let component = |name: &str| match value {
                Value::Map(map) => map
                    .iter()
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, value)| value),
                Value::Choice(chosen, value) => (chosen == name).then_some(value.as_ref()),
                _ => None,
            };

            'alternatives: for alternative in alternatives {
                for constraint in alternative {
                    let found = component(&constraint.name);

                    if constraint
                        .presence
                        .is_some_and(|present| present != found.is_some())
                    {
                        continue 'alternatives;
                    }

                    let member = members.all().find(|member| member.name == constraint.name);

                    if let (Some(found), Some(member)) = (found, member)
                        && !satisfies(types, member.ty, &constraint.constraint, found, depth)?
                    {
                        continue 'alternatives;
                    }
                }

                return Ok(true);
            }

            Ok(false)
        }
        (inner, Kind::Duration, Value::Text(text)) => Ok(satisfies_duration(inner, text)),
        _ => Ok(true),
    }
}

/// A time element of DURATION-EQUIVALENT, or one of its fractional part:
/// its name and its digits.
type Field<'a> = (&'a str, alloc::string::String);

/// If a DURATION satisfies an inner type constraint, which is one of its
/// DURATION-EQUIVALENT value (X.680 38.4.4.3).
fn satisfies_duration(inner: &Inner, text: &str) -> bool {
    // Not a duration, which is the business of the codec.
    let Ok(elements) = crate::time::duration_equivalent(text) else {
        return true;
    };
    let fields: Vec<Field> = elements
        .iter()
        .map(|(name, digits, _)| (*name, (*digits).into()))
        .collect();
    let fraction: Option<Vec<Field>> = elements.iter().find_map(|(_, _, fraction)| {
        fraction.map(|digits| {
            alloc::vec![
                ("number-of-digits", alloc::format!("{}", digits.len())),
                ("fractional-value", digits.into()),
            ]
        })
    });

    satisfies_fields(inner, &fields, fraction.as_deref())
}

/// If an integer that is not negative, given by its digits, is in the
/// range of a constraint.
fn digits_in_range(checked: &Checked, digits: &str) -> bool {
    #[cfg(feature = "bigint")]
    if let Some(range) = &checked.big_range {
        return digits
            .parse::<num_bigint::BigInt>()
            .is_ok_and(|number| range.allows(&number));
    }

    match digits.parse::<i128>() {
        Ok(number) => in_range(checked.range, &checked.range_parts, number),
        // Greater than any bound.
        Err(_) => checked.range.is_none_or(|range| {
            range.extensible
                || match &checked.range_parts {
                    Some(parts) => parts.iter().any(|(_, upper)| upper.is_none()),
                    None => range.max.is_none(),
                }
        }),
    }
}

/// If fields satisfy WITH COMPONENTS, with those of a fractional part
/// that is a component of its own.
fn satisfies_fields(inner: &Inner, fields: &[Field], fraction: Option<&[Field]>) -> bool {
    let Inner::Components(alternatives) = inner else {
        return true;
    };

    alternatives.iter().any(|alternative| {
        alternative.iter().all(|constraint| {
            let field = fields.iter().find(|(name, _)| *name == constraint.name);
            let is_fraction = constraint.name == "fractional-part";
            let present = if is_fraction {
                fraction.is_some()
            } else {
                field.is_some()
            };

            if constraint
                .presence
                .is_some_and(|presence| presence != present)
            {
                return false;
            }

            let checked = &constraint.constraint;

            match (is_fraction, fraction, field) {
                (true, Some(fraction), _) => checked
                    .inner
                    .as_ref()
                    .is_none_or(|inner| satisfies_fields(inner, fraction, None)),
                (false, _, Some((_, digits))) => digits_in_range(checked, digits),
                _ => true,
            }
        })
    })
}
