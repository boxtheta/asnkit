// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The runtime of asnkit for the web and JavaScript: the runtime for
//! Rust, built as WebAssembly with `wasm-bindgen`.
//!
//! It loads `.asni` files and encodes and decodes with them, as the
//! runtime for Python does, with the values of JavaScript (see
//! `convert`). The JavaScript of `js/` loads the WebAssembly, in a
//! browser or in Node.js, and is what is used.

use asnkitrt::container::{Binding, Checks, KEY_ID_SIZE, KEY_SIZE, Key, Nonce, Signature};
use asnkitrt::runtime::{Options, Specification as Inner};
use asnkitrt::types::{Codec, TypeId};
use asnkitrt::value::Value;
use js_sys::{Array, Function, Uint8Array};
use wasm_bindgen::prelude::*;

mod convert;

/// An error of JavaScript named `name`, as `EncodeError`.
fn error(name: &str, message: &str) -> JsValue {
    let error = js_sys::Error::new(message);

    error.set_name(name);
    error.into()
}

fn type_error(message: &str) -> JsValue {
    js_sys::TypeError::new(message).into()
}

/// The error of JavaScript of an error of the runtime.
fn runtime_error(runtime: asnkitrt::Error) -> JsValue {
    use asnkitrt::Error::*;

    let message = runtime.to_string();

    match runtime {
        Encode(_) => error("EncodeError", &message),
        Decode(_, offset) => {
            let decode = error("DecodeError", &message);
            let _ = js_sys::Reflect::set(&decode, &"offset".into(), &JsValue::from(offset));

            decode
        }
        Constraints(_) => error("ConstraintsError", &message),
        Unsupported(_) => error("UnsupportedError", &message),
        NoSuchType => error("NoSuchTypeError", &message),
        // Everything else is about the file.
        _ => error("LoadError", &message),
    }
}

fn codec_of(name: &str) -> Result<Codec, JsValue> {
    Ok(match name {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "uper" => Codec::Uper,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        _ => {
            return Err(type_error(&format!(
                "Unsupported codec '{name}': expected ber, der, oer, per, uper, jer or xer."
            )));
        }
    })
}

/// The field `key` of an options object, which may be `undefined`.
fn option(options: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    if options.is_undefined() || options.is_null() {
        return Ok(JsValue::UNDEFINED);
    }

    if !options.is_object() {
        return Err(type_error("Expected the options to be an object."));
    }

    js_sys::Reflect::get(options, &key.into())
}

fn flag(options: &JsValue, key: &str) -> Result<bool, JsValue> {
    let value = option(options, key)?;

    if value.is_undefined() {
        return Ok(false);
    }

    value
        .as_bool()
        .ok_or_else(|| type_error(&format!("Expected '{key}' to be a boolean.")))
}

fn indent(options: &JsValue) -> Result<Option<usize>, JsValue> {
    let value = option(options, "indent")?;

    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }

    match value.as_f64() {
        Some(number) if number >= 0.0 && number.fract() == 0.0 && number <= 1024.0 => {
            Ok(Some(number as usize))
        }
        _ => Err(type_error(
            "Expected 'indent' to be a whole number from 0 to 1024.",
        )),
    }
}

fn text(options: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    let value = option(options, key)?;

    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }

    value
        .as_string()
        .map(Some)
        .ok_or_else(|| type_error(&format!("Expected '{key}' to be a string.")))
}

fn bytes_of<const N: usize>(value: &JsValue, what: &str) -> Result<[u8; N], JsValue> {
    value
        .dyn_ref::<Uint8Array>()
        .and_then(|bytes| bytes.to_vec().try_into().ok())
        .ok_or_else(|| type_error(&format!("Expected {what} to be a Uint8Array of {N} bytes.")))
}

