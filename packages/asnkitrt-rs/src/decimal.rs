// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What the text codecs write numbers and bytes as: integers of any
//! size in decimal, floats as Python writes them, and bytes in hex.

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use crate::value::Value;

/// The bytes in upper case hex.
pub(crate) fn hex_upper(bytes: &[u8]) -> String {
    use core::fmt::Write;

    let mut text = String::with_capacity(bytes.len() * 2);

    for byte in bytes {
        let _ = write!(text, "{byte:02X}");
    }

    text
}

/// The bytes of hex of any case, with an even number of digits.
pub(crate) fn hex_decode(text: &str) -> Option<Vec<u8>> {
    let digits = text.as_bytes();

    if digits.len() & 1 == 1 {
        return None;
    }

    digits
        .chunks(2)
        .map(|pair| {
            let high = (pair[0] as char).to_digit(16)?;
            let low = (pair[1] as char).to_digit(16)?;

            Some((high * 16 + low) as u8)
        })
        .collect()
}

/// The most digits of an integer that is read or written, as Python
/// does: more is slow, and nothing sane has them.
pub(crate) const MAX_DIGITS: usize = 4300;

/// An integer in decimal, for its two's complement (most significant
/// byte first). `None` if there are too many digits.
pub(crate) fn twos_complement_to_decimal(bytes: &[u8]) -> Option<String> {
    let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);

    // The magnitude, as words with the least significant first.
    let mut magnitude: Vec<u8> = bytes.to_vec();

    if negative {
        // Invert and add one.
        let mut carry = true;

        for byte in magnitude.iter_mut().rev() {
            *byte = !*byte;

            if carry {
                let (sum, overflow) = byte.overflowing_add(1);
                *byte = sum;
                carry = overflow;
            }
        }

        if carry {
            magnitude.insert(0, 1);
        }
    }

    // About 2.4 digits to a byte.
    if magnitude.len() > MAX_DIGITS / 2 + 1 {
        return None;
    }

    let mut words: Vec<u32> = Vec::new();

    for chunk in magnitude.rchunks(4) {
        let mut word = 0u32;

        for byte in chunk {
            word = (word << 8) | u32::from(*byte);
        }

        words.push(word);
    }

    while words.last() == Some(&0) {
        words.pop();
    }

    let mut parts: Vec<u32> = Vec::new();

    while !words.is_empty() {
        let mut remainder = 0u64;

        for word in words.iter_mut().rev() {
            let current = (remainder << 32) | u64::from(*word);

            *word = (current / 1_000_000_000) as u32;
            remainder = current % 1_000_000_000;
        }

        parts.push(remainder as u32);

        while words.last() == Some(&0) {
            words.pop();
        }
    }

    let mut text = String::new();

    match parts.pop() {
        None => text.push('0'),
        Some(first) => {
            if negative {
                text.push('-');
            }

            text.push_str(&format!("{first}"));

            for part in parts.iter().rev() {
                text.push_str(&format!("{part:09}"));
            }
        }
    }

    (text.len() <= MAX_DIGITS + 1).then_some(text)
}

/// An integer written in decimal, with an optional sign. `None` if it is
/// not one, or has too many digits.
pub(crate) fn decimal_to_value(text: &str) -> Option<Value> {
    let (negative, digits) = match text.as_bytes().first()? {
        b'-' => (true, &text[1..]),
        b'+' => (false, &text[1..]),
        _ => (false, text),
    };

    if digits.is_empty() || digits.len() > MAX_DIGITS || !digits.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }

    if digits.len() <= 38
        && let Ok(value) = digits.parse::<i128>()
    {
        return Some(Value::Integer(if negative { -value } else { value }));
    }

    // Multiply by a billion and add the next nine digits, over and over.
    let mut words: Vec<u32> = Vec::new();
    let bytes = digits.as_bytes();
    let first = bytes.len() % 9;
    let mut chunks = Vec::new();

    if first > 0 {
        chunks.push(&bytes[..first]);
    }

    chunks.extend(bytes[first..].chunks(9));

    for chunk in chunks {
        let mut carry = chunk
            .iter()
            .fold(0u64, |number, digit| number * 10 + u64::from(digit - b'0'));
        let factor = 10u64.pow(chunk.len() as u32);

        for word in words.iter_mut() {
            let current = u64::from(*word) * factor + carry;

            *word = current as u32;
            carry = current >> 32;
        }

        if carry > 0 {
            words.push(carry as u32);
        }
    }

    let mut magnitude: Vec<u8> = alloc::vec![0];

    for word in words.iter().rev() {
        magnitude.extend_from_slice(&word.to_be_bytes());
    }

    if negative {
        let mut carry = true;

        for byte in magnitude.iter_mut().rev() {
            *byte = !*byte;

            if carry {
                let (sum, overflow) = byte.overflowing_add(1);
                *byte = sum;
                carry = overflow;
            }
        }
    }

    Some(Value::from_twos_complement(&magnitude))
}

