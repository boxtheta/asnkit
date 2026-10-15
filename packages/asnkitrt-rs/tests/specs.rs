// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads the compiled specifications of `tests/vectors/specs` as nodes.

use std::fs;
use std::path::PathBuf;

use asnkitrt::container::{self, Checks};
use asnkitrt::node::Spec;

fn specs() -> Vec<(String, Vec<u8>)> {
    let folder = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/vectors/specs");
    let mut specs: Vec<_> = fs::read_dir(folder)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| {
            path.extension()
                .is_some_and(|extension| extension == "asni")
        })
        .map(|path| {
            (
                path.file_stem().unwrap().to_string_lossy().into_owned(),
                fs::read(path).unwrap(),
            )
        })
        .collect();

    specs.sort();
    specs
}

#[test]
fn every_specification_is_read() {
    let specs = specs();

    assert!(specs.len() > 100);

    for (name, data) in specs {
        let asni =
            container::read(&data, &Checks::default()).unwrap_or_else(|e| panic!("{name}: {e}"));
        let spec = Spec::from_nodes(&asni.nodes).unwrap_or_else(|e| panic!("{name}: {e}"));

        assert!(!spec.modules.is_empty(), "{name}");
    }
}

#[test]
fn every_specification_is_lowered() {
    use asnkitrt::types::{Codec, lower};

    let mut failed = Vec::new();

    for (name, data) in specs() {
        let asni = container::read(&data, &Checks::default()).unwrap();
        let spec = Spec::from_nodes(&asni.nodes).unwrap();

        for codec in [Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper] {
            if let Err(error) = lower(&spec, codec, false, false) {
                // Only OER and PER have open types.
                if error.to_string().starts_with("OpenType") {
                    continue;
                }

                // What needs a feature that is not built in, such as bigint.
                #[cfg(not(feature = "bigint"))]
                if matches!(error, asnkitrt::Error::Unsupported(_)) {
                    continue;
                }

                failed.push(format!("{name} {codec:?}: {error}"));
            }
        }
    }

    assert!(failed.is_empty(), "{}", failed.join("\n"));
}
