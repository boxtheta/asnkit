// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The command `asnkitcc convert`, which decodes a value with one codec
//! and encodes it with another, with a compiled specification or one
//! that is compiled first:
//!
//! ```text
//! asnkitcc convert -t Question --from jer --to der -i q.json -o q.der foo.asni
//! asnkitcc convert -t Question -i q.ber --to jer --indent 2 foo.asn
//! ```

use asnkit_compiler::container::Protection;
use asnkit_compiler::{Input, compile_to_asni};
use asnkitrt::container::{self, Binding, Checks, Nonce, Signature};
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;

use std::io::{Read, Write};
use std::path::{Path, PathBuf};

const CODECS: &str = "ber, der, oer, per, uper, jer, xer or gser";

/// The codec of a name, as `der` or `DER`.
pub fn codec_of(name: &str) -> Result<Codec, String> {
    Ok(match name.to_ascii_lowercase().as_str() {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "uper" => Codec::Uper,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        "gser" => Codec::Gser,
        _ => return Err(format!("error: unknown codec '{name}', expected {CODECS}.")),
    })
}

/// The codec a file is in, told by its extension.
fn codec_of_path(path: &Path) -> Option<Codec> {
    let extension = path.extension()?.to_str()?.to_ascii_lowercase();

    match extension.as_str() {
        "json" => Some(Codec::Jer),
        "xml" => Some(Codec::Xer),
        other => codec_of(other).ok(),
    }
}

fn is_text(codec: Codec) -> bool {
    matches!(codec, Codec::Jer | Codec::Xer | Codec::Gser)
}

pub struct Conversion<'a> {
    /// `Type` or `Module.Type`.
    pub type_name: &'a str,
    pub from: Option<Codec>,
    pub to: Option<Codec>,
    /// Standard input if not given.
    pub input: Option<&'a Path>,
    /// Standard output if not given.
    pub output: Option<&'a Path>,
    pub input_hex: bool,
    pub output_hex: bool,
    pub indent: Option<usize>,
    pub lenient: bool,
    /// An `.asni` file, or `.asn` files to compile.
    pub specification: &'a [PathBuf],
    pub encoding: &'a str,
    pub decryption_key: Option<container::Key>,
    pub verbose: bool,
}

/// The contents of an `.asni` file, or of one compiled from the files.
fn read_asni(conversion: &Conversion<'_>) -> Result<Vec<u8>, String> {
    let files = conversion.specification;
    let compiled: Vec<&PathBuf> = files
        .iter()
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "asni")
        })
        .collect();

    match compiled[..] {
        [] => {
            let input = Input::Files {
                paths: files.to_vec(),
                encoding: conversion.encoding.to_string(),
            };

            compile_to_asni(&input, None, None, &Protection::default())
                .map_err(|error| format!("error: {}", error.message))
        }
        [path] if files.len() == 1 => {
            std::fs::read(path).map_err(|error| format!("error: {}: {error}", path.display()))
        }
        _ => {
            Err("error: give one .asni file, or the ASN.1 files to compile, not both.".to_string())
        }
    }
}

fn decode_hex(text: &[u8]) -> Result<Vec<u8>, String> {
    let digits: Vec<u8> = text
        .iter()
        .copied()
        .filter(|byte| !byte.is_ascii_whitespace())
        .collect();
    let digits = digits.strip_prefix(b"0x").unwrap_or(&digits);

    if !digits.len().is_multiple_of(2) || !digits.iter().all(u8::is_ascii_hexdigit) {
        return Err("error: expected the input as hexadecimal digits.".to_string());
    }

    Ok(digits
        .chunks(2)
        .map(|pair| {
            let digit = |byte: u8| (byte as char).to_digit(16).unwrap_or_default() as u8;

            digit(pair[0]) << 4 | digit(pair[1])
        })
        .collect())
}

