// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Values of JavaScript to values of the runtime and back.
//!
//! They are those of the runtime for Python, in the types of
//! JavaScript: a SEQUENCE is an object, a CHOICE `[name, value]`, a BIT
//! STRING `[bytes, bits]`, an OCTET STRING a `Uint8Array` and so on. As
//! an array is both a CHOICE and a SEQUENCE OF, and a REAL of `1.0` is
//! the number `1`, a value is converted as its type says it is.
//!
//! A value crosses between JavaScript and WebAssembly as one buffer, and
//! not a call for each of its parts, which costs much more than the
//! codecs: `js/values.js` writes a value of JavaScript as the types of
//! JavaScript it is made of, which `parse()` reads here, and reads the
//! values that `to_buffer()` writes. The tags and their layouts are in
//! both, and have to agree. Numbers are little-endian, and lengths and
//! counts are `u32`.

use asnkitrt::types::{Kind, TypeId, Types, const_matches};
use asnkitrt::value::{Date, DateTime, Time, Value};
use std::borrow::Cow;

/// The greatest integer a number of JavaScript holds exactly.
const MAX_SAFE_INTEGER: i128 = (1 << 53) - 1;

/// How deep a value may be nested, as deeper ones are refused rather
/// than to run out of stack.
const MAX_DEPTH: usize = 256;

/// The tags of the values of JavaScript, from `js/values.js`.
mod from_js {
    pub const NULL: u8 = 0;
    pub const FALSE: u8 = 1;
    pub const TRUE: u8 = 2;
    /// An `f64`.
    pub const NUMBER: u8 = 3;
    /// A number that is an `i32`.
    pub const INT32: u8 = 4;
    /// Its text in base 16, as `-1f`.
    pub const BIGINT: u8 = 5;
    /// A length and UTF-8.
    pub const STRING: u8 = 6;
    /// A length and the bytes of a `Uint8Array`.
    pub const BYTES: u8 = 7;
    /// A count and the items.
    pub const ARRAY: u8 = 8;
    /// A count, and for each property that is not `undefined` its name,
    /// as a length and UTF-8, and its value.
    pub const OBJECT: u8 = 9;
    /// A `Date`: its year, month from 0, day, hours, minutes, seconds
    /// and milliseconds in UTC, as `f64`s.
    pub const DATE: u8 = 10;
    /// A function or a symbol.
    pub const UNSUPPORTED: u8 = 11;
    /// An array or an object nested deeper than is converted, in place
    /// of it.
    pub const TOO_DEEP: u8 = 12;
}

/// The tags of the values given to JavaScript, which `js/values.js`
/// reads.
mod to_js {
    pub const NULL: u8 = 0;
    pub const FALSE: u8 = 1;
    pub const TRUE: u8 = 2;
    /// An `f64`.
    pub const NUMBER: u8 = 3;
    /// An `i32`.
    pub const INT32: u8 = 4;
    /// A length and the integer in two's complement, for a `BigInt`.
    pub const BIGINT: u8 = 5;
    /// A length and UTF-8.
    pub const STRING: u8 = 6;
    /// A length and the bytes, for a `Uint8Array`.
    pub const BYTES: u8 = 7;
    /// A length, the bytes and the number of bits as an `f64`.
    pub const BITS: u8 = 8;
    /// A count and the items.
    pub const LIST: u8 = 9;
    /// A count, and the name and the value of each member.
    pub const MAP: u8 = 10;
    /// The name and the value.
    pub const CHOICE: u8 = 11;
    /// A date and a time, as below.
    pub const DATE_TIME: u8 = 12;
    /// The year as an `i32`, the month and the day as `u8`s.
    pub const DATE: u8 = 13;
    /// The hour, minute and second as `u8`s, the microsecond as a `u32`,
    /// and the offset as a `u8` that is 1 if there is one and an `i32`.
    pub const TIME: u8 = 14;
}

