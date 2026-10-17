// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Compiles C with include/asnkitrt.h, links it with the shared library
//! that Cargo built for these tests, and runs it: examples/question.c on
//! examples/compiled/foo.asni, and tests/api.c.
//!
//! It needs a compiler of C, `cc` or the one `CC` names, and is for the
//! systems of CI: Linux and macOS.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// Compiles `source`, of this package, and runs it with `arguments`.
fn compile_and_run(source: &str, arguments: &[PathBuf]) -> String {
    // Where Cargo put the libraries for the tests: `deps/`, with the
    // test.
    let test = std::env::current_exe().expect("the path of the test");
    let libraries = test.parent().expect("the folder of the test");
    let name = Path::new(source).file_stem().expect("a file");
    let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let compiler = std::env::var("CC").unwrap_or_else(|_| "cc".into());
    let compiled = Command::new(&compiler)
        .args(["-std=c99", "-Wall", "-Wextra", "-Werror", "-pedantic"])
        .arg("-I")
        .arg(manifest().join("include"))
        .arg(manifest().join(source))
        .arg("-o")
        .arg(&output)
        .arg("-L")
        .arg(libraries)
        .arg("-lasnkitrt_c")
        .arg(format!("-Wl,-rpath,{}", libraries.display()))
        .status()
        .unwrap_or_else(|error| panic!("Can not run {compiler}: {error}"));

    assert!(compiled.success(), "{compiler} failed on {source}");

    let Output {
        status,
        stdout,
        stderr,
    } = Command::new(&output)
        .args(arguments)
        .output()
        .expect("the program runs");
    let stdout = String::from_utf8_lossy(&stdout).into_owned();

    assert!(
        status.success(),
        "{source}: {stdout}{}",
        String::from_utf8_lossy(&stderr)
    );

    stdout
}

#[test]
fn the_example_round_trips_a_question() {
    let stdout = compile_and_run(
        "examples/question.c",
        &[manifest().join("../../examples/compiled/foo.asni")],
    );

    assert!(
        stdout.contains("BER: 300e0201011609497320312b313d333f\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Decoded: id 1, question \"Is 1+1=3?\"\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\"question\": \"Is 1+1=3?\""), "{stdout}");
}

#[test]
fn the_api_does_what_the_header_says() {
    let stdout = compile_and_run(
        "tests/api.c",
        &[
            manifest().join("../asnkitrt-wasm/tests/fixtures/types.asni"),
            manifest().join("../../examples/corpus"),
        ],
    );

    assert_eq!(stdout, "ok\n");
}
