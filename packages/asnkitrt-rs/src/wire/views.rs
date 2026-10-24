// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Values that borrow what they are decoded from, without copying it.
//!
//! Each is either what a caller gives, to be encoded, or a view of the
//! data it was decoded from, which is read again when it is looked at.
//! The data was checked when it was decoded, so reading it again does
//! not fail.

use core::fmt;
use core::marker::PhantomData;

use super::{ber, per};
use crate::Result;
use crate::per::Known;

/// Bytes, as those of an OCTET STRING, of a string or of an open type.
#[derive(Clone, Copy)]
pub struct Octets<'a>(Repr<'a>);

#[derive(Clone, Copy)]
enum Repr<'a> {
    Slice(&'a [u8]),
    /// The contents of a constructed string of BER: its parts, and the
    /// parts of those, of which the bytes of the primitive ones are the
    /// string. `len` is all of the bytes, and `bits` if the parts are
    /// of a BIT STRING, of which the first byte is the unused bits.
    BerParts {
        data: &'a [u8],
        len: usize,
        bits: bool,
    },
    /// `len` bytes from the bit `at` of `data`, which may not be at the
    /// start of a byte, as UPER has them. With `fragmented`, `at` is
    /// the first length of X.691 11.9.3.8, and the bytes are after it
    /// and those after them.
    Per {
        data: &'a [u8],
        at: usize,
        len: usize,
        fragmented: bool,
    },
}

impl<'a> Octets<'a> {
    pub const fn new(bytes: &'a [u8]) -> Octets<'a> {
        Octets(Repr::Slice(bytes))
    }

    pub(crate) fn ber_parts(data: &'a [u8], len: usize, bits: bool) -> Octets<'a> {
        Octets(Repr::BerParts { data, len, bits })
    }

    pub(crate) fn per(data: &'a [u8], at: usize, len: usize, fragmented: bool) -> Octets<'a> {
        Octets(Repr::Per {
            data,
            at,
            len,
            fragmented,
        })
    }

    pub fn len(&self) -> usize {
        match self.0 {
            Repr::Slice(bytes) => bytes.len(),
            Repr::BerParts { len, .. } | Repr::Per { len, .. } => len,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The bytes, if they are one slice of what was decoded.
    pub fn as_slice(&self) -> Option<&'a [u8]> {
        match self.0 {
            Repr::Slice(bytes) => Some(bytes),
            Repr::Per {
                data,
                at,
                len,
                fragmented: false,
            } if at % 8 == 0 => data.get(at / 8..at / 8 + len),
            _ => None,
        }
    }

    pub fn bytes(&self) -> Bytes<'a> {
        Bytes {
            state: match self.0 {
                Repr::Slice(bytes) => State::Slice(bytes.iter()),
                Repr::BerParts { data, bits, .. } => State::BerParts {
                    data,
                    at: 0,
                    part: &[],
                    bits,
                },
                Repr::Per {
                    data,
                    at,
                    len,
                    fragmented,
                } => {
                    let mut reader = per::Reader::new(data, false);

                    reader.position = at;

                    State::Per {
                        reader,
                        left: if fragmented { 0 } else { len },
                        total: len,
                        fragmented,
                    }
                }
            },
        }
    }

    /// Copies the bytes to the start of `out`, which is to be as long, and
    /// returns how many there are.
    pub fn copy_to(&self, out: &mut [u8]) -> usize {
        for (index, byte) in self.bytes().enumerate() {
            if let Some(slot) = out.get_mut(index) {
                *slot = byte;
            }
        }

        self.len()
    }

    pub fn to_vec(&self) -> alloc::vec::Vec<u8> {
        match self.0 {
            Repr::Slice(bytes) => alloc::vec::Vec::from(bytes),
            // The bits of a part, shifted a byte at a time.
            Repr::Per {
                data,
                at,
                len,
                fragmented: false,
            } => {
                let mut reader = per::Reader::new(data, false);

                reader.position = at;
                reader.bytes(len).unwrap_or_else(|_| self.bytes().collect())
            }
            _ => self.bytes().collect(),
        }
    }
}

impl<'a> From<&'a [u8]> for Octets<'a> {
    fn from(bytes: &'a [u8]) -> Octets<'a> {
        Octets::new(bytes)
    }
}

impl<'a, const N: usize> From<&'a [u8; N]> for Octets<'a> {
    fn from(bytes: &'a [u8; N]) -> Octets<'a> {
        Octets::new(bytes)
    }
}

impl PartialEq for Octets<'_> {
    fn eq(&self, other: &Octets<'_>) -> bool {
        match (self.as_slice(), other.as_slice()) {
            (Some(a), Some(b)) => a == b,
            _ => self.len() == other.len() && self.bytes().eq(other.bytes()),
        }
    }
}

impl Eq for Octets<'_> {}

impl PartialEq<[u8]> for Octets<'_> {
    fn eq(&self, other: &[u8]) -> bool {
        self.len() == other.len() && self.bytes().eq(other.iter().copied())
    }
}

impl fmt::Debug for Octets<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Octets(")?;

        for byte in self.bytes() {
            write!(f, "{byte:02x}")?;
        }

        f.write_str(")")
    }
}