/// A value of JavaScript, as `js/values.js` wrote it.
enum Js<'a> {
    Null,
    Boolean(bool),
    Number(f64),
    /// The text in base 16.
    BigInt(&'a str),
    Text(&'a str),
    Bytes(&'a [u8]),
    Array(Vec<Js<'a>>),
    /// The properties that are not `undefined`.
    Object(Vec<(&'a str, Js<'a>)>),
    /// The year, month from 0, day, hours, minutes, seconds and
    /// milliseconds of a `Date`, in UTC.
    Date([f64; 7]),
    Unsupported,
    TooDeep,
}

impl<'a> Js<'a> {
    /// The property `key` of an object, `None` if it is not there or this
    /// is not an object, as it is `undefined` then.
    fn get(&self, key: &str) -> Option<&Js<'a>> {
        match self {
            Js::Object(properties) => properties
                .iter()
                .find(|(name, _)| *name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    fn as_f64(&self) -> Option<f64> {
        match self {
            Js::Number(number) => Some(*number),
            _ => None,
        }
    }

    /// The two items of an array of two items.
    fn pair(&self) -> Option<(&Js<'a>, &Js<'a>)> {
        match self {
            Js::Array(items) if items.len() == 2 => Some((&items[0], &items[1])),
            _ => None,
        }
    }

    fn is_object(&self) -> bool {
        matches!(
            self,
            Js::Object(_) | Js::Array(_) | Js::Bytes(_) | Js::Date(_) | Js::TooDeep
        )
    }
}

/// Reads what `js/values.js` wrote.
struct Reader<'a> {
    data: &'a [u8],
    at: usize,
}

const MALFORMED: &str = "The value was not written as expected.";

impl<'a> Reader<'a> {
    fn take(&mut self, length: usize) -> Result<&'a [u8], String> {
        let end = self.at.checked_add(length).ok_or(MALFORMED)?;
        let bytes = self.data.get(self.at..end).ok_or(MALFORMED)?;

        self.at = end;

        Ok(bytes)
    }

    fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
        Ok(self.take(N)?.try_into().expect("N bytes"))
    }

    fn u32(&mut self) -> Result<usize, String> {
        Ok(u32::from_le_bytes(self.array()?) as usize)
    }

    fn f64(&mut self) -> Result<f64, String> {
        Ok(f64::from_le_bytes(self.array()?))
    }

    fn text(&mut self) -> Result<&'a str, String> {
        let length = self.u32()?;

        std::str::from_utf8(self.take(length)?).map_err(|_| MALFORMED.into())
    }

    /// A count of items, each of at least one byte.
    fn count(&mut self) -> Result<usize, String> {
        let count = self.u32()?;

        if count > self.data.len() - self.at {
            return Err(MALFORMED.into());
        }

        Ok(count)
    }

    fn value(&mut self, depth: usize) -> Result<Js<'a>, String> {
        // `js/values.js` writes no deeper than a little more than
        // MAX_DEPTH.
        if depth > 2 * MAX_DEPTH {
            return Err(MALFORMED.into());
        }

        let [tag] = self.array()?;

        Ok(match tag {
            from_js::NULL => Js::Null,
            from_js::FALSE => Js::Boolean(false),
            from_js::TRUE => Js::Boolean(true),
            from_js::NUMBER => Js::Number(self.f64()?),
            from_js::INT32 => Js::Number(f64::from(i32::from_le_bytes(self.array()?))),
            from_js::BIGINT => Js::BigInt(self.text()?),
            from_js::STRING => Js::Text(self.text()?),
            from_js::BYTES => {
                let length = self.u32()?;

                Js::Bytes(self.take(length)?)
            }
            from_js::ARRAY => {
                let count = self.count()?;
                let mut items = Vec::with_capacity(count);

                for _ in 0..count {
                    items.push(self.value(depth + 1)?);
                }

                Js::Array(items)
            }
            from_js::OBJECT => {
                let count = self.count()?;
                let mut properties = Vec::with_capacity(count);

                for _ in 0..count {
                    let key = self.text()?;

                    properties.push((key, self.value(depth + 1)?));
                }

                Js::Object(properties)
            }
            from_js::DATE => {
                let mut fields = [0.0; 7];

                for field in fields.iter_mut() {
                    *field = self.f64()?;
                }

                Js::Date(fields)
            }
            from_js::UNSUPPORTED => Js::Unsupported,
            from_js::TOO_DEEP => Js::TooDeep,
            _ => return Err(MALFORMED.into()),
        })
    }
}

/// Converts the value `js/values.js` wrote in `data` to a value of the
/// type `id`.
pub fn to_value<'a>(types: &'a Types, id: TypeId, data: &[u8]) -> Result<Value<'a>, String> {
    let mut reader = Reader { data, at: 0 };
    let value = reader.value(0)?;

    if reader.at != data.len() {
        return Err(MALFORMED.into());
    }

    Converter { types }.typed(Some(id), &value, 0)
}

