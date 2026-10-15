// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! A reference name is assigned once in a module, and a module is
//! defined once (X.680 13.1). A name given again is an error, and not
//! one that takes the place of the first.

use asnkit_compiler::{Input, compile};
use std::fs;
use std::path::PathBuf;

fn error_of(text: &str) -> String {
    let error = compile(&Input::String(text.to_string())).unwrap_err();

    assert_eq!(error.kind, "ParseError");

    error.message
}

#[test]
fn a_name_assigned_twice_is_an_error() {
    let message = error_of("Foo DEFINITIONS ::= BEGIN\nA ::= INTEGER\nA ::= BOOLEAN\nEND\n");

    assert_eq!(
        message,
        "'A' is assigned twice in module 'Foo', at line 3, column 1."
    );

    let message = error_of("Foo DEFINITIONS ::= BEGIN\na INTEGER ::= 1\na INTEGER ::= 2\nEND\n");

    assert_eq!(
        message,
        "'a' is assigned twice in module 'Foo', at line 3, column 1."
    );

    // A type and an object class have names of the same kind.
    let message =
        error_of("Foo DEFINITIONS ::= BEGIN\nA ::= INTEGER\nA ::= CLASS { &id INTEGER }\nEND\n");

    assert_eq!(
        message,
        "'A' is assigned twice in module 'Foo', at line 3, column 1."
    );
}

#[test]
fn a_module_defined_twice_is_an_error() {
    let message = error_of(
        "Foo DEFINITIONS ::= BEGIN A ::= INTEGER END\n\
         Foo DEFINITIONS ::= BEGIN B ::= BOOLEAN END\n",
    );

    assert_eq!(
        message,
        "Module 'Foo' is defined twice, at line 2, column 1."
    );

    // In another file, which the message tells.
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("names");
    fs::create_dir_all(&folder).unwrap();
    let first = folder.join("first.asn");
    let second = folder.join("second.asn");
    fs::write(&first, "Foo DEFINITIONS ::= BEGIN A ::= INTEGER END\n").unwrap();
    fs::write(&second, "\nFoo DEFINITIONS ::= BEGIN B ::= BOOLEAN END\n").unwrap();

    let error = compile(&Input::Files {
        paths: vec![first, second.clone()],
        encoding: "utf-8".to_string(),
    })
    .unwrap_err();

    assert_eq!(
        error.message,
        format!(
            "Module 'Foo' is defined twice, in '{}' at line 2, column 1.",
            second.display()
        )
    );
}

#[test]
fn the_same_name_in_two_modules_is_not_an_error() {
    assert!(
        compile(&Input::String(
            "Foo DEFINITIONS ::= BEGIN A ::= INTEGER END\n\
             Bar DEFINITIONS ::= BEGIN A ::= BOOLEAN END\n"
                .to_string()
        ))
        .is_ok()
    );
}
