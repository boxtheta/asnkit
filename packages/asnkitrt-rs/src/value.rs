// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The values that are encoded and decoded.
//!
//! The runtime for Python has the same, with its own types: a
//! SEQUENCE is a `dict`, a CHOICE a `tuple(str, object)` and so on.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

/// A date, as `datetime.date` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Date {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A time of day, as `datetime.time` is. `offset` is minutes east of
/// UTC if the time is not naive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Time {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub microsecond: u32,
    pub offset: Option<i32>,
}

/// A date and a time, as `datetime.datetime` is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTime {
    pub date: Date,
    pub time: Time,
}

/// A value of an ASN.1 type.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// NULL. Also what an unknown value of an extensible ENUMERATED is.
    Null,
    Boolean(bool),
    /// INTEGER, also a named number when encoding.
    Integer(i128),
    /// An INTEGER that does not fit in 128 bits, as its two's
    /// complement, most significant byte first and as short as it can
    /// be.
    BigInteger(Vec<u8>),
    Real(f64),
    /// A character string, an OBJECT IDENTIFIER as `1.2.3`, and an
    /// ENUMERATED by name. (With `numeric_enums` it is an `Integer`.)
    Text(String),
    /// OCTET STRING.
    Bytes(Vec<u8>),
    /// BIT STRING: its bytes, and how many bits of them there are.
    Bits(Vec<u8>, usize),
    /// SEQUENCE OF and SET OF.
    List(Vec<Value>),
    /// SEQUENCE and SET: the members that are there.
    Map(Vec<(String, Value)>),
    /// CHOICE: the name of the alternative and its value.
    Choice(String, Box<Value>),
    DateTime(DateTime),
    Date(Date),
    Time(Time),
}

impl Value {
    pub fn text(text: &str) -> Value {
        Value::Text(String::from(text))
    }

    /// The member of a map with this name.
    pub fn get(&self, name: &str) -> Option<&Value> {
        match self {
            Value::Map(members) => members
                .iter()
                .find(|(member, _)| member == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }

    /// An integer from its two's complement, most significant byte
    /// first. Nothing at all is 0.
    pub fn from_twos_complement(bytes: &[u8]) -> Value {
        // Leading bytes that are only the sign add nothing.
        let mut start = 0;

        while bytes.len() - start > 1
            && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
                || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
        {
            start += 1;
        }

        let bytes = &bytes[start..];

        if bytes.len() > 16 {
            return Value::BigInteger(Vec::from(bytes));
        }

        let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);
        let mut value: i128 = if negative { -1 } else { 0 };

        for byte in bytes {
            value = (value << 8) | i128::from(*byte);
        }

        Value::Integer(value)
    }

    /// The two's complement of an integer, as few bytes as it can be.
    pub fn to_twos_complement(&self) -> Option<Vec<u8>> {
        match self {
            Value::Integer(value) => Some(integer_to_twos_complement(*value)),
            Value::BigInteger(bytes) => Some(bytes.clone()),
            _ => None,
        }
    }
}

/// The two's complement of `value`, as few bytes as it can be.
pub fn integer_to_twos_complement(value: i128) -> Vec<u8> {
    let bytes = value.to_be_bytes();
    let mut start = 0;

    while start < 15
        && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }

    Vec::from(&bytes[start..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn twos_complement() {
        for (value, bytes) in [
            (0i128, &[0x00][..]),
            (1, &[0x01]),
            (127, &[0x7f]),
            (128, &[0x00, 0x80]),
            (255, &[0x00, 0xff]),
            (256, &[0x01, 0x00]),
            (-1, &[0xff]),
            (-128, &[0x80]),
            (-129, &[0xff, 0x7f]),
            (-256, &[0xff, 0x00]),
        ] {
            assert_eq!(integer_to_twos_complement(value), bytes, "{value}");
            assert_eq!(
                Value::from_twos_complement(bytes),
                Value::Integer(value),
                "{value}"
            );
        }

        assert_eq!(Value::from_twos_complement(&[]), Value::Integer(0));
        assert_eq!(Value::from_twos_complement(&[0, 0, 5]), Value::Integer(5));
        assert_eq!(
            Value::from_twos_complement(&[0xff, 0xff, 0xfb]),
            Value::Integer(-5)
        );
        assert_eq!(integer_to_twos_complement(i128::MIN).len(), 16);
        assert_eq!(integer_to_twos_complement(i128::MAX).len(), 16);

        let big = [0x01u8; 17];
        assert_eq!(
            Value::from_twos_complement(&big),
            Value::BigInteger(Vec::from(big))
        );
        assert_eq!(
            Value::BigInteger(Vec::from(big)).to_twos_complement(),
            Some(Vec::from(big))
        );
    }
}
