// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

//! `asnkitcc` tells what is likely to cause trouble in a specification
//! on standard error, and still writes what it makes, but refuses a type
//! defined by itself alone.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

fn run(folder: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asnkitcc"))
        .current_dir(folder)
        .args(args)
        .env("PATH", "")
        .output()
        .unwrap()
}

#[test]
fn a_warning_is_told_and_the_file_written() {
    let folder = tempfile::tempdir().unwrap();
    let folder = folder.path();
    fs::write(
        folder.join("list.asn"),
        "Foo DEFINITIONS ::= BEGIN\nList ::= SEQUENCE { tail List OPTIONAL }\nEND\n",
    )
    .unwrap();

    let output = run(folder, &["compile", "list.asn"]);

    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "warning: Type 'List' in module 'Foo' refers to itself, in 'list.asn' at line 2, \
         column 1: its values nest as deep as the data has them, and asnkit decodes no more \
         than 128 levels of nesting.\n"
    );
    assert!(folder.join("list.asni").exists());
}

#[test]
fn a_type_defined_by_itself_is_an_error() {
    let folder = tempfile::tempdir().unwrap();
    let folder = folder.path();
    fs::write(
        folder.join("alias.asn"),
        "Foo DEFINITIONS ::= BEGIN\nA ::= B\nB ::= [0] A\nEND\n",
    )
    .unwrap();

    let output = run(folder, &["compile", "alias.asn"]);

    assert!(!output.status.success());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "error: Type 'A' in module 'Foo' is defined by itself, through 'B', in 'alias.asn' at \
         line 2, column 1.\n"
    );
    assert!(!folder.join("alias.asni").exists());
}