/// A float as `repr()` of Python writes it: the fewest digits that read
/// back as the same float.
pub(crate) fn float_repr(value: f64) -> String {
    if value.is_nan() {
        return String::from("nan");
    }

    if value.is_infinite() {
        return String::from(if value > 0.0 { "inf" } else { "-inf" });
    }

    if value == 0.0 {
        return String::from(if value.is_sign_negative() {
            "-0.0"
        } else {
            "0.0"
        });
    }

    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    let sign = if value < 0.0 { "-" } else { "" };

    if (-4..16).contains(&exponent) {
        if exponent >= 0 {
            let whole = exponent as usize + 1;

            if digits.len() <= whole {
                format!("{sign}{digits}{}.0", "0".repeat(whole - digits.len()))
            } else {
                format!("{sign}{}.{}", &digits[..whole], &digits[whole..])
            }
        } else {
            format!("{sign}0.{}{digits}", "0".repeat((-exponent - 1) as usize))
        }
    } else {
        let mantissa = if digits.len() > 1 {
            format!("{}.{}", &digits[..1], &digits[1..])
        } else {
            digits
        };

        format!(
            "{sign}{mantissa}e{}{:02}",
            if exponent < 0 { '-' } else { '+' },
            exponent.abs()
        )
    }
}

/// A finite float as an XER `realnumber` (X.680 12.9), with an
/// exponent: the float as `float_repr()` has it and `E0` if it is 0 or
/// at least 0.0001 and less than 10, else its fewest digits with one
/// before the point. The digits are those of the float, so it reads
/// back as the same float.
#[cfg(feature = "xer")]
pub(crate) fn xml_real(value: f64) -> String {
    let repr = float_repr(value);

    if value == 0.0 || (value.abs() < 10.0 && !repr.contains('e')) {
        return format!("{repr}E0");
    }

    let scientific = format!("{:e}", value.abs());
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let sign = if value < 0.0 { "-" } else { "" };

    if mantissa.contains('.') {
        format!("{sign}{mantissa}E{exponent}")
    } else {
        format!("{sign}{mantissa}.0E{exponent}")
    }
}

