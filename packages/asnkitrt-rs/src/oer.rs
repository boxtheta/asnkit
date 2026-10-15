// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Octet Encoding Rules (OER), X.696.
//!
//! It does what the runtime for Python does, also where that is not all
//! of the standard.

use alloc::string::String;
use alloc::vec::Vec;

use crate::ber::{
    decode_object_identifier, decode_real, encode_object_identifier, encode_real,
    twos_complement_to_f64,
};
use crate::node::TagClass;
use crate::per::{Reader, Writer};
use crate::time;
use crate::types::{
    Addition, Kind, MemberInfo, Members, RealFormat, StrKind, TypeId, Types, clean_bits, is_default,
};
use crate::value::{Date, DateTime, Time, Value, integer_to_twos_complement};
use crate::{Error, Result};

const MAX_DEPTH: usize = 128;

fn encode_error<T>(what: &'static str) -> Result<T> {
    Err(Error::Encode(what))
}

fn decode_error<T>(what: &'static str, position: usize) -> Result<T> {
    Err(Error::Decode(what, position))
}

pub fn encode(types: &Types, id: TypeId, value: &Value) -> Result<Vec<u8>> {
    let mut w = Writer::new(false);

    Coder { types }.encode_type(id, value, &mut w, 0)?;

    Ok(w.bytes)
}

pub fn decode(types: &Types, id: TypeId, data: &[u8]) -> Result<Value> {
    let mut r = Reader {
        data,
        position: 0,
        aligned: true,
        budget: 8 * data.len() + 1024,
    };

    Coder { types }.decode_type(id, &mut r, 0)
}

// The length of a field.

fn put_length(w: &mut Writer, length: usize) -> Result<()> {
    if length < 128 {
        w.uint(length as u128, 8);
    } else {
        let bytes = length.to_be_bytes();
        let skip = bytes.iter().take_while(|byte| **byte == 0).count();

        w.uint(0x80 | (bytes.len() - skip) as u128, 8);
        w.bytes(&bytes[skip..]);
    }

    Ok(())
}

fn get_length(r: &mut Reader<'_>) -> Result<usize> {
    let value = r.uint(8)? as usize;

    if value & 0x80 == 0 {
        return Ok(value);
    }

    let count = value & 0x7f;

    if count > core::mem::size_of::<usize>() {
        return decode_error("A length is too large.", r.position);
    }

    Ok(r.uint(8 * count)? as usize)
}

fn put_integer(w: &mut Writer, bytes: &[u8]) -> Result<()> {
    put_length(w, bytes.len())?;
    w.bytes(bytes);

    Ok(())
}

fn get_integer(r: &mut Reader<'_>) -> Result<Value> {
    let length = get_length(r)?;
    let bytes = r.bytes(length)?;

    Ok(Value::from_twos_complement(&bytes))
}

fn put_unsigned(w: &mut Writer, value: u128) -> Result<()> {
    let bits = (128 - value.leading_zeros()).max(1) as usize;
    let count = bits.div_ceil(8);

    put_length(w, count)?;
    w.uint(value, 8 * count);

    Ok(())
}

/// The range of an integer as OER sees it, which is that of `i128` that
/// gives the same encoding if a bound does not fit in an `i128`.
fn oer_range(node: &crate::types::TypeNode) -> Option<crate::types::Range> {
    #[cfg(feature = "bigint")]
    if let Some(range) = &node.effective_big_range {
        return Some(range.as_oer_range());
    }

    node.effective_range
}

/// A time element of DURATION-INTERVAL-ENCODING, which is an INTEGER
/// (0..MAX) (X.696 29.4.1): a `Value::Integer`, or a `Value::BigInteger`
/// if it does not fit in an `i128`, which needs the feature `bigint`.
fn get_duration_field(r: &mut Reader<'_>) -> Result<Value> {
    let length = get_length(r)?;
    let bytes = r.bytes(length)?;
    let significant = &bytes[bytes.iter().take_while(|byte| **byte == 0).count()..];
    let value = significant.iter().try_fold(0i128, |value, byte| {
        value.checked_mul(256)?.checked_add(i128::from(*byte))
    });

    match value {
        Some(value) => Ok(Value::Integer(value)),
        #[cfg(feature = "bigint")]
        None => {
            let mut twos_complement = alloc::vec![0u8];

            twos_complement.extend_from_slice(significant);

            Ok(Value::from_twos_complement(&twos_complement))
        }
        #[cfg(not(feature = "bigint"))]
        None => Err(Error::Unsupported(time::ELEMENT_TOO_LARGE)),
    }
}