/// The bytes of [`Octets`].
#[derive(Clone)]
pub struct Bytes<'a> {
    state: State<'a>,
}

#[derive(Clone)]
enum State<'a> {
    Slice(core::slice::Iter<'a, u8>),
    BerParts {
        data: &'a [u8],
        at: usize,
        part: &'a [u8],
        bits: bool,
    },
    Per {
        reader: per::Reader<'a>,
        /// The bytes left of the part it is in.
        left: usize,
        /// The bytes left of all.
        total: usize,
        fragmented: bool,
    },
}

impl Iterator for Bytes<'_> {
    type Item = u8;

    fn next(&mut self) -> Option<u8> {
        match &mut self.state {
            State::Slice(iter) => iter.next().copied(),
            State::BerParts {
                data,
                at,
                part,
                bits,
            } => loop {
                if let Some((first, rest)) = part.split_first() {
                    *part = rest;

                    return Some(*first);
                }

                // The next primitive part: the constructed ones are
                // stepped into and their ends stepped over, as they were
                // checked when decoded.
                let next = ber::next_part(data, *at)?;

                *at = next.1;
                *part = next.0;

                if *bits {
                    *part = part.get(1..).unwrap_or_default();
                }
            },
            State::Per {
                reader,
                left,
                total,
                fragmented,
            } => {
                if *total == 0 {
                    return None;
                }

                if *left == 0 && *fragmented {
                    *left = reader.length().ok()?;
                }

                *left = left.checked_sub(1)?;
                *total -= 1;

                // The last byte of the bits of a BIT STRING may be past
                // the end of the data, of which the bits are then zero.
                let count = reader.left().min(8);

                reader
                    .uint(count)
                    .ok()
                    .map(|byte| (byte << (8 - count)) as u8)
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match &self.state {
            State::Slice(iter) => iter.size_hint(),
            State::Per { total, .. } => (*total, Some(*total)),
            State::BerParts { .. } => (0, None),
        }
    }
}

/// An INTEGER of any size: a number that fits in an `i128`, or the bytes
/// of its two's complement, which are borrowed.
#[derive(Clone, Copy)]
pub struct Integer<'a>(IntRepr<'a>);

#[derive(Clone, Copy)]
enum IntRepr<'a> {
    Small(i128),
    /// The two's complement in as few bytes as it can be, or, if
    /// `unsigned`, the magnitude of a number that is not negative, of
    /// which a byte 0 is in front if the first bit is 1.
    Bytes {
        bytes: Octets<'a>,
        unsigned: bool,
    },
}

