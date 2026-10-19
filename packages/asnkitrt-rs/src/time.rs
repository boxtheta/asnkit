// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The strings of UTCTime, GeneralizedTime, DATE, TIME-OF-DAY and
//! DATE-TIME, and the values they are, and those of DURATION.

use alloc::format;
use alloc::string::String;

use crate::value::{Date, DateTime, Time, Value};
use crate::{Error, Result};

fn bad() -> Error {
    Error::encode("Expected a time string.")
}

fn is_leap(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)
}

fn days_in(year: i32, month: u8) -> u8 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        _ if is_leap(year) => 29,
        _ => 28,
    }
}

fn checked_date(year: i32, month: u8, day: u8) -> Result<Date> {
    if !(1..=9999).contains(&year)
        || !(1..=12).contains(&month)
        || day < 1
        || day > days_in(year, month)
    {
        return Err(bad());
    }

    Ok(Date { year, month, day })
}

fn checked_time(
    hour: u8,
    minute: u8,
    second: u8,
    microsecond: u32,
    offset: Option<i32>,
) -> Result<Time> {
    if hour > 23 || minute > 59 || second > 59 {
        return Err(bad());
    }

    Ok(Time {
        hour,
        minute,
        second,
        microsecond,
        offset,
    })
}

/// A cursor over the characters of a string.
struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Cursor<'_> {
    /// A number of exactly `digits` digits.
    fn number(&mut self, digits: usize) -> Result<u32> {
        let part = self.bytes.get(self.at..self.at + digits).ok_or_else(bad)?;
        let mut value = 0;

        for byte in part {
            if !byte.is_ascii_digit() {
                return Err(bad());
            }

            value = value * 10 + u32::from(byte - b'0');
        }

        self.at += digits;

        Ok(value)
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.at).copied()
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }

    /// The digits of a fraction of a unit of `unit` microseconds, as
    /// microseconds, rounded down. There may be any number of digits
    /// (X.680 46.3), but those past the 20th are less than a
    /// microsecond, and are not looked at.
    fn fraction(&mut self, unit: u128) -> Result<u64> {
        let mut digits = 0;
        let mut value = 0u128;
        let mut any = false;

        while let Some(byte) = self.peek() {
            if !byte.is_ascii_digit() {
                break;
            }

            if digits < 20 {
                value = value * 10 + u128::from(byte - b'0');
                digits += 1;
            }

            any = true;
            self.at += 1;
        }

        if !any {
            return Err(bad());
        }

        Ok((value * unit / 10u128.pow(digits)) as u64)
    }

    /// `+HHMM` or `-HHMM`, or with `hours_only` also `+HH` or `-HH`, as
    /// minutes east of UTC.
    fn offset(&mut self, hours_only: bool) -> Result<i32> {
        let sign = match self.peek() {
            Some(b'+') => 1,
            Some(b'-') => -1,
            _ => return Err(bad()),
        };

        self.at += 1;

        let hours = self.number(2)? as i32;
        let minutes = if hours_only && self.done() {
            0
        } else {
            self.number(2)? as i32
        };

        if minutes > 59 || hours > 23 {
            return Err(bad());
        }

        Ok(sign * (hours * 60 + minutes))
    }
}

fn format_offset(offset: i32) -> String {
    let sign = if offset < 0 { '-' } else { '+' };
    let offset = offset.abs();

    format!("{sign}{:02}{:02}", offset / 60, offset % 60)
}

/// The seconds from the start of year 1 of `value`, in UTC.
fn to_seconds(value: &DateTime) -> i64 {
    let year = i64::from(value.date.year) - 1;
    let mut days = year * 365 + year / 4 - year / 100 + year / 400;

    for month in 1..value.date.month {
        days += i64::from(days_in(value.date.year, month));
    }

    days += i64::from(value.date.day) - 1;

    days * 86400
        + i64::from(value.time.hour) * 3600
        + i64::from(value.time.minute) * 60
        + i64::from(value.time.second)
        - i64::from(value.time.offset.unwrap_or(0)) * 60
}

fn from_seconds(mut seconds: i64, microsecond: u32) -> DateTime {
    let days = seconds.div_euclid(86400);

    seconds = seconds.rem_euclid(86400);

    // The days from 0000-03-01, so that a leap day is the last of its
    // year, in eras of 400 years, which all have as many days. (This
    // is `civil_from_days` of Howard Hinnant.)
    let days = days + 306;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    // From March.
    let shifted_month = (5 * day_of_year + 2) / 153;
    let days = day_of_year - (153 * shifted_month + 2) / 5;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u8;
    let year = (era * 400 + year_of_era + i64::from(month <= 2)) as i32;

    DateTime {
        date: Date {
            year,
            month,
            day: days as u8 + 1,
        },
        time: Time {
            hour: (seconds / 3600) as u8,
            minute: (seconds % 3600 / 60) as u8,
            second: (seconds % 60) as u8,
            microsecond,
            offset: None,
        },
    }
}