struct Converter<'a> {
    types: &'a Types,
}

impl<'a> Converter<'a> {
    /// Converts a value of the type `id`, or of no known type if `None`,
    /// as in an open type.
    fn typed(&self, id: Option<TypeId>, value: &Js, depth: usize) -> Result<Value<'a>, String> {
        if depth > MAX_DEPTH {
            return Err("The value is nested too deep.".into());
        }

        let Some(id) = id else {
            return self.untyped(value, depth);
        };
        let kind = &self.types.nodes[self.types.nodes[id].concrete].kind;

        match kind {
            Kind::Real => match value.as_f64() {
                Some(number) => Ok(Value::Real(number)),
                None => self.untyped(value, depth),
            },
            Kind::BitString(_) => match value.pair() {
                Some((bytes, bits)) => match (bytes, bits.as_f64()) {
                    (Js::Bytes(bytes), Some(bits)) if bits >= 0.0 && bits.fract() == 0.0 => {
                        Ok(Value::Bits(bytes.to_vec(), bits as usize))
                    }
                    _ => Err("Expected a BIT STRING as [Uint8Array, number of bits].".into()),
                },
                None => self.untyped(value, depth),
            },
            Kind::Choice(members) => match value.pair() {
                Some((name, inner)) => {
                    let Js::Text(name) = name else {
                        return Err("Expected a CHOICE as [name, value].".into());
                    };
                    let member = members.all().find(|member| member.name == *name);
                    let name = match member {
                        Some(member) => Cow::Borrowed(member.name.as_str()),
                        None => Cow::Owned(name.to_string()),
                    };

                    Ok(Value::Choice(
                        name,
                        Box::new(self.typed(member.map(|member| member.ty), inner, depth + 1)?),
                    ))
                }
                None => self.untyped(value, depth),
            },
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members)
                if matches!(value, Js::Object(_)) =>
            {
                let mut slots: Vec<Option<(Cow<'a, str>, Value<'a>)>> = Vec::new();
                let mut defined_by = Vec::new();

                for member in members.all() {
                    // A member that is undefined is not there.
                    let Some(inner) = value.get(&member.name) else {
                        continue;
                    };

                    // An ANY DEFINED BY is of the type that another member
                    // tells, which is converted first.
                    if let Kind::AnyDefinedBy {
                        value: key,
                        choices,
                    } = &self.types.nodes[self.types.nodes[member.ty].concrete].kind
                        && !choices.is_empty()
                    {
                        defined_by.push((slots.len(), member, key, choices, inner));
                        slots.push(None);
                    } else {
                        slots.push(Some((
                            Cow::Borrowed(member.name.as_str()),
                            self.typed(Some(member.ty), inner, depth + 1)?,
                        )));
                    }
                }

                for (slot, member, key, choices, inner) in defined_by {
                    let key = slots
                        .iter()
                        .flatten()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| value);
                    let ty = key.and_then(|key| {
                        choices
                            .iter()
                            .find(|(candidate, _)| const_matches(candidate, key))
                            .map(|(_, ty)| *ty)
                    });

                    slots[slot] = Some((
                        Cow::Borrowed(member.name.as_str()),
                        self.typed(ty, inner, depth + 1)?,
                    ));
                }

                let mut map: Vec<(Cow<'a, str>, Value<'a>)> = slots.into_iter().flatten().collect();

                // Any other is left for the codec to reject.
                if let Js::Object(properties) = value {
                    for (key, inner) in properties {
                        if !members.all().any(|member| member.name == *key) {
                            map.push((
                                Cow::Owned(key.to_string()),
                                self.untyped(inner, depth + 1)?,
                            ));
                        }
                    }
                }

