// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Values of JavaScript to values of the runtime and back.
//!
//! They are those of the runtime for Python, in the types of
//! JavaScript: a SEQUENCE is an object, a CHOICE `[name, value]`, a BIT
//! STRING `[bytes, bits]`, an OCTET STRING a `Uint8Array` and so on. As
//! an array is both a CHOICE and a SEQUENCE OF, and a REAL of `1.0` is
//! the number `1`, a value is converted as its type says it is.

use asnkitrt::types::{Kind, TypeId, Types, const_matches};
use asnkitrt::value::{Date, DateTime, Time, Value};
use js_sys::{Array, BigInt, Object, Reflect, Uint8Array};
use wasm_bindgen::{JsCast, JsValue};

/// The greatest integer a number of JavaScript holds exactly.
const MAX_SAFE_INTEGER: i128 = (1 << 53) - 1;

/// How deep a value may be nested, as deeper ones are refused rather
/// than to run out of stack.
const MAX_DEPTH: usize = 256;

/// Converts `value` to a value of the type `id`.
pub fn to_value(types: &Types, id: TypeId, value: &JsValue) -> Result<Value, String> {
    Converter { types }.typed(Some(id), value, 0)
}

struct Converter<'a> {
    types: &'a Types,
}

impl Converter<'_> {
    /// Converts a value of the type `id`, or of no known type if `None`,
    /// as in an open type.
    fn typed(&self, id: Option<TypeId>, value: &JsValue, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("The value is nested too deep.".into());
        }

        let Some(id) = id else {
            return self.untyped(value, depth);
        };
        let kind = &self.types.nodes[self.types.nodes[id].concrete].kind;

        match kind {
            Kind::Real => match value.as_f64() {
                Some(number) => Ok(Value::Real(number)),
                None => self.untyped(value, depth),
            },
            Kind::BitString(_) => match pair(value) {
                Some((bytes, bits)) => match (bytes.dyn_ref::<Uint8Array>(), bits.as_f64()) {
                    (Some(bytes), Some(bits)) if bits >= 0.0 && bits.fract() == 0.0 => {
                        Ok(Value::Bits(bytes.to_vec(), bits as usize))
                    }
                    _ => Err("Expected a BIT STRING as [Uint8Array, number of bits].".into()),
                },
                None => self.untyped(value, depth),
            },
            Kind::Choice(members) => match pair(value) {
                Some((name, inner)) => {
                    let Some(name) = name.as_string() else {
                        return Err("Expected a CHOICE as [name, value].".into());
                    };
                    let ty = members
                        .all()
                        .find(|member| member.name == name)
                        .map(|member| member.ty);

                    Ok(Value::Choice(
                        name,
                        Box::new(self.typed(ty, &inner, depth + 1)?),
                    ))
                }
                None => self.untyped(value, depth),
            },
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members)
                if is_plain_object(value) =>
            {
                let mut slots: Vec<Option<(String, Value)>> = Vec::new();
                let mut defined_by = Vec::new();

                for member in members.all() {
                    let inner = get(value, &member.name)?;

                    // An undefined member is one that is not there.
                    if inner.is_undefined() {
                        continue;
                    }

                    // An ANY DEFINED BY is of the type that another member
                    // tells, which is converted first.
                    if let Kind::AnyDefinedBy {
                        value: key,
                        choices,
                    } = &self.types.nodes[self.types.nodes[member.ty].concrete].kind
                        && !choices.is_empty()
                    {
                        defined_by.push((slots.len(), member, key, choices, inner));
                        slots.push(None);
                    } else {
                        slots.push(Some((
                            member.name.clone(),
                            self.typed(Some(member.ty), &inner, depth + 1)?,
                        )));
                    }
                }

                for (slot, member, key, choices, inner) in defined_by {
                    let key = slots
                        .iter()
                        .flatten()
                        .find(|(name, _)| name == key)
                        .map(|(_, value)| value);
                    let ty = key.and_then(|key| {
                        choices
                            .iter()
                            .find(|(candidate, _)| const_matches(candidate, key))
                            .map(|(_, ty)| *ty)
                    });

                    slots[slot] = Some((member.name.clone(), self.typed(ty, &inner, depth + 1)?));
                }

                let mut map: Vec<(String, Value)> = slots.into_iter().flatten().collect();

                // Any other is left for the codec to reject.
                for key in Object::keys(value.unchecked_ref::<Object>()).iter() {
                    let key = key.as_string().unwrap_or_default();

                    if !members.all().any(|member| member.name == key) {
                        let inner = get(value, &key)?;

                        if !inner.is_undefined() {
                            map.push((key, self.untyped(&inner, depth + 1)?));
                        }
                    }
                }

                Ok(Value::Map(map))
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) if Array::is_array(value) => {
                let mut list = Vec::new();

                for item in value.unchecked_ref::<Array>().iter() {
                    list.push(self.typed(Some(*element), &item, depth + 1)?);
                }

                Ok(Value::List(list))
            }
            Kind::Utc | Kind::Generalized | Kind::DateTime if is_object(value) => {
                Ok(Value::DateTime(date_time(value)?))
            }
            Kind::Date if is_object(value) => Ok(Value::Date(date(value)?)),
            Kind::TimeOfDay if is_object(value) => Ok(Value::Time(time(value)?)),
            _ => self.untyped(value, depth),
        }
    }

    /// Converts a value as the type of JavaScript it is.
    fn untyped(&self, value: &JsValue, depth: usize) -> Result<Value, String> {
        if depth > MAX_DEPTH {
            return Err("The value is nested too deep.".into());
        }

        if value.is_null() || value.is_undefined() {
            Ok(Value::Null)
        } else if let Some(boolean) = value.as_bool() {
            Ok(Value::Boolean(boolean))
        } else if let Some(number) = value.as_f64() {
            Ok(number_to_value(number))
        } else if value.is_bigint() {
            bigint_to_value(value.unchecked_ref::<BigInt>())
        } else if let Some(text) = value.as_string() {
            Ok(Value::Text(text))
        } else if let Some(bytes) = value.dyn_ref::<Uint8Array>() {
            Ok(Value::Bytes(bytes.to_vec()))
        } else if Array::is_array(value) {
            value
                .unchecked_ref::<Array>()
                .iter()
                .map(|item| self.untyped(&item, depth + 1))
                .collect::<Result<_, _>>()
                .map(Value::List)
        } else if value.is_instance_of::<js_sys::Date>() {
            Ok(Value::DateTime(date_time(value)?))
        } else if is_plain_object(value) {
            let mut map = Vec::new();

            for key in Object::keys(value.unchecked_ref::<Object>()).iter() {
                let key = key.as_string().unwrap_or_default();
                let inner = get(value, &key)?;

                if !inner.is_undefined() {
                    map.push((key, self.untyped(&inner, depth + 1)?));
                }
            }

            Ok(Value::Map(map))
        } else {
            Err("A value of a type that can not be encoded.".into())
        }
    }
}

