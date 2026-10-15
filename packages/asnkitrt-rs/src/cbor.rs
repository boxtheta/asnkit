// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The part of CBOR (RFC 8949) the files use: integers, floating point
//! numbers, text strings, booleans, null, arrays and maps with text
//! strings as keys, all of a known length. Bignums are the only byte
//! strings and the only tags.

use alloc::string::String;
use alloc::vec::Vec;

use crate::{Error, Result};

/// How deep a reader goes into arrays and maps by default. The nodes
/// of the compiled specifications in this repository go to 15.
pub const DEFAULT_MAX_DEPTH: usize = 64;

/// A CBOR item.
#[derive(Debug, Clone, PartialEq)]
pub enum Cbor {
    Null,
    Bool(bool),
    /// An integer of up to 64 bits, positive or negative.
    Int(i128),
    /// An integer of more than 64 bits, as the sign and the value
    /// without it, most significant byte first and with no leading
    /// zeros.
    Big {
        negative: bool,
        magnitude: Vec<u8>,
    },
    Float(f64),
    Text(String),
    Array(Vec<Cbor>),
    Map(Vec<(String, Cbor)>),
}

impl Cbor {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Cbor::Text(text) => Some(text),
            _ => None,
        }
    }

    pub fn as_array(&self) -> Option<&[Cbor]> {
        match self {
            Cbor::Array(items) => Some(items),
            _ => None,
        }
    }

    pub fn as_int(&self) -> Option<i128> {
        match self {
            Cbor::Int(value) => Some(*value),
            _ => None,
        }
    }

    /// The value of a key of a map. If it is there more than once, the
    /// last, as the reader of Python does.
    pub fn get(&self, key: &str) -> Option<&Cbor> {
        match self {
            Cbor::Map(members) => members
                .iter()
                .rev()
                .find(|(name, _)| name == key)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

/// Reads the item at `offset` of `data`, and returns it with the
/// offset of what follows. Arrays and maps may be nested `max_depth`
/// deep.
pub fn parse_first(data: &[u8], offset: usize, max_depth: usize) -> Result<(Cbor, usize)> {
    let mut parser = Parser { data, offset };
    let item = parser.item(max_depth)?;

    Ok((item, parser.offset))
}

/// Reads `data` as one item and nothing else.
pub fn parse(data: &[u8], max_depth: usize) -> Result<Cbor> {
    let (item, offset) = parse_first(data, 0, max_depth)?;

    if offset != data.len() {
        return Err(Error::TrailingData);
    }

    Ok(item)
}

const MAJOR_UNSIGNED: u8 = 0;
const MAJOR_NEGATIVE: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;
const MAJOR_TAG: u8 = 6;
const MAJOR_SIMPLE: u8 = 7;

const TAG_POSITIVE_BIGNUM: u64 = 2;
const TAG_NEGATIVE_BIGNUM: u64 = 3;

struct Parser<'a> {
    data: &'a [u8],
    offset: usize,
}

impl<'a> Parser<'a> {
    fn take(&mut self, size: usize) -> Result<&'a [u8]> {
        // The size is from the file, so it can not be added to the
        // offset without looking.
        if size > self.data.len() - self.offset {
            return Err(Error::OutOfData);
        }

        self.offset += size;

        Ok(&self.data[self.offset - size..self.offset])
    }

    /// The major type, the additional information and its argument.
    fn head(&mut self) -> Result<(u8, u8, u64)> {
        let initial = self.take(1)?[0];
        let info = initial & 0x1f;
        let argument = match info {
            0..=23 => u64::from(info),
            24..=27 => {
                let bytes = self.take(1 << (info - 24))?;

                bytes
                    .iter()
                    .fold(0, |value, byte| (value << 8) | u64::from(*byte))
            }
            // Reserved, and the indefinite lengths.
            _ => return Err(Error::UnsupportedCbor),
        };

        Ok((initial >> 5, info, argument))
    }

    /// The size of a string, or the count of items of an array or a
    /// map. As no item is less than a byte, a count that is more than
    /// what is left is out of data, which keeps what a file asks to
    /// be allocated to the size of the file.
    fn length(&self, argument: u64, per_item: usize) -> Result<usize> {
        let left = self.data.len() - self.offset;

        match usize::try_from(argument) {
            Ok(length) if length <= left / per_item => Ok(length),
            _ => Err(Error::OutOfData),
        }
    }

    fn item(&mut self, depth: usize) -> Result<Cbor> {
        let (major, info, argument) = self.head()?;

        match major {
            MAJOR_UNSIGNED => Ok(Cbor::Int(i128::from(argument))),
            MAJOR_NEGATIVE => Ok(Cbor::Int(-1 - i128::from(argument))),
            MAJOR_TEXT => {
                let size = self.length(argument, 1)?;
                let text = core::str::from_utf8(self.take(size)?).map_err(|_| Error::BadText)?;

                Ok(Cbor::Text(String::from(text)))
            }
            MAJOR_ARRAY => {
                let depth = depth.checked_sub(1).ok_or(Error::TooDeep)?;
                let count = self.length(argument, 1)?;
                let mut items = Vec::with_capacity(count);

                for _ in 0..count {
                    items.push(self.item(depth)?);
                }

                Ok(Cbor::Array(items))
            }
            MAJOR_MAP => {
                let depth = depth.checked_sub(1).ok_or(Error::TooDeep)?;
                // A key and a value are two bytes at the least.
                let count = self.length(argument, 2)?;
                let mut members = Vec::with_capacity(count);

                for _ in 0..count {
                    let Cbor::Text(key) = self.item(depth)? else {
                        return Err(Error::UnsupportedCbor);
                    };

                    members.push((key, self.item(depth)?));
                }

                Ok(Cbor::Map(members))
            }
            MAJOR_TAG if argument == TAG_POSITIVE_BIGNUM || argument == TAG_NEGATIVE_BIGNUM => {
                let (major, _, size) = self.head()?;

                if major != MAJOR_BYTES {
                    return Err(Error::UnsupportedCbor);
                }

                let size = self.length(size, 1)?;

                Ok(bignum(self.take(size)?, argument == TAG_NEGATIVE_BIGNUM))
            }
            MAJOR_SIMPLE => match info {
                20 => Ok(Cbor::Bool(false)),
                21 => Ok(Cbor::Bool(true)),
                22 => Ok(Cbor::Null),
                25 => Ok(Cbor::Float(half(argument as u16))),
                26 => Ok(Cbor::Float(f64::from(f32::from_bits(argument as u32)))),
                27 => Ok(Cbor::Float(f64::from_bits(argument))),
                _ => Err(Error::UnsupportedCbor),
            },
            _ => Err(Error::UnsupportedCbor),
        }
    }
}

