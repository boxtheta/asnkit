// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Values: an `asnkit_value *` is a `Value` of the runtime.
//!
//! A value that is made or decoded is owned by C, and freed with
//! `asnkit_value_free()`, unless it is given to a list, a map or a
//! choice, which own it then. What the accessors give is borrowed from
//! the value it is in, and valid as long as that is and is not changed.
//! Text is given with its length and has no NUL at its end, as it may
//! have one in it.

use std::ffi::{c_char, c_int};

use asnkitrt::value::{self, Date, DateTime, Time, Value};

use crate::{
    ASNKIT_ERROR_ARGUMENT, ASNKIT_ERROR_KIND, ASNKIT_ERROR_RANGE, Failure, Result, bytes, failure,
    guard, guard_handle, handle, out,
};

pub const ASNKIT_NULL: c_int = 0;
pub const ASNKIT_BOOLEAN: c_int = 1;
pub const ASNKIT_INTEGER: c_int = 2;
pub const ASNKIT_REAL: c_int = 3;
pub const ASNKIT_TEXT: c_int = 4;
pub const ASNKIT_BYTES: c_int = 5;
pub const ASNKIT_BITS: c_int = 6;
pub const ASNKIT_LIST: c_int = 7;
pub const ASNKIT_MAP: c_int = 8;
pub const ASNKIT_CHOICE: c_int = 9;
pub const ASNKIT_DATE_TIME: c_int = 10;
pub const ASNKIT_DATE: c_int = 11;
pub const ASNKIT_TIME: c_int = 12;

/// A date, as in the header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CDate {
    pub year: i32,
    pub month: u8,
    pub day: u8,
}

/// A time of day, as in the header.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct CTime {
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub microsecond: u32,
    pub has_offset: c_int,
    /// Minutes east of UTC.
    pub offset: i32,
}

