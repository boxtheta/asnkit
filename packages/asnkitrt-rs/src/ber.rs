// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Basic Encoding Rules (BER) and Distinguished Encoding Rules (DER),
//! X.690.
//!
//! A type is encoded as a tag, a length and the contents. The tags a
//! type has on the wire are worked out when the types are lowered: an
//! explicit tag is a tag around the tag of the type it tags, and an
//! implicit one takes the place of that tag.

use alloc::borrow::Cow;
use alloc::string::String;
use alloc::vec::Vec;

use crate::node::TagClass;
use crate::time;
use crate::types::{
    Addition, Codec, Kind, MemberInfo, Members, StrKind, TypeId, Types, WireTag, clean_bits,
    const_matches, is_default,
};
use crate::value::{Lookup, ObjectIdentifier, Value, integer_to_twos_complement};
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

/// Encodes `value` as the type `id`.
pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    let mut out = Vec::new();

    Coder::new(types).encode_type(id, value, &mut out, None, 0)?;

    Ok(out)
}

/// Decodes the type `id` from the start of `data`, and returns it with
/// the length of what was decoded.
pub fn decode_with_length<'a>(
    types: &'a Types,
    id: TypeId,
    data: &'a [u8],
) -> Result<(Value<'a>, usize)> {
    match Coder::new(types).decode_type(id, data, 0, None, 0)? {
        Some(decoded) => Ok(decoded),
        None => Err(Error::decode("Expected a tag that the type has.", 0)),
    }
}

/// Decodes the type `id` from `data`, which is all of it.
pub fn decode<'a>(types: &'a Types, id: TypeId, data: &'a [u8]) -> Result<Value<'a>> {
    let (value, length) = decode_with_length(types, id, data)?;

    if length != data.len() {
        return Err(Error::decode(
            "Expected the data to be all of the value.",
            length,
        ));
    }

    Ok(value)
}

/// The total length of the first value in `data`, if it is known.
pub fn decode_length(data: &[u8]) -> Option<usize> {
    let start = skip_tag(data, 0).ok()?;

    match read_length(data, start, true, false) {
        Ok((Some(length), offset)) => Some(offset + length),
        Err(Error::Decode(..)) => {
            // Not all there: the length it says, if it can be read.
            let (length, offset) = read_length_only(data, start)?;

            Some(offset + length)
        }
        _ => None,
    }
}

fn read_length_only(data: &[u8], offset: usize) -> Option<(usize, usize)> {
    let first = *data.get(offset)?;

    if first & 0x80 == 0 {
        return Some((usize::from(first), offset + 1));
    }

    let count = usize::from(first & 0x7f);

    if count == 0 || count > 8 {
        return None;
    }

    let bytes = data.get(offset + 1..offset + 1 + count)?;
    let length = bytes
        .iter()
        .fold(0usize, |length, byte| (length << 8) | usize::from(*byte));

    Some((length, offset + 1 + count))
}

fn decode_error<T>(what: &'static str, offset: usize) -> Result<T> {
    Err(Error::decode(what, offset))
}

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::encode(what))
}

fn write_tag(tag: &WireTag, out: &mut Vec<u8>) {
    let flags = match tag.class {
        TagClass::Universal => 0x00,
        TagClass::Application => 0x40,
        TagClass::ContextSpecific => 0x80,
        TagClass::Private => 0xc0,
    } | if tag.constructed { 0x20 } else { 0 };

    if tag.number < 31 {
        out.push(flags | tag.number as u8);
    } else {
        out.push(flags | 0x1f);

        let mut groups = Vec::new();
        let mut number = tag.number;

        while number > 0 {
            groups.push(0x80 | (number & 0x7f) as u8);
            number >>= 7;
        }

        groups[0] &= 0x7f;
        groups.reverse();
        out.extend_from_slice(&groups);
    }
}

/// A TLV of `tag`, of which `contents` writes the contents after the
/// tag and the length, which is made longer after them if it has to be.
fn write_tlv(
    tag: &WireTag,
    out: &mut Vec<u8>,
    contents: impl FnOnce(&mut Vec<u8>) -> Result<()>,
) -> Result<()> {
    write_tag(tag, out);

    let at = out.len();

    // Most lengths are a byte.
    out.push(0);
    contents(out)?;

    let length = out.len() - at - 1;

    if length <= 127 {
        out[at] = length as u8;
    } else {
        let bytes = length.to_be_bytes();
        let skip = bytes.iter().take_while(|byte| **byte == 0).count();

        out[at] = 0x80 | (bytes.len() - skip) as u8;
        out.splice(at + 1..at + 1, bytes[skip..].iter().copied());
    }

    Ok(())
}

/// The tag at `offset`: the class, if it is constructed, the number
/// and the offset after it.
fn read_tag(data: &[u8], offset: usize) -> Result<(TagClass, bool, u64, usize)> {
    let first = *data
        .get(offset)
        .ok_or_else(|| Error::decode("Ran out of data when reading tag", offset))?;
    let class = match first >> 6 {
        0 => TagClass::Universal,
        1 => TagClass::Application,
        2 => TagClass::ContextSpecific,
        _ => TagClass::Private,
    };
    let constructed = first & 0x20 != 0;
    let mut at = offset + 1;
    let mut number = u64::from(first & 0x1f);

    if number == 0x1f {
        number = 0;

        loop {
            let byte = *data
                .get(at)
                .ok_or_else(|| Error::decode("Ran out of data when reading tag", at))?;

            at += 1;

            if number >> 57 != 0 {
                return decode_error("A tag number is too large.", offset);
            }

            number = (number << 7) | u64::from(byte & 0x7f);

            if byte & 0x80 == 0 {
                break;
            }
        }
    }

    Ok((class, constructed, number, at))
}

/// The offset after the tag at `offset`. As BER does, it needs a byte
/// after the tag.
fn skip_tag(data: &[u8], offset: usize) -> Result<usize> {
    let (_, _, _, at) = read_tag(data, offset)?;

    if at >= data.len() {
        return decode_error("Ran out of data when reading tag", at);
    }

    Ok(at)
}

/// Reads a length: `None` is indefinite. Checks that there is as much
/// data after it.
fn read_length(
    data: &[u8],
    offset: usize,
    enforce_definite: bool,
    enforce_minimal: bool,
) -> Result<(Option<usize>, usize)> {
    let first = *data
        .get(offset)
        .ok_or_else(|| Error::decode("Ran out of data when trying to read length", offset))?;
    let mut at = offset + 1;
    let length;

    if first & 0x80 == 0 {
        length = usize::from(first);
    } else if first == 0x80 {
        if enforce_definite {
            return decode_error("Expected definite length, but got indefinite.", offset);
        }

        return Ok((None, at));
    } else {
        let count = usize::from(first & 0x7f);
        let bytes = data
            .get(at..at + count)
            .ok_or_else(|| Error::decode("Ran out of data when trying to read length", at))?;

        if count > core::mem::size_of::<usize>()
            && bytes.iter().take(count - 8).any(|byte| *byte != 0)
        {
            return decode_error("A length is too large.", at);
        }

        length = bytes
            .iter()
            .fold(0usize, |length, byte| (length << 8) | usize::from(*byte));
        at += count;

        if enforce_minimal && (length < 128 || bytes[0] == 0) {
            return decode_error("Expected the length in as few bytes as possible.", offset);
        }
    }

    if length > data.len() - at {
        return decode_error("Expected more contents bytes than there are.", at);
    }

    Ok((Some(length), at))
}

