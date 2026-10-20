// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The values that are encoded and decoded.
//!
//! The runtime for Python has the same, with its own types: a
//! SEQUENCE is a `dict`, a CHOICE a `tuple(str, object)` and so on.

use alloc::borrow::Cow;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;
use core::str::FromStr;

use crate::{Error, Result};

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
///
/// The names of the members of a map and of the alternative of a choice
/// are borrowed for `'a`: a decoder borrows them from the
/// [`Specification`](crate::runtime::Specification), and a caller
/// building a value writes `("id".into(), …)`, which borrows a `&str` or
/// takes a `String`. [`Value::into_owned`] makes a value that borrows
/// nothing, to keep it past what it borrows from.
#[derive(Debug, Clone, PartialEq)]
pub enum Value<'a> {
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
    /// A character string, and an ENUMERATED by name. (With
    /// `numeric_enums` it is an `Integer`.)
    Text(String),
    /// OBJECT IDENTIFIER. The runtime for Python has it as a `str`.
    ObjectIdentifier(ObjectIdentifier<'a>),
    /// OCTET STRING.
    Bytes(Vec<u8>),
    /// BIT STRING: its bytes, and how many bits of them there are.
    Bits(Vec<u8>, usize),
    /// SEQUENCE OF and SET OF.
    List(Vec<Value<'a>>),
    /// SEQUENCE and SET: the members that are there.
    Map(Vec<(Cow<'a, str>, Value<'a>)>),
    /// CHOICE: the name of the alternative and its value.
    Choice(Cow<'a, str>, Box<Value<'a>>),
    DateTime(DateTime),
    Date(Date),
    Time(Time),
}

impl<'a> Value<'a> {
    pub fn text(text: &str) -> Value<'a> {
        Value::Text(String::from(text))
    }

    /// The same value, borrowing nothing.
    pub fn into_owned(self) -> Value<'static> {
        match self {
            Value::Null => Value::Null,
            Value::Boolean(value) => Value::Boolean(value),
            Value::Integer(value) => Value::Integer(value),
            Value::BigInteger(bytes) => Value::BigInteger(bytes),
            Value::Real(value) => Value::Real(value),
            Value::Text(text) => Value::Text(text),
            Value::ObjectIdentifier(oid) => Value::ObjectIdentifier(oid.into_owned()),
            Value::Bytes(bytes) => Value::Bytes(bytes),
            Value::Bits(bytes, bits) => Value::Bits(bytes, bits),
            Value::List(items) => Value::List(items.into_iter().map(Value::into_owned).collect()),
            Value::Map(members) => Value::Map(
                members
                    .into_iter()
                    .map(|(name, value)| (Cow::Owned(name.into_owned()), value.into_owned()))
                    .collect(),
            ),
            Value::Choice(name, value) => {
                Value::Choice(Cow::Owned(name.into_owned()), Box::new(value.into_owned()))
            }
            Value::DateTime(value) => Value::DateTime(value),
            Value::Date(value) => Value::Date(value),
            Value::Time(value) => Value::Time(value),
        }
    }

    /// The member of a map with this name.
    pub fn get(&self, name: &str) -> Option<&Value<'a>> {
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
    pub fn from_twos_complement(bytes: &[u8]) -> Value<'a> {
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

/// An OBJECT IDENTIFIER, as the contents octets that BER, DER, OER, PER
/// and UPER all encode it as (X.690 8.19): its subidentifiers, each in
/// as few bytes as it can be, of which the first is that of the first
/// two arcs. A decoder borrows them from what it decodes, and the text,
/// as `1.2.840.113549`, is made only when it is asked for, with
/// [`Display`](fmt::Display), and read with [`FromStr`].
///
/// The bytes are always those of an OBJECT IDENTIFIER, so two are equal
/// if and only if their bytes are.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ObjectIdentifier<'a>(Cow<'a, [u8]>);

/// What is wrong with the contents octets of an OBJECT IDENTIFIER, and
/// at which of them, as `ber::decode_object_identifier` told before.
fn check_object_identifier(
    bytes: &[u8],
    minimal: bool,
) -> core::result::Result<(), (&'static str, usize)> {
    let mut arc: u128 = 0;
    let mut start = true;

    for (index, byte) in bytes.iter().enumerate() {
        // A subidentifier is in as few bytes as possible.
        if minimal && start && *byte == 0x80 {
            return Err((
                "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible.",
                index,
            ));
        }

        if arc >> 120 != 0 {
            return Err(("An arc of an OBJECT IDENTIFIER is too large.", index));
        }

        start = byte & 0x80 == 0;
        arc = if start {
            0
        } else {
            (arc << 7) | u128::from(byte & 0x7f)
        };
    }

    if bytes.is_empty() || !start {
        return Err(("Expected an OBJECT IDENTIFIER.", 0));
    }

    Ok(())
}

