// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The plugin `asnkit-gen-rust`, as `asnkitcc generate` runs it:
//!
//! ```text
//! asnkit-gen-rust --asnc FILE [--asnst FILE] --out FOLDER [--opt KEY=VALUE ...]
//! ```
//!
//! It writes one Rust module to the folder. The options are `name`,
//! which is the name of the module without `.rs` (default: that of the
//! first file the specification is from), `no_std`, which is `true` for
//! code that needs `alloc` and not `std` (default `false`), and
//! `integer`, which is what an INTEGER with no bounds is: `i64`
//! (default), `i128` or `big`.

use std::path::PathBuf;
use std::process::ExitCode;

use asnkit_gen_rust::{Error, Integer, Options, VERSION, generate};

const USAGE: &str =
    "usage: asnkit-gen-rust --asnc FILE [--asnst FILE] --out FOLDER [--opt KEY=VALUE ...]";

struct Arguments {
    asnc: PathBuf,
    out: PathBuf,
    options: Vec<String>,
}

fn parse_arguments(arguments: &[String]) -> Result<Option<Arguments>, String> {
    let mut asnc = None;
    let mut out = None;
    let mut options = Vec::new();
    let mut iterator = arguments.iter();

    while let Some(argument) = iterator.next() {
        let mut value = |name: &str| {
            iterator
                .next()
                .cloned()
                .ok_or_else(|| format!("error: {name} needs a value.\n{USAGE}"))
        };

        match argument.as_str() {
            "--version" => {
                println!("asnkit-gen-rust {VERSION}");

                return Ok(None);
            }
            "--help" | "-h" => {
                println!(
                    "{USAGE}\n\nMakes Rust code with structs and enums from a compiled ASN.1 specification. Run by `asnkitcc generate`."
                );

                return Ok(None);
            }
            "--asnc" => asnc = Some(PathBuf::from(value("--asnc")?)),
            // The names of the files are in the .asnc file too.
            "--asnst" => {
                value("--asnst")?;
            }
            "--out" => out = Some(PathBuf::from(value("--out")?)),
            "--opt" => options.push(value("--opt")?),
            other => return Err(format!("error: unknown argument '{other}'.\n{USAGE}")),
        }
    }

    Ok(Some(Arguments {
        asnc: asnc.ok_or_else(|| format!("error: --asnc is needed.\n{USAGE}"))?,
        out: out.ok_or_else(|| format!("error: --out is needed.\n{USAGE}"))?,
        options,
    }))
}

fn run(arguments: &Arguments) -> Result<(), Error> {
    let mut options = Options::default();
    let mut name = None;

    for item in &arguments.options {
        let (key, value) = item.split_once('=').unwrap_or((item, ""));

        match key {
            "name" if !value.is_empty() => name = Some(value.to_string()),
            "no_std" if ["true", "false"].contains(&value) => options.no_std = value == "true",
            "integer" if Integer::parse(value).is_some() => {
                options.integer = Integer::parse(value).unwrap_or_default();
            }
            _ => {
                return Err(Error(format!(
                    "Expected the option {item} as name=NAME, no_std=true|false or integer=i64|i128|big."
                )));
            }
        }
    }

    let asnc = std::fs::read(&arguments.asnc)
        .map_err(|error| Error(format!("{}: {error}", arguments.asnc.display())))?;
    let (text, sources) = generate(&asnc, options)?;

    let name = match name {
        Some(name) => name,
        None => sources
            .first()
            .map(|source| {
                let stem = std::path::Path::new(source)
                    .file_stem()
                    .map_or(String::new(), |stem| stem.to_string_lossy().into_owned());

                stem.replace('-', "_")
            })
            .unwrap_or_else(|| "specification".to_string()),
    };

    let mut chars = name.chars();

    if !chars
        .next()
        .is_some_and(|first| first.is_alphabetic() || first == '_')
        || !chars.all(|c| c.is_alphanumeric() || c == '_')
    {
        return Err(Error(format!(
            "Expected a name of a module, but got '{name}'."
        )));
    }

    std::fs::create_dir_all(&arguments.out)
        .map_err(|error| Error(format!("{}: {error}", arguments.out.display())))?;

    let path = arguments.out.join(format!("{name}.rs"));

    std::fs::write(&path, text).map_err(|error| Error(format!("{}: {error}", path.display())))
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    match parse_arguments(&arguments) {
        Ok(None) => ExitCode::SUCCESS,
        Ok(Some(arguments)) => match run(&arguments) {
            Ok(()) => ExitCode::SUCCESS,
            Err(error) => {
                eprintln!("error: {error}");

                ExitCode::FAILURE
            }
        },
        Err(message) => {
            eprintln!("{message}");

            ExitCode::FAILURE
        }
    }
}