fn string_bytes(kind: StrKind, text: &str) -> Result<Vec<u8>> {
    match kind {
        StrKind::Utf8 => Ok(Vec::from(text.as_bytes())),
        StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
            if text.is_ascii() {
                Ok(Vec::from(text.as_bytes()))
            } else {
                encode_error("A character is not ASCII.")
            }
        }
        StrKind::General
        | StrKind::Graphic
        | StrKind::ObjectDescriptor
        | StrKind::Teletex
        | StrKind::Videotex => text
            .chars()
            .map(|character| {
                u8::try_from(u32::from(character))
                    .map_err(|_| Error::encode("A character is not Latin-1."))
            })
            .collect(),
        StrKind::Bmp => bmp_bytes(text),
        StrKind::Universal => {
            let mut bytes = Vec::new();

            for character in text.chars() {
                bytes.extend_from_slice(&u32::from(character).to_be_bytes());
            }

            Ok(bytes)
        }
    }
}

/// A BMPString in the 2-octet form, of which each character is in the
/// BMP (X.690 8.23.8, X.696 27.4).
pub(crate) fn bmp_bytes(text: &str) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();

    for character in text.chars() {
        let unit = u16::try_from(u32::from(character))
            .map_err(|_| Error::encode("A character is not in the BMP."))?;

        bytes.extend_from_slice(&unit.to_be_bytes());
    }

    Ok(bytes)
}

/// The text of a BMPString in the 2-octet form. A surrogate is not a
/// character.
pub(crate) fn bmp_text(bytes: &[u8], offset: usize) -> Result<String> {
    if !bytes.len().is_multiple_of(2) {
        return decode_error("The string is not a BMPString.", offset);
    }

    bytes
        .chunks(2)
        .map(|pair| char::from_u32(u32::from(u16::from_be_bytes([pair[0], pair[1]]))))
        .collect::<Option<String>>()
        .ok_or_else(|| Error::decode("The string is not a BMPString.", offset))
}

/// The text of a string from its bytes, borrowed where they are its
/// UTF-8: a UTF8String, and the others when they are ASCII.
pub(crate) fn string_text(kind: StrKind, bytes: &[u8], offset: usize) -> Result<Cow<'_, str>> {
    match kind {
        StrKind::Utf8 => core::str::from_utf8(bytes)
            .map(Cow::Borrowed)
            .map_err(|_| Error::decode("The string is not UTF-8.", offset)),
        StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
            ascii(bytes, offset).map(Cow::Borrowed)
        }
        StrKind::General
        | StrKind::Graphic
        | StrKind::ObjectDescriptor
        | StrKind::Teletex
        | StrKind::Videotex => Ok(match ascii(bytes, offset) {
            Ok(text) => Cow::Borrowed(text),
            Err(_) => Cow::Owned(bytes.iter().map(|byte| char::from(*byte)).collect()),
        }),
        StrKind::Bmp => bmp_text(bytes, offset).map(Cow::Owned),
        StrKind::Universal => {
            if !bytes.len().is_multiple_of(4) {
                return decode_error("The string is not UTF-32.", offset);
            }

            bytes
                .chunks(4)
                .map(|quad| {
                    char::from_u32(u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]]))
                })
                .collect::<Option<String>>()
                .map(Cow::Owned)
                .ok_or_else(|| Error::decode("The string is not UTF-32.", offset))
        }
    }
}

/// The REAL of X.690 8.5 in the form of DER (X.690 11.3.1), which PER
/// and OER have too: base 2, no scaling factor, and an odd mantissa and
/// its exponent each in the fewest octets.
pub(crate) fn encode_real(value: f64) -> Result<Vec<u8>> {
    if value == f64::INFINITY {
        return Ok(alloc::vec![0x40]);
    } else if value == f64::NEG_INFINITY {
        return Ok(alloc::vec![0x41]);
    } else if value.is_nan() {
        return Ok(alloc::vec![0x42]);
    } else if value == 0.0 {
        // Minus zero is a special value (X.690 8.5.3).
        return Ok(if value.is_sign_negative() {
            alloc::vec![0x43]
        } else {
            Vec::new()
        });
    }

    let negative = value < 0.0;
    let bits = value.abs().to_bits();
    let biased = ((bits >> 52) & 0x7ff) as i64;
    let fraction = bits & ((1u64 << 52) - 1);
    // The number is mantissa * 2**exponent.
    let (mut mantissa, mut exponent) = if biased == 0 {
        (fraction, -1074i64)
    } else {
        (fraction | (1u64 << 52), biased - 1075)
    };
    let shift = mantissa.trailing_zeros();

    mantissa >>= shift;
    exponent += i64::from(shift);

    let sign = if negative { 0x40 } else { 0 };
    let mut out = Vec::new();

    if (-128..=127).contains(&exponent) {
        out.push(0x80 | sign);
        out.push(exponent as i8 as u8);
    } else {
        // Every exponent of an f64 fits in two octets.
        out.push(0x81 | sign);
        out.extend_from_slice(&(exponent as i16).to_be_bytes());
    }

    let length = (64 - mantissa.leading_zeros() as usize).div_ceil(8);

    out.extend_from_slice(&mantissa.to_be_bytes()[8 - length..]);

    Ok(out)
}

pub(crate) fn twos_complement_to_f64(bytes: &[u8]) -> Result<f64> {
    let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);
    let mut value = 0.0f64;

    for byte in bytes {
        let byte = if negative { !*byte } else { *byte };

        value = value * 256.0 + f64::from(byte);
    }

    let value = if negative { -(value + 1.0) } else { value };

    if value.is_finite() {
        Ok(value)
    } else {
        encode_error("The integer is too large for a REAL.")
    }
}

fn pow2(exponent: i64) -> f64 {
    // Two to the power of an exponent in the range of the normal numbers.
    f64::from_bits(((exponent + 1023) as u64) << 52)
}

/// `value` times two to the power of `exponent`, which is exact when
/// the result is a float.
fn scale(mut value: f64, mut exponent: i64) -> f64 {
    while exponent > 1000 {
        value *= pow2(1000);
        exponent -= 1000;
    }

    while exponent < -1000 {
        value *= pow2(-1000);
        exponent += 1000;
    }

    value * pow2(exponent)
}

/// If the two's complement number has an octet more than it needs.
fn redundant(bytes: &[u8]) -> bool {
    bytes.len() > 1
        && ((bytes[0] == 0x00 && bytes[1] & 0x80 == 0)
            || (bytes[0] == 0xff && bytes[1] & 0x80 != 0))
}

/// A two's complement number, as one beyond 2**100 is that to the
/// float it makes.
fn exponent_value(mut bytes: &[u8]) -> i128 {
    while redundant(bytes) {
        bytes = &bytes[1..];
    }

    if bytes.len() > 12 {
        return if bytes[0] & 0x80 != 0 {
            -(1 << 100)
        } else {
            1 << 100
        };
    }

    let mut value = if bytes.first().is_some_and(|byte| byte & 0x80 != 0) {
        -1i128
    } else {
        0
    };

    for byte in bytes {
        value = (value << 8) | i128::from(*byte);
    }

    value
}