impl From<CDate> for Date {
    fn from(date: CDate) -> Date {
        Date {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }
}

impl From<Date> for CDate {
    fn from(date: Date) -> CDate {
        CDate {
            year: date.year,
            month: date.month,
            day: date.day,
        }
    }
}

impl From<CTime> for Time {
    fn from(time: CTime) -> Time {
        Time {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            microsecond: time.microsecond,
            offset: (time.has_offset != 0).then_some(time.offset),
        }
    }
}

impl From<Time> for CTime {
    fn from(time: Time) -> CTime {
        CTime {
            hour: time.hour,
            minute: time.minute,
            second: time.second,
            microsecond: time.microsecond,
            has_offset: time.offset.is_some() as c_int,
            offset: time.offset.unwrap_or(0),
        }
    }
}

fn new(value: Value) -> *mut Value {
    Box::into_raw(Box::new(value))
}

/// Takes a value that C gives to the runtime.
unsafe fn take(value: *mut Value) -> Result<Value> {
    if value.is_null() {
        return Err(failure(ASNKIT_ERROR_ARGUMENT, "Expected a value."));
    }

    Ok(*unsafe { Box::from_raw(value) })
}

unsafe fn value<'a>(value: *const Value) -> Result<&'a Value> {
    unsafe { handle(value, "a value") }
}

unsafe fn text_of<'a>(text: *const c_char, length: usize, what: &str) -> Result<&'a str> {
    std::str::from_utf8(unsafe { bytes(text.cast(), length, what) }?)
        .map_err(|_| failure(ASNKIT_ERROR_ARGUMENT, format!("Expected {what} in UTF-8.")))
}

fn wrong_kind(expected: &str) -> Failure {
    failure(ASNKIT_ERROR_KIND, format!("The value is not {expected}."))
}

/// Gives text as a pointer and a length.
unsafe fn give_text(text: &str, out_text: *mut *const c_char, length: *mut usize) -> Result<()> {
    *unsafe { out(out_text, "the text") }? = text.as_ptr().cast();
    *unsafe { out(length, "the length") }? = text.len();
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_null() -> *mut Value {
    guard_handle(|| Ok(new(Value::Null)))
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_boolean(boolean: c_int) -> *mut Value {
    guard_handle(|| Ok(new(Value::Boolean(boolean != 0))))
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_integer(integer: i64) -> *mut Value {
    guard_handle(|| Ok(new(Value::Integer(i128::from(integer)))))
}

/// An integer of any size from its two's complement, most significant
/// byte first. No bytes are 0.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_new_integer_bytes(
    data: *const u8,
    length: usize,
) -> *mut Value {
    guard_handle(|| {
        let data = unsafe { bytes(data, length, "the bytes of the integer") }?;

        Ok(new(Value::from_twos_complement(data)))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_real(real: f64) -> *mut Value {
    guard_handle(|| Ok(new(Value::Real(real))))
}

/// A character string, an OBJECT IDENTIFIER as `1.2.3` or an ENUMERATED
/// by name: `length` bytes of UTF-8.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_new_text(text: *const c_char, length: usize) -> *mut Value {
    guard_handle(|| {
        let text = unsafe { text_of(text, length, "the text") }?;

        Ok(new(Value::text(text)))
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_new_bytes(data: *const u8, length: usize) -> *mut Value {
    guard_handle(|| {
        let data = unsafe { bytes(data, length, "the bytes") }?;

        Ok(new(Value::Bytes(data.to_vec())))
    })
}

/// A BIT STRING of the first `bits` bits of the `length` bytes at
/// `data`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_new_bits(
    data: *const u8,
    length: usize,
    bits: usize,
) -> *mut Value {
    guard_handle(|| {
        let data = unsafe { bytes(data, length, "the bytes") }?;

        if bits.div_ceil(8) != length {
            return Err(failure(
                ASNKIT_ERROR_RANGE,
                "Expected as many bytes as the bits take.",
            ));
        }

        Ok(new(Value::Bits(data.to_vec(), bits)))
    })
}

/// An empty SEQUENCE OF or SET OF, for `asnkit_list_push()`.
#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_list() -> *mut Value {
    guard_handle(|| Ok(new(Value::List(Vec::new()))))
}

/// An empty SEQUENCE or SET, for `asnkit_map_set()`.
#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_map() -> *mut Value {
    guard_handle(|| Ok(new(Value::Map(Vec::new()))))
}

/// A CHOICE of the alternative `name`, which takes `inner`, even if it
/// fails.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_new_choice(
    name: *const c_char,
    inner: *mut Value,
) -> *mut Value {
    guard_handle(|| {
        let inner = unsafe { take(inner) }?;
        let name = unsafe { crate::text(name, "the name of the alternative") }?;

        Ok(new(Value::Choice(name.into(), Box::new(inner))))
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_date(date: CDate) -> *mut Value {
    guard_handle(|| Ok(new(Value::Date(date.into()))))
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_time(time: CTime) -> *mut Value {
    guard_handle(|| Ok(new(Value::Time(time.into()))))
}

#[unsafe(no_mangle)]
pub extern "C" fn asnkit_value_new_date_time(date: CDate, time: CTime) -> *mut Value {
    guard_handle(|| {
        Ok(new(Value::DateTime(DateTime {
            date: date.into(),
            time: time.into(),
        })))
    })
}

/// A copy of `value`, which C owns.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_clone(value: *const Value) -> *mut Value {
    guard_handle(|| Ok(new(unsafe { self::value(value) }?.clone())))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_free(value: *mut Value) {
    if !value.is_null() {
        drop(unsafe { Box::from_raw(value) });
    }
}

/// Adds `item`, which it takes even if it fails, to the end of a list.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_list_push(list: *mut Value, item: *mut Value) -> c_int {
    guard(|| {
        let item = unsafe { take(item) }?;

        match unsafe { list.as_mut() } {
            Some(Value::List(items)) => items.push(item),
            Some(_) => return Err(wrong_kind("a list")),
            None => return Err(failure(ASNKIT_ERROR_ARGUMENT, "Expected a list.")),
        }

        Ok(())
    })
}

/// Sets the member `name` of a map to `member`, which it takes even if
/// it fails. A member of that name that is there is replaced.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_map_set(
    map: *mut Value,
    name: *const c_char,
    member: *mut Value,
) -> c_int {
    guard(|| {
        let member = unsafe { take(member) }?;
        let name = unsafe { crate::text(name, "the name of the member") }?;
        let members = match unsafe { map.as_mut() } {
            Some(Value::Map(members)) => members,
            Some(_) => return Err(wrong_kind("a map")),
            None => return Err(failure(ASNKIT_ERROR_ARGUMENT, "Expected a map.")),
        };

        match members.iter_mut().find(|(found, _)| found == name) {
            Some((_, found)) => *found = member,
            None => members.push((name.into(), member)),
        }

        Ok(())
    })
}

/// The kind of a value, as `ASNKIT_MAP`, or -1 for null.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_kind(value: *const Value) -> c_int {
    match unsafe { value.as_ref() } {
        None => -1,
        Some(Value::Null) => ASNKIT_NULL,
        Some(Value::Boolean(_)) => ASNKIT_BOOLEAN,
        Some(Value::Integer(_) | Value::BigInteger(_)) => ASNKIT_INTEGER,
        Some(Value::Real(_)) => ASNKIT_REAL,
        Some(Value::Text(_)) => ASNKIT_TEXT,
        Some(Value::Bytes(_)) => ASNKIT_BYTES,
        Some(Value::Bits(..)) => ASNKIT_BITS,
        Some(Value::List(_)) => ASNKIT_LIST,
        Some(Value::Map(_)) => ASNKIT_MAP,
        Some(Value::Choice(..)) => ASNKIT_CHOICE,
        Some(Value::DateTime(_)) => ASNKIT_DATE_TIME,
        Some(Value::Date(_)) => ASNKIT_DATE,
        Some(Value::Time(_)) => ASNKIT_TIME,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_boolean(value: *const Value, boolean: *mut c_int) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Boolean(found) => {
            *unsafe { out(boolean, "the boolean") }? = *found as c_int;
            Ok(())
        }
        _ => Err(wrong_kind("a boolean")),
    })
}

/// An integer that fits in 64 bits; `ASNKIT_ERROR_RANGE` if it does
/// not, and `asnkit_value_integer_bytes()` gives it.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_integer(value: *const Value, integer: *mut i64) -> c_int {
    guard(|| {
        let found = match unsafe { self::value(value) }? {
            Value::Integer(found) => i64::try_from(*found).ok(),
            Value::BigInteger(_) => None,
            _ => return Err(wrong_kind("an integer")),
        };
        let found = found
            .ok_or_else(|| failure(ASNKIT_ERROR_RANGE, "The integer does not fit in 64 bits."))?;

        *unsafe { out(integer, "the integer") }? = found;
        Ok(())
    })
}

/// Writes the two's complement of an integer, most significant byte
/// first and as short as it can be, to the `capacity` bytes at `data`,
/// and its length to `length`. With too little capacity, or none and a
/// null `data`, it writes only the length and returns
/// `ASNKIT_ERROR_RANGE`.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_integer_bytes(
    value: *const Value,
    data: *mut u8,
    capacity: usize,
    length: *mut usize,
) -> c_int {
    guard(|| {
        let found = match unsafe { self::value(value) }? {
            Value::Integer(found) => value::integer_to_twos_complement(*found),
            Value::BigInteger(found) => found.clone(),
            _ => return Err(wrong_kind("an integer")),
        };

        *unsafe { out(length, "the length") }? = found.len();

        if data.is_null() || capacity < found.len() {
            return Err(failure(
                ASNKIT_ERROR_RANGE,
                "The buffer is too small for the integer.",
            ));
        }

        unsafe { std::ptr::copy_nonoverlapping(found.as_ptr(), data, found.len()) };
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_real(value: *const Value, real: *mut f64) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Real(found) => {
            *unsafe { out(real, "the real") }? = *found;
            Ok(())
        }
        _ => Err(wrong_kind("a real")),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_text(
    value: *const Value,
    text: *mut *const c_char,
    length: *mut usize,
) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Text(found) => unsafe { give_text(found, text, length) },
        _ => Err(wrong_kind("text")),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_bytes(
    value: *const Value,
    data: *mut *const u8,
    length: *mut usize,
) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Bytes(found) => {
            *unsafe { out(data, "the bytes") }? = found.as_ptr();
            *unsafe { out(length, "the length") }? = found.len();
            Ok(())
        }
        _ => Err(wrong_kind("bytes")),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_bits(
    value: *const Value,
    data: *mut *const u8,
    length: *mut usize,
    bits: *mut usize,
) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Bits(found, count) => {
            *unsafe { out(data, "the bytes") }? = found.as_ptr();
            *unsafe { out(length, "the length") }? = found.len();
            *unsafe { out(bits, "the number of bits") }? = *count;
            Ok(())
        }
        _ => Err(wrong_kind("bits")),
    })
}

/// The number of items of a list or of members of a map.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_length(value: *const Value, length: *mut usize) -> c_int {
    guard(|| {
        let found = match unsafe { self::value(value) }? {
            Value::List(items) => items.len(),
            Value::Map(members) => members.len(),
            _ => return Err(wrong_kind("a list or a map")),
        };

        *unsafe { out(length, "the length") }? = found;
        Ok(())
    })
}

/// The item `index` of a list, or null if there is none.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_list_get(list: *const Value, index: usize) -> *const Value {
    match unsafe { list.as_ref() } {
        Some(Value::List(items)) => items
            .get(index)
            .map_or(std::ptr::null(), |item| item as *const Value),
        _ => std::ptr::null(),
    }
}

