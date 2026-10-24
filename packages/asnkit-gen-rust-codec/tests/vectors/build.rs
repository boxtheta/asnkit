// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Makes the code of each specification of tests/vectors, and `run`,
//! which decodes data with it and encodes it again.

use std::fmt::Write;
use std::fs;
use std::path::PathBuf;

use asnkit_gen_rust_codec::{Encoding, Integer, Options, generate_types};

fn main() {
    let specs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../tests/vectors/specs");
    let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());
    let mut modules = String::new();
    let mut arms = String::new();
    let mut entries: Vec<_> = fs::read_dir(&specs)
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();

    entries.sort();
    println!("cargo::rerun-if-changed={}", specs.display());
    println!("cargo::rerun-if-changed=../../src/lib.rs");

    for path in entries {
        let stem = path.file_stem().unwrap().to_string_lossy().into_owned();
        for codec in [Encoding::Ber, Encoding::Der, Encoding::Uper] {
            let encoding = codec.name();
            // An INTEGER of any size, as some vectors have.
            let options = Options {
                codec,
                skip_unsupported: true,
                integer: Integer::Big,
                ..Options::default()
            };
            let Ok((text, _, types)) = generate_types(&fs::read(&path).unwrap(), options) else {
                continue;
            };
            let module = format!("s{stem}_{encoding}");

            fs::write(out.join(format!("{module}.rs")), text).unwrap();
            let _ = writeln!(
                modules,
                "#[allow(dead_code)]\npub mod {module} {{\n    include!(concat!(env!(\"OUT_DIR\"), \"/{module}.rs\"));\n}}"
            );

            // A name that is in more than one module is not looked up.
            for ty in &types {
                if types.iter().filter(|other| other.name == ty.name).count() > 1 {
                    continue;
                }

                let functions = match &ty.functions {
                    None => format!("{module}::{}", ty.rust),
                    Some(functions) => format!("{module}::{functions}"),
                };
                let call = format!(
                    "by_functions(data, {functions}::decode_{encoding}, {functions}::encode_{encoding})"
                );
                let _ = writeln!(
                    arms,
                    "        ({stem:?}, {:?}, {encoding:?}) => Some({call}),",
                    ty.name
                );
            }
        }
    }

    let text = format!(
        "{modules}\n/// Decodes `data` as the type `ty` of `spec` with `codec`, and encodes\n/// what it decoded again, if there is code for the type.\npub fn run(spec: &str, ty: &str, codec: &str, data: &[u8]) -> Option<Result<Vec<u8>>> {{\n    match (spec, ty, codec) {{\n{arms}        _ => None,\n    }}\n}}\n"
    );

    fs::write(out.join("run.rs"), text).unwrap();
}