/// The float nearest to `mantissa` times two to the power of
/// `exponent`, with ties to even, as Python reads an integer as a
/// float. `mantissa` is unsigned and big-endian.
fn nearest_float(mantissa: &[u8], exponent: i128) -> core::result::Result<f64, &'static str> {
    let start = mantissa.iter().position(|byte| *byte != 0);
    let Some(start) = start else {
        return Ok(0.0);
    };
    let mantissa = &mantissa[start..];
    let length = mantissa.len() as i128 * 8 - i128::from(mantissa[0].leading_zeros());
    let bit = |index: i128| -> u64 {
        let byte = mantissa[mantissa.len() - 1 - (index / 8) as usize];

        u64::from((byte >> (index % 8)) & 1)
    };
    // Of the highest bit.
    let top = length - 1 + exponent;

    if top > 1023 {
        return Err("The REAL is too large for a float.");
    } else if top < -1075 {
        return Ok(0.0);
    }

    // The bits a float has for it: fewer if it is subnormal.
    let precision = if top >= -1022 { 53 } else { 53 + 1022 + top };
    let dropped = length - precision;
    let mut kept = 0u64;

    for index in (dropped.max(0)..length).rev() {
        kept = (kept << 1) | bit(index);
    }

    if dropped > 0 {
        let half = bit(dropped - 1) == 1;
        let sticky = (0..dropped - 1).any(|index| bit(index) == 1);

        if half && (sticky || kept & 1 == 1) {
            kept += 1;
        }
    }

    let value = scale(kept as f64, (exponent + dropped.max(0)) as i64);

    if value.is_finite() {
        Ok(value)
    } else {
        Err("The REAL is too large for a float.")
    }
}

/// A REAL in binary (X.690 8.5.7), of DER only if `strict` (X.690
/// 11.3.1).
fn binary_real(control: u8, rest: &[u8], strict: bool) -> core::result::Result<f64, &'static str> {
    const OUT: &str = "Out of data in a REAL.";

    let base_bits: i128 = match (control >> 4) & 0x03 {
        0 => 1,
        1 => 3,
        2 => 4,
        _ => return Err("A REAL has a reserved base."),
    };
    let (exponent, mantissa) = match control & 0x03 {
        0x03 => {
            let (&count, rest) = rest.split_first().ok_or(OUT)?;

            if count == 0 {
                return Err("A REAL has an exponent of no octets.");
            }

            rest.split_at_checked(usize::from(count)).ok_or(OUT)?
        }
        form => rest.split_at_checked(usize::from(form) + 1).ok_or(OUT)?,
    };

    // The long form has no first nine bits all the same (X.690
    // 8.5.7.4 d).
    if control & 0x03 == 0x03 && redundant(exponent) {
        return Err("The exponent of a REAL is not in the fewest octets.");
    }

    if strict {
        if control & 0x3c != 0 {
            return Err("A REAL of DER has the base 2 and no scaling factor.");
        }

        if redundant(exponent) || control & 0x03 == 0x03 && exponent.len() <= 3 {
            return Err("The exponent of a REAL is not in the fewest octets.");
        }

        if mantissa.first().is_none_or(|byte| *byte == 0)
            || mantissa.last().is_some_and(|byte| byte & 1 == 0)
        {
            return Err("The mantissa of a REAL is not odd and in the fewest octets.");
        }
    }

    let scaling = i128::from((control >> 2) & 0x03);
    let value = nearest_float(mantissa, exponent_value(exponent) * base_bits + scaling)?;

    Ok(if control & 0x40 != 0 { -value } else { value })
}

/// A REAL in decimal (X.690 8.5.8): ISO 6093 NR1, NR2 or NR3, after
/// spaces, or only NR3 in the form of DER if `strict` (X.690 11.3.2).
fn decimal_real(control: u8, text: &[u8], strict: bool) -> core::result::Result<f64, &'static str> {
    const BAD: &str = "Bad decimal REAL.";

    fn digits(text: &[u8]) -> usize {
        text.iter().take_while(|c| c.is_ascii_digit()).count()
    }

    let form = control & 0x3f;

    if strict {
        // -?[1-9]([0-9]*[1-9])?.E(+0|-?[1-9][0-9]*)
        let rest = text.strip_prefix(b"-").unwrap_or(text);
        let mantissa = digits(rest);
        let exponent = rest[mantissa..].strip_prefix(b".E").ok_or(BAD)?;
        let good_exponent = exponent == b"+0" || {
            let exponent = exponent.strip_prefix(b"-").unwrap_or(exponent);

            exponent.first().is_some_and(|c| (b'1'..=b'9').contains(c))
                && digits(exponent) == exponent.len()
        };
        let good = form == 0x03
            && mantissa > 0
            && rest[0] != b'0'
            && rest[mantissa - 1] != b'0'
            && good_exponent;

        if !good {
            return Err("A decimal REAL of DER is not in the form of X.690 11.3.2.");
        }
    } else {
        let rest = &text[text.iter().take_while(|c| **c == b' ').count()..];
        let rest = rest
            .strip_prefix(b"-")
            .or_else(|| rest.strip_prefix(b"+"))
            .unwrap_or(rest);
        let whole = digits(rest);
        let mut rest = &rest[whole..];
        let mut point = false;
        let mut fraction = 0;

        if let Some(after) = rest.strip_prefix(b".").or_else(|| rest.strip_prefix(b",")) {
            point = true;
            fraction = digits(after);
            rest = &after[fraction..];
        }

        let mut exponent = false;

        if let Some(after) = rest.strip_prefix(b"E").or_else(|| rest.strip_prefix(b"e")) {
            let after = after
                .strip_prefix(b"-")
                .or_else(|| after.strip_prefix(b"+"))
                .unwrap_or(after);

            exponent = true;

            if digits(after) == 0 || digits(after) != after.len() {
                return Err(BAD);
            }

            rest = &[];
        }

        let good = rest.is_empty()
            && whole + fraction > 0
            && match form {
                0x01 => !point && !exponent,
                0x02 => point && !exponent,
                0x03 => point && exponent,
                _ => return Err("A decimal REAL has a reserved form."),
            };

        if !good {
            return Err(BAD);
        }
    }

    let text: String = text
        .iter()
        .filter(|c| **c != b' ')
        .map(|c| if *c == b',' { '.' } else { char::from(*c) })
        .collect();

    match text.parse::<f64>() {
        Ok(value) if value.is_finite() => Ok(value),
        Ok(_) => Err("The REAL is too large for a float."),
        Err(_) => Err(BAD),
    }
}

/// A REAL (X.690 8.5), as the float nearest to it. `strict` takes only
/// the form of DER (X.690 11.3), which PER and OER have too.
pub(crate) fn decode_real(data: &[u8], offset: usize, strict: bool) -> Result<f64> {
    let Some((&control, rest)) = data.split_first() else {
        return Ok(0.0);
    };

    if control & 0x80 != 0 {
        binary_real(control, rest, strict)
    } else if control & 0x40 != 0 {
        // One octet only (X.690 8.5.9).
        match (control, rest.is_empty()) {
            (0x40, true) => Ok(f64::INFINITY),
            (0x41, true) => Ok(f64::NEG_INFINITY),
            (0x42, true) => Ok(f64::NAN),
            (0x43, true) => Ok(-0.0),
            (_, false) => Err("A special REAL has more than one octet."),
            _ => Err("Unsupported special REAL control word."),
        }
    } else {
        decimal_real(control, rest, strict)
    }
    .map_err(|what| Error::decode(what, offset))
}