/// The two items of an array of two items.
fn pair(value: &JsValue) -> Option<(JsValue, JsValue)> {
    let array = value.dyn_ref::<Array>()?;

    (array.length() == 2).then(|| (array.get(0), array.get(1)))
}

fn is_object(value: &JsValue) -> bool {
    value.is_object() && !value.is_function()
}

/// An object that is not an array, bytes or a date.
fn is_plain_object(value: &JsValue) -> bool {
    is_object(value)
        && !Array::is_array(value)
        && !value.is_instance_of::<Uint8Array>()
        && !value.is_instance_of::<js_sys::Date>()
}

fn get(value: &JsValue, key: &str) -> Result<JsValue, String> {
    Reflect::get(value, &JsValue::from_str(key)).map_err(|_| format!("Can not read '{key}'."))
}

fn number_to_value(number: f64) -> Value {
    // An integer is an INTEGER, as far as it is exact.
    if number.fract() == 0.0 && number.abs() <= MAX_SAFE_INTEGER as f64 {
        Value::Integer(number as i128)
    } else {
        Value::Real(number)
    }
}

/// The integer of a `BigInt`, by its digits in base 16.
fn bigint_to_value(value: &BigInt) -> Result<Value, String> {
    let text: String = value
        .to_string(16)
        .map_err(|_| "Can not read a BigInt.")?
        .into();
    let (negative, digits) = match text.strip_prefix('-') {
        Some(digits) => (true, digits),
        None => (false, text.as_str()),
    };
    // The magnitude, most significant byte first, with a byte for the
    // sign.
    let padded = if digits.len() % 2 == 1 {
        format!("0{digits}")
    } else {
        digits.to_string()
    };
    let mut bytes = vec![0u8];

    for index in (0..padded.len()).step_by(2) {
        bytes.push(
            u8::from_str_radix(&padded[index..index + 2], 16)
                .map_err(|_| "Can not read a BigInt.")?,
        );
    }

    if negative {
        // The two's complement: inverted, plus one.
        for byte in bytes.iter_mut() {
            *byte = !*byte;
        }

        for byte in bytes.iter_mut().rev() {
            let (sum, carry) = byte.overflowing_add(1);

            *byte = sum;

            if !carry {
                break;
            }
        }
    }

    Ok(Value::from_twos_complement(&bytes))
}

/// The `BigInt` of an integer in two's complement.
fn twos_complement_to_bigint(bytes: &[u8]) -> JsValue {
    let negative = bytes.first().is_some_and(|byte| byte & 0x80 != 0);
    let mut magnitude = bytes.to_vec();

    if negative {
        for byte in magnitude.iter_mut() {
            *byte = !*byte;
        }

        for byte in magnitude.iter_mut().rev() {
            let (sum, carry) = byte.overflowing_add(1);

            *byte = sum;

            if !carry {
                break;
            }
        }
    }

    let hex: String = magnitude.iter().map(|byte| format!("{byte:02x}")).collect();
    let value: JsValue = BigInt::new(&JsValue::from_str(&format!("0x{hex}")))
        .map(Into::into)
        .unwrap_or(JsValue::NULL);

    if negative { -&value } else { value }
}

