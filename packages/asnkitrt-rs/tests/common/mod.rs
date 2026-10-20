// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The vectors in `tests/vectors`, shared by the test that holds this
//! runtime to them and by `examples/record_vectors.rs`, which makes
//! them. `tests/vectors/README.rst` tells how a vector is written.

// Each user takes what it needs.
#![allow(dead_code)]

use std::fs;
use std::path::PathBuf;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::{Codec, Kind, TypeId, Types};
use asnkitrt::value::{Date, DateTime, ObjectIdentifier, Time, Value};
use serde_json::Value as Json;

pub fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/vectors")
}

pub fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

pub fn hex_of(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

/// An integer written in decimal as its two's complement.
fn big_integer(text: &str) -> Value<'static> {
    let negative = text.starts_with('-');
    let mut digits: Vec<u8> = text
        .trim_start_matches('-')
        .bytes()
        .map(|digit| digit - b'0')
        .collect();
    let mut magnitude = Vec::new();

    // Divide by 256 over and over.
    while !digits.is_empty() {
        let mut remainder = 0u32;
        let mut quotient = Vec::new();

        for digit in &digits {
            let current = remainder * 10 + u32::from(*digit);

            quotient.push((current / 256) as u8);
            remainder = current % 256;
        }

        magnitude.push(remainder as u8);
        digits = quotient
            .into_iter()
            .skip_while(|digit| *digit == 0)
            .collect();
    }

    magnitude.push(0);
    magnitude.reverse();

    if negative {
        negate(&mut magnitude);
    }

    Value::from_twos_complement(&magnitude)
}

/// Invert and add one.
fn negate(bytes: &mut [u8]) {
    let mut carry = true;

    for byte in bytes.iter_mut().rev() {
        *byte = !*byte;

        if carry {
            let (sum, overflow) = byte.overflowing_add(1);
            *byte = sum;
            carry = overflow;
        }
    }
}

/// A two's complement written in decimal.
fn decimal(twos_complement: &[u8]) -> String {
    let mut magnitude = Vec::from(twos_complement);
    let negative = magnitude.first().is_some_and(|byte| byte & 0x80 != 0);

    if negative {
        negate(&mut magnitude);
    }

    // Digits, least significant first, multiplied by 256 and added to
    // for each byte.
    let mut digits = vec![0u32];

    for byte in magnitude {
        let mut carry = u32::from(byte);

        for digit in digits.iter_mut() {
            let current = *digit * 256 + carry;

            *digit = current % 10;
            carry = current / 10;
        }

        while carry > 0 {
            digits.push(carry % 10);
            carry /= 10;
        }
    }

    while digits.len() > 1 && digits.last() == Some(&0) {
        digits.pop();
    }

    let mut text: String = if negative { "-".into() } else { String::new() };

    text.extend(
        digits
            .iter()
            .rev()
            .map(|digit| char::from(b'0' + *digit as u8)),
    );

    text
}

fn parse_date(text: &str) -> Date {
    Date {
        year: text[0..4].parse().unwrap(),
        month: text[5..7].parse().unwrap(),
        day: text[8..10].parse().unwrap(),
    }
}

fn parse_time(text: &str) -> Time {
    let mut time = Time {
        hour: text[0..2].parse().unwrap(),
        minute: text[3..5].parse().unwrap(),
        second: text[6..8].parse().unwrap(),
        microsecond: 0,
        offset: None,
    };
    let mut rest = &text[8..];

    if let Some(fraction) = rest.strip_prefix('.') {
        let digits: String = fraction.chars().take_while(char::is_ascii_digit).collect();

        time.microsecond = digits.parse().unwrap();
        rest = &fraction[digits.len()..];
    }

    if rest.len() >= 6 {
        let sign = if rest.starts_with('-') { -1 } else { 1 };
        let hours: i32 = rest[1..3].parse().unwrap();
        let minutes: i32 = rest[4..6].parse().unwrap();

        time.offset = Some(sign * (hours * 60 + minutes));
    }

    time
}

/// As `date.isoformat()` in Python.
fn format_date(date: &Date) -> String {
    format!("{:04}-{:02}-{:02}", date.year, date.month, date.day)
}

/// As `time.isoformat()` in Python.
fn format_time(time: &Time) -> String {
    let mut text = format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second);

    if time.microsecond != 0 {
        text += &format!(".{:06}", time.microsecond);
    }

    if let Some(offset) = time.offset {
        let sign = if offset < 0 { '-' } else { '+' };
        let offset = offset.abs();

        text += &format!("{sign}{:02}:{:02}", offset / 60, offset % 60);
    }

    text
}

