// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Values and the types of serde.
//!
//! [`to_value`] and [`from_value`] take a Rust type to a [`Value`] and
//! back, so that it is encoded and decoded with a specification:
//!
//! | ASN.1 | Rust |
//! |---|---|
//! | BOOLEAN | `bool` |
//! | INTEGER | integers up to 128 bits, or [`BigInt`] for any |
//! | REAL | `f64` or `f32` |
//! | NULL | `()` |
//! | OCTET STRING | [`OctetString`], or `Vec<u8>` with `#[serde(with = "asnkitrt::serde_value::bytes")]` |
//! | BIT STRING | [`BitString`] |
//! | OBJECT IDENTIFIER, strings | `String` |
//! | UTCTime, GeneralizedTime, DATE-TIME | [`DateTime`](crate::value::DateTime) |
//! | DATE | [`Date`](crate::value::Date) |
//! | TIME-OF-DAY | [`Time`](crate::value::Time) |
//! | ANY, open types | [`Value`] |
//! | ENUMERATED | an enum of unit variants, by the name of the value |
//! | SEQUENCE, SET | a struct, by the name of the member |
//! | SEQUENCE OF, SET OF | `Vec<T>` |
//! | CHOICE | an enum, by the name of the alternative |
//! | OPTIONAL | `Option<T>` with `#[serde(skip_serializing_if = "Option::is_none")]` |
//!
//! `#[serde(rename = "...")]` gives the name where it is not one Rust has.

use alloc::boxed::Box;
use alloc::string::{String, ToString};
use alloc::vec::Vec;
use core::fmt;

use serde::de::{self, DeserializeSeed, IntoDeserializer, Visitor};
use serde::ser::{self, Serialize};

use crate::Error;
use crate::value::Value;

impl ser::Error for Error {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Error::encode("The value can not be written as a value of ASN.1.")
    }
}

impl de::Error for Error {
    fn custom<T: fmt::Display>(_: T) -> Self {
        Error::decode("The value is not what the type is.", 0)
    }
}

// With `std` the error is `std::error::Error`, see `error.rs`.
#[cfg(not(feature = "std"))]
impl ser::StdError for Error {}

/// The name that makes a BIT STRING of a newtype struct.
const BIT_STRING: &str = "$asnkitrt::BitString";

/// The names that make a date and a time of a newtype struct.
const BIG_INT: &str = "$asnkitrt::BigInt";
const DATE_TIME: &str = "$asnkitrt::DateTime";
const DATE: &str = "$asnkitrt::Date";
const TIME: &str = "$asnkitrt::Time";

/// An INTEGER of any size: its two's complement, most significant byte
/// first. Nothing at all is 0.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct BigInt(pub Vec<u8>);

impl BigInt {
    /// The integer, if it fits in 128 bits.
    pub fn to_i128(&self) -> Option<i128> {
        match Value::from_twos_complement(&self.0) {
            Value::Integer(value) => Some(value),
            _ => None,
        }
    }
}

impl From<i128> for BigInt {
    fn from(value: i128) -> BigInt {
        BigInt(crate::value::integer_to_twos_complement(value))
    }
}

impl Serialize for BigInt {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(BIG_INT, &BytesRef(&self.0))
    }
}

impl<'de> de::Deserialize<'de> for BigInt {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BigIntVisitor;

        struct BytesVisitor;

        impl<'de> Visitor<'de> for BytesVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("bytes")
            }

            fn visit_bytes<E: de::Error>(self, bytes: &[u8]) -> Result<Vec<u8>, E> {
                Ok(bytes.to_vec())
            }

            fn visit_byte_buf<E: de::Error>(self, bytes: Vec<u8>) -> Result<Vec<u8>, E> {
                Ok(bytes)
            }
        }

        impl<'de> Visitor<'de> for BigIntVisitor {
            type Value = BigInt;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("an INTEGER")
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<BigInt, D::Error> {
                deserializer.deserialize_bytes(BytesVisitor).map(BigInt)
            }
        }

        deserializer.deserialize_newtype_struct(BIG_INT, BigIntVisitor)
    }
}

/// An OCTET STRING.
#[derive(Debug, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OctetString(pub Vec<u8>);

