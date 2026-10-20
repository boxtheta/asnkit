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
//! Decoding accepts both bare values and the assignments written above.
//! Whitespace includes line breaks so indented output can be decoded too.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use crate::decimal;
use crate::time::{self, iso};
use crate::types::{Addition, Kind, MemberInfo, Members, TypeId, Types};
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
fn member_type(
    types: &Types,
    member: &MemberInfo,
    map: &[(Cow<'_, str>, Value<'_>)],
) -> Option<TypeId> {
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

/// Decodes a bare GSER value or the value assignment written by [`encode`].
/// Errors carry byte offsets into `data`. Trailing data is rejected.
pub fn decode<'a>(types: &'a Types, id: TypeId, data: &[u8]) -> Result<Value<'a>> {
    let text = core::str::from_utf8(data)
        .map_err(|error| Error::decode("Expected UTF-8 GSER data.", error.valid_up_to()))?;
    let mut decoder = Decoder {
        types,
        text,
        pos: 0,
    };
    // An assignment starts with two words and ::=; bare values never do.
    let start = decoder.pos;
    if decoder.token().is_ok() {
        if let Ok(name) = decoder.token() {
            if decoder.take("::=") {
                if !types
                    .modules
                    .iter()
                    .flat_map(|(_, names)| names.iter())
                    .any(|(candidate, ty)| candidate == name && *ty == id)
                {
                    return decoder.error("The assignment has a different type.");
                }
            } else {
                decoder.pos = start;
            }
        } else {
            decoder.pos = start;
        }
    }
    let value = decoder.value(id, 0)?;
    decoder.space();
    if decoder.pos != text.len() {
        return decoder.error("Unexpected data after the GSER value.");
    }
    Ok(value)
}

// The decimal forms in RFC 3641 have no leading plus or redundant zeroes.
fn integer_token(text: &str) -> bool {
    if text == "0" {
        return true;
    }
    let digits = text.strip_prefix('-').unwrap_or(text);
    !digits.is_empty() && !digits.starts_with('0') && digits.bytes().all(|b| b.is_ascii_digit())
}