/// The member `name` of a map, or null if it is not there.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_map_get(map: *const Value, name: *const c_char) -> *const Value {
    let Some(map) = (unsafe { map.as_ref() }) else {
        return std::ptr::null();
    };
    let Ok(name) = (unsafe { crate::text(name, "the name of the member") }) else {
        return std::ptr::null();
    };

    map.get(name)
        .map_or(std::ptr::null(), |member| member as *const Value)
}

/// The name and the value of the member `index` of a map, in the order
/// they are in.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_map_member(
    map: *const Value,
    index: usize,
    name: *mut *const c_char,
    name_length: *mut usize,
    member: *mut *const Value,
) -> c_int {
    guard(|| {
        let Value::Map(members) = unsafe { value(map) }? else {
            return Err(wrong_kind("a map"));
        };
        let (found, inner) = members
            .get(index)
            .ok_or_else(|| failure(ASNKIT_ERROR_RANGE, "The map has no member at that index."))?;

        unsafe { give_text(found, name, name_length) }?;
        *unsafe { out(member, "the member") }? = inner;
        Ok(())
    })
}

/// The name of the alternative of a choice, and its value.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_choice_get(
    choice: *const Value,
    name: *mut *const c_char,
    name_length: *mut usize,
    inner: *mut *const Value,
) -> c_int {
    guard(|| {
        let Value::Choice(found, value) = unsafe { self::value(choice) }? else {
            return Err(wrong_kind("a choice"));
        };

        unsafe { give_text(found, name, name_length) }?;
        *unsafe { out(inner, "the value") }? = &**value;
        Ok(())
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_date(value: *const Value, date: *mut CDate) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Date(found) => {
            *unsafe { out(date, "the date") }? = (*found).into();
            Ok(())
        }
        _ => Err(wrong_kind("a date")),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_time(value: *const Value, time: *mut CTime) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::Time(found) => {
            *unsafe { out(time, "the time") }? = (*found).into();
            Ok(())
        }
        _ => Err(wrong_kind("a time")),
    })
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn asnkit_value_date_time(
    value: *const Value,
    date: *mut CDate,
    time: *mut CTime,
) -> c_int {
    guard(|| match unsafe { self::value(value) }? {
        Value::DateTime(found) => {
            *unsafe { out(date, "the date") }? = found.date.into();
            *unsafe { out(time, "the time") }? = found.time.into();
            Ok(())
        }
        _ => Err(wrong_kind("a date and a time")),
    })
}