impl From<Vec<u8>> for OctetString {
    fn from(bytes: Vec<u8>) -> OctetString {
        OctetString(bytes)
    }
}

impl From<OctetString> for Vec<u8> {
    fn from(bytes: OctetString) -> Vec<u8> {
        bytes.0
    }
}

impl core::ops::Deref for OctetString {
    type Target = Vec<u8>;

    fn deref(&self) -> &Vec<u8> {
        &self.0
    }
}

impl core::ops::DerefMut for OctetString {
    fn deref_mut(&mut self) -> &mut Vec<u8> {
        &mut self.0
    }
}

impl Serialize for OctetString {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> de::Deserialize<'de> for OctetString {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        bytes::deserialize(deserializer).map(OctetString)
    }
}

impl Serialize for crate::value::Date {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(DATE, &(self.year, self.month, self.day))
    }
}

impl<'de> de::Deserialize<'de> for crate::value::Date {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DateVisitor;

        impl<'de> Visitor<'de> for DateVisitor {
            type Value = crate::value::Date;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a DATE")
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                let (year, month, day) = de::Deserialize::deserialize(deserializer)?;

                Ok(crate::value::Date { year, month, day })
            }
        }

        deserializer.deserialize_newtype_struct(DATE, DateVisitor)
    }
}

type TimeParts = (u8, u8, u8, u32, bool, i32);

fn time_parts(time: &crate::value::Time) -> TimeParts {
    (
        time.hour,
        time.minute,
        time.second,
        time.microsecond,
        time.offset.is_some(),
        time.offset.unwrap_or(0),
    )
}

fn time_of_parts(parts: TimeParts) -> crate::value::Time {
    let (hour, minute, second, microsecond, has_offset, offset) = parts;

    crate::value::Time {
        hour,
        minute,
        second,
        microsecond,
        offset: has_offset.then_some(offset),
    }
}

impl Serialize for crate::value::Time {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(TIME, &time_parts(self))
    }
}

impl<'de> de::Deserialize<'de> for crate::value::Time {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct TimeVisitor;

        impl<'de> Visitor<'de> for TimeVisitor {
            type Value = crate::value::Time;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a TIME-OF-DAY")
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                let parts: TimeParts = de::Deserialize::deserialize(deserializer)?;

                Ok(time_of_parts(parts))
            }
        }

        deserializer.deserialize_newtype_struct(TIME, TimeVisitor)
    }
}

impl Serialize for crate::value::DateTime {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(
            DATE_TIME,
            &(
                (self.date.year, self.date.month, self.date.day),
                time_parts(&self.time),
            ),
        )
    }
}

impl<'de> de::Deserialize<'de> for crate::value::DateTime {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct DateTimeVisitor;

        impl<'de> Visitor<'de> for DateTimeVisitor {
            type Value = crate::value::DateTime;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a DATE-TIME")
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Self::Value, D::Error> {
                let ((year, month, day), time): ((i32, u8, u8), TimeParts) =
                    de::Deserialize::deserialize(deserializer)?;

                Ok(crate::value::DateTime {
                    date: crate::value::Date { year, month, day },
                    time: time_of_parts(time),
                })
            }
        }

        deserializer.deserialize_newtype_struct(DATE_TIME, DateTimeVisitor)
    }
}

/// A BIT STRING.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BitString {
    pub data: Vec<u8>,
    pub bits: usize,
}

impl Serialize for BitString {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_newtype_struct(BIT_STRING, &(serde_bytes_ref(&self.data), self.bits))
    }
}

impl<'de> de::Deserialize<'de> for BitString {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct BitVisitor;

        impl<'de> Visitor<'de> for BitVisitor {
            type Value = BitString;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a BIT STRING")
            }

            fn visit_newtype_struct<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<BitString, D::Error> {
                let (data, bits): (Vec<u8>, usize) = de::Deserialize::deserialize(deserializer)?;

                Ok(BitString { data, bits })
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<BitString, A::Error> {
                let data: Vec<u8> = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(0, &self))?;
                let bits: usize = seq
                    .next_element()?
                    .ok_or_else(|| de::Error::invalid_length(1, &self))?;

                Ok(BitString { data, bits })
            }
        }

        deserializer.deserialize_newtype_struct(BIT_STRING, BitVisitor)
    }
}