/// The date and time in UTC, as a naive one.
/// The time in UTC, with no offset.
pub(crate) fn to_utc(value: &DateTime) -> DateTime {
    if value.time.offset.is_none() {
        return *value;
    }

    from_seconds(to_seconds(value), value.time.microsecond)
}

fn two_digit_year(year: u32) -> i32 {
    // As `%y` is read: 69 to 99 are 1969 to 1999.
    if year >= 69 {
        1900 + year as i32
    } else {
        2000 + year as i32
    }
}

/// A UTCTime string. `restricted` is the form DER has.
pub fn utc_time_to_string(value: &DateTime, restricted: bool) -> String {
    if restricted {
        let utc = to_utc(value);

        return format!(
            "{:02}{:02}{:02}{:02}{:02}{:02}Z",
            utc.date.year % 100,
            utc.date.month,
            utc.date.day,
            utc.time.hour,
            utc.time.minute,
            utc.time.second
        );
    }

    let mut string = format!(
        "{:02}{:02}{:02}{:02}{:02}",
        value.date.year % 100,
        value.date.month,
        value.date.day,
        value.time.hour,
        value.time.minute
    );

    if value.time.second > 0 {
        string.push_str(&format!("{:02}", value.time.second));
    }

    match value.time.offset {
        None => string.push('Z'),
        Some(offset) => string.push_str(&format_offset(offset)),
    }

    string
}

pub fn utc_time_from_string(string: &str, restricted: bool) -> Result<DateTime> {
    let error = || Error::decode("Expected a UTC time string.", 0);
    let mut cursor = Cursor {
        bytes: string.as_bytes(),
        at: 0,
    };
    let parse = |cursor: &mut Cursor<'_>| -> Result<DateTime> {
        let year = two_digit_year(cursor.number(2)?);
        let month = cursor.number(2)? as u8;
        let day = cursor.number(2)? as u8;
        let hour = cursor.number(2)? as u8;
        let minute = cursor.number(2)? as u8;
        let second = if restricted || cursor.peek().is_some_and(|byte| byte.is_ascii_digit()) {
            cursor.number(2)? as u8
        } else {
            0
        };
        let offset = match cursor.peek() {
            Some(b'Z') => {
                cursor.at += 1;

                None
            }
            _ if restricted => return Err(bad()),
            _ => Some(cursor.offset(false)?),
        };

        if !cursor.done() {
            return Err(bad());
        }

        Ok(DateTime {
            date: checked_date(year, month, day)?,
            time: checked_time(hour, minute, second, 0, offset)?,
        })
    };

    parse(&mut cursor).map_err(|_| error())
}

/// A GeneralizedTime string. `restricted` is the form DER has.
pub fn generalized_time_to_string(value: &DateTime, restricted: bool) -> String {
    let fraction = |microsecond: u32| -> String {
        if microsecond > 0 {
            let digits = format!("{microsecond:06}");

            format!(".{}", digits.trim_end_matches('0'))
        } else {
            String::new()
        }
    };

    if restricted {
        let utc = to_utc(value);

        return format!(
            "{:04}{:02}{:02}{:02}{:02}{:02}{}Z",
            utc.date.year,
            utc.date.month,
            utc.date.day,
            utc.time.hour,
            utc.time.minute,
            utc.time.second,
            fraction(value.time.microsecond)
        );
    }

    let mut string = format!(
        "{:04}{:02}{:02}{:02}{:02}",
        value.date.year, value.date.month, value.date.day, value.time.hour, value.time.minute
    );

    // A fraction after the minute would be one of a minute (X.680 46.3).
    if value.time.second != 0 || value.time.microsecond != 0 {
        string.push_str(&format!("{:02}", value.time.second));
    }

    string.push_str(&fraction(value.time.microsecond));

    match value.time.offset {
        Some(offset) if offset != 0 => string.push_str(&format_offset(offset)),
        Some(_) => string.push('Z'),
        None => {}
    }

    string
}

