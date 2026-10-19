// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! XML Encoding Rules (XER), X.693.
//!
//! A value is an element named after its type or its member. A SEQUENCE
//! and a SET have an element for each member, a CHOICE one for the
//! alternative, a SEQUENCE OF and a SET OF one for each item, named
//! after the type of the items. A BOOLEAN is `<true />` or `<false />`
//! and an ENUMERATED the element of its value, in an element. What is
//! written is the text of `ElementTree.tostring` of the runtime for
//! Python, in ASCII.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::decimal;
use crate::time::{self, iso};
use crate::types::{Kind, MemberInfo, Members, TypeId, Types};
use crate::value::Value;
use crate::xml::{self, Element};
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

fn decode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::decode(what, 0))
}

/// The name a type is given at the top, that of its definition.
fn name_of(types: &Types, id: TypeId) -> &str {
    types
        .modules
        .iter()
        .flat_map(|(_, assignments)| assignments)
        .find(|(_, candidate)| *candidate == id)
        .map_or("", |(name, _)| name)
}

/// Encodes `value` as the type `id`.
pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    let element = Coder { types }.encode(id, name_of(types, id), value, 0)?;
    let mut out = String::new();

    xml::write(&element, &mut out);

    Ok(out.into_bytes())
}

/// Encodes `value` as the type `id`, with `indent` spaces for each
/// level.
pub fn encode_indented(types: &Types, id: TypeId, value: &Value, indent: usize) -> Result<Vec<u8>> {
    let element = Coder { types }.encode(id, name_of(types, id), value, 0)?;
    let mut out = String::new();

    xml::write_indented(&element, &mut out, indent);

    Ok(out.into_bytes())
}

/// Decodes `data` as the type `id`.
pub fn decode(types: &Types, id: TypeId, data: &[u8]) -> Result<Value> {
    let Ok(text) = core::str::from_utf8(data) else {
        return decode_error("Expected UTF-8.");
    };

    Coder { types }.decode(id, &xml::parse(text)?, 0)
}

struct Coder<'a> {
    types: &'a Types,
}

fn names<'a>(
    root: &'a [(String, i64)],
    additions: &'a Option<Vec<(String, i64)>>,
) -> impl Iterator<Item = &'a (String, i64)> {
    root.iter().chain(additions.iter().flatten())
}

/// Whether `c` is white-space in XML.
fn is_space(c: char) -> bool {
    matches!(c, ' ' | '\t' | '\n' | '\r')
}

/// Whether `text` is nothing but white-space.
fn is_blank(text: &Option<String>) -> bool {
    text.as_deref().unwrap_or("").chars().all(is_space)
}

/// A BOOLEAN, `<true/>` or `<false/>` (X.680 18.3): an empty element
/// and nothing else, but space.
fn boolean(element: &Element) -> Result<bool> {
    match element.name.as_str() {
        "true" | "false" if element.text.is_none() && element.children.is_empty() => {
            Ok(element.name == "true")
        }
        _ => decode_error("Expected <true/> or <false/>."),
    }
}

fn time_error<T>(result: Result<T>) -> Result<T> {
    result.map_err(|_| Error::decode("Expected a date or a time.", 0))
}