impl<'a> ObjectIdentifier<'a> {
    /// The OBJECT IDENTIFIER of these contents octets, if they are those
    /// of one: subidentifiers in as few bytes as they can be, of at most
    /// 127 bits.
    pub fn from_bytes(bytes: impl Into<Cow<'a, [u8]>>) -> Result<ObjectIdentifier<'a>> {
        let bytes = bytes.into();

        check_object_identifier(&bytes, true)
            .map_err(|(what, index)| Error::decode(what, index))?;

        Ok(ObjectIdentifier(bytes))
    }

    /// The OBJECT IDENTIFIER of the contents octets of a decoder, at
    /// `offset` of the data. BER allows a subidentifier in more bytes
    /// than it needs, but for `strict`, which is made the bytes it needs.
    pub(crate) fn decode(
        bytes: impl Into<Cow<'a, [u8]>>,
        offset: usize,
        strict: bool,
    ) -> Result<ObjectIdentifier<'a>> {
        let bytes = bytes.into();

        match check_object_identifier(&bytes, true) {
            Ok(()) => Ok(ObjectIdentifier(bytes)),
            Err((_, index)) if !strict && bytes.get(index) == Some(&0x80) => {
                check_object_identifier(&bytes, false)
                    .map_err(|(what, index)| Error::decode(what, offset + index))?;

                // The bytes are made again, as few as they can be.
                let mut minimal = Vec::with_capacity(bytes.len());
                for subidentifier in (Subidentifiers { bytes: &bytes }) {
                    push_subidentifier(subidentifier, &mut minimal);
                }

                Ok(ObjectIdentifier(Cow::Owned(minimal)))
            }
            Err((what, index)) => Err(Error::decode(what, offset + index)),
        }
    }

    /// The contents octets.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The contents octets, borrowed if they are.
    pub fn into_bytes(self) -> Cow<'a, [u8]> {
        self.0
    }

    /// The same OBJECT IDENTIFIER, borrowing nothing.
    pub fn into_owned(self) -> ObjectIdentifier<'static> {
        ObjectIdentifier(Cow::Owned(self.0.into_owned()))
    }

    /// The arcs, of which there are at least two.
    pub fn arcs(&self) -> Arcs<'_> {
        Arcs {
            subidentifiers: self.subidentifiers(),
            first: true,
            second: None,
        }
    }

    fn subidentifiers(&self) -> Subidentifiers<'_> {
        Subidentifiers { bytes: &self.0 }
    }
}

/// The digits of `value`, as `write!` has them, without its machinery.
pub(crate) fn push_decimal(value: u128, text: &mut String) {
    let mut digits = [0u8; 39];
    let mut at = digits.len();
    let mut value = value;

    loop {
        at -= 1;
        digits[at] = b'0' + (value % 10) as u8;
        value /= 10;

        if value == 0 {
            break;
        }
    }

    // They are ASCII digits.
    text.push_str(core::str::from_utf8(&digits[at..]).unwrap_or_default());
}

impl fmt::Display for ObjectIdentifier<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // A byte is 7 bits of an arc, which are at most 3 digits, and the
        // first arc is two.
        let mut text = String::with_capacity(3 * self.0.len() + 2);

        for (index, arc) in self.arcs().enumerate() {
            if index > 0 {
                text.push('.');
            }

            push_decimal(arc, &mut text);
        }

        f.write_str(&text)
    }
}

impl fmt::Debug for ObjectIdentifier<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ObjectIdentifier({self})")
    }
}

