// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The command `asnkitcc`, which compiles and parses ASN.1 specifications:
//!
//! ```text
//! asnkitcc compile foo.asn             # Makes foo.asni.
//! asnkitcc parse foo.asn               # Makes foo.asnst.
//! asnkitcc convert -t Question --from jer --to der foo.asni < q.json > q.der
//! asnkitcc schema --format json-schema -t Question foo.asn > question.schema.json
//! ```

mod convert;
mod schema;

use asnkit_compiler::{
    CompileOptions, Compiled, Input, compile_to_asni_with_warnings, parse_to_asnst, plugin,
};

use std::path::{Path, PathBuf};
use std::process::ExitCode;

const USAGE: &str = "usage: asnkitcc {compile,parse} [-v] [-o OUTPUT] [-e ENCODING] FILE [FILE ...]\n       asnkitcc generate --plugin NAME [--plugin-opt KEY=VALUE ...] [-v] [-o FOLDER] [-e ENCODING] FILE [FILE ...]
       asnkitcc convert -t [MODULE.]TYPE [--from CODEC] [--to CODEC] [-i INPUT] [-o OUTPUT] [--input-hex] [--output-hex]
       [--indent N] [--lenient] [-v] [-e ENCODING] {FILE.asni | FILE.asn [FILE.asn ...]}
       asnkitcc schema [--format FORMAT] [-t [MODULE.]TYPE ...] [--skip-unsupported] [-o OUTPUT] [-v] [-e ENCODING]
       {FILE.asni | FILE.asn [FILE.asn ...]}
       asnkitcc {-h,--help,help}
       asnkitcc {-V,--version,version}

--any-defined-by FILE adds TOML choice mappings to compile, generate or schema; it can be repeated.
The codecs are ber, der, oer, per, uper, jer, xer and gser. Without --from or
--to, the codec is told by the extension of the input or the output (.json is jer, .xml is xer). The
input is read from standard input and the output written to standard output when not given.
schema writes a JSON Schema of JER (--format json-schema), or an XML Schema (--format xsd) or a
DTD (--format dtd) of XER, of the types or of those given with -t, told by the extension of the
output (.json, .xsd or .dtd) without --format. What a schema can not tell, as an ANY, is an error, or left out with --skip-unsupported.
--version tells the versions of asnkitcc and of the plugins found where programs are.";

