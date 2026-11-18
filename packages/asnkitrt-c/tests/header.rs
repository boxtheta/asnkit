// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! include/asnkitrt.h is written by hand: it has to declare the
//! functions of src/, all of them and no others, and give its constants
//! the values they have there. include/asnkitrt.hpp, which wraps it for
//! C++, has to use all of them too, so that it does not fall behind.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

fn read(path: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(path);

    std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn sources() -> String {
    ["src/lib.rs", "src/spec.rs", "src/value.rs"]
        .iter()
        .map(|path| read(path))
        .collect()
}

/// The identifier that `text` starts with.
fn identifier(text: &str) -> &str {
    let end = text
        .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .unwrap_or(text.len());

    &text[..end]
}

/// The functions that src/ exports.
fn exported() -> BTreeSet<String> {
    let sources = sources();
    let mut found = BTreeSet::new();

    for marker in ["pub extern \"C\" fn ", "pub unsafe extern \"C\" fn "] {
        for (at, _) in sources.match_indices(marker) {
            found.insert(identifier(&sources[at + marker.len()..]).to_string());
        }
    }

    found
}

/// `text` without its comments of C, and of C++ if `line_comments`.
fn without_comments(mut text: String, line_comments: bool) -> String {
    while let Some(open) = text.find("/*") {
        let close = open + text[open..].find("*/").expect("a comment that ends") + 2;

        text.replace_range(open..close, "");
    }

    if line_comments {
        while let Some(open) = text.find("//") {
            let close = open + text[open..].find('\n').unwrap_or(text.len() - open);

            text.replace_range(open..close, "");
        }
    }

    text
}

/// The header, without its comments.
fn header() -> String {
    without_comments(read("include/asnkitrt.h"), false)
}

/// The functions that `header` declares or calls: an identifier
/// `asnkit_…` followed by `(`, which the type of the callback is not.
fn functions(header: &str) -> BTreeSet<String> {
    let mut found = BTreeSet::new();

    for (at, _) in header.match_indices("asnkit_") {
        if header[..at]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }

        let name = identifier(&header[at..]);

        if header[at + name.len()..].starts_with('(') {
            found.insert(name.to_string());
        }
    }

    found
}

/// The functions that the header declares.
fn declared() -> BTreeSet<String> {
    functions(&header())
}

/// The functions of C that include/asnkitrt.hpp may leave unused, each
/// with why. None for now.
const UNUSED_BY_CPP: &[&str] = &[];

/// The constants `ASNKIT_…` that are given a value, with it.
fn constants(text: &str, separator: &str) -> BTreeMap<String, String> {
    let mut found = BTreeMap::new();

    for line in text.lines() {
        let line = line.trim();
        let Some(at) = line.find("ASNKIT_") else {
            continue;
        };
        let name = identifier(&line[at..]);
        let rest = line[at + name.len()..].trim_start();

        if let Some(rest) = rest.strip_prefix(separator) {
            let value = rest.trim().trim_end_matches([',', ';']).trim();

            found.insert(name.to_string(), value.to_string());
        }
    }

    found
}

#[test]
fn the_header_declares_every_function() {
    assert_eq!(declared(), exported());
}

#[test]
fn the_constants_agree() {
    let in_header = constants(&header(), "=");
    let in_rust = constants(&sources(), ": c_int =");

    assert!(!in_rust.is_empty());

    for (name, value) in &in_rust {
        assert_eq!(in_header.get(name), Some(value), "{name}");
    }

    assert_eq!(
        in_header.keys().collect::<Vec<_>>(),
        in_rust.keys().collect::<Vec<_>>()
    );
}

#[test]
fn the_cpp_header_uses_every_function() {
    let used = functions(&without_comments(read("include/asnkitrt.hpp"), true));
    let exported = exported();

    for name in UNUSED_BY_CPP {
        assert!(exported.contains(*name), "{name} is not a function");
        assert!(!used.contains(*name), "{name} is used, and not unused");
    }

    let missing: Vec<_> = exported
        .iter()
        .filter(|name| !used.contains(*name) && !UNUSED_BY_CPP.contains(&name.as_str()))
        .collect();

    assert!(missing.is_empty(), "asnkitrt.hpp does not use {missing:?}");
}
