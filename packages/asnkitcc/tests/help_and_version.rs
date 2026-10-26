// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

//! `asnkitcc` tells its usage and its version on standard output, and
//! exits with success.

use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asnkitcc"))
        .args(args)
        .env("PATH", "")
        .output()
        .unwrap()
}

#[test]
fn help() {
    for args in [
        &["-h"][..],
        &["--help"],
        &["help"],
        &["compile", "--help"],
        &["convert", "-h"],
        &["generate", "--plugin", "rust", "--help"],
    ] {
        let output = run(args);

        assert!(output.status.success(), "{args:?}");
        assert!(output.stderr.is_empty(), "{args:?}");
        assert!(
            String::from_utf8(output.stdout)
                .unwrap()
                .starts_with("usage: asnkitcc"),
            "{args:?}"
        );
    }
}

#[test]
fn version() {
    for args in [
        &["-V"][..],
        &["--version"],
        &["version"],
        &["convert", "--version"],
    ] {
        let output = run(args);

        assert!(output.status.success(), "{args:?}");
        assert_eq!(
            String::from_utf8(output.stdout).unwrap(),
            format!("asnkitcc {}\n", env!("CARGO_PKG_VERSION")),
            "{args:?}"
        );
    }
}

#[test]
fn unknown_command() {
    let output = run(&["--helpme"]);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
}
