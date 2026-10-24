// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! UPER for generated code: a writer into a buffer that is given, and
//! what the interpreter in `crate::per` reads, into views. Neither
//! allocates.
//!
//! The numbers, the sizes and the strings are as the interpreter has
//! them, of which these are the cases with no `Value`.

pub use crate::per::{
    Alphabet, BitSink, Characters, Fragmenter, Known, Reader, bits_of, bits_of_power_of_two,
};
pub use crate::types::Range;

use super::views::{BitString, Integer, Octets, Oid, Text, Width};
use crate::per::{check_size, extended, is_unbound, read_extended, size_bounds};
use crate::types::StrKind;
use crate::value::DateTime;
use crate::{Error, Result, time};

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

fn decode_error<T>(what: &'static str, position: usize) -> Result<T> {
    Err(Error::decode(what, position))
}

/// Writes bits into a buffer. What does not fit is counted, so that
/// [`Writer::finish`] tells how large the buffer has to be, and a writer
/// of an empty buffer counts the bits of what it is given.
pub struct Writer<'b> {
    buf: &'b mut [u8],
    bits: usize,
    overflow: bool,
}

impl<'b> Writer<'b> {
    pub fn new(buf: &'b mut [u8]) -> Writer<'b> {
        Writer {
            buf,
            bits: 0,
            overflow: false,
        }
    }

    /// How many bytes are written, or an error that tells how many there
    /// would be if they did not all fit.
    pub fn finish(self) -> Result<usize> {
        if self.overflow {
            Err(Error::BufferTooSmall(self.bits.div_ceil(8)))
        } else {
            Ok(self.bits.div_ceil(8))
        }
    }

    /// The `count` low bits of `chunk`, at most 8.
    fn chunk(&mut self, chunk: u8, count: usize) {
        let chunk = chunk & (0xff >> (8 - count));
        let used = self.bits % 8;
        let at = self.bits / 8;
        let free = 8 - used;

        self.bits += count;

        if self.overflow || self.bits.div_ceil(8) > self.buf.len() {
            self.overflow = true;

            return;
        }

        if used == 0 {
            self.buf[at] = 0;
        }

        if count <= free {
            self.buf[at] |= chunk << (free - count);
        } else {
            self.buf[at] |= chunk >> (count - free);
            self.buf[at + 1] = chunk << (8 - (count - free));
        }
    }

    /// Sets the bit `index`, which was written as 0.
    pub fn set_bit(&mut self, index: usize) {
        if !self.overflow {
            self.buf[index / 8] |= 0x80 >> (index % 8);
        }
    }

    /// An open type of what `value` writes: its complete encoding, which
    /// is one byte 0 if it is empty (X.691 11.1), as an unconstrained
    /// OCTET STRING (11.2). It is written once to count its bits.
    pub fn open_type(&mut self, value: impl Fn(&mut Writer<'_>) -> Result<()>) -> Result<()> {
        let mut counter = Writer::new(&mut []);

        value(&mut counter)?;

        let bits = counter.bits;
        let length = bits.div_ceil(8).max(1);

        if length >= 16384 {
            return Err(Error::unsupported(
                "An open type of 16384 bytes or more is not supported by generated code.",
            ));
        }

        self.align();
        self.length(length);

        let start = self.bits;

        value(self)?;

        // The padding to the bytes, or the byte of nothing.
        let written = self.bits - start;

        self.uint(0, 8 * length - written);

        Ok(())
    }
}

impl BitSink for Writer<'_> {
    fn bit(&mut self, bit: bool) {
        self.chunk(u8::from(bit), 1);
    }

    fn uint(&mut self, value: u128, mut count: usize) {
        while count > 128 {
            self.chunk(0, 8.min(count - 128));
            count -= 8.min(count - 128);
        }

        while count > 0 {
            let take = count.min(8 - self.bits % 8);

            count -= take;
            self.chunk((value >> count) as u8, take);
        }
    }

    fn bytes(&mut self, data: &[u8]) {
        if self.bits.is_multiple_of(8) && !self.overflow {
            let at = self.bits / 8;

            self.bits += 8 * data.len();

            match self.buf.get_mut(at..at + data.len()) {
                Some(slot) => slot.copy_from_slice(data),
                None => self.overflow = true,
            }

            return;
        }

        for byte in data {
            self.chunk(*byte, 8);
        }
    }

    fn bits_of(&mut self, data: &[u8], count: usize) {
        self.bytes(&data[..count / 8]);

        if !count.is_multiple_of(8) {
            self.chunk(data[count / 8] >> (8 - count % 8), count % 8);
        }
    }

    fn align_always(&mut self) {
        let pad = (8 - self.bits % 8) % 8;

        if pad > 0 {
            self.chunk(0, pad);
        }
    }

    fn aligned(&self) -> bool {
        false
    }

    fn position(&self) -> usize {
        self.bits
    }
}

// Encoding.

/// An INTEGER with the bounds of `range` (X.691 13).
pub fn integer(w: &mut impl BitSink, value: i128, range: Option<Range>) -> Result<()> {
    let lower = range.and_then(|range| range.min);
    let upper = range.and_then(|range| range.max);
    let inside =
        lower.is_none_or(|lower| value >= lower) && upper.is_none_or(|upper| value <= upper);

    if range.is_some_and(|range| range.extensible) {
        w.bit(!inside);

        // Outside the root, as if there was no constraint (X.691 13.1).
        if !inside {
            w.align();

            return unconstrained(w, value);
        }
    } else if !inside {
        return encode_error("The value is not in the range.");
    }

    let (Some(min), Some(max)) = (lower, upper) else {
        w.align();

        return match lower {
            // A semi-constrained whole number: the value less the lower
            // bound (X.691 13.2.3, 11.7).
            Some(min) => {
                let offset = (value as u128).wrapping_sub(min as u128);
                let bytes = offset.to_be_bytes();
                let skip = bytes.iter().take_while(|byte| **byte == 0).count().min(15);

                w.unconstrained_whole_number(&bytes[skip..])
            }
            // An unconstrained whole number (13.2.4, 11.8).
            None => unconstrained(w, value),
        };
    };
    put_bounded(w, value, min, max);

    Ok(())
}

/// An INTEGER from `min` to `max`, a constrained whole number of X.691
/// 13.2.2, which aligned PER writes after its length if the range is
/// more than 65536 values (11.5.7.4).
pub(crate) fn put_bounded(w: &mut impl BitSink, value: i128, min: i128, max: i128) {
    let size = (max as u128).wrapping_sub(min as u128);
    let count = bits_of(size);

    if !w.aligned() {
        w.uint((value as u128).wrapping_sub(min as u128), count);

        return;
    }

    let count = if size <= 65535 {
        count
    } else {
        let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
        let length = crate::per::bytes_of((value as u128).wrapping_sub(min as u128));

        w.constrained_whole_number(length as i128 - 1, 0, 1i128 << indefinite, indefinite);
        w.align();

        8 * length
    };

    w.constrained_whole_number(value, min, max, count);
}

/// An INTEGER of any size with the bounds of `range`, which fit in an
/// `i128`: one that does not is out of them, or after the lower bound.
pub fn big_integer(w: &mut impl BitSink, value: &Integer<'_>, range: Option<Range>) -> Result<()> {
    if let Some(small) = value.to_i128() {
        return integer(w, small, range);
    }

    let lower = range.and_then(|range| range.min);
    let upper = range.and_then(|range| range.max);
    let negative = value.is_negative();
    let inside = (lower.is_none() || !negative) && (upper.is_none() || negative);
    let unconstrained = |w: &mut _| put_counted(w, value.bytes(), value.len());

    if range.is_some_and(|range| range.extensible) {
        w.bit(!inside);

        if !inside {
            w.align();

            return unconstrained(w);
        }
    } else if !inside {
        return encode_error("The value is not in the range.");
    }

    w.align();

    match lower {
        // A semi-constrained whole number of a value beyond an `i128`: of
        // the bound 0, its magnitude.
        Some(0) => {
            let skip = usize::from(value.bytes().next() == Some(0));

            put_counted(w, value.bytes().skip(skip), value.len() - skip)
        }
        Some(_) => Err(Error::unsupported(
            "An INTEGER beyond 128 bits with a lower bound that is not 0 is not supported by generated code.",
        )),
        None => unconstrained(w),
    }
}

/// `count` bytes after their number, as an unconstrained whole number is.
fn put_counted(w: &mut impl BitSink, bytes: impl Iterator<Item = u8>, count: usize) -> Result<()> {
    if count >= 16384 {
        return encode_error("An INTEGER is too large.");
    }

    w.length(count);
    bytes.for_each(|byte| w.uint(u128::from(byte), 8));

    Ok(())
}

/// The two's complement of `value` in as few bytes as it can be, after
/// their number.
fn unconstrained(w: &mut impl BitSink, value: i128) -> Result<()> {
    let bytes = value.to_be_bytes();
    let mut start = 0;

    while start < 15
        && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }

