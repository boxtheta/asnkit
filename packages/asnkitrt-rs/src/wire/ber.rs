// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! BER and DER for generated code: a writer into a buffer that is given
//! and a reader of what it decodes into views, neither of which
//! allocates.

use core::cmp::Ordering;

use super::views::{BitString, Integer, Octets, Oid, Text, Width};
use crate::ber::{read_length, read_tag, tag_bytes};
use crate::node::TagClass;
use crate::types::StrKind;
use crate::value::DateTime;
use crate::{Error, Result, time};

const MAX_DEPTH: usize = 128;

fn decode_error<T>(what: &'static str, offset: usize) -> Result<T> {
    Err(Error::decode(what, offset))
}

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

/// Writes BER into a buffer. What does not fit is counted, so that
/// [`Writer::finish`] tells how large the buffer has to be.
pub struct Writer<'b> {
    buf: &'b mut [u8],
    len: usize,
    overflow: bool,
}

impl<'b> Writer<'b> {
    pub fn new(buf: &'b mut [u8]) -> Writer<'b> {
        Writer {
            buf,
            len: 0,
            overflow: false,
        }
    }

    /// How many bytes are written, also those that did not fit.
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// How many bytes are written, or an error that tells how many there
    /// would be if they did not all fit.
    pub fn finish(self) -> Result<usize> {
        if self.overflow {
            Err(Error::BufferTooSmall(self.len))
        } else {
            Ok(self.len)
        }
    }

    pub fn put(&mut self, data: &[u8]) {
        let end = self.len + data.len();

        if !self.overflow && end <= self.buf.len() {
            self.buf[self.len..end].copy_from_slice(data);
        } else {
            self.overflow = true;
        }

        self.len = end;
    }

    pub fn byte(&mut self, byte: u8) {
        self.put(&[byte]);
    }

    pub fn tag(&mut self, class: TagClass, constructed: bool, number: u64) {
        let (bytes, count) = tag_bytes(class, constructed, number);

        self.put(&bytes[..count]);
    }

    /// Starts a TLV: its tag, and a length that [`Writer::close`] writes
    /// once the contents after it are written. Returns where they start.
    pub fn open(&mut self, class: TagClass, constructed: bool, number: u64) -> usize {
        self.tag(class, constructed, number);
        self.byte(0);

        self.len
    }

    /// Ends the TLV of which the contents start at `start`, writing its
    /// length in as few bytes as it can be, before them.
    pub fn close(&mut self, start: usize) {
        let length = self.len - start;

        if length <= 127 {
            if !self.overflow {
                self.buf[start - 1] = length as u8;
            }

            return;
        }

        let bytes = length.to_be_bytes();
        let skip = bytes.iter().take_while(|byte| **byte == 0).count();
        let extra = bytes.len() - skip;
        let end = self.len + extra;

        if !self.overflow && end <= self.buf.len() {
            self.buf.copy_within(start..self.len, start + extra);
            self.buf[start - 1] = 0x80 | extra as u8;
            self.buf[start..start + extra].copy_from_slice(&bytes[skip..]);
        } else {
            self.overflow = true;
        }

        self.len = end;
    }

    /// The contents of an INTEGER.
    pub fn integer(&mut self, value: i128) {
        let bytes = value.to_be_bytes();

        self.put(&bytes[minimal_start(&bytes)..]);
    }

    /// The contents of an INTEGER of any size.
    pub fn big_integer(&mut self, value: &Integer<'_>) {
        match value.to_i128() {
            Some(small) => self.integer(small),
            None => value.bytes().for_each(|byte| self.byte(byte)),
        }
    }

    /// The contents of a BOOLEAN.
    pub fn boolean(&mut self, value: bool) {
        self.byte(if value { 0xff } else { 0x00 });
    }

    pub fn octets(&mut self, value: &Octets<'_>) {
        match value.as_slice() {
            Some(bytes) => self.put(bytes),
            None => value.bytes().for_each(|byte| self.byte(byte)),
        }
    }

    /// The contents of a BIT STRING, of which a trailing zero bit is left
    /// out if it has `named` bits, in DER (X.690 11.2.2).
    pub fn bit_string(&mut self, value: &BitString<'_>, named: bool, der: bool) {
        let bits = if der && named {
            value.trimmed_len()
        } else {
            value.len()
        };

        self.byte(((8 - bits % 8) % 8) as u8);
        value
            .bytes()
            .take(bits.div_ceil(8))
            .enumerate()
            .for_each(|(index, byte)| {
                // The bits after the last, as there may be fewer.
                if index + 1 == bits.div_ceil(8) && bits % 8 != 0 {
                    self.byte(byte & !(0xffu8 >> (bits % 8)));
                } else {
                    self.byte(byte);
                }
            });
    }

    /// The contents of an OBJECT IDENTIFIER, its subidentifiers in as few
    /// bytes as they can be.
    pub fn oid(&mut self, value: &Oid<'_>) {
        if value.is_minimal() {
            return self.octets(&value.octets());
        }

        for subidentifier in value.subidentifiers() {
            let groups = (128 - subidentifier.leading_zeros() as usize)
                .div_ceil(7)
                .max(1);

            for group in (0..groups).rev() {
                let bits = (subidentifier >> (7 * group)) as u8 & 0x7f;

                self.byte(if group == 0 { bits } else { 0x80 | bits });
            }
        }
    }

    /// The contents of a character string of `kind`.
    pub fn text(&mut self, value: &Text<'_>, kind: StrKind) -> Result<()> {
        if let (Some(text), StrKind::Utf8) = (value.as_str(), kind) {
            self.put(text.as_bytes());

            return Ok(());
        }

        for character in value.chars() {
            let code = u32::from(character);

            match kind {
                StrKind::Utf8 => {
                    let mut buffer = [0; 4];

                    self.put(character.encode_utf8(&mut buffer).as_bytes());
                }
                StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
                    if code > 0x7f {
                        return encode_error("A character is not ASCII.");
                    }

                    self.byte(code as u8);
                }
                StrKind::Bmp => match u16::try_from(code) {
                    Ok(unit) => self.put(&unit.to_be_bytes()),
                    Err(_) => return encode_error("A character is not in the BMP."),
                },
                StrKind::Universal => self.put(&code.to_be_bytes()),
                _ => match u8::try_from(code) {
                    Ok(byte) => self.byte(byte),
                    Err(_) => return encode_error("A character is not Latin-1."),
                },
            }
        }

        Ok(())
    }