fn integer_to_js(value: i128) -> JsValue {
    if value.abs() <= MAX_SAFE_INTEGER {
        JsValue::from_f64(value as f64)
    } else {
        twos_complement_to_bigint(&asnkitrt::value::integer_to_twos_complement(value))
    }
}

fn field(value: &JsValue, key: &str) -> Result<Option<f64>, String> {
    let inner = get(value, key)?;

    if inner.is_undefined() || inner.is_null() {
        return Ok(None);
    }

    match inner.as_f64() {
        Some(number) if number.fract() == 0.0 => Ok(Some(number)),
        _ => Err(format!("Expected '{key}' to be an integer.")),
    }
}

fn required(value: &JsValue, key: &str) -> Result<f64, String> {
    field(value, key)?.ok_or_else(|| format!("Expected '{key}'."))
}

fn date(value: &JsValue) -> Result<Date, String> {
    if let Some(date) = value.dyn_ref::<js_sys::Date>() {
        return Ok(Date {
            year: date.get_utc_full_year() as i32,
            month: date.get_utc_month() as u8 + 1,
            day: date.get_utc_date() as u8,
        });
    }

    Ok(Date {
        year: required(value, "year")? as i32,
        month: required(value, "month")? as u8,
        day: required(value, "day")? as u8,
    })
}

fn time(value: &JsValue) -> Result<Time, String> {
    if let Some(date) = value.dyn_ref::<js_sys::Date>() {
        return Ok(Time {
            hour: date.get_utc_hours() as u8,
            minute: date.get_utc_minutes() as u8,
            second: date.get_utc_seconds() as u8,
            microsecond: date.get_utc_milliseconds() * 1000,
            offset: Some(0),
        });
    }

    Ok(Time {
        hour: required(value, "hour")? as u8,
        minute: field(value, "minute")?.unwrap_or(0.0) as u8,
        second: field(value, "second")?.unwrap_or(0.0) as u8,
        microsecond: field(value, "microsecond")?.unwrap_or(0.0) as u32,
        offset: field(value, "offset")?.map(|offset| offset as i32),
    })
}

/// A date and a time from an object with the fields of both, or from a
/// `Date`, which is in UTC.
fn date_time(value: &JsValue) -> Result<DateTime, String> {
    Ok(DateTime {
        date: date(value)?,
        time: time(value)?,
    })
}

fn set(object: &Object, key: &str, value: &JsValue) {
    // Setting a property of a new plain object does not fail.
    let _ = Reflect::set(object, &JsValue::from_str(key), value);
}

fn date_fields(object: &Object, date: &Date) {
    set(object, "year", &JsValue::from(date.year));
    set(object, "month", &JsValue::from(date.month));
    set(object, "day", &JsValue::from(date.day));
}

fn time_fields(object: &Object, time: &Time) {
    set(object, "hour", &JsValue::from(time.hour));
    set(object, "minute", &JsValue::from(time.minute));
    set(object, "second", &JsValue::from(time.second));
    set(object, "microsecond", &JsValue::from(time.microsecond));
    set(
        object,
        "offset",
        &time.offset.map(JsValue::from).unwrap_or(JsValue::NULL),
    );
}

/// Converts a decoded value to JavaScript.
pub fn to_js(value: &Value) -> JsValue {
    match value {
        Value::Null => JsValue::NULL,
        Value::Boolean(boolean) => JsValue::from_bool(*boolean),
        Value::Integer(integer) => integer_to_js(*integer),
        Value::BigInteger(bytes) => twos_complement_to_bigint(bytes),
        Value::Real(number) => JsValue::from_f64(*number),
        Value::Text(text) => JsValue::from_str(text),
        Value::Bytes(bytes) => Uint8Array::from(bytes.as_slice()).into(),
        Value::Bits(bytes, bits) => Array::of2(
            &Uint8Array::from(bytes.as_slice()),
            &JsValue::from_f64(*bits as f64),
        )
        .into(),
        Value::List(items) => items.iter().map(to_js).collect::<Array>().into(),
        Value::Map(members) => {
            let object = Object::new();

            for (name, member) in members {
                set(&object, name, &to_js(member));
            }

            object.into()
        }
        Value::Choice(name, inner) => Array::of2(&JsValue::from_str(name), &to_js(inner)).into(),
        Value::DateTime(date_time) => {
            let object = Object::new();

            date_fields(&object, &date_time.date);
            time_fields(&object, &date_time.time);

            object.into()
        }
        Value::Date(date) => {
            let object = Object::new();

            date_fields(&object, date);

            object.into()
        }
        Value::Time(time) => {
            let object = Object::new();

            time_fields(&object, time);

            object.into()
        }
    }
}