    w.unconstrained_whole_number(&bytes[start..])
}

/// A number from `min` to `max`, as a constrained whole number (X.691
/// 11.5).
pub fn range(w: &mut impl BitSink, value: i128, min: i128, max: i128) {
    let count = bits_of((max - min) as u128);

    if w.aligned() {
        w.constrained_whole_number(value, min, max, count);
    } else {
        w.uint((value - min) as u128, count);
    }
}

/// The value of an ENUMERATED (X.691 14): its index among the values of
/// the root in the order of their numbers, or among the additions.
pub fn enumerated(
    w: &mut impl BitSink,
    root: usize,
    extensible: bool,
    index: core::result::Result<usize, usize>,
) {
    match index {
        Ok(index) => {
            if extensible {
                w.bit(false);
            }

            w.uint(index as u128, bits_of(root.saturating_sub(1) as u128));
        }
        Err(addition) => {
            w.bit(true);
            w.normally_small_whole_number(addition as u128);
        }
    }
}

/// The index of an alternative of a CHOICE of the root (X.691 23.6), of
/// `root` of them.
pub fn choice_index(w: &mut impl BitSink, root: usize, index: usize) {
    if root > 1 {
        let maximum = root - 1;
        let count = bits_of(maximum as u128);

        if !w.aligned() {
            w.uint(index as u128, count);
        } else if maximum <= 65535 {
            w.constrained_whole_number(index as i128, 0, maximum as i128, count);
        } else {
            let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
            let length = crate::per::bytes_of(index as u128);

            w.constrained_whole_number(length as i128 - 1, 0, 1i128 << indefinite, indefinite);
            w.align();
            w.constrained_whole_number(index as i128, 0, maximum as i128, 8 * length);
        }
    }
}