/// Bytes as bytes and not as a list of numbers.
struct BytesRef<'a>(&'a [u8]);

impl Serialize for BytesRef<'_> {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(self.0)
    }
}

fn serde_bytes_ref(data: &[u8]) -> BytesRef<'_> {
    BytesRef(data)
}

/// For `#[serde(with = "asnkitrt::serde_value::bytes")]` on a `Vec<u8>`.
pub mod bytes {
    use alloc::vec::Vec;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(data: &[u8], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(data)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<u8>, D::Error> {
        serde::Deserialize::deserialize(deserializer)
    }
}

// A value as the types of serde.

impl Serialize for Value {
    fn serialize<S: ser::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use ser::{SerializeMap, SerializeSeq, SerializeStruct};

        match self {
            Value::Null => serializer.serialize_unit(),
            Value::Boolean(value) => serializer.serialize_bool(*value),
            Value::Integer(value) => serializer.serialize_i128(*value),
            Value::BigInteger(bytes) => serializer.serialize_bytes(bytes),
            Value::Real(value) => serializer.serialize_f64(*value),
            Value::Text(text) => serializer.serialize_str(text),
            Value::Bytes(bytes) => serializer.serialize_bytes(bytes),
            Value::Bits(data, bits) => {
                let mut state = serializer.serialize_struct("BitString", 2)?;

                state.serialize_field("data", &BytesRef(data))?;
                state.serialize_field("bits", bits)?;
                state.end()
            }
            Value::List(items) => {
                let mut state = serializer.serialize_seq(Some(items.len()))?;

                for item in items {
                    state.serialize_element(item)?;
                }

                state.end()
            }
            Value::Map(members) => {
                let mut state = serializer.serialize_map(Some(members.len()))?;

                for (name, value) in members {
                    state.serialize_entry(name, value)?;
                }

                state.end()
            }
            Value::Choice(name, value) => {
                let mut state = serializer.serialize_map(Some(1))?;

                state.serialize_entry(name, value)?;
                state.end()
            }
            Value::DateTime(value) => {
                let mut state = serializer.serialize_struct("DateTime", 7)?;

                state.serialize_field("year", &value.date.year)?;
                state.serialize_field("month", &value.date.month)?;
                state.serialize_field("day", &value.date.day)?;
                state.serialize_field("hour", &value.time.hour)?;
                state.serialize_field("minute", &value.time.minute)?;
                state.serialize_field("second", &value.time.second)?;
                state.serialize_field("microsecond", &value.time.microsecond)?;
                state.end()
            }
            Value::Date(value) => {
                let mut state = serializer.serialize_struct("Date", 3)?;

                state.serialize_field("year", &value.year)?;
                state.serialize_field("month", &value.month)?;
                state.serialize_field("day", &value.day)?;
                state.end()
            }
            Value::Time(value) => {
                let mut state = serializer.serialize_struct("Time", 4)?;

                state.serialize_field("hour", &value.hour)?;
                state.serialize_field("minute", &value.minute)?;
                state.serialize_field("second", &value.second)?;
                state.serialize_field("microsecond", &value.microsecond)?;
                state.end()
            }
        }
    }
}

impl<'de> de::Deserialize<'de> for Value {
    fn deserialize<D: de::Deserializer<'de>>(deserializer: D) -> Result<Value, D::Error> {
        struct ValueVisitor;

        impl<'de> Visitor<'de> for ValueVisitor {
            type Value = Value;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a value")
            }

            fn visit_unit<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_none<E>(self) -> Result<Value, E> {
                Ok(Value::Null)
            }