pub fn generalized_time_from_string(string: &str, restricted: bool) -> Result<DateTime> {
    let error = || Error::decode("Expected a generalized time string.", 0);
    let bytes = string.as_bytes();

    if restricted {
        if bytes.last() != Some(&b'Z') {
            return Err(error());
        }

        if string.contains('.')
            && (bytes.len() < 2 || bytes[bytes.len() - 2] == b'0' || bytes.get(14) != Some(&b'.'))
        {
            return Err(error());
        }

        if !string.contains('.') && bytes.len() != 15 {
            return Err(error());
        }
    }

    let mut cursor = Cursor { bytes, at: 0 };
    let parse = |cursor: &mut Cursor<'_>| -> Result<DateTime> {
        let year = cursor.number(4)? as i32;
        let month = cursor.number(2)? as u8;
        let day = cursor.number(2)? as u8;
        let hour = cursor.number(2)? as u8;
        let is_digit =
            |cursor: &Cursor<'_>| cursor.peek().is_some_and(|byte| byte.is_ascii_digit());
        // The minute and the second may be left out, and a fraction is
        // one of the last of the hour, the minute and the second there
        // is (X.680 46.3).
        let minute = if is_digit(cursor) {
            Some(cursor.number(2)? as u8)
        } else {
            None
        };
        let second = if minute.is_some() && is_digit(cursor) {
            Some(cursor.number(2)? as u8)
        } else {
            None
        };
        let unit: u128 = match (minute, second) {
            (None, _) => 3_600_000_000,
            (Some(_), None) => 60_000_000,
            (Some(_), Some(_)) => 1_000_000,
        };
        let fraction = if matches!(cursor.peek(), Some(b'.' | b',')) {
            cursor.at += 1;
            cursor.fraction(unit)?
        } else {
            0
        };
        let offset = match cursor.peek() {
            Some(b'Z') => {
                cursor.at += 1;

                Some(0)
            }
            Some(b'+' | b'-') => Some(cursor.offset(true)?),
            _ => None,
        };

        if !cursor.done() {
            return Err(bad());
        }

        let minute = u64::from(minute.unwrap_or(0)) + fraction / 60_000_000;
        let fraction = fraction % 60_000_000;
        let second = u64::from(second.unwrap_or(0)) + fraction / 1_000_000;

        Ok(DateTime {
            date: checked_date(year, month, day)?,
            // The restricted form is read as a naive time.
            time: checked_time(
                hour,
                u8::try_from(minute).map_err(|_| bad())?,
                u8::try_from(second).map_err(|_| bad())?,
                (fraction % 1_000_000) as u32,
                if restricted { None } else { offset },
            )?,
        })
    };

    parse(&mut cursor).map_err(|_| error())
}

pub fn date_to_string(value: &Date) -> String {
    format!("{:04}{:02}{:02}", value.year, value.month, value.day)
}

pub fn date_from_string(string: &str) -> Result<Date> {
    let mut cursor = Cursor {
        bytes: string.as_bytes(),
        at: 0,
    };
    let year = cursor.number(4)? as i32;
    let month = cursor.number(2)? as u8;
    let day = cursor.number(2)? as u8;

    if !cursor.done() {
        return Err(bad());
    }

    checked_date(year, month, day)
}

/// Checks that a time is one of a TIME-OF-DAY or a DATE-TIME, which
/// are a local time to the second (X.680 38.4.2, 38.4.3).
pub fn check_local_time(value: &Time) -> Result<()> {
    if value.microsecond != 0 || value.offset.is_some() {
        return Err(Error::encode(
            "A TIME-OF-DAY or a DATE-TIME has no fraction of a second and no offset.",
        ));
    }

    Ok(())
}

/// A TIME-OF-DAY as BER writes it (X.690 8.26.3).
pub fn time_of_day_to_string(value: &Time) -> Result<String> {
    check_local_time(value)?;

    Ok(format!(
        "{:02}{:02}{:02}",
        value.hour, value.minute, value.second
    ))
}

pub fn time_of_day_from_string(string: &str) -> Result<Time> {
    let mut cursor = Cursor {
        bytes: string.as_bytes(),
        at: 0,
    };
    let hour = cursor.number(2)? as u8;
    let minute = cursor.number(2)? as u8;
    let second = cursor.number(2)? as u8;

    if !cursor.done() {
        return Err(bad());
    }

    checked_time(hour, minute, second, 0, None)
}

/// A DATE-TIME as BER writes it (X.690 8.26.4).
pub fn date_time_to_string(value: &DateTime) -> Result<String> {
    check_local_time(&value.time)?;

    Ok(format!(
        "{:04}{:02}{:02}{:02}{:02}{:02}",
        value.date.year,
        value.date.month,
        value.date.day,
        value.time.hour,
        value.time.minute,
        value.time.second
    ))
}

