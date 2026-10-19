// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! JSON Encoding Rules (JER), X.697.
//!
//! A SEQUENCE and a SET are objects, a CHOICE an object with one name,
//! a SEQUENCE OF and a SET OF arrays, an OCTET STRING and a BIT STRING
//! hex in upper case, an INTEGER a number and the strings strings. What
//! is written is compact and with ASCII only, as `json.dumps` of the
//! runtime for Python writes it.

use alloc::string::String;
use alloc::vec::Vec;

use crate::decimal;
use crate::json::{self, Json};
use crate::time::{self, iso};
use crate::types::{Kind, MemberInfo, Members, TypeId, Types, clean_bits};
use crate::value::Value;
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

fn decode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::decode(what, 0))
}

/// Encodes `value` as the type `id`.
pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    let mut out = String::new();

    Coder { types }.encode(id, value, &mut out, 0)?;

    Ok(out.into_bytes())
}

/// Encodes `value` as the type `id`, with `indent` spaces for each
/// level.
pub fn encode_indented(types: &Types, id: TypeId, value: &Value, indent: usize) -> Result<Vec<u8>> {
    let mut out = String::new();

    Coder { types }.encode(id, value, &mut out, 0)?;

    Ok(json::indent(&out, indent).into_bytes())
}

/// Decodes `data` as the type `id`.
pub fn decode(types: &Types, id: TypeId, data: &[u8]) -> Result<Value> {
    let Ok(text) = core::str::from_utf8(data) else {
        return decode_error("Expected UTF-8.");
    };

    Coder { types }.decode(id, &json::parse(text)?, 0)
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

impl Coder<'_> {
    /// The size of a BIT STRING that has one size only.
    fn fixed_size(&self, id: TypeId) -> Option<usize> {
        let size = self.types.node(id).effective_size?;

        match (size.min, size.max) {
            (Some(min), Some(max)) if min == max => usize::try_from(min).ok(),
            _ => None,
        }
    }

    fn write_integer(&self, value: &Value, out: &mut String) -> Result<()> {
        let text = match value {
            Value::Integer(number) => alloc::format!("{number}"),
            Value::BigInteger(bytes) => match decimal::twos_complement_to_decimal(bytes) {
                Some(text) => text,
                None => return encode_error("The integer has too many digits."),
            },
            _ => return encode_error("Expected an integer."),
        };

        out.push_str(&text);

        Ok(())
    }

    fn encode(&self, id: TypeId, value: &Value, out: &mut String, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return encode_error("The value is nested too deeply.");
        }

        let kind = self.types.kind(id);

        match kind {
            Kind::Ref(_) => encode_error("The type has no contents of its own."),
            Kind::Boolean => match value {
                Value::Boolean(value) => {
                    out.push_str(if *value { "true" } else { "false" });

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
            Kind::Real => match value {
                Value::Real(value) => {
                    // The special values are strings (X.697 23.2).
                    out.push_str(&if value.is_nan() {
                        String::from("\"NaN\"")
                    } else if value.is_infinite() {
                        String::from(if *value > 0.0 { "\"INF\"" } else { "\"-INF\"" })
                    } else if *value == 0.0 && value.is_sign_negative() {
                        String::from("\"-0\"")
                    } else {
                        decimal::float_repr(*value)
                    });

                    Ok(())
                }
                Value::Integer(_) | Value::BigInteger(_) => self.write_integer(value, out),
                _ => encode_error("Expected data of type float or int."),
            },
            Kind::Null => match value {
                Value::Null => {
                    out.push_str("null");

                    Ok(())
                }
                _ => encode_error("Expected None."),
            },
            Kind::BitString(named) => {
                let Value::Bits(bytes, bits) = value else {
                    return encode_error("Expected data of type tuple(bytes, int).");
                };

                if bytes.len() * 8 < *bits {
                    return encode_error("Expected more bits of data.");
                }

                // The bytes of the bits, with those past them 0 (X.697
                // 24.2.1).
                let (mut bytes, mut bits) = clean_bits(bytes, *bits, false);

                match self.fixed_size(id) {
                    Some(size) => {
                        // With named bits, trailing 0 bits are added or
                        // removed to make the size (X.697 24.2.2).
                        if !named.is_empty() {
                            let (trimmed, length) = clean_bits(&bytes, bits, true);

                            if length <= size {
                                bytes = trimmed;
                                bytes.resize(size.div_ceil(8), 0);
                                bits = size;
                            }
                        }

                        if bits != size {
                            return encode_error("The BIT STRING is not of its size.");
                        }

                        json::write_string(out, &decimal::hex_upper(&bytes));
                    }
                    None => {
                        out.push_str("{\"value\":");
                        json::write_string(out, &decimal::hex_upper(&bytes));
                        out.push_str(",\"length\":");
                        out.push_str(&alloc::format!("{bits}"));
                        out.push('}');
                    }
                }

                Ok(())
            }
            Kind::OctetString => match value {
                Value::Bytes(bytes) => {
                    json::write_string(out, &decimal::hex_upper(bytes));

                    Ok(())
                }
                _ => encode_error("Expected data of type bytes or bytearray."),
            },
            Kind::ObjectIdentifier | Kind::Str(_) => match value {
                Value::Text(text) => {
                    json::write_string(out, text);

                    Ok(())
                }
                _ => encode_error("Expected data of type str."),
            },
            Kind::Enumerated { root, additions } => {
                match value {
                    Value::Text(name) if !self.types.numeric_enums => {
                        if !names(root, additions).any(|(candidate, _)| candidate == name) {
                            return encode_error("Expected an enumeration value.");
                        }

                        json::write_string(out, name);
                    }
                    // The name, also of a value given as its number (X.697 22).
                    Value::Integer(number) if self.types.numeric_enums => {
                        let Some((name, _)) = names(root, additions)
                            .find(|(_, candidate)| i128::from(*candidate) == *number)
                        else {
                            return encode_error("Expected an enumeration value.");
                        };

                        json::write_string(out, name);
                    }
                    _ => return encode_error("Expected an enumeration value."),
                }

                Ok(())
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                self.encode_members(members, value, out, depth)
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let Value::List(items) = value else {
                    return encode_error("Expected data of type list.");
                };

                out.push('[');

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push(',');
                    }

                    self.encode(*element, item, out, depth + 1)
                        .map_err(|error| error.in_element(index))?;
                }

                out.push(']');

                Ok(())
            }
            Kind::Utc => match value {
                Value::DateTime(value) => {
                    json::write_string(out, &time::utc_time_to_string(value, false));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => {
                    json::write_string(out, &time::generalized_time_to_string(value, false));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Date => match value {
                Value::Date(value) => {
                    json::write_string(out, &iso::date_to_string(value));

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.date."),
            },
            Kind::TimeOfDay => match value {
                Value::Time(value) => {
                    json::write_string(out, &iso::time_to_string(value)?);

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.time."),
            },
            Kind::DateTime => match value {
                Value::DateTime(value) => {
                    json::write_string(out, &iso::date_time_to_string(value)?);

                    Ok(())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Duration => match value {
                Value::Text(text) => {
                    time::validate_duration(text)?;
                    json::write_string(out, text);
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

                out.push('{');
                json::write_string(out, name);
                out.push(':');
                self.encode(member.ty, inner, out, depth + 1)
                    .map_err(|error| error.in_member(name))?;
                out.push('}');

                Ok(())
            }
            // The value of the type it is (X.697 41), which only an ANY
            // DEFINED BY with choices tells, as a member.
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => Err(Error::unsupported(
                "JER needs the type of the value of an ANY or an open type.",
            )),
        }
    }

    fn encode_members(
        &self,
        members: &Members,
        value: &Value,
        out: &mut String,
        depth: usize,
    ) -> Result<()> {
        let Value::Map(map) = value else {
            return encode_error("Expected data of type dict.");
        };
        let mut first = true;

        out.push('{');

        for member in members.all() {
            match map.iter().find(|(name, _)| name == &member.name) {
                Some((_, value)) => {
                    if !first {
                        out.push(',');
                    }

                    first = false;
                    json::write_string(out, &member.name);
                    out.push(':');
                    let ty = self
                        .member_type(member, map)
                        .ok_or_else(|| Error::encode("Bad AnyDefinedBy choice."))?;

                    self.encode(ty, value, out, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?;
                }
                None if member.optional || member.has_default() => {}
                None => return encode_error("A member is not found in the SEQUENCE or SET."),
            }
        }

        out.push('}');

        Ok(())
    }

    fn decode(&self, id: TypeId, json: &Json, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.");
        }

        let kind = self.types.kind(id);

        match kind {
            Kind::Ref(_) => decode_error("The type has no contents of its own."),
            Kind::Boolean => match json {
                Json::Bool(value) => Ok(Value::Boolean(*value)),
                _ => decode_error("Expected a JSON boolean."),
            },
            Kind::Integer(_) => match json {
                Json::Int(value) => Ok(value.clone()),
                _ => decode_error("Expected a JSON integer."),
            },
            Kind::Real => match json {
                Json::Float(value) => Ok(Value::Real(*value)),
                Json::Int(Value::Integer(value)) => Ok(Value::Real(*value as f64)),
                Json::Int(Value::BigInteger(bytes)) => {
                    Ok(Value::Real(crate::ber::twos_complement_to_f64(bytes)?))
                }
                Json::Str(text) => match text.as_str() {
                    "INF" => Ok(Value::Real(f64::INFINITY)),
                    "-INF" => Ok(Value::Real(f64::NEG_INFINITY)),
                    "NaN" => Ok(Value::Real(f64::NAN)),
                    "-0" => Ok(Value::Real(-0.0)),
                    _ => decode_error("Expected a JSON number or INF, -INF, NaN or -0."),
                },
                _ => decode_error("Expected a JSON number."),
            },
            Kind::Null => match json {
                Json::Null => Ok(Value::Null),
                _ => decode_error("Expected JSON null."),
            },
            Kind::BitString(_) => {
                let (hex, bits) = match (self.fixed_size(id), json) {
                    (Some(size), Json::Str(hex)) => (hex, size),
                    (None, Json::Object(_)) => {
                        let (Some(Json::Str(hex)), Some(Json::Int(Value::Integer(length)))) =
                            (json.get("value"), json.get("length"))
                        else {
                            return decode_error("Expected a value and a length of a BIT STRING.");
                        };
                        let Ok(length) = usize::try_from(*length) else {
                            return decode_error("Bad length of a BIT STRING.");
                        };

                        (hex, length)
                    }
                    _ => return decode_error("Expected a BIT STRING."),
                };
                let Some(bytes) = decimal::hex_decode(hex) else {
                    return decode_error("Expected hex.");
                };

                if bytes.len() * 8 < bits || bytes.len() > bits.div_ceil(8) {
                    return decode_error("The length of a BIT STRING is not that of its value.");
                }

                Ok(Value::Bits(bytes, bits))
            }
            Kind::OctetString => match json {
                Json::Str(hex) => match decimal::hex_decode(hex) {
                    Some(bytes) => Ok(Value::Bytes(bytes)),
                    None => decode_error("Expected hex."),
                },
                _ => decode_error("Expected a JSON string."),
            },
            Kind::ObjectIdentifier | Kind::Str(_) => match json {
                Json::Str(text) => Ok(Value::Text(text.clone())),
                _ => decode_error("Expected a JSON string."),
            },
            Kind::Enumerated { root, additions } => {
                let found = match json {
                    Json::Str(name) => names(root, additions)
                        .find(|(candidate, _)| candidate == name)
                        .map(|(name, number)| {
                            if self.types.numeric_enums {
                                Value::Integer(i128::from(*number))
                            } else {
                                Value::Text(name.clone())
                            }
                        }),
                    _ => None,
                };

                match found {
                    Some(value) => Ok(value),
                    None if additions.is_some() => Ok(Value::Null),
                    None => decode_error("Expected an enumeration value."),
                }
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                self.decode_members(members, json, depth)
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let Json::Array(items) = json else {
                    return decode_error("Expected a JSON array.");
                };

                items
                    .iter()
                    .enumerate()
                    .map(|(index, item)| {
                        self.decode(*element, item, depth + 1)
                            .map_err(|error| error.in_element(index))
                    })
                    .collect::<Result<Vec<_>>>()
                    .map(Value::List)
            }
            Kind::Utc => match json {
                Json::Str(text) => {
                    time_error(time::utc_time_from_string(text, false)).map(Value::DateTime)
                }
                _ => decode_error("Expected a JSON string."),
            },
            Kind::Generalized => match json {
                Json::Str(text) => {
                    time_error(time::generalized_time_from_string(text, false)).map(Value::DateTime)
                }
                _ => decode_error("Expected a JSON string."),
            },
            Kind::Date => match json {
                Json::Str(text) => time_error(iso::date_from_string(text)).map(Value::Date),
                _ => decode_error("Expected a JSON string."),
            },
            Kind::TimeOfDay => match json {
                Json::Str(text) => time_error(iso::time_from_string(text)).map(Value::Time),
                _ => decode_error("Expected a JSON string."),
            },
            Kind::DateTime => match json {
                Json::Str(text) => {
                    time_error(iso::date_time_from_string(text)).map(Value::DateTime)
                }
                _ => decode_error("Expected a JSON string."),
            },
            Kind::Duration => match json {
                Json::Str(text) => {
                    time::validate_duration(text)
                        .map_err(|_| Error::decode("Invalid DURATION.", 0))?;
                    Ok(Value::Text(text.clone()))
                }
                _ => decode_error("Expected a JSON string."),
            },
            Kind::Choice(members) => {
                let Json::Object(entries) = json else {
                    return decode_error("Expected a JSON object.");
                };
                let Some((name, inner)) = entries.first() else {
                    return decode_error("Expected an alternative of the CHOICE.");
                };

                match members.all().find(|member| &member.name == name) {
                    Some(member) => Ok(Value::Choice(
                        name.clone(),
                        self.decode(member.ty, inner, depth + 1)
                            .map_err(|error| error.in_member(name))?
                            .into(),
                    )),
                    None if members.additions.is_some() => {
                        Ok(Value::Choice(String::new(), Value::Null.into()))
                    }
                    None => decode_error("Expected a name of an alternative of the CHOICE."),
                }
            }
            // The value of the type it is (X.697 41), which only an ANY
            // DEFINED BY with choices tells, as a member.
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => Err(Error::unsupported(
                "JER needs the type of the value of an ANY or an open type.",
            )),
        }
    }

    fn decode_members(&self, members: &Members, json: &Json, depth: usize) -> Result<Value> {
        if !matches!(json, Json::Object(_)) {
            return decode_error("Expected a JSON object.");
        }

        let mut map = Vec::new();

        for member in members.all() {
            match json.get(&member.name) {
                Some(inner) => {
                    let ty = self
                        .member_type(member, &map)
                        .ok_or_else(|| Error::decode("Bad AnyDefinedBy choice.", 0))?;

                    let value = self
                        .decode(ty, inner, depth + 1)
                        .map_err(|error| error.in_member(&member.name))?;

                    map.push((member.name.clone(), value));
                }
                None => self.missing(member, &mut map)?,
            }
        }

        Ok(Value::Map(map))
    }

    /// The type of a member, which for an ANY DEFINED BY with choices is
    /// that of the choice of the value of the member it names, if there is
    /// one.
    fn member_type(&self, member: &MemberInfo, map: &[(String, Value)]) -> Option<TypeId> {
        self.types
            .defined_by(member.ty, map)
            .unwrap_or(Some(member.ty))
    }

    fn missing(&self, member: &MemberInfo, map: &mut Vec<(String, Value)>) -> Result<()> {
        if member.optional {
            Ok(())
        } else if let Some(default) = &member.default {
            map.push((member.name.clone(), default.clone()));

            Ok(())
        } else {
            decode_error("A member is not found in the object.")
        }
    }
}

fn time_error<T>(result: Result<T>) -> Result<T> {
    result.map_err(|_| Error::decode("Expected a date or a time.", 0))
}
