// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The command `asnkitcc`, which compiles and parses ASN.1 specifications:
//!
//! ```text
//! asnkitcc compile foo.asn             # Makes foo.asni.
//! asnkitcc parse foo.asn               # Makes foo.asnst.
//! asnkitcc convert -t Question --from jer --to der foo.asni < q.json > q.der
//! ```

mod convert;

use asnkit_compiler::container::{KEY_ID_SIZE, KEY_SIZE, Key, Protection, derive_key_id};
use asnkit_compiler::{
    CompileOptions, Input, compile_to_asni_with_summary_and_options, parse_to_asnst, plugin,
};

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: asnkitcc {compile,parse} [-v] [-o OUTPUT] [-e ENCODING] [--nonce N] [--bound-to STRING]\n       [--signing-key-file FILE [--signing-key-id ID]] [--encryption-key-file FILE [--encryption-key-id ID]] FILE [FILE ...]\n       asnkitcc generate --plugin NAME [--plugin-opt KEY=VALUE ...] [-v] [-o FOLDER] [-e ENCODING] FILE [FILE ...]
       asnkitcc convert -t [MODULE.]TYPE [--from CODEC] [--to CODEC] [-i INPUT] [-o OUTPUT] [--input-hex] [--output-hex]
       [--indent N] [--lenient] [--encryption-key-file FILE] [-v] [-e ENCODING] {FILE.asni | FILE.asn [FILE.asn ...]}
       asnkitcc {-h,--help,help}
       asnkitcc {-V,--version,version}

--any-defined-by FILE adds TOML choice mappings to compile or generate; it can be repeated.
The codecs are ber, der, oer, per, uper, jer, xer and gser. Without --from or
--to, the codec is told by the extension of the input or the output (.json is jer, .xml is xer). The
input is read from standard input and the output written to standard output when not given.
--version tells the versions of asnkitcc and of the plugins found where programs are.";

/// What the command line asks for.
enum Request {
    Run(Box<Arguments>),
    Help,
    Version,
}

struct Arguments {
    command: String,
    output: Option<PathBuf>,
    verbose: bool,
    encoding: String,
    nonce: Option<u32>,
    bound_to: Option<String>,
    signing_key_id: Option<String>,
    signing_key_file: Option<PathBuf>,
    encryption_key_id: Option<String>,
    encryption_key_file: Option<PathBuf>,
    plugin: Option<String>,
    plugin_options: Vec<String>,
    type_name: Option<String>,
    from: Option<String>,
    to: Option<String>,
    input: Option<PathBuf>,
    input_hex: bool,
    output_hex: bool,
    indent: Option<usize>,
    lenient: bool,
    files: Vec<PathBuf>,
    any_defined_by: Vec<PathBuf>,
}