/// A bignum is the number, or one less than minus the number.
fn bignum(bytes: &[u8], negative: bool) -> Cbor {
    let start = bytes
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(bytes.len());
    let mut magnitude = Vec::from(&bytes[start..]);

    if magnitude.len() <= 8 {
        let value = magnitude
            .iter()
            .fold(0i128, |value, byte| (value << 8) | i128::from(*byte));

        return Cbor::Int(if negative { -1 - value } else { value });
    }

    if negative {
        // One more than the number, which carries into a new byte if
        // it is all ones.
        let mut carry = true;

        for byte in magnitude.iter_mut().rev() {
            let (sum, overflow) = byte.overflowing_add(u8::from(carry));
            *byte = sum;
            carry = overflow;
        }

        if carry {
            magnitude.insert(0, 1);
        }
    }

    Cbor::Big {
        negative,
        magnitude,
    }
}

/// Two to the power of `exponent`, which is a normal number for any
/// that is used here. (`powi` is not in `core`.)
fn pow2(exponent: i32) -> f64 {
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// A floating point number of 16 bits (IEEE 754 binary16).
fn half(bits: u16) -> f64 {
    let sign = if bits & 0x8000 != 0 { -1.0 } else { 1.0 };
    let exponent = i32::from((bits >> 10) & 0x1f);
    let fraction = f64::from(bits & 0x3ff);

    match exponent {
        0 => sign * fraction * pow2(-24),
        31 if fraction == 0.0 => sign * f64::INFINITY,
        31 => f64::NAN,
        _ => sign * (1.0 + fraction / 1024.0) * pow2(exponent - 15),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_hex(hex: &str) -> Result<Cbor> {
        let bytes: Vec<u8> = (0..hex.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
            .collect();

        parse(&bytes, DEFAULT_MAX_DEPTH)
    }

    #[test]
    fn integers() {
        assert_eq!(parse_hex("00"), Ok(Cbor::Int(0)));
        assert_eq!(parse_hex("17"), Ok(Cbor::Int(23)));
        assert_eq!(parse_hex("1818"), Ok(Cbor::Int(24)));
        assert_eq!(parse_hex("1903e8"), Ok(Cbor::Int(1000)));
        assert_eq!(parse_hex("1a000f4240"), Ok(Cbor::Int(1_000_000)));
        assert_eq!(
            parse_hex("1bffffffffffffffff"),
            Ok(Cbor::Int(u64::MAX.into()))
        );
        assert_eq!(parse_hex("20"), Ok(Cbor::Int(-1)));
        assert_eq!(parse_hex("3903e7"), Ok(Cbor::Int(-1000)));
        assert_eq!(
            parse_hex("3bffffffffffffffff"),
            Ok(Cbor::Int(-1 - i128::from(u64::MAX)))
        );
    }

    #[test]
    fn bignums() {
        // 2**64 and -2**64 - 1, from RFC 8949 appendix A.
        let big = Cbor::Big {
            negative: false,
            magnitude: Vec::from([1, 0, 0, 0, 0, 0, 0, 0, 0]),
        };
        assert_eq!(parse_hex("c249010000000000000000"), Ok(big));
        let big = Cbor::Big {
            negative: true,
            magnitude: Vec::from([1, 0, 0, 0, 0, 0, 0, 0, 1]),
        };
        assert_eq!(parse_hex("c349010000000000000000"), Ok(big));
        // All ones carries into a new byte: -(2**72 - 1) - 1 = -2**72.
        let big = Cbor::Big {
            negative: true,
            magnitude: Vec::from([1, 0, 0, 0, 0, 0, 0, 0, 0, 0]),
        };
        assert_eq!(parse_hex("c349ffffffffffffffffff"), Ok(big));
        // Leading zeros do not make it big.
        assert_eq!(parse_hex("c24900000000000000000a"), Ok(Cbor::Int(10)));
        assert_eq!(parse_hex("c340"), Ok(Cbor::Int(-1)));
        assert_eq!(parse_hex("c240"), Ok(Cbor::Int(0)));
    }

    #[test]
    fn floats() {
        assert_eq!(parse_hex("f90000"), Ok(Cbor::Float(0.0)));
        assert_eq!(parse_hex("f93c00"), Ok(Cbor::Float(1.0)));
        assert_eq!(parse_hex("f93e00"), Ok(Cbor::Float(1.5)));
        assert_eq!(parse_hex("f97bff"), Ok(Cbor::Float(65504.0)));
        assert_eq!(parse_hex("f90001"), Ok(Cbor::Float(5.960464477539063e-8)));
        assert_eq!(parse_hex("f9c400"), Ok(Cbor::Float(-4.0)));
        assert_eq!(parse_hex("f97c00"), Ok(Cbor::Float(f64::INFINITY)));
        assert_eq!(parse_hex("f9fc00"), Ok(Cbor::Float(f64::NEG_INFINITY)));
        assert_eq!(parse_hex("fa47c35000"), Ok(Cbor::Float(100000.0)));
        assert_eq!(parse_hex("fb3ff199999999999a"), Ok(Cbor::Float(1.1)));
        assert!(matches!(parse_hex("f97e00"), Ok(Cbor::Float(value)) if value.is_nan()));
    }

    #[test]
    fn simple_and_text() {
        assert_eq!(parse_hex("f4"), Ok(Cbor::Bool(false)));
        assert_eq!(parse_hex("f5"), Ok(Cbor::Bool(true)));
        assert_eq!(parse_hex("f6"), Ok(Cbor::Null));
        assert_eq!(parse_hex("60"), Ok(Cbor::Text(String::new())));
        assert_eq!(
            parse_hex("6449455446"),
            Ok(Cbor::Text(String::from("IETF")))
        );
        assert_eq!(parse_hex("62c3bc"), Ok(Cbor::Text(String::from("\u{fc}"))));
    }

    #[test]
    fn containers() {
        assert_eq!(parse_hex("80"), Ok(Cbor::Array(Vec::new())));
        assert_eq!(
            parse_hex("83010203"),
            Ok(Cbor::Array(Vec::from([
                Cbor::Int(1),
                Cbor::Int(2),
                Cbor::Int(3)
            ])))
        );

        let map = parse_hex("a2616101616202").unwrap();
        assert_eq!(map.get("a"), Some(&Cbor::Int(1)));
        assert_eq!(map.get("b"), Some(&Cbor::Int(2)));
        assert_eq!(map.get("c"), None);

        // The last of a key that is there twice.
        let map = parse_hex("a2616101616102").unwrap();
        assert_eq!(map.get("a"), Some(&Cbor::Int(2)));
    }

    #[test]
    fn rejected() {
        // Out of data.
        assert_eq!(parse_hex(""), Err(Error::OutOfData));
        assert_eq!(parse_hex("18"), Err(Error::OutOfData));
        assert_eq!(parse_hex("6161ff").unwrap_err(), Error::TrailingData);
        assert_eq!(parse_hex("62"), Err(Error::OutOfData));
        assert_eq!(parse_hex("8201"), Err(Error::OutOfData));
        // A count that is more than the file is long, without memory
        // being asked for.
        assert_eq!(parse_hex("9bffffffffffffffff"), Err(Error::OutOfData));
        assert_eq!(parse_hex("7bffffffffffffffff"), Err(Error::OutOfData));
        assert_eq!(parse_hex("a2616101"), Err(Error::OutOfData));
        // Indefinite lengths, byte strings, other tags, other simple
        // values, and a map with a key that is not text.
        assert_eq!(parse_hex("9f01ff"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("5f"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("4101"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("c101"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("c26161"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("f7"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("f0"), Err(Error::UnsupportedCbor));
        assert_eq!(parse_hex("a10101"), Err(Error::UnsupportedCbor));
        // Text that is not UTF-8.
        assert_eq!(parse_hex("61ff"), Err(Error::BadText));
    }

    #[test]
    fn depth() {
        let data = [0x81u8; 10];
        let mut nested = Vec::from(data);
        nested.push(0x80);

        assert!(parse(&nested, 11).is_ok());
        assert_eq!(parse(&nested, 10), Err(Error::TooDeep));
        // Deep enough to overflow the stack, if there were no limit.
        let deep = alloc::vec![0x81u8; 1_000_000];
        assert_eq!(parse(&deep, DEFAULT_MAX_DEPTH), Err(Error::TooDeep));
    }
}