pub fn date_time_from_string(string: &str) -> Result<DateTime> {
    let mut cursor = Cursor {
        bytes: string.as_bytes(),
        at: 0,
    };
    let year = cursor.number(4)? as i32;
    let month = cursor.number(2)? as u8;
    let day = cursor.number(2)? as u8;
    let hour = cursor.number(2)? as u8;
    let minute = cursor.number(2)? as u8;
    let second = cursor.number(2)? as u8;

    if !cursor.done() {
        return Err(bad());
    }

    Ok(DateTime {
        date: checked_date(year, month, day)?,
        time: checked_time(hour, minute, second, 0, None)?,
    })
}

/// The text codecs write dates and times as `str()` of Python does,
/// and read them with `%Y-%m-%d`, `%H:%M:%S` and `%Y-%m-%dT%H:%M:%S`.
#[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
pub(crate) mod iso {
    use super::*;

    pub fn date_to_string(value: &Date) -> String {
        format!("{:04}-{:02}-{:02}", value.year, value.month, value.day)
    }

    pub fn time_to_string(value: &Time) -> Result<String> {
        check_local_time(value)?;

        Ok(format!(
            "{:02}:{:02}:{:02}",
            value.hour, value.minute, value.second
        ))
    }

    pub fn date_time_to_string(value: &DateTime) -> Result<String> {
        Ok(format!(
            "{}T{}",
            date_to_string(&value.date),
            time_to_string(&value.time)?
        ))
    }