/// The length of a SEQUENCE OF of `length` elements with the size
/// `size`, after which the elements are written, if there are fewer
/// than 16384. Returns `false` if they are to be written in fragments,
/// see [`Fragmenter`].
pub fn count(w: &mut impl BitSink, length: usize, size: Option<Range>) -> Result<bool> {
    let size = extended(size, length as i128, w);

    check_size(size, length as i128)?;

    if is_unbound(size) {
        w.align();

        return Ok(length < 16384 && {
            w.length(length);

            true
        });
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));

    if min != max {
        let count = bits_of((max - min) as u128);

        if w.aligned() {
            w.constrained_whole_number(length as i128, min, max, count);
        } else {
            w.uint((length as i128 - min) as u128, count);
        }
    }

    Ok(true)
}

pub fn octet_string(w: &mut impl BitSink, data: &Octets<'_>, size: Option<Range>) -> Result<()> {
    let length = data.len();
    let size = extended(size, length as i128, w);

    check_size(size, length as i128)?;

    let bounds = size_bounds(size).filter(|_| !is_unbound(size));

    match bounds {
        Some((min, max)) if !(min == max && max >= 65536) => {
            let span = (max - min) as u128;
            let count = bits_of(span);
            let mut align = true;

            if min != max {
                if w.aligned() {
                    w.constrained_whole_number(length as i128, min, max, count);
                } else {
                    w.uint((length as i128 - min) as u128, count);
                }

                // No padding for nothing (X.691 11.9.3.3).
                align = length > 0;
            } else if max <= 2 {
                align = false;
            }

            if align {
                w.align();
            }

            put_octets(w, data, 0, length);
        }
        _ => {
            w.align();

            let mut fragments = Fragmenter::new(length);

            while let Some((offset, part)) = fragments.next(w) {
                w.align();
                put_octets(w, data, offset, part);
            }
        }
    }

    Ok(())
}