fn padded_compare(a: &[u8], b: &[u8]) -> core::cmp::Ordering {
    let longest = a.len().max(b.len());

    (0..longest)
        .map(|i| {
            (
                a.get(i).copied().unwrap_or(0),
                b.get(i).copied().unwrap_or(0),
            )
        })
        .find(|(x, y)| x != y)
        .map_or(core::cmp::Ordering::Equal, |(x, y)| x.cmp(&y))
}

fn kind_has_base_tag(kind: &Kind) -> bool {
    !matches!(
        kind,
        Kind::Choice(_) | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType | Kind::Ref(_)
    )
}

/// Types whose contents can be in the tag with or without the
/// constructed bit.
fn is_primitive_or_constructed(kind: &Kind) -> bool {
    matches!(kind, Kind::BitString(_) | Kind::OctetString | Kind::Str(_))
}

fn allows_indefinite(kind: &Kind) -> bool {
    matches!(
        kind,
        Kind::Sequence(_) | Kind::Set(_) | Kind::External(_) | Kind::SequenceOf(_) | Kind::SetOf(_)
    )
}

struct Coder<'a> {
    types: &'a Types,
    der: bool,
    /// What DER requires of what it decodes.
    strict: bool,
}

impl<'a> Coder<'a> {
    fn new(types: &'a Types) -> Coder<'a> {
        let der = types.codec == Codec::Der;

        Coder {
            types,
            der,
            strict: der && !types.lenient,
        }
    }

    // Encoding.

    fn encode_type(
        &self,
        id: TypeId,
        value: &Value,
        out: &mut Vec<u8>,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        depth: usize,
    ) -> Result<()> {
        if depth > MAX_DEPTH {
            return encode_error("The value is nested too deeply.");
        }

        let node = self.types.node(id);

        self.encode_tags(&node.tags, node.concrete, value, out, siblings, depth)
    }

    fn encode_tags(
        &self,
        tags: &[WireTag],
        concrete: TypeId,
        value: &Value,
        out: &mut Vec<u8>,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        depth: usize,
    ) -> Result<()> {
        let kind = &self.types.node(concrete).kind;

        let Some((tag, rest)) = tags.split_first() else {
            return self.encode_untagged(kind, value, out, siblings, depth);
        };

        if rest.is_empty() && kind_has_base_tag(kind) {
            write_tlv(tag, out, |out| {
                self.encode_content(kind, concrete, value, siblings, out, depth)
            })
        } else {
            write_tlv(tag, out, |out| {
                self.encode_tags(rest, concrete, value, out, siblings, depth)
            })
        }
    }

    /// A type with no tag of its own: a CHOICE is its alternative.
    fn encode_untagged(
        &self,
        kind: &'a Kind,
        value: &Value,
        out: &mut Vec<u8>,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        depth: usize,
    ) -> Result<()> {
        match kind {
            Kind::Choice(members) => {
                let Value::Choice(name, inner) = value else {
                    return encode_error("Expected data of type tuple(str, object).");
                };
                let Some(member) = members.all().find(|member| &member.name == name) else {
                    return encode_error("Expected a name of an alternative of the CHOICE.");
                };

                self.encode_type(member.ty, inner, out, None, depth + 1)
                    .map_err(|error| error.in_member(name))
            }
            Kind::Any => match value {
                Value::Bytes(bytes) => {
                    out.extend_from_slice(bytes);

                    Ok(())
                }
                _ => encode_error("Expected data of type bytes."),
            },
            Kind::AnyDefinedBy {
                value: member,
                choices,
            } => {
                if choices.is_empty() {
                    return match value {
                        Value::Bytes(bytes) => {
                            out.extend_from_slice(bytes);

                            Ok(())
                        }
                        _ => encode_error("Expected data of type bytes."),
                    };
                }

                let key = siblings
                    .and_then(|siblings| siblings.iter().find(|(name, _)| name == member))
                    .map(|(_, key)| key);
                let choice = key.and_then(|key| {
                    choices
                        .iter()
                        .find(|(candidate, _)| const_matches(candidate, key))
                });

                match choice {
                    Some((_, ty)) => self.encode_type(*ty, value, out, None, depth + 1),
                    None => encode_error("Bad AnyDefinedBy choice."),
                }
            }
            _ => encode_error("The type has no tag."),
        }
    }

    /// The contents of a value of a type with a tag of its own.
    fn encode_content(
        &self,
        kind: &'a Kind,
        id: TypeId,
        value: &Value,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        out: &mut Vec<u8>,
        depth: usize,
    ) -> Result<()> {
        match (kind, value) {
            (Kind::Sequence(members) | Kind::External(members) | Kind::Set(members), _) => {
                self.encode_members(members, matches!(kind, Kind::Set(_)), value, out, depth)
            }
            (Kind::SequenceOf(element) | Kind::SetOf(element), _) => {
                self.encode_elements(*element, matches!(kind, Kind::SetOf(_)), value, out, depth)
            }
            (Kind::OctetString, Value::Bytes(bytes)) => {
                out.extend_from_slice(bytes);

                Ok(())
            }
            (Kind::ObjectIdentifier, Value::ObjectIdentifier(oid)) => {
                out.extend_from_slice(oid.as_bytes());

                Ok(())
            }
            _ => {
                out.extend_from_slice(&self.encode_primitive(kind, id, value, siblings)?);

                Ok(())
            }
        }
    }

    /// The contents of a value of a type with no others in it.
    fn encode_primitive(
        &self,
        kind: &'a Kind,
        id: TypeId,
        value: &Value,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
    ) -> Result<Vec<u8>> {
        let _ = siblings;

        match kind {
            Kind::Boolean => match value {
                Value::Boolean(value) => Ok(alloc::vec![if *value { 0xff } else { 0x00 }]),
                _ => encode_error("Expected data of type bool."),
            },
            Kind::Integer(named) => match value {
                Value::Integer(number) => Ok(integer_to_twos_complement(*number)),
                Value::BigInteger(bytes) => Ok(bytes.clone()),
                Value::Text(name) => match named.iter().find(|(candidate, _)| candidate == name) {
                    Some((_, number)) => Ok(integer_to_twos_complement(*number)),
                    None => encode_error("Expected an integer or a named number."),
                },
                _ => encode_error("Expected data of type int or str."),
            },
            Kind::Real => match value {
                Value::Real(value) => encode_real(*value),
                Value::Integer(value) => encode_real(*value as f64),
                Value::BigInteger(bytes) => encode_real(twos_complement_to_f64(bytes)?),
                _ => encode_error("Expected data of type float or int."),
            },
            Kind::Null => match value {
                Value::Null => Ok(Vec::new()),
                _ => encode_error("Expected None."),
            },
            Kind::BitString(named) => {
                let Value::Bits(bytes, bits) = value else {
                    return encode_error("Expected data of type tuple(bytes, int).");
                };

                if bytes.len() * 8 < *bits {
                    return encode_error("Expected more bits of data.");
                }

                // X.690 11.2.2: trailing zero bits of a named bit list
                // are left out.
                let (bytes, bits) = if self.der && !named.is_empty() {
                    clean_bits(bytes, *bits, true)
                } else {
                    (Vec::from(&bytes[..]), *bits)
                };
                let whole = bits / 8;
                let rest = bits % 8;
                let mut out = Vec::with_capacity(whole + 2);

                if rest == 0 {
                    out.push(0);
                    out.extend_from_slice(&bytes[..whole]);
                } else {
                    out.push((8 - rest) as u8);
                    out.extend_from_slice(&bytes[..whole]);
                    out.push(bytes[whole] & !(0xffu8 >> rest));
                }

                Ok(out)
            }
            Kind::OctetString => match value {
                Value::Bytes(bytes) => Ok(Vec::from(&bytes[..])),
                _ => encode_error("Expected data of type bytes or bytearray."),
            },
            Kind::ObjectIdentifier => match value {
                Value::ObjectIdentifier(oid) => Ok(Vec::from(oid.as_bytes())),
                _ => encode_error("Expected an OBJECT IDENTIFIER."),
            },
            Kind::Enumerated { root, additions } => {
                let number = match value {
                    Value::Text(name) if !self.types.numeric_enums => root
                        .iter()
                        .chain(additions.iter().flatten())
                        .find(|(candidate, _)| candidate == name)
                        .map(|(_, number)| *number),
                    Value::Integer(number) if self.types.numeric_enums => root
                        .iter()
                        .chain(additions.iter().flatten())
                        .find(|(_, candidate)| i128::from(*candidate) == *number)
                        .map(|(_, number)| *number),
                    _ => return encode_error("Expected an enumeration value."),
                };

                match number {
                    Some(number) => Ok(integer_to_twos_complement(i128::from(number))),
                    None => encode_error("Expected an enumeration value."),
                }
            }
            Kind::Str(string) => match value {
                Value::Text(text) => string_bytes(*string, text),
                _ => encode_error("Expected data of type str."),
            },
            Kind::Utc => match value {
                Value::DateTime(value) => {
                    Ok(time::utc_time_to_string(value, self.der).into_bytes())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => {
                    Ok(time::generalized_time_to_string(value, self.der).into_bytes())
                }
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Date => match value {
                Value::Date(value) => Ok(time::date_to_string(value).into_bytes()),
                _ => encode_error("Expected data of type datetime.date."),
            },
            Kind::TimeOfDay => match value {
                Value::Time(value) => Ok(time::time_of_day_to_string(value)?.into_bytes()),
                _ => encode_error("Expected data of type datetime.time."),
            },
            Kind::DateTime => match value {
                Value::DateTime(value) => Ok(time::date_time_to_string(value)?.into_bytes()),
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Duration => match value {
                Value::Text(text) => {
                    time::validate_duration(text)?;
                    let canonical = if self.der {
                        time::duration_to_canonical(text)?
                    } else {
                        String::from(&text[..])
                    };
                    Ok(canonical.as_bytes()[1..].to_vec())
                }
                _ => encode_error("Expected data of type str."),
            },
            // See `encode_content`.
            Kind::Sequence(_)
            | Kind::External(_)
            | Kind::Set(_)
            | Kind::SequenceOf(_)
            | Kind::SetOf(_)
            | Kind::Choice(_)
            | Kind::Any
            | Kind::AnyDefinedBy { .. }
            | Kind::OpenType
            | Kind::Ref(_) => {
                let _ = id;

                encode_error("The type has no contents of its own.")
            }
        }
    }

    fn encode_elements(
        &self,
        element: TypeId,
        is_set: bool,
        value: &Value,
        out: &mut Vec<u8>,
        depth: usize,
    ) -> Result<()> {
        let Value::List(items) = value else {
            return encode_error("Expected data of type list.");
        };

        if !(self.der && is_set) {
            for (index, item) in items.iter().enumerate() {
                self.encode_type(element, item, out, None, depth + 1)
                    .map_err(|error| error.in_element(index))?;
            }

            return Ok(());
        }

        let mut encoded = Vec::with_capacity(items.len());

        for (index, item) in items.iter().enumerate() {
            let mut one = Vec::new();

            self.encode_type(element, item, &mut one, None, depth + 1)
                .map_err(|error| error.in_element(index))?;
            encoded.push(one);
        }

        // X.690 11.6: sorted as octet strings, the shorter padded with
        // zero octets.
        encoded.sort_by(|a, b| padded_compare(a, b));

        for one in encoded {
            out.extend_from_slice(&one);
        }

        Ok(())
    }

    fn encode_members(
        &self,
        members: &'a Members,
        is_set: bool,
        value: &Value,
        out: &mut Vec<u8>,
        depth: usize,
    ) -> Result<()> {
        let Value::Map(map) = value else {
            return encode_error("Expected data of type dict.");
        };
        // DER sorts the members of a SET: where each ends.
        let sorted = is_set && self.der;
        let start = out.len();
        let mut ends = Vec::new();
        let mut lookup = Lookup::new(map);

        if is_set {
            for index in &members.order {
                self.encode_member(&members.root[*index], &mut lookup, out, depth)?;

                if sorted {
                    ends.push(out.len());
                }
            }
        } else {
            for member in &members.root {
                self.encode_member(member, &mut lookup, out, depth)?;
            }
        }

        // An addition that can not be encoded ends the additions.
        'additions: for addition in members.additions.iter().flatten() {
            let group = match addition {
                Addition::Member(member) => core::slice::from_ref(member),
                Addition::Group(group) => group.as_slice(),
            };
            let before = out.len();

            for member in group {
                match self.encode_member(member, &mut lookup, out, depth) {
                    Ok(()) => {}
                    Err(Error::Encode(..)) => {
                        out.truncate(before);
                        ends.retain(|end| *end <= before);

                        break 'additions;
                    }
                    Err(error) => return Err(error),
                }

                if sorted {
                    ends.push(out.len());
                }
            }
        }

        // DER writes the members of a SET in the order of the tags they
        // are encoded with, which for an untagged CHOICE is that of the
        // alternative there (X.690 10.3).
        if sorted {
            let encoded = out.split_off(start);
            let mut parts = Vec::with_capacity(ends.len());
            let mut from = 0;

            for end in ends {
                parts.push(&encoded[from..end - start]);
                from = end - start;
            }

            parts.retain(|part| !part.is_empty());
            parts.sort_by_key(|part| {
                read_tag(part, 0).map_or((TagClass::Private, u64::MAX), |(class, _, number, _)| {
                    (class, number)
                })
            });

            for part in parts {
                out.extend_from_slice(part);
            }
        }

        Ok(())
    }

    fn encode_member(
        &self,
        member: &MemberInfo,
        lookup: &mut Lookup<'_, '_>,
        out: &mut Vec<u8>,
        depth: usize,
    ) -> Result<()> {
        match lookup.find(&member.name) {
            Some(value) => {
                if !self.is_default(member, value) {
                    self.encode_type(member.ty, value, out, Some(lookup.map), depth + 1)
                        .map_err(|error| error.in_member(&member.name))?;
                }

                Ok(())
            }
            None if member.optional || member.has_default() => Ok(()),
            None => encode_error("A member is not found in the SEQUENCE or SET."),
        }
    }

    fn is_default(&self, member: &MemberInfo, value: &Value) -> bool {
        is_default(self.types, member, value)
    }

    // Decoding.

    fn indefinite_ok(&self, kind: &'a Kind) -> bool {
        !self.strict && allows_indefinite(kind)
    }

    /// Decodes the type `id` at `offset`, or says its tag is not the
    /// one there.
    fn decode_type(
        &self,
        id: TypeId,
        data: &'a [u8],
        offset: usize,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        depth: usize,
    ) -> Result<Option<(Value<'a>, usize)>> {
        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.", offset);
        }

        let node = self.types.node(id);

        self.decode_tags(
            &node.tags,
            node.concrete,
            data,
            offset,
            siblings,
            true,
            depth,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn decode_tags(
        &self,
        tags: &[WireTag],
        concrete: TypeId,
        data: &'a [u8],
        offset: usize,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        outer: bool,
        depth: usize,
    ) -> Result<Option<(Value<'a>, usize)>> {
        let kind = &self.types.node(concrete).kind;

        let Some((tag, rest)) = tags.split_first() else {
            return self.decode_untagged(kind, data, offset, siblings, depth);
        };

        let (class, constructed, number, after_tag) = read_tag(data, offset)?;
        let is_base = rest.is_empty() && kind_has_base_tag(kind);
        let constructed_ok = if is_base && is_primitive_or_constructed(kind) {
            true
        } else {
            constructed == tag.constructed
        };

        if class != tag.class || number != tag.number || !constructed_ok {
            if outer {
                return Ok(None);
            }

            return decode_error("Expected another tag.", offset);
        }

        if is_base {
            let alternatives = is_primitive_or_constructed(kind);
            let (length, start) = read_length(
                data,
                after_tag,
                !(self.indefinite_ok(kind) || alternatives && !self.strict),
                self.strict,
            )?;

            return if alternatives && constructed {
                self.decode_constructed(kind, data, start, length, depth)
                    .map(Some)
            } else if let Kind::Sequence(members) | Kind::External(members) | Kind::Set(members) =
                kind
            {
                let end = length.map(|length| start + length);

                self.decode_members(
                    members,
                    matches!(kind, Kind::Set(_)),
                    data,
                    start,
                    end,
                    depth,
                )
                .map(Some)
            } else if let Kind::SequenceOf(element) | Kind::SetOf(element) = kind {
                let end = length.map(|length| start + length);

                self.decode_array(
                    *element,
                    matches!(kind, Kind::SetOf(_)),
                    data,
                    start,
                    end,
                    depth,
                )
                .map(Some)
            } else {
                let length =
                    length.ok_or_else(|| Error::decode("Expected definite length.", after_tag))?;
                let value =
                    self.decode_content(kind, concrete, data, start, length, siblings, depth)?;

                Ok(Some((value, start + length)))
            };
        }

        // An explicit tag around the rest.
        let (length, start) = read_length(data, after_tag, self.strict, self.strict)?;
        let Some((value, mut end)) =
            self.decode_tags(rest, concrete, data, start, siblings, false, depth)?
        else {
            return decode_error("Expected another tag.", start);
        };

        if length.is_none() {
            if data.get(end..end + 2) != Some(&[0, 0]) {
                return decode_error("Expected end-of-contents tag.", end);
            }

            end += 2;
        }

        Ok(Some((value, end)))
    }

    fn decode_untagged(
        &self,
        kind: &'a Kind,
        data: &'a [u8],
        offset: usize,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        depth: usize,
    ) -> Result<Option<(Value<'a>, usize)>> {
        match kind {
            Kind::Choice(members) => {
                let (class, _, number, _) = read_tag(data, offset)?;

                for member in members.all() {
                    if self.types.accepts(member.ty, class, number) {
                        return match self
                            .decode_type(member.ty, data, offset, None, depth + 1)
                            .map_err(|error| error.in_member(&member.name))?
                        {
                            Some((value, end)) => Ok(Some((
                                Value::Choice(Cow::Borrowed(member.name.as_str()), value.into()),
                                end,
                            ))),
                            None => Ok(None),
                        };
                    }
                }

                if members.additions.is_some() {
                    // An alternative of a later version: skipped.
                    let start = skip_tag(data, offset)?;
                    let (length, start) = read_length(data, start, true, false)?;
                    let end = start + length.unwrap_or(0);

                    return Ok(Some((
                        Value::Choice(Cow::Borrowed(""), Value::Null.into()),
                        end,
                    )));
                }

                Ok(None)
            }
            Kind::Any => self.decode_any(data, offset).map(Some),
            Kind::AnyDefinedBy { value, choices } => {
                if choices.is_empty() {
                    return self.decode_any(data, offset).map(Some);
                }

                let key = siblings
                    .and_then(|siblings| siblings.iter().find(|(name, _)| name == value))
                    .map(|(_, key)| key);
                let choice = key.and_then(|key| {
                    choices
                        .iter()
                        .find(|(candidate, _)| const_matches(candidate, key))
                });

                match choice {
                    Some((_, ty)) => self.decode_type(*ty, data, offset, None, depth + 1),
                    None => decode_error("Bad AnyDefinedBy choice.", offset),
                }
            }
            _ => decode_error("The type has no tag.", offset),
        }
    }

    fn decode_any(&self, data: &'a [u8], offset: usize) -> Result<(Value<'a>, usize)> {
        let start = skip_tag(data, offset)?;
        let (length, start) = read_length(data, start, true, false)?;
        let end = start + length.unwrap_or(0);

        Ok((Value::Bytes(Cow::Borrowed(&data[offset..end])), end))
    }

    #[allow(clippy::too_many_arguments)]
    fn decode_content(
        &self,
        kind: &'a Kind,
        id: TypeId,
        data: &'a [u8],
        offset: usize,
        length: usize,
        siblings: Option<&[(Cow<'_, str>, Value<'_>)]>,
        _depth: usize,
    ) -> Result<Value<'a>> {
        let _ = (id, siblings);
        let end = offset + length;
        let contents = &data[offset..end];

        match kind {
            Kind::Boolean => {
                if length != 1 {
                    return decode_error("Expected BOOLEAN contents length 1.", offset);
                }

                if self.strict && contents[0] != 0x00 && contents[0] != 0xff {
                    return decode_error("Expected BOOLEAN contents of 0x00 or 0xff.", offset);
                }

                Ok(Value::Boolean(contents[0] != 0))
            }
            Kind::Integer(_) => {
                self.check_minimal_integer(contents, offset)?;

                Ok(Value::from_twos_complement(contents))
            }
            Kind::Real => decode_real(contents, offset, self.strict).map(Value::Real),
            Kind::Null => {
                if length != 0 {
                    return decode_error("Expected NULL contents length 0.", offset);
                }

                Ok(Value::Null)
            }
            Kind::BitString(named) => self.decode_bit_string(contents, !named.is_empty(), offset),
            Kind::OctetString => Ok(Value::Bytes(Cow::Borrowed(contents))),
            Kind::ObjectIdentifier => {
                ObjectIdentifier::decode(Cow::Borrowed(contents), offset, self.strict)
                    .map(Value::ObjectIdentifier)
            }
            Kind::Enumerated { root, additions } => {
                self.check_minimal_integer(contents, offset)?;

                let Value::Integer(number) = Value::from_twos_complement(contents) else {
                    return decode_error("Expected an enumeration value.", offset);
                };
                let found = root
                    .iter()
                    .chain(additions.iter().flatten())
                    .find(|(_, candidate)| i128::from(*candidate) == number);

                match found {
                    Some((name, _)) if self.types.numeric_enums => {
                        let _ = name;

                        Ok(Value::Integer(number))
                    }
                    Some((name, _)) => Ok(Value::Text(Cow::Borrowed(name))),
                    None if additions.is_some() => Ok(Value::Null),
                    None => decode_error("Expected an enumeration value.", offset),
                }
            }
            Kind::Sequence(_)
            | Kind::External(_)
            | Kind::Set(_)
            | Kind::SequenceOf(_)
            | Kind::SetOf(_) => decode_error("Members are decoded elsewhere.", offset),
            Kind::Str(string) => string_text(*string, contents, offset).map(Value::Text),
            // DER has one form of each (X.690 11.7, 11.8). A lenient
            // reader also takes any other BER has, as the same time in UTC.
            Kind::Utc => {
                let text = ascii(contents, offset)?;
                let value = time::utc_time_from_string(text, self.der);

                match value {
                    Err(_) if self.der && !self.strict => {
                        time::utc_time_from_string(text, false).map(|value| time::to_utc(&value))
                    }
                    _ => value,
                }
                .map(Value::DateTime)
            }
            Kind::Generalized => {
                let text = ascii(contents, offset)?;
                let value = time::generalized_time_from_string(text, self.der);

                match value {
                    Err(_) if self.der && !self.strict => {
                        time::generalized_time_from_string(text, false)
                            .map(|value| time::to_utc(&value))
                    }
                    _ => value,
                }
                .map(Value::DateTime)
            }
            Kind::Date => time::date_from_string(ascii(contents, offset)?).map(Value::Date),
            Kind::TimeOfDay => {
                time::time_of_day_from_string(ascii(contents, offset)?).map(Value::Time)
            }
            Kind::DateTime => {
                time::date_time_from_string(ascii(contents, offset)?).map(Value::DateTime)
            }
            Kind::Duration => {
                let raw = ascii(contents, offset)?;
                let mut text = String::from("P");
                text.push_str(raw);
                time::validate_duration(&text)
                    .map_err(|_| Error::decode("Invalid DURATION.", offset))?;
                if self.der && self.strict {
                    let canonical = time::duration_to_canonical(&text)
                        .map_err(|_| Error::decode("Invalid DURATION.", offset))?;
                    if canonical != text {
                        return decode_error("The DURATION is not in canonical form.", offset);
                    }
                }
                Ok(Value::Text(Cow::Owned(text)))
            }
            Kind::Choice(_)
            | Kind::Any
            | Kind::AnyDefinedBy { .. }
            | Kind::OpenType
            | Kind::Ref(_) => decode_error("The type has no contents of its own.", offset),
        }
    }

    fn check_minimal_integer(&self, contents: &[u8], offset: usize) -> Result<()> {
        if !self.strict {
            return Ok(());
        }

        if contents.is_empty() {
            return decode_error("Expected at least one byte of an INTEGER.", offset);
        }

        if contents.len() > 1
            && ((contents[0] == 0x00 && contents[1] < 0x80)
                || (contents[0] == 0xff && contents[1] >= 0x80))
        {
            return decode_error("Expected an INTEGER in as few bytes as possible.", offset);
        }

        Ok(())
    }

    fn decode_bit_string(
        &self,
        contents: &'a [u8],
        named: bool,
        offset: usize,
    ) -> Result<Value<'a>> {
        let Some((&unused, bytes)) = contents.split_first() else {
            return decode_error("Expected at least one byte of a BIT STRING.", offset);
        };

        if unused > 7 || (bytes.is_empty() && unused != 0) {
            return decode_error("Expected at most 7 unused bits of a BIT STRING.", offset);
        }

        let bits = 8 * bytes.len() - usize::from(unused);

        if self.strict {
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

        Ok(Value::Bits(Cow::Borrowed(bytes), bits))
    }

    /// A string in parts, the way BER allows.
    fn decode_constructed(
        &self,
        kind: &'a Kind,
        data: &'a [u8],
        start: usize,
        length: Option<usize>,
        depth: usize,
    ) -> Result<(Value<'a>, usize)> {
        if self.strict {
            return decode_error("Expected a string that is not constructed.", start);
        }

        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.", start);
        }

        // The parts have the universal tag of the type, also if the
        // type is tagged otherwise: those of a BIT STRING are BIT
        // STRINGs, and the others are OCTET STRINGs.
        let segment_tag = WireTag {
            class: TagClass::Universal,
            number: if matches!(kind, Kind::BitString(_)) {
                3
            } else {
                4
            },
            constructed: false,
        };
        let segment_kind = match kind {
            Kind::Str(_) => &Kind::OctetString,
            other => other,
        };
        let end = length.map(|length| start + length);
        let mut at = start;
        let mut segments = Vec::new();

        loop {
            match end {
                Some(end) if at >= end => break,
                None => {
                    if data.get(at..at + 2).is_none() {
                        return decode_error(
                            "Ran out of data when trying to find End of Contents tag.",
                            at,
                        );
                    }

                    if data[at..at + 2] == [0, 0] {
                        at += 2;

                        break;
                    }
                }
                _ => {}
            }

            let (class, constructed, number, after_tag) = read_tag(data, at)?;

            if class != segment_tag.class || number != segment_tag.number {
                return decode_error("Expected another tag.", at);
            }

            let (inner_length, inner_start) = read_length(data, after_tag, false, false)?;

            if constructed {
                let (value, next) = self.decode_constructed(
                    segment_kind,
                    data,
                    inner_start,
                    inner_length,
                    depth + 1,
                )?;

                segments.push(value);
                at = next;
            } else {
                let inner_length = inner_length
                    .ok_or_else(|| Error::decode("Expected definite length.", after_tag))?;
                let contents = &data[inner_start..inner_start + inner_length];

                segments.push(match segment_kind {
                    Kind::BitString(named) => {
                        self.decode_bit_string(contents, !named.is_empty(), inner_start)?
                    }
                    _ => Value::Bytes(Cow::Borrowed(contents)),
                });
                at = inner_start + inner_length;
            }
        }

        let value = match kind {
            Kind::BitString(_) => {
                let mut bytes = Vec::new();
                let mut bits = 0;

                for segment in segments {
                    if let Value::Bits(part, count) = segment {
                        // Only the last part has unused bits (X.690 8.6.4).
                        if bits % 8 != 0 {
                            return decode_error(
                                "Expected unused bits only in the last part of a BIT STRING.",
                                start,
                            );
                        }

                        bytes.extend_from_slice(&part);
                        bits += count;
                    }
                }

                Value::Bits(Cow::Owned(bytes), bits)
            }
            Kind::Str(string) => {
                let mut bytes = Vec::new();

                for segment in segments {
                    if let Value::Bytes(part) = segment {
                        bytes.extend_from_slice(&part);
                    }
                }

                Value::Text(Cow::Owned(
                    string_text(*string, &bytes, start)?.into_owned(),
                ))
            }
            _ => {
                let mut bytes = Vec::new();

                for segment in segments {
                    if let Value::Bytes(part) = segment {
                        bytes.extend_from_slice(&part);
                    }
                }

                Value::Bytes(Cow::Owned(bytes))
            }
        };

        Ok((value, at))
    }

    /// The elements of a SEQUENCE OF or a SET OF, up to `end` or the
    /// end-of-contents octets.
    fn decode_array(
        &self,
        element: TypeId,
        is_set: bool,
        data: &'a [u8],
        offset: usize,
        end: Option<usize>,
        depth: usize,
    ) -> Result<(Value<'a>, usize)> {
        let mut items = Vec::new();
        let mut at = offset;
        let mut previous: Option<&[u8]> = None;

        loop {
            match end {
                Some(end) if at >= end => break,
                Some(_) => {}
                None => match data.get(at..at + 2) {
                    Some([0, 0]) => {
                        at += 2;

                        break;
                    }
                    Some(_) => {}
                    None => {
                        return decode_error(
                            "Ran out of data when trying to find End of Contents tag.",
                            at,
                        );
                    }
                },
            }

            let start = at;
            let Some((item, next)) = self
                .decode_type(element, data, at, None, depth + 1)
                .map_err(|error| error.in_element(items.len()))?
            else {
                return decode_error("Expected the tag of the element.", at);
            };

            at = next;

            if self.strict && is_set {
                let encoded = &data[start..at];

                if let Some(previous) = previous
                    && padded_compare(previous, encoded) == core::cmp::Ordering::Greater
                {
                    return decode_error(
                        "Expected the elements of a SET OF in ascending order of their encodings.",
                        start,
                    );
                }

                previous = Some(encoded);
            }

            items.push(item);
        }

        Ok((Value::List(items), at))
    }

    /// The members of a SEQUENCE or a SET, from `offset` up to `end`
    /// or, if there is none, the end-of-contents octets. Returns the
    /// value and the offset after it.
    fn decode_members(
        &self,
        members: &'a Members,
        is_set: bool,
        data: &'a [u8],
        offset: usize,
        end: Option<usize>,
        depth: usize,
    ) -> Result<(Value<'a>, usize)> {
        let mut values: Vec<(Cow<'a, str>, Value<'a>)> = Vec::with_capacity(members.root.len());
        let additions =
            || -> Vec<&'a MemberInfo> { members.all().skip(members.root.len()).collect() };
        let (at, out_of_data) = if is_set {
            // The members of a SET are in any order, the additions among
            // them, and DER has them in the order of their tags.
            let mut all: Vec<&'a MemberInfo> = members
                .order
                .iter()
                .map(|index| &members.root[*index])
                .collect();
            let additions_from = all.len();

            all.extend(additions());

            self.decode_member_list(
                all.iter().copied(),
                additions_from,
                true,
                data,
                offset,
                end,
                &mut values,
                depth,
            )?
        } else {
            let (mut at, mut out_of_data) = self.decode_member_list(
                members.root.iter(),
                members.root.len(),
                false,
                data,
                offset,
                end,
                &mut values,
                depth,
            )?;

            // The additions, even when out of data, so that defaults are
            // added.
            if members.additions.is_some() {
                (at, out_of_data) = self.decode_member_list(
                    additions().iter().copied(),
                    0,
                    false,
                    data,
                    at,
                    end,
                    &mut values,
                    depth,
                )?;
            }

            (at, out_of_data)
        };

        if out_of_data {
            return Ok((Value::Map(values), at));
        }

        match end {
            None => decode_error(
                "Could not find end-of-contents tag for indefinite length field.",
                at,
            ),
            // Extra data is allowed in cases of versioned additions.
            Some(end) => Ok((Value::Map(values), end)),
        }
    }

    fn is_end(&self, data: &[u8], offset: usize, end: Option<usize>) -> Result<(bool, usize)> {
        match end {
            Some(end) => Ok((offset >= end, offset)),
            None => match data.get(offset..offset + 2) {
                Some([0, 0]) => Ok((true, offset + 2)),
                Some(_) => Ok((false, offset)),
                None => decode_error(
                    "Ran out of data when trying to find End of Contents tag.",
                    offset,
                ),
            },
        }
    }

    #[allow(clippy::too_many_arguments)]
    /// Decodes `members`, of which those from `additions_from` on are
    /// additions, which may be missing. With `by_tags`, the members of a
    /// SET, DER has them in the order of their tags, and else in the
    /// order they are declared in.
    fn decode_member_list<I>(
        &self,
        members: I,
        additions_from: usize,
        by_tags: bool,
        data: &'a [u8],
        offset: usize,
        end: Option<usize>,
        values: &mut Vec<(Cow<'a, str>, Value<'a>)>,
        depth: usize,
    ) -> Result<(usize, bool)>
    where
        I: ExactSizeIterator<Item = &'a MemberInfo> + Clone,
    {
        let mut decoded = Decoded::new(members.len());
        let mut at = offset;
        let mut success_earlier = false;
        let mut last_tag: Option<(TagClass, u64)> = None;
        let (mut out_of_data, next) = self.is_end(data, at, end)?;

        at = next;

        // More than one pass lets the members be in any order.
        loop {
            let mut success = false;

            for (index, member) in members.clone().enumerate() {
                if out_of_data || decoded.has(index) {
                    continue;
                }

                let start = at;

                if let Some((value, next)) = self
                    .decode_type(member.ty, data, at, Some(values), depth + 1)
                    .map_err(|error| error.in_member(&member.name))?
                {
                    if self.strict && by_tags {
                        let (class, _, number, _) = read_tag(data, start)?;

                        if last_tag.is_some_and(|last| last >= (class, number)) {
                            return decode_error(
                                "The members of a SET are not in the order of their tags.",
                                start,
                            );
                        }

                        last_tag = Some((class, number));
                    } else if success_earlier && self.strict {
                        return decode_error(
                            "A member is not in the order it is declared in.",
                            next,
                        );
                    }

                    // A member with its default is left out in DER.
                    if self.strict && member.has_default() && self.is_default(member, &value) {
                        return decode_error(
                            "A member is encoded with its default value, which DER leaves out.",
                            next,
                        );
                    }

                    success = true;
                    at = next;
                    decoded.set(index);
                    values.push((Cow::Borrowed(member.name.as_str()), value));
                }

                (out_of_data, at) = self.is_end(data, at, end)?;
            }

            success_earlier = success_earlier || success;

            if out_of_data || !success {
                break;
            }
        }

        for (index, member) in members.enumerate() {
            if decoded.has(index) || member.optional {
                continue;
            }

            if let Some(default) = &member.default {
                values.push((Cow::Borrowed(member.name.as_str()), default.reborrow()));
            } else if index >= additions_from {
                break;
            } else if out_of_data {
                return decode_error("A member is missing and has no default value.", at);
            } else {
                return decode_error("Expected the tag of a member.", at);
            }
        }

        Ok((at, out_of_data))
    }
}

/// Which members are decoded, with no allocation for up to 128 of them.
struct Decoded {
    first: u128,
    rest: Vec<bool>,
}

impl Decoded {
    fn new(count: usize) -> Decoded {
        Decoded {
            first: 0,
            rest: alloc::vec![false; count.saturating_sub(128)],
        }
    }

    fn has(&self, index: usize) -> bool {
        match index.checked_sub(128) {
            None => self.first >> index & 1 == 1,
            Some(index) => self.rest[index],
        }
    }

    fn set(&mut self, index: usize) {
        match index.checked_sub(128) {
            None => self.first |= 1 << index,
            Some(index) => self.rest[index] = true,
        }
    }
}

fn ascii(contents: &[u8], offset: usize) -> Result<&str> {
    if !contents.is_ascii() {
        return decode_error("The string is not ASCII.", offset);
    }

    core::str::from_utf8(contents).map_err(|_| Error::decode("The string is not ASCII.", offset))
}