    /// One or two digits, as many as there are.
    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    fn number(cursor: &mut Cursor<'_>) -> Result<u32> {
        let mut value = 0;
        let mut digits = 0;

        while digits < 2
            && let Some(byte) = cursor.bytes.get(cursor.at)
            && byte.is_ascii_digit()
        {
            value = value * 10 + u32::from(byte - b'0');
            cursor.at += 1;
            digits += 1;
        }

        if digits == 0 { Err(bad()) } else { Ok(value) }
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    fn expect(cursor: &mut Cursor<'_>, byte: u8) -> Result<()> {
        if cursor.bytes.get(cursor.at) == Some(&byte) {
            cursor.at += 1;

            Ok(())
        } else {
            Err(bad())
        }
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    fn read_date(cursor: &mut Cursor<'_>) -> Result<Date> {
        let year = cursor.number(4)? as i32;

        expect(cursor, b'-')?;

        let month = number(cursor)? as u8;

        expect(cursor, b'-')?;

        let day = number(cursor)? as u8;

        checked_date(year, month, day)
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    fn read_time(cursor: &mut Cursor<'_>) -> Result<Time> {
        let hour = number(cursor)? as u8;

        expect(cursor, b':')?;

        let minute = number(cursor)? as u8;

        expect(cursor, b':')?;

        let second = number(cursor)? as u8;

        checked_time(hour, minute, second, 0, None)
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    pub fn date_from_string(string: &str) -> Result<Date> {
        let mut cursor = Cursor {
            bytes: string.as_bytes(),
            at: 0,
        };
        let date = read_date(&mut cursor)?;

        if cursor.done() { Ok(date) } else { Err(bad()) }
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    pub fn time_from_string(string: &str) -> Result<Time> {
        let mut cursor = Cursor {
            bytes: string.as_bytes(),
            at: 0,
        };
        let time = read_time(&mut cursor)?;

        if cursor.done() { Ok(time) } else { Err(bad()) }
    }

    #[cfg(any(feature = "jer", feature = "xer", feature = "gser"))]
    pub fn date_time_from_string(string: &str) -> Result<DateTime> {
        let mut cursor = Cursor {
            bytes: string.as_bytes(),
            at: 0,
        };
        let date = read_date(&mut cursor)?;

        expect(&mut cursor, b'T')?;

        let time = read_time(&mut cursor)?;

        if cursor.done() {
            Ok(DateTime { date, time })
        } else {
            Err(bad())
        }
    }
}

/// The most digits of a fractional part of a DURATION in PER and OER,
/// as the fractional value is an `i128`. X.691 and X.696 set no limit,
/// but a decoder needs one: a few bytes can give a number of digits
/// that does not fit in memory. The runtime for Python has the same.
pub const MAX_FRACTION_DIGITS: usize = 38;

/// A time element of a DURATION of more than 128 bits, without the
/// feature `bigint`.
pub(crate) const ELEMENT_TOO_LARGE: &str =
    "A time element of a DURATION does not fit in 128 bits, which needs the feature bigint.";

/// The fields of DURATION-INTERVAL-ENCODING (X.691 32.6, X.696 29.4). A
/// time element is a `Value::Integer`, or a `Value::BigInteger` with the
/// feature `bigint`; the fractional part is (number-of-digits,
/// fractional-value), which fit in an `i128` (see `MAX_FRACTION_DIGITS`).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DurationComponents {
    pub years: Option<Value>,
    pub months: Option<Value>,
    pub weeks: Option<Value>,
    pub days: Option<Value>,
    pub hours: Option<Value>,
    pub minutes: Option<Value>,
    pub seconds: Option<Value>,
    pub fractional_part: Option<(i128, i128)>,
}

/// A time element given by its digits.
fn element_of_digits(digits: &str) -> Result<Value> {
    if let Ok(number) = digits.parse::<i128>() {
        return Ok(Value::Integer(number));
    }

    #[cfg(feature = "bigint")]
    if let Ok(number) = digits.parse::<num_bigint::BigInt>() {
        return Ok(crate::bigint::to_value(&number));
    }

    Err(Error::unsupported(ELEMENT_TOO_LARGE))
}

/// A time element written as its digits.
fn write_element(out: &mut String, element: &Value) -> core::result::Result<(), &'static str> {
    use core::fmt::Write;

    match element {
        Value::Integer(number) => {
            let _ = write!(out, "{number}");

            Ok(())
        }
        #[cfg(feature = "bigint")]
        Value::BigInteger(_) => {
            let number = crate::bigint::to_big(element).ok_or("Expected an integer.")?;
            let _ = write!(out, "{number}");

            Ok(())
        }
        _ => Err(ELEMENT_TOO_LARGE),
    }
}

/// A number-of-digits or a fractional-value that a decoder read, which
/// is no more than an `i128` if it is valid.
pub(crate) fn fraction_field(value: Value) -> core::result::Result<i128, &'static str> {
    match value {
        Value::Integer(number) => Ok(number),
        _ => Err("The fractional part of a DURATION is out of range."),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Designator {
    Years,
    Months,
    Weeks,
    Days,
    Hours,
    Minutes,
    Seconds,
}

struct ParsedComp<'a> {
    designator: Designator,
    // The digits, so that BER, DER, JER and XER take a value of any
    // size; only PER and OER need it as a number.
    val: &'a str,
    frac: Option<&'a str>,
}

fn parse_number(s: &str) -> Option<(&str, Option<&str>, Option<char>)> {
    if s.is_empty() {
        return None;
    }
    let (int_part, frac_part, sep) = if let Some(idx) = s.find(['.', ',']) {
        let sep = s.as_bytes()[idx] as char;
        let int_p = &s[..idx];
        let frac_p = &s[idx + 1..];
        if frac_p.is_empty() || !frac_p.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        (int_p, Some(frac_p), Some(sep))
    } else {
        (s, None, None)
    };

    if int_part.is_empty() || !int_part.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if int_part.len() > 1 && int_part.starts_with('0') {
        return None;
    }
    Some((int_part, frac_part, sep))
}

fn parse_duration_internal<'a>(text: &'a str) -> Result<alloc::vec::Vec<ParsedComp<'a>>> {
    if !text.starts_with('P') {
        return Err(bad());
    }
    let rest = &text[1..];
    if rest.is_empty() {
        return Err(bad());
    }

    if rest.ends_with('W')
        && !rest.contains('T')
        && !rest[..rest.len() - 1].contains(['Y', 'M', 'D'])
    {
        let (val, frac, _) = parse_number(&rest[..rest.len() - 1]).ok_or_else(bad)?;
        return Ok(alloc::vec![ParsedComp {
            designator: Designator::Weeks,
            val,
            frac,
        }]);
    }

    if rest.contains('W') {
        return Err(bad());
    }

    let (date_part, time_part, has_t) = match rest.split_once('T') {
        Some((d, t)) => (d, t, true),
        None => (rest, "", false),
    };

    if has_t && time_part.is_empty() {
        return Err(bad());
    }

    let mut comps = alloc::vec::Vec::new();

    let mut pos = 0;
    for (des, letter) in [
        (Designator::Years, 'Y'),
        (Designator::Months, 'M'),
        (Designator::Days, 'D'),
    ] {
        if let Some(idx) = date_part[pos..].find(letter) {
            let actual_idx = pos + idx;
            let (val, frac, _) = parse_number(&date_part[pos..actual_idx]).ok_or_else(bad)?;
            comps.push(ParsedComp {
                designator: des,
                val,
                frac,
            });
            pos = actual_idx + 1;
        }
    }
    if pos != date_part.len() {
        return Err(bad());
    }

    if has_t {
        let mut pos = 0;
        for (des, letter) in [
            (Designator::Hours, 'H'),
            (Designator::Minutes, 'M'),
            (Designator::Seconds, 'S'),
        ] {
            if let Some(idx) = time_part[pos..].find(letter) {
                let actual_idx = pos + idx;
                let (val, frac, _) = parse_number(&time_part[pos..actual_idx]).ok_or_else(bad)?;
                comps.push(ParsedComp {
                    designator: des,
                    val,
                    frac,
                });
                pos = actual_idx + 1;
            }
        }
        if pos != time_part.len() {
            return Err(bad());
        }
    }

    if comps.is_empty() {
        return Err(bad());
    }

    for c in &comps[..comps.len() - 1] {
        if c.frac.is_some() {
            return Err(bad());
        }
    }

    Ok(comps)
}

pub fn validate_duration(text: &str) -> Result<()> {
    parse_duration_internal(text).map(|_| ())
}

pub fn duration_to_canonical(text: &str) -> Result<String> {
    let comps = parse_duration_internal(text)?;
    let last_idx = comps.len() - 1;
    let mut out = String::from("P");
    let mut has_time = false;

    for (idx, c) in comps.iter().enumerate() {
        if c.val != "0" || c.frac.is_some() || idx == last_idx {
            match c.designator {
                Designator::Hours | Designator::Minutes | Designator::Seconds if !has_time => {
                    out.push('T');
                    has_time = true;
                }
                _ => {}
            }
            out.push_str(c.val);
            if let Some(frac) = c.frac {
                out.push('.');
                out.push_str(frac);
            }
            out.push(match c.designator {
                Designator::Years => 'Y',
                Designator::Months => 'M',
                Designator::Weeks => 'W',
                Designator::Days => 'D',
                Designator::Hours => 'H',
                Designator::Minutes => 'M',
                Designator::Seconds => 'S',
            });
        }
    }

    Ok(out)
}

/// The time elements of a duration as DURATION-EQUIVALENT has them
/// (X.680 38.4.4.2), with no zero one but the least significant, as
/// they are the same abstract value (X.680 38.2.3): the name of each,
/// its digits, and those of its fractional part.
pub fn duration_equivalent(
    text: &str,
) -> Result<alloc::vec::Vec<(&'static str, &str, Option<&str>)>> {
    let comps = parse_duration_internal(text)?;
    let last_idx = comps.len() - 1;

    Ok(comps
        .iter()
        .enumerate()
        .filter(|(idx, c)| c.val != "0" || c.frac.is_some() || *idx == last_idx)
        .map(|(_, c)| {
            let name = match c.designator {
                Designator::Years => "years",
                Designator::Months => "months",
                Designator::Weeks => "weeks",
                Designator::Days => "days",
                Designator::Hours => "hours",
                Designator::Minutes => "minutes",
                Designator::Seconds => "seconds",
            };

            (name, c.val, c.frac)
        })
        .collect())
}

pub fn duration_to_components(text: &str) -> Result<DurationComponents> {
    let comps = parse_duration_internal(text)?;
    let last_idx = comps.len() - 1;
    let mut result = DurationComponents::default();

    for (idx, c) in comps.iter().enumerate() {
        if c.val != "0" || c.frac.is_some() || idx == last_idx {
            let val = element_of_digits(c.val)?;
            match c.designator {
                Designator::Years => result.years = Some(val),
                Designator::Months => result.months = Some(val),
                Designator::Weeks => result.weeks = Some(val),
                Designator::Days => result.days = Some(val),
                Designator::Hours => result.hours = Some(val),
                Designator::Minutes => result.minutes = Some(val),
                Designator::Seconds => result.seconds = Some(val),
            }
            if let Some(frac) = c.frac {
                if frac.len() > MAX_FRACTION_DIGITS {
                    return Err(Error::encode(
                        "A fractional part of a DURATION can have at most 38 digits in PER and OER.",
                    ));
                }
                let fv = frac.parse::<i128>().map_err(|_| bad())?;
                result.fractional_part = Some((frac.len() as i128, fv));
            }
        }
    }

    Ok(result)
}

/// Checks the fields of a decoded DURATION-INTERVAL-ENCODING as X.691
/// 32.6 and X.696 29.4 have them.
fn check_components(c: &DurationComponents) -> core::result::Result<(), &'static str> {
    let present = [
        &c.years, &c.months, &c.weeks, &c.days, &c.hours, &c.minutes, &c.seconds,
    ];
    let present: alloc::vec::Vec<&Value> = present.into_iter().flatten().collect();
    let negative = |value: &Value| match value {
        Value::Integer(number) => *number < 0,
        Value::BigInteger(bytes) => bytes.first().is_some_and(|byte| byte & 0x80 != 0),
        _ => true,
    };