fn put_octets(w: &mut impl BitSink, data: &Octets<'_>, offset: usize, count: usize) {
    match data.as_slice() {
        Some(bytes) => w.bytes(&bytes[offset..offset + count]),
        None => data
            .bytes()
            .skip(offset)
            .take(count)
            .for_each(|byte| w.uint(u128::from(byte), 8)),
    }
}

/// A BIT STRING, of which the trailing zero bits are left out if it has
/// `named` bits, but for the least size (X.691 16.2, 16.3).
pub fn bit_string(
    w: &mut impl BitSink,
    data: &BitString<'_>,
    named: bool,
    size: Option<Range>,
) -> Result<()> {
    let bits = if named {
        let trimmed = data.trimmed_len();

        match size.and_then(|size| size.min) {
            Some(min) if (trimmed as i128) < min => min as usize,
            _ => trimmed,
        }
    } else {
        data.len()
    };
    let size = extended(size, bits as i128, w);

    check_size(size, bits as i128)?;

    // The bits, and zero bits after them if there are to be more.
    let put = |w: &mut _, offset: usize, count: usize| put_bits(w, data, offset, count);

    if is_unbound(size) {
        w.align();

        let mut fragments = Fragmenter::new(bits);

        while let Some((offset, length)) = fragments.next(w) {
            put(w, offset, length);
        }

        return Ok(());
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));
    let count = bits_of((max - min) as u128);

    if min != max {
        if w.aligned() {
            w.constrained_whole_number(bits as i128, min, max, count);

            if bits > 0 {
                w.align();
            }
        } else {
            w.uint((bits as i128 - min) as u128, count);
        }
    } else if w.aligned() && min > 16 {
        w.align();
    }

    put(w, 0, bits);

    Ok(())
}

/// `count` bits of `data` from the bit `offset`, which is a multiple of
/// 8, and zero bits past its end.
fn put_bits(w: &mut impl BitSink, data: &BitString<'_>, offset: usize, count: usize) {
    // Those there are, all at once, and zero bits after them.
    if let Some(bytes) = data.as_slice() {
        let there = data.len().saturating_sub(offset).min(count);

        w.bits_of(&bytes[offset / 8..], there);
        w.uint(0, count - there);

        return;
    }

    let mut left = count;

    for byte in data.bytes().skip(offset / 8) {
        if left == 0 {
            return;
        }

        let take = left.min(8);

        w.uint(u128::from(byte >> (8 - take)), take);
        left -= take;
    }

    w.uint(0, left);
}

/// A string of a known multiplier (X.691 30.5): NumericString,
/// PrintableString, IA5String, VisibleString, BMPString and
/// UniversalString, of the characters of `known`.
pub fn known_string(
    w: &mut impl BitSink,
    text: &Text<'_>,
    size: Option<Range>,
    known: &Known,
) -> Result<()> {
    let length = text.char_count();
    // Outside the root of an extensible size, with the permitted
    // alphabet still (X.691 30.4).
    let size = extended(size, length as i128, w);

    check_size(size, length as i128)?;

    let bits = known.bits_per_character;
    let put = |w: &mut _, offset: usize, count: usize| -> Result<()> {
        for character in text.chars().skip(offset).take(count) {
            BitSink::uint(
                w,
                u128::from(known.alphabet.encode(u32::from(character))?),
                bits,
            );
        }

        Ok(())
    };

    if is_unbound(size) {
        w.align();

        let mut fragments = Fragmenter::new(length);

        while let Some((offset, count)) = fragments.next(w) {
            put(w, offset, count)?;
        }

        return Ok(());
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));

    if min != max {
        let count = bits_of((max - min) as u128);

        if w.aligned() {
            w.constrained_whole_number(length as i128, min, max, count);

            if max * bits as i128 >= 16 && length > 0 {
                w.align();
            }
        } else {
            w.uint((length as i128 - min) as u128, count);
        }
    } else if w.aligned() && max * bits as i128 > 16 {
        w.align();
    }

    put(w, 0, length)
}

