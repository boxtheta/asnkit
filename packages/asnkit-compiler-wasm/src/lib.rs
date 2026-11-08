// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The compiler of asnkit for the web and JavaScript: the library
//! `asnkit-compiler`, built as WebAssembly with `wasm-bindgen`.
//!
//! It compiles the text of an ASN.1 specification to the contents of an
//! `.asni` file, which the runtimes load, parses it to those of an
//! `.asnst` file, and makes a JSON Schema, an XML Schema or a DTD of the
//! types of an `.asni` file, as `asnkitcc schema` does. There are no
//! files to read and no plugins to run: the JavaScript of `js/` gives it
//! the text and does what it wants with what it makes.

use asnkit_compiler::{
    CompileOptions, Compiled, Input, compile_to_asni_with_warnings, parse_to_asnst,
};
use asnkitcc::schema::{self, format_of, generate};
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

const UNSIGNED: &str = ".asni files have no nonce and are not signed or encrypted.";
const UNBOUND: &str =
    ".asni files are not bound to anything. They have a CRC-32, which tells a corrupt file.";

/// The options of what `.asni` files no longer have: a nonce, a
/// signature, encryption and a binding, with why. They are rejected
/// rather than ignored, so that nobody is given a file that is not what
/// they asked for.
const REMOVED_OPTIONS: [(&str, &str); 6] = [
    ("nonce", UNSIGNED),
    ("signingKey", UNSIGNED),
    ("signingKeyId", UNSIGNED),
    ("encryptionKey", UNSIGNED),
    ("encryptionKeyId", UNSIGNED),
    ("boundTo", UNBOUND),
];

fn reject_removed(options: &JsValue) -> Result<(), JsValue> {
    for (key, why) in REMOVED_OPTIONS {
        if !absent(&option(options, key)?) {
            return Err(type_error(&format!(
                "'{key}' is no longer supported: {why}"
            )));
        }
    }

    Ok(())
}

fn compile_to_asni(specification: String, options: &JsValue) -> Result<Compiled, JsValue> {
    reject_removed(options)?;

    compile_to_asni_with_warnings(&Input::String(specification), &CompileOptions::default())
        .map_err(compiler_error)
}

/// Compiles the text of a specification, of one module or more, to the
/// contents of an `.asni` file, and tells what its modules have and what
/// in it is likely to cause trouble. The options are those of
/// `compile()`.
#[wasm_bindgen(js_name = compileWithSummary)]
pub fn compile_with_summary(specification: String, options: JsValue) -> Result<Object, JsValue> {
    let Compiled {
        asni,
        modules: summary,
        warnings: found,
    } = compile_to_asni(specification, &options)?;

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

    let warnings = Array::new();

    for warning in found {
        let entry = Object::new();

        Reflect::set(&entry, &"kind".into(), &warning.kind.into())?;
        Reflect::set(&entry, &"message".into(), &warning.message.into())?;
        warnings.push(&entry);
    }

    let result = Object::new();

    Reflect::set(&result, &"asni".into(), &Uint8Array::from(asni.as_slice()))?;
    Reflect::set(&result, &"modules".into(), &modules)?;
    Reflect::set(&result, &"warnings".into(), &warnings)?;

    Ok(result)
}

/// Compiles the text of a specification, of one module or more, to the
/// contents of an `.asni` file. No options are taken yet; `options` is
/// checked for those that were removed.
#[wasm_bindgen]
pub fn compile(specification: String, options: JsValue) -> Result<Vec<u8>, JsValue> {
    Ok(compile_to_asni(specification, &options)?.asni)
}

/// Parses the text of a specification to the contents of an `.asnst`
/// file, the syntax tree that plugins generate code from.
#[wasm_bindgen]
pub fn parse(specification: String) -> Result<Vec<u8>, JsValue> {
    parse_to_asnst(&Input::String(specification)).map_err(compiler_error)
}

/// The names of the types of `schema()`: none, a string or an array of
/// strings.
fn type_names(value: &JsValue) -> Result<Vec<String>, JsValue> {
    const EXPECTED: &str = "Expected 'types' to be a string or an array of strings.";

    if absent(value) {
        return Ok(Vec::new());
    }

    if let Some(name) = value.as_string() {
        return Ok(vec![name]);
    }

    if !Array::is_array(value) {
        return Err(type_error(EXPECTED));
    }

    Array::from(value)
        .iter()
        .map(|name| name.as_string().ok_or_else(|| type_error(EXPECTED)))
        .collect()
}

/// Makes what the values of the types of an `.asni` file are in a text
/// codec, as `asnkitcc schema` does: a JSON Schema of JER, or an XML
/// Schema or a DTD of BASIC-XER. The options are `format`
/// (`json-schema`, the default, `xsd` or `dtd`), `types`, the names of
/// the types the documents are values of, as `Type` or `Module.Type`,
/// all of them if none, `skipUnsupported`, to leave out what a
/// schema can not tell rather than fail, and `specification`, the text
/// the `.asni` file is compiled from, whose pragma `schema_comment` is
/// the comment of the schema. What it cannot make is an `Error` named
/// `SchemaError`.
#[wasm_bindgen]
pub fn schema(asni: &[u8], options: JsValue) -> Result<Object, JsValue> {
    let format = match option(&options, "format")? {
        value if absent(&value) => schema::Format::JsonSchema,
        value => format_of(
            &value
                .as_string()
                .ok_or_else(|| type_error("Expected 'format' to be a string."))?,
        )
        .map_err(|message| type_error(&message))?,
    };
    let types = type_names(&option(&options, "types")?)?;
    let skip_unsupported = match option(&options, "skipUnsupported")? {
        value if absent(&value) => false,
        value => value
            .as_bool()
            .ok_or_else(|| type_error("Expected 'skipUnsupported' to be a boolean."))?,
    };
    let comments = match option(&options, "specification")? {
        value if absent(&value) => Vec::new(),
        value => schema::comments_of(&Input::String(
            value
                .as_string()
                .ok_or_else(|| type_error("Expected 'specification' to be a string."))?,
        ))
        .map_err(compiler_error)?,
    };
    let made = generate(&schema::Request {
        format,
        types: &types,
        skip_unsupported,
        asni,
        comments: &comments,
    })
    .map_err(|message| error("SchemaError", &message))?;

    let skipped = Array::new();

    for (name, why) in made.skipped {
        let entry = Object::new();

        Reflect::set(&entry, &"type".into(), &name.into())?;
        Reflect::set(&entry, &"reason".into(), &why.into())?;
        skipped.push(&entry);
    }

    let result = Object::new();

    Reflect::set(&result, &"text".into(), &made.text.into())?;
    Reflect::set(&result, &"types".into(), &made.types.into())?;
    Reflect::set(&result, &"skipped".into(), &skipped)?;

    Ok(result)
}

/// The version of the compiler, which it writes in what it makes.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}