                Ok(Value::Map(map))
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => match value {
                Js::Array(items) => items
                    .iter()
                    .map(|item| self.typed(Some(*element), item, depth + 1))
                    .collect::<Result<_, _>>()
                    .map(Value::List),
                _ => self.untyped(value, depth),
            },
            Kind::Utc | Kind::Generalized if value.is_object() => {
                Ok(Value::DateTime(date_time(value)?))
            }
            // A local time, which a `Date` is as its time in UTC.
            Kind::DateTime if value.is_object() => {
                let mut date_time = date_time(value)?;

                if matches!(value, Js::Date(_)) {
                    date_time.time.offset = None;
                }

                Ok(Value::DateTime(date_time))
            }
            Kind::Date if value.is_object() => Ok(Value::Date(date(value)?)),
            Kind::TimeOfDay if value.is_object() => {
                let mut time = time(value)?;

                if matches!(value, Js::Date(_)) {
                    time.offset = None;
                }

                Ok(Value::Time(time))
            }
            _ => self.untyped(value, depth),
        }
    }

    /// Converts a value as the type of JavaScript it is.
    fn untyped(&self, value: &Js, depth: usize) -> Result<Value<'a>, String> {
        if depth > MAX_DEPTH {
            return Err("The value is nested too deep.".into());
        }

        match value {
            Js::Null => Ok(Value::Null),
            Js::Boolean(boolean) => Ok(Value::Boolean(*boolean)),
            Js::Number(number) => Ok(number_to_value(*number)),
            Js::BigInt(text) => bigint_to_value(text),
            Js::Text(text) => Ok(Value::Text(text.to_string())),
            Js::Bytes(bytes) => Ok(Value::Bytes(bytes.to_vec())),
            Js::Array(items) => items
                .iter()
                .map(|item| self.untyped(item, depth + 1))
                .collect::<Result<_, _>>()
                .map(Value::List),
            Js::Date(_) => Ok(Value::DateTime(date_time(value)?)),
            Js::Object(properties) => properties
                .iter()
                .map(|(key, inner)| {
                    Ok((Cow::Owned(key.to_string()), self.untyped(inner, depth + 1)?))
                })
                .collect::<Result<_, String>>()
                .map(Value::Map),
            // Only written in place of what is deeper than MAX_DEPTH.
            Js::TooDeep => Err("The value is nested too deep.".into()),
            Js::Unsupported => Err("A value of a type that can not be encoded.".into()),
        }
    }
}

fn number_to_value(number: f64) -> Value<'static> {
    // An integer is an INTEGER, as far as it is exact.
    if number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER as f64 {
        Value::Integer(number as i128)
    } else {
        Value::Real(number)
    }
}

/// The integer of a `BigInt`, by its digits in base 16.
fn bigint_to_value(text: &str) -> Result<Value<'static>, String> {
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text),
    };
    // The magnitude, most significant byte first, with a byte for the
    // sign.
    let padded = if digits.len() % 2 == 1 {
        format!("0{digits}")
    } else {
        digits.to_string()
    };
    let mut bytes = vec![0u8];

    for index in (0..padded.len()).step_by(2) {
        bytes.push(
            padded
                .get(index..index + 2)
                .and_then(|pair| u8::from_str_radix(pair, 16).ok())
                .ok_or("Can not read a BigInt.")?,
        );
    }

    if negative {
        // The two's complement: inverted, plus one.
        for byte in bytes.iter_mut() {
            *byte = !*byte;
        }

        for byte in bytes.iter_mut().rev() {
            let (sum, carry) = byte.overflowing_add(1);

            *byte = sum;

            if !carry {
                break;
            }
        }
    }

    Ok(Value::from_twos_complement(&bytes))
}

fn field(value: &Js, key: &str) -> Result<Option<f64>, String> {
    match value.get(key) {
        None | Some(Js::Null) => Ok(None),
        Some(Js::Number(number)) if number.fract() == 0.0 => Ok(Some(*number)),
        Some(_) => Err(format!("Expected '{key}' to be an integer.")),
    }
}

fn required(value: &Js, key: &str) -> Result<f64, String> {
    field(value, key)?.ok_or_else(|| format!("Expected '{key}'."))
}

fn date(value: &Js) -> Result<Date, String> {
    if let Js::Date([year, month, day, ..]) = value {
        return Ok(Date {
            year: *year as i32,
            month: *month as u8 + 1,
            day: *day as u8,
        });
    }

    Ok(Date {
        year: required(value, "year")? as i32,
        month: required(value, "month")? as u8,
        day: required(value, "day")? as u8,
    })
}