/// A string of no known multiplier, as UTF8String, which is its length
/// and its bytes, of UTF-8 or of a byte for each character, in
/// fragments (X.691 30.6).
pub fn other_string(w: &mut impl BitSink, text: &Text<'_>, kind: StrKind) -> Result<()> {
    let utf8 = kind == StrKind::Utf8;
    // The length is of characters, but of bytes for UTF8String.
    let count = if utf8 {
        text.utf8_len()
    } else {
        text.char_count()
    };

    if !utf8 && text.chars().any(|character| u32::from(character) > 0xff) {
        return encode_error("A character is not Latin-1.");
    }

    w.align();

    let mut fragments = Fragmenter::new(count);

    while let Some((offset, length)) = fragments.next(w) {
        if utf8 {
            match text.as_str() {
                Some(text) => w.bytes(&text.as_bytes()[offset..offset + length]),
                None => {
                    let mut buffer = [0; 4];

                    text.chars()
                        .flat_map(|character| {
                            let encoded = character.encode_utf8(&mut buffer);
                            let mut bytes = [0u8; 4];
                            let used = encoded.len();

                            bytes[..used].copy_from_slice(encoded.as_bytes());
                            bytes.into_iter().take(used)
                        })
                        .skip(offset)
                        .take(length)
                        .for_each(|byte| w.uint(u128::from(byte), 8));
                }
            }
        } else {
            text.chars()
                .skip(offset)
                .take(length)
                .for_each(|character| w.uint(u128::from(u32::from(character)), 8));
        }
    }

    Ok(())
}

/// An OBJECT IDENTIFIER: the length of its contents octets and those
/// (X.691 24), its subidentifiers in as few bytes as they can be.
pub fn oid(w: &mut impl BitSink, value: &Oid<'_>) {
    let groups = |subidentifier: u128| {
        (128 - subidentifier.leading_zeros() as usize)
            .div_ceil(7)
            .max(1)
    };

    w.align();

    if value.is_minimal() {
        let octets = value.octets();

        w.length(octets.len());

        return put_octets(w, &octets, 0, octets.len());
    }

    w.length(value.subidentifiers().map(groups).sum());

    for subidentifier in value.subidentifiers() {
        for group in (0..groups(subidentifier)).rev() {
            let bits = (subidentifier >> (7 * group)) as u8 & 0x7f;

            w.uint(u128::from(if group == 0 { bits } else { 0x80 | bits }), 8);
        }
    }
}

/// A UTCTime, which is a VisibleString of the form DER has (X.691 30.5,
/// 31).
pub fn utc_time(w: &mut impl BitSink, value: &DateTime) -> Result<()> {
    let mut text = time::TimeText::new();

    time::write_utc_time(value, true, &mut text);
    known_string(w, &Text::new(text.as_str()), None, &VISIBLE)
}

/// A GeneralizedTime, which is a VisibleString of the form DER has.
pub fn generalized_time(w: &mut impl BitSink, value: &DateTime) -> Result<()> {
    let mut text = time::TimeText::new();

    time::write_generalized_time(value, true, &mut text);
    known_string(w, &Text::new(text.as_str()), None, &VISIBLE)
}

/// The characters of a VisibleString with no permitted alphabet.
pub static VISIBLE: Known = Known {
    alphabet: Alphabet {
        characters: Characters::Visible,
        by_index: false,
    },
    bits_per_character: 7,
};

// Decoding.

/// An INTEGER with the bounds of `range`, which has to fit in an `i128`.
pub fn read_integer(r: &mut Reader<'_>, range: Option<Range>) -> Result<i128> {
    if range.is_some_and(|range| range.extensible) && r.bit()? {
        r.align();

        return read_unconstrained(r);
    }

    let lower = range.and_then(|range| range.min);
    let upper = range.and_then(|range| range.max);
    let (Some(min), Some(max)) = (lower, upper) else {
        r.align();

        return match lower {
            Some(min) => {
                let length = r.length()?;

                if length > 16 {
                    return Err(Error::unsupported("An INTEGER is too large."));
                }

                min.checked_add_unsigned(r.uint(8 * length)?)
                    .ok_or_else(|| Error::unsupported("An INTEGER is too large."))
            }
            None => read_unconstrained(r),
        };
    };
    read_bounded(r, min, max)
}

