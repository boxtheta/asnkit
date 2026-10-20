// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Packed Encoding Rules, aligned (PER) and unaligned (UPER), X.691.
//!
//! One implementation does both: aligned only differs by where it pads
//! to a byte, and by a few rules of its own for the size of a field.
//! It does what the runtime for Python does, also where that is not all
//! of the standard.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;
use core::borrow::Borrow;

use crate::ber::{decode_real, encode_real};
use crate::time;
use crate::types::{
    Addition, Codec, Kind, MemberInfo, Members, Range, StrKind, TypeId, Types, clean_bits,
    is_default,
};
use crate::value::{
    Date, DateTime, Lookup, ObjectIdentifier, Time, Value, integer_to_twos_complement,
};
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

/// The number of bits it takes to write `size`.
fn bits_of(size: u128) -> usize {
    (128 - size.leading_zeros()) as usize
}

/// The same, as a power of two.
fn bits_of_power_of_two(size: u128) -> usize {
    if size == 0 {
        return 0;
    }

    let mut power = 1;

    while bits_of(size) > power {
        power <<= 1;
    }

    power
}

fn bytes_of(size: u128) -> usize {
    if size == 0 {
        1
    } else {
        bits_of(size).div_ceil(8)
    }
}

/// If a size is in the range of a size. A bound that is missing is no
/// bound.
fn in_size(range: Option<Range>, size: i128) -> bool {
    range.is_none_or(|range| {
        range.min.is_none_or(|min| min <= size) && range.max.is_none_or(|max| size <= max)
    })
}

/// The additions of a CHOICE in the canonical order of their tags.
fn choice_additions(members: &Members) -> Vec<&MemberInfo> {
    let additions: Vec<&MemberInfo> = members.all().skip(members.root.len()).collect();

    members
        .addition_order
        .iter()
        .map(|index| additions[*index])
        .collect()
}

/// If an integer is in the range of its bounds. A bound that is missing
/// is no bound.
fn in_range(number: &Value, lower: Option<i128>, upper: Option<i128>) -> bool {
    match number {
        Value::Integer(value) => {
            lower.is_none_or(|lower| *value >= lower) && upper.is_none_or(|upper| *value <= upper)
        }
        // Larger than any bound, which is no more than 128 bits.
        Value::BigInteger(bytes) => {
            let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);

            if negative {
                lower.is_none()
            } else {
                upper.is_none()
            }
        }
        _ => false,
    }
}

/// The two's complement of `bytes` and of `value`, of `length` bytes.
fn widened(bytes: &[u8], length: usize) -> Vec<u8> {
    let fill = if bytes.first().is_some_and(|byte| byte & 0x80 != 0) {
        0xff
    } else {
        0
    };
    let mut wide = alloc::vec![fill; length - bytes.len()];

    wide.extend_from_slice(bytes);
    wide
}

/// The integer of the two's complement `bytes` less `min`, which is not
/// negative, as few bytes as it can be with no sign.
fn less(bytes: &[u8], min: i128) -> Vec<u8> {
    let length = bytes.len().max(16) + 1;
    let a = widened(bytes, length);
    let b = widened(&min.to_be_bytes(), length);
    let mut difference = alloc::vec![0u8; length];
    let mut borrow = 0i16;

    for index in (0..length).rev() {
        let mut digit = i16::from(a[index]) - i16::from(b[index]) - borrow;

        borrow = i16::from(digit < 0);

        if digit < 0 {
            digit += 256;
        }

        difference[index] = digit as u8;
    }

    let first = difference
        .iter()
        .position(|byte| *byte != 0)
        .unwrap_or(length - 1);

    difference.split_off(first)
}

/// The integer of the unsigned `bytes` plus `min`.
fn plus(bytes: &[u8], min: i128) -> Value<'static> {
    let mut unsigned = alloc::vec![0u8];

    unsigned.extend_from_slice(bytes);

    let length = unsigned.len().max(16) + 1;
    let a = widened(&unsigned, length);
    let b = widened(&min.to_be_bytes(), length);
    let mut sum = alloc::vec![0u8; length];
    let mut carry = 0u16;

    for index in (0..length).rev() {
        let digit = u16::from(a[index]) + u16::from(b[index]) + carry;

        carry = digit >> 8;
        sum[index] = digit as u8;
    }

    Value::from_twos_complement(&sum)
}

/// The bounds of a size, of which a missing lower bound is 0 (X.691 16.4,
/// 17.2, 20.2, 30.3). `None` if there is no upper bound.
fn size_bounds(size: Option<Range>) -> Option<(i128, i128)> {
    let size = size?;

    Some((size.min.unwrap_or(0), size.max?))
}

/// The size a value is encoded with: the one of its type or, if the size
/// is extensible and the value is not in its root, none (X.691 16.6,
/// 17.3, 20.4, 30.4). Writes the bit that tells which, if there is one.
fn extended(size: Option<Range>, length: i128, w: &mut Writer) -> Option<Range> {
    if !size.is_some_and(|size| size.extensible) {
        return size;
    }

    let inside = in_size(size, length);

    w.bit(!inside);

    if inside { size } else { None }
}

/// Rejects a value that is not of the size of its type, which can not be
/// encoded.
fn check_size(size: Option<Range>, length: i128) -> Result<()> {
    if in_size(size, length) {
        Ok(())
    } else {
        encode_error("The size is not in the range.")
    }
}

/// The same, when decoding.
fn read_extended(size: Option<Range>, r: &mut Reader<'_>) -> Result<Option<Range>> {
    if size.is_some_and(|size| size.extensible) && r.bit()? {
        return Ok(None);
    }

    Ok(size)
}

fn is_unbound(range: Option<Range>) -> bool {
    match size_bounds(range) {
        None => true,
        Some((_, max)) => max > 65535,
    }
}

// The bits.

pub(crate) struct Writer {
    pub(crate) bytes: Vec<u8>,
    pub(crate) bits: usize,
    aligned: bool,
}

impl Writer {
    pub(crate) fn new(aligned: bool) -> Writer {
        Writer {
            bytes: Vec::new(),
            bits: 0,
            aligned,
        }
    }

    pub(crate) fn bit(&mut self, bit: bool) {
        if self.bits.is_multiple_of(8) {
            self.bytes.push(0);
        }

        if bit {
            *self.bytes.last_mut().unwrap() |= 0x80 >> (self.bits % 8);
        }

        self.bits += 1;
    }

    /// The `count` low bits of `chunk`, at most 8, the most significant
    /// first.
    fn chunk(&mut self, chunk: u8, count: usize) {
        let chunk = chunk & (0xff >> (8 - count));
        let used = self.bits % 8;

        if used == 0 {
            self.bytes.push(0);
        }

        let free = 8 - used;
        let last = self.bytes.len() - 1;

        if count <= free {
            self.bytes[last] |= chunk << (free - count);
        } else {
            self.bytes[last] |= chunk >> (count - free);
            self.bytes.push(chunk << (8 - (count - free)));
        }

        self.bits += count;
    }

    /// `count` zero bits.
    fn zeros(&mut self, count: usize) {
        self.bits += count;
        // The bits after the last are zero.
        self.bytes.resize(self.bits.div_ceil(8), 0);
    }

    /// The `count` low bits of `value`, the most significant first.
    pub(crate) fn uint(&mut self, value: u128, mut count: usize) {
        if count > 128 {
            self.zeros(count - 128);
            count = 128;
        }

        // Up to the end of the last byte, then a byte at a time.
        while count > 0 {
            let take = count.min(8 - self.bits % 8);

            count -= take;
            self.chunk((value >> count) as u8, take);
        }
    }

    /// The first `count` bits of `data`.
    pub(crate) fn bits_of(&mut self, data: &[u8], count: usize) {
        let whole = &data[..count / 8];
        let rest = count % 8;

        if self.bits.is_multiple_of(8) {
            self.bytes.extend_from_slice(whole);
            self.bits += 8 * whole.len();
        } else {
            self.bytes.reserve(whole.len() + 1);

            for byte in whole {
                self.chunk(*byte, 8);
            }
        }

        if rest > 0 {
            self.chunk(data[count / 8] >> (8 - rest), rest);
        }
    }

    pub(crate) fn bytes(&mut self, data: &[u8]) {
        if self.bits.is_multiple_of(8) {
            self.bytes.extend_from_slice(data);
            self.bits += 8 * data.len();
        } else {
            self.bits_of(data, 8 * data.len());
        }
    }

    pub(crate) fn align_always(&mut self) {
        self.zeros((8 - self.bits % 8) % 8);
    }

    fn align(&mut self) {
        if self.aligned {
            self.align_always();
        }
    }

    pub(crate) fn number_of_bytes(&self) -> usize {
        self.bits.div_ceil(8)
    }

    pub(crate) fn set_bit(&mut self, index: usize) {
        self.bytes[index / 8] |= 0x80 >> (index % 8);
    }

    pub(crate) fn append(&mut self, other: &Writer) {
        self.bits_of(&other.bytes, other.bits);
    }

    pub(crate) fn are_all_bits_zero(&self) -> bool {
        self.bytes.iter().all(|byte| *byte == 0)
    }

    pub(crate) fn sub(&self) -> Writer {
        Writer::new(self.aligned)
    }

    /// A length. Fragments are the caller's, see `Fragmenter`.
    fn length(&mut self, length: usize) -> usize {
        if length < 128 {
            self.uint(length as u128, 8);

            length
        } else if length < 16384 {
            self.uint(0x8000 | length as u128, 16);

            length
        } else {
            let (code, fragment) = match length {
                16384..=32767 => (0xc1, 16384),
                32768..=49151 => (0xc2, 32768),
                49152..=65535 => (0xc3, 49152),
                _ => (0xc4, 65536),
            };

            self.uint(code, 8);

            fragment
        }
    }

