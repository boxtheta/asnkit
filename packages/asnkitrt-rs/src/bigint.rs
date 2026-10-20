// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Ranges of integers with a bound of more than 128 bits, with the
//! feature `bigint`. Without it, a specification with such a bound is not
//! loaded, and the time elements of a DURATION are no more than 128 bits
//! in PER and OER.
//!
//! The values of the runtime stay `Value::Integer` and
//! `Value::BigInteger`, whatever the features; only what is done with
//! them here needs `num-bigint`.

use alloc::vec::Vec;

use num_bigint::{BigInt, BigUint, Sign};

use crate::node::{Bound, Const, Element};
use crate::types::Range;
use crate::value::Value;
use crate::{Error, Result};

/// The root of a constraint of which a bound does not fit in an `i128`:
/// its values and ranges, the least and the greatest of them, and if
/// there is an extension marker. A bound that is `None` is MIN or MAX.
#[derive(Debug, Clone, PartialEq)]
pub struct BigRange {
    pub min: Option<BigInt>,
    pub max: Option<BigInt>,
    pub parts: Vec<(Option<BigInt>, Option<BigInt>)>,
    pub extensible: bool,
}

impl BigRange {
    /// If a value is in the least and the greatest of the root, which is
    /// what PER encodes in (X.691 10.3.10).
    pub(crate) fn in_bounds(&self, value: &BigInt) -> bool {
        self.min.as_ref().is_none_or(|min| value >= min)
            && self.max.as_ref().is_none_or(|max| value <= max)
    }

    /// If a value satisfies the constraint, of which an extensible one is
    /// not checked.
    pub(crate) fn allows(&self, value: &BigInt) -> bool {
        self.extensible
            || self.parts.iter().any(|(lower, upper)| {
                lower.as_ref().is_none_or(|lower| value >= lower)
                    && upper.as_ref().is_none_or(|upper| value <= upper)
            })
    }

    /// The range of `i128` that OER makes the same choices with. A bound
    /// of more than 128 bits leaves out the fixed-size forms (X.696 10.3,
    /// 10.4), so only whether the integer is signed is left: if it has a
    /// lower bound, and if that is negative.
    pub(crate) fn as_oer_range(&self) -> Range {
        Range {
            min: self
                .min
                .as_ref()
                .map(|min| if min.sign() == Sign::Minus { -1 } else { 0 }),
            max: None,
            extensible: self.extensible,
        }
    }
}

fn bound(bound: &Bound, is_lower: bool) -> Result<Option<BigInt>> {
    let (value, excluded) = match bound {
        Bound::Value(value) => (value, false),
        Bound::Excluded(value) => (value, true),
    };
    let number = match value {
        Const::Int(number) => BigInt::from(*number),
        Const::Big {
            negative,
            magnitude,
        } => BigInt::from_signed_bytes_be(&crate::types::twos_complement_of(*negative, magnitude)),
        Const::Identifier(_) => {
            return Err(Error::BadNodes("A value in a constraint is not resolved."));
        }
        // MIN, MAX, or a number that is not an integer.
        _ => return Ok(None),
    };

    // An excluded bound is the next integer in.
    Ok(Some(match (excluded, is_lower) {
        (false, _) => number,
        (true, true) => number + 1,
        (true, false) => number - 1,
    }))
}

/// The root of a constraint of the range of an integer, which is the
/// union of its elements, as `types::range_of` and `types::parts_of`
/// have it.
pub(crate) fn range_of(elements: &[Element]) -> Result<BigRange> {
    let extensible = elements
        .iter()
        .any(|element| matches!(element, Element::ExtensionMarker));
    let mut parts = Vec::new();

    for element in elements {
        parts.push(match element {
            Element::ExtensionMarker => break,
            Element::Range(lower, upper) => (bound(lower, true)?, bound(upper, false)?),
            Element::Value(value) => {
                let value = bound(&Bound::Value(value.clone()), true)?;

                (value.clone(), value)
            }
            Element::Type(_) => (None, None),
        });
    }

    let min = match parts.iter().any(|(lower, _)| lower.is_none()) {
        true => None,
        false => parts.iter().filter_map(|(lower, _)| lower.clone()).min(),
    };
    let max = match parts.iter().any(|(_, upper)| upper.is_none()) {
        true => None,
        false => parts.iter().filter_map(|(_, upper)| upper.clone()).max(),
    };

    Ok(BigRange {
        min,
        max,
        parts,
        extensible,
    })
}

/// An integer of the runtime as a `BigInt`.
pub(crate) fn to_big(value: &Value) -> Option<BigInt> {
    match value {
        Value::Integer(number) => Some(BigInt::from(*number)),
        Value::BigInteger(bytes) => Some(BigInt::from_signed_bytes_be(bytes)),
        _ => None,
    }
}

/// A `BigInt` as an integer of the runtime, `Value::Integer` if it fits.
pub(crate) fn to_value(number: &BigInt) -> Value<'static> {
    Value::from_twos_complement(&number.to_signed_bytes_be())
}

/// The number of bits of an integer that is not negative, at least 0.
pub(crate) fn bits(number: &BigUint) -> usize {
    number.bits() as usize
}
