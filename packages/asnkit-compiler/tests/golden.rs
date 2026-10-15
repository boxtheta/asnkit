// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Holds the compiler to what the Python compiler did, frozen in
//! `tests/golden/cases` (the compiler in Python is not part of this
//! repository). See `tests/golden/README.rst`.
//!

use asnkit_compiler::{Error, Input};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn cases_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/cases")
}

/// Python's own exceptions, which the Python compiler lets out when it
/// cannot handle a specification. A compiler that fails with any error
/// agrees with those.
fn is_python_crash(kind: &str) -> bool {
    matches!(
        kind,
        "KeyError"
            | "TypeError"
            | "AttributeError"
            | "IndexError"
            | "ValueError"
            | "RecursionError"
            | "ZeroDivisionError"
            | "UnboundLocalError"
    )
}

/// Whether two errors are the same for the purpose of the tests: the
/// same kind, and for a syntax error the same file, line and column,
/// which is what an error has to tell. The words of the message are not
/// held to.
fn same_error(a: &Error, b: &Error) -> bool {
    if is_python_crash(&b.kind) {
        return true;
    }

    if a.kind != b.kind {
        return false;
    }

    let place = |message: &str| {
        let file = message
            .split("in '")
            .nth(1)
            .and_then(|rest| rest.split('\'').next())
            .map(str::to_string);
        let line_and_column = message.split(" at line ").nth(1).and_then(|rest| {
            let line = rest.split(',').next()?.to_string();
            let column = rest.split("column ").nth(1)?.split(':').next()?.to_string();

            Some((line, column))
        });

        (file, line_and_column)
    };

    if a.message.starts_with("Invalid ASN.1 syntax")
        && b.message.starts_with("Invalid ASN.1 syntax")
    {
        return place(&a.message) == place(&b.message);
    }

    a.message == b.message
}

fn load_cases() -> Vec<(String, Value)> {
    let mut cases = Vec::new();

    for entry in fs::read_dir(cases_dir()).expect("no cases, run record.py") {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut text = String::new();

        if name.ends_with(".json.gz") {
            let file = fs::File::open(&path).unwrap();
            flate2::read::GzDecoder::new(file)
                .read_to_string(&mut text)
                .unwrap();
        } else if name.ends_with(".json") {
            text = fs::read_to_string(&path).unwrap();
        } else {
            continue;
        }

        cases.push((name, serde_json::from_str(&text).unwrap()));
    }

    cases.sort_by(|a, b| a.0.cmp(&b.0));

    cases
}

/// Makes the input of a case, with the files made by the test in a
/// folder of their own, which is given back to be kept until done.
fn make_input(case: &Value) -> (Input, Option<tempfile::TempDir>) {
    let encoding = case["encoding"].as_str().unwrap().to_string();

    if let Some(text) = case.get("string") {
        return (Input::String(text.as_str().unwrap().to_string()), None);
    }

    if let Some(files) = case.get("files") {
        let paths = files
            .as_array()
            .unwrap()
            .iter()
            .map(|path| root().join(path.as_str().unwrap()))
            .collect();

        return (Input::Files { paths, encoding }, None);
    }

    let directory = tempfile::tempdir().unwrap();
    let mut paths = Vec::new();

    for file in case["inline_files"].as_array().unwrap() {
        let path = directory.path().join(file["name"].as_str().unwrap());
        fs::write(&path, file["text"].as_str().unwrap()).unwrap();
        paths.push(path);
    }

    (Input::Files { paths, encoding }, Some(directory))
}

/// What a case expects of a step, `parse` or `compile`: the value, or the
/// error.
fn expected(case: &Value, step: &str) -> Result<Value, Error> {
    let step = &case[step];

    if step["ok"].as_bool().unwrap() {
        let key = if step.get("tree").is_some() {
            "tree"
        } else {
            "nodes"
        };

        Ok(step[key].clone())
    } else {
        Err(Error {
            kind: step["error"]["type"].as_str().unwrap().to_string(),
            message: step["error"]["message"].as_str().unwrap().to_string(),
        })
    }
}

/// Runs a step for all cases and tells which differ from what is
/// expected.
fn differences(step: &str, run: impl Fn(&Input) -> Result<Value, Error>) -> (usize, Vec<String>) {
    let cases = load_cases();
    let mut differing = Vec::new();

    for (name, case) in &cases {
        let (input, directory) = make_input(case);
        let mut actual = run(&input);

        // The messages of the cases tell where their files were with
        // <DIR>.
        if let (Err(error), Some(directory)) = (&mut actual, &directory) {
            error.message = error
                .message
                .replace(directory.path().to_str().unwrap(), "<DIR>");
        }

        let same = match (&actual, &expected(case, step)) {
            (Err(a), Err(e)) => same_error(a, e),
            (a, e) => a == e,
        };

        if !same {
            differing.push(name.clone());
        }
    }

    (cases.len(), differing)
}

fn report(step: &str, total: usize, differing: &[String]) {
    println!(
        "{step}: {} of {total} cases agree with the Python compiler",
        total - differing.len()
    );

    for name in differing.iter().take(20) {
        println!("  differs: {name}");
    }

    if differing.len() > 20 {
        println!("  and {} more", differing.len() - 20);
    }
}

#[test]
fn cases_are_well_formed() {
    let cases = load_cases();

    assert!(cases.len() > 100, "only {} cases", cases.len());

    for (name, case) in &cases {
        assert!(case["id"].is_string(), "{name}");
        assert!(case["encoding"].is_string(), "{name}");

        let inputs = ["string", "files", "inline_files"]
            .iter()
            .filter(|key| case.get(**key).is_some())
            .count();
        assert_eq!(inputs, 1, "{name} has more or less than one input");

        for step in ["parse", "compile"] {
            let result = &case[step];

            if result["ok"].as_bool().unwrap() {
                let key = if step == "parse" { "tree" } else { "nodes" };
                assert!(!result[key].is_null(), "{name} {step}");
            } else {
                assert!(result["error"]["type"].is_string(), "{name} {step}");
                assert!(result["error"]["message"].is_string(), "{name} {step}");
            }
        }
    }
}

#[test]
fn files_of_cases_are_those_that_were_frozen() {
    for (name, case) in load_cases() {
        let Some(files) = case.get("files") else {
            continue;
        };
        let hashes = case["sha256"].as_array().unwrap();

        for (path, hash) in files.as_array().unwrap().iter().zip(hashes) {
            let bytes = fs::read(root().join(path.as_str().unwrap()))
                .unwrap_or_else(|e| panic!("{name}: {path}: {e}"));
            let digest: String = Sha256::digest(&bytes)
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect();

            assert_eq!(
                digest,
                hash.as_str().unwrap(),
                "{name}: {path} changed since it was frozen. If the change can not change what is compiled, as one in a comment, change the sha256 of the case to the one above."
            );
        }
    }
}

#[test]
fn parse_agrees_with_the_python_compiler() {
    let (total, differing) = differences("parse", asnkit_compiler::parse);

    report("parse", total, &differing);
    assert!(differing.is_empty());
}

#[test]
fn compile_agrees_with_the_python_compiler() {
    let (total, differing) = differences("compile", asnkit_compiler::compile);

    report("compile", total, &differing);
    assert!(differing.is_empty());
}