    fn constrained_whole_number(&mut self, value: i128, min: i128, max: i128, count: usize) {
        let range = (max as u128).wrapping_sub(min as u128).wrapping_add(1);
        let value = (value as u128).wrapping_sub(min as u128);

        if range != 0 && range <= 255 {
            self.uint(value, count);
        } else if range == 256 {
            self.align_always();
            self.uint(value, 8);
        } else if range != 0 && range <= 65536 {
            self.align_always();
            self.uint(value, 16);
        } else {
            self.align_always();
            self.uint(value, count);
        }
    }

    fn unconstrained_whole_number(&mut self, bytes: &[u8]) -> Result<()> {
        if bytes.len() >= 16384 {
            return encode_error("An INTEGER is too large.");
        }

        self.length(bytes.len());
        self.bytes(bytes);

        Ok(())
    }

    fn normally_small_whole_number(&mut self, value: u128) {
        if value < 64 {
            self.uint(value, 7);
        } else {
            self.bit(true);

            let length = bits_of(value).div_ceil(8);

            self.length(length);
            self.uint(value, 8 * length);
        }
    }

    fn normally_small_length(&mut self, value: usize) -> Result<()> {
        if value <= 64 {
            self.uint(value as u128 - 1, 7);
        } else if value <= 127 {
            self.uint(0x100 | value as u128, 9);
        } else {
            return encode_error("Normally small length number >127 is not yet supported.");
        }

        Ok(())
    }
}

/// The parts a length is written in: the length of each is written
/// as it is asked for, so that the caller writes the part after it.
struct Fragmenter {
    length: usize,
    offset: usize,
    chunk: usize,
    done: bool,
}

impl Fragmenter {
    pub(crate) fn new(length: usize) -> Fragmenter {
        Fragmenter {
            length,
            offset: 0,
            chunk: length,
            done: false,
        }
    }

    /// The offset and the size of the next part, after writing its length.
    fn next(&mut self, w: &mut Writer) -> Option<(usize, usize)> {
        if self.done {
            return None;
        }

        let chunk = w.length(self.chunk);
        let part = (self.offset, chunk);

        if chunk < 16384 {
            self.done = true;
        } else {
            self.offset += chunk;
            self.chunk = self.length - self.offset;
        }

        Some(part)
    }
}

pub(crate) struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) position: usize,
    pub(crate) aligned: bool,
    /// How many more items and characters may be decoded. Some take no
    /// bits, so the data alone does not keep a length in check.
    pub(crate) budget: usize,
}

fn out_of_data<T>(position: usize) -> Result<T> {
    Err(Error::decode("out of data", position))
}

fn decode_error<T>(what: &'static str, position: usize) -> Result<T> {
    Err(Error::decode(what, position))
}

impl<'a> Reader<'a> {
    pub(crate) fn spend(&mut self, count: usize) -> Result<()> {
        match self.budget.checked_sub(count) {
            Some(budget) => {
                self.budget = budget;

                Ok(())
            }
            None => decode_error("Too many items for the size of the data.", self.position),
        }
    }

    pub(crate) fn left(&self) -> usize {
        8 * self.data.len() - self.position
    }

    pub(crate) fn bit(&mut self) -> Result<bool> {
        if self.left() == 0 {
            return out_of_data(self.position);
        }

        let bit = self.data[self.position / 8] & (0x80 >> (self.position % 8)) != 0;

        self.position += 1;

        Ok(bit)
    }

    pub(crate) fn uint(&mut self, count: usize) -> Result<u128> {
        if count > self.left() {
            return out_of_data(self.position);
        }

        if count > 128 {
            return decode_error("A number is too large.", self.position);
        }

        let mut value = 0u128;
        let mut count = count;

        // Up to the end of a byte, then a byte at a time.
        while count > 0 {
            let used = self.position % 8;
            let take = count.min(8 - used);
            let byte = self.data[self.position / 8] << used;

            value = (value << take) | u128::from(byte >> (8 - take));
            self.position += take;
            count -= take;
        }

        Ok(value)
    }

    /// `count` bits, as bytes with the bits first.
    pub(crate) fn bits(&mut self, count: usize) -> Result<Vec<u8>> {
        if count > self.left() {
            return out_of_data(self.position);
        }

        let start = self.position / 8;
        let shift = self.position % 8;
        let length = count.div_ceil(8);
        let mut bytes = if shift == 0 {
            Vec::from(&self.data[start..start + length])
        } else {
            // A byte is the end of one and the start of the next, which
            // is there unless the bits end before it.
            (0..length)
                .map(|index| {
                    let next = self.data.get(start + index + 1).copied().unwrap_or(0);

                    self.data[start + index] << shift | next >> (8 - shift)
                })
                .collect()
        };

        // The bits after the last are zero.
        if !count.is_multiple_of(8)
            && let Some(last) = bytes.last_mut()
        {
            *last &= 0xff << (8 - count % 8);
        }

        self.position += count;

        Ok(bytes)
    }

    pub(crate) fn bytes(&mut self, count: usize) -> Result<Vec<u8>> {
        if count > self.left() / 8 {
            return out_of_data(self.position);
        }

        if self.position.is_multiple_of(8) {
            let start = self.position / 8;

            self.position += 8 * count;

            return Ok(Vec::from(&self.data[start..start + count]));
        }

        self.bits(8 * count)
    }

    /// `count` bits, borrowed from the data when they are whole bytes of
    /// it.
    pub(crate) fn bit_octets(&mut self, count: usize) -> Result<Cow<'a, [u8]>> {
        if count.is_multiple_of(8) {
            self.octets(count / 8)
        } else {
            self.bits(count).map(Cow::Owned)
        }
    }

    /// The same, borrowed from the data when they are whole bytes of it.
    pub(crate) fn octets(&mut self, count: usize) -> Result<Cow<'a, [u8]>> {
        if self.position.is_multiple_of(8) && count <= self.left() / 8 {
            let start = self.position / 8;

            self.position += 8 * count;

            return Ok(Cow::Borrowed(&self.data[start..start + count]));
        }

        self.bytes(count).map(Cow::Owned)
    }

    /// Skips the `count` bits that tell which members are there, which
    /// `bit_at` reads from where it returns. It runs out of data where
    /// reading them one at a time does.
    pub(crate) fn presence(&mut self, count: usize) -> Result<usize> {
        if count > self.left() {
            return out_of_data(8 * self.data.len());
        }

        let start = self.position;

        self.position += count;

        Ok(start)
    }

    /// The bit at `position`, which is one that was skipped.
    pub(crate) fn bit_at(&self, position: usize) -> bool {
        self.data[position / 8] & (0x80 >> (position % 8)) != 0
    }

    pub(crate) fn skip(&mut self, count: usize) -> Result<()> {
        if count > self.left() {
            return out_of_data(self.position);
        }

        self.position += count;

        Ok(())
    }

    pub(crate) fn align_always(&mut self) {
        self.position += (8 - self.position % 8) % 8;
    }

    fn align(&mut self) {
        if self.aligned {
            self.align_always();
        }
    }

    fn length(&mut self) -> Result<usize> {
        let value = self.uint(8)? as usize;

        if value & 0x80 == 0 {
            Ok(value)
        } else if value & 0xc0 == 0x80 {
            Ok(((value & 0x7f) << 8) | self.uint(8)? as usize)
        } else {
            match value {
                0xc1 => Ok(16384),
                0xc2 => Ok(32768),
                0xc3 => Ok(49152),
                0xc4 => Ok(65536),
                _ => decode_error("Bad length determinant fragmentation value.", self.position),
            }
        }
    }

    /// The lengths of the parts, one at a time: the caller reads each.
    fn constrained_whole_number(&mut self, min: i128, max: i128, count: usize) -> Result<i128> {
        let range = (max as u128).wrapping_sub(min as u128).wrapping_add(1);
        let value = if range != 0 && range <= 255 {
            self.uint(count)?
        } else if range == 256 {
            self.align_always();
            self.uint(8)?
        } else if range != 0 && range <= 65536 {
            self.align_always();
            self.uint(16)?
        } else {
            self.align_always();
            self.uint(count)?
        };

        Ok((value as i128).wrapping_add(min))
    }

    fn unconstrained_whole_number(&mut self) -> Result<Value<'static>> {
        let length = self.length()?;
        let bytes = self.bytes(length)?;

        Ok(Value::from_twos_complement(&bytes))
    }

    fn normally_small_whole_number(&mut self) -> Result<u128> {
        if !self.bit()? {
            self.uint(6)
        } else {
            let length = self.length()?;

            self.uint(8 * length)
        }
    }

    fn normally_small_length(&mut self) -> Result<usize> {
        if !self.bit()? {
            Ok(self.uint(6)? as usize + 1)
        } else if !self.bit()? {
            Ok(self.uint(7)? as usize)
        } else {
            decode_error(
                "Normally small length number >64 is not yet supported.",
                self.position,
            )
        }
    }
}

/// The characters a string of a known multiplier can have: those of its
/// kind, or those of its permitted alphabet.
enum Characters {
    /// " 0123456789".
    Numeric,
    Printable,
    Ia5,
    Visible,
    Bmp,
    Universal,
    /// Sorted, with no character twice.
    Custom(Vec<u32>),
}

impl Characters {
    fn count(&self) -> u64 {
        match self {
            Characters::Numeric => 11,
            Characters::Printable => 74,
            Characters::Ia5 => 128,
            Characters::Visible => 95,
            Characters::Bmp => 63488,
            Characters::Universal => 1 << 32,
            Characters::Custom(codes) => codes.len() as u64,
        }
    }

    /// The greatest code of a character.
    fn greatest(&self) -> u32 {
        match self {
            Characters::Numeric => 57,
            Characters::Printable => 122,
            Characters::Ia5 => 127,
            Characters::Visible => 126,
            Characters::Bmp => 0xffff,
            Characters::Universal => u32::MAX,
            Characters::Custom(codes) => codes.last().copied().unwrap_or(0),
        }
    }

