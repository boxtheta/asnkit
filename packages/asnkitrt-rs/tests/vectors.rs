// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Holds the runtime to the vectors in `tests/vectors`. They are what
//! this runtime does, made by `examples/record_vectors.rs`, so that a
//! change of what it does is seen, and the vectors made again with it.
//!
//! `VECTORS_CODEC=ber` runs the vectors of one codec, and
//! `VECTORS_VERBOSE=1` tells each one that fails.

mod common;

use std::collections::HashMap;
use std::fs;
use std::io::Read;

use asnkitrt::runtime::Specification;
use common::{Done, SpecKey, hex, hex_of, load, perform, same, spec_key, to_value};
use serde_json::Value as Json;

/// A specification, or why it is not loaded and if that is that it needs
/// what is not built in, such as the feature bigint.
type Loaded = Result<Specification, (String, bool)>;

#[test]
fn vectors() {
    let only = std::env::var("VECTORS_CODEC").ok();
    let verbose = std::env::var("VECTORS_VERBOSE").is_ok();
    let mut text = String::new();

    flate2::read::GzDecoder::new(
        fs::File::open(common::folder().join("vectors.jsonl.gz")).unwrap(),
    )
    .read_to_string(&mut text)
    .unwrap();

    let mut specs: HashMap<SpecKey, Loaded> = HashMap::new();
    let (mut passed, mut skipped) = (
        HashMap::<String, usize>::new(),
        HashMap::<String, usize>::new(),
    );
    let mut failures: Vec<String> = Vec::new();
    let unsupported = |error: &asnkitrt::Error| matches!(error, asnkitrt::Error::Unsupported(..));

    for line in text.lines() {
        let vector: Json = serde_json::from_str(line).unwrap();
        let key = spec_key(&vector);
        let codec_name = key.1.clone();

        if only.as_ref().is_some_and(|only| only != &codec_name) {
            continue;
        }

        let spec = specs.entry(key.clone()).or_insert_with(|| {
            load(&key).map_err(|error| (error.to_string(), unsupported(&error)))
        });
        let describe = format!(
            "{codec_name} {} {} {}: {}",
            vector["operation"], key.0, vector["type"], vector["value"]
        );
        let expected_error = vector["error"].as_str();

        let spec = match spec {
            Ok(spec) => spec,
            Err((_, true)) => {
                *skipped
                    .entry(format!("{codec_name} unsupported"))
                    .or_default() += 1;
                continue;
            }
            Err((error, false)) => {
                failures.push(format!("not loaded: {error} <= {describe}"));
                continue;
            }
        };
        let outcome: Result<(), String> = match (perform(spec, &vector), expected_error) {
            (Err(error), None) if unsupported(&error) => {
                *skipped
                    .entry(format!("{codec_name} unsupported"))
                    .or_default() += 1;
                continue;
            }
            (Ok(Done::Encoded(bytes)), None) => {
                let encoded = hex(vector["encoded"].as_str().unwrap());

                if bytes == encoded {
                    Ok(())
                } else {
                    Err(format!(
                        "encoded {} instead of {}",
                        hex_of(&bytes),
                        hex_of(&encoded)
                    ))
                }
            }
            (Ok(Done::Decoded(decoded)), None) => {
                if same(&decoded, &to_value(&vector["value"])) {
                    Ok(())
                } else {
                    Err(format!("decoded {decoded:?}"))
                }
            }
            (Err(_), Some(_)) => Ok(()),
            (Ok(Done::Encoded(bytes)), Some(error)) => {
                Err(format!("encoded {} but expected {error}", hex_of(&bytes)))
            }
            (Ok(Done::Decoded(decoded)), Some(error)) => {
                Err(format!("decoded {decoded:?} but expected {error}"))
            }
            (Err(error), None) => Err(format!("failed: {error}")),
        };

        match outcome {
            Ok(()) => *passed.entry(codec_name).or_default() += 1,
            Err(message) => failures.push(format!(
                "{} <= {describe}",
                message.chars().take(160).collect::<String>()
            )),
        }
    }

    println!("passed: {passed:?}");
    println!("skipped: {skipped:?}");
    println!("failed: {}", failures.len());

    if verbose {
        for failure in failures.iter().take(60) {
            println!("{}", failure.chars().take(300).collect::<String>());
        }
    }

    assert!(
        failures.is_empty(),
        "{} vectors failed, see VECTORS_VERBOSE",
        failures.len()
    );
}