/// A time element of DURATION-INTERVAL-ENCODING.
fn put_duration_field(w: &mut Writer, value: &Value) -> Result<()> {
    match value {
        Value::Integer(number) if *number >= 0 => put_unsigned(w, *number as u128),
        // A positive number: drop the byte that only holds the sign.
        Value::BigInteger(bytes) if bytes.first().is_some_and(|byte| byte & 0x80 == 0) => {
            let start = usize::from(bytes[0] == 0 && bytes.len() > 1);

            put_integer(w, &bytes[start..])
        }
        _ => encode_error("Expected a number that is not negative."),
    }
}

fn get_unsigned(r: &mut Reader<'_>) -> Result<u128> {
    let length = get_length(r)?;

    if length > 16 {
        // Leading zeros are fine, anything else does not fit.
        let bytes = r.bytes(length)?;

        if bytes[..length - 16].iter().any(|byte| *byte != 0) {
            return decode_error("An unsigned integer is too large.", r.position);
        }

        return Ok(bytes[length - 16..]
            .iter()
            .fold(0u128, |value, byte| (value << 8) | u128::from(*byte)));
    }

    r.uint(8 * length)
}

/// A tag the way OER writes it: the class and the number.
fn put_tag(w: &mut Writer, class: TagClass, number: u64) {
    let flags: u8 = match class {
        TagClass::Universal => 0x00,
        TagClass::Application => 0x40,
        TagClass::ContextSpecific => 0x80,
        TagClass::Private => 0xc0,
    };

    if number < 63 {
        w.uint(u128::from(flags) | u128::from(number), 8);
    } else {
        w.uint(u128::from(flags | 0x3f), 8);

        let mut groups = Vec::new();
        let mut rest = number;

        while rest > 0 {
            groups.push(0x80 | (rest & 0x7f) as u8);
            rest >>= 7;
        }

        groups[0] &= 0x7f;
        groups.reverse();
        w.bytes(&groups);
    }
}

fn get_tag(r: &mut Reader<'_>) -> Result<(TagClass, u64)> {
    let first = r.uint(8)? as u8;
    let class = match first >> 6 {
        0 => TagClass::Universal,
        1 => TagClass::Application,
        2 => TagClass::ContextSpecific,
        _ => TagClass::Private,
    };
    let mut number = u64::from(first & 0x3f);

    if number == 0x3f {
        number = 0;

        loop {
            let byte = r.uint(8)? as u8;

            if number >> 57 != 0 {
                return decode_error("A tag number is too large.", r.position);
            }

            number = (number << 7) | u64::from(byte & 0x7f);

            if byte & 0x80 == 0 {
                break;
            }
        }
    }

    Ok((class, number))
}

struct Coder<'a> {
    types: &'a Types,
}

/// The octets of a character of a string of a known multiplier (X.696
/// 27.1), or `None` for the other strings.
fn known_multiplier(kind: StrKind) -> Option<usize> {
    match kind {
        StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => Some(1),
        StrKind::Bmp => Some(2),
        StrKind::Universal => Some(4),
        _ => None,
    }
}

/// What the size of a string, an OCTET STRING or a BIT STRING says: a
/// fixed size, if it has one.
fn fixed_size(range: Option<crate::types::Range>) -> Option<usize> {
    let range = range?;

    if range.extensible || range.min.is_none() && range.max.is_none() {
        return None;
    }

    if range.min == range.max {
        return range.min.and_then(|size| usize::try_from(size).ok());
    }

    None
}