/// An INTEGER from `min` to `max`, see `put_bounded`.
pub(crate) fn read_bounded(r: &mut Reader<'_>, min: i128, max: i128) -> Result<i128> {
    let size = (max as u128).wrapping_sub(min as u128);
    let count = bits_of(size);

    if !r.aligned {
        return Ok((r.uint(count)? as i128).wrapping_add(min));
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
}

/// An INTEGER of any size with the bounds of `range`, which borrows what
/// does not fit in an `i128`.
pub fn read_big_integer<'a>(r: &mut Reader<'a>, range: Option<Range>) -> Result<Integer<'a>> {
    let lower = range.and_then(|range| range.min);
    let upper = range.and_then(|range| range.max);

    // Within bounds that fit in an `i128`, or outside them.
    if lower.is_some() && upper.is_some() {
        let at = r.position;

        if !(range.is_some_and(|range| range.extensible) && r.bit()?) {
            r.position = at;

            return read_integer(r, range).map(Integer::new);
        }
    }

    // In the extension, as if there was no constraint (X.691 13.1).
    let extended = range.is_some_and(|range| range.extensible)
        && (lower.is_some() && upper.is_some() || r.bit()?);

    r.align();

    let length = r.length()?;
    let at = r.position;

    r.skip(8 * length)?;

    let bytes = Octets::per(r.data, at, length, false);

    match lower {
        Some(min) if !extended => {
            let offset = Integer::from_octets(bytes, true);

            match (min, offset.to_i128()) {
                (0, _) => Ok(offset),
                (_, Some(offset)) => min
                    .checked_add(offset)
                    .map(Integer::new)
                    .ok_or_else(|| Error::unsupported("An INTEGER is too large.")),
                _ => Err(Error::unsupported(
                    "An INTEGER beyond 128 bits with a lower bound that is not 0 is not supported by generated code.",
                )),
            }
        }
        _ => Ok(Integer::from_octets(bytes, false)),
    }
}

fn read_unconstrained(r: &mut Reader<'_>) -> Result<i128> {
    let length = r.length()?;

    if length > 16 {
        return Err(Error::unsupported("An INTEGER is too large."));
    }

    if length == 0 {
        return Ok(0);
    }

    let value = r.uint(8 * length)?;
    let shift = 128 - 8 * length;

    // Sign-extended.
    Ok(((value << shift) as i128) >> shift)
}

/// An integer that has to fit in the type the code has for it.
pub fn fit<T: TryFrom<i128>>(r: &Reader<'_>, value: i128) -> Result<T> {
    T::try_from(value).map_err(|_| Error::decode("The INTEGER is out of range.", r.position))
}

/// A number from `min` to `max`.
pub fn read_range(r: &mut Reader<'_>, min: i128, max: i128) -> Result<i128> {
    let count = bits_of((max - min) as u128);

    if r.aligned {
        r.constrained_whole_number(min, max, count)
    } else {
        Ok(r.uint(count)? as i128 + min)
    }
}

/// The index of the value of an ENUMERATED: of the root, or `Err` of the
/// additions.
pub fn read_enumerated(
    r: &mut Reader<'_>,
    root: usize,
    extensible: bool,
) -> Result<core::result::Result<usize, usize>> {
    if extensible && r.bit()? {
        return Ok(Err(r.normally_small_whole_number()? as usize));
    }

    let index = r.uint(bits_of(root.saturating_sub(1) as u128))? as usize;

    if index >= root {
        return decode_error("Expected an enumeration index.", r.position);
    }

    Ok(Ok(index))
}

/// The index of an alternative of the root of a CHOICE of `root` of them.
pub fn read_choice_index(r: &mut Reader<'_>, root: usize) -> Result<usize> {
    if root <= 1 {
        return Ok(0);
    }

    let maximum = root - 1;
    let count = bits_of(maximum as u128);
    let index = if !r.aligned {
        r.uint(count)? as usize
    } else if maximum <= 65535 {
        r.constrained_whole_number(0, maximum as i128, count)? as usize
    } else {
        let indefinite = bits_of((count.div_ceil(8) - 1) as u128);
        let length = r.constrained_whole_number(0, 1i128 << indefinite, indefinite)? + 1;

        r.align();

        r.constrained_whole_number(0, maximum as i128, 8 * length as usize)? as usize
    };

    if index > maximum {
        return decode_error("Expected a choice index.", r.position);
    }

    Ok(index)
}