fn real_token(text: &str) -> bool {
    if text == "0" {
        return true;
    }
    let text = text.strip_prefix('-').unwrap_or(text);
    let Some((mantissa, exponent)) = text.split_once('E') else {
        return false;
    };
    if !integer_token(exponent) {
        return false;
    }
    let (whole, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
    integer_token(whole)
        && !whole.starts_with('-')
        && fraction.bytes().all(|b| b.is_ascii_digit())
        && mantissa.bytes().any(|b| matches!(b, b'1'..=b'9'))
}

struct Decoder<'a, 't> {
    types: &'a Types,
    text: &'t str,
    pos: usize,
}

impl<'a, 't> Decoder<'a, 't> {
    fn error<T>(&self, message: &'static str) -> Result<T> {
        Err(Error::decode(message, self.pos))
    }

    fn space(&mut self) {
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(u8::is_ascii_whitespace)
        {
            self.pos += 1;
        }
    }

    fn take(&mut self, token: &str) -> bool {
        self.space();
        if self.text[self.pos..].starts_with(token) {
            self.pos += token.len();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: &str) -> Result<()> {
        if self.take(token) {
            Ok(())
        } else {
            self.error("Expected GSER punctuation.")
        }
    }

    fn token(&mut self) -> Result<&'t str> {
        self.space();
        let start = self.pos;
        while self
            .text
            .as_bytes()
            .get(self.pos)
            .is_some_and(|b| b.is_ascii_alphanumeric() || matches!(b, b'-' | b'+' | b'.'))
        {
            self.pos += 1;
        }
        if start == self.pos {
            return self.error("Expected a GSER token.");
        }
        Ok(&self.text[start..self.pos])
    }

    fn string(&mut self) -> Result<String> {
        self.expect("\"")?;
        let mut out = String::new();
        loop {
            let start = self.pos;
            let Some(end) = self.text[start..].find('"') else {
                return self.error("Unterminated GSER string.");
            };
            out.push_str(&self.text[start..start + end]);
            self.pos = start + end + 1;
            if self.text.as_bytes().get(self.pos) != Some(&b'"') {
                return Ok(out);
            }
            self.pos += 1;
            out.push('"');
        }
    }

    fn literal(&mut self) -> Result<(Vec<u8>, usize, u8)> {
        self.expect("'")?;
        let start = self.pos;
        let Some(end) = self.text[start..].find('\'') else {
            return self.error("Unterminated GSER bit or hex string.");
        };
        self.pos = start + end + 1;
        let radix = match self.text.as_bytes().get(self.pos) {
            Some(b'B') => 2,
            Some(b'H') => 16,
            _ => return self.error("Expected B or H after a GSER literal."),
        };
        self.pos += 1;
        let width = if radix == 2 { 1 } else { 4 };
        let bits = end * width;
        let mut bytes = alloc::vec![0; bits.div_ceil(8)];
        for (index, digit) in self.text[start..start + end].bytes().enumerate() {
            let Some(number) = (digit as char).to_digit(radix) else {
                return Err(Error::decode(
                    "Invalid digit in GSER literal.",
                    start + index,
                ));
            };
            bytes[index * width / 8] |= (number as u8) << (8 - width - index * width % 8);
        }
        Ok((bytes, bits, radix as u8))
    }

    fn value(&mut self, id: TypeId, depth: usize) -> Result<Value<'a>> {
        self.space();
        if depth > MAX_DEPTH {
            return self.error("The value is nested too deeply.");
        }
        match self.types.kind(id) {
            Kind::Ref(_) => self.error("The type has no contents of its own."),
            Kind::Boolean => match self.token()? {
                "TRUE" => Ok(Value::Boolean(true)),
                "FALSE" => Ok(Value::Boolean(false)),
                _ => self.error("Expected TRUE or FALSE."),
            },
            Kind::Null => {
                if self.token()? == "NULL" {
                    Ok(Value::Null)
                } else {
                    self.error("Expected NULL.")
                }
            }
            Kind::Integer(named) => {
                let token = self.token()?;
                if let Some((_, number)) = named.iter().find(|(name, _)| name == token) {
                    return Ok(Value::Integer(*number));
                }
                if !integer_token(token) {
                    return self.error("Expected an integer or a named number.");
                }
                decimal::decimal_to_value(token).ok_or_else(|| {
                    Error::decode("Expected an integer or a named number.", self.pos)
                })
            }
            Kind::Real if self.text[self.pos..].starts_with('{') => self.real_components(),
            Kind::Real => {
                let token = self.token()?;
                let number = match token {
                    "PLUS-INFINITY" => f64::INFINITY,
                    "MINUS-INFINITY" => f64::NEG_INFINITY,
                    _ if !real_token(token) => return self.error("Expected a GSER real number."),
                    _ => match token.parse::<f64>() {
                        Ok(value) if value.is_finite() => value,
                        _ => return self.error("Expected a GSER real number."),
                    },
                };
                Ok(Value::Real(number))
            }
            Kind::BitString(named) if self.text[self.pos..].starts_with('{') => {
                self.expect("{")?;
                let mut bytes = Vec::new();
                let mut bits = 0;
                if !self.take("}") {
                    loop {
                        let token = self.token()?;
                        let Some((_, index)) = named.iter().find(|(name, _)| name == token) else {
                            return self.error("Unknown named bit.");
                        };
                        let index = usize::try_from(*index)
                            .map_err(|_| Error::decode("Invalid named bit index.", self.pos))?;
                        // Do not allocate arbitrarily large buffers from a schema bit index.
                        if index > self.text.len().saturating_mul(8).max(1_048_576) {
                            return self.error("Named bit index is too large.");
                        }
                        bytes.resize(bytes.len().max(index / 8 + 1), 0);
                        let mask = 0x80 >> (index % 8);
                        if bytes[index / 8] & mask != 0 {
                            return self.error("Duplicate named bit.");
                        }
                        bytes[index / 8] |= mask;
                        bits = bits.max(index + 1);
                        if self.take("}") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(Value::Bits(bytes, bits))
            }
            Kind::BitString(_) => {
                let (bytes, bits, _) = self.literal()?;
                Ok(Value::Bits(bytes, bits))
            }
            Kind::OctetString | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                let (bytes, _, radix) = self.literal()?;
                if radix != 16 {
                    return self.error("Expected a hexadecimal octet string.");
                }
                Ok(Value::Bytes(bytes))
            }
            Kind::ObjectIdentifier => {
                let token = self.token()?;
                if token.split('.').count() < 2
                    || !token.split('.').all(|arc| {
                        !arc.is_empty()
                            && arc.bytes().all(|b| b.is_ascii_digit())
                            && (arc == "0" || !arc.starts_with('0'))
                    })
                {
                    return self.error("Expected a dotted object identifier.");
                }
                Ok(Value::Text(String::from(token)))
            }
            Kind::Str(_) => Ok(Value::Text(self.string()?)),
            Kind::Enumerated { root, additions } => {
                let token = self.token()?;
                let Some((name, number)) = names(root, additions).find(|(name, _)| name == token)
                else {
                    return self.error("Unknown enumeration value.");
                };
                Ok(if self.types.numeric_enums {
                    Value::Integer(i128::from(*number))
                } else {
                    Value::Text(name.clone())
                })
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                self.members(members, depth)
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                self.expect("{")?;
                let mut items = Vec::new();
                if !self.take("}") {
                    loop {
                        items.push(
                            self.value(*element, depth + 1)
                                .map_err(|error| error.in_element(items.len()))?,
                        );
                        if self.take("}") {
                            break;
                        }
                        self.expect(",")?;
                    }
                }
                Ok(Value::List(items))
            }
            Kind::Choice(members) => {
                let name = self.token()?;
                let Some(member) = members.all().find(|member| member.name == name) else {
                    return self.error("Unknown CHOICE alternative.");
                };
                self.expect(":")?;
                let value = self
                    .value(member.ty, depth + 1)
                    .map_err(|error| error.in_member(name))?;
                Ok(Value::Choice(
                    Cow::Borrowed(member.name.as_str()),
                    alloc::boxed::Box::new(value),
                ))
            }
            kind @ (Kind::Utc
            | Kind::Generalized
            | Kind::Date
            | Kind::TimeOfDay
            | Kind::DateTime
            | Kind::Duration) => {
                let start = self.pos;
                let text = self.string()?;
                let result = match kind {
                    Kind::Utc => time::utc_time_from_string(&text, false).map(Value::DateTime),
                    Kind::Generalized => {
                        time::generalized_time_from_string(&text, false).map(Value::DateTime)
                    }
                    Kind::Date => iso::date_from_string(&text).map(Value::Date),
                    Kind::TimeOfDay => iso::time_from_string(&text).map(Value::Time),
                    Kind::DateTime => iso::date_time_from_string(&text).map(Value::DateTime),
                    _ => time::validate_duration(&text).map(|()| Value::Text(text)),
                };
                result.map_err(|_| Error::decode("Expected a valid date, time or duration.", start))
            }
        }
    }

    // The ASN.1 REAL value notation, { mantissa INTEGER, base 2|10,
    // exponent INTEGER }, is also a GSER representation (RFC 3641 3.19).
    fn real_components(&mut self) -> Result<Value<'static>> {
        self.expect("{")?;
        if self.token()? != "mantissa" {
            return self.error("Expected the REAL mantissa.");
        }
        let mantissa = self.token()?;
        if !integer_token(mantissa) {
            return self.error("Expected an integer mantissa.");
        }
        let integer = decimal::decimal_to_value(mantissa)
            .ok_or_else(|| Error::decode("Expected an integer mantissa.", self.pos))?;
        self.expect(",")?;
        if self.token()? != "base" {
            return self.error("Expected the REAL base.");
        }
        let base = self.token()?;
        if base != "2" && base != "10" {
            return self.error("Expected REAL base 2 or 10.");
        }
        self.expect(",")?;
        if self.token()? != "exponent" {
            return self.error("Expected the REAL exponent.");
        }
        let exponent = self.token()?;
        if !integer_token(exponent) {
            return self.error("Expected an integer exponent.");
        }
        let exponent = exponent
            .parse::<i64>()
            .map_err(|_| Error::decode("Expected an integer exponent.", self.pos))?;
        self.expect("}")?;
        let number = if base == "10" {
            alloc::format!("{mantissa}E{exponent}")
                .parse::<f64>()
                .map_err(|_| Error::decode("Invalid REAL value.", self.pos))?
        } else {
            let mut magnitude = integer.to_twos_complement().unwrap();
            let negative = magnitude.first().is_some_and(|b| b & 0x80 != 0);
            if negative {
                let mut carry = true;
                for byte in magnitude.iter_mut().rev() {
                    *byte = !*byte;
                    if carry {
                        let (sum, overflow) = byte.overflowing_add(1);
                        *byte = sum;
                        carry = overflow;
                    }
                }
            }
            // Let the BER REAL reader do the rounding, including subnormals.
            let exponent = crate::value::integer_to_twos_complement(i128::from(exponent));
            let form = (exponent.len() - 1).min(3) as u8;
            let mut binary = alloc::vec![(if negative { 0xc0 } else { 0x80 }) | form];
            if form == 3 {
                binary.push(exponent.len() as u8);
            }
            binary.extend_from_slice(&exponent);
            binary.extend_from_slice(&magnitude);
            crate::ber::decode_real(&binary, self.pos, false)?
        };
        if !number.is_finite() {
            return self.error("The REAL is too large for a float.");
        }
        Ok(Value::Real(number))
    }

    fn members(&mut self, members: &'a Members, depth: usize) -> Result<Value<'a>> {
        self.expect("{")?;
        let mut map = Vec::new();
        let mut next = 0;
        let count = members.defined().count();
        let mut present = alloc::vec![false; count];
        let additions_start = members.root.len() - members.root_after_additions;
        let additions_end = count - members.root_after_additions;
        let is_addition = |index| (additions_start..additions_end).contains(&index);
        if !self.take("}") {
            loop {
                let name = self.token()?;
                let Some((index, member)) = members
                    .defined()
                    .enumerate()
                    .find(|(_, member)| member.name == name)
                else {
                    return self.error("Unknown SEQUENCE or SET member.");
                };
                if index < next {
                    return self.error("Duplicate or out-of-order member.");
                }
                for (index, missing) in members.defined().enumerate().skip(next).take(index - next)
                {
                    self.missing(missing, &mut map, is_addition(index))?;
                }
                present[index] = true;
                next = index + 1;
                let before = self.pos;
                self.space();
                if self.pos == before {
                    return self.error("Expected whitespace after a member name.");
                }
                let ty = member_type(self.types, member, &map).ok_or_else(|| {
                    Error::decode("Bad AnyDefinedBy choice.", self.pos).in_member(name)
                })?;
                let value = self
                    .value(ty, depth + 1)
                    .map_err(|error| error.in_member(name))?;
                map.push((Cow::Borrowed(member.name.as_str()), value));
                if self.take("}") {
                    break;
                }
                self.expect(",")?;
            }
        }
        for (index, member) in members.defined().enumerate().skip(next) {
            self.missing(member, &mut map, is_addition(index))?;
        }
        // A whole extension group may be absent. Once any of its members
        // is supplied, its required members must also be present.
        let mut index = additions_start;
        for addition in members.additions.iter().flatten() {
            match addition {
                Addition::Member(_) => index += 1,
                Addition::Group(group) => {
                    let supplied = &present[index..index + group.len()];
                    if supplied.iter().any(|present| *present) {
                        for (member, present) in group.iter().zip(supplied) {
                            if !present && !member.optional && !member.has_default() {
                                return Err(Error::decode(
                                    "A required member is missing.",
                                    self.pos,
                                )
                                .in_member(&member.name));
                            }
                        }
                    }
                    index += group.len();
                }
            }
        }
        Ok(Value::Map(map))
    }

    fn missing(
        &self,
        member: &'a MemberInfo,
        map: &mut Vec<(Cow<'a, str>, Value<'a>)>,
        addition: bool,
    ) -> Result<()> {
        if let Some(default) = &member.default {
            map.push((Cow::Borrowed(member.name.as_str()), default.clone()));
        } else if !member.optional && !addition {
            return Err(
                Error::decode("A required member is missing.", self.pos).in_member(&member.name)
            );
        }
        Ok(())
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
