// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Shows how far the compiler is from agreeing with the golden cases.
//!
//! ```text
//! cargo run --release --example golden -- parse            # the summary
//! cargo run --release --example golden -- parse foo        # the first difference of cases named so
//! ```

use asnkit_compiler::{Error, Input};
use serde_json::Value;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn load_cases() -> Vec<(String, Value)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/cases");
    let mut cases = Vec::new();

    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let name = path.file_name().unwrap().to_string_lossy().to_string();
        let mut text = String::new();

        if name.ends_with(".json.gz") {
            flate2::read::GzDecoder::new(fs::File::open(&path).unwrap())
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

/// The path of the first difference, and the two values there.
fn first_difference(path: String, a: &Value, b: &Value) -> Option<(String, String, String)> {
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            let mut keys: Vec<_> = x.keys().chain(y.keys()).collect();
            keys.sort();
            keys.dedup();

            for key in keys {
                let found = match (x.get(key), y.get(key)) {
                    (Some(p), Some(q)) => first_difference(format!("{path}.{key}"), p, q),
                    (p, q) if p != q => Some((
                        format!("{path}.{key}"),
                        p.map(|v| v.to_string()).unwrap_or("(missing)".into()),
                        q.map(|v| v.to_string()).unwrap_or("(missing)".into()),
                    )),
                    _ => None,
                };

                if found.is_some() {
                    return found;
                }
            }

            None
        }
        (Value::Array(x), Value::Array(y)) => {
            for index in 0..x.len().max(y.len()) {
                let found = match (x.get(index), y.get(index)) {
                    (Some(p), Some(q)) => first_difference(format!("{path}[{index}]"), p, q),
                    (p, q) => Some((
                        format!("{path}[{index}]"),
                        p.map(|v| v.to_string()).unwrap_or("(missing)".into()),
                        q.map(|v| v.to_string()).unwrap_or("(missing)".into()),
                    )),
                };

                if found.is_some() {
                    return found;
                }
            }

            None
        }
        _ if a == b => None,
        _ => Some((path, a.to_string(), b.to_string())),
    }
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

fn cut(text: String) -> String {
    if text.len() > 300 {
        format!("{}...", text.chars().take(300).collect::<String>())
    } else {
        text
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let step = args.get(1).map(String::as_str).unwrap_or("parse");
    let filter = args.get(2);
    let mut agree = 0;
    let mut exact = 0;
    let mut total = 0;

    for (name, case) in load_cases() {
        if let Some(filter) = filter
            && !name.contains(filter.as_str())
        {
            continue;
        }

        total += 1;
        let (input, directory) = make_input(&case);
        let mut actual = match step {
            "parse" => asnkit_compiler::parse(&input),
            _ => asnkit_compiler::compile(&input),
        };

        if let (Err(error), Some(directory)) = (&mut actual, &directory) {
            error.message = error
                .message
                .replace(directory.path().to_str().unwrap(), "<DIR>");
        }

        let expected = expected(&case, step);
        let same = match (&actual, &expected) {
            (Err(a), Err(e)) => {
                if a == e {
                    exact += 1;
                }

                same_error(a, e)
            }
            _ => actual == expected,
        };

        if same {
            agree += 1;
        } else if filter.is_some() {
            println!("== {name}");

            match (&actual, &expected) {
                (Ok(a), Ok(e)) => {
                    if let Some((path, got, want)) = first_difference(String::new(), a, e) {
                        println!("at {path}\n  got:  {}\n  want: {}", cut(got), cut(want));
                    }
                }
                (a, e) => println!("got:  {a:?}\nwant: {e:?}"),
            }
        } else {
            println!("differs: {name}");
        }
    }

    println!("{step}: {agree} of {total} agree ({exact} errors with the same words)");
}