/// The options of what `.asni` files no longer have: a nonce, a
/// signature and encryption. They are told apart from options that never
/// were, so that whoever still gives them knows the file is not signed.
const REMOVED_OPTIONS: [&str; 5] = [
    "--nonce",
    "--signing-key-file",
    "--signing-key-id",
    "--encryption-key-file",
    "--encryption-key-id",
];

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
    plugin: Option<String>,
    plugin_options: Vec<String>,
    type_name: Option<String>,
    /// Every `-t`, of which `schema` takes several.
    types: Vec<String>,
    format: Option<schema::Format>,
    skip_unsupported: bool,
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

    if !["compile", "parse", "generate", "convert", "schema"].contains(&command.as_str()) {
        return Err(format!("error: unknown command '{command}'.\n{USAGE}"));
    }

    let mut parsed = Arguments {
        command,
        output: None,
        verbose: false,
        encoding: "utf-8".to_string(),
        plugin: None,
        plugin_options: Vec::new(),
        type_name: None,
        types: Vec::new(),
        format: None,
        skip_unsupported: false,
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
            "-t" | "--type" => {
                let name = value("--type")?;

                parsed.types.push(name.clone());
                parsed.type_name = Some(name);
            }
            "--format" => parsed.format = Some(schema::format_of(&value("--format")?)?),
            "--skip-unsupported" => parsed.skip_unsupported = true,
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
            "--bound-to" => {
                return Err(
                    "error: --bound-to is no longer supported: .asni files are not bound to \
                     anything. They have a CRC-32, which tells a corrupt file."
                        .to_string(),
                );
            }
            removed if REMOVED_OPTIONS.contains(&removed) => {
                return Err(format!(
                    "error: {removed} is no longer supported: .asni files have no nonce and are \
                     not signed or encrypted."
                ));
            }
            other if other.starts_with('-') => {
                return Err(format!("error: unknown option '{other}'.\n{USAGE}"));
            }
            file => parsed.files.push(PathBuf::from(file)),
        }
    }

    if !parsed.any_defined_by.is_empty()
        && !matches!(parsed.command.as_str(), "compile" | "generate" | "schema")
    {
        return Err("error: --any-defined-by is for compile, generate and schema.".into());
    }

    let converting = parsed.from.is_some()
        || parsed.to.is_some()
        || parsed.input.is_some()
        || parsed.input_hex
        || parsed.output_hex
        || parsed.indent.is_some()
        || parsed.lenient;

    if (parsed.format.is_some() || parsed.skip_unsupported) && parsed.command != "schema" {
        return Err(format!(
            "error: --format and --skip-unsupported are for schema.\n{USAGE}"
        ));
    }

    if parsed.command == "convert" {
        if parsed.type_name.is_none() {
            return Err(format!("error: convert needs --type.\n{USAGE}"));
        }

        if parsed.types.len() > 1 {
            return Err(format!("error: convert takes one --type.\n{USAGE}"));
        }

        if parsed.plugin.is_some() || !parsed.plugin_options.is_empty() {
            return Err(format!("error: convert takes no plugin.\n{USAGE}"));
        }
    } else if converting {
        return Err(format!(
            "error: --from, --to, --input, --input-hex, --output-hex, --indent and --lenient \
             are for convert.\n{USAGE}"
        ));
    } else if parsed.command == "schema" {
        if parsed.plugin.is_some() || !parsed.plugin_options.is_empty() {
            return Err(format!("error: schema takes no plugin.\n{USAGE}"));
        }
    } else if parsed.type_name.is_some() {
        return Err(format!("error: --type is for convert and schema.\n{USAGE}"));
    } else if parsed.command == "generate" {
        if parsed.plugin.is_none() {
            return Err(format!("error: generate needs --plugin.\n{USAGE}"));
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

/// Compiles the files, and tells what in them is likely to cause
/// trouble, on standard error, as warnings.
fn compile(input: &Input, arguments: &Arguments) -> Result<Compiled, String> {
    let compiled = compile_to_asni_with_warnings(input, &compile_options(arguments)?)
        .map_err(|error| format!("error: {}", error.message))?;

    for warning in &compiled.warnings {
        eprintln!("warning: {}", warning.message);
    }

    Ok(compiled)
}

fn generate(arguments: &Arguments) -> Result<(), String> {
    let input = Input::Files {
        paths: arguments.files.clone(),
        encoding: arguments.encoding.clone(),
    };
    let Compiled {
        asni,
        modules: summary,
        ..
    } = compile(&input, arguments)?;
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

fn schema(arguments: &Arguments) -> Result<(), String> {
    // Compiled here with --any-defined-by, which gives the types of the
    // values of an ANY DEFINED BY.
    let asni = if arguments.any_defined_by.is_empty() {
        convert::read_asni(&arguments.files, &arguments.encoding)?
    } else if arguments.files.iter().any(|path| {
        path.extension()
            .is_some_and(|extension| extension == "asni")
    }) {
        return Err(
            "error: --any-defined-by is for the ASN.1 files that are compiled.".to_string(),
        );
    } else {
        let input = Input::Files {
            paths: arguments.files.clone(),
            encoding: arguments.encoding.clone(),
        };

        compile(&input, arguments)?.asni
    };

    schema::schema(&schema::Request {
        format: arguments.format,
        types: &arguments.types,
        skip_unsupported: arguments.skip_unsupported,
        output: arguments.output.as_deref(),
        asni: &asni,
        verbose: arguments.verbose,
    })
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
            "schema" => return schema(&arguments),
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
        let started = std::time::Instant::now();

        for file in &arguments.files {
            say(format!(
                "Reading {} as {}.",
                file.display(),
                arguments.encoding
            ));
        }

        let data = if arguments.command == "compile" {
            let Compiled {
                asni: data,
                modules: summary,
                ..
            } = compile(&input, &arguments)?;

            for module in &summary {
                say(format!(
                    "Module {}: {} types, {} object classes.",
                    module.name, module.types, module.object_classes
                ));
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