impl<'a> Integer<'a> {
    pub const fn new(value: i128) -> Integer<'a> {
        Integer(IntRepr::Small(value))
    }

    /// The INTEGER of the two's complement `bytes`, the most significant
    /// first, of which there is at least one.
    pub fn from_bytes(bytes: &'a [u8]) -> Integer<'a> {
        Integer::from_octets(Octets::new(bytes), false)
    }

    /// The same, of bytes that are not in one slice, or of the magnitude
    /// of a number that is not negative if `unsigned`. Those that fit in
    /// an `i128` are one.
    pub(crate) fn from_octets(bytes: Octets<'a>, unsigned: bool) -> Integer<'a> {
        let (skip, negative) = significant(&bytes, unsigned);
        let length = bytes.len() - skip;
        let first = bytes.bytes().nth(skip).unwrap_or(0);
        let fits = length < 16 || (length == 16 && (!unsigned || first & 0x80 == 0));

        if !fits {
            return Integer(IntRepr::Bytes { bytes, unsigned });
        }

        // Those that are skipped are only the sign, which shifts out.
        let mut value: i128 = if negative { -1 } else { 0 };

        for byte in bytes.bytes() {
            value = (value << 8) | i128::from(byte);
        }

        Integer(IntRepr::Small(value))
    }

    /// The number, if it fits in an `i128`.
    pub fn to_i128(&self) -> Option<i128> {
        match self.0 {
            IntRepr::Small(value) => Some(value),
            IntRepr::Bytes { .. } => None,
        }
    }

    /// The number, if it fits in an `i64`.
    pub fn to_i64(&self) -> Option<i64> {
        self.to_i128().and_then(|value| i64::try_from(value).ok())
    }

    /// The two's complement, in as few bytes as it can be, the most
    /// significant first.
    pub fn bytes(&self) -> impl Iterator<Item = u8> + Clone + 'a {
        let (small, count, big, sign) = match self.0 {
            IntRepr::Small(value) => {
                let all = value.to_be_bytes();
                let (skip, _) = significant(&Octets::new(&all), false);
                let mut small = [0u8; 16];

                small[..16 - skip].copy_from_slice(&all[skip..]);

                (small, 16 - skip, None, false)
            }
            IntRepr::Bytes { bytes, unsigned } => {
                let (skip, _) = significant(&bytes, unsigned);
                // A magnitude of which the first bit is 1 has a byte 0
                // before it.
                let sign = unsigned && bytes.bytes().nth(skip).is_some_and(|byte| byte & 0x80 != 0);

                ([0; 16], 0, Some(bytes.bytes().skip(skip)), sign)
            }
        };

        sign.then_some(0u8)
            .into_iter()
            .chain(small.into_iter().take(count))
            .chain(big.into_iter().flatten())
    }

    /// The number of bytes of its two's complement.
    pub fn len(&self) -> usize {
        self.bytes().count()
    }

    pub fn is_empty(&self) -> bool {
        false
    }

    /// If it is less than zero.
    pub fn is_negative(&self) -> bool {
        self.bytes().next().is_some_and(|byte| byte & 0x80 != 0)
    }
}

/// How many of the first bytes are only the sign of those after them,
/// and if the number is negative.
fn significant(bytes: &Octets<'_>, unsigned: bool) -> (usize, bool) {
    let negative = !unsigned && bytes.bytes().next().is_some_and(|byte| byte & 0x80 != 0);
    let fill = if negative { 0xff } else { 0x00 };
    let mut iter = bytes.bytes().peekable();
    let mut skip = 0;

    while let Some(byte) = iter.next() {
        match iter.peek() {
            Some(next) if byte == fill && (unsigned || (next & 0x80 != 0) == negative) => {
                skip += 1;
            }
            _ => break,
        }
    }

    (skip, negative)
}

impl From<i128> for Integer<'_> {
    fn from(value: i128) -> Self {
        Integer::new(value)
    }
}

impl From<i64> for Integer<'_> {
    fn from(value: i64) -> Self {
        Integer::new(value.into())
    }
}

impl From<u64> for Integer<'_> {
    fn from(value: u64) -> Self {
        Integer::new(value.into())
    }
}

impl From<i32> for Integer<'_> {
    fn from(value: i32) -> Self {
        Integer::new(value.into())
    }
}

impl From<u32> for Integer<'_> {
    fn from(value: u32) -> Self {
        Integer::new(value.into())
    }
}

impl PartialEq for Integer<'_> {
    fn eq(&self, other: &Integer<'_>) -> bool {
        match (self.0, other.0) {
            (IntRepr::Small(a), IntRepr::Small(b)) => a == b,
            _ => self.bytes().eq(other.bytes()),
        }
    }
}

impl Eq for Integer<'_> {}

impl PartialEq<i128> for Integer<'_> {
    fn eq(&self, other: &i128) -> bool {
        self.to_i128() == Some(*other)
    }
}

impl fmt::Debug for Integer<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.0 {
            IntRepr::Small(value) => write!(f, "{value}"),
            IntRepr::Bytes { .. } => {
                f.write_str("Integer(0x")?;

                for byte in self.bytes() {
                    write!(f, "{byte:02x}")?;
                }

                f.write_str(")")
            }
        }
    }
}

/// A BIT STRING: `bits` bits, the first the most significant bit of
/// the first byte.
#[derive(Clone, Copy)]
pub struct BitString<'a> {
    bytes: Octets<'a>,
    bits: usize,
}