pub fn to_value(json: &Json) -> Value<'static> {
    match json {
        Json::Null => Value::Null,
        Json::Bool(value) => Value::Boolean(*value),
        Json::Number(number) => {
            let text = number.to_string();

            if text.contains(['.', 'e', 'E']) {
                Value::Real(text.parse().unwrap())
            } else if let Ok(value) = text.parse::<i128>() {
                Value::Integer(value)
            } else {
                big_integer(&text)
            }
        }
        Json::String(text) => Value::Text(text.clone().into()),
        Json::Array(items) => Value::List(items.iter().map(to_value).collect()),
        Json::Object(members) => {
            if members.len() == 1 {
                let (key, value) = members.iter().next().unwrap();

                match key.as_str() {
                    "$bytes" => return Value::Bytes(hex(value.as_str().unwrap()).into()),
                    "$bits" => {
                        let parts = value.as_array().unwrap();

                        return Value::Bits(
                            hex(parts[0].as_str().unwrap()).into(),
                            parts[1].as_u64().unwrap() as usize,
                        );
                    }
                    "$choice" => {
                        let parts = value.as_array().unwrap();

                        return Value::Choice(
                            parts[0].as_str().unwrap().to_string().into(),
                            to_value(&parts[1]).into(),
                        );
                    }
                    "$float" => {
                        return Value::Real(match value.as_str().unwrap() {
                            "inf" => f64::INFINITY,
                            "-inf" => f64::NEG_INFINITY,
                            "-0.0" => -0.0,
                            _ => f64::NAN,
                        });
                    }
                    "$datetime" => {
                        let text = value.as_str().unwrap();

                        return Value::DateTime(DateTime {
                            date: parse_date(text),
                            time: parse_time(&text[11..]),
                        });
                    }
                    "$date" => return Value::Date(parse_date(value.as_str().unwrap())),
                    "$time" => return Value::Time(parse_time(value.as_str().unwrap())),
                    _ => {}
                }
            }

            Value::Map(
                members
                    .iter()
                    .map(|(name, value)| (name.clone().into(), to_value(value)))
                    .collect(),
            )
        }
    }
}

/// The value of `json` for the type `name` of `spec`, in which the text of
/// an OBJECT IDENTIFIER is one, as the JSON has it as text.
pub fn value_of(spec: &Specification, name: &str, json: &Json) -> Value<'static> {
    let mut value = to_value(json);

    if let Some(id) = spec.types().find_unique(name) {
        type_object_identifiers(spec.types(), id, &mut value, 0);
    }

    value
}

fn type_object_identifiers(types: &Types, id: TypeId, value: &mut Value<'_>, depth: usize) {
    // As deep as the codecs go.
    if depth > 128 {
        return;
    }

    match (types.kind(id), &mut *value) {
        (Kind::ObjectIdentifier, Value::Text(text)) => {
            if let Ok(oid) = text.parse::<ObjectIdentifier<'static>>() {
                *value = Value::ObjectIdentifier(oid);
            }
        }
        (Kind::SequenceOf(element) | Kind::SetOf(element), Value::List(items)) => {
            for item in items {
                type_object_identifiers(types, *element, item, depth + 1);
            }
        }
        (Kind::Choice(members), Value::Choice(name, inner)) => {
            if let Some(member) = members.all().find(|member| member.name == **name) {
                type_object_identifiers(types, member.ty, inner, depth + 1);
            }
        }
        (
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members),
            Value::Map(map),
        ) => {
            let member_of = |name: &str| {
                members
                    .all()
                    .find(|member| member.name == name)
                    .map(|m| m.ty)
            };

            // The members an ANY DEFINED BY is defined by first, as the
            // choice is found by their value.
            for (name, member) in map.iter_mut() {
                if let Some(ty) = member_of(name)
                    && !matches!(types.kind(ty), Kind::AnyDefinedBy { .. })
                {
                    type_object_identifiers(types, ty, member, depth + 1);
                }
            }

            let chosen: Vec<Option<TypeId>> = map
                .iter()
                .map(|(name, _)| {
                    member_of(name).and_then(|ty| types.defined_by(ty, map.iter()).flatten())
                })
                .collect();

            for ((_, member), ty) in map.iter_mut().zip(chosen) {
                if let Some(ty) = ty {
                    type_object_identifiers(types, ty, member, depth + 1);
                }
            }
        }
        _ => {}
    }
}

fn tagged(key: &str, value: Json) -> Json {
    Json::Object([(key.to_string(), value)].into_iter().collect())
}