    /// The contents of a UTCTime, in the form of DER with `der`.
    pub fn utc_time(&mut self, value: &DateTime, der: bool) {
        let mut text = time::TimeText::new();

        time::write_utc_time(value, der, &mut text);
        self.put(text.as_bytes());
    }

    /// The contents of a GeneralizedTime, in the form of DER with `der`.
    pub fn generalized_time(&mut self, value: &DateTime, der: bool) {
        let mut text = time::TimeText::new();

        time::write_generalized_time(value, der, &mut text);
        self.put(text.as_bytes());
    }

    /// Sorts the TLVs from `start` to what is written as DER has the
    /// elements of a SET OF (X.690 11.6), in place.
    pub fn sort_elements(&mut self, start: usize) {
        self.sort(start, padded_compare);
    }

    /// Sorts the TLVs from `start` to what is written in the order of
    /// their tags, as DER has the members of a SET (X.690 10.3).
    pub fn sort_members(&mut self, start: usize) {
        self.sort(start, |a, b| {
            let tag = |tlv: &[u8]| {
                read_tag(tlv, 0).map_or((TagClass::Private, u64::MAX), |(class, _, number, _)| {
                    (class, number)
                })
            };

            tag(a).cmp(&tag(b))
        });
    }

    /// An insertion sort of the TLVs, which moves them with rotations so
    /// that it needs no memory of its own.
    fn sort(&mut self, start: usize, compare: impl Fn(&[u8], &[u8]) -> Ordering) {
        if self.overflow {
            return;
        }

        let region = &mut self.buf[start..self.len];
        // The start of the element that is next to be put in its place.
        let mut sorted = 0;

        while sorted < region.len() {
            let Some(length) = tlv_length(region, sorted) else {
                return;
            };
            let end = sorted + length;
            // The first of those before it that is greater than it.
            let mut at = 0;

            while at < sorted {
                let Some(other) = tlv_length(region, at) else {
                    return;
                };

                if compare(&region[at..at + other], &region[sorted..end]) == Ordering::Greater {
                    break;
                }

                at += other;
            }

            if at < sorted {
                region[at..end].rotate_right(length);
            }

            sorted = end;
        }
    }
}