            fn visit_some<D: de::Deserializer<'de>>(
                self,
                deserializer: D,
            ) -> Result<Value, D::Error> {
                de::Deserialize::deserialize(deserializer)
            }

            fn visit_bool<E>(self, value: bool) -> Result<Value, E> {
                Ok(Value::Boolean(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Value, E> {
                Ok(Value::Integer(i128::from(value)))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Value, E> {
                Ok(Value::Integer(i128::from(value)))
            }

            fn visit_i128<E>(self, value: i128) -> Result<Value, E> {
                Ok(Value::Integer(value))
            }

            fn visit_f64<E>(self, value: f64) -> Result<Value, E> {
                Ok(Value::Real(value))
            }

            fn visit_str<E>(self, value: &str) -> Result<Value, E> {
                Ok(Value::Text(value.to_string()))
            }

            fn visit_string<E>(self, value: String) -> Result<Value, E> {
                Ok(Value::Text(value))
            }

            fn visit_bytes<E>(self, value: &[u8]) -> Result<Value, E> {
                Ok(Value::Bytes(Vec::from(value)))
            }

            fn visit_byte_buf<E>(self, value: Vec<u8>) -> Result<Value, E> {
                Ok(Value::Bytes(value))
            }

            fn visit_seq<A: de::SeqAccess<'de>>(self, mut seq: A) -> Result<Value, A::Error> {
                let mut items = Vec::new();

                while let Some(item) = seq.next_element()? {
                    items.push(item);
                }

                Ok(Value::List(items))
            }

            fn visit_map<A: de::MapAccess<'de>>(self, mut map: A) -> Result<Value, A::Error> {
                let mut members = Vec::new();

                while let Some((name, value)) = map.next_entry::<String, Value>()? {
                    members.push((name, value));
                }

                Ok(Value::Map(members))
            }
        }

        deserializer.deserialize_any(ValueVisitor)
    }
}

// From Rust to a value.

/// The time that the list of `time_parts` is.
fn time_of_list(value: &Value) -> Result<crate::value::Time, Error> {
    if let Value::List(parts) = value
        && let [
            Value::Integer(hour),
            Value::Integer(minute),
            Value::Integer(second),
            Value::Integer(microsecond),
            Value::Boolean(has_offset),
            Value::Integer(offset),
        ] = parts.as_slice()
    {
        return Ok(crate::value::Time {
            hour: *hour as u8,
            minute: *minute as u8,
            second: *second as u8,
            microsecond: *microsecond as u32,
            offset: has_offset.then_some(*offset as i32),
        });
    }

    Err(Error::encode("Expected the parts of a time."))
}

/// Takes `value` to a [`Value`].
pub fn to_value<T: Serialize + ?Sized>(value: &T) -> Result<Value, Error> {
    value.serialize(ValueSerializer)
}

struct ValueSerializer;

struct SeqState {
    items: Vec<Value>,
}

struct MapState {
    members: Vec<(String, Value)>,
    key: Option<String>,
}

struct VariantState<T> {
    name: &'static str,
    inner: T,
}

impl ser::Serializer for ValueSerializer {
    type Ok = Value;
    type Error = Error;
    type SerializeSeq = SeqState;
    type SerializeTuple = SeqState;
    type SerializeTupleStruct = SeqState;
    type SerializeTupleVariant = VariantState<SeqState>;
    type SerializeMap = MapState;
    type SerializeStruct = MapState;
    type SerializeStructVariant = VariantState<MapState>;

    fn serialize_bool(self, value: bool) -> Result<Value, Error> {
        Ok(Value::Boolean(value))
    }

    fn serialize_i8(self, value: i8) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_i16(self, value: i16) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_i32(self, value: i32) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_i64(self, value: i64) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_i128(self, value: i128) -> Result<Value, Error> {
        Ok(Value::Integer(value))
    }

    fn serialize_u8(self, value: u8) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_u16(self, value: u16) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_u32(self, value: u32) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_u64(self, value: u64) -> Result<Value, Error> {
        Ok(Value::Integer(i128::from(value)))
    }

    fn serialize_u128(self, value: u128) -> Result<Value, Error> {
        i128::try_from(value)
            .map(Value::Integer)
            .map_err(|_| Error::encode("The integer is too large."))
    }

    fn serialize_f32(self, value: f32) -> Result<Value, Error> {
        Ok(Value::Real(f64::from(value)))
    }

    fn serialize_f64(self, value: f64) -> Result<Value, Error> {
        Ok(Value::Real(value))
    }

    fn serialize_char(self, value: char) -> Result<Value, Error> {
        Ok(Value::Text(value.to_string()))
    }