impl Coder<'_> {
    /// The name of the value of an ENUMERATED.
    fn enumeration_name<'v>(
        &self,
        root: &'v [(String, i64)],
        additions: &'v Option<Vec<(String, i64)>>,
        value: &'v Value,
    ) -> Result<&'v str> {
        let found = match value {
            Value::Text(name) if !self.types.numeric_enums => names(root, additions)
                .find(|(candidate, _)| candidate == name)
                .map(|(name, _)| name.as_str()),
            Value::Integer(number) if self.types.numeric_enums => names(root, additions)
                .find(|(_, candidate)| i128::from(*candidate) == *number)
                .map(|(name, _)| name.as_str()),
            _ => None,
        };

        found.ok_or_else(|| Error::encode("Expected an enumeration value."))
    }

    fn integer_text(&self, value: &Value) -> Result<String> {
        match value {
            Value::Integer(number) => Ok(format!("{number}")),
            Value::BigInteger(bytes) => decimal::twos_complement_to_decimal(bytes)
                .ok_or_else(|| Error::encode("The integer has too many digits.")),
            _ => encode_error("Expected an integer."),
        }
    }

    /// The element of a value that is an item of a SEQUENCE OF or a
    /// SET OF: a BOOLEAN, an ENUMERATED and a CHOICE are not in an element
    /// of their own then.
    fn encode_of(&self, id: TypeId, name: &str, value: &Value, depth: usize) -> Result<Element> {
        match self.types.kind(id) {
            Kind::Boolean => match value {
                Value::Boolean(value) => Ok(Element::new(if *value { "true" } else { "false" })),
                _ => encode_error("Expected data of type bool."),
            },
            Kind::Enumerated { root, additions } => {
                Ok(Element::new(self.enumeration_name(root, additions, value)?))
            }
            Kind::Choice(members) => {
                let (member, inner) = self.alternative(members, value)?;

                self.encode(member.ty, &member.name, inner, depth + 1)
                    .map_err(|error| error.in_member(&member.name))
            }
            _ => self.encode(id, name, value, depth),
        }
    }

    fn alternative<'m, 'v>(
        &self,
        members: &'m Members,
        value: &'v Value,
    ) -> Result<(&'m MemberInfo, &'v Value)> {
        let Value::Choice(name, inner) = value else {
            return encode_error("Expected data of type tuple(str, object).");
        };

        match members.all().find(|member| &member.name == name) {
            Some(member) => Ok((member, inner)),
            None => encode_error("Expected a name of an alternative of the CHOICE."),
        }
    }

    fn encode(&self, id: TypeId, name: &str, value: &Value, depth: usize) -> Result<Element> {
        if depth > MAX_DEPTH {
            return encode_error("The value is nested too deeply.");
        }

        let kind = self.types.kind(id);
        let mut element = Element::new(name);

        match kind {
            Kind::Ref(_) => return encode_error("The type has no contents of its own."),
            Kind::Boolean => match value {
                Value::Boolean(value) => {
                    element
                        .children
                        .push(Element::new(if *value { "true" } else { "false" }))
                }
                _ => return encode_error("Expected data of type bool."),
            },
            Kind::Integer(named) => {
                let text = match value {
                    Value::Text(name) => {
                        match named.iter().find(|(candidate, _)| candidate == name) {
                            Some((_, number)) => format!("{number}"),
                            None => return encode_error("Expected an integer or a named number."),
                        }
                    }
                    Value::Integer(_) | Value::BigInteger(_) => self.integer_text(value)?,
                    _ => return encode_error("Expected data of type int or str."),
                };

                element.text = Some(text);
            }
            Kind::Real => {
                let number = match value {
                    Value::Real(number) => *number,
                    Value::Integer(number) => *number as f64,
                    Value::BigInteger(bytes) => crate::ber::twos_complement_to_f64(bytes)?,
                    _ => return encode_error("Expected data of type float or int."),
                };

                // The special values are empty elements (X.680 21.9).
                if number.is_nan() {
                    element.children.push(Element::new("NOT-A-NUMBER"));
                } else if number.is_infinite() {
                    element.children.push(Element::new(if number > 0.0 {
                        "PLUS-INFINITY"
                    } else {
                        "MINUS-INFINITY"
                    }));
                } else {
                    element.text = Some(decimal::xml_real(number));
                }
            }
            Kind::Null => match value {
                Value::Null => {}
                _ => return encode_error("Expected None."),
            },
            Kind::BitString(_) => {
                let Value::Bits(bytes, bits) = value else {
                    return encode_error("Expected data of type tuple(bytes, int).");
                };

                if bytes.len() * 8 < *bits {
                    return encode_error("Expected more bits of data.");
                }

                if *bits > 0 {
                    element.text = Some(
                        (0..*bits)
                            .map(|bit| {
                                if bytes[bit / 8] & (0x80 >> (bit % 8)) != 0 {
                                    '1'
                                } else {
                                    '0'
                                }
                            })
                            .collect(),
                    );
                }
            }
            Kind::OctetString => match value {
                Value::Bytes(bytes) => {
                    if !bytes.is_empty() {
                        element.text = Some(decimal::hex_upper(bytes));
                    }
                }
                _ => return encode_error("Expected data of type bytes or bytearray."),
            },
            Kind::ObjectIdentifier | Kind::Str(_) => match value {
                Value::Text(text) => return Ok(Element::with_text(name, text)),
                _ => return encode_error("Expected data of type str."),
            },
            Kind::Enumerated { root, additions } => {
                let value = self.enumeration_name(root, additions, value)?;

                element.children.push(Element::new(value));
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                let Value::Map(map) = value else {
                    return encode_error("Expected data of type dict.");
                };

                for member in members.all() {
                    match map.iter().find(|(candidate, _)| candidate == &member.name) {
                        Some((_, value)) => element.children.push(
                            self.encode(
                                self.member_type(member, map)
                                    .ok_or_else(|| Error::encode("Bad AnyDefinedBy choice."))?,
                                &member.name,
                                value,
                                depth + 1,
                            )
                            .map_err(|error| error.in_member(&member.name))?,
                        ),
                        None if member.optional || member.has_default() => {}
                        None => {
                            return encode_error("A member is not found in the SEQUENCE or SET.");
                        }
                    }
                }
            }
            Kind::SequenceOf(item) | Kind::SetOf(item) => {
                let Value::List(items) = value else {
                    return encode_error("Expected data of type list.");
                };
                let item_name = self
                    .types
                    .node(self.types.node(id).concrete)
                    .element_name
                    .as_deref()
                    .unwrap_or("");

                for (index, entry) in items.iter().enumerate() {
                    element.children.push(
                        self.encode_of(*item, item_name, entry, depth + 1)
                            .map_err(|error| error.in_element(index))?,
                    );
                }
            }
            Kind::Utc => match value {
                Value::DateTime(value) => {
                    element.text = Some(time::utc_time_to_string(value, false));
                }
                _ => return encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => {
                    element.text = Some(time::generalized_time_to_string(value, false));
                }
                _ => return encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Date => match value {
                Value::Date(value) => element.text = Some(iso::date_to_string(value)),
                _ => return encode_error("Expected data of type datetime.date."),
            },
            Kind::TimeOfDay => match value {
                Value::Time(value) => element.text = Some(iso::time_to_string(value)?),
                _ => return encode_error("Expected data of type datetime.time."),
            },
            Kind::DateTime => match value {
                Value::DateTime(value) => element.text = Some(iso::date_time_to_string(value)?),
                _ => return encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Duration => match value {
                Value::Text(text) => {
                    time::validate_duration(text)?;
                    let canon = time::duration_to_canonical(text)?;
                    element.text = Some(canon);
                }
                _ => return encode_error("Expected data of type str."),
            },
            Kind::Choice(members) => {
                let (member, inner) = self.alternative(members, value)?;

                element.children.push(
                    self.encode(member.ty, &member.name, inner, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?,
                );
            }
            // The value of the type it is (X.693 8.5), which only an ANY
            // DEFINED BY with choices tells, as a member.
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                return Err(Error::unsupported(
                    "XER needs the type of the value of an ANY or an open type.",
                ));
            }
        }

        Ok(element)
    }

    /// The type of a member, which for an ANY DEFINED BY with choices is
    /// that of the choice of the value of the member it names, if there is
    /// one.
    fn member_type(&self, member: &MemberInfo, map: &[(String, Value)]) -> Option<TypeId> {
        self.types
            .defined_by(member.ty, map)
            .unwrap_or(Some(member.ty))
    }

    /// Decodes an item of a SEQUENCE OF or a SET OF.
    fn decode_of(&self, id: TypeId, element: &Element, depth: usize) -> Result<Value> {
        match self.types.kind(id) {
            Kind::Boolean => boolean(element).map(Value::Boolean),
            Kind::Enumerated { root, additions } => {
                match names(root, additions).find(|(name, _)| name == &element.name) {
                    Some((name, number)) => Ok(self.enumeration_value(name, *number)),
                    None => decode_error("Expected an enumeration value."),
                }
            }
            Kind::Choice(members) => match members.all().find(|member| member.name == element.name)
            {
                Some(member) => Ok(Value::Choice(
                    member.name.clone(),
                    self.decode(member.ty, element, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?
                        .into(),
                )),
                None => decode_error("Expected a name of an alternative of the CHOICE."),
            },
            _ => self.decode(id, element, depth),
        }
    }

    fn enumeration_value(&self, name: &str, number: i64) -> Value {
        if self.types.numeric_enums {
            Value::Integer(i128::from(number))
        } else {
            Value::text(name)
        }
    }

    fn text<'e>(&self, element: &'e Element) -> Result<&'e str> {
        match &element.text {
            Some(text) => Ok(text),
            None => decode_error("Expected text in the XML element."),
        }
    }

    /// The text of a character string, in which a control character is
    /// an empty element, as `<bel/>` (X.680 12.15.5).
    fn string(&self, element: &Element) -> Result<String> {
        let mut text = String::from(element.text.as_deref().unwrap_or(""));

        for child in &element.children {
            let control = crate::xml::control(&child.name)
                .filter(|_| child.text.is_none() && child.children.is_empty());

            match control {
                Some(control) => text.push(control),
                None => return decode_error("Expected text in the XML element."),
            }

            text.push_str(child.tail.as_deref().unwrap_or(""));
        }

        Ok(text)
    }

    fn decode(&self, id: TypeId, element: &Element, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.");
        }

        match self.types.kind(id) {
            Kind::Ref(_) => decode_error("The type has no contents of its own."),
            Kind::Boolean => match element.children.as_slice() {
                [value] if is_blank(&element.text) && is_blank(&value.tail) => {
                    boolean(value).map(Value::Boolean)
                }
                _ => decode_error("Expected <true/> or <false/>."),
            },
            Kind::Integer(_) => match decimal::decimal_to_value(self.text(element)?.trim()) {
                Some(value) => Ok(value),
                None => decode_error("Expected an integer."),
            },
            Kind::Real => match element.children.as_slice() {
                [] => {
                    let text = self.text(element)?.trim();

                    match text.parse::<f64>() {
                        Ok(value) if decimal::is_xml_real(text) => Ok(Value::Real(value)),
                        _ => decode_error("Expected a real."),
                    }
                }
                [special]
                    if element.text.as_deref().unwrap_or("").trim().is_empty()
                        && special.text.is_none()
                        && special.children.is_empty() =>
                {
                    match special.name.as_str() {
                        "PLUS-INFINITY" => Ok(Value::Real(f64::INFINITY)),
                        "MINUS-INFINITY" => Ok(Value::Real(f64::NEG_INFINITY)),
                        "NOT-A-NUMBER" => Ok(Value::Real(f64::NAN)),
                        _ => decode_error("Expected a real."),
                    }
                }
                _ => decode_error("Expected a real."),
            },
            Kind::Null => Ok(Value::Null),
            // An `xmlbstring` may have space in it (X.680 12.10).
            Kind::BitString(_) => {
                let text: String = element
                    .text
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| !is_space(*c))
                    .collect();
                let mut bytes = alloc::vec![0u8; text.len().div_ceil(8)];

                for (bit, c) in text.bytes().enumerate() {
                    match c {
                        b'0' => {}
                        b'1' => bytes[bit / 8] |= 0x80 >> (bit % 8),
                        _ => return decode_error("Expected 0 and 1 in a BIT STRING."),
                    }
                }

                Ok(Value::Bits(bytes, text.len()))
            }
            // An `xmlhstring` may have space in it (X.680 12.12), and an
            // odd number of digits is ended by a zero (X.680 22.3).
            Kind::OctetString => {
                let mut text: String = element
                    .text
                    .as_deref()
                    .unwrap_or("")
                    .chars()
                    .filter(|c| !is_space(*c))
                    .collect();

                if text.len() % 2 == 1 {
                    text.push('0');
                }

                match decimal::hex_decode(&text) {
                    Some(bytes) => Ok(Value::Bytes(bytes)),
                    None => decode_error("Expected hex."),
                }
            }
            Kind::ObjectIdentifier => Ok(Value::text(self.text(element)?)),
            Kind::Str(_) => self.string(element).map(Value::Text),
            Kind::Enumerated { root, additions } => {
                let Some(inner) = element.children.first() else {
                    return decode_error("Expected an enumeration value.");
                };

                match names(root, additions).find(|(name, _)| name == &inner.name) {
                    Some((name, number)) => Ok(self.enumeration_value(name, *number)),
                    None if additions.is_some() => Ok(Value::Null),
                    None => decode_error("Expected an enumeration value."),
                }
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                let mut map = Vec::new();

                for member in members.all() {
                    match element.child(&member.name) {
                        Some(inner) => {
                            let ty = self
                                .member_type(member, &map)
                                .ok_or_else(|| Error::decode("Bad AnyDefinedBy choice.", 0))?;

                            let value = self
                                .decode(ty, inner, depth + 1)
                                .map_err(|error| error.in_member(&member.name))?;

                            map.push((member.name.clone(), value));
                        }
                        None if member.optional => {}
                        None => match &member.default {
                            Some(default) => map.push((member.name.clone(), default.clone())),
                            None => return decode_error("A member is not found in the element."),
                        },
                    }
                }

                Ok(Value::Map(map))
            }
            Kind::SequenceOf(item) | Kind::SetOf(item) => element
                .children
                .iter()
                .enumerate()
                .map(|(index, child)| {
                    self.decode_of(*item, child, depth + 1)
                        .map_err(|error| error.in_element(index))
                })
                .collect::<Result<Vec<_>>>()
                .map(Value::List),
            Kind::Utc => time_error(time::utc_time_from_string(self.text(element)?, false))
                .map(Value::DateTime),
            Kind::Generalized => time_error(time::generalized_time_from_string(
                self.text(element)?,
                false,
            ))
            .map(Value::DateTime),
            Kind::Date => time_error(iso::date_from_string(self.text(element)?)).map(Value::Date),
            Kind::TimeOfDay => {
                time_error(iso::time_from_string(self.text(element)?)).map(Value::Time)
            }
            Kind::DateTime => {
                time_error(iso::date_time_from_string(self.text(element)?)).map(Value::DateTime)
            }
            Kind::Duration => {
                let text = self.text(element)?;
                time::validate_duration(text).map_err(|_| Error::decode("Invalid DURATION.", 0))?;
                Ok(Value::text(text))
            }
            Kind::Choice(members) => {
                let Some(inner) = element.children.first() else {
                    return decode_error("Expected an alternative of the CHOICE.");
                };

                match members.all().find(|member| member.name == inner.name) {
                    Some(member) => Ok(Value::Choice(
                        member.name.clone(),
                        self.decode(member.ty, inner, depth + 1)
                            .map_err(|error| error.in_member(&member.name))?
                            .into(),
                    )),
                    None if members.additions.is_some() => {
                        Ok(Value::Choice(String::new(), Value::Null.into()))
                    }
                    None => decode_error("Expected a name of an alternative of the CHOICE."),
                }
            }
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => Err(Error::unsupported(
                "XER needs the type of the value of an ANY or an open type.",
            )),
        }
    }
}