/// The first byte of the two's complement `bytes` that is not only the
/// sign of the next.
fn minimal_start(bytes: &[u8]) -> usize {
    let mut start = 0;

    while start + 1 < bytes.len()
        && ((bytes[start] == 0x00 && bytes[start + 1] & 0x80 == 0)
            || (bytes[start] == 0xff && bytes[start + 1] & 0x80 != 0))
    {
        start += 1;
    }

    start
}

/// The length of the TLV of definite length at `offset`.
fn tlv_length(data: &[u8], offset: usize) -> Option<usize> {
    let (_, _, _, after) = read_tag(data, offset).ok()?;
    let (length, start) = read_length(data, after, true, false).ok()?;

    Some(start + length? - offset)
}

fn padded_compare(a: &[u8], b: &[u8]) -> Ordering {
    let longest = a.len().max(b.len());

    (0..longest)
        .map(|i| {
            (
                a.get(i).copied().unwrap_or(0),
                b.get(i).copied().unwrap_or(0),
            )
        })
        .find(|(x, y)| x != y)
        .map_or(Ordering::Equal, |(x, y)| x.cmp(&y))
}

/// The bytes of the next primitive part of a constructed string from
/// `at`, and where the one after it is. Those parts were checked when
/// the string was decoded.
pub(crate) fn next_part(data: &[u8], mut at: usize) -> Option<(&[u8], usize)> {
    loop {
        if data.get(at..at + 2)? == [0, 0] {
            at += 2;

            continue;
        }

        let (_, constructed, _, after) = read_tag(data, at).ok()?;
        let (length, start) = read_length(data, after, false, false).ok()?;

        if constructed {
            at = start;

            continue;
        }

        let end = start + length?;

        return Some((&data[start..end], end));
    }
}

/// The contents of a constructed value: up to `end`, or the
/// end-of-contents octets if its length is indefinite.
#[derive(Debug, Clone, Copy)]
pub struct Region {
    end: Option<usize>,
}

/// Reads BER, or DER that it checks is DER.
#[derive(Clone)]
pub struct Reader<'a> {
    pub(crate) data: &'a [u8],
    pub(crate) position: usize,
    der: bool,
    depth: usize,
}

impl<'a> Reader<'a> {
    pub fn new(data: &'a [u8], der: bool) -> Reader<'a> {
        Reader {
            data,
            position: 0,
            der,
            depth: 0,
        }
    }

    /// If it reads DER, and checks what DER requires.
    pub fn der(&self) -> bool {
        self.der
    }

    pub fn position(&self) -> usize {
        self.position
    }

    pub fn data(&self) -> &'a [u8] {
        self.data
    }

    /// Checks that all of the data was read.
    pub fn finish(&self) -> Result<()> {
        if self.position != self.data.len() {
            return decode_error("Expected the data to be all of the value.", self.position);
        }

        Ok(())
    }

    /// The class and the number of the tag it is at.
    pub fn peek_tag(&self) -> Option<(TagClass, u64)> {
        read_tag(self.data, self.position)
            .ok()
            .map(|(class, _, number, _)| (class, number))
    }

    /// If there is more of `region` to read.
    pub fn more(&self, region: &Region) -> Result<bool> {
        match region.end {
            Some(end) => Ok(self.position < end),
            None => match self.data.get(self.position..self.position + 2) {
                Some([0, 0]) => Ok(false),
                Some(_) => Ok(true),
                None => decode_error(
                    "Ran out of data when trying to find End of Contents tag.",
                    self.position,
                ),
            },
        }
    }

    /// The class and the number of the tag of the next TLV in `region`,
    /// if there is one.
    pub fn peek_in(&self, region: &Region) -> Result<Option<(TagClass, u64)>> {
        if !self.more(region)? {
            return Ok(None);
        }

        Ok(self.peek_tag())
    }