impl<'a> BitString<'a> {
    /// The first `bits` bits of `bytes`, which has to have them: else it
    /// is as many bits as there are.
    pub const fn new(bytes: &'a [u8], bits: usize) -> BitString<'a> {
        BitString {
            bytes: Octets::new(bytes),
            bits: if bits > 8 * bytes.len() {
                8 * bytes.len()
            } else {
                bits
            },
        }
    }

    pub(crate) fn from_octets(bytes: Octets<'a>, bits: usize) -> BitString<'a> {
        BitString { bytes, bits }
    }

    /// The number of bits.
    pub fn len(&self) -> usize {
        self.bits
    }

    /// The bytes the bits are in, if they are one slice, of which the
    /// bits after the last may not be zero.
    pub(crate) fn as_slice(&self) -> Option<&'a [u8]> {
        self.bytes.as_slice()
    }

    pub fn is_empty(&self) -> bool {
        self.bits == 0
    }

    /// The bytes the bits are in, with the bits after the last one zero.
    pub fn bytes(&self) -> impl Iterator<Item = u8> + Clone + 'a {
        let bits = self.bits;
        let count = bits.div_ceil(8);

        self.bytes
            .bytes()
            .take(count)
            .enumerate()
            .map(move |(index, byte)| {
                if index + 1 == count && !bits.is_multiple_of(8) {
                    byte & !(0xffu8 >> (bits % 8))
                } else {
                    byte
                }
            })
    }

    /// The bit `index`.
    pub fn bit(&self, index: usize) -> bool {
        index < self.bits
            && self
                .bytes
                .bytes()
                .nth(index / 8)
                .is_some_and(|byte| byte & (0x80 >> (index % 8)) != 0)
    }

    /// The number of bits without those that are zero at the end, as
    /// DER and PER leave out of a BIT STRING with named bits.
    pub fn trimmed_len(&self) -> usize {
        let mut last = 0;

        for (index, byte) in self.bytes().enumerate() {
            if byte != 0 {
                last = 8 * index + 8 - byte.trailing_zeros() as usize;
            }
        }

        last
    }
}

impl BitString<'_> {
    /// If the bits are those of `other`, as a value of a type with `named`
    /// bits is, of which the trailing zero bits are not looked at.
    pub fn same_as(&self, other: &BitString<'_>, named: bool) -> bool {
        if !named {
            return self == other;
        }

        let bits = self.trimmed_len();

        bits == other.trimmed_len()
            && self
                .bytes()
                .take(bits.div_ceil(8))
                .eq(other.bytes().take(bits.div_ceil(8)))
    }
}

impl PartialEq for BitString<'_> {
    fn eq(&self, other: &BitString<'_>) -> bool {
        self.bits == other.bits && self.bytes().eq(other.bytes())
    }
}

impl Eq for BitString<'_> {}

impl fmt::Debug for BitString<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("BitString(")?;

        for index in 0..self.bits {
            f.write_str(if self.bit(index) { "1" } else { "0" })?;
        }

        f.write_str(")")
    }
}

/// How the bytes of a string are its characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Width {
    Utf8,
    /// A byte, of Latin-1, for the strings of 8 bits.
    Latin1,
    /// Two bytes, of a BMPString.
    Bmp,
    /// Four bytes, of a UniversalString.
    Universal,
}

/// The text of a character string.
#[derive(Clone, Copy)]
pub struct Text<'a>(TextRepr<'a>);

