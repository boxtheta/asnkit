// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Generic String Encoding Rules (GSER), RFC 3641.
//!
//! A value is written as the runtime for Python writes it: as a value
//! assignment, `question Question ::= { id 1, question "Is 1+1=3?" }`,
//! with the name of the type in lower case for the name of the value.
//! A SEQUENCE and a SET are `{ name value, ... }`, a CHOICE `name :
//! value`, a SEQUENCE OF and a SET OF `{ value, ... }`, an OCTET STRING
//! `'0123'H`, a BIT STRING `'0101'B` and the strings and times between
//! `"`, with a `"` in them written twice (RFC 3641 3.2).
//!
//! It is for reading by people, as debug output, and is not decoded.

use alloc::string::String;
use alloc::vec::Vec;

use crate::decimal;
use crate::time::{self, iso};
use crate::types::{Kind, MemberInfo, Members, TypeId, Types};
use crate::value::Value;
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

/// Encodes `value` as the type `id`, on one line.
pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    encode_with(types, id, value, " ", 0)
}

/// Encodes `value` as the type `id`, with a line for each member and
/// element and `indent` spaces for each level.
pub fn encode_indented(types: &Types, id: TypeId, value: &Value, indent: usize) -> Result<Vec<u8>> {
    encode_with(types, id, value, "\n", indent)
}

fn encode_with(
    types: &Types,
    id: TypeId,
    value: &Value,
    separator: &str,
    indent: usize,
) -> Result<Vec<u8>> {
    let Some(name) = type_name(types, id) else {
        return encode_error("The type has no name.");
    };
    let mut out = String::new();

    out.push_str(&name.to_ascii_lowercase());
    out.push(' ');
    out.push_str(name);
    out.push_str(" ::= ");
    Encoder { types, indent }.encode(id, value, separator, &mut out, 0)?;

    Ok(out.into_bytes())
}

/// The name of the type `id`, in the first module that has it.
fn type_name(types: &Types, id: TypeId) -> Option<&str> {
    types
        .modules
        .iter()
        .flat_map(|(_, names)| names.iter())
        .find(|(_, candidate)| *candidate == id)
        .map(|(name, _)| name.as_str())
}

fn names<'a>(
    root: &'a [(String, i64)],
    additions: &'a Option<Vec<(String, i64)>>,
) -> impl Iterator<Item = &'a (String, i64)> {
    root.iter().chain(additions.iter().flatten())
}

/// The type of a member, which for an ANY DEFINED BY with choices is
/// that of the choice of the value of the member it names, if there is
/// one.
fn member_type(types: &Types, member: &MemberInfo, map: &[(String, Value)]) -> Option<TypeId> {
    types.defined_by(member.ty, map).unwrap_or(Some(member.ty))
}

struct Encoder<'a> {
    types: &'a Types,
    /// The spaces of a level, after a new line.
    indent: usize,
}