    /// Reads a tag that has to be `class` and `number`, and constructed
    /// or not as `constructed` says, if it says. Returns if it is
    /// constructed, and the offset after it.
    fn expect_tag(
        &mut self,
        class: TagClass,
        number: u64,
        constructed: Option<bool>,
    ) -> Result<(bool, usize)> {
        let (found_class, found_constructed, found_number, after) =
            read_tag(self.data, self.position)?;

        if found_class != class
            || found_number != number
            || constructed.is_some_and(|constructed| constructed != found_constructed)
        {
            return decode_error("Expected another tag.", self.position);
        }

        Ok((found_constructed, after))
    }

    /// A constructed TLV of the tag `class` and `number`, as an explicit
    /// tag, a SEQUENCE or a SEQUENCE OF is. Returns its contents, which
    /// [`Reader::close`] ends.
    pub fn constructed(&mut self, class: TagClass, number: u64) -> Result<Region> {
        if self.depth >= MAX_DEPTH {
            return decode_error("The data is nested too deeply.", self.position);
        }

        let (_, after) = self.expect_tag(class, number, Some(true))?;
        let (length, start) = read_length(self.data, after, self.der, self.der)?;

        self.depth += 1;
        self.position = start;

        Ok(Region {
            end: length.map(|length| start + length),
        })
    }

    /// Checks that the element of a SET OF from `start` to where it is
    /// is not less than the one before it, of which `previous` is where
    /// it is, as DER has them (X.690 11.6).
    pub fn check_element_order(
        &self,
        previous: &mut Option<(usize, usize)>,
        start: usize,
    ) -> Result<()> {
        let encoded = &self.data[start..self.position];

        if let Some((from, to)) = *previous
            && padded_compare(&self.data[from..to], encoded) == Ordering::Greater
        {
            return decode_error(
                "Expected the elements of a SET OF in ascending order of their encodings.",
                start,
            );
        }

        *previous = Some((start, self.position));

        Ok(())
    }

    /// Checks that the member of a SET with the tag `tag` is after the
    /// one before it, of which `previous` is the tag, as DER has them.
    pub fn check_member_order(
        &self,
        previous: &mut Option<(TagClass, u64)>,
        tag: (TagClass, u64),
    ) -> Result<()> {
        if previous.is_some_and(|previous| previous >= tag) {
            return decode_error(
                "The members of a SET are not in the order of their tags.",
                self.position,
            );
        }

        *previous = Some(tag);

        Ok(())
    }

    /// Ends `region`, of which all is read.
    pub fn close(&mut self, region: Region) -> Result<()> {
        self.depth -= 1;

        match region.end {
            Some(end) if self.position == end => Ok(()),
            Some(_) => decode_error("Expected the end of the contents.", self.position),
            None => {
                if self.data.get(self.position..self.position + 2) != Some(&[0, 0]) {
                    return decode_error("Expected end-of-contents tag.", self.position);
                }

                self.position += 2;

                Ok(())
            }
        }
    }

    /// Skips what is left of `region`, as additions of a later version
    /// of a SEQUENCE, and ends it.
    pub fn skip_rest(&mut self, region: Region) -> Result<()> {
        while self.more(&region)? {
            self.any()?;
        }

        self.close(region)
    }

    /// The contents of a primitive TLV of the tag `class` and `number`.
    pub fn primitive(&mut self, class: TagClass, number: u64) -> Result<&'a [u8]> {
        let (_, after) = self.expect_tag(class, number, Some(false))?;
        let (length, start) = read_length(self.data, after, true, self.der)?;
        let end = start + length.unwrap_or(0);

        self.position = end;

