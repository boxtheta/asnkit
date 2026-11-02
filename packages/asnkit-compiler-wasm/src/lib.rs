// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The compiler of asnkit for the web and JavaScript: the library
//! `asnkit-compiler`, built as WebAssembly with `wasm-bindgen`.
//!
//! It compiles the text of an ASN.1 specification to the contents of an
//! `.asni` file, which the runtimes load, and parses it to those of an
//! `.asnst` file. There are no files to read and no plugins to run: the
//! JavaScript of `js/` gives it the text and does what it wants with
//! what it makes.

use asnkit_compiler::{Input, ModuleSummary, compile_to_asni_with_summary, parse_to_asnst};
use js_sys::{Array, Object, Reflect, Uint8Array};
use wasm_bindgen::prelude::*;

/// An error of JavaScript named `name`, as `CompileError`.
fn error(name: &str, message: &str) -> JsValue {
    let error = js_sys::Error::new(message);

    error.set_name(name);
    error.into()
}

fn type_error(message: &str) -> JsValue {
    js_sys::TypeError::new(message).into()
}

/// The error of JavaScript of an error of the compiler, named by its
/// kind: `ParseError`, `CompileError` or `Error`.
fn compiler_error(compiler: asnkit_compiler::Error) -> JsValue {
    error(&compiler.kind, &compiler.message)
}

/// The field `key` of an options object, which may be `undefined`.
fn option(options: &JsValue, key: &str) -> Result<JsValue, JsValue> {
    if options.is_undefined() || options.is_null() {
        return Ok(JsValue::UNDEFINED);
    }

    if !options.is_object() {
        return Err(type_error("Expected the options to be an object."));
    }

    Reflect::get(options, &key.into())
}

fn absent(value: &JsValue) -> bool {
    value.is_undefined() || value.is_null()
}

fn text(options: &JsValue, key: &str) -> Result<Option<String>, JsValue> {
    let value = option(options, key)?;

    if absent(&value) {
        return Ok(None);
    }

    value
        .as_string()
        .map(Some)
        .ok_or_else(|| type_error(&format!("Expected '{key}' to be a string.")))
}

/// The options of what `.asni` files no longer have: a nonce, a
/// signature and encryption. They are rejected rather than ignored, so
/// that nobody is given a file that is not signed when they asked for
/// one that is.
const REMOVED_OPTIONS: [&str; 5] = [
    "nonce",
    "signingKey",
    "signingKeyId",
    "encryptionKey",
    "encryptionKeyId",
];

fn reject_removed(options: &JsValue) -> Result<(), JsValue> {
    for key in REMOVED_OPTIONS {
        if !absent(&option(options, key)?) {
            return Err(type_error(&format!(
                "'{key}' is no longer supported: .asni files have no nonce and are not signed or encrypted."
            )));
        }
    }

    Ok(())
}

fn compile_to_asni(
    specification: String,
    options: &JsValue,
) -> Result<(Vec<u8>, Vec<ModuleSummary>), JsValue> {
    reject_removed(options)?;

    compile_to_asni_with_summary(&Input::String(specification), text(options, "boundTo")?)
        .map_err(compiler_error)
}

/// Compiles the text of a specification, of one module or more, to the
/// contents of an `.asni` file, and tells what its modules have. The
/// options are those of `compile()`.
#[wasm_bindgen(js_name = compileWithSummary)]
pub fn compile_with_summary(specification: String, options: JsValue) -> Result<Object, JsValue> {
    let (asni, summary) = compile_to_asni(specification, &options)?;

    let modules = Array::new();

    for module in summary {
        let entry = Object::new();

        Reflect::set(&entry, &"name".into(), &module.name.into())?;
        Reflect::set(&entry, &"types".into(), &module.types.into())?;
        Reflect::set(
            &entry,
            &"objectClasses".into(),
            &module.object_classes.into(),
        )?;
        modules.push(&entry);
    }

    let result = Object::new();

    Reflect::set(&result, &"asni".into(), &Uint8Array::from(asni.as_slice()))?;
    Reflect::set(&result, &"modules".into(), &modules)?;

    Ok(result)
}

/// Compiles the text of a specification, of one module or more, to the
/// contents of an `.asni` file. The options are those of `asnkitcc
/// compile`, in camel case: `boundTo`.
#[wasm_bindgen]
pub fn compile(specification: String, options: JsValue) -> Result<Vec<u8>, JsValue> {
    Ok(compile_to_asni(specification, &options)?.0)
}

/// Parses the text of a specification to the contents of an `.asnst`
/// file, the syntax tree that plugins generate code from.
#[wasm_bindgen]
pub fn parse(specification: String) -> Result<Vec<u8>, JsValue> {
    parse_to_asnst(&Input::String(specification)).map_err(compiler_error)
}

/// The version of the compiler, which it writes in what it makes.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