/// The inverse of `to_value()`.
pub fn to_json(value: &Value) -> Json {
    match value {
        Value::Null => Json::Null,
        Value::Boolean(value) => Json::Bool(*value),
        Value::Integer(value) => Json::Number(value.to_string().parse().unwrap()),
        Value::BigInteger(bytes) => Json::Number(decimal(bytes).parse().unwrap()),
        Value::Real(value) => {
            if value.is_nan() {
                tagged("$float", "nan".into())
            } else if value.is_infinite() {
                tagged("$float", if *value > 0.0 { "inf" } else { "-inf" }.into())
            } else if *value == 0.0 && value.is_sign_negative() {
                tagged("$float", "-0.0".into())
            } else {
                Json::Number(serde_json::Number::from_f64(*value).unwrap())
            }
        }
        Value::Text(text) => Json::String(text.to_string()),
        // An OBJECT IDENTIFIER is text in the JSON, as in Python.
        Value::ObjectIdentifier(oid) => Json::String(oid.to_string()),
        Value::Bytes(bytes) => tagged("$bytes", hex_of(bytes).into()),
        Value::Bits(bytes, length) => tagged(
            "$bits",
            Json::Array(vec![hex_of(bytes).into(), (*length).into()]),
        ),
        Value::List(items) => Json::Array(items.iter().map(to_json).collect()),
        Value::Map(members) => Json::Object(
            members
                .iter()
                .map(|(name, value)| (name.to_string(), to_json(value)))
                .collect(),
        ),
        Value::Choice(name, value) => tagged(
            "$choice",
            Json::Array(vec![name.to_string().into(), to_json(value)]),
        ),
        Value::DateTime(value) => tagged(
            "$datetime",
            format!("{}T{}", format_date(&value.date), format_time(&value.time)).into(),
        ),
        Value::Date(value) => tagged("$date", format_date(value).into()),
        Value::Time(value) => tagged("$time", format_time(value).into()),
    }
}

/// If two values are the same, whatever order the members of a map are
/// in and if a number is not a number.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Real(a), Value::Real(b)) => (a.is_nan() && b.is_nan()) || a == b,
        (Value::Real(a), Value::Integer(b)) | (Value::Integer(b), Value::Real(a)) => {
            *a == *b as f64
        }
        (Value::List(a), Value::List(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| same(a, b))
        }
        (Value::Map(a), Value::Map(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(name, a)| b.iter().any(|(other, b)| name == other && same(a, b)))
        }
        (Value::Choice(a, x), Value::Choice(b, y)) => a == b && same(x, y),
        _ => a == b,
    }
}

fn codec_of(name: &str) -> Codec {
    match name {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        "gser" => Codec::Gser,
        _ => Codec::Uper,
    }
}

/// What a specification is loaded by: its name, the codec, and
/// `numeric_enums` and `lenient`.
pub type SpecKey = (String, String, bool, bool);

pub fn spec_key(vector: &Json) -> SpecKey {
    (
        vector["spec"].as_str().unwrap().to_string(),
        vector["codec"].as_str().unwrap().to_string(),
        vector["numeric_enums"].as_bool().unwrap(),
        vector["lenient"].as_bool().unwrap(),
    )
}

pub fn load(
    (spec, codec, numeric_enums, lenient): &SpecKey,
) -> Result<Specification, asnkitrt::Error> {
    let data = fs::read(folder().join("specs").join(format!("{spec}.asni"))).unwrap();
    let mut options = Options::new(codec_of(codec));

    options.numeric_enums = *numeric_enums;
    options.lenient = *lenient;

    Specification::from_asni(&data, &Checks::default(), options)
}

/// What an operation of a vector gives.
pub enum Done<'a> {
    Encoded(Vec<u8>),
    Decoded(Value<'a>),
}

/// Does what a vector says to with its inputs: encodes its `value` or
/// decodes its `encoded`, with its `options`.
pub fn perform<'a>(spec: &'a Specification, vector: &Json) -> Result<Done<'a>, asnkitrt::Error> {
    let name = vector["type"].as_str().unwrap();
    let options = &vector["options"];
    let check = options["check_constraints"].as_bool() == Some(true);
    let indent = options["indent"].as_u64().map(|indent| indent as usize);

    if vector["operation"] == "encode" {
        let value = value_of(spec, name, &vector["value"]);

        if check {
            spec.check_constraints(name, &value)?;
        }

        match indent {
            Some(indent) => spec.encode_indented(name, &value, indent),
            None => spec.encode(name, &value),
        }
        .map(Done::Encoded)
    } else {
        let decoded = spec.decode_owned(name, &hex(vector["encoded"].as_str().unwrap()))?;

        if check {
            spec.check_constraints(name, &decoded)?;
        }

        Ok(Done::Decoded(decoded))
    }
}

/// The name of an error in a vector: its variant.
pub fn error_name(error: &asnkitrt::Error) -> String {
    let debug = format!("{error:?}");

    debug
        .split(|c: char| !c.is_ascii_alphanumeric())
        .next()
        .unwrap()
        .to_string()
}