/// If the text is an XER real number: a `realnumber` (X.680 12.9),
/// after a `-` or not (X.680 21.9).
#[cfg(feature = "xer")]
pub(crate) fn is_xml_real(text: &str) -> bool {
    fn digits(text: &[u8]) -> usize {
        text.iter().take_while(|c| c.is_ascii_digit()).count()
    }

    let text = text.as_bytes();
    let text = text.strip_prefix(b"-").unwrap_or(text);
    let whole = digits(text);

    if whole == 0 {
        return false;
    }

    let mut rest = &text[whole..];

    if let Some(after) = rest.strip_prefix(b".") {
        rest = &after[digits(after)..];
    }

    if let Some(after) = rest.strip_prefix(b"e").or_else(|| rest.strip_prefix(b"E")) {
        let after = after
            .strip_prefix(b"-")
            .or_else(|| after.strip_prefix(b"+"))
            .unwrap_or(after);
        let exponent = digits(after);

        exponent > 0 && exponent == after.len()
    } else {
        rest.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "xer")]
    #[test]
    fn xml_reals() {
        for (value, text) in [
            (0.0, "0.0E0"),
            (-0.0, "-0.0E0"),
            (1.0, "1.0E0"),
            (0.5, "0.5E0"),
            (0.001, "0.001E0"),
            (-2.25, "-2.25E0"),
            (10.0, "1.0E1"),
            (12345.0, "1.2345E4"),
            (1e-5, "1.0E-5"),
            (-1.5e-300, "-1.5E-300"),
            (f64::MAX, "1.7976931348623157E308"),
            (5e-324, "5.0E-324"),
        ] {
            assert_eq!(xml_real(value), text);
            assert!(is_xml_real(&xml_real(value)));
            assert_eq!(xml_real(value).parse::<f64>(), Ok(value));
        }

        for value in [0.1 + 0.2, 123456789.123, 1e23] {
            assert_eq!(xml_real(value).parse::<f64>(), Ok(value));
        }
    }

    #[cfg(feature = "xer")]
    #[test]
    fn xml_real_text() {
        for text in [
            "1", "-1", "1.", "1.5", "007.50", "1e5", "1E-5", "1.e+5", "-0",
        ] {
            assert!(is_xml_real(text), "{text}");
        }

        for text in [
            "", "-", ".5", "+1", "1.5.", "1e", "1e+", "inf", "NaN", "INF", "1 ", "--1",
        ] {
            assert!(!is_xml_real(text), "{text}");
        }
    }

    #[test]
    fn floats_as_python_has_them() {
        for (value, text) in [
            (0.0, "0.0"),
            (-0.0, "-0.0"),
            (1.0, "1.0"),
            (1.5, "1.5"),
            (-2.25, "-2.25"),
            (100.0, "100.0"),
            (1e15, "1000000000000000.0"),
            (1e16, "1e+16"),
            (1.5e16, "1.5e+16"),
            (123456789012345680.0, "1.2345678901234568e+17"),
            (0.0001, "0.0001"),
            (0.00001, "1e-05"),
            (1.5e-7, "1.5e-07"),
            (1e100, "1e+100"),
            (0.1 + 0.2, "0.30000000000000004"),
            (f64::MAX, "1.7976931348623157e+308"),
            (5e-324, "5e-324"),
            (f64::NAN, "nan"),
            (f64::INFINITY, "inf"),
            (f64::NEG_INFINITY, "-inf"),
        ] {
            assert_eq!(float_repr(value), text);
        }
    }

    #[test]
    fn integers_of_any_size() {
        for text in [
            "0",
            "1",
            "-1",
            "255",
            "256",
            "-256",
            "-257",
            "170141183460469231731687303715884105727",
            "170141183460469231731687303715884105728",
            "-170141183460469231731687303715884105728",
            "-170141183460469231731687303715884105729",
            "123456789012345678901234567890123456789012345678901234567890",
            "-123456789012345678901234567890123456789012345678901234567890",
        ] {
            let value = decimal_to_value(text).unwrap();
            let bytes = value.to_twos_complement().unwrap();

            assert_eq!(twos_complement_to_decimal(&bytes).unwrap(), text);
        }

        assert_eq!(decimal_to_value("+7"), Some(Value::Integer(7)));
        assert_eq!(decimal_to_value("-0"), Some(Value::Integer(0)));
        assert_eq!(decimal_to_value("007"), Some(Value::Integer(7)));
        assert_eq!(decimal_to_value(""), None);
        assert_eq!(decimal_to_value("-"), None);
        assert_eq!(decimal_to_value("1.0"), None);
        assert_eq!(decimal_to_value("1e3"), None);
        assert_eq!(decimal_to_value(&"9".repeat(MAX_DIGITS + 1)), None);
        assert!(decimal_to_value(&"9".repeat(MAX_DIGITS)).is_some());
        assert_eq!(twos_complement_to_decimal(&[0xff]).unwrap(), "-1");
        assert_eq!(twos_complement_to_decimal(&[]).unwrap(), "0");
    }
}
