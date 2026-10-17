// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Parse and compile ASN.1 specifications to the files `asnkit` loads,
//! for an application to do it itself; the command `asnkitcc` is one
//! that does (`packages/asnkitcc`).
//!
//! ```
//! use asnkit_compiler::container::Protection;
//! use asnkit_compiler::{Input, compile_to_asni, parse_to_asnst};
//!
//! let input = Input::String(
//!     "Foo DEFINITIONS ::= BEGIN Number ::= INTEGER (0..9) END".to_string(),
//! );
//!
//! // What a runtime loads, and the syntax tree for plugins.
//! let asni = compile_to_asni(&input, None, None, &Protection::default())?;
//! let asnst = parse_to_asnst(&input)?;
//!
//! assert!(asni.starts_with(b"ASNI"));
//! assert!(!asnst.is_empty());
//! # Ok::<(), asnkit_compiler::Error>(())
//! ```
//!
//! `parse()` and `compile()` give the syntax tree in its readable form
//! and the nodes of a compiled specification in their compact form, as
//! JSON values, which is what the golden tests in `tests/golden/` hold
//! the compiler to. `plugin` runs a plugin on a specification (but not
//! in WebAssembly, which runs no programs), and `container` signs and
//! encrypts the files it writes.

// The grammar follows that of the Python parser closely, to be compared with
// it. These are the lints about its style.
#![allow(
    clippy::collapsible_if,
    clippy::large_enum_variant,
    clippy::redundant_guards,
    clippy::redundant_pattern_matching
)]

pub mod ast;
pub mod compact;
mod compile;
pub mod container;
mod data;
mod grammar;
mod objects;
// A plugin is a program that is run, which WebAssembly has no way to.
#[cfg(not(target_arch = "wasm32"))]
pub mod plugin;
mod source;

use serde_json::Value;
use std::fmt;
use std::path::PathBuf;
use std::rc::Rc;
use winnow::error::ErrMode;

/// What to parse or compile.
#[derive(Debug, Clone)]
pub enum Input {
    /// The text of one specification.
    String(String),
    /// Files to read, in the order they are given, in the text encoding
    /// `encoding`.
    Files {
        paths: Vec<PathBuf>,
        encoding: String,
    },
}

/// What went wrong, as the kind of the error and its message. The kind is
/// `ParseError`, `CompileError` or `PluginError`. A message of a parse or
/// compile error tells where in the specification, with the file, the
/// line and the column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error {
    pub kind: String,
    pub message: String,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.kind, self.message)
    }
}

impl std::error::Error for Error {}

fn parse_error(message: String) -> Error {
    Error {
        kind: "ParseError".to_string(),
        message,
    }
}

/// The contents of the sources, and their names if they are files.
fn read_sources(input: &Input) -> Result<(Vec<String>, Option<Vec<String>>), Error> {
    match input {
        Input::String(text) => Ok((vec![text.clone()], None)),
        Input::Files { paths, encoding } => {
            let mut sources = Vec::new();
            let mut names = Vec::new();

            for path in paths {
                let bytes = std::fs::read(path).map_err(|e| Error {
                    kind: "OSError".to_string(),
                    message: format!("{}: {e}", path.display()),
                })?;

                // Only UTF-8 is read for now. Characters that are not are
                // replaced, as the Python compiler does.
                let _ = encoding;
                let text = String::from_utf8_lossy(&bytes);

                // A text file is read with every kind of line break as
                // one.
                sources.push(text.replace("\r\n", "\n").replace('\r', "\n"));
                names.push(path.display().to_string());
            }

            Ok((sources, Some(names)))
        }
    }
}

/// The metadata of the files made from an input: where the specifications
/// come from, and what made the file.
fn metadata_of(input: &Input, include_text: bool) -> Result<container::Metadata, Error> {
    let (sources, names) = read_sources(input)?;
    let sources = sources
        .into_iter()
        .enumerate()
        .map(|(index, text)| container::Source {
            name: names.as_ref().map(|names| {
                std::path::Path::new(&names[index])
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default()
            }),
            text: Some(text),
            include_text,
        })
        .collect();

    Ok(container::Metadata {
        generator: Some(format!(
            "asnkit-compiler {} (Rust)",
            env!("CARGO_PKG_VERSION")
        )),
        sources,
        bound_to: None,
    })
}

fn container_error(message: String) -> Error {
    Error {
        kind: "Error".to_string(),
        message,
    }
}

/// What a compiled module has, for those who want to tell of it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleSummary {
    pub name: String,
    pub types: usize,
    pub object_classes: usize,
}