/// Keys given as `[id, key]` pairs, as a `Map` or an array of them.
fn keys(options: &JsValue, key: &str) -> Result<Option<Vec<Key>>, JsValue> {
    let value = option(options, key)?;

    if value.is_undefined() || value.is_null() {
        return Ok(None);
    }

    let iterator = js_sys::try_iter(&value)?
        .ok_or_else(|| type_error(&format!("Expected '{key}' to be [id, key] pairs.")))?;
    let mut found = Vec::new();

    for pair in iterator {
        let pair: Array = pair?
            .dyn_into()
            .map_err(|_| type_error(&format!("Expected '{key}' to be [id, key] pairs.")))?;

        found.push(Key {
            id: bytes_of::<KEY_ID_SIZE>(&pair.get(0), "a key id")?,
            key: bytes_of::<KEY_SIZE>(&pair.get(1), "a key")?,
        });
    }

    Ok(Some(found))
}

fn data_of(data: &JsValue) -> Result<Vec<u8>, JsValue> {
    if let Some(bytes) = data.dyn_ref::<Uint8Array>() {
        Ok(bytes.to_vec())
    } else if let Some(text) = data.as_string() {
        // The text of JER and XER.
        Ok(text.into_bytes())
    } else {
        Err(type_error(
            "Expected the data to be a Uint8Array or a string.",
        ))
    }
}

/// A compiled specification, loaded for a codec.
#[wasm_bindgen]
pub struct Specification {
    inner: Inner,
    codec: String,
}

#[wasm_bindgen]
impl Specification {
    /// Loads the contents of an `.asni` file for `codec`, `ber` if not
    /// given. The options are those of `asnkitrt.load_asni()` of the
    /// runtime for Python, in camel case: `numericEnums`, `lenient`,
    /// `nonce` (a number, or a function that tells if a nonce is
    /// accepted), `acceptAnyNonce`, `boundTo`, `acceptAnyBinding`,
    /// `verificationKeys`, `acceptAnySignature` and `decryptionKeys`. Keys
    /// are `[id, key]` pairs of `Uint8Array`s, as a `Map` or an array.
    #[wasm_bindgen(constructor)]
    pub fn new(
        asni: &[u8],
        codec: Option<String>,
        options: JsValue,
    ) -> Result<Specification, JsValue> {
        let codec = codec.unwrap_or_else(|| "ber".into());
        let mut load = Options::new(codec_of(&codec)?);

        load.numeric_enums = flag(&options, "numericEnums")?;
        load.lenient = flag(&options, "lenient")?;

        let verification_keys = keys(&options, "verificationKeys")?;
        let decryption_keys = keys(&options, "decryptionKeys")?;
        let bound_to = text(&options, "boundTo")?;
        let nonce = option(&options, "nonce")?;
        let accept = nonce.dyn_ref::<Function>().cloned();
        let accept_nonce = |value: u32| {
            accept.as_ref().is_some_and(|accept| {
                accept
                    .call1(&JsValue::NULL, &JsValue::from(value))
                    .is_ok_and(|accepted| accepted.is_truthy())
            })
        };
        let signature = match &verification_keys {
            Some(keys) => Signature::Keys(keys),
            None if flag(&options, "acceptAnySignature")? => Signature::Any,
            None => Signature::None,
        };
        let binding = match &bound_to {
            Some(bound_to) => Binding::To(bound_to),
            None if flag(&options, "acceptAnyBinding")? => Binding::Any,
            None => Binding::None,
        };
        let nonce = if accept.is_some() {
            Nonce::Accept(&accept_nonce)
        } else if let Some(nonce) = nonce.as_f64() {
            if !(0.0..=f64::from(u32::MAX)).contains(&nonce) || nonce.fract() != 0.0 {
                return Err(type_error(
                    "Expected 'nonce' to be an unsigned 32-bit integer.",
                ));
            }

            Nonce::Exact(nonce as u32)
        } else if !nonce.is_undefined() && !nonce.is_null() {
            return Err(type_error("Expected 'nonce' to be a number or a function."));
        } else if flag(&options, "acceptAnyNonce")? {
            Nonce::Any
        } else {
            Nonce::None
        };
        let checks = Checks {
            signature,
            nonce,
            decryption_keys: decryption_keys.as_deref(),
            bound_to: binding,
            ..Checks::default()
        };

        Ok(Specification {
            inner: Inner::from_asni(asni, &checks, load).map_err(runtime_error)?,
            codec,
        })
    }

    /// The codec the specification is loaded for.
    #[wasm_bindgen(getter)]
    pub fn codec(&self) -> String {
        self.codec.clone()
    }