#[derive(Clone, Copy)]
enum TextRepr<'a> {
    Str(&'a str),
    Bytes(Octets<'a>, Width),
    /// `count` characters of UPER, of `known.bits_per_character` bits
    /// each, from the bit `at` of `data`. With `fragmented`, `at` is the
    /// first length, as `Octets` has it.
    Packed {
        data: &'a [u8],
        at: usize,
        count: usize,
        known: &'a Known,
        fragmented: bool,
    },
}

impl<'a> Text<'a> {
    pub const fn new(text: &'a str) -> Text<'a> {
        Text(TextRepr::Str(text))
    }

    pub(crate) fn from_bytes(bytes: Octets<'a>, width: Width) -> Text<'a> {
        match (width, bytes.as_slice()) {
            // Checked when decoded.
            (Width::Utf8, Some(slice)) => match core::str::from_utf8(slice) {
                Ok(text) => Text(TextRepr::Str(text)),
                Err(_) => Text(TextRepr::Bytes(bytes, width)),
            },
            (Width::Latin1, Some(slice)) if slice.is_ascii() => Text(TextRepr::Str(
                core::str::from_utf8(slice).unwrap_or_default(),
            )),
            _ => Text(TextRepr::Bytes(bytes, width)),
        }
    }

    pub(crate) fn packed(
        data: &'a [u8],
        at: usize,
        count: usize,
        known: &'a Known,
        fragmented: bool,
    ) -> Text<'a> {
        Text(TextRepr::Packed {
            data,
            at,
            count,
            known,
            fragmented,
        })
    }

    /// The text, if it is one `str` of what was decoded or given.
    pub fn as_str(&self) -> Option<&'a str> {
        match self.0 {
            TextRepr::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn chars(&self) -> Chars<'a> {
        Chars {
            state: match self.0 {
                TextRepr::Str(text) => CharState::Str(text.chars()),
                TextRepr::Bytes(bytes, width) => CharState::Bytes(bytes.bytes(), width),
                TextRepr::Packed {
                    data,
                    at,
                    count,
                    known,
                    fragmented,
                } => {
                    let mut reader = per::Reader::new(data, false);

                    reader.position = at;

                    CharState::Packed {
                        reader,
                        left: if fragmented { 0 } else { count },
                        total: count,
                        known,
                        fragmented,
                    }
                }
            },
        }
    }

    /// The number of characters.
    pub fn char_count(&self) -> usize {
        match self.0 {
            TextRepr::Str(text) => text.chars().count(),
            TextRepr::Bytes(bytes, Width::Latin1) => bytes.len(),
            TextRepr::Bytes(bytes, Width::Bmp) => bytes.len() / 2,
            TextRepr::Bytes(bytes, Width::Universal) => bytes.len() / 4,
            TextRepr::Bytes(..) => self.chars().count(),
            TextRepr::Packed { count, .. } => count,
        }
    }

    /// The number of bytes of the text in UTF-8.
    pub fn utf8_len(&self) -> usize {
        match self.0 {
            TextRepr::Str(text) => text.len(),
            TextRepr::Bytes(bytes, Width::Utf8) => bytes.len(),
            _ => self.chars().map(char::len_utf8).sum(),
        }
    }
}

impl<'a> From<&'a str> for Text<'a> {
    fn from(text: &'a str) -> Text<'a> {
        Text::new(text)
    }
}

impl PartialEq for Text<'_> {
    fn eq(&self, other: &Text<'_>) -> bool {
        match (self.as_str(), other.as_str()) {
            (Some(a), Some(b)) => a == b,
            _ => self.chars().eq(other.chars()),
        }
    }
}

impl Eq for Text<'_> {}

impl PartialEq<str> for Text<'_> {
    fn eq(&self, other: &str) -> bool {
        self.chars().eq(other.chars())
    }
}

impl PartialEq<&str> for Text<'_> {
    fn eq(&self, other: &&str) -> bool {
        self.chars().eq(other.chars())
    }
}

impl fmt::Display for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(text) = self.as_str() {
            return f.write_str(text);
        }

        for character in self.chars() {
            fmt::Write::write_char(f, character)?;
        }

        Ok(())
    }
}

impl fmt::Debug for Text<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("\"")?;

        for character in self.chars() {
            for escaped in character.escape_debug() {
                fmt::Write::write_char(f, escaped)?;
            }
        }

        f.write_str("\"")
    }
}

/// The characters of a [`Text`].
#[derive(Clone)]
pub struct Chars<'a> {
    state: CharState<'a>,
}

#[derive(Clone)]
enum CharState<'a> {
    Str(core::str::Chars<'a>),
    Bytes(Bytes<'a>, Width),
    Packed {
        reader: per::Reader<'a>,
        left: usize,
        total: usize,
        known: &'a Known,
        fragmented: bool,
    },
}

impl Iterator for Chars<'_> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        match &mut self.state {
            CharState::Str(chars) => chars.next(),
            CharState::Bytes(bytes, width) => {
                let code = match width {
                    Width::Latin1 => u32::from(bytes.next()?),
                    Width::Bmp => u32::from(u16::from_be_bytes([bytes.next()?, bytes.next()?])),
                    Width::Universal => u32::from_be_bytes([
                        bytes.next()?,
                        bytes.next()?,
                        bytes.next()?,
                        bytes.next()?,
                    ]),
                    Width::Utf8 => {
                        let first = bytes.next()?;
                        let (mut code, more) = match first {
                            0x00..=0x7f => (u32::from(first), 0),
                            0xc0..=0xdf => (u32::from(first & 0x1f), 1),
                            0xe0..=0xef => (u32::from(first & 0x0f), 2),
                            _ => (u32::from(first & 0x07), 3),
                        };

                        for _ in 0..more {
                            code = (code << 6) | u32::from(bytes.next()? & 0x3f);
                        }

                        code
                    }
                };

                char::from_u32(code)
            }
            CharState::Packed {
                reader,
                left,
                total,
                known,
                fragmented,
            } => {
                if *total == 0 {
                    return None;
                }

                if *left == 0 && *fragmented {
                    *left = reader.length().ok()?;
                }

                *left = left.checked_sub(1)?;
                *total -= 1;

                let value = reader.uint(known.bits_per_character).ok()? as u32;
                let code = known.alphabet.decode(value, 0).ok()?;

                char::from_u32(code)
            }
        }
    }
}