    fn serialize_str(self, value: &str) -> Result<Value, Error> {
        Ok(Value::Text(value.to_string()))
    }

    fn serialize_bytes(self, value: &[u8]) -> Result<Value, Error> {
        Ok(Value::Bytes(Vec::from(value)))
    }

    fn serialize_none(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_some<T: Serialize + ?Sized>(self, value: &T) -> Result<Value, Error> {
        value.serialize(self)
    }

    fn serialize_unit(self) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_struct(self, _: &'static str) -> Result<Value, Error> {
        Ok(Value::Null)
    }

    fn serialize_unit_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
    ) -> Result<Value, Error> {
        Ok(Value::Text(variant.to_string()))
    }

    fn serialize_newtype_struct<T: Serialize + ?Sized>(
        self,
        name: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        let inner = value.serialize(ValueSerializer)?;

        if name == BIG_INT {
            return match inner {
                Value::Bytes(bytes) => Ok(Value::from_twos_complement(&bytes)),
                _ => Err(Error::encode("Expected the bytes of an INTEGER.")),
            };
        }

        if name == DATE {
            return match inner {
                Value::List(parts) => match parts.as_slice() {
                    [
                        Value::Integer(year),
                        Value::Integer(month),
                        Value::Integer(day),
                    ] => Ok(Value::Date(crate::value::Date {
                        year: *year as i32,
                        month: *month as u8,
                        day: *day as u8,
                    })),
                    _ => Err(Error::encode("Expected a year, a month and a day.")),
                },
                _ => Err(Error::encode("Expected a year, a month and a day.")),
            };
        }

        if name == TIME {
            return time_of_list(&inner).map(Value::Time);
        }

        if name == DATE_TIME {
            if let Value::List(parts) = &inner
                && let [Value::List(date), time] = parts.as_slice()
                && let [
                    Value::Integer(year),
                    Value::Integer(month),
                    Value::Integer(day),
                ] = date.as_slice()
            {
                return Ok(Value::DateTime(crate::value::DateTime {
                    date: crate::value::Date {
                        year: *year as i32,
                        month: *month as u8,
                        day: *day as u8,
                    },
                    time: time_of_list(time)?,
                }));
            }

            return Err(Error::encode("Expected a date and a time."));
        }

        if name == BIT_STRING {
            if let Value::List(parts) = inner
                && let [Value::Bytes(data), Value::Integer(bits)] = parts.as_slice()
            {
                return Ok(Value::Bits(data.clone(), *bits as usize));
            }

            return Err(Error::encode(
                "Expected the bytes and the number of bits of a BIT STRING.",
            ));
        }

        Ok(inner)
    }

    fn serialize_newtype_variant<T: Serialize + ?Sized>(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        value: &T,
    ) -> Result<Value, Error> {
        Ok(Value::Choice(
            variant.to_string(),
            Box::new(value.serialize(ValueSerializer)?),
        ))
    }

    fn serialize_seq(self, length: Option<usize>) -> Result<SeqState, Error> {
        Ok(SeqState {
            items: Vec::with_capacity(length.unwrap_or(0).min(1024)),
        })
    }

    fn serialize_tuple(self, length: usize) -> Result<SeqState, Error> {
        self.serialize_seq(Some(length))
    }

    fn serialize_tuple_struct(self, _: &'static str, length: usize) -> Result<SeqState, Error> {
        self.serialize_seq(Some(length))
    }

    fn serialize_tuple_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        length: usize,
    ) -> Result<VariantState<SeqState>, Error> {
        Ok(VariantState {
            name: variant,
            inner: self.serialize_seq(Some(length))?,
        })
    }

    fn serialize_map(self, _: Option<usize>) -> Result<MapState, Error> {
        Ok(MapState {
            members: Vec::new(),
            key: None,
        })
    }

    fn serialize_struct(self, _: &'static str, _: usize) -> Result<MapState, Error> {
        self.serialize_map(None)
    }

    fn serialize_struct_variant(
        self,
        _: &'static str,
        _: u32,
        variant: &'static str,
        _: usize,
    ) -> Result<VariantState<MapState>, Error> {
        Ok(VariantState {
            name: variant,
            inner: self.serialize_map(None)?,
        })
    }
}