fn parse_arguments(arguments: &[String]) -> Result<Request, String> {
    let mut iterator = arguments.iter();
    let command = iterator.next().ok_or(USAGE)?.clone();

    match command.as_str() {
        "-h" | "--help" | "help" => return Ok(Request::Help),
        "-V" | "--version" | "version" => return Ok(Request::Version),
        _ => {}
    }

    if !["compile", "parse", "generate", "convert"].contains(&command.as_str()) {
        return Err(format!("error: unknown command '{command}'.\n{USAGE}"));
    }

    let mut parsed = Arguments {
        command,
        output: None,
        verbose: false,
        encoding: "utf-8".to_string(),
        nonce: None,
        bound_to: None,
        signing_key_id: None,
        signing_key_file: None,
        encryption_key_id: None,
        encryption_key_file: None,
        plugin: None,
        plugin_options: Vec::new(),
        type_name: None,
        from: None,
        to: None,
        input: None,
        input_hex: false,
        output_hex: false,
        indent: None,
        lenient: false,
        files: Vec::new(),
        any_defined_by: Vec::new(),
    };

    while let Some(argument) = iterator.next() {
        let mut value = |what: &str| {
            iterator
                .next()
                .cloned()
                .ok_or_else(|| format!("error: {what} needs a value."))
        };

        match argument.as_str() {
            "-h" | "--help" => return Ok(Request::Help),
            "-V" | "--version" => return Ok(Request::Version),
            "-v" | "--verbose" => parsed.verbose = true,
            "--any-defined-by" => parsed
                .any_defined_by
                .push(PathBuf::from(value("--any-defined-by")?)),
            "--plugin" => parsed.plugin = Some(value("--plugin")?),
            "--plugin-opt" => parsed.plugin_options.push(value("--plugin-opt")?),
            "-o" | "--output" => parsed.output = Some(PathBuf::from(value("-o")?)),
            "-e" | "--encoding" => parsed.encoding = value("-e")?,
            "--nonce" => {
                let text = value("--nonce")?;
                let number = match text.strip_prefix("0x") {
                    Some(hex) => u32::from_str_radix(hex, 16),
                    None => text.parse(),
                };

                parsed.nonce = Some(number.map_err(|_| format!("error: bad nonce '{text}'."))?);
            }
            "-t" | "--type" => parsed.type_name = Some(value("--type")?),
            "--from" => parsed.from = Some(value("--from")?),
            "--to" => parsed.to = Some(value("--to")?),
            "-i" | "--input" => parsed.input = Some(PathBuf::from(value("-i")?)),
            "--input-hex" => parsed.input_hex = true,
            "--output-hex" => parsed.output_hex = true,
            "--lenient" => parsed.lenient = true,
            "--indent" => {
                let text = value("--indent")?;

                parsed.indent = Some(
                    text.parse()
                        .map_err(|_| format!("error: bad indent '{text}'."))?,
                );
            }
            "--bound-to" => parsed.bound_to = Some(value("--bound-to")?),
            "--signing-key-id" => parsed.signing_key_id = Some(value("--signing-key-id")?),
            "--signing-key-file" => {
                parsed.signing_key_file = Some(PathBuf::from(value("--signing-key-file")?));
            }
            "--encryption-key-id" => parsed.encryption_key_id = Some(value("--encryption-key-id")?),
            "--encryption-key-file" => {
                parsed.encryption_key_file = Some(PathBuf::from(value("--encryption-key-file")?));
            }
            other if other.starts_with('-') => {
                return Err(format!("error: unknown option '{other}'.\n{USAGE}"));
            }
            file => parsed.files.push(PathBuf::from(file)),
        }
    }

    if !parsed.any_defined_by.is_empty()
        && !matches!(parsed.command.as_str(), "compile" | "generate")
    {
        return Err("error: --any-defined-by is for compile and generate.".into());
    }

    let converting = parsed.type_name.is_some()
        || parsed.from.is_some()
        || parsed.to.is_some()
        || parsed.input.is_some()
        || parsed.input_hex
        || parsed.output_hex
        || parsed.indent.is_some()
        || parsed.lenient;

    if parsed.command == "convert" {
        if parsed.type_name.is_none() {
            return Err(format!("error: convert needs --type.\n{USAGE}"));
        }

        // The key of an encrypted file is the one to decrypt it with.
        let protected = parsed.nonce.is_some()
            || parsed.bound_to.is_some()
            || parsed.signing_key_file.is_some()
            || parsed.signing_key_id.is_some();

        if protected || parsed.plugin.is_some() || !parsed.plugin_options.is_empty() {
            return Err(format!(
                "error: convert takes no nonce, binding, signing key or plugin.\n{USAGE}"
            ));
        }
    } else if converting {
        return Err(format!(
            "error: --type, --from, --to, --input, --input-hex, --output-hex, --indent and \
             --lenient are for convert.\n{USAGE}"
        ));
    } else if parsed.command == "generate" {
        if parsed.plugin.is_none() {
            return Err(format!("error: generate needs --plugin.\n{USAGE}"));
        }

        // What is made is embedded as it is, and nothing is there to
        // tell the keys or the number it needs to be loaded.
        let protected = parsed.nonce.is_some()
            || parsed.bound_to.is_some()
            || parsed.signing_key_file.is_some()
            || parsed.signing_key_id.is_some()
            || parsed.encryption_key_file.is_some()
            || parsed.encryption_key_id.is_some();

        if protected {
            return Err(
                "error: generate does not sign, encrypt, bind or give a nonce: \
                        what the plugin is given is embedded as it is."
                    .to_string(),
            );
        }
    } else if parsed.plugin.is_some() || !parsed.plugin_options.is_empty() {
        return Err(format!(
            "error: --plugin and --plugin-opt are for generate.\n{USAGE}"
        ));
    }

    if parsed.files.is_empty() {
        return Err(format!("error: no files given.\n{USAGE}"));
    }

    Ok(Request::Run(Box::new(parsed)))
}