/// An OBJECT IDENTIFIER, as its contents octets (X.690 8.19).
#[derive(Clone, Copy)]
pub struct Oid<'a> {
    bytes: Octets<'a>,
    /// If each subidentifier is in as few bytes as it can be, as DER has
    /// them. BER may have them in more, which are written in fewer.
    minimal: bool,
}

impl<'a> Oid<'a> {
    /// The OBJECT IDENTIFIER of these contents octets, if they are those
    /// of one.
    pub fn new(bytes: &'a [u8]) -> Result<Oid<'a>> {
        Oid::checked(Octets::new(bytes), true, 0)
    }

    /// Contents octets that are known to be those of an OBJECT
    /// IDENTIFIER, as those the plugin writes.
    pub const fn from_static(bytes: &'a [u8]) -> Oid<'a> {
        Oid {
            bytes: Octets::new(bytes),
            minimal: true,
        }
    }

    /// Checks the contents octets, which with `strict` are to have each
    /// subidentifier in as few bytes as it can be.
    pub(crate) fn checked(bytes: Octets<'a>, strict: bool, offset: usize) -> Result<Oid<'a>> {
        let check = |minimal| {
            crate::value::check_object_identifier_bytes(bytes.bytes(), bytes.len(), minimal)
        };

        match check(true) {
            Ok(()) => Ok(Oid {
                bytes,
                minimal: true,
            }),
            Err(_) if !strict && check(false).is_ok() => Ok(Oid {
                bytes,
                minimal: false,
            }),
            Err((what, index)) => Err(crate::Error::decode(what, offset + index)),
        }
    }

    /// The contents octets as they were decoded or given.
    pub fn octets(&self) -> Octets<'a> {
        self.bytes
    }

    /// The subidentifiers, of which the first is of the first two arcs.
    pub fn subidentifiers(&self) -> Subidentifiers<'a> {
        Subidentifiers {
            bytes: self.bytes.bytes(),
        }
    }

    /// The arcs, of which there are at least two.
    pub fn arcs(&self) -> impl Iterator<Item = u128> + 'a {
        let mut subidentifiers = self.subidentifiers();
        let mut second = None;
        let mut first = true;

        core::iter::from_fn(move || {
            if let Some(second) = second.take() {
                return Some(second);
            }

            let subidentifier = subidentifiers.next()?;

            if !first {
                return Some(subidentifier);
            }

            first = false;

            let (one, two) = match subidentifier {
                0..40 => (0, subidentifier),
                40..80 => (1, subidentifier - 40),
                _ => (2, subidentifier - 80),
            };

            second = Some(two);

            Some(one)
        })
    }

    pub(crate) fn is_minimal(&self) -> bool {
        self.minimal
    }
}

impl PartialEq for Oid<'_> {
    fn eq(&self, other: &Oid<'_>) -> bool {
        if self.minimal && other.minimal {
            self.bytes == other.bytes
        } else {
            self.subidentifiers().eq(other.subidentifiers())
        }
    }
}

impl Eq for Oid<'_> {}

impl fmt::Display for Oid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, arc) in self.arcs().enumerate() {
            if index > 0 {
                f.write_str(".")?;
            }

            write!(f, "{arc}")?;
        }

        Ok(())
    }
}

impl fmt::Debug for Oid<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Oid({self})")
    }
}

/// The subidentifiers of an [`Oid`].
#[derive(Clone)]
pub struct Subidentifiers<'a> {
    bytes: Bytes<'a>,
}

impl Iterator for Subidentifiers<'_> {
    type Item = u128;

    fn next(&mut self) -> Option<u128> {
        let mut value: u128 = 0;

        for byte in self.bytes.by_ref() {
            value = (value << 7) | u128::from(byte & 0x7f);

            if byte & 0x80 == 0 {
                return Some(value);
            }
        }

        None
    }
}

/// The elements of a SEQUENCE OF or a SET OF: a slice that is given, or
/// those of the data they were decoded from, which are decoded again
/// as they are iterated.
pub struct SequenceOf<'a, T>(SeqRepr<'a, T>);