impl ser::SerializeSeq for SeqState {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        self.items.push(value.serialize(ValueSerializer)?);

        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::List(self.items))
    }
}

impl ser::SerializeTuple for SeqState {
    type Ok = Value;
    type Error = Error;

    fn serialize_element<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleStruct for SeqState {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(self, value)
    }

    fn end(self) -> Result<Value, Error> {
        ser::SerializeSeq::end(self)
    }
}

impl ser::SerializeTupleVariant for VariantState<SeqState> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        ser::SerializeSeq::serialize_element(&mut self.inner, value)
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Choice(
            self.name.to_string(),
            Box::new(ser::SerializeSeq::end(self.inner)?),
        ))
    }
}

impl ser::SerializeMap for MapState {
    type Ok = Value;
    type Error = Error;

    fn serialize_key<T: Serialize + ?Sized>(&mut self, key: &T) -> Result<(), Error> {
        match key.serialize(ValueSerializer)? {
            Value::Text(name) => {
                self.key = Some(name);

                Ok(())
            }
            _ => Err(Error::encode("The name of a member has to be a string.")),
        }
    }

    fn serialize_value<T: Serialize + ?Sized>(&mut self, value: &T) -> Result<(), Error> {
        let name = self
            .key
            .take()
            .ok_or_else(|| Error::encode("A value comes after its name."))?;

        self.members.push((name, value.serialize(ValueSerializer)?));

        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(self.members))
    }
}

impl ser::SerializeStruct for MapState {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        self.members
            .push((name.to_string(), value.serialize(ValueSerializer)?));

        Ok(())
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Map(self.members))
    }
}

impl ser::SerializeStructVariant for VariantState<MapState> {
    type Ok = Value;
    type Error = Error;

    fn serialize_field<T: Serialize + ?Sized>(
        &mut self,
        name: &'static str,
        value: &T,
    ) -> Result<(), Error> {
        ser::SerializeStruct::serialize_field(&mut self.inner, name, value)
    }

    fn end(self) -> Result<Value, Error> {
        Ok(Value::Choice(
            self.name.to_string(),
            Box::new(Value::Map(self.inner.members)),
        ))
    }
}

// From a value to Rust.

/// Takes `value` to a `T`.
pub fn from_value<'de, T: de::Deserialize<'de>>(value: &'de Value) -> Result<T, Error> {
    T::deserialize(ValueDeserializer(value))
}

struct ValueDeserializer<'de>(&'de Value);