impl Coder<'_> {
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
                    w.uint(if *value { 0xff } else { 0 }, 8);

                    Ok(())
                }
                _ => encode_error("Expected data of type bool."),
            },
            Kind::Integer(named) => {
                let number = match value {
                    Value::Text(name) => {
                        match named.iter().find(|(candidate, _)| candidate == name) {
                            Some((_, number)) => Value::Integer(*number),
                            None => return encode_error("Expected an integer or a named number."),
                        }
                    }
                    Value::Integer(_) | Value::BigInteger(_) => value.clone(),
                    _ => return encode_error("Expected data of type int or str."),
                };

                self.encode_integer(&number, oer_range(node), w)
            }
            Kind::Real => {
                let number = match value {
                    Value::Real(value) => *value,
                    Value::Integer(value) => *value as f64,
                    Value::BigInteger(bytes) => twos_complement_to_f64(bytes)?,
                    _ => return encode_error("Expected data of type float or int."),
                };

                match self.types.node(node.concrete).real {
                    None => {
                        let encoded = encode_real(number)?;

                        put_length(w, encoded.len())?;
                        w.bytes(&encoded);
                    }
                    Some(RealFormat::Binary32) => {
                        let narrow = number as f32;

                        if narrow.is_infinite() && number.is_finite() {
                            return encode_error(
                                "Expected an IEEE 754 32 bits floating point number.",
                            );
                        }

                        w.bytes(&narrow.to_be_bytes());
                    }
                    Some(RealFormat::Binary64) => w.bytes(&number.to_be_bytes()),
                }

                Ok(())
            }
            Kind::Null => match value {
                Value::Null => Ok(()),
                _ => encode_error("Expected None."),
            },
            Kind::BitString(_) => match value {
                Value::Bits(bytes, bits) => {
                    if bytes.len() * 8 < *bits {
                        return encode_error("Expected more bits of data.");
                    }

                    let (data, _) = clean_bits(bytes, *bits, false);
                    let unused = (8 - bits % 8) % 8;

                    if fixed_size(size).is_none() {
                        put_length(w, data.len() + 1)?;
                        w.uint(unused as u128, 8);
                    }

                    w.bytes(&data);

                    Ok(())
                }
                _ => encode_error("Expected data of type tuple(bytes, int)."),
            },
            Kind::OctetString => match value {
                Value::Bytes(bytes) => {
                    if fixed_size(size).is_none() {
                        put_length(w, bytes.len())?;
                    }

                    w.bytes(bytes);

                    Ok(())
                }
                _ => encode_error("Expected data of type bytes or bytearray."),
            },
            Kind::ObjectIdentifier => match value {
                Value::Text(text) => {
                    let encoded = encode_object_identifier(text)?;

                    put_length(w, encoded.len())?;
                    w.bytes(&encoded);

                    Ok(())
                }
                _ => encode_error("Expected data of type str."),
            },
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

                put_unsigned(w, items.len() as u128)?;

                for item in items {
                    self.encode_type(*element, item, w, depth + 1)?;
                }

                Ok(())
            }
            Kind::Str(string) => match value {
                Value::Text(text) => self.encode_string(*string, text, size, w),
                _ => encode_error("Expected data of type str."),
            },
            Kind::Utc => match value {
                Value::DateTime(value) => self.encode_string(
                    StrKind::Visible,
                    &time::utc_time_to_string(value, false),
                    None,
                    w,
                ),
                _ => encode_error("Expected data of type datetime.datetime."),
            },
            Kind::Generalized => match value {
                Value::DateTime(value) => self.encode_string(
                    StrKind::Visible,
                    &time::generalized_time_to_string(value, false),
                    None,
                    w,
                ),
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
            // An ANY DEFINED BY with choices is a member, see
            // `encode_member`.
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                encode_error("Bad AnyDefinedBy choice.")
            }
            // The encoding of the value, after its length (X.696 30). An
            // ANY is encoded as the open type that took its place.
            Kind::OpenType | Kind::Any | Kind::AnyDefinedBy { .. } => match value {
                Value::Bytes(bytes) => {
                    put_length(w, bytes.len())?;
                    w.bytes(bytes);

                    Ok(())
                }
                _ => encode_error("Expected data of type bytes."),
            },
            Kind::Ref(_) => encode_error("The type is a reference."),
        }
    }

    fn encode_integer(
        &self,
        number: &Value,
        range: Option<crate::types::Range>,
        w: &mut Writer,
    ) -> Result<()> {
        // An extensible constraint is not OER-visible, so the integer has
        // no bounds (X.696 8.2.2, 10.4 Note 2).
        let signed =
            range.is_none_or(|range| range.extensible || range.min.is_none_or(|min| min < 0));
        let fixed = range.and_then(|range| {
            let (min, max) = (range.min?, range.max?);

            if range.extensible {
                return None;
            }

            let unsigned = min >= 0;
            let length: usize = if unsigned {
                match max {
                    ..=255 => 1,
                    256..=65535 => 2,
                    65536..=4294967295 => 4,
                    4294967296..=18446744073709551615 => 8,
                    _ => return None,
                }
            } else if min >= -128 && max < 128 {
                1
            } else if min >= -32768 && max < 32768 {
                2
            } else if min >= -2147483648 && max < 2147483648 {
                4
            } else if min >= -9223372036854775808 && max < 9223372036854775808 {
                8
            } else {
                return None;
            };

            Some((length, unsigned))
        });

        match fixed {
            Some((length, _)) => {
                let Value::Integer(value) = number else {
                    return encode_error("The value does not fit in the field.");
                };
                let bytes = value.to_be_bytes();
                let fits = if signed {
                    let top = 1i128 << (8 * length - 1);

                    -top <= *value && *value < top
                } else {
                    *value >= 0 && *value < (1i128 << (8 * length))
                };

                if !fits {
                    return encode_error("The value does not fit in the field.");
                }

                w.bytes(&bytes[16 - length..]);

                Ok(())
            }
            None if signed => put_integer(w, &number.to_twos_complement().unwrap()),
            None => match number {
                Value::Integer(value) if *value >= 0 => put_unsigned(w, *value as u128),
                Value::BigInteger(bytes) if bytes.first().is_some_and(|byte| byte & 0x80 == 0) => {
                    // A positive number: drop the byte that only holds the sign.
                    let trimmed = if bytes[0] == 0 && bytes.len() > 1 {
                        &bytes[1..]
                    } else {
                        &bytes[..]
                    };

                    put_integer(w, trimmed)
                }
                _ => encode_error("Expected a number that is not negative."),
            },
        }
    }

    fn encode_enumerated(
        &self,
        root: &[(String, i64)],
        additions: Option<&[(String, i64)]>,
        value: &Value,
        w: &mut Writer,
    ) -> Result<()> {
        let all = || root.iter().chain(additions.into_iter().flatten());
        let number = match value {
            Value::Text(name) if !self.types.numeric_enums => {
                all().find(|(candidate, _)| candidate == name).map(|e| e.1)
            }
            Value::Integer(number) if self.types.numeric_enums => all()
                .find(|(_, candidate)| i128::from(*candidate) == *number)
                .map(|e| e.1),
            _ => None,
        };
        let Some(number) = number else {
            return encode_error("Expected an enumeration value.");
        };

        if (0..=127).contains(&number) {
            w.uint(number as u128, 8);
        } else {
            // The length of the number, with the first bit set.
            let bytes = integer_to_twos_complement(i128::from(number));

            w.uint(0x80 | bytes.len() as u128, 8);
            w.bytes(&bytes);
        }

        Ok(())
    }

    fn encode_choice(
        &self,
        members: &Members,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let Value::Choice(name, inner) = value else {
            return encode_error("Expected data of type tuple(str, object).");
        };
        let root_count = members.root.len();
        let found = members
            .all()
            .enumerate()
            .find(|(_, member)| &member.name == name);
        let Some((index, member)) = found else {
            return encode_error("Expected a name of an alternative of the CHOICE.");
        };
        let Some((class, number)) = self.chosen_tag(member.ty, inner) else {
            return encode_error("An alternative of a CHOICE needs a tag.");
        };

        put_tag(w, class, number);

        if index < root_count {
            return self.encode_type(member.ty, inner, w, depth + 1);
        }

        let mut open = Writer::new(false);

        self.encode_type(member.ty, inner, &mut open, depth + 1)?;
        put_length(w, open.number_of_bytes())?;
        w.append(&open);

        Ok(())
    }

    /// The outermost tag of the type of the chosen alternative, which for
    /// an untagged CHOICE is that of the alternative chosen in it, and so
    /// on (X.696 20.1).
    fn chosen_tag(&self, mut ty: TypeId, mut value: &Value) -> Option<(TagClass, u64)> {
        for _ in 0..MAX_DEPTH {
            let node = self.types.node(ty);

            if let Some(tag) = node.tags.first() {
                return Some((tag.class, tag.number));
            }

            let (Kind::Choice(members), Value::Choice(name, inner)) = (self.types.kind(ty), value)
            else {
                return None;
            };

            ty = members.all().find(|member| &member.name == name)?.ty;
            value = inner;
        }

        None
    }

    fn encode_string(
        &self,
        kind: StrKind,
        text: &str,
        size: Option<crate::types::Range>,
        w: &mut Writer,
    ) -> Result<()> {
        let bytes: Vec<u8> = match kind {
            StrKind::Utf8 => Vec::from(text.as_bytes()),
            StrKind::Bmp => crate::ber::bmp_bytes(text)?,
            StrKind::Universal => text
                .chars()
                .flat_map(|c| u32::from(c).to_be_bytes())
                .collect(),
            StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
                if !text.is_ascii() {
                    return encode_error("A character is not ASCII.");
                }

                Vec::from(text.as_bytes())
            }
            _ => text
                .chars()
                .map(|c| {
                    u8::try_from(u32::from(c))
                        .map_err(|_| Error::Encode("A character is not Latin-1."))
                })
                .collect::<Result<_>>()?,
        };
        // A string of a known multiplier with a size of one value has no
        // length (X.696 27.2).
        match known_multiplier(kind).and(fixed_size(size)) {
            Some(characters) => {
                if text.chars().count() != characters {
                    return encode_error("Expected a string of the size of its type.");
                }
            }
            None => put_length(w, bytes.len())?,
        }

        w.bytes(&bytes);

        Ok(())
    }

    fn encode_date(&self, date: &Date, w: &mut Writer) -> Result<()> {
        put_integer(w, &integer_to_twos_complement(i128::from(date.year)))?;
        w.uint(u128::from(date.month), 8);
        w.uint(u128::from(date.day), 8);

        Ok(())
    }

    fn encode_time_of_day(&self, time: &Time, w: &mut Writer) -> Result<()> {
        w.uint(u128::from(time.hour), 8);
        w.uint(u128::from(time.minute), 8);
        w.uint(u128::from(time.second), 8);

        Ok(())
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

        w.align_always();

        for element in [
            &c.years, &c.months, &c.weeks, &c.days, &c.hours, &c.minutes, &c.seconds,
        ]
        .into_iter()
        .flatten()
        {
            put_duration_field(w, element)?;
        }
        if let Some((nod, fv)) = c.fractional_part {
            put_unsigned(w, nod as u128)?;
            put_unsigned(w, fv as u128)?;
        }

        Ok(())
    }

    fn encode_members(
        &self,
        members: &Members,
        is_set: bool,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let Value::Map(map) = value else {
            return encode_error("Expected data of type dict.");
        };
        let root: Vec<&MemberInfo> = if is_set {
            members
                .order
                .iter()
                .map(|index| &members.root[*index])
                .collect()
        } else {
            members.root.iter().collect()
        };

        match &members.additions {
            None => self.encode_root(&root, map, w, depth),
            Some(additions) => {
                let marker = w.bits;

                w.bit(false);
                self.encode_root(&root, map, w, depth)?;

                let flat: Vec<&MemberInfo> = additions
                    .iter()
                    .flat_map(|addition| match addition {
                        Addition::Member(member) => core::slice::from_ref(member).iter(),
                        Addition::Group(group) => group.iter(),
                    })
                    .collect();

                if !flat.is_empty() && self.encode_additions(&flat, map, w, depth)? {
                    w.set_bit(marker);
                }

                Ok(())
            }
        }
    }

    fn encode_root(
        &self,
        root: &[&MemberInfo],
        map: &[(String, Value)],
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        for member in root
            .iter()
            .filter(|member| member.optional || member.has_default())
        {
            let present = map.iter().find(|(name, _)| name == &member.name);

            if member.optional {
                w.bit(present.is_some());
            } else {
                w.bit(present.is_some_and(|(_, value)| !is_default(self.types, member, value)));
            }
        }

        w.align_always();

        for member in root {
            self.encode_member(member, map, w, false, depth)?;
        }

        Ok(())
    }

    fn encode_member(
        &self,
        member: &MemberInfo,
        map: &[(String, Value)],
        w: &mut Writer,
        encode_default: bool,
        depth: usize,
    ) -> Result<()> {
        match map.iter().find(|(name, _)| name == &member.name) {
            Some((_, value)) => {
                if !member.has_default() || !is_default(self.types, member, value) || encode_default
                {
                    match self.types.defined_by(member.ty, map) {
                        Some(Some(ty)) => self.encode_open_type(ty, value, w, depth)?,
                        Some(None) => return encode_error("Bad AnyDefinedBy choice."),
                        None => self.encode_type(member.ty, value, w, depth + 1)?,
                    }
                }

                Ok(())
            }
            None if member.optional || member.has_default() => Ok(()),
            None => encode_error("A member is not found in the SEQUENCE or SET."),
        }
    }

    /// The value of an open type of the type `id`: its encoding, after
    /// its length (X.696 30).
    fn encode_open_type(
        &self,
        id: TypeId,
        value: &Value,
        w: &mut Writer,
        depth: usize,
    ) -> Result<()> {
        let mut open = Writer::new(false);

        self.encode_type(id, value, &mut open, depth + 1)?;
        put_length(w, open.number_of_bytes())?;
        w.append(&open);

        Ok(())
    }

    fn encode_additions(
        &self,
        additions: &[&MemberInfo],
        map: &[(String, Value)],
        w: &mut Writer,
        depth: usize,
    ) -> Result<bool> {
        let mut presence: u128 = 0;
        let mut encoders: Vec<Writer> = Vec::new();
        let mut processed = 0;

        // An addition that can not be encoded ends the additions.
        for addition in additions {
            presence <<= 1;
            processed += 1;

            let mut sub = Writer::new(false);

            match self.encode_member(addition, map, &mut sub, true, depth) {
                Ok(()) => {
                    if sub.bits > 0 || map.iter().any(|(name, _)| name == &addition.name) {
                        encoders.push(sub);
                        presence |= 1;
                    }
                }
                Err(Error::Encode(_)) => break,
                Err(error) => return Err(error),
            }
        }

        if encoders.is_empty() {
            return Ok(false);
        }

        // The additions after the last are not there.
        presence <<= additions.len() - processed;

        let count = additions.len();
        let unused = (8 - count % 8) % 8;

        put_length(w, count.div_ceil(8) + 1)?;
        w.uint(unused as u128, 8);
        w.uint(presence << unused, count + unused);

        for sub in encoders {
            put_length(w, sub.number_of_bytes())?;
            w.append(&sub);
        }

        Ok(true)
    }

    // Decoding.

    fn decode_type(&self, id: TypeId, r: &mut Reader<'_>, depth: usize) -> Result<Value> {
        if depth > MAX_DEPTH {
            return decode_error("The data is nested too deeply.", r.position);
        }

        let node = self.types.node(id);
        let kind = &self.types.node(node.concrete).kind;
        let size = node.effective_size;

        match kind {
            Kind::Boolean => Ok(Value::Boolean(r.uint(8)? != 0)),
            Kind::Integer(_) => self.decode_integer(oer_range(node), r),
            Kind::Real => match self.types.node(node.concrete).real {
                None => {
                    let length = get_length(r)?;
                    let bytes = r.bytes(length)?;

                    // The contents of CER and DER (X.696 12.4).
                    decode_real(&bytes, r.position / 8, true).map(Value::Real)
                }
                Some(RealFormat::Binary32) => {
                    let bytes = r.bytes(4)?;

                    Ok(Value::Real(f64::from(f32::from_be_bytes([
                        bytes[0], bytes[1], bytes[2], bytes[3],
                    ]))))
                }
                Some(RealFormat::Binary64) => {
                    let bytes = r.bytes(8)?;
                    let mut array = [0u8; 8];

                    array.copy_from_slice(&bytes);

                    Ok(Value::Real(f64::from_be_bytes(array)))
                }
            },
            Kind::Null => Ok(Value::Null),
            Kind::BitString(_) => {
                if let Some(bits) = fixed_size(size) {
                    return r
                        .bytes(bits.div_ceil(8))
                        .map(|bytes| Value::Bits(bytes, bits));
                }

                let length = get_length(r)?;
                let unused = r.uint(8)? as usize;

                if length == 0 {
                    return decode_error("Expected at least one byte of a BIT STRING.", r.position);
                }

                let bytes = r.bytes(length - 1)?;
                let bits = (8 * (length - 1)).checked_sub(unused);

                match bits {
                    Some(bits) => Ok(Value::Bits(bytes, bits)),
                    None => decode_error("Too many unused bits.", r.position),
                }
            }
            Kind::OctetString => {
                let length = match fixed_size(size) {
                    Some(length) => length,
                    None => get_length(r)?,
                };

                r.bytes(length).map(Value::Bytes)
            }
            Kind::ObjectIdentifier => {
                let length = get_length(r)?;
                let bytes = r.bytes(length)?;

                decode_object_identifier(&bytes, r.position / 8, false).map(Value::Text)
            }
            Kind::Enumerated { root, additions } => {
                let first = r.uint(8)? as i128;
                let number = if first & 0x80 != 0 {
                    let length = (first & 0x7f) as usize;
                    let bytes = r.bytes(length)?;

                    match Value::from_twos_complement(&bytes) {
                        Value::Integer(number) => number,
                        _ => return decode_error("Expected an enumeration value.", r.position),
                    }
                } else {
                    first
                };
                let found = root
                    .iter()
                    .chain(additions.iter().flatten())
                    .find(|(_, candidate)| i128::from(*candidate) == number);

                match found {
                    Some(_) if self.types.numeric_enums => Ok(Value::Integer(number)),
                    Some((name, _)) => Ok(Value::Text(name.clone())),
                    None if additions.is_some() => Ok(Value::Null),
                    None => decode_error("Expected an enumeration value.", r.position),
                }
            }
            Kind::Sequence(members) | Kind::External(members) | Kind::Set(members) => {
                self.decode_members(members, matches!(kind, Kind::Set(_)), r, depth)
            }
            Kind::Choice(members) => self.decode_choice(members, r, depth),
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let length = get_unsigned(r)?;
                let length = usize::try_from(length)
                    .map_err(|_| Error::Decode("A length is too large.", r.position))?;
                let mut items = Vec::new();

                r.spend(length)?;

                for _ in 0..length {
                    items.push(self.decode_type(*element, r, depth + 1)?);
                }

                Ok(Value::List(items))
            }
            Kind::Str(string) => self.decode_string(*string, size, r).map(Value::Text),
            Kind::Utc => {
                let text = self.decode_string(StrKind::Visible, None, r)?;

                time::utc_time_from_string(&text, false).map(Value::DateTime)
            }
            Kind::Generalized => {
                let text = self.decode_string(StrKind::Visible, None, r)?;

                time::generalized_time_from_string(&text, false).map(Value::DateTime)
            }
            Kind::Date => self.decode_date(r).map(Value::Date),
            Kind::TimeOfDay => self.decode_time_of_day(r).map(Value::Time),
            Kind::DateTime => {
                let date = self.decode_date(r)?;
                let time = self.decode_time_of_day(r)?;

                Ok(Value::DateTime(DateTime { date, time }))
            }
            Kind::Duration => self.decode_duration(r).map(Value::Text),
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                decode_error("Bad AnyDefinedBy choice.", r.position)
            }
            Kind::OpenType | Kind::Any | Kind::AnyDefinedBy { .. } => {
                let length = get_length(r)?;

                r.bytes(length).map(Value::Bytes)
            }
            Kind::Ref(_) => decode_error("The type is a reference.", r.position),
        }
    }

    fn decode_integer(
        &self,
        range: Option<crate::types::Range>,
        r: &mut Reader<'_>,
    ) -> Result<Value> {
        // An extensible constraint is not OER-visible, so the integer has
        // no bounds (X.696 8.2.2, 10.4 Note 2).
        let signed =
            range.is_none_or(|range| range.extensible || range.min.is_none_or(|min| min < 0));
        let fixed = range.and_then(|range| {
            let (min, max) = (range.min?, range.max?);

            if range.extensible {
                return None;
            }

            Some(if min >= 0 {
                match max {
                    ..=255 => 1,
                    256..=65535 => 2,
                    65536..=4294967295 => 4,
                    4294967296..=18446744073709551615 => 8,
                    _ => return None,
                }
            } else if min >= -128 && max < 128 {
                1
            } else if min >= -32768 && max < 32768 {
                2
            } else if min >= -2147483648 && max < 2147483648 {
                4
            } else if min >= -9223372036854775808 && max < 9223372036854775808 {
                8
            } else {
                return None;
            })
        });

        if let Some(length) = fixed {
            let raw = r.uint(8 * length)? as i128;

            return Ok(Value::Integer(
                if signed && raw >= 1i128 << (8 * length - 1) {
                    raw - (1i128 << (8 * length))
                } else {
                    raw
                },
            ));
        }

        if signed {
            return get_integer(r);
        }

        let length = get_length(r)?;
        let bytes = r.bytes(length)?;
        let mut padded = alloc::vec![0u8];

        padded.extend_from_slice(&bytes);

        Ok(Value::from_twos_complement(&padded))
    }

    fn decode_choice(&self, members: &Members, r: &mut Reader<'_>, depth: usize) -> Result<Value> {
        let (class, number) = get_tag(r)?;
        let root_count = members.root.len();
        // An untagged CHOICE has the tags of its alternatives, which it
        // reads again (X.696 20.1, note 3).
        let found = members
            .all()
            .enumerate()
            .find(|(_, member)| self.types.accepts(member.ty, class, number));

        match found {
            Some((index, member)) => {
                if index >= root_count {
                    get_length(r)?;
                }

                let value = self.decode_type(member.ty, r, depth + 1)?;

                Ok(Value::Choice(member.name.clone(), value.into()))
            }
            None if members.additions.is_some() => {
                let length = get_length(r)?;

                r.skip(8 * length)?;

                Ok(Value::Choice(String::new(), Value::Null.into()))
            }
            None => decode_error(
                "Expected the tag of an alternative of the CHOICE.",
                r.position,
            ),
        }
    }

    fn decode_string(
        &self,
        kind: StrKind,
        size: Option<crate::types::Range>,
        r: &mut Reader<'_>,
    ) -> Result<String> {
        let length = match known_multiplier(kind).zip(fixed_size(size)) {
            Some((width, characters)) => width * characters,
            None => get_length(r)?,
        };
        let bytes = r.bytes(length)?;

        match kind {
            StrKind::Utf8 => core::str::from_utf8(&bytes)
                .map(String::from)
                .map_err(|_| Error::Decode("The string is not UTF-8.", r.position)),
            StrKind::Bmp => crate::ber::bmp_text(&bytes, r.position),
            StrKind::Universal => {
                if bytes.len() % 4 != 0 {
                    return decode_error("The string is not UTF-32.", r.position);
                }

                bytes
                    .chunks(4)
                    .map(|quad| {
                        char::from_u32(u32::from_be_bytes([quad[0], quad[1], quad[2], quad[3]]))
                    })
                    .collect::<Option<String>>()
                    .ok_or(Error::Decode("The string is not UTF-32.", r.position))
            }
            StrKind::Numeric | StrKind::Printable | StrKind::Ia5 | StrKind::Visible => {
                if bytes.is_ascii() {
                    Ok(bytes.iter().map(|byte| char::from(*byte)).collect())
                } else {
                    decode_error("The string is not ASCII.", r.position)
                }
            }
            _ => Ok(bytes.iter().map(|byte| char::from(*byte)).collect()),
        }
    }

    fn decode_date(&self, r: &mut Reader<'_>) -> Result<Date> {
        let year = match get_integer(r)? {
            Value::Integer(year) => year as i32,
            _ => return decode_error("The year is too large.", r.position),
        };
        let month = r.uint(8)? as u8;
        let day = r.uint(8)? as u8;

        Ok(Date { year, month, day })
    }

    fn decode_time_of_day(&self, r: &mut Reader<'_>) -> Result<Time> {
        Ok(Time {
            hour: r.uint(8)? as u8,
            minute: r.uint(8)? as u8,
            second: r.uint(8)? as u8,
            microsecond: 0,
            offset: None,
        })
    }

    fn decode_duration(&self, r: &mut Reader<'_>) -> Result<String> {
        let has_years = r.bit()?;
        let has_months = r.bit()?;
        let has_weeks = r.bit()?;
        let has_days = r.bit()?;
        let has_hours = r.bit()?;
        let has_minutes = r.bit()?;
        let has_seconds = r.bit()?;
        let has_fractional_part = r.bit()?;

        r.align_always();

        let mut c = time::DurationComponents::default();

        for (present, element) in [
            (has_years, &mut c.years),
            (has_months, &mut c.months),
            (has_weeks, &mut c.weeks),
            (has_days, &mut c.days),
            (has_hours, &mut c.hours),
            (has_minutes, &mut c.minutes),
            (has_seconds, &mut c.seconds),
        ] {
            if present {
                *element = Some(get_duration_field(r)?);
            }
        }
        if has_fractional_part {
            let nod = get_duration_field(r)?;
            let fv = get_duration_field(r)?;
            let field = |value| {
                time::fraction_field(value).map_err(|message| Error::Decode(message, r.position))
            };

            c.fractional_part = Some((field(nod)?, field(fv)?));
        }

        time::components_to_duration(&c).map_err(|message| Error::Decode(message, r.position))
    }

    fn decode_members(
        &self,
        members: &Members,
        is_set: bool,
        r: &mut Reader<'_>,
        depth: usize,
    ) -> Result<Value> {
        let root: Vec<&MemberInfo> = if is_set {
            members
                .order
                .iter()
                .map(|index| &members.root[*index])
                .collect()
        } else {
            members.root.iter().collect()
        };
        let has_additions = members.additions.is_some() && r.bit()?;
        let mut values = Vec::new();
        let mut present = Vec::new();

        for member in &root {
            if member.optional || member.has_default() {
                present.push(r.bit()?);
            }
        }

        r.align_always();

        let mut next = 0;

        for member in &root {
            let is_present = if member.optional || member.has_default() {
                next += 1;

                present[next - 1]
            } else {
                true
            };

            if is_present {
                let value = self.decode_member(member, &values, r, depth)?;

                values.push((member.name.clone(), value));
            } else if let Some(default) = &member.default {
                values.push((member.name.clone(), default.clone()));
            }
        }

        if has_additions {
            let additions: Vec<&MemberInfo> = members.all().skip(members.root.len()).collect();

            self.decode_additions(&additions, &mut values, r, depth)?;
        }

        Ok(Value::Map(values))
    }

    /// The value of a member, which is that of an open type if it is an ANY
    /// DEFINED BY with choices, of the type the member it names among
    /// `values` tells.
    fn decode_member(
        &self,
        member: &MemberInfo,
        values: &[(String, Value)],
        r: &mut Reader<'_>,
        depth: usize,
    ) -> Result<Value> {
        match self.types.defined_by(member.ty, values) {
            Some(Some(ty)) => self.decode_open_type(ty, r, depth),
            Some(None) => decode_error("Bad AnyDefinedBy choice.", r.position),
            None => self.decode_type(member.ty, r, depth + 1),
        }
    }

    /// The value of an open type of the type `id`, see `encode_open_type`.
    /// It has to fill the length it is in.
    fn decode_open_type(&self, id: TypeId, r: &mut Reader<'_>, depth: usize) -> Result<Value> {
        let length = get_length(r)?;
        let data = r.bytes(length)?;
        let mut inner = Reader {
            data: &data,
            position: 0,
            aligned: true,
            budget: r.budget,
        };
        let value = self.decode_type(id, &mut inner, depth + 1)?;

        r.budget = inner.budget;

        if inner.position != 8 * data.len() {
            return decode_error("The value does not fill its open type.", r.position);
        }

        Ok(value)
    }

    fn decode_additions(
        &self,
        additions: &[&MemberInfo],
        values: &mut Vec<(String, Value)>,
        r: &mut Reader<'_>,
        depth: usize,
    ) -> Result<()> {
        let length = get_length(r)?;
        let unused = r.uint(8)? as usize;

        if length == 0 {
            return decode_error("Expected the presence bits of the additions.", r.position);
        }

        let count = (8 * (length - 1))
            .checked_sub(unused)
            .ok_or(Error::Decode("Bad unused bits.", r.position))?;
        let mut presence = Vec::new();

        r.spend(count)?;

        for _ in 0..count {
            presence.push(r.bit()?);
        }

        r.align_always();

        // The bits that are left of the last byte are padding.
        for (index, present) in presence.iter().enumerate() {
            if !present {
                continue;
            }

            let member_length = get_length(r)?;

            match additions.get(index) {
                Some(member) => {
                    let value = self.decode_member(member, values, r, depth)?;

                    values.push((member.name.clone(), value));
                }
                None => r.skip(8 * member_length)?,
            }
        }

        Ok(())
    }
}