    fn contains(&self, code: u32) -> bool {
        match self {
            Characters::Numeric => code == 32 || (48..=57).contains(&code),
            Characters::Printable => {
                matches!(code, 65..=90 | 97..=122 | 48..=57)
                    || " '()+,-./:=?".bytes().any(|byte| u32::from(byte) == code)
            }
            Characters::Ia5 => code < 128,
            Characters::Visible => (32..127).contains(&code),
            Characters::Bmp => code < 0x10000 && !(0xd800..=0xdfff).contains(&code),
            Characters::Universal => true,
            Characters::Custom(codes) => codes.binary_search(&code).is_ok(),
        }
    }

    /// The place of a character in the canonical order, counted from 0.
    /// Only those of a NumericString and of a permitted alphabet are ever
    /// counted so (X.691 30.5.4).
    fn index(&self, code: u32) -> Option<u32> {
        match self {
            Characters::Numeric if code == 32 => Some(0),
            Characters::Numeric => Some(code - 47),
            Characters::Custom(codes) => codes.binary_search(&code).ok().map(|index| index as u32),
            _ => None,
        }
    }

    fn at(&self, index: u32) -> Option<u32> {
        match self {
            Characters::Numeric => match index {
                0 => Some(32),
                1..=10 => Some(index + 47),
                _ => None,
            },
            Characters::Custom(codes) => codes.get(index as usize).copied(),
            _ => None,
        }
    }
}

/// The characters of a string of a known multiplier, and how a character
/// is a number.
struct Alphabet {
    characters: Characters,
    /// A character is its place in the canonical order, and not its code,
    /// when the greatest code does not fit in the bits of a character
    /// (X.691 30.5.4).
    by_index: bool,
}

impl Alphabet {
    fn encode(&self, code: u32) -> Result<u32> {
        if !self.characters.contains(code) {
            return encode_error("Expected a character in the permitted alphabet.");
        }

        if !self.by_index {
            return Ok(code);
        }

        self.characters
            .index(code)
            .ok_or_else(|| Error::encode("Expected a character in the permitted alphabet."))
    }

    fn decode(&self, value: u32, position: usize) -> Result<u32> {
        let code = if self.by_index {
            self.characters.at(value)
        } else {
            Some(value)
        };

        match code {
            Some(code) if self.characters.contains(code) => Ok(code),
            _ => decode_error("Expected a value in the permitted alphabet.", position),
        }
    }
}

/// How a string of a known multiplier is coded.
struct Known {
    alphabet: Alphabet,
    bits_per_character: usize,
}

impl Known {
    pub(crate) fn new(kind: StrKind, characters: &Option<Vec<char>>, aligned: bool) -> Known {
        let characters = match characters {
            Some(characters) => {
                let mut codes: Vec<u32> = characters.iter().map(|c| u32::from(*c)).collect();

                codes.sort_unstable();
                codes.dedup();

                Characters::Custom(codes)
            }
            None => match kind {
                StrKind::Numeric => Characters::Numeric,
                StrKind::Printable => Characters::Printable,
                StrKind::Ia5 => Characters::Ia5,
                StrKind::Bmp => Characters::Bmp,
                StrKind::Universal => Characters::Universal,
                _ => Characters::Visible,
            },
        };
        // B bits are enough for the characters, and B2 is the power of two
        // that is the least not less than B (X.691 30.5.2).
        let largest = u128::from(characters.count().saturating_sub(1));
        let bits_per_character = if aligned {
            bits_of_power_of_two(largest)
        } else {
            bits_of(largest)
        };
        let by_index = bits_per_character < 32
            && u64::from(characters.greatest()) > (1u64 << bits_per_character) - 1;

        Known {
            alphabet: Alphabet {
                characters,
                by_index,
            },
            bits_per_character,
        }
    }
}

/// The type of an item in the data: the kind, and where its constraints
/// are.
/// What an open type holds: a value of a type, of a member or the members
/// of a group (as a map).
enum Open<'a> {
    Type(TypeId),
    Member(&'a MemberInfo),
    Group(&'a [MemberInfo]),
}

struct Coder<'a> {
    types: &'a Types,
    aligned: bool,
}

pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    let coder = Coder {
        types,
        aligned: types.codec == Codec::Per,
    };
    let mut writer = Writer::new(coder.aligned);

    coder.encode_type(id, value, &mut writer, 0)?;

    Ok(writer.bytes)
}

pub fn decode<'a>(types: &'a Types, id: TypeId, data: &'a [u8]) -> Result<Value<'a>> {
    let coder = Coder {
        types,
        aligned: types.codec == Codec::Per,
    };
    let mut reader = Reader {
        data,
        position: 0,
        aligned: coder.aligned,
        budget: 8 * data.len() + 1024,
    };

    coder.decode_type(id, &mut reader, 0)
}

fn text_of<'v>(value: &'v Value<'_>) -> Result<&'v str> {
    match value {
        Value::Text(text) => Ok(text),
        _ => encode_error("Expected data of type str."),
    }
}