/// The modules of the nodes of a compiled specification.
fn summarize(nodes: &Value) -> Vec<ModuleSummary> {
    // [1, [0, [2, name, [0, types...], [0, object classes...]], ...]]
    let count = |list: Option<&Value>| {
        list.and_then(Value::as_array)
            .map_or(0, |items| items.len().saturating_sub(1))
    };

    nodes
        .get(1)
        .and_then(Value::as_array)
        .map(|modules| {
            modules
                .iter()
                .skip(1)
                .map(|module| ModuleSummary {
                    name: module
                        .get(1)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string(),
                    types: count(module.get(2)),
                    object_classes: count(module.get(3)),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The contents of the `.asni` file of an input.
pub fn compile_to_asni(
    input: &Input,
    nonce: Option<u32>,
    bound_to: Option<String>,
    protection: &container::Protection,
) -> Result<Vec<u8>, Error> {
    compile_to_asni_with_summary(input, nonce, bound_to, protection).map(|(data, _)| data)
}

/// Same as `compile_to_asni()`, and tells what the modules have.
pub fn compile_to_asni_with_summary(
    input: &Input,
    nonce: Option<u32>,
    bound_to: Option<String>,
    protection: &container::Protection,
) -> Result<(Vec<u8>, Vec<ModuleSummary>), Error> {
    let nodes = compile(input)?;
    let summary = summarize(&nodes);
    let mut metadata = metadata_of(input, false)?;
    metadata.bound_to = bound_to;

    let data =
        container::write_asni(&metadata, &nodes, nonce, protection).map_err(container_error)?;

    Ok((data, summary))
}

/// The contents of the `.asnst` file of an input, which has the text of
/// the specifications.
pub fn parse_to_asnst(input: &Input) -> Result<Vec<u8>, Error> {
    let tree = parse(input)?;
    let metadata = metadata_of(input, true)?;

    container::write_asnst(&metadata, &tree).map_err(container_error)
}

/// Parses an input to its syntax tree, in the readable form of a node
/// (see `docs/file_formats.md`).
pub fn parse(input: &Input) -> Result<Value, Error> {
    Ok(parse_tree(input)?.to_tree())
}

fn parse_tree(input: &Input) -> Result<ast::SyntaxTree, Error> {
    let (sources, names) = read_sources(input)?;
    let string = sources.join("\n");
    let sizes: Vec<usize> = sources.iter().map(|s| s.len() + 1).collect();
    let locator = Rc::new(source::Locator::new(&string, &sizes));

    let where_ = |source: usize| match &names {
        Some(names) => format!("in '{}' ", names[source]),
        None => String::new(),
    };

    let stripped = match source::ignore_comments(&string) {
        Ok(stripped) => stripped,
        Err(error) => {
            let (source, line, column, marked) = locator.locate_error(error.offset);

            return Err(parse_error(format!(
                "Invalid ASN.1 syntax {}at line {line}, column {column}: '{marked}': {}.",
                where_(source),
                error.message
            )));
        }
    };

    let mut grammar_input = grammar::make_input(&stripped, locator.clone());
    grammar::reset_failures();

    match grammar::parse_all(&mut grammar_input) {
        Ok(tree) => Ok(tree),
        Err(_) if grammar_input.state.conversion_error.is_some() => {
            let message = grammar_input.state.conversion_error.take().unwrap();

            match grammar_input.state.conversion_offset {
                Some(offset) => {
                    let (source, line, column, _) = locator.locate_error(offset);

                    Err(parse_error(format!(
                        "{message}, {}at line {line}, column {column}.",
                        where_(source)
                    )))
                }
                None => Err(parse_error(message)),
            }
        }
        Err(ErrMode::Backtrack(_) | ErrMode::Cut(_) | ErrMode::Incomplete(_)) => {
            let state = &grammar_input.state;
            let (source, line, column, marked) =
                locator.locate_error(grammar::error_offset(state.farthest));
            let expected = if state.expected.is_empty() {
                "Expected end of text".to_string()
            } else {
                format!("Expected {}", state.expected.join(" or "))
            };

            Err(parse_error(format!(
                "Invalid ASN.1 syntax {}at line {line}, column {column}: '{marked}': {expected}.",
                where_(source)
            )))
        }
    }
}

/// Compiles an input to the nodes of a compiled specification, in the
/// compact form of a node.
pub fn compile(input: &Input) -> Result<Value, Error> {
    let tree = parse_tree(input)?;
    let mut compiler = compile::Compiler {
        modules: tree.modules,
    };

    compiler.resolve().map_err(|message| Error {
        kind: "CompileError".to_string(),
        message,
    })?;

    Ok(compact::to_compact(&compile::compiled_specification(
        compiler.modules,
    )))
}
