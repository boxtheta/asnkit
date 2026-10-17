// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! include/asnkitrt.h is written by hand: it has to declare the
//! functions of src/, all of them and no others, and give its constants
//! the values they have there.

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

/// The header, without its comments.
fn header() -> String {
    let mut header = read("include/asnkitrt.h");

    while let Some(open) = header.find("/*") {
        let close = open + header[open..].find("*/").expect("a comment that ends") + 2;

        header.replace_range(open..close, "");
    }

    header
}

/// The functions that the header declares: an identifier `asnkit_…`
/// followed by `(`, which the type of the callback is not.
fn declared() -> BTreeSet<String> {
    let header = header();
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
fn the_sizes_of_keys_agree() {
    let header = header();

    for (name, size) in [
        ("ASNKIT_KEY_ID_SIZE", asnkitrt::container::KEY_ID_SIZE),
        ("ASNKIT_KEY_SIZE", asnkitrt::container::KEY_SIZE),
    ] {
        assert!(
            header.contains(&format!("#define {name} {size}\n")),
            "{name}"
        );
    }
}
