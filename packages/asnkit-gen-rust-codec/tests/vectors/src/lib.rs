// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The code asnkit-gen-rust-codec makes, held to the vectors of
//! tests/vectors: what it decodes, it encodes again to the bytes that
//! the runtime does, and what the runtime does not decode, neither does
//! it.
//!
//!   cargo test --manifest-path packages/asnkit-gen-rust-codec/tests/vectors/Cargo.toml

use asnkitrt::wire::{Error, Result};

include!(concat!(env!("OUT_DIR"), "/run.rs"));

/// The bytes an encoding into a buffer of the size it needs gives.
fn encoded(encode: impl Fn(&mut [u8]) -> Result<usize>) -> Result<Vec<u8>> {
    let length = match encode(&mut []) {
        Ok(length) | Err(Error::BufferTooSmall(length)) => length,
        Err(error) => return Err(error),
    };
    let mut out = vec![0; length];
    let written = encode(&mut out)?;

    assert_eq!(
        written, length,
        "The length an encoding needs is what it writes."
    );

    Ok(out)
}

fn by_functions<'a, T>(
    data: &'a [u8],
    decode: fn(&'a [u8]) -> Result<T>,
    encode: fn(&T, &mut [u8]) -> Result<usize>,
) -> Result<Vec<u8>> {
    let value = decode(data)?;
    encoded(|out| encode(&value, out))
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    use asnkitrt::container::Checks;
    use asnkitrt::runtime::{Options, Specification};
    use asnkitrt::types::Codec;
    use asnkitrt::wire::Error;
    use serde_json::Value as Json;

    /// Where the code does not do what the runtime does, on purpose, and
    /// why.
    const DIFFERENT: &[(&str, &str, &str)] = &[
        (
            "ae6da67ff033cda8",
            "Sequence12",
            "a SEQUENCE that is not extensible with more in it is not decoded",
        ),
        (
            "fb94b0f8e76c6e71",
            "Fie",
            "an ANY DEFINED BY is any value, not one of those of its table",
        ),
        (
            "833213ee8ef1d931",
            "Fie",
            "an ANY DEFINED BY is any value, not one of those of its table",
        ),
        (
            "6b60beea8b26cde8",
            "Fie",
            "an ANY DEFINED BY is any value, not one of those of its table",
        ),
    ];

    fn folder() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tests/vectors")
    }

    fn hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
            .collect()
    }

    fn hex_of(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }

    #[test]
    fn the_code_does_what_the_runtime_does() {
        let file = std::fs::File::open(folder().join("vectors.jsonl.gz")).unwrap();
        let lines = BufReader::new(flate2::read::GzDecoder::new(file)).lines();
        let mut specs: HashMap<(String, String), Specification> = HashMap::new();
        let mut failures = Vec::new();
        let (mut passed, mut skipped, mut unsupported, mut different) = (0, 0, 0, 0);
        // The vectors of types the plugin makes no code for.
        let mut no_code = 0;

        for line in lines {
            let vector: Json = serde_json::from_str(&line.unwrap()).unwrap();
            let codec = vector["codec"].as_str().unwrap();

            if !["ber", "der", "uper"].contains(&codec)
                || vector["lenient"] == true
                || vector["numeric_enums"] == true
            {
                continue;
            }

            let spec = vector["spec"].as_str().unwrap();
            let ty = vector["type"].as_str().unwrap();
            let data = hex(vector["encoded"].as_str().unwrap_or_default());
            let error = vector["error"].as_str();
            let decode = vector["operation"] == "decode";

            // What is encoded has to be decoded to be encoded again, and
            // the constraints of a value are not checked.
            if !decode && error.is_some() || error == Some("Constraints") {
                skipped += 1;

                continue;
            }

            let Some(result) = super::run(spec, ty, codec, &data) else {
                no_code += 1;

                continue;
            };
            let expected = match error {
                Some(_) => None,
                None if decode => {
                    let runtime = specs
                        .entry((spec.to_string(), codec.to_string()))
                        .or_insert_with(|| {
                            let asni =
                                std::fs::read(folder().join(format!("specs/{spec}.asni"))).unwrap();
                            let codec = match codec {
                                "ber" => Codec::Ber,
                                "der" => Codec::Der,
                                _ => Codec::Uper,
                            };

                            Specification::from_asni(&asni, &Checks::default(), Options::new(codec))
                                .unwrap()
                        });
                    let value = runtime.decode(ty, &data).unwrap();

                    match runtime.encode(ty, &value) {
                        Ok(bytes) => Some(bytes),
                        // The runtime does not encode it again.
                        Err(_) => {
                            skipped += 1;

                            continue;
                        }
                    }
                }
                None => Some(data.clone()),
            };
            let what = format!("{spec} {ty} {codec} {}", vector["operation"]);

            if let Err(Error::Unsupported(..)) = result {
                unsupported += 1;

                continue;
            }

            if DIFFERENT.iter().any(|(s, t, _)| *s == spec && *t == ty) {
                different += 1;

                continue;
            }

            match (result, expected) {
                (Ok(bytes), Some(expected)) if bytes == expected => passed += 1,
                (Ok(bytes), Some(expected)) => failures.push(format!(
                    "{what}: {} is {}, not {}",
                    hex_of(&data),
                    hex_of(&bytes),
                    hex_of(&expected)
                )),
                (Err(_), None) => passed += 1,
                (Ok(bytes), None) => failures.push(format!(
                    "{what}: {} is decoded and is {}, where the runtime fails",
                    hex_of(&data),
                    hex_of(&bytes)
                )),
                (Err(error), Some(_)) => {
                    failures.push(format!("{what}: {} fails: {error}", hex_of(&data)))
                }
            }
        }

        println!(
            "{passed} passed, {skipped} skipped, {no_code} of types with no code, {unsupported} not supported by the code, {different} different on purpose, {} failed",
            failures.len()
        );

        for failure in &failures {
            println!("{failure}");
        }

        assert!(failures.is_empty(), "{} vectors failed", failures.len());
    }
}