fn time(value: &Js) -> Result<Time, String> {
    if let Js::Date([.., hour, minute, second, millisecond]) = value {
        return Ok(Time {
            hour: *hour as u8,
            minute: *minute as u8,
            second: *second as u8,
            microsecond: (*millisecond as u32) * 1000,
            offset: Some(0),
        });
    }

    Ok(Time {
        hour: required(value, "hour")? as u8,
        minute: field(value, "minute")?.unwrap_or(0.0) as u8,
        second: field(value, "second")?.unwrap_or(0.0) as u8,
        microsecond: field(value, "microsecond")?.unwrap_or(0.0) as u32,
        offset: field(value, "offset")?.map(|offset| offset as i32),
    })
}

/// A date and a time from an object with the fields of both, or from a
/// `Date`, which is in UTC.
fn date_time(value: &Js) -> Result<DateTime, String> {
    Ok(DateTime {
        date: date(value)?,
        time: time(value)?,
    })
}

/// Writes a decoded value for `js/values.js` to read.
pub fn to_buffer(value: &Value) -> Vec<u8> {
    let mut buffer = Vec::with_capacity(256);

    write(&mut buffer, value);

    buffer
}

fn write_length(buffer: &mut Vec<u8>, length: usize) {
    buffer.extend_from_slice(&(length as u32).to_le_bytes());
}

fn write_bytes(buffer: &mut Vec<u8>, bytes: &[u8]) {
    write_length(buffer, bytes.len());
    buffer.extend_from_slice(bytes);
}

fn write_date(buffer: &mut Vec<u8>, date: &Date) {
    buffer.extend_from_slice(&date.year.to_le_bytes());
    buffer.push(date.month);
    buffer.push(date.day);
}

fn write_time(buffer: &mut Vec<u8>, time: &Time) {
    buffer.extend_from_slice(&[time.hour, time.minute, time.second]);
    buffer.extend_from_slice(&time.microsecond.to_le_bytes());
    buffer.push(time.offset.is_some() as u8);
    buffer.extend_from_slice(&time.offset.unwrap_or(0).to_le_bytes());
}

fn write(buffer: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Null => buffer.push(to_js::NULL),
        Value::Boolean(false) => buffer.push(to_js::FALSE),
        Value::Boolean(true) => buffer.push(to_js::TRUE),
        Value::Integer(integer) => {
            if let Ok(small) = i32::try_from(*integer) {
                buffer.push(to_js::INT32);
                buffer.extend_from_slice(&small.to_le_bytes());
            } else if integer.abs() <= MAX_SAFE_INTEGER {
                buffer.push(to_js::NUMBER);
                buffer.extend_from_slice(&(*integer as f64).to_le_bytes());
            } else {
                buffer.push(to_js::BIGINT);
                write_bytes(
                    buffer,
                    &asnkitrt::value::integer_to_twos_complement(*integer),
                );
            }
        }
        Value::BigInteger(bytes) => {
            buffer.push(to_js::BIGINT);
            write_bytes(buffer, bytes);
        }
        Value::Real(number) => {
            buffer.push(to_js::NUMBER);
            buffer.extend_from_slice(&number.to_le_bytes());
        }
        Value::Text(text) => {
            buffer.push(to_js::STRING);
            write_bytes(buffer, text.as_bytes());
        }
        Value::Bytes(bytes) => {
            buffer.push(to_js::BYTES);
            write_bytes(buffer, bytes);
        }
        Value::Bits(bytes, bits) => {
            buffer.push(to_js::BITS);
            write_bytes(buffer, bytes);
            buffer.extend_from_slice(&(*bits as f64).to_le_bytes());
        }
        Value::List(items) => {
            buffer.push(to_js::LIST);
            write_length(buffer, items.len());

            for item in items {
                write(buffer, item);
            }
        }
        Value::Map(members) => {
            buffer.push(to_js::MAP);
            write_length(buffer, members.len());

            for (name, member) in members {
                write_bytes(buffer, name.as_bytes());
                write(buffer, member);
            }
        }
        Value::Choice(name, inner) => {
            buffer.push(to_js::CHOICE);
            write_bytes(buffer, name.as_bytes());
            write(buffer, inner);
        }
        Value::DateTime(date_time) => {
            buffer.push(to_js::DATE_TIME);
            write_date(buffer, &date_time.date);
            write_time(buffer, &date_time.time);
        }
        Value::Date(date) => {
            buffer.push(to_js::DATE);
            write_date(buffer, date);
        }
        Value::Time(time) => {
            buffer.push(to_js::TIME);
            write_time(buffer, time);
        }
    }
}