impl<'a> Coder<'a> {
    fn encode_type(&self, id: TypeId, value: &Value, w: &mut Writer, depth: usize) -> Result<()> {
        if depth > MAX_DEPTH {
            return encode_error("The value is nested too deeply.");
        }

        let node = self.types.node(id);
        let kind = &self.types.node(node.concrete).kind;
        let size = node.effective_size;

        match kind {
            Kind::Boolean => match value {
                Value::Boolean(value) => {
                    w.bit(*value);

                    Ok(())
                }
                _ => encode_error("Expected data of type bool."),
            },
            Kind::Integer(named) => {
                let number = match value {
                    Value::Integer(number) => Value::Integer(*number),
                    Value::BigInteger(bytes) => Value::BigInteger(bytes.clone()),
                    Value::Text(name) => {
                        match named.iter().find(|(candidate, _)| candidate == name) {
                            Some((_, number)) => Value::Integer(*number),
                            None => return encode_error("Expected an integer or a named number."),
                        }
                    }
                    _ => return encode_error("Expected data of type int or str."),
                };

                #[cfg(feature = "bigint")]
                if let Some(range) = &node.effective_big_range {
                    return self.encode_big_integer(&number, range, w);
                }

                self.encode_integer(&number, node.effective_range, w)
            }
            Kind::Real => {
                let encoded = match value {
                    Value::Real(value) => encode_real(*value)?,
                    Value::Integer(value) => encode_real(*value as f64)?,
                    Value::BigInteger(bytes) => {
                        encode_real(crate::ber::twos_complement_to_f64(bytes)?)?
                    }
                    _ => return encode_error("Expected data of type float or int."),
                };

                w.align();
                w.length(encoded.len());
                w.bytes(&encoded);

                Ok(())
            }
            Kind::Null => match value {
                Value::Null => Ok(()),
                _ => encode_error("Expected None."),
            },
            Kind::BitString(named) => match value {
                Value::Bits(bytes, bits) => {
                    self.encode_bit_string(bytes, *bits, !named.is_empty(), size, w)
                }
                _ => encode_error("Expected data of type tuple(bytes, int)."),
            },
            Kind::OctetString => match value {
                Value::Bytes(bytes) => self.encode_octet_string(bytes, size, w),
                _ => encode_error("Expected data of type bytes or bytearray."),
            },
            Kind::ObjectIdentifier => {
                let Value::ObjectIdentifier(oid) = value else {
                    return encode_error("Expected an OBJECT IDENTIFIER.");
                };

                w.align();
                w.length(oid.as_bytes().len());
                w.bytes(oid.as_bytes());

                Ok(())
            }
            Kind::Enumerated { root, additions } => {
                self.encode_enumerated(root, additions.as_deref(), value, w)
            }
            Kind::Sequence(members) | Kind::External(members) | Kind::Set(members) => {
                self.encode_members(members, matches!(kind, Kind::Set(_)), value, w, depth)
            }
            Kind::Choice(members) => self.encode_choice(members, value, w, depth),
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let Value::List(items) = value else {
                    return encode_error("Expected data of type list.");
                };

                self.encode_array(*element, items, size, w, depth)
            }
            Kind::Str(string) => {
                let text = text_of(value)?;

                self.encode_string(*string, text, size, &node.effective_alphabet, w)
            }
            Kind::Utc => match value {
                Value::DateTime(value) => {
                    let text = time::utc_time_to_string(value, true);

                    self.encode_string(StrKind::Visible, &text, None, &None, w)
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => {
                    let text = time::generalized_time_to_string(value, true);

                    self.encode_string(StrKind::Visible, &text, None, &None, w)
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Date => match value {
                Value::Date(value) => self.encode_date(value, w),
                _ => encode_error("Expected data of type datetime.date."),
            },
            Kind::TimeOfDay => match value {
                Value::Time(value) => self.encode_time_of_day(value, w),
                _ => encode_error("Expected data of type datetime.time."),
            },
            Kind::DateTime => match value {
                Value::DateTime(value) => {
                    self.encode_date(&value.date, w)?;
                    self.encode_time_of_day(&value.time, w)
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Duration => match value {
                Value::Text(text) => self.encode_duration(text, w),
                _ => encode_error("Expected data of type str."),
            },
            // An ANY is encoded as the open type that took its place
            // (X.691 11.2), with the encoding of its value.
            // An ANY DEFINED BY with choices is a member, see
            // `encode_member`.
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                encode_error("Bad AnyDefinedBy choice.")
            }
            Kind::OpenType | Kind::Any | Kind::AnyDefinedBy { .. } => match value {
                Value::Bytes(bytes) => self.encode_unbound_octets(bytes, w),
                _ => encode_error("Expected data of type bytes."),
            },
            Kind::Ref(_) => encode_error("The type is a reference."),
        }
    }

    fn encode_integer(&self, number: &Value, range: Option<Range>, w: &mut Writer) -> Result<()> {
        let bytes = number.to_twos_complement().unwrap();
        let lower = range.and_then(|range| range.min);
        let upper = range.and_then(|range| range.max);
        let inside = in_range(number, lower, upper);

        if range.is_some_and(|range| range.extensible) {
            w.bit(!inside);

            // Outside the root, as if there was no constraint (X.691 13.1).
            if !inside {
                w.align();

                return w.unconstrained_whole_number(&bytes);
            }
        } else if !inside {
            return encode_error("The value is not in the range.");
        }

        let (Some(min), Some(max)) = (lower, upper) else {
            w.align();

            return match lower {
                // A semi-constrained whole number: the value less the lower
                // bound (X.691 13.2.3, 11.7).
                Some(min) => w.unconstrained_whole_number(&less(&bytes, min)),
                // An unconstrained whole number (13.2.4, 11.8).
                None => w.unconstrained_whole_number(&bytes),
            };
        };
        let Value::Integer(value) = number else {
            return encode_error("The value is not in the range.");
        };
        let value = *value;
        let size = (max as u128).wrapping_sub(min as u128);
        let count = bits_of(size);

        if !self.aligned {
            w.uint((value as u128).wrapping_sub(min as u128), count);

            return Ok(());
        }

        let count = if size <= 65535 {
            count
        } else {
            let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
            let length = bytes_of((value as u128).wrapping_sub(min as u128));

            w.constrained_whole_number(length as i128 - 1, 0, 1i128 << indefinite, indefinite);
            w.align();

            8 * length
        };

        w.constrained_whole_number(value, min, max, count);

        Ok(())
    }

    /// An integer of which a bound does not fit in an `i128`, as
    /// `encode_integer` encodes the others (X.691 13).
    #[cfg(feature = "bigint")]
    fn encode_big_integer(
        &self,
        number: &Value,
        range: &crate::bigint::BigRange,
        w: &mut Writer,
    ) -> Result<()> {
        use crate::bigint;

        let Some(value) = bigint::to_big(number) else {
            return encode_error("Expected data of type int or str.");
        };
        let inside = range.in_bounds(&value);

        if range.extensible {
            w.bit(!inside);

            // Outside the root, as if there was no constraint (X.691 13.1).
            if !inside {
                w.align();

                return w.unconstrained_whole_number(&value.to_signed_bytes_be());
            }
        } else if !inside {
            return encode_error("The value is not in the range.");
        }

        let Some(min) = &range.min else {
            w.align();

            return w.unconstrained_whole_number(&value.to_signed_bytes_be());
        };
        let offset = (&value - min).magnitude().clone();
        let Some(max) = &range.max else {
            // A semi-constrained whole number (X.691 13.2.3, 11.7).
            w.align();

            return w.unconstrained_whole_number(&offset.to_bytes_be());
        };
        let size = (max - min).magnitude().clone();
        let count = bigint::bits(&size);
        let put = |w: &mut Writer, count: usize| {
            for index in (0..count).rev() {
                w.bit(offset.bit(index as u64));
            }
        };

        if !self.aligned {
            put(w, count);

            return Ok(());
        }

        // As Writer::constrained_whole_number has it (X.691 11.5.7).
        let values = &size + 1u32;

        if values <= 255u32.into() {
            put(w, count);
        } else if values == 256u32.into() {
            w.align_always();
            put(w, 8);
        } else if values <= 65536u32.into() {
            w.align_always();
            put(w, 16);
        } else {
            let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
            let length = offset.to_bytes_be().len();

            w.constrained_whole_number(length as i128 - 1, 0, 1i128 << indefinite, indefinite);
            w.align_always();
            put(w, 8 * length);
        }

        Ok(())
    }

    fn encode_bit_string(
        &self,
        bytes: &[u8],
        bits: usize,
        named: bool,
        size: Option<Range>,
        w: &mut Writer,
    ) -> Result<()> {
        if bytes.len() * 8 < bits {
            return encode_error("Expected more bits of data.");
        }

        let (data, bits) = if named {
            // Trailing zero bits are left out, but for the minimum (X.691
            // 16.2, 16.3).
            let (mut data, mut count) = clean_bits(bytes, 8 * bytes.len(), true);

            if let Some(min) = size.and_then(|size| size.min)
                && (count as i128) < min
            {
                count = min as usize;
                data.resize(count.div_ceil(8), 0);
            }

            (data, count)
        } else {
            (Vec::from(bytes), bits)
        };
        let size = extended(size, bits as i128, w);

        check_size(size, bits as i128)?;

        if is_unbound(size) {
            w.align();

            let mut fragments = Fragmenter::new(bits);

            while let Some((offset, length)) = fragments.next(w) {
                w.bits_of(&data[offset / 8..], length);
            }

            return Ok(());
        }

        let (min, max) = size_bounds(size).unwrap();
        let count = bits_of((max - min) as u128);

        if min != max {
            if self.aligned {
                w.constrained_whole_number(bits as i128, min, max, count);

                // No padding for nothing (X.691 11.9.3.3).
                if bits > 0 {
                    w.align();
                }
            } else {
                w.uint((bits as i128 - min) as u128, count);
            }
        } else if self.aligned && min > 16 {
            w.align();
        }

        w.bits_of(&data, bits);

        Ok(())
    }

    fn encode_octet_string(&self, data: &[u8], size: Option<Range>, w: &mut Writer) -> Result<()> {
        let size = extended(size, data.len() as i128, w);

        check_size(size, data.len() as i128)?;

        let Some((min, max)) = size_bounds(size).filter(|_| !is_unbound(size)) else {
            return self.encode_unbound_octets(data, w);
        };
        let span = (max - min) as u128;

        if span == 0 && max >= 65536 {
            return self.encode_unbound_octets(data, w);
        }

        let count = bits_of(span);
        let mut align = true;

        if min != max {
            if self.aligned {
                w.constrained_whole_number(data.len() as i128, min, max, count);
            } else {
                w.uint((data.len() as i128 - min) as u128, count);
            }

            // No padding for nothing (X.691 11.9.3.3).
            align = !data.is_empty();
        } else if max <= 2 {
            align = false;
        }

        if align && self.aligned {
            w.align();
        }

        w.bytes(data);

        Ok(())
    }

    fn encode_unbound_octets(&self, data: &[u8], w: &mut Writer) -> Result<()> {
        w.align();

        let mut fragments = Fragmenter::new(data.len());

        while let Some((offset, length)) = fragments.next(w) {
            w.align();
            w.bytes(&data[offset..offset + length]);
        }

        Ok(())
    }

    /// The value of an open type of the type `id`, see `put_open_type`.
    fn encode_open_type(
        &self,
        id: TypeId,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let mut inner = w.sub();

        self.encode_type(id, value, &mut inner, depth + 1)?;
        self.put_open_type(inner, w)
    }

    /// An open type of what `inner` holds: its complete encoding, which is
    /// one byte 0 if it is empty (X.691 11.1), as an unconstrained OCTET
    /// STRING, in fragments if it is 16384 bytes or more (11.2).
    fn put_open_type(&self, mut inner: Writer, w: &mut Writer) -> Result<()> {
        if inner.bits == 0 {
            inner.bytes.push(0);
        }

        self.encode_unbound_octets(&inner.bytes, w)
    }

    fn encode_enumerated(
        &self,
        root: &[(String, i64)],
        additions: Option<&[(String, i64)]>,
        value: &Value,
        w: &mut Writer,
    ) -> Result<()> {
        let mut sorted: Vec<&(String, i64)> = root.iter().collect();

        sorted.sort_by_key(|(_, number)| *number);

        let find = |entry: &&(String, i64)| match value {
            Value::Text(name) if !self.types.numeric_enums => &entry.0 == name,
            Value::Integer(number) if self.types.numeric_enums => i128::from(entry.1) == *number,
            _ => false,
        };
        let count = bits_of(root.len().saturating_sub(1) as u128);

        if let Some(index) = sorted.iter().position(find) {
            if additions.is_some() {
                w.bit(false);
            }

            w.uint(index as u128, count);

            return Ok(());
        }

        let Some(additions) = additions else {
            return encode_error("Expected an enumeration value.");
        };
        let index = additions
            .iter()
            .position(|entry| find(&entry))
            .ok_or_else(|| Error::encode("Expected an enumeration value."))?;

        w.bit(true);
        w.normally_small_whole_number(index as u128);

        Ok(())
    }

    fn encode_choice(
        &self,
        members: &'a Members,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let Value::Choice(name, inner) = value else {
            return encode_error("Expected data of type tuple(str, object).");
        };
        let additions = choice_additions(members);
        // The index of an alternative is its place in the canonical order
        // of tags (X.691 23.2).
        let root_index = members
            .order
            .iter()
            .position(|index| &members.root[*index].name == name);

        if members.additions.is_some() {
            w.bit(root_index.is_none());
        }

        if let Some(index) = root_index {
            if members.root.len() > 1 {
                let maximum = members.root.len() - 1;
                let count = bits_of(maximum as u128);

                if !self.aligned {
                    w.uint(index as u128, count);
                } else if maximum <= 65535 {
                    w.constrained_whole_number(index as i128, 0, maximum as i128, count);
                } else {
                    let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
                    let length = bytes_of(index as u128);

                    w.constrained_whole_number(
                        length as i128 - 1,
                        0,
                        1i128 << indefinite,
                        indefinite,
                    );
                    w.align();
                    w.constrained_whole_number(index as i128, 0, maximum as i128, 8 * length);
                }
            }

            return self
                .encode_type(members.root[members.order[index]].ty, inner, w, depth + 1)
                .map_err(|error| error.in_member(name));
        }

        let Some(index) = additions.iter().position(|member| &member.name == name) else {
            return encode_error("Expected a name of an alternative of the CHOICE.");
        };
        let mut open = w.sub();

        self.encode_type(additions[index].ty, inner, &mut open, depth + 1)
            .map_err(|error| error.in_member(name))?;
        w.normally_small_whole_number(index as u128);
        self.put_open_type(open, w)
    }

    fn encode_array(
        &self,
        element: TypeId,
        items: &[Value],
        size: Option<Range>,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let length = items.len() as i128;
        let size = extended(size, length, w);

        check_size(size, length)?;

        if is_unbound(size) {
            w.align();

            let mut fragments = Fragmenter::new(items.len());

            while let Some((offset, count)) = fragments.next(w) {
                for (index, item) in items[offset..offset + count].iter().enumerate() {
                    self.encode_type(element, item, w, depth + 1)
                        .map_err(|error| error.in_element(offset + index))?;
                }
            }

            return Ok(());
        }

        let (min, max) = size_bounds(size).unwrap();

        if min != max {
            let count = bits_of((max - min) as u128);

            if self.aligned {
                w.constrained_whole_number(length, min, max, count);
            } else {
                w.uint((length - min) as u128, count);
            }
        }

        for (index, item) in items.iter().enumerate() {
            self.encode_type(element, item, w, depth + 1)
                .map_err(|error| error.in_element(index))?;
        }

        Ok(())
    }

    fn encode_string(
        &self,
        kind: StrKind,
        text: &str,
        size: Option<Range>,
        characters: &Option<Vec<char>>,
        w: &mut Writer,
    ) -> Result<()> {
        match kind {
            StrKind::Numeric
            | StrKind::Printable
            | StrKind::Ia5
            | StrKind::Visible
            | StrKind::Bmp
            | StrKind::Universal => self.encode_known(kind, text, size, characters, w),
            _ => {
                let bytes: Vec<u8> = match kind {
                    StrKind::Utf8 => Vec::from(text.as_bytes()),
                    StrKind::Universal => text
                        .chars()
                        .flat_map(|c| u32::from(c).to_be_bytes())
                        .collect(),
                    _ => text
                        .chars()
                        .map(|c| {
                            u8::try_from(u32::from(c))
                                .map_err(|_| Error::encode("A character is not Latin-1."))
                        })
                        .collect::<Result<_>>()?,
                };
                let multiplier = if kind == StrKind::Universal { 4 } else { 1 };
                // The length is of characters, except for UTF8String.
                let count = if kind == StrKind::Utf8 {
                    bytes.len()
                } else {
                    text.chars().count()
                };

                w.align();

                let mut fragments = Fragmenter::new(count);

                while let Some((offset, length)) = fragments.next(w) {
                    let (start, end) = if kind == StrKind::Utf8 {
                        (offset, offset + length)
                    } else {
                        (offset * multiplier, (offset + length) * multiplier)
                    };

                    w.bytes(&bytes[start..end]);
                }

                Ok(())
            }
        }
    }

    fn encode_known(
        &self,
        kind: StrKind,
        text: &str,
        size: Option<Range>,
        characters: &Option<Vec<char>>,
        w: &mut Writer,
    ) -> Result<()> {
        let known = Known::new(kind, characters, self.aligned);
        let codes: Vec<u32> = text.chars().map(u32::from).collect();
        let length = codes.len() as i128;
        // Outside the root of an extensible size, with the permitted
        // alphabet still (X.691 30.4).
        let size = extended(size, length, w);

        check_size(size, length)?;

        let put = |w: &mut Writer, codes: &[u32]| -> Result<()> {
            for code in codes {
                w.uint(
                    u128::from(known.alphabet.encode(*code)?),
                    known.bits_per_character,
                );
            }

            Ok(())
        };

        if is_unbound(size) {
            w.align();

            let mut fragments = Fragmenter::new(codes.len());

            while let Some((offset, count)) = fragments.next(w) {
                put(w, &codes[offset..offset + count])?;
            }

            return Ok(());
        }

        let (min, max) = size_bounds(size).unwrap();

        if min != max {
            let count = bits_of((max - min) as u128);

            if self.aligned {
                w.constrained_whole_number(length, min, max, count);

                if max * known.bits_per_character as i128 >= 16 && length > 0 {
                    w.align();
                }
            } else {
                w.uint((length - min) as u128, count);
            }
        } else if self.aligned && max * known.bits_per_character as i128 > 16 {
            w.align();
        }

        put(w, &codes)
    }

    fn encode_date(&self, date: &Date, w: &mut Writer) -> Result<()> {
        let year = i128::from(date.year);
        let (index, min, max) = if (2005..=2020).contains(&year) {
            (0, Some(2005), Some(2020))
        } else if (2021..=2276).contains(&year) {
            (1, Some(2021), Some(2276))
        } else if (1749..=2004).contains(&year) {
            (2, Some(1749), Some(2004))
        } else {
            (3, None, Some(1748))
        };

        // The choice, in 2 bits.
        w.uint(index, 2);

        match (min, max) {
            (Some(min), Some(max)) => {
                let count = bits_of((max - min) as u128);

                if self.aligned {
                    w.constrained_whole_number(year, min, max, count);
                } else {
                    w.uint((year - min) as u128, count);
                }
            }
            _ => {
                w.align();
                w.unconstrained_whole_number(&integer_to_twos_complement(year))?;
            }
        }

        self.put_range(i128::from(date.month), 1, 12, w);
        self.put_range(i128::from(date.day), 1, 31, w);

        Ok(())
    }

    fn put_range(&self, value: i128, min: i128, max: i128, w: &mut Writer) {
        let count = bits_of((max - min) as u128);

        if self.aligned {
            w.constrained_whole_number(value, min, max, count);
        } else {
            w.uint((value - min) as u128, count);
        }
    }

    fn encode_time_of_day(&self, time: &Time, w: &mut Writer) -> Result<()> {
        time::check_local_time(time)?;

        self.put_range(i128::from(time.hour), 0, 24, w);
        self.put_range(i128::from(time.minute), 0, 59, w);
        self.put_range(i128::from(time.second), 0, 60, w);

        Ok(())
    }

    fn put_extensible_integer(
        &self,
        value: i128,
        min: i128,
        max: i128,
        w: &mut Writer,
    ) -> Result<()> {
        let inside = value >= min && value <= max;
        w.bit(!inside);
        if inside {
            self.put_range(value, min, max, w);
            Ok(())
        } else {
            w.align();
            w.unconstrained_whole_number(&integer_to_twos_complement(value))
        }
    }

    /// A time element of DURATION-INTERVAL-ENCODING, of which the root
    /// is `min` to `max` and the extension what is above it.
    fn put_duration_element(
        &self,
        value: &Value,
        min: i128,
        max: i128,
        w: &mut Writer,
    ) -> Result<()> {
        match value {
            Value::Integer(number) => self.put_extensible_integer(*number, min, max, w),
            // More than 128 bits, so in the extension.
            Value::BigInteger(bytes) => {
                w.bit(true);
                w.align();
                w.unconstrained_whole_number(bytes)
            }
            _ => encode_error("Expected an integer."),
        }
    }

    fn encode_duration(&self, text: &str, w: &mut Writer) -> Result<()> {
        time::validate_duration(text)?;
        let c = time::duration_to_components(text)?;

        w.bit(c.years.is_some());
        w.bit(c.months.is_some());
        w.bit(c.weeks.is_some());
        w.bit(c.days.is_some());
        w.bit(c.hours.is_some());
        w.bit(c.minutes.is_some());
        w.bit(c.seconds.is_some());
        w.bit(c.fractional_part.is_some());

        for (element, max) in [
            (&c.years, 31),
            (&c.months, 15),
            (&c.weeks, 63),
            (&c.days, 31),
            (&c.hours, 31),
            (&c.minutes, 63),
            (&c.seconds, 63),
        ] {
            if let Some(element) = element {
                self.put_duration_element(element, 0, max, w)?;
            }
        }
        if let Some((nod, fv)) = c.fractional_part {
            self.put_extensible_integer(nod, 1, 3, w)?;
            self.put_extensible_integer(fv, 0, 999, w)?;
        }

        Ok(())
    }

    fn encode_members(
        &self,
        members: &'a Members,
        is_set: bool,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let Value::Map(map) = value else {
            return encode_error("Expected data of type dict.");
        };
        let mut lookup = Lookup::new(map);
        // A SET is encoded in the order of its tags.
        let encode_root = |w: &mut Writer, lookup: &mut Lookup<'_, '_>| {
            if is_set {
                let root: Vec<&MemberInfo> = members
                    .order
                    .iter()
                    .map(|index| &members.root[*index])
                    .collect();

                self.encode_root(&root, lookup, w, depth)
            } else {
                self.encode_root(&members.root, lookup, w, depth)
            }
        };

        match &members.additions {
            None => encode_root(w, &mut lookup),
            Some(additions) => {
                let marker = w.bits;

                w.bit(false);
                encode_root(w, &mut lookup)?;

                if !additions.is_empty()
                    && self.encode_additions(additions, &mut lookup, w, depth)?
                {
                    w.set_bit(marker);
                }

                Ok(())
            }
        }
    }

    fn encode_root<M: Borrow<MemberInfo>>(
        &self,
        root: &[M],
        lookup: &mut Lookup<'_, '_>,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        for member in root
            .iter()
            .map(Borrow::<MemberInfo>::borrow)
            .filter(|member| member.optional || member.has_default())
        {
            let present = lookup
                .map
                .iter()
                .find(|(name, _)| name == &member.name)
                .map(|(_, value)| value);

            if member.optional {
                w.bit(present.is_some());
            } else {
                w.bit(present.is_some_and(|value| !is_default(self.types, member, value)));
            }
        }

        for member in root {
            self.encode_member(member.borrow(), lookup, w, false, depth)?;
        }

        Ok(())
    }

    fn encode_member(
        &self,
        member: &MemberInfo,
        lookup: &mut Lookup<'_, '_>,
        w: &mut Writer,
        encode_default: bool,
        depth: usize,
    ) -> Result<bool> {
        match lookup.find(&member.name) {
            Some(value) => {
                if !member.has_default() || !is_default(self.types, member, value) || encode_default
                {
                    match self.types.defined_by(member.ty, lookup.map) {
                        Some(Some(ty)) => self.encode_open_type(ty, value, w, depth),
                        Some(None) => encode_error("Bad AnyDefinedBy choice."),
                        None => self.encode_type(member.ty, value, w, depth + 1),
                    }
                    .map_err(|error| error.in_member(&member.name))?;
                }

                Ok(true)
            }
            None if member.optional || member.has_default() => Ok(false),
            None => encode_error("A member is not found in the SEQUENCE or SET."),
        }
    }

    /// The extension additions. Returns if any are there.
    fn encode_additions(
        &self,
        additions: &[Addition],
        lookup: &mut Lookup<'_, '_>,
        w: &mut Writer,
        depth: usize,
    ) -> Result<bool> {
        let mut presence: u128 = 0;
        let mut encoders: Vec<Writer> = Vec::new();
        let mut counted = 0;

        // An addition that can not be encoded ends the additions.
        for addition in additions {
            presence <<= 1;
            counted += 1;

            let mut sub = w.sub();
            let result = match addition {
                Addition::Group(group) => {
                    let optionals = group
                        .iter()
                        .filter(|m| m.optional || m.has_default())
                        .count();

                    self.encode_root(group, lookup, &mut sub, depth).map(|()| {
                        // A group with none of its members is not there.
                        if sub.are_all_bits_zero() && sub.bits == optionals {
                            sub = w.sub();
                        }

                        false
                    })
                }
                Addition::Member(member) => {
                    self.encode_member(member, lookup, &mut sub, true, depth)
                }
            };

            match result {
                Ok(named) => {
                    if sub.bits > 0 || named {
                        encoders.push(sub);
                        presence |= 1;
                    }
                }
                Err(Error::Encode(..)) => break,
                Err(error) => return Err(error),
            }
        }

        if encoders.is_empty() {
            return Ok(false);
        }

        presence <<= additions.len() - counted;
        w.normally_small_length(additions.len())?;
        w.uint(presence, additions.len());
        w.align();

        for sub in encoders {
            self.put_open_type(sub, w)?;
        }

        Ok(true)
    }

    // Decoding.

    fn decode_type(&self, id: TypeId, r: &mut Reader<'a>, depth: usize) -> Result<Value<'a>> {
        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.", r.position);
        }

        let node = self.types.node(id);
        let kind = &self.types.node(node.concrete).kind;
        let size = node.effective_size;

        match kind {
            Kind::Boolean => r.bit().map(Value::Boolean),
            #[cfg(feature = "bigint")]
            Kind::Integer(_) if node.effective_big_range.is_some() => {
                self.decode_big_integer(node.effective_big_range.as_ref().unwrap(), r)
            }
            Kind::Integer(_) => self.decode_integer(node.effective_range, r),
            Kind::Real => {
                r.align();

                let length = r.length()?;
                let bytes = r.octets(length)?;

                // The contents of CER and DER (X.691 15.2).
                decode_real(&bytes, r.position / 8, true).map(Value::Real)
            }
            Kind::Null => Ok(Value::Null),
            Kind::BitString(_) => self.decode_bit_string(size, r),
            Kind::OctetString => self.decode_octet_string(size, r),
            Kind::ObjectIdentifier => {
                r.align();

                let length = r.length()?;
                let bytes = r.octets(length)?;

                ObjectIdentifier::decode(bytes, r.position / 8, false).map(Value::ObjectIdentifier)
            }
            Kind::Enumerated { root, additions } => {
                self.decode_enumerated(root, additions.as_deref(), r)
            }
            Kind::Sequence(members) | Kind::External(members) | Kind::Set(members) => {
                self.decode_members(members, matches!(kind, Kind::Set(_)), r, depth)
            }
            Kind::Choice(members) => self.decode_choice(members, r, depth),
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                self.decode_array(*element, size, r, depth)
            }
            Kind::Str(string) => self.decode_string(*string, size, &node.effective_alphabet, r),
            Kind::Utc => {
                let text = self.decode_known(StrKind::Visible, None, &None, r)?;

                time::utc_time_from_string(&text, true).map(Value::DateTime)
            }
            Kind::Generalized => {
                let text = self.decode_known(StrKind::Visible, None, &None, r)?;

                time::generalized_time_from_string(&text, true).map(Value::DateTime)
            }
            Kind::Date => self.decode_date(r).map(Value::Date),
            Kind::TimeOfDay => self.decode_time_of_day(r).map(Value::Time),
            Kind::DateTime => {
                let date = self.decode_date(r)?;
                let time = self.decode_time_of_day(r)?;

                Ok(Value::DateTime(DateTime { date, time }))
            }
            Kind::Duration => self
                .decode_duration(r)
                .map(|text| Value::Text(Cow::Owned(text))),
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                decode_error("Bad AnyDefinedBy choice.", r.position)
            }
            Kind::OpenType | Kind::Any | Kind::AnyDefinedBy { .. } => {
                self.decode_unbound_octets(r).map(Value::Bytes)
            }
            Kind::Ref(_) => decode_error("The type is a reference.", r.position),
        }
    }