enum SeqRepr<'a, T> {
    Slice(&'a [T]),
    Ber {
        data: &'a [u8],
        at: usize,
        count: usize,
        der: bool,
        read: fn(&mut ber::Reader<'a>) -> Result<T>,
    },
    /// As `Octets` has it: `at` is the first element, or the first
    /// length if it is `fragmented`.
    Per {
        data: &'a [u8],
        at: usize,
        count: usize,
        fragmented: bool,
        read: fn(&mut per::Reader<'a>) -> Result<T>,
    },
}

impl<T> Clone for SeqRepr<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SeqRepr<'_, T> {}

impl<T> Clone for SequenceOf<'_, T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> Copy for SequenceOf<'_, T> {}

impl<'a, T> SequenceOf<'a, T> {
    pub const fn new(items: &'a [T]) -> SequenceOf<'a, T> {
        SequenceOf(SeqRepr::Slice(items))
    }

    #[doc(hidden)]
    pub fn ber(
        data: &'a [u8],
        at: usize,
        count: usize,
        der: bool,
        read: fn(&mut ber::Reader<'a>) -> Result<T>,
    ) -> SequenceOf<'a, T> {
        SequenceOf(SeqRepr::Ber {
            data,
            at,
            count,
            der,
            read,
        })
    }

    #[doc(hidden)]
    pub fn per(
        data: &'a [u8],
        at: usize,
        count: usize,
        fragmented: bool,
        read: fn(&mut per::Reader<'a>) -> Result<T>,
    ) -> SequenceOf<'a, T> {
        SequenceOf(SeqRepr::Per {
            data,
            at,
            count,
            fragmented,
            read,
        })
    }

    pub fn len(&self) -> usize {
        match self.0 {
            SeqRepr::Slice(items) => items.len(),
            SeqRepr::Ber { count, .. } | SeqRepr::Per { count, .. } => count,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The elements, if they were given as a slice.
    pub fn as_slice(&self) -> Option<&'a [T]> {
        match self.0 {
            SeqRepr::Slice(items) => Some(items),
            _ => None,
        }
    }

    /// The elements: cloned from the slice, or decoded again.
    pub fn iter(&self) -> Iter<'a, T>
    where
        T: Clone,
    {
        Iter {
            state: match self.0 {
                SeqRepr::Slice(items) => IterState::Slice(items.iter()),
                SeqRepr::Ber {
                    data,
                    at,
                    count,
                    der,
                    read,
                } => {
                    let mut reader = ber::Reader::new(data, der);

                    reader.position = at;

                    IterState::Ber {
                        reader,
                        left: count,
                        read,
                    }
                }
                SeqRepr::Per {
                    data,
                    at,
                    count,
                    fragmented,
                    read,
                } => {
                    let mut reader = per::Reader::new(data, false);

                    reader.position = at;

                    IterState::Per {
                        reader,
                        left: if fragmented { 0 } else { count },
                        total: count,
                        fragmented,
                        read,
                    }
                }
            },
            phantom: PhantomData,
        }
    }
}

impl<'a, T> SequenceOf<'a, T> {
    /// Calls `f` with each element, which is decoded again if it was
    /// decoded, until it fails.
    pub fn try_for_each(&self, mut f: impl FnMut(&T) -> Result<()>) -> Result<()> {
        match self.0 {
            SeqRepr::Slice(items) => items.iter().try_for_each(f),
            SeqRepr::Ber {
                data,
                at,
                count,
                der,
                read,
            } => {
                let mut reader = ber::Reader::new(data, der);

                reader.position = at;

                for _ in 0..count {
                    f(&read(&mut reader)?)?;
                }

                Ok(())
            }
            SeqRepr::Per {
                data,
                at,
                count,
                fragmented,
                read,
            } => {
                let mut reader = per::Reader::new(data, false);
                let mut left = if fragmented { 0 } else { count };

                reader.position = at;

                for _ in 0..count {
                    if left == 0 {
                        left = reader.length()?;
                    }

                    left -= 1;
                    f(&read(&mut reader)?)?;
                }

                Ok(())
            }
        }
    }

    /// The elements in BER, after which DER sorts those of a SET OF.
    pub fn write_ber(
        &self,
        w: &mut ber::Writer<'_>,
        der: bool,
        set_of: bool,
        write: impl Fn(&T, &mut ber::Writer<'_>, bool) -> Result<()>,
    ) -> Result<()> {
        let start = w.len();

        self.try_for_each(|item| write(item, w, der))?;

        if der && set_of {
            w.sort_elements(start);
        }

        Ok(())
    }

    /// The elements of the contents `region`, which DER has in order if
    /// they are of a SET OF.
    pub fn read_ber(
        r: &mut ber::Reader<'a>,
        region: &ber::Region,
        set_of: bool,
        read: fn(&mut ber::Reader<'a>) -> Result<T>,
    ) -> Result<SequenceOf<'a, T>> {
        let at = r.position();
        let mut count = 0;
        let mut previous = None;

        while r.more(region)? {
            let start = r.position();

            read(r)?;

            if set_of && r.der() {
                r.check_element_order(&mut previous, start)?;
            }

            count += 1;
        }

        Ok(SequenceOf::ber(r.data(), at, count, r.der(), read))
    }

    /// The elements in UPER, after their number for the size `size`.
    pub fn write_uper(
        &self,
        w: &mut per::Writer<'_>,
        size: Option<crate::types::Range>,
        write: impl Fn(&T, &mut per::Writer<'_>) -> Result<()>,
    ) -> Result<()> {
        if per::count(w, self.len(), size)? {
            return self.try_for_each(|item| write(item, w));
        }

        let mut fragments = per::Fragmenter::new(self.len());
        let mut left = 0;

        self.try_for_each(|item| {
            if left == 0 {
                left = fragments.next(w).map_or(0, |(_, count)| count);
            }

            left = left.saturating_sub(1);
            write(item, w)
        })?;

        // The empty fragment after those of 16384 elements.
        while fragments.next(w).is_some() {}

        Ok(())
    }

    pub fn read_uper(
        r: &mut per::Reader<'a>,
        size: Option<crate::types::Range>,
        read: fn(&mut per::Reader<'a>) -> Result<T>,
    ) -> Result<SequenceOf<'a, T>> {
        let (count, fragmented) = per::read_count(r, size)?;
        let at = r.position();

        if !fragmented {
            for _ in 0..count {
                read(r)?;
            }

            return Ok(SequenceOf::per(r.data(), at, count, false, read));
        }

        let mut total = 0;

        loop {
            let count = r.length()?;

            r.spend(count)?;

            for _ in 0..count {
                read(r)?;
            }

            total += count;

            if count < 16384 {
                break;
            }
        }

        Ok(SequenceOf::per(r.data(), at, total, true, read))
    }
}

impl<'a, T> From<&'a [T]> for SequenceOf<'a, T> {
    fn from(items: &'a [T]) -> SequenceOf<'a, T> {
        SequenceOf::new(items)
    }
}

impl<'a, T, const N: usize> From<&'a [T; N]> for SequenceOf<'a, T> {
    fn from(items: &'a [T; N]) -> SequenceOf<'a, T> {
        SequenceOf::new(items)
    }
}

impl<'a, T: Clone> IntoIterator for &SequenceOf<'a, T> {
    type Item = T;
    type IntoIter = Iter<'a, T>;

    fn into_iter(self) -> Iter<'a, T> {
        self.iter()
    }
}

impl<T: Clone + PartialEq> PartialEq for SequenceOf<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        self.len() == other.len() && self.iter().eq(other.iter())
    }
}

impl<T: Clone + fmt::Debug> fmt::Debug for SequenceOf<'_, T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_list().entries(self.iter()).finish()
    }
}

/// The elements of a [`SequenceOf`].
pub struct Iter<'a, T> {
    state: IterState<'a, T>,
    phantom: PhantomData<T>,
}