        Ok(&self.data[start..end])
    }

    /// A TLV, all of it, whatever its tag.
    pub fn any(&mut self) -> Result<&'a [u8]> {
        let start = self.position;
        let (_, constructed, _, after) = read_tag(self.data, start)?;

        if after >= self.data.len() {
            return decode_error("Ran out of data when reading tag", after);
        }

        let (length, contents) = read_length(self.data, after, false, false)?;

        self.position = match length {
            Some(length) => contents + length,
            None if constructed => {
                if self.depth >= MAX_DEPTH {
                    return decode_error("The data is nested too deeply.", start);
                }

                self.depth += 1;
                self.position = contents;

                let region = Region { end: None };

                while self.more(&region)? {
                    self.any()?;
                }

                self.depth -= 1;
                self.position + 2
            }
            None => return decode_error("Expected definite length.", after),
        };

        Ok(&self.data[start..self.position])
    }

    /// The contents of a string of the tag `class` and `number`, which
    /// BER may have in parts (X.690 8.7.3) of the tag of `universal`:
    /// those of a BIT STRING are BIT STRINGs, of the others OCTET
    /// STRINGs.
    /// Returns the bytes, and with `bits` the unused bits of the last
    /// part if it is in parts.
    fn string(&mut self, class: TagClass, number: u64, bits: bool) -> Result<(Octets<'a>, u8)> {
        let (constructed, after) = self.expect_tag(class, number, None)?;

        if !constructed {
            let (length, start) = read_length(self.data, after, true, self.der)?;
            let end = start + length.unwrap_or(0);

            self.position = end;

            return Ok((Octets::new(&self.data[start..end]), 0));
        }

        if self.der {
            return decode_error("Expected a string that is not constructed.", self.position);
        }

        let (length, start) = read_length(self.data, after, false, false)?;
        let mut total = 0;
        let mut previous_unused = 0;

        self.position = start;
        self.parts(
            length.map(|length| start + length),
            bits,
            &mut total,
            &mut previous_unused,
            0,
        )?;

        let end = self.position;

        Ok((
            Octets::ber_parts(&self.data[start..end], total, bits),
            previous_unused,
        ))
    }

    /// Checks the parts of a constructed string, and counts their bytes.
    fn parts(
        &mut self,
        end: Option<usize>,
        bits: bool,
        total: &mut usize,
        previous_unused: &mut u8,
        depth: usize,
    ) -> Result<()> {
        if self.depth + depth >= MAX_DEPTH {
            return decode_error("The data is nested too deeply.", self.position);
        }

        let tag = if bits { 3 } else { 4 };

        loop {
            match end {
                Some(end) if self.position >= end => break,
                None => match self.data.get(self.position..self.position + 2) {
                    Some([0, 0]) => {
                        self.position += 2;

                        break;
                    }
                    Some(_) => {}
                    None => {
                        return decode_error(
                            "Ran out of data when trying to find End of Contents tag.",
                            self.position,
                        );
                    }
                },
                _ => {}
            }

            let (class, constructed, number, after) = read_tag(self.data, self.position)?;

            if class != TagClass::Universal || number != tag {
                return decode_error("Expected another tag.", self.position);
            }

            let (length, start) = read_length(self.data, after, false, false)?;

            if constructed {
                self.position = start;
                self.parts(
                    length.map(|length| start + length),
                    bits,
                    total,
                    previous_unused,
                    depth + 1,
                )?;

                continue;
            }

            let length = length.ok_or_else(|| Error::decode("Expected definite length.", after))?;
            let contents = &self.data[start..start + length];

            if bits {
                // Only the last part has unused bits (X.690 8.6.4).
                if *previous_unused != 0 {
                    return decode_error(
                        "Expected unused bits only in the last part of a BIT STRING.",
                        start,
                    );
                }

                check_bit_string(contents, false, false, start)?;
                *previous_unused = contents[0];
                *total += length - 1;
            } else {
                *total += length;
            }

            self.position = start + length;
        }

        Ok(())
    }

    pub fn octet_string(&mut self, class: TagClass, number: u64) -> Result<Octets<'a>> {
        self.string(class, number, false).map(|(octets, _)| octets)
    }

    /// A BIT STRING, of which DER checks that the unused bits are zero
    /// and, if it has `named` bits, that there are no trailing zero bits.
    pub fn bit_string(
        &mut self,
        class: TagClass,
        number: u64,
        named: bool,
    ) -> Result<BitString<'a>> {
        let start = self.position;
        let (octets, unused) = self.string(class, number, true)?;

        match octets.as_slice() {
            Some(contents) => {
                let bits = check_bit_string(contents, self.der, named, start)?;

                Ok(BitString::from_octets(Octets::new(&contents[1..]), bits))
            }
            None => Ok(BitString::from_octets(
                octets,
                8 * octets.len() - usize::from(unused),
            )),
        }
    }

    /// A character string of `kind`, which it checks the bytes are of.
    pub fn text(&mut self, class: TagClass, number: u64, kind: StrKind) -> Result<Text<'a>> {
        let start = self.position;
        let (octets, _) = self.string(class, number, false)?;
        let width = check_text(&octets, kind, start)?;

        Ok(Text::from_bytes(octets, width))
    }
}

