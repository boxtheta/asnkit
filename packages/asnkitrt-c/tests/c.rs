// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Compiles C with include/asnkitrt.h and C++ with include/asnkitrt.hpp,
//! links them with the shared library that Cargo builds, and runs them:
//! examples/question.c and examples/question.cpp on
//! examples/compiled/foo.asni, tests/api.c, and tests/api.cpp with
//! tests/api_odr.cpp. The C++ is C++17, and is also compiled (with no
//! linking) as C++20 and C++23.
//!
//! It needs a compiler of C, `cc` or the one `CC` names, and one of C++,
//! `c++` or the one `CXX` names, and is for the systems of CI: Linux and
//! macOS.

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

/// What a source is written in: the compiler, which a variable may name,
/// and the standard.
struct Language {
    variable: &'static str,
    compiler: &'static str,
    standard: &'static str,
}

const C: Language = Language {
    variable: "CC",
    compiler: "cc",
    standard: "c99",
};

const CXX: Language = Language {
    variable: "CXX",
    compiler: "c++",
    standard: "c++17",
};

const WARNINGS: [&str; 4] = ["-Wall", "-Wextra", "-Werror", "-pedantic"];

impl Language {
    /// The compiler, with the warnings that fail it and the headers, for
    /// `standard`.
    fn command(&self, standard: &str) -> (String, Command) {
        let compiler = std::env::var(self.variable).unwrap_or_else(|_| self.compiler.into());
        let mut command = Command::new(&compiler);

        command
            .arg(format!("-std={standard}"))
            .args(WARNINGS)
            .arg("-I")
            .arg(manifest().join("include"));

        (compiler, command)
    }
}

/// Compiles `sources`, of this package and in `language`, into one
/// program, and runs it with `arguments`.
fn compile_and_run(language: &Language, sources: &[&str], arguments: &[PathBuf]) -> String {
    let libraries = libraries();
    // Named for the first source with its extension, as question.c and
    // question.cpp are compiled at once.
    let name = sources[0]
        .rsplit('/')
        .next()
        .expect("a file")
        .replace('.', "_");
    let output = Path::new(env!("CARGO_TARGET_TMPDIR")).join(name);
    let source = sources.join(" ");
    let (compiler, mut command) = language.command(language.standard);
    let compiled = command
        .args(sources.iter().map(|source| manifest().join(source)))
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

/// Checks what examples/question.c or .cpp printed.
fn assert_asked(stdout: &str) {
    assert!(
        stdout.contains("BER: 300e0201011609497320312b313d333f\n"),
        "{stdout}"
    );
    assert!(
        stdout.contains("Decoded: id 1, question \"Is 1+1=3?\"\n"),
        "{stdout}"
    );
    assert!(stdout.contains("\"question\": \"Is 1+1=3?\""), "{stdout}");
    assert!(stdout.contains("\nTruncated: "), "{stdout}");
}

fn api_arguments() -> [PathBuf; 2] {
    [
        manifest().join("../asnkitrt-wasm/tests/fixtures/types.asni"),
        manifest().join("../../examples/corpus"),
    ]
}

#[test]
fn the_example_round_trips_a_question() {
    let stdout = compile_and_run(
        &C,
        &["examples/question.c"],
        &[manifest().join("../../examples/compiled/foo.asni")],
    );

    assert_asked(&stdout);
}

#[test]
fn the_api_does_what_the_header_says() {
    let stdout = compile_and_run(&C, &["tests/api.c"], &api_arguments());

    assert_eq!(stdout, "ok\n");
}

#[test]
fn the_cpp_example_round_trips_a_question() {
    let stdout = compile_and_run(
        &CXX,
        &["examples/question.cpp"],
        &[manifest().join("../../examples/compiled/foo.asni")],
    );

    assert_asked(&stdout);
    // The offset of the decoding error, which the exception has.
    assert!(stdout.contains(" (at 2)\n"), "{stdout}");
}

/// tests/api_odr.cpp is a second translation unit with the header, for
/// a function that is not inline to fail the link.
#[test]
fn the_cpp_api_does_what_the_header_says() {
    let stdout = compile_and_run(
        &CXX,
        &["tests/api.cpp", "tests/api_odr.cpp"],
        &api_arguments(),
    );

    assert_eq!(stdout, "ok\n");
}

#[test]
fn the_cpp_header_compiles_as_later_standards() {
    for standard in ["c++20", "c++23"] {
        let (compiler, mut command) = CXX.command(standard);
        let status = command
            .arg("-fsyntax-only")
            .args(
                [
                    "tests/api.cpp",
                    "tests/api_odr.cpp",
                    "examples/question.cpp",
                ]
                .map(|source| manifest().join(source)),
            )
            .status()
            .unwrap_or_else(|error| panic!("Can not run {compiler}: {error}"));

        assert!(status.success(), "{compiler} failed as {standard}");
    }
}