/// The version of asnkitcc, then that of each plugin found where programs
/// are, as the plugin tells it, by the name it is run with.
fn version() -> String {
    let mut text = format!("asnkitcc {}", env!("CARGO_PKG_VERSION"));
    let mut plugins = std::collections::BTreeMap::new();

    for folder in std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default()) {
        let Ok(entries) = std::fs::read_dir(&folder) else {
            continue;
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_stem().and_then(|name| name.to_str()) else {
                continue;
            };

            // The first found is the one a name runs.
            if name.starts_with("asnkit-gen-") && path.is_file() {
                plugins.entry(name.to_string()).or_insert(path);
            }
        }
    }

    for (name, path) in plugins {
        let told = std::process::Command::new(&path)
            .arg("--version")
            .stdin(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout).into_owned();

                stdout.split_whitespace().last().map(str::to_string)
            });

        match told {
            Some(version) => text.push_str(&format!("\n{name} {version}")),
            None => text.push_str(&format!("\n{name} (no version, at {})", path.display())),
        }
    }

    text
}

fn decode_hex(text: &str) -> Option<Vec<u8>> {
    if !text.bytes().all(|byte| byte.is_ascii_hexdigit()) || !text.len().is_multiple_of(2) {
        return None;
    }

    (0..text.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&text[index..index + 2], 16).ok())
        .collect()
}

/// The key in a file, as bytes or as hexadecimal digits, with the given
/// identifier or the one derived from the key.
fn read_key(id: Option<&str>, path: Option<&Path>, what: &str) -> Result<Option<Key>, String> {
    let Some(path) = path else {
        if id.is_some() {
            return Err(format!(
                "error: Expected --{what}-key-file with --{what}-key-id."
            ));
        }

        return Ok(None);
    };

    let id = match id {
        Some(id) => Some(
            decode_hex(id)
                .and_then(|id| <[u8; KEY_ID_SIZE]>::try_from(id).ok())
                .ok_or_else(|| {
                    format!(
                        "error: Expected the identifier of the {what} key as {} hexadecimal digits.",
                        2 * KEY_ID_SIZE
                    )
                })?,
        ),
        None => None,
    };

    let mut bytes =
        std::fs::read(path).map_err(|error| format!("error: {}: {error}", path.display()))?;

    if bytes.len() != KEY_SIZE {
        bytes = std::str::from_utf8(&bytes)
            .ok()
            .and_then(|text| decode_hex(text.trim()))
            .unwrap_or_default();
    }

    let key = <[u8; KEY_SIZE]>::try_from(bytes).map_err(|_| {
        format!(
            "error: Expected the {what} key in '{}' as {KEY_SIZE} bytes or {} hexadecimal digits.",
            path.display(),
            2 * KEY_SIZE
        )
    })?;

    Ok(Some(Key {
        id: id.unwrap_or_else(|| derive_key_id(&key, what == "signing")),
        key,
    }))
}

fn compile_options(arguments: &Arguments) -> Result<CompileOptions, String> {
    let mut options = CompileOptions::default();
    for path in &arguments.any_defined_by {
        let text = std::fs::read_to_string(path)
            .map_err(|error| format!("error: {}: {error}", path.display()))?;
        let file = CompileOptions::from_toml(&text)
            .map_err(|error| format!("error: {}: {}", path.display(), error.message))?;
        options.any_defined_by.extend(file.any_defined_by);
    }
    Ok(options)
}

fn generate(arguments: &Arguments) -> Result<(), String> {
    let input = Input::Files {
        paths: arguments.files.clone(),
        encoding: arguments.encoding.clone(),
    };
    let (asni, summary) = compile_to_asni_with_summary_and_options(
        &input,
        None,
        None,
        &Protection::default(),
        &compile_options(arguments)?,
    )
    .map_err(|error| format!("error: {}", error.message))?;
    let asnst = parse_to_asnst(&input).map_err(|error| format!("error: {}", error.message))?;

    if arguments.verbose {
        for module in &summary {
            eprintln!(
                "Module {}: {} types, {} object classes.",
                module.name, module.types, module.object_classes
            );
        }
    }

    let out = arguments
        .output
        .clone()
        .unwrap_or_else(|| PathBuf::from("."));

    let name = arguments.plugin.as_deref().unwrap_or_default();

    if arguments.verbose {
        eprintln!("Running {}.", plugin::program(name).display());
    }

    plugin::run(name, &arguments.plugin_options, &out, &asni, &asnst)
        .map_err(|error| format!("error: {}", error.message))
}