pub fn convert(conversion: &Conversion<'_>) -> Result<(), String> {
    let say = |message: String| {
        if conversion.verbose {
            eprintln!("{message}");
        }
    };

    let from = conversion
        .from
        .or_else(|| conversion.input.and_then(codec_of_path))
        .ok_or_else(|| format!("error: give the codec of the input with --from: {CODECS}."))?;
    let to = conversion
        .to
        .or_else(|| conversion.output.and_then(codec_of_path))
        .ok_or_else(|| format!("error: give the codec of the output with --to: {CODECS}."))?;

    if conversion.input_hex && is_text(from) {
        return Err("error: --input-hex is for the binary codecs.".to_string());
    }

    if conversion.output_hex && is_text(to) {
        return Err("error: --output-hex is for the binary codecs.".to_string());
    }

    if conversion.indent.is_some() && !is_text(to) {
        return Err("error: --indent is for JER and XER.".to_string());
    }

    let asni = read_asni(conversion)?;
    let keys: Vec<container::Key> = conversion.decryption_key.iter().cloned().collect();
    // What the file was made with is not checked: it is converted with,
    // and not trusted to be what it says.
    let checks = Checks {
        signature: Signature::Any,
        nonce: Nonce::Any,
        decryption_keys: (!keys.is_empty()).then_some(&keys[..]),
        bound_to: Binding::Any,
        ..Checks::default()
    };
    let load = |codec: Codec| {
        let mut options = Options::new(codec);
        options.lenient = conversion.lenient;

        Specification::from_asni(&asni, &checks, options)
            .map_err(|error| format!("error: the specification: {error}"))
    };
    let decoder = load(from)?;
    let encoder = if to == from {
        decoder.clone()
    } else {
        load(to)?
    };

    let mut data = Vec::new();

    match conversion.input {
        Some(path) => {
            data = std::fs::read(path)
                .map_err(|error| format!("error: {}: {error}", path.display()))?;
        }
        None => {
            std::io::stdin()
                .read_to_end(&mut data)
                .map_err(|error| format!("error: standard input: {error}"))?;
        }
    }

    if conversion.input_hex {
        data = decode_hex(&data)?;
    } else if is_text(from) {
        // A file of text ends with a new line, which is not in the value.
        let end = data.trim_ascii_end().len();
        data.truncate(end);
    }

    let name = conversion.type_name;
    let (module, name) = match name.split_once('.') {
        Some((module, name)) => (Some(module), name),
        None => (None, name),
    };

    let value = match module {
        Some(module) => decoder.decode_in(module, name, &data),
        None => decoder.decode(name, &data),
    }
    .map_err(|error| match error {
        asnkitrt::Error::Decode(what, offset, location) => {
            let unit = if matches!(from, Codec::Per | Codec::Uper) {
                "bit"
            } else {
                "byte"
            };

            if location.is_empty() {
                format!("error: decoding: {what} (at {unit} {offset})")
            } else {
                format!("error: decoding: {location}: {what} (at {unit} {offset})")
            }
        }
        asnkitrt::Error::NoSuchType => no_such_type(conversion.type_name, module.is_some()),
        error => format!("error: decoding: {error}"),
    })?;

    say(format!(
        "Decoded {} bytes of {from:?} as {}.",
        data.len(),
        conversion.type_name
    ));

    let encoded = match (module, conversion.indent) {
        (Some(module), Some(indent)) => encoder.encode_in_indented(module, name, &value, indent),
        (None, Some(indent)) => encoder.encode_indented(name, &value, indent),
        (Some(module), None) => encoder.encode_in(module, name, &value),
        (None, None) => encoder.encode(name, &value),
    }
    .map_err(|error| format!("error: encoding: {error}"))?;

    let mut encoded = if conversion.output_hex {
        encoded
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
            .into_bytes()
    } else {
        encoded
    };

    if (conversion.output_hex || is_text(to)) && encoded.last() != Some(&b'\n') {
        encoded.push(b'\n');
    }

    match conversion.output {
        Some(path) => std::fs::write(path, &encoded)
            .map_err(|error| format!("error: {}: {error}", path.display()))?,
        None => std::io::stdout()
            .write_all(&encoded)
            .and_then(|()| std::io::stdout().flush())
            .map_err(|error| format!("error: standard output: {error}"))?,
    }

    say(format!("Encoded as {to:?} in {} bytes.", encoded.len()));

    Ok(())
}

fn no_such_type(name: &str, qualified: bool) -> String {
    if qualified {
        format!("error: no type '{name}' in the specification.")
    } else {
        format!(
            "error: no type '{name}' in the specification, or it is in more than one \
             module: give it as Module.{name}."
        )
    }
}
