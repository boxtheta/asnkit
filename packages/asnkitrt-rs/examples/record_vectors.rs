// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Makes the vectors in `tests/vectors` from their cases: what this
//! runtime encodes and decodes is what every runtime is held to.
//! `tests/vectors/record.py` runs it; see there.
//!
//! It reads cases from stdin, one JSON object per line, written as the
//! vectors are but for what an operation gives: an `encode` needs its
//! `value` and a `decode` its `encoded`. For each it writes a line to
//! stdout, the vector with `value`, `encoded` and `error` as this
//! runtime has them, or `null` if it has nothing to say: if it does not
//! load the specification or does not support the operation.

#[path = "../tests/common/mod.rs"]
mod common;

use std::collections::HashMap;
use std::io::{BufRead, Write};

use asnkitrt::runtime::Specification;
use common::{Done, SpecKey, error_name, hex_of, load, perform, spec_key, to_json};
use serde_json::Value as Json;

fn record(spec: &Result<Specification, asnkitrt::Error>, mut vector: Json) -> Json {
    let Ok(spec) = spec else {
        return Json::Null;
    };

    match perform(spec, &vector) {
        Ok(Done::Encoded(bytes)) => {
            vector["encoded"] = hex_of(&bytes).into();
            vector["error"] = Json::Null;
        }
        Ok(Done::Decoded(value)) => {
            vector["value"] = to_json(&value);
            vector["error"] = Json::Null;
        }
        Err(asnkitrt::Error::Unsupported(_)) => return Json::Null,
        Err(error) => {
            if vector["operation"] == "encode" {
                vector["encoded"] = "".into();
            } else {
                vector["value"] = Json::Null;
            }

            vector["error"] = error_name(&error).into();
        }
    }

    vector
}

fn main() {
    let mut specs: HashMap<SpecKey, Result<Specification, asnkitrt::Error>> = HashMap::new();
    let mut out = std::io::BufWriter::new(std::io::stdout().lock());

    for line in std::io::stdin().lock().lines() {
        let vector: Json = serde_json::from_str(&line.unwrap()).unwrap();
        let key = spec_key(&vector);
        let spec = specs.entry(key.clone()).or_insert_with(|| load(&key));

        writeln!(out, "{}", record(spec, vector)).unwrap();
    }
}