    fn decode_integer(&self, range: Option<Range>, r: &mut Reader<'a>) -> Result<Value<'a>> {
        if range.is_some_and(|range| range.extensible) && r.bit()? {
            r.align();

            return r.unconstrained_whole_number();
        }

        let lower = range.and_then(|range| range.min);
        let upper = range.and_then(|range| range.max);
        let (Some(min), Some(max)) = (lower, upper) else {
            r.align();

            return match lower {
                Some(min) => {
                    let length = r.length()?;
                    let bytes = r.octets(length)?;

                    Ok(plus(&bytes, min))
                }
                None => r.unconstrained_whole_number(),
            };
        };
        let size = (max as u128).wrapping_sub(min as u128);
        let count = bits_of(size);

        if !self.aligned {
            return Ok(Value::Integer((r.uint(count)? as i128).wrapping_add(min)));
        }

        let count = if size <= 65535 {
            count
        } else {
            let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
            let length = r.constrained_whole_number(0, 1i128 << indefinite, indefinite)? + 1;

            r.align();

            8 * length as usize
        };

        r.constrained_whole_number(min, max, count)
            .map(Value::Integer)
    }

    /// An integer of which a bound does not fit in an `i128`, as
    /// `decode_integer` decodes the others.
    #[cfg(feature = "bigint")]
    fn decode_big_integer(
        &self,
        range: &crate::bigint::BigRange,
        r: &mut Reader<'a>,
    ) -> Result<Value<'a>> {
        use crate::bigint;
        use num_bigint::{BigInt, BigUint};

        if range.extensible && r.bit()? {
            r.align();

            return r.unconstrained_whole_number();
        }

        let Some(min) = &range.min else {
            r.align();

            return r.unconstrained_whole_number();
        };
        let Some(max) = &range.max else {
            r.align();

            let length = r.length()?;
            let bytes = r.octets(length)?;

            return Ok(bigint::to_value(
                &(min + BigInt::from(BigUint::from_bytes_be(&bytes))),
            ));
        };
        let size = (max - min).magnitude().clone();
        let count = bigint::bits(&size);
        let get = |r: &mut Reader<'a>, count: usize| -> Result<BigUint> {
            let bytes = r.bits(count)?;

            Ok(BigUint::from_bytes_be(&bytes) >> (8 * bytes.len() - count))
        };
        let values = &size + 1u32;
        let offset = if !self.aligned || values <= 255u32.into() {
            get(r, count)?
        } else if values == 256u32.into() {
            r.align_always();
            get(r, 8)?
        } else if values <= 65536u32.into() {
            r.align_always();
            get(r, 16)?
        } else {
            let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
            let length = r.constrained_whole_number(0, 1i128 << indefinite, indefinite)? + 1;

            r.align_always();
            get(r, 8 * length as usize)?
        };