/// The number of elements of a SEQUENCE OF, and if they are in fragments,
/// of which the reader is then at the first length.
pub fn read_count(r: &mut Reader<'_>, size: Option<Range>) -> Result<(usize, bool)> {
    let size = read_extended(size, r)?;

    if is_unbound(size) {
        r.align();

        let start = r.position;
        let first = r.length()?;

        if first < 16384 {
            r.spend(first)?;

            return Ok((first, false));
        }

        // Counts the elements of all of the fragments, which are skipped
        // by the caller as it reads them, so it reads them again.
        r.position = start;

        return Ok((0, true));
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));
    let count = bits_of((max - min) as u128);
    let length = if min == max {
        min as usize
    } else if r.aligned {
        r.constrained_whole_number(min, max, count)? as usize
    } else {
        r.uint(count)? as usize + min as usize
    };

    r.spend(length)?;

    Ok((length, false))
}

/// The bytes of an unconstrained OCTET STRING, as an open type has them,
/// in fragments if there are 16384 or more.
pub fn read_unbound_octets<'a>(r: &mut Reader<'a>) -> Result<Octets<'a>> {
    r.align();

    let start = r.position;
    let mut total = 0;

    loop {
        let length = r.length()?;

        r.align();

        let at = r.position;

        r.skip(8 * length)?;

        if total == 0 && length < 16384 {
            return Ok(Octets::per(r.data, at, length, false));
        }

        total += length;

        if length < 16384 {
            break;
        }
    }

    Ok(Octets::per(r.data, start, total, true))
}

pub fn read_octet_string<'a>(r: &mut Reader<'a>, size: Option<Range>) -> Result<Octets<'a>> {
    let size = read_extended(size, r)?;
    let Some((min, max)) = size_bounds(size).filter(|_| !is_unbound(size)) else {
        return read_unbound_octets(r);
    };
    let span = (max - min) as u128;

    if span == 0 && max >= 65536 {
        return read_unbound_octets(r);
    }

    let count = bits_of(span);
    let mut length = min;
    let mut align = true;

    if min != max {
        length = if r.aligned {
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

    let at = r.position;

    r.skip(8 * length as usize)?;

    Ok(Octets::per(r.data, at, length as usize, false))
}

pub fn read_bit_string<'a>(r: &mut Reader<'a>, size: Option<Range>) -> Result<BitString<'a>> {
    let size = read_extended(size, r)?;

    if is_unbound(size) {
        r.align();

        let at = r.position;
        let length = r.length()?;

        if length >= 16384 {
            return Err(Error::unsupported(
                "A BIT STRING of 16384 bits or more is not supported by generated code.",
            ));
        }

        let start = r.position;

        r.skip(length)?;
        let _ = at;

        return Ok(BitString::from_octets(
            Octets::per(r.data, start, length.div_ceil(8), false),
            length,
        ));
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));
    let count = bits_of((max - min) as u128);
    let mut bits = min;

    if min != max {
        bits = if r.aligned {
            r.constrained_whole_number(min, max, count)?
        } else {
            r.uint(count)? as i128 + min
        };

        if bits > 0 {
            r.align();
        }
    } else if r.aligned && min > 16 {
        r.align();
    }

    let bits = bits as usize;
    let start = r.position;

    r.skip(bits)?;

    Ok(BitString::from_octets(
        Octets::per(r.data, start, bits.div_ceil(8), false),
        bits,
    ))
}

/// A string of a known multiplier, of the characters of `known`.
pub fn read_known_string<'a: 'k, 'k>(
    r: &mut Reader<'a>,
    size: Option<Range>,
    known: &'k Known,
) -> Result<Text<'k>> {
    let (at, count, fragmented) = read_known_chars(r, size, known, |_| {})?;

    Ok(Text::packed(r.data, at, count, known, fragmented))
}