enum IterState<'a, T> {
    Slice(core::slice::Iter<'a, T>),
    Ber {
        reader: ber::Reader<'a>,
        left: usize,
        read: fn(&mut ber::Reader<'a>) -> Result<T>,
    },
    Per {
        reader: per::Reader<'a>,
        left: usize,
        total: usize,
        fragmented: bool,
        read: fn(&mut per::Reader<'a>) -> Result<T>,
    },
}

impl<T: Clone> Iterator for Iter<'_, T> {
    type Item = T;

    fn next(&mut self) -> Option<T> {
        match &mut self.state {
            IterState::Slice(items) => items.next().cloned(),
            IterState::Ber { reader, left, read } => {
                *left = left.checked_sub(1)?;

                read(reader).ok()
            }
            IterState::Per {
                reader,
                left,
                total,
                fragmented,
                read,
            } => {
                if *total == 0 {
                    return None;
                }

                if *left == 0 && *fragmented {
                    *left = reader.length().ok()?;
                }

                *left = left.checked_sub(1)?;
                *total -= 1;

                read(reader).ok()
            }
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let left = match &self.state {
            IterState::Slice(items) => items.len(),
            IterState::Ber { left, .. } => *left,
            IterState::Per { total, .. } => *total,
        };

        (left, Some(left))
    }
}