/// An OBJECT IDENTIFIER of its arcs, as `1.2.840.113549`.
impl FromStr for ObjectIdentifier<'static> {
    type Err = Error;

    fn from_str(text: &str) -> Result<ObjectIdentifier<'static>> {
        // Most arcs are a byte or two.
        let mut out = Vec::with_capacity(text.len());
        let mut parts = text.split('.');
        let mut arc = || -> Result<Option<u128>> {
            parts
                .next()
                .map(|part| {
                    // Parsing a u64 is faster, and an arc is all but always one.
                    part.parse::<u64>()
                        .map(u128::from)
                        .or_else(|_| part.parse::<u128>())
                        .map_err(|_| Error::encode("Expected an OBJECT IDENTIFIER."))
                })
                .transpose()
        };
        let (Some(first), Some(second)) = (arc()?, arc()?) else {
            return Err(Error::encode(
                "Expected an OBJECT IDENTIFIER of at least two arcs.",
            ));
        };
        let too_large = || Error::encode("An arc of an OBJECT IDENTIFIER is too large.");
        // A subidentifier is read back if it is of at most 127 bits.
        let push = |subidentifier: u128, out: &mut Vec<u8>| {
            if subidentifier >> 127 != 0 {
                return Err(too_large());
            }

            push_subidentifier(subidentifier, out);

            Ok(())
        };

        push(
            first
                .checked_mul(40)
                .and_then(|first| first.checked_add(second))
                .ok_or_else(too_large)?,
            &mut out,
        )?;

        while let Some(arc) = arc()? {
            push(arc, &mut out)?;
        }

        Ok(ObjectIdentifier(Cow::Owned(out)))
    }
}

struct Subidentifiers<'o> {
    bytes: &'o [u8],
}

impl Iterator for Subidentifiers<'_> {
    type Item = u128;

    fn next(&mut self) -> Option<u128> {
        let mut value: u128 = 0;

        while let Some((byte, rest)) = self.bytes.split_first() {
            self.bytes = rest;
            value = (value << 7) | u128::from(byte & 0x7f);

            if byte & 0x80 == 0 {
                return Some(value);
            }
        }

        None
    }
}

/// The arcs of an [`ObjectIdentifier`].
pub struct Arcs<'o> {
    subidentifiers: Subidentifiers<'o>,
    first: bool,
    second: Option<u128>,
}

impl Iterator for Arcs<'_> {
    type Item = u128;

    fn next(&mut self) -> Option<u128> {
        if let Some(second) = self.second.take() {
            return Some(second);
        }

        let subidentifier = self.subidentifiers.next()?;

        if !self.first {
            return Some(subidentifier);
        }

        // The first subidentifier is of the first two arcs (X.690 8.19.4).
        self.first = false;

        let (first, second) = match subidentifier {
            0..40 => (0, subidentifier),
            40..80 => (1, subidentifier - 40),
            _ => (2, subidentifier - 80),
        };

        self.second = Some(second);

        Some(first)
    }
}

/// A subidentifier: groups of 7 bits, the most significant first, all
/// but the last with the high bit set.
pub(crate) fn push_subidentifier(arc: u128, out: &mut Vec<u8>) {
    let groups = (128 - arc.leading_zeros() as usize).div_ceil(7).max(1);

    for group in (0..groups).rev() {
        let bits = (arc >> (7 * group)) as u8 & 0x7f;

        out.push(if group == 0 { bits } else { 0x80 | bits });
    }
}

/// Looks the members of a map up by name, in the order they are
/// declared. A map built by a decoder, by serde or by most callers has
/// them in that order, so the entry after the last one found is tried
/// before the others are searched.
pub(crate) struct Lookup<'m, 'a> {
    pub map: &'m [(Cow<'a, str>, Value<'a>)],
    next: usize,
}

impl<'m, 'a> Lookup<'m, 'a> {
    pub fn new(map: &'m [(Cow<'a, str>, Value<'a>)]) -> Lookup<'m, 'a> {
        Lookup { map, next: 0 }
    }

    pub fn find(&mut self, name: &str) -> Option<&'m Value<'a>> {
        let index = match self.map.get(self.next) {
            Some((candidate, _)) if candidate == name => self.next,
            _ => self
                .map
                .iter()
                .position(|(candidate, _)| candidate == name)?,
        };

        self.next = index + 1;

        Some(&self.map[index].1)
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
    use alloc::format;
    use alloc::string::ToString;

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

    #[test]
    fn lookup() {
        let map = [
            ("a".into(), Value::Integer(1)),
            ("c".into(), Value::Integer(3)),
            ("b".into(), Value::Integer(2)),
        ];
        let mut lookup = Lookup::new(&map);

        // In order, out of order, absent, and again after the end.
        assert_eq!(lookup.find("a"), Some(&Value::Integer(1)));
        assert_eq!(lookup.find("b"), Some(&Value::Integer(2)));
        assert_eq!(lookup.find("x"), None);
        assert_eq!(lookup.find("c"), Some(&Value::Integer(3)));
        assert_eq!(lookup.find("b"), Some(&Value::Integer(2)));
        assert_eq!(lookup.find("a"), Some(&Value::Integer(1)));
        assert_eq!(Lookup::new(&[]).find("a"), None);
    }

    fn oid(text: &str) -> ObjectIdentifier<'static> {
        text.parse().unwrap()
    }

