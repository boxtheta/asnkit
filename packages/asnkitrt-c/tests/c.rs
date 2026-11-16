// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Compiles C with include/asnkitrt.h, links it with the shared library
//! that Cargo builds, and runs it: examples/question.c on
//! examples/compiled/foo.asni, and tests/api.c.
//!
//! It needs a compiler of C, `cc` or the one `CC` names, and is for the
//! systems of CI: Linux and macOS.

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

fn manifest() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}

/// The folder of the shared library. Cargo is asked where it is, as it
/// is not always next to the test: since 1.100, Cargo gives each crate
/// a folder of its own. The library is built already, for the test.
fn libraries() -> &'static Path {
    static LIBRARIES: OnceLock<PathBuf> = OnceLock::new();

    LIBRARIES.get_or_init(|| {
        let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".into());
        let mut command = Command::new(&cargo);

        command
            .args(["build", "--lib", "--message-format=json", "--manifest-path"])
            .arg(manifest().join("Cargo.toml"))
            .stderr(Stdio::inherit());

        // The profile of the test, to build nothing again.
        if !cfg!(debug_assertions) {
            command.arg("--release");
        }

        let output = command
            .output()
            .unwrap_or_else(|error| panic!("Can not run {cargo}: {error}"));

        assert!(output.status.success(), "{cargo} build failed");

        // A path is between quotes in the messages, and the file is
        // named as the library: libasnkitrt_c.so, or .dylib.
        String::from_utf8_lossy(&output.stdout)
            .split('"')
            .map(Path::new)
            .find(|path| {
                path.file_stem().is_some_and(|stem| stem == "libasnkitrt_c")
                    && path
                        .extension()
                        .is_some_and(|extension| extension == "so" || extension == "dylib")
            })
            .and_then(Path::parent)
            .expect("the shared library in what Cargo built")
            .to_path_buf()
    })
}

/// Compiles `source`, of this package, and runs it with `arguments`.
fn compile_and_run(source: &str, arguments: &[PathBuf]) -> String {
    let libraries = libraries();
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