impl<'de> de::Deserializer<'de> for ValueDeserializer<'de> {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Null => visitor.visit_unit(),
            Value::Boolean(value) => visitor.visit_bool(*value),
            Value::Integer(value) => match i64::try_from(*value) {
                Ok(value) => visitor.visit_i64(value),
                Err(_) => visitor.visit_i128(*value),
            },
            Value::BigInteger(bytes) => visitor.visit_bytes(bytes),
            Value::Real(value) => visitor.visit_f64(*value),
            Value::Text(text) => visitor.visit_str(text),
            Value::Bytes(bytes) => visitor.visit_bytes(bytes),
            Value::Bits(data, bits) => visitor.visit_seq(SeqDeserializer {
                items: alloc::vec![Value::Bytes(data.clone()), Value::Integer(*bits as i128)],
                position: 0,
            }),
            Value::List(items) => visitor.visit_seq(RefSeq {
                items: items.iter(),
            }),
            Value::Map(members) => visitor.visit_map(RefMap {
                members: members.iter(),
                value: None,
            }),
            Value::Choice(name, value) => visitor.visit_map(ChoiceMap {
                name,
                value,
                done: false,
            }),
            _ => Err(Error::decode("The value is not supported by serde.", 0)),
        }
    }

    fn deserialize_option<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_some(self)
    }

    /// Bytes are a list of numbers to what asks for a list.
    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Bytes(bytes) => visitor.visit_seq(de::value::SeqDeserializer::<_, Error>::new(
                bytes.iter().copied(),
            )),
            _ => self.deserialize_any(visitor),
        }
    }

    fn deserialize_newtype_struct<V: Visitor<'de>>(
        self,
        name: &'static str,
        visitor: V,
    ) -> Result<V::Value, Error> {
        if name == BIG_INT {
            return match self.0.to_twos_complement() {
                Some(bytes) => visitor.visit_newtype_struct(OwnedDeserializer(Value::Bytes(bytes))),
                None => Err(Error::decode("Expected an INTEGER.", 0)),
            };
        }

        let parts = |time: &crate::value::Time| {
            alloc::vec![
                Value::Integer(i128::from(time.hour)),
                Value::Integer(i128::from(time.minute)),
                Value::Integer(i128::from(time.second)),
                Value::Integer(i128::from(time.microsecond)),
                Value::Boolean(time.offset.is_some()),
                Value::Integer(i128::from(time.offset.unwrap_or(0))),
            ]
        };
        let date_parts = |date: &crate::value::Date| {
            alloc::vec![
                Value::Integer(i128::from(date.year)),
                Value::Integer(i128::from(date.month)),
                Value::Integer(i128::from(date.day)),
            ]
        };

        if name == DATE {
            return match self.0 {
                Value::Date(date) => {
                    visitor.visit_newtype_struct(SeqValueDeserializer(date_parts(date)))
                }
                _ => Err(Error::decode("Expected a DATE.", 0)),
            };
        }

        if name == TIME {
            return match self.0 {
                Value::Time(time) => {
                    visitor.visit_newtype_struct(SeqValueDeserializer(parts(time)))
                }
                _ => Err(Error::decode("Expected a TIME-OF-DAY.", 0)),
            };
        }

        if name == DATE_TIME {
            return match self.0 {
                Value::DateTime(value) => {
                    visitor.visit_newtype_struct(SeqValueDeserializer(alloc::vec![
                        Value::List(date_parts(&value.date)),
                        Value::List(parts(&value.time)),
                    ]))
                }
                _ => Err(Error::decode("Expected a date and a time.", 0)),
            };
        }

        if name == BIT_STRING {
            return match self.0 {
                Value::Bits(data, bits) => {
                    visitor.visit_newtype_struct(SeqValueDeserializer(alloc::vec![
                        Value::Bytes(data.clone()),
                        Value::Integer(*bits as i128)
                    ]))
                }
                _ => Err(Error::decode("Expected a BIT STRING.", 0)),
            };
        }

        visitor.visit_newtype_struct(self)
    }

    fn deserialize_enum<V: Visitor<'de>>(
        self,
        _: &'static str,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        match self.0 {
            Value::Text(name) => visitor.visit_enum(name.as_str().into_deserializer()),
            Value::Choice(name, value) => visitor.visit_enum(ChoiceAccess { name, value }),
            _ => Err(Error::decode(
                "Expected an alternative of a CHOICE or an enumerated value.",
                0,
            )),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf unit unit_struct
        tuple tuple_struct map struct identifier ignored_any
    }
}

struct SeqValueDeserializer(Vec<Value>);

impl<'de> de::Deserializer<'de> for SeqValueDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        visitor.visit_seq(SeqDeserializer {
            items: self.0,
            position: 0,
        })
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct
        newtype_struct seq tuple tuple_struct map struct enum identifier ignored_any
    }
}

struct SeqDeserializer {
    items: Vec<Value>,
    position: usize,
}

impl<'de> de::SeqAccess<'de> for SeqDeserializer {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        let Some(item) = self.items.get(self.position) else {
            return Ok(None);
        };

        self.position += 1;

        // The item is owned by this and the types of serde that borrow
        // from it can not be given.
        seed.deserialize(OwnedDeserializer(item.clone())).map(Some)
    }
}

/// A value that is owned, for what is not borrowed from the original.
struct OwnedDeserializer(Value);

impl<'de> de::Deserializer<'de> for OwnedDeserializer {
    type Error = Error;