    if present.iter().any(|value| negative(value)) {
        return Err("A time element of a DURATION is negative.");
    }

    // A zero time element is absent unless it is the least significant
    // one (X.691 32.6.3, X.696 29.4.4).
    if let Some((_, rest)) = present.split_last()
        && rest.contains(&&Value::Integer(0))
    {
        return Err("A time element of a DURATION is zero and not its least significant one.");
    }

    if let Some((digits, value)) = c.fractional_part {
        if !(1..=MAX_FRACTION_DIGITS as i128).contains(&digits) {
            return Err(
                "The number of digits of the fractional part of a DURATION is out of range.",
            );
        }
        if value < 0 || value >= 10i128.pow(digits as u32) {
            return Err("The fractional value of a DURATION has more digits than it says.");
        }
    }

    Ok(())
}

pub fn components_to_duration(
    c: &DurationComponents,
) -> core::result::Result<String, &'static str> {
    use core::fmt::Write;
    check_components(c)?;
    if let Some(weeks) = &c.weeks {
        if c.years.is_some()
            || c.months.is_some()
            || c.days.is_some()
            || c.hours.is_some()
            || c.minutes.is_some()
            || c.seconds.is_some()
        {
            return Err("Weeks cannot be combined with other time elements in a DURATION.");
        }
        let mut out = String::from("P");
        write_element(&mut out, weeks)?;
        if let Some((width, fv)) = c.fractional_part {
            out.push('.');
            let _ = write!(out, "{:0width$}", fv, width = width as usize);
        }
        out.push('W');
        return Ok(out);
    }

    let mut least = None;
    for (des, is_some) in [
        (Designator::Seconds, c.seconds.is_some()),
        (Designator::Minutes, c.minutes.is_some()),
        (Designator::Hours, c.hours.is_some()),
        (Designator::Days, c.days.is_some()),
        (Designator::Months, c.months.is_some()),
        (Designator::Years, c.years.is_some()),
    ] {
        if is_some {
            least = Some(des);
            break;
        }
    }

    let Some(least) = least else {
        return Err("Expected at least one time element in a DURATION.");
    };

    let mut out = String::from("P");
    for (des, val_opt, letter) in [
        (Designator::Years, &c.years, 'Y'),
        (Designator::Months, &c.months, 'M'),
        (Designator::Days, &c.days, 'D'),
    ] {
        if let Some(val) = val_opt {
            write_element(&mut out, val)?;
            if least == des
                && let Some((width, fv)) = c.fractional_part
            {
                out.push('.');
                let _ = write!(out, "{:0width$}", fv, width = width as usize);
            }
            out.push(letter);
        }
    }

    let has_time = c.hours.is_some() || c.minutes.is_some() || c.seconds.is_some();
    if has_time {
        out.push('T');
        for (des, val_opt, letter) in [
            (Designator::Hours, &c.hours, 'H'),
            (Designator::Minutes, &c.minutes, 'M'),
            (Designator::Seconds, &c.seconds, 'S'),
        ] {
            if let Some(val) = val_opt {
                write_element(&mut out, val)?;
                if least == des
                    && let Some((width, fv)) = c.fractional_part
                {
                    out.push('.');
                    let _ = write!(out, "{:0width$}", fv, width = width as usize);
                }
                out.push(letter);
            }
        }
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn naive(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> DateTime {
        DateTime {
            date: Date { year, month, day },
            time: Time {
                hour,
                minute,
                second,
                microsecond: 0,
                offset: None,
            },
        }
    }

    #[test]
    fn utc_time() {
        let value = naive(2018, 6, 11, 11, 4, 59);

        assert_eq!(utc_time_to_string(&value, false), "180611110459Z");
        assert_eq!(utc_time_from_string("180611110459Z", false), Ok(value));
        assert_eq!(
            utc_time_from_string("1806111104Z", false),
            Ok(naive(2018, 6, 11, 11, 4, 0))
        );
        assert_eq!(
            utc_time_to_string(&naive(2018, 6, 11, 11, 4, 0), false),
            "1806111104Z"
        );
        assert!(utc_time_from_string("180611110459", false).is_err());
        assert!(utc_time_from_string("18061111045", false).is_err());
        assert_eq!(
            utc_time_from_string("491231235959Z", false)
                .unwrap()
                .date
                .year,
            2049
        );
        assert_eq!(
            utc_time_from_string("500101000000Z", false)
                .unwrap()
                .date
                .year,
            2050
        );
        assert_eq!(
            utc_time_from_string("700101000000Z", false)
                .unwrap()
                .date
                .year,
            1970
        );

        let aware = utc_time_from_string("180611110459+0130", false).unwrap();

        assert_eq!(aware.time.offset, Some(90));
        assert_eq!(utc_time_to_string(&aware, false), "180611110459+0130");
        // DER: in UTC.
        assert_eq!(utc_time_to_string(&aware, true), "180611093459Z");
    }

    #[test]
    fn generalized_time() {
        let value = naive(2018, 1, 31, 5, 0, 47);

        assert_eq!(generalized_time_to_string(&value, false), "20180131050047");
        assert_eq!(
            generalized_time_from_string("20180131050047", false),
            Ok(value)
        );

        let utc = generalized_time_from_string("20180131050047Z", false).unwrap();

        assert_eq!(utc.time.offset, Some(0));
        assert_eq!(generalized_time_to_string(&utc, false), "20180131050047Z");

        let fraction = generalized_time_from_string("20180131050047.5Z", false).unwrap();

        assert_eq!(fraction.time.microsecond, 500_000);
        assert_eq!(
            generalized_time_to_string(&fraction, true),
            "20180131050047.5Z"
        );
        assert!(generalized_time_from_string("20180131050047.50Z", true).is_err());
        assert!(generalized_time_from_string("20180131050047", true).is_err());
    }

    #[test]
    fn other_times() {
        let date = Date {
            year: 1985,
            month: 4,
            day: 12,
        };

        assert_eq!(date_to_string(&date), "19850412");
        assert_eq!(date_from_string("19850412"), Ok(date));
        assert!(date_from_string("19850230").is_err());

        let time = Time {
            hour: 15,
            minute: 27,
            second: 46,
            microsecond: 0,
            offset: None,
        };

        assert_eq!(time_of_day_to_string(&time).as_deref(), Ok("152746"));
        assert_eq!(time_of_day_from_string("152746"), Ok(time));
        assert_eq!(
            date_time_to_string(&naive(1985, 4, 12, 15, 27, 30)).as_deref(),
            Ok("19850412152730")
        );
        assert_eq!(
            date_time_from_string("19850412152730"),
            Ok(naive(1985, 4, 12, 15, 27, 30))
        );
    }

    #[test]
    fn utc_conversion() {
        let mut value = naive(2018, 1, 1, 0, 30, 0);

        value.time.offset = Some(60);

        // 00:30 at UTC+1 is 23:30 the day before in UTC.
        assert_eq!(to_utc(&value), naive(2017, 12, 31, 23, 30, 0));
    }

    #[test]
    fn every_day_is_the_day_of_its_seconds() {
        for year in 1..=9999 {
            for month in 1..=12 {
                for day in 1..=days_in(year, month) {
                    let value = naive(year, month, day, 23, 59, 59);

                    assert_eq!(from_seconds(to_seconds(&value), 0), value);
                }
            }
        }
    }

    #[test]
    fn duration_parsing_and_canonical() {
        assert!(validate_duration("P1Y2M3DT4H5M6S").is_ok());
        assert!(validate_duration("P2W").is_ok());
        assert!(validate_duration("PT0.5S").is_ok());
        assert!(validate_duration("P0Y29M0D").is_ok());
        assert!(validate_duration("1Y").is_err());
        assert!(validate_duration("P").is_err());
        assert!(validate_duration("PT").is_err());
        assert!(validate_duration("P1YT").is_err());
        assert!(validate_duration("P1W2D").is_err());
        assert!(validate_duration("P01Y").is_err());

        assert_eq!(duration_to_canonical("P0Y29M0D").unwrap(), "P29M0D");
        assert_eq!(duration_to_canonical("P1,5Y").unwrap(), "P1.5Y");
        assert_eq!(duration_to_canonical("P0Y0M").unwrap(), "P0M");

        let c = duration_to_components("P1Y2M3DT4H5M6S").unwrap();
        assert_eq!(components_to_duration(&c).unwrap(), "P1Y2M3DT4H5M6S");

        let c_frac = duration_to_components("PT0.5S").unwrap();
        assert_eq!(c_frac.seconds, Some(Value::Integer(0)));
        assert_eq!(c_frac.fractional_part, Some((1, 5)));
        assert_eq!(components_to_duration(&c_frac).unwrap(), "PT0.5S");

        let c_weeks = duration_to_components("P2W").unwrap();
        assert_eq!(c_weeks.weeks, Some(Value::Integer(2)));
        assert_eq!(components_to_duration(&c_weeks).unwrap(), "P2W");
    }
}