fn convert(arguments: &Arguments) -> Result<(), String> {
    let codec = |name: &Option<String>| name.as_deref().map(convert::codec_of).transpose();

    convert::convert(&convert::Conversion {
        type_name: arguments.type_name.as_deref().unwrap_or_default(),
        from: codec(&arguments.from)?,
        to: codec(&arguments.to)?,
        input: arguments.input.as_deref(),
        output: arguments.output.as_deref(),
        input_hex: arguments.input_hex,
        output_hex: arguments.output_hex,
        indent: arguments.indent,
        lenient: arguments.lenient,
        specification: &arguments.files,
        encoding: &arguments.encoding,
        decryption_key: read_key(
            arguments.encryption_key_id.as_deref(),
            arguments.encryption_key_file.as_deref(),
            "encryption",
        )?
        .map(|key| asnkitrt::container::Key {
            id: key.id,
            key: key.key,
        }),
        verbose: arguments.verbose,
    })
}

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();

    let result = parse_arguments(&arguments).and_then(|request| {
        let arguments = match request {
            Request::Run(arguments) => arguments,
            Request::Help => {
                println!("{USAGE}");

                return Ok(());
            }
            Request::Version => {
                println!("{}", version());

                return Ok(());
            }
        };

        match arguments.command.as_str() {
            "generate" => return generate(&arguments),
            "convert" => return convert(&arguments),
            _ => {}
        }

        let extension = if arguments.command == "compile" {
            "asni"
        } else {
            "asnst"
        };
        let output = arguments.output.clone().unwrap_or_else(|| {
            Path::new(arguments.files[0].file_name().unwrap_or_default()).with_extension(extension)
        });
        let input = Input::Files {
            paths: arguments.files.clone(),
            encoding: arguments.encoding.clone(),
        };

        let verbose = arguments.verbose;
        let say = |message: String| {
            if verbose {
                eprintln!("{message}");
            }
        };
        let hex = |bytes: &[u8]| {
            bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<String>()
        };
        let started = std::time::Instant::now();

        for file in &arguments.files {
            say(format!(
                "Reading {} as {}.",
                file.display(),
                arguments.encoding
            ));
        }

        let data = if arguments.command == "compile" {
            let protection = Protection {
                signing_key: read_key(
                    arguments.signing_key_id.as_deref(),
                    arguments.signing_key_file.as_deref(),
                    "signing",
                )?,
                encryption_key: read_key(
                    arguments.encryption_key_id.as_deref(),
                    arguments.encryption_key_file.as_deref(),
                    "encryption",
                )?,
                iv: None,
            };

            let (data, summary) = compile_to_asni_with_summary_and_options(
                &input,
                arguments.nonce,
                arguments.bound_to.clone(),
                &protection,
                &compile_options(&arguments)?,
            )
            .map_err(|error| format!("error: {}", error.message))?;

            for module in &summary {
                say(format!(
                    "Module {}: {} types, {} object classes.",
                    module.name, module.types, module.object_classes
                ));
            }

            if let Some(key) = &protection.encryption_key {
                say(format!("Encrypted with the key {}.", hex(&key.id)));
            }

            if let Some(key) = &protection.signing_key {
                say(format!("Signed with the key {}.", hex(&key.id)));
            }

            if let Some(nonce) = arguments.nonce {
                say(format!("Nonce {nonce}."));
            }

            if let Some(bound_to) = &arguments.bound_to {
                say(format!("Bound to '{bound_to}'."));
            }

            data
        } else {
            parse_to_asnst(&input).map_err(|error| format!("error: {}", error.message))?
        };

        let size = data.len();

        std::fs::write(&output, data)
            .map_err(|error| format!("error: {}: {error}", output.display()))?;

        say(format!(
            "Wrote {size} bytes to {} in {} ms.",
            output.display(),
            started.elapsed().as_millis()
        ));

        Ok(())
    });

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");

            ExitCode::FAILURE
        }
    }
}