    /// The type `name`, of the module `module` of the options if given.
    fn find(&self, name: &str, options: &JsValue) -> Result<TypeId, JsValue> {
        let types = self.inner.types();
        let found = match text(options, "module")? {
            Some(module) => types.find(&module, name),
            None => types.find_unique(name),
        };

        found.ok_or_else(|| runtime_error(asnkitrt::Error::NoSuchType))
    }

    fn value_of(&self, name: &str, value: &JsValue, options: &JsValue) -> Result<Value, JsValue> {
        let id = self.find(name, options)?;

        convert::to_value(self.inner.types(), id, value)
            .map_err(|message| error("EncodeError", &message))
    }

    fn check(&self, name: &str, value: &Value, options: &JsValue) -> Result<(), JsValue> {
        match text(options, "module")? {
            Some(module) => self.inner.check_constraints_in(&module, name, value),
            None => self.inner.check_constraints(name, value),
        }
        .map_err(runtime_error)
    }

    /// Encodes `value` as the type `name`. The options are
    /// `checkConstraints`, to check the value first, `module`, the
    /// module of the type if more than one has a type with that name,
    /// and `indent`, the number of spaces for each level of JER and XER.
    pub fn encode(&self, name: &str, value: JsValue, options: JsValue) -> Result<Vec<u8>, JsValue> {
        let value = self.value_of(name, &value, &options)?;

        if flag(&options, "checkConstraints")? {
            self.check(name, &value, &options)?;
        }

        match (text(&options, "module")?, indent(&options)?) {
            (Some(module), None) => self.inner.encode_in(&module, name, &value),
            (None, None) => self.inner.encode(name, &value),
            (Some(module), Some(indent)) => {
                self.inner.encode_in_indented(&module, name, &value, indent)
            }
            (None, Some(indent)) => self.inner.encode_indented(name, &value, indent),
        }
        .map_err(runtime_error)
    }

    /// Decodes `data`, a `Uint8Array` or, for JER and XER, a string, as
    /// the type `name`. The options are those of `encode()`.
    pub fn decode(&self, name: &str, data: JsValue, options: JsValue) -> Result<JsValue, JsValue> {
        let data = data_of(&data)?;
        let value = match text(&options, "module")? {
            Some(module) => self.inner.decode_in(&module, name, &data),
            None => self.inner.decode(name, &data),
        }
        .map_err(runtime_error)?;

        if flag(&options, "checkConstraints")? {
            self.check(name, &value, &options)?;
        }

        Ok(convert::to_js(&value))
    }

    /// The same as `decode()`, but returns `[value, length]`, with the
    /// number of bytes the value was encoded in. BER and DER only.
    #[wasm_bindgen(js_name = decodeWithLength)]
    pub fn decode_with_length(
        &self,
        name: &str,
        data: JsValue,
        options: JsValue,
    ) -> Result<Array, JsValue> {
        if text(&options, "module")?.is_some() {
            return Err(type_error("decodeWithLength() does not take a module."));
        }

        let (value, length) = self
            .inner
            .decode_with_length(name, &data_of(&data)?)
            .map_err(runtime_error)?;

        if flag(&options, "checkConstraints")? {
            self.check(name, &value, &options)?;
        }

        Ok(Array::of2(&convert::to_js(&value), &JsValue::from(length)))
    }

    /// The length of the first value of `data` in bytes, or `null` if
    /// there is not enough data to tell. BER and DER only.
    #[wasm_bindgen(js_name = decodeLength)]
    pub fn decode_length(&self, data: JsValue) -> Result<JsValue, JsValue> {
        Ok(self
            .inner
            .decode_length(&data_of(&data)?)
            .map_err(runtime_error)?
            .map_or(JsValue::NULL, JsValue::from))
    }

    /// Checks `value` against the constraints of the type `name`, and
    /// throws a `ConstraintsError` if it does not fulfil them. The option
    /// is `module`.
    #[wasm_bindgen(js_name = checkConstraints)]
    pub fn check_constraints(
        &self,
        name: &str,
        value: JsValue,
        options: JsValue,
    ) -> Result<(), JsValue> {
        let value = self.value_of(name, &value, &options)?;

        self.check(name, &value, &options)
    }
}