        Ok(bigint::to_value(&(min + BigInt::from(offset))))
    }

    fn decode_bit_string(&self, size: Option<Range>, r: &mut Reader<'a>) -> Result<Value<'a>> {
        let size = read_extended(size, r)?;

        if is_unbound(size) {
            r.align();

            let mut data = Cow::Borrowed(&[][..]);
            let mut total = 0;

            loop {
                let length = r.length()?;
                let part = r.bit_octets(length)?;

                // Most are a fragment.
                if total == 0 && length < 16384 {
                    data = part;
                    total = length;

                    break;
                }

                // The parts are whole bytes but for the last.
                data.to_mut().extend_from_slice(&part);
                total += length;

                if length < 16384 {
                    break;
                }
            }

            return Ok(Value::Bits(data, total));
        }

        let (min, max) = size_bounds(size).unwrap();
        let count = bits_of((max - min) as u128);
        let mut bits = min;

        if min != max {
            bits = if self.aligned {
                r.constrained_whole_number(min, max, count)?
            } else {
                r.uint(count)? as i128 + min
            };

            if bits > 0 {
                r.align();
            }
        } else if self.aligned && min > 16 {
            r.align();
        }

        let bits = bits as usize;

        r.bit_octets(bits).map(|data| Value::Bits(data, bits))
    }

    fn decode_octet_string(&self, size: Option<Range>, r: &mut Reader<'a>) -> Result<Value<'a>> {
        let size = read_extended(size, r)?;
        let unbound = |r: &mut Reader<'a>| self.decode_unbound_octets(r).map(Value::Bytes);
        let Some((min, max)) = size_bounds(size).filter(|_| !is_unbound(size)) else {
            return unbound(r);
        };
        let span = (max - min) as u128;

        if span == 0 && max >= 65536 {
            return unbound(r);
        }

        let count = bits_of(span);
        let mut length = min;
        let mut align = true;

        if min != max {
            length = if self.aligned {
                r.constrained_whole_number(min, max, count)?
            } else {
                r.uint(count)? as i128 + min
            };
            align = length > 0;
        } else if max <= 2 {
            align = false;
        }

        if align {
            r.align();
        }

        r.octets(length as usize).map(Value::Bytes)
    }

    /// The octets of an unconstrained OCTET STRING, in fragments if there
    /// are 16384 or more.
    fn decode_unbound_octets(&self, r: &mut Reader<'a>) -> Result<Cow<'a, [u8]>> {
        r.align();

        let mut data = Vec::new();

        loop {
            let length = r.length()?;

            r.align();

            // Most are a fragment.
            if data.is_empty() && length < 16384 {
                return r.octets(length);
            }

            data.extend_from_slice(&r.octets(length)?);

            if length < 16384 {
                break;
            }
        }

        Ok(Cow::Owned(data))
    }

    /// The value of an open type of the type `id`, see `get_open_type`.
    fn decode_open_type(&self, id: TypeId, r: &mut Reader<'a>, depth: usize) -> Result<Value<'a>> {
        self.get_open_type(r, Open::Type(id), &[], depth)
    }

    /// What `open` reads from an open type, see `put_open_type`. It has
    /// to fill the octets, but for the padding of the last. The value
    /// borrows from the data, but for the octets that are not whole bytes
    /// of it or are in fragments, which it is decoded from a copy of.
    fn get_open_type(
        &self,
        r: &mut Reader<'a>,
        open: Open<'a>,
        values: &[(Cow<'_, str>, Value<'_>)],
        depth: usize,
    ) -> Result<Value<'a>> {
        let offset = r.position;

        match self.decode_unbound_octets(r)? {
            Cow::Borrowed(data) => self.decode_open(data, offset, r, open, values, depth),
            Cow::Owned(data) => self
                .decode_open(&data, offset, r, open, values, depth)
                .map(Value::into_owned),
        }
    }

    fn decode_open<'d>(
        &self,
        data: &'d [u8],
        offset: usize,
        r: &mut Reader<'_>,
        open: Open<'d>,
        values: &[(Cow<'_, str>, Value<'_>)],
        depth: usize,
    ) -> Result<Value<'d>>
    where
        'a: 'd,
    {
        let coder: &Coder<'d> = self;
        let mut inner = Reader {
            data,
            position: 0,
            aligned: r.aligned,
            budget: r.budget,
        };
        // Where in all of the data, or at the open type if it is in
        // fragments.
        let start = r.position - 8 * data.len();
        let value = match open {
            Open::Type(id) => coder.decode_type(id, &mut inner, depth + 1),
            Open::Member(member) => coder.decode_member(member, values, &[], &mut inner, depth),
            Open::Group(group) => coder
                .decode_root(group.iter(), values, &mut inner, depth)
                .map(Value::Map),
        }
        .map_err(|error| match error {
            Error::Decode(what, at, location) if data.len() < 16384 => {
                Error::Decode(what, start + at, location)
            }
            Error::Decode(what, _, location) => Error::Decode(what, offset, location),
            error => error,
        })?;

        r.budget = inner.budget;

        let empty = inner.position == 0 && data == [0];

        if inner.position.div_ceil(8) != data.len() && !empty {
            return decode_error("The value does not fill its open type.", r.position);
        }

        Ok(value)
    }

    fn decode_enumerated(
        &self,
        root: &'a [(String, i64)],
        additions: Option<&'a [(String, i64)]>,
        r: &mut Reader<'a>,
    ) -> Result<Value<'a>> {
        let mut sorted: Vec<&'a (String, i64)> = root.iter().collect();

        sorted.sort_by_key(|(_, number)| *number);

        let named = |entry: &'a (String, i64)| {
            if self.types.numeric_enums {
                Value::Integer(i128::from(entry.1))
            } else {
                Value::Text(Cow::Borrowed(entry.0.as_str()))
            }
        };

        if additions.is_some() && r.bit()? {
            let index = r.normally_small_whole_number()? as usize;

            return Ok(additions
                .and_then(|additions| additions.get(index))
                .map_or(Value::Null, named));
        }

        let count = bits_of(root.len().saturating_sub(1) as u128);
        let index = r.uint(count)? as usize;

        match sorted.get(index) {
            Some(entry) => Ok(named(entry)),
            None => decode_error("Expected an enumeration index.", r.position),
        }
    }

    fn decode_choice(
        &self,
        members: &'a Members,
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<Value<'a>> {
        let additions = choice_additions(members);

        if members.additions.is_some() && r.bit()? {
            let index = r.normally_small_whole_number()? as usize;

            return match additions.get(index) {
                // One of a later version.
                None => {
                    self.decode_unbound_octets(r)?;

                    Ok(Value::Choice(Cow::Borrowed(""), Value::Null.into()))
                }
                Some(member) => {
                    let value = self
                        .decode_open_type(member.ty, r, depth)
                        .map_err(|error| error.in_member(&member.name))?;

                    Ok(Value::Choice(
                        Cow::Borrowed(member.name.as_str()),
                        value.into(),
                    ))
                }
            };
        }

        let mut index = 0;

        if members.root.len() > 1 {
            let maximum = members.root.len() - 1;
            let count = bits_of(maximum as u128);

            index = if !self.aligned {
                r.uint(count)? as usize
            } else if maximum <= 65535 {
                r.constrained_whole_number(0, maximum as i128, count)? as usize
            } else {
                let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
                let length = r.constrained_whole_number(0, 1i128 << indefinite, indefinite)? + 1;

                r.align();

                r.constrained_whole_number(0, maximum as i128, 8 * length as usize)? as usize
            };
        }

        match members.order.get(index).map(|index| &members.root[*index]) {
            Some(member) => {
                let value = self
                    .decode_type(member.ty, r, depth + 1)
                    .map_err(|error| error.in_member(&member.name))?;

                Ok(Value::Choice(
                    Cow::Borrowed(member.name.as_str()),
                    value.into(),
                ))
            }
            None => decode_error("Expected a choice index.", r.position),
        }
    }

    fn decode_array(
        &self,
        element: TypeId,
        size: Option<Range>,
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<Value<'a>> {
        let size = read_extended(size, r)?;
        let mut items = Vec::new();

        if is_unbound(size) {
            r.align();

            loop {
                let count = r.length()?;

                r.spend(count)?;

                for _ in 0..count {
                    items.push(
                        self.decode_type(element, r, depth + 1)
                            .map_err(|error| error.in_element(items.len()))?,
                    );
                }

                if count < 16384 {
                    break;
                }
            }

            return Ok(Value::List(items));
        }

        let length = {
            let (min, max) = size_bounds(size).unwrap();
            let count = bits_of((max - min) as u128);

            if min == max {
                min as usize
            } else if self.aligned {
                r.constrained_whole_number(min, max, count)? as usize
            } else {
                r.uint(count)? as usize + min as usize
            }
        };

        r.spend(length)?;

        for _ in 0..length {
            items.push(
                self.decode_type(element, r, depth + 1)
                    .map_err(|error| error.in_element(items.len()))?,
            );
        }

        Ok(Value::List(items))
    }

    fn decode_string(
        &self,
        kind: StrKind,
        size: Option<Range>,
        characters: &Option<Vec<char>>,
        r: &mut Reader<'a>,
    ) -> Result<Value<'a>> {
        match kind {
            StrKind::Numeric
            | StrKind::Printable
            | StrKind::Ia5
            | StrKind::Visible
            | StrKind::Bmp
            | StrKind::Universal => self
                .decode_known(kind, size, characters, r)
                .map(|text| Value::Text(Cow::Owned(text))),
            _ => {
                r.align();

                let mut bytes = Cow::Borrowed(&[][..]);

                loop {
                    let length = r.length()?;
                    let part = r.octets(length)?;

                    // Most are a fragment.
                    if bytes.is_empty() && length < 16384 {
                        bytes = part;

                        break;
                    }

                    bytes.to_mut().extend_from_slice(&part);

                    if length < 16384 {
                        break;
                    }
                }

                match bytes {
                    Cow::Borrowed(bytes) => crate::ber::string_text(kind, bytes, r.position),
                    Cow::Owned(bytes) => crate::ber::string_text(kind, &bytes, r.position)
                        .map(|text| Cow::Owned(text.into_owned())),
                }
                .map(Value::Text)
            }
        }
    }

    fn decode_known(
        &self,
        kind: StrKind,
        size: Option<Range>,
        characters: &Option<Vec<char>>,
        r: &mut Reader<'a>,
    ) -> Result<String> {
        let known = Known::new(kind, characters, self.aligned);
        let size = read_extended(size, r)?;

        let mut text = String::new();
        let get = |r: &mut Reader<'a>, count: usize, text: &mut String| -> Result<()> {
            r.spend(count)?;

            for _ in 0..count {
                let value = r.uint(known.bits_per_character)? as u32;
                let code = known.alphabet.decode(value, r.position)?;

                text.push(
                    char::from_u32(code)
                        .ok_or_else(|| Error::decode("Bad character.", r.position))?,
                );
            }

            Ok(())
        };

        if is_unbound(size) {
            r.align();

            loop {
                let count = r.length()?;

                get(r, count, &mut text)?;

                if count < 16384 {
                    break;
                }
            }

            return Ok(text);
        }

        let (min, max) = size_bounds(size).unwrap();
        let count = bits_of((max - min) as u128);
        let length;

        if min != max {
            length = if self.aligned {
                r.constrained_whole_number(min, max, count)?
            } else {
                r.uint(count)? as i128 + min
            };

            if self.aligned && max * known.bits_per_character as i128 >= 16 && length > 0 {
                r.align();
            }
        } else {
            length = min;

            if self.aligned && max * known.bits_per_character as i128 > 16 {
                r.align();
            }
        }

        get(r, length as usize, &mut text)?;

        Ok(text)
    }

    fn get_range(&self, min: i128, max: i128, r: &mut Reader<'a>) -> Result<i128> {
        let count = bits_of((max - min) as u128);

        if self.aligned {
            r.constrained_whole_number(min, max, count)
        } else {
            Ok(r.uint(count)? as i128 + min)
        }
    }

    fn decode_date(&self, r: &mut Reader<'a>) -> Result<Date> {
        let index = r.uint(2)?;
        let year = match index {
            0 => self.get_range(2005, 2020, r)?,
            1 => self.get_range(2021, 2276, r)?,
            2 => self.get_range(1749, 2004, r)?,
            _ => {
                r.align();

                match r.unconstrained_whole_number()? {
                    Value::Integer(year) => year,
                    _ => return decode_error("The year is too large.", r.position),
                }
            }
        };
        let month = self.get_range(1, 12, r)?;
        let day = self.get_range(1, 31, r)?;

        Ok(Date {
            year: year as i32,
            month: month as u8,
            day: day as u8,
        })
    }

    fn decode_time_of_day(&self, r: &mut Reader<'a>) -> Result<Time> {
        let hour = self.get_range(0, 24, r)?;
        let minute = self.get_range(0, 59, r)?;
        let second = self.get_range(0, 60, r)?;

        Ok(Time {
            hour: hour as u8,
            minute: minute as u8,
            second: second as u8,
            microsecond: 0,
            offset: None,
        })
    }

    /// A time element of DURATION-INTERVAL-ENCODING, of which the root
    /// is `min` to `max`, and of which only the values above it are in
    /// the extension (X.691 32.6.1, 13.1).
    fn get_duration_element(
        &self,
        min: i128,
        max: i128,
        r: &mut Reader<'a>,
    ) -> Result<Value<'static>> {
        if !r.bit()? {
            return self.get_range(min, max, r).map(Value::Integer);
        }

        r.align();

        match r.unconstrained_whole_number()? {
            Value::Integer(value) if value > max => Ok(Value::Integer(value)),
            #[cfg(feature = "bigint")]
            Value::BigInteger(bytes) if bytes[0] & 0x80 == 0 => Ok(Value::BigInteger(bytes)),
            #[cfg(not(feature = "bigint"))]
            Value::BigInteger(bytes) if bytes[0] & 0x80 == 0 => {
                Err(Error::unsupported(time::ELEMENT_TOO_LARGE))
            }
            _ => decode_error(
                "Expected a value above the root in the extension.",
                r.position,
            ),
        }
    }

    fn decode_duration(&self, r: &mut Reader<'a>) -> Result<String> {
        let has_years = r.bit()?;
        let has_months = r.bit()?;
        let has_weeks = r.bit()?;
        let has_days = r.bit()?;
        let has_hours = r.bit()?;
        let has_minutes = r.bit()?;
        let has_seconds = r.bit()?;
        let has_fractional_part = r.bit()?;

        let mut c = time::DurationComponents::default();

        for (present, element, max) in [
            (has_years, &mut c.years, 31),
            (has_months, &mut c.months, 15),
            (has_weeks, &mut c.weeks, 63),
            (has_days, &mut c.days, 31),
            (has_hours, &mut c.hours, 31),
            (has_minutes, &mut c.minutes, 63),
            (has_seconds, &mut c.seconds, 63),
        ] {
            if present {
                *element = Some(self.get_duration_element(0, max, r)?);
            }
        }
        if has_fractional_part {
            let nod = self.get_duration_element(1, 3, r)?;
            let fv = self.get_duration_element(0, 999, r)?;
            let field = |value| {
                time::fraction_field(value).map_err(|message| Error::decode(message, r.position))
            };

            c.fractional_part = Some((field(nod)?, field(fv)?));
        }

        time::components_to_duration(&c).map_err(|message| Error::decode(message, r.position))
    }

    fn decode_members(
        &self,
        members: &'a Members,
        is_set: bool,
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<Value<'a>> {
        let has_additions = members.additions.is_some() && r.bit()?;
        let mut values = if is_set {
            let root = members.order.iter().map(|index| &members.root[*index]);

            self.decode_root(root, &[], r, depth)?
        } else {
            self.decode_root(members.root.iter(), &[], r, depth)?
        };

        if has_additions {
            let additions = members.additions.as_deref().unwrap_or(&[]);

            self.decode_additions(additions, &mut values, r, depth)?;
        }

        Ok(Value::Map(values))
    }

    /// The members of `root`, after the members `before` of the same
    /// SEQUENCE or SET, which an ANY DEFINED BY may be defined by.
    fn decode_root<I>(
        &self,
        root: I,
        before: &[(Cow<'_, str>, Value<'_>)],
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<Vec<(Cow<'a, str>, Value<'a>)>>
    where
        I: ExactSizeIterator<Item = &'a MemberInfo> + Clone,
    {
        let mut values = Vec::with_capacity(root.len());
        let mut next = r.presence(
            root.clone()
                .filter(|member| member.optional || member.has_default())
                .count(),
        )?;

        for member in root {
            let is_present = if member.optional || member.has_default() {
                next += 1;

                r.bit_at(next - 1)
            } else {
                true
            };

            if is_present {
                let value = self.decode_member(member, &values, before, r, depth)?;

                values.push((Cow::Borrowed(member.name.as_str()), value));
            } else if let Some(default) = &member.default {
                values.push((Cow::Borrowed(member.name.as_str()), default.reborrow()));
            }
        }

        Ok(values)
    }

    /// The value of a member, which is that of an open type if it is an ANY
    /// DEFINED BY with choices, of the type the member it names among
    /// `values` or `before` tells.
    fn decode_member(
        &self,
        member: &MemberInfo,
        values: &[(Cow<'_, str>, Value<'_>)],
        before: &[(Cow<'_, str>, Value<'_>)],
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<Value<'a>> {
        match self
            .types
            .defined_by(member.ty, values.iter().chain(before))
        {
            Some(Some(ty)) => self.decode_open_type(ty, r, depth),
            Some(None) => decode_error("Bad AnyDefinedBy choice.", r.position),
            None => self.decode_type(member.ty, r, depth + 1),
        }
        .map_err(|error| error.in_member(&member.name))
    }

    fn decode_additions(
        &self,
        additions: &'a [Addition],
        values: &mut Vec<(Cow<'a, str>, Value<'a>)>,
        r: &mut Reader<'a>,
        depth: usize,
    ) -> Result<()> {
        let length = r.normally_small_length()?;
        let presence = r.uint(length)?;

        r.align();

        for index in 0..length {
            if presence & (1 << (length - index - 1)) == 0 {
                continue;
            }

            match additions.get(index) {
                Some(Addition::Group(group)) => {
                    let decoded = self.get_open_type(r, Open::Group(group), values, depth)?;

                    if let Value::Map(decoded) = decoded {
                        values.extend(decoded);
                    }
                }
                Some(Addition::Member(member)) => {
                    let value = self.get_open_type(r, Open::Member(member), values, depth)?;

                    values.push((Cow::Borrowed(member.name.as_str()), value));
                }
                // One of a later version.
                None => {
                    self.decode_unbound_octets(r)?;
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Bytes with every pattern of bits in them.
    const DATA: [u8; 8] = [0xa5, 0x3c, 0xff, 0x00, 0x81, 0x7e, 0x96, 0x69];

    fn bit_of(data: &[u8], index: usize) -> bool {
        data[index / 8] & (0x80 >> (index % 8)) != 0
    }

    #[test]
    fn bits_are_written_as_one_at_a_time() {
        for offset in 0..16 {
            for count in 0..=40 {
                let mut writer = Writer::new(false);

                writer.uint(0b101, offset);
                writer.bits_of(&DATA, count);
                writer.uint(0x1234_5678, count);
                writer.align_always();

                let mut expected = Writer::new(false);

                for index in (0..offset).rev() {
                    expected.bit(index < 128 && (0b101u128 >> index) & 1 == 1);
                }

                for index in 0..count {
                    expected.bit(bit_of(&DATA, index));
                }

                for index in (0..count).rev() {
                    expected.bit(index < 128 && (0x1234_5678u128 >> index) & 1 == 1);
                }

                while !expected.bits.is_multiple_of(8) {
                    expected.bit(false);
                }

                assert_eq!((writer.bytes, writer.bits), (expected.bytes, expected.bits));
            }
        }
    }

    #[test]
    fn bits_are_read_as_one_at_a_time() {
        for offset in 0..16 {
            for count in 0..=(8 * DATA.len() - offset) {
                let mut expected = alloc::vec![0u8; count.div_ceil(8)];

                for index in 0..count {
                    if bit_of(&DATA, offset + index) {
                        expected[index / 8] |= 0x80 >> (index % 8);
                    }
                }

                let mut r = reader_at(offset);

                assert_eq!(r.bits(count), Ok(expected.clone()));
                assert_eq!(r.position, offset + count);

                if count <= 128 {
                    let value = (0..count).fold(0u128, |value, index| {
                        (value << 1) | u128::from(bit_of(&expected, index))
                    });

                    assert_eq!(reader_at(offset).uint(count), Ok(value));
                }
            }

            let mut r = reader_at(offset);

            assert!(r.bits(8 * DATA.len() - offset + 1).is_err());
        }
    }

    fn reader_at(position: usize) -> Reader<'static> {
        Reader {
            data: &DATA,
            position,
            aligned: false,
            budget: usize::MAX,
        }
    }
}