/// Checks the contents of a BIT STRING, and returns its number of bits.
pub(crate) fn check_bit_string(
    contents: &[u8],
    strict: bool,
    named: bool,
    offset: usize,
) -> Result<usize> {
    let Some((&unused, bytes)) = contents.split_first() else {
        return decode_error("Expected at least one byte of a BIT STRING.", offset);
    };

    if unused > 7 || (bytes.is_empty() && unused != 0) {
        return decode_error("Expected at most 7 unused bits of a BIT STRING.", offset);
    }

    let bits = 8 * bytes.len() - usize::from(unused);

    if strict {
        if unused != 0
            && bytes
                .last()
                .is_some_and(|last| last & ((1 << unused) - 1) != 0)
        {
            return decode_error(
                "Expected the unused bits of a BIT STRING to be zero.",
                offset,
            );
        }

        if named && bits > 0 && bytes.last().is_some_and(|last| last >> unused & 1 == 0) {
            return decode_error(
                "Expected a BIT STRING with named bits without trailing zero bits.",
                offset,
            );
        }
    }

    Ok(bits)
}

/// Checks that the bytes are those of a string of `kind`.
fn check_text(octets: &Octets<'_>, kind: StrKind, offset: usize) -> Result<Width> {
    let error = |what| Err(Error::decode(what, offset));

    match kind {
        StrKind::Utf8 => {
            let valid = match octets.as_slice() {
                Some(bytes) => core::str::from_utf8(bytes).is_ok(),
                None => utf8_valid(octets.bytes()),
            };

            if !valid {
                return error("The string is not UTF-8.");
            }

            Ok(Width::Utf8)
        }
        StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
            if !octets.bytes().all(|byte| byte.is_ascii()) {
                return error("The string is not ASCII.");
            }

            Ok(Width::Latin1)
        }
        StrKind::Bmp => {
            let mut bytes = octets.bytes();

            if !octets.len().is_multiple_of(2) {
                return error("The string is not a BMPString.");
            }

            while let (Some(high), Some(low)) = (bytes.next(), bytes.next()) {
                if char::from_u32(u32::from(u16::from_be_bytes([high, low]))).is_none() {
                    return error("The string is not a BMPString.");
                }
            }

            Ok(Width::Bmp)
        }
        StrKind::Universal => {
            let mut bytes = octets.bytes();

            if !octets.len().is_multiple_of(4) {
                return error("The string is not UTF-32.");
            }

            while let (Some(a), Some(b), Some(c), Some(d)) =
                (bytes.next(), bytes.next(), bytes.next(), bytes.next())
            {
                if char::from_u32(u32::from_be_bytes([a, b, c, d])).is_none() {
                    return error("The string is not UTF-32.");
                }
            }

            Ok(Width::Universal)
        }
        _ => Ok(Width::Latin1),
    }
}

/// If the bytes are UTF-8, without copying them.
pub(crate) fn utf8_valid(bytes: impl Iterator<Item = u8>) -> bool {
    let mut buffer = [0u8; 4];
    let mut have = 0;
    let mut need = 0;

    for byte in bytes {
        if have == 0 {
            need = match byte {
                0x00..=0x7f => 1,
                0xc2..=0xdf => 2,
                0xe0..=0xef => 3,
                0xf0..=0xf4 => 4,
                _ => return false,
            };
        }

        buffer[have] = byte;
        have += 1;

        if have == need {
            if core::str::from_utf8(&buffer[..have]).is_err() {
                return false;
            }

            have = 0;
        }
    }

    have == 0
}