    fn deserialize_any<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Bytes(bytes) => visitor.visit_byte_buf(bytes),
            Value::Integer(value) => match i64::try_from(value) {
                Ok(value) => visitor.visit_i64(value),
                Err(_) => visitor.visit_i128(value),
            },
            Value::Boolean(value) => visitor.visit_bool(value),
            Value::List(items) => visitor.visit_seq(SeqDeserializer { items, position: 0 }),
            _ => Err(Error::decode("Expected bytes, a number or a list.", 0)),
        }
    }

    fn deserialize_seq<V: Visitor<'de>>(self, visitor: V) -> Result<V::Value, Error> {
        match self.0 {
            Value::Bytes(bytes) => visitor.visit_seq(de::value::SeqDeserializer::<_, Error>::new(
                bytes.into_iter(),
            )),
            _ => Err(Error::decode("Expected bytes.", 0)),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string bytes byte_buf option unit unit_struct
        newtype_struct tuple tuple_struct map struct enum identifier ignored_any
    }
}

struct RefSeq<'de> {
    items: core::slice::Iter<'de, Value>,
}

impl<'de> de::SeqAccess<'de> for RefSeq<'de> {
    type Error = Error;

    fn next_element_seed<T: DeserializeSeed<'de>>(
        &mut self,
        seed: T,
    ) -> Result<Option<T::Value>, Error> {
        match self.items.next() {
            Some(item) => seed.deserialize(ValueDeserializer(item)).map(Some),
            None => Ok(None),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.items.len())
    }
}

struct RefMap<'de> {
    members: core::slice::Iter<'de, (String, Value)>,
    value: Option<&'de Value>,
}

impl<'de> de::MapAccess<'de> for RefMap<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        match self.members.next() {
            Some((name, value)) => {
                self.value = Some(value);

                seed.deserialize(name.as_str().into_deserializer())
                    .map(Some)
            }
            None => Ok(None),
        }
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        match self.value.take() {
            Some(value) => seed.deserialize(ValueDeserializer(value)),
            None => Err(Error::decode("A value comes after its name.", 0)),
        }
    }

    fn size_hint(&self) -> Option<usize> {
        Some(self.members.len())
    }
}

/// A CHOICE as a map of its one alternative.
struct ChoiceMap<'de> {
    name: &'de String,
    value: &'de Value,
    done: bool,
}

impl<'de> de::MapAccess<'de> for ChoiceMap<'de> {
    type Error = Error;

    fn next_key_seed<K: DeserializeSeed<'de>>(
        &mut self,
        seed: K,
    ) -> Result<Option<K::Value>, Error> {
        if self.done {
            return Ok(None);
        }

        self.done = true;

        seed.deserialize(self.name.as_str().into_deserializer())
            .map(Some)
    }

    fn next_value_seed<V: DeserializeSeed<'de>>(&mut self, seed: V) -> Result<V::Value, Error> {
        seed.deserialize(ValueDeserializer(self.value))
    }
}

struct ChoiceAccess<'de> {
    name: &'de String,
    value: &'de Value,
}

impl<'de> de::EnumAccess<'de> for ChoiceAccess<'de> {
    type Error = Error;
    type Variant = ChoiceVariant<'de>;

    fn variant_seed<V: DeserializeSeed<'de>>(
        self,
        seed: V,
    ) -> Result<(V::Value, ChoiceVariant<'de>), Error> {
        let name = seed.deserialize(self.name.as_str().into_deserializer())?;

        Ok((name, ChoiceVariant(self.value)))
    }
}

struct ChoiceVariant<'de>(&'de Value);

impl<'de> de::VariantAccess<'de> for ChoiceVariant<'de> {
    type Error = Error;

    fn unit_variant(self) -> Result<(), Error> {
        match self.0 {
            Value::Null => Ok(()),
            _ => Err(Error::decode("Expected NULL.", 0)),
        }
    }

    fn newtype_variant_seed<T: DeserializeSeed<'de>>(self, seed: T) -> Result<T::Value, Error> {
        seed.deserialize(ValueDeserializer(self.0))
    }

    fn tuple_variant<V: Visitor<'de>>(self, _: usize, visitor: V) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_seq(ValueDeserializer(self.0), visitor)
    }

    fn struct_variant<V: Visitor<'de>>(
        self,
        _: &'static [&'static str],
        visitor: V,
    ) -> Result<V::Value, Error> {
        de::Deserializer::deserialize_map(ValueDeserializer(self.0), visitor)
    }
}