    #[test]
    fn object_identifiers_round_trip() {
        let largest = (1u128 << 127) - 1;

        for text in [
            "0.0".to_string(),
            "0.39".to_string(),
            "1.2".to_string(),
            "2.999.3".to_string(),
            "1.2.840.113549.1.1.11".to_string(),
            format!("1.3.{largest}"),
            format!("2.{}", largest - 80),
        ] {
            let parsed = oid(&text);

            assert_eq!(parsed.to_string(), text);
            assert_eq!(
                ObjectIdentifier::from_bytes(parsed.as_bytes()).unwrap(),
                parsed
            );
            assert_eq!(parsed.arcs().count(), text.split('.').count());
        }

        assert_eq!(
            oid("1.2.840.113549").as_bytes(),
            [0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d]
        );
        assert_eq!(oid("1.2.840").arcs().collect::<Vec<_>>(), [1, 2, 840]);

        // The first subidentifier is of the first two arcs.
        for (bytes, text) in [
            (&[0x00][..], "0.0"),
            (&[0x28], "1.0"),
            (&[0x50], "2.0"),
            (&[0x81, 0x00], "2.48"),
        ] {
            assert_eq!(
                ObjectIdentifier::from_bytes(bytes).unwrap().to_string(),
                text
            );
        }

        assert_eq!(format!("{:?}", oid("1.2")), "ObjectIdentifier(1.2)");
    }

    #[test]
    fn object_identifiers_are_rejected_as_ber_rejected_them() {
        let mut too_large = alloc::vec![0x82];
        too_large.extend([0xff; 17]);
        too_large.push(0x7f);

        let mut largest = alloc::vec![0x81];
        largest.extend([0xff; 17]);
        largest.push(0x7f);

        assert!(ObjectIdentifier::from_bytes(largest.as_slice()).is_ok());

        for (bytes, what) in [
            (&[][..], "Expected an OBJECT IDENTIFIER."),
            (&[0x2a, 0x86], "Expected an OBJECT IDENTIFIER."),
            (
                &[0x80, 0x01],
                "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible.",
            ),
            (
                &[0x2a, 0x80, 0x01],
                "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible.",
            ),
            (&too_large, "An arc of an OBJECT IDENTIFIER is too large."),
        ] {
            match ObjectIdentifier::from_bytes(bytes) {
                Err(Error::Decode(found, ..)) => assert_eq!(found, what, "{bytes:02x?}"),
                other => panic!("{bytes:02x?}: {other:?}"),
            }
        }

        let largest = (1u128 << 127) - 1;

        for text in [
            "".to_string(),
            "1".to_string(),
            "1.".to_string(),
            "1..2".to_string(),
            "a.b".to_string(),
            "1.2.-3".to_string(),
            format!("1.2.{}", largest + 1),
            format!("2.{}", largest - 79),
        ] {
            assert!(text.parse::<ObjectIdentifier>().is_err(), "{text}");
        }
    }

    #[test]
    fn ber_object_identifiers_are_made_minimal() {
        let bytes = [0x2a, 0x80, 0x86, 0x48];

        assert!(ObjectIdentifier::decode(&bytes[..], 0, true).is_err());

        let decoded = ObjectIdentifier::decode(&bytes[..], 0, false).unwrap();

        assert_eq!(decoded, oid("1.2.840"));
        assert!(matches!(decoded.into_bytes(), Cow::Owned(_)));
        assert!(matches!(
            ObjectIdentifier::decode(&bytes[2..], 0, false)
                .unwrap()
                .into_bytes(),
            Cow::Borrowed(_)
        ));

        // Its offset is that of the data.
        assert!(matches!(
            ObjectIdentifier::decode(&[0x2a, 0x86][..], 10, false),
            Err(Error::Decode(_, 10, _))
        ));
    }
}