/// The contents of a BOOLEAN, which DER has as 0x00 or 0xff.
pub fn boolean(r: &Reader<'_>, contents: &[u8]) -> Result<bool> {
    check_boolean(contents, r.der, r.position - contents.len())
}

/// The same, of `contents` at `offset`, which are DER if `strict`.
pub(crate) fn check_boolean(contents: &[u8], strict: bool, offset: usize) -> Result<bool> {
    if contents.len() != 1 {
        return decode_error("Expected BOOLEAN contents length 1.", offset);
    }

    if strict && contents[0] != 0x00 && contents[0] != 0xff {
        return decode_error("Expected BOOLEAN contents of 0x00 or 0xff.", offset);
    }

    Ok(contents[0] != 0)
}

/// The contents of an INTEGER or an ENUMERATED, which DER has in as few
/// bytes as it can be.
pub fn integer(r: &Reader<'_>, contents: &[u8]) -> Result<i128> {
    check_integer(r, contents)?;

    let start = minimal_start(contents);
    let bytes = &contents[start..];

    if bytes.len() > 16 {
        return Err(Error::unsupported("An INTEGER is too large."));
    }

    let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);
    let mut value: i128 = if negative { -1 } else { 0 };

    for byte in bytes {
        value = (value << 8) | i128::from(*byte);
    }

    Ok(value)
}

/// The contents of an INTEGER of any size, borrowed.
pub fn big_integer<'a>(r: &Reader<'a>, contents: &'a [u8]) -> Result<Integer<'a>> {
    check_integer(r, contents)?;

    Ok(Integer::from_bytes(contents))
}

/// Checks what DER requires of the contents of an INTEGER.
fn check_integer(r: &Reader<'_>, contents: &[u8]) -> Result<()> {
    check_integer_contents(contents, r.der, r.position - contents.len())
}

/// The same, of `contents` at `offset`, which are DER if `strict`.
pub(crate) fn check_integer_contents(contents: &[u8], strict: bool, offset: usize) -> Result<()> {
    if strict {
        if contents.is_empty() {
            return decode_error("Expected at least one byte of an INTEGER.", offset);
        }

        if contents.len() > 1
            && ((contents[0] == 0x00 && contents[1] < 0x80)
                || (contents[0] == 0xff && contents[1] >= 0x80))
        {
            return decode_error("Expected an INTEGER in as few bytes as possible.", offset);
        }
    }

    Ok(())
}

/// An integer that has to fit in the type the code has for it.
pub fn fit<T: TryFrom<i128>>(r: &Reader<'_>, value: i128) -> Result<T> {
    T::try_from(value).map_err(|_| Error::decode("The INTEGER is out of range.", r.position))
}

/// The contents of a NULL.
pub fn null(r: &Reader<'_>, contents: &[u8]) -> Result<()> {
    if !contents.is_empty() {
        return decode_error("Expected NULL contents length 0.", r.position);
    }

    Ok(())
}

/// The contents of an OBJECT IDENTIFIER.
pub fn oid<'a>(r: &Reader<'a>, contents: &'a [u8]) -> Result<Oid<'a>> {
    Oid::checked(Octets::new(contents), r.der, r.position - contents.len())
}

fn ascii(contents: &[u8], offset: usize) -> Result<&str> {
    if !contents.is_ascii() {
        return decode_error("The string is not ASCII.", offset);
    }

    core::str::from_utf8(contents).map_err(|_| Error::decode("The string is not ASCII.", offset))
}

/// The contents of a UTCTime. DER has one form of it (X.690 11.8).
pub fn utc_time(r: &Reader<'_>, contents: &[u8]) -> Result<DateTime> {
    let text = ascii(contents, r.position - contents.len())?;

    time::utc_time_from_string(text, r.der)
}

/// The contents of a GeneralizedTime. DER has one form of it (X.690
/// 11.7).
pub fn generalized_time(r: &Reader<'_>, contents: &[u8]) -> Result<DateTime> {
    let text = ascii(contents, r.position - contents.len())?;

    time::generalized_time_from_string(text, r.der)
}