impl Encoder<'_> {
    fn write_integer(&self, value: &Value, out: &mut String) -> Result<()> {
        match value {
            Value::Integer(number) => out.push_str(&alloc::format!("{number}")),
            Value::BigInteger(bytes) => match decimal::twos_complement_to_decimal(bytes) {
                Some(text) => out.push_str(&text),
                None => return encode_error("The integer has too many digits."),
            },
            _ => return encode_error("Expected an integer."),
        }

        Ok(())
    }

    /// A `StringValue`, with a `"` in it twice (RFC 3641 3.2).
    fn write_string(out: &mut String, text: &str) {
        out.push('"');

        for character in text.chars() {
            if character == '"' {
                out.push('"');
            }

            out.push(character);
        }

        out.push('"');
    }

    /// The separator of the members or elements of a level that begins
    /// with `separator`.
    fn inner(&self, separator: &str) -> String {
        let mut inner = String::from(separator);

        inner.extend(core::iter::repeat_n(' ', self.indent));

        inner
    }

    fn encode(
        &self,
        id: TypeId,
        value: &Value,
        separator: &str,
        out: &mut String,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return encode_error("The value is nested too deeply.");
        }

        match self.types.kind(id) {
            Kind::Ref(_) => encode_error("The type has no contents of its own."),
            Kind::Boolean => match value {
                Value::Boolean(value) => {
                    out.push_str(if *value { "TRUE" } else { "FALSE" });

                    Ok(())
                }
                _ => encode_error("Expected data of type bool."),
            },
            Kind::Integer(named) => match value {
                Value::Integer(_) | Value::BigInteger(_) => self.write_integer(value, out),
                Value::Text(name) => match named.iter().find(|(candidate, _)| candidate == name) {
                    Some((_, number)) => self.write_integer(&Value::Integer(*number), out),
                    None => encode_error("Expected an integer or a named number."),
                },
                _ => encode_error("Expected data of type int or str."),
            },
            Kind::Real => {
                // An integer is written as the float it is, as a runtime
                // with numbers that are all floats can only give it.
                let value = match value {
                    Value::Real(value) => *value,
                    Value::Integer(value) => *value as f64,
                    Value::BigInteger(bytes) => crate::ber::twos_complement_to_f64(bytes)?,
                    _ => return encode_error("Expected data of type float or int."),
                };

                if value.is_nan() {
                    return encode_error("Cannot encode floating point number NaN.");
                } else if value.is_infinite() {
                    out.push_str(if value > 0.0 {
                        "PLUS-INFINITY"
                    } else {
                        "MINUS-INFINITY"
                    });
                } else if value == 0.0 {
                    out.push('0');
                } else {
                    out.push_str(&real(value));
                }

                Ok(())
            }
            Kind::Null => match value {
                Value::Null => {
                    out.push_str("NULL");

                    Ok(())
                }
                _ => encode_error("Expected None."),
            },
            Kind::BitString(_) => {
                let Value::Bits(bytes, bits) = value else {
                    return encode_error("Expected data of type tuple(bytes, int).");
                };

                if bytes.len() * 8 < *bits {
                    return encode_error("Expected more bits of data.");
                }

                out.push('\'');
                out.extend(
                    (0..*bits).map(|bit| match bytes[bit / 8] & (0x80 >> (bit % 8)) {
                        0 => '0',
                        _ => '1',
                    }),
                );
                out.push_str("'B");

                Ok(())
            }
            Kind::OctetString | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                match value {
                    Value::Bytes(bytes) => {
                        out.push('\'');
                        out.push_str(&decimal::hex_upper(bytes));
                        out.push_str("'H");

                        Ok(())
                    }
                    _ => encode_error("Expected data of type bytes or bytearray."),
                }
            }
            Kind::ObjectIdentifier => match value {
                Value::Text(text) => {
                    out.push_str(text);

                    Ok(())
                }
                _ => encode_error("Expected data of type str."),
            },
            Kind::Str(_) => match value {
                Value::Text(text) => {
                    Self::write_string(out, text);

                    Ok(())
                }
                _ => encode_error("Expected data of type str."),
            },
            Kind::Enumerated { root, additions } => {
                let name = match value {
                    Value::Text(name) if !self.types.numeric_enums => {
                        names(root, additions).find(|(candidate, _)| candidate == name)
                    }
                    Value::Integer(number) if self.types.numeric_enums => names(root, additions)
                        .find(|(_, candidate)| i128::from(*candidate) == *number),
                    _ => None,
                };

                match name {
                    Some((name, _)) => {
                        out.push_str(name);

                        Ok(())
                    }
                    None => encode_error("Expected an enumeration value."),
                }
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                self.encode_members(members, value, separator, out, depth)
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let Value::List(items) = value else {
                    return encode_error("Expected data of type list.");
                };
                let inner = self.inner(separator);

                out.push('{');

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }

                    out.push_str(&inner);
                    self.encode(*element, item, &inner, out, depth + 1)
                        .map_err(|error| error.in_element(index))?;
                }

                out.push_str(separator);
                out.push('}');

                Ok(())
            }
            Kind::Utc => match value {
                Value::DateTime(value) => {
                    Self::write_string(out, &time::utc_time_to_string(value, false));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => {
                    Self::write_string(out, &time::generalized_time_to_string(value, false));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Date => match value {
                Value::Date(value) => {
                    Self::write_string(out, &iso::date_to_string(value));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.date."),
            },
            Kind::TimeOfDay => match value {
                Value::Time(value) => {
                    Self::write_string(out, &iso::time_to_string(value)?);

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.time."),
            },
            Kind::DateTime => match value {
                Value::DateTime(value) => {
                    Self::write_string(out, &iso::date_time_to_string(value)?);

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Duration => match value {
                Value::Text(text) => {
                    time::validate_duration(text)?;
                    Self::write_string(out, text);

                    Ok(())
                }
                _ => encode_error("Expected data of type str."),
            },
            Kind::Choice(members) => {
                let Value::Choice(name, inner) = value else {
                    return encode_error("Expected data of type tuple(str, object).");
                };
                let Some(member) = members.all().find(|member| &member.name == name) else {
                    return encode_error("Expected a name of an alternative of the CHOICE.");
                };

                out.push_str(name);
                out.push_str(" : ");

                self.encode(member.ty, inner, separator, out, depth + 1)
                    .map_err(|error| error.in_member(name))
            }
        }
    }

    fn encode_members(
        &self,
        members: &Members,
        value: &Value,
        separator: &str,
        out: &mut String,
        depth: usize,
    ) -> Result<()> {
        let Value::Map(map) = value else {
            return encode_error("Expected data of type dict.");
        };
        let inner = self.inner(separator);
        let mut first = true;

        out.push('{');

        // In the order they are defined in, as the runtime for Python
        // writes them.
        for member in members.defined() {
            match map.iter().find(|(name, _)| name == &member.name) {
                Some((_, value)) => {
                    if !first {
                        out.push(',');
                    }

                    first = false;
                    out.push_str(&inner);
                    out.push_str(&member.name);
                    out.push(' ');

                    let ty = member_type(self.types, member, map)
                        .ok_or_else(|| Error::encode("Bad AnyDefinedBy choice."))?;

                    self.encode(ty, value, &inner, out, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?;
                }
                None if member.optional || member.has_default() => {}
                None => return encode_error("A member is not found in the SEQUENCE or SET."),
            }
        }

        out.push_str(separator);
        out.push('}');

        Ok(())
    }
}

/// A finite float that is not 0 as a `realNumber` (RFC 3641 3.4): as
/// `repr()` of Python writes it and `E0`, as the runtime for Python
/// does, or with its exponent after `E` if it has one.
fn real(value: f64) -> String {
    let repr = decimal::float_repr(value);

    match repr.split_once('e') {
        Some((mantissa, exponent)) => {
            let (sign, digits) = match exponent.as_bytes().first() {
                Some(b'-') => ("-", &exponent[1..]),
                Some(b'+') => ("", &exponent[1..]),
                _ => ("", exponent),
            };

            let digits = match digits.trim_start_matches('0') {
                "" => "0",
                digits => digits,
            };

            alloc::format!("{mantissa}E{sign}{digits}")
        }
        None => alloc::format!("{repr}E0"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reals_are_written_as_python_writes_them() {
        assert_eq!(real(1.0), "1.0E0");
        assert_eq!(real(1.1), "1.1E0");
        assert_eq!(real(1234.5678), "1234.5678E0");
        assert_eq!(real(0.625), "0.625E0");
        assert_eq!(real(-2.5), "-2.5E0");
        // Python writes 1e+20E0, which is not GSER.
        assert_eq!(real(1e20), "1E20");
        assert_eq!(real(1.5e-7), "1.5E-7");
    }
}