/// Reads the characters of a string of a known multiplier, which `f` is
/// given one at a time, and returns where the first is, how many there
/// are and if they are in fragments, of which the first length is then
/// where they are.
pub fn read_known_chars(
    r: &mut Reader<'_>,
    size: Option<Range>,
    known: &Known,
    mut f: impl FnMut(char),
) -> Result<(usize, usize, bool)> {
    let size = read_extended(size, r)?;
    let bits = known.bits_per_character;
    let mut read = |r: &mut Reader<'_>, count: usize| -> Result<()> {
        r.spend(count)?;

        for _ in 0..count {
            let value = r.uint(bits)? as u32;
            let code = known.alphabet.decode(value, r.position)?;

            f(char::from_u32(code).ok_or_else(|| Error::decode("Bad character.", r.position))?);
        }

        Ok(())
    };

    if is_unbound(size) {
        r.align();

        let start = r.position;
        let mut total = 0;

        loop {
            let count = r.length()?;

            read(r, count)?;
            total += count;

            if count < 16384 {
                break;
            }
        }

        let fragmented = total >= 16384;
        let at = if fragmented {
            start
        } else {
            // After the length.
            start + if total < 128 { 8 } else { 16 }
        };

        return Ok((at, total, fragmented));
    }

    let (min, max) = size_bounds(size).unwrap_or((0, 0));
    let count = bits_of((max - min) as u128);
    let length;

    if min != max {
        length = if r.aligned {
            r.constrained_whole_number(min, max, count)?
        } else {
            r.uint(count)? as i128 + min
        };

        if r.aligned && max * bits as i128 >= 16 && length > 0 {
            r.align();
        }
    } else {
        length = min;

        if r.aligned && max * bits as i128 > 16 {
            r.align();
        }
    }

    let at = r.position;

    read(r, length as usize)?;

    Ok((at, length as usize, false))
}

/// A string of no known multiplier, of `kind`.
pub fn read_other_string<'a>(r: &mut Reader<'a>, kind: StrKind) -> Result<Text<'a>> {
    let start = r.position;
    let octets = read_unbound_octets(r)?;

    match kind {
        StrKind::Utf8 => {
            let valid = match octets.as_slice() {
                Some(bytes) => core::str::from_utf8(bytes).is_ok(),
                None => super::ber::utf8_valid(octets.bytes()),
            };

            if !valid {
                return decode_error("The string is not UTF-8.", start);
            }

            Ok(Text::from_bytes(octets, Width::Utf8))
        }
        _ => Ok(Text::from_bytes(octets, Width::Latin1)),
    }
}

pub fn read_oid<'a>(r: &mut Reader<'a>) -> Result<Oid<'a>> {
    r.align();

    let length = r.length()?;
    let at = r.position;

    r.skip(8 * length)?;
    Oid::checked(Octets::per(r.data, at, length, false), false, at / 8)
}

/// The characters of a UTCTime or a GeneralizedTime, which are at most
/// as many as fit the text of one.
fn read_time_text<'t>(r: &mut Reader<'_>, buffer: &'t mut [u8; 64]) -> Result<&'t str> {
    let text = read_known_string(r, None, &VISIBLE)?;
    let mut length = 0;

    for character in text.chars() {
        let Some(slot) = buffer.get_mut(length) else {
            return decode_error("Expected a time string.", r.position);
        };

        *slot = character as u8;
        length += 1;
    }

    core::str::from_utf8(&buffer[..length]).map_err(|_| Error::decode("Bad character.", r.position))
}

pub fn read_utc_time(r: &mut Reader<'_>) -> Result<DateTime> {
    let mut buffer = [0; 64];

    time::utc_time_from_string(read_time_text(r, &mut buffer)?, true)
}

pub fn read_generalized_time(r: &mut Reader<'_>) -> Result<DateTime> {
    let mut buffer = [0; 64];

    time::generalized_time_from_string(read_time_text(r, &mut buffer)?, true)
}

/// The value of an open type, which `read` decodes from its bytes, of
/// which it has to read all but the padding of the last.
pub fn read_open_type<'a, T>(
    r: &mut Reader<'a>,
    read: impl FnOnce(&mut Reader<'a>) -> Result<T>,
) -> Result<T> {
    let octets = read_unbound_octets(r)?;
    let length = octets.len();

    if length >= 16384 {
        return Err(Error::unsupported(
            "An open type of 16384 bytes or more is not supported by generated code.",
        ));
    }

    let end = r.position;
    let start = end - 8 * length;
    // The data up to the end of the open type, so that it reads no
    // further.
    let mut inner = Reader::new(&r.data[..end.div_ceil(8)], r.aligned);

    inner.position = start;
    inner.budget = r.budget;

    let value = read(&mut inner)?;
    let used = inner.position - start;
    let empty = used == 0 && length == 1 && octets.bytes().next() == Some(0);

    r.budget = inner.budget;

    if used.div_ceil(8) != length && !empty {
        return decode_error("The value does not fill its open type.", end);
    }

    Ok(value)
}
