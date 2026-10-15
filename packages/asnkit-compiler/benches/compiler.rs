// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! How long the compiler takes to parse and compile specifications, to
//! see when that changes. The harness is that of the runtime for Rust.
//!
//!   cargo bench -p asnkit-compiler --bench compiler [-- FILTER...]

#[path = "../../asnkitrt-rs/benches/harness/mod.rs"]
mod harness;

use asnkit_compiler::Input;
use asnkit_compiler::container::Protection;

const SPECIFICATIONS: [(&str, &str); 2] = [
    (
        "rfc5280",
        include_str!("../../../examples/compiled/rfc5280.asn"),
    ),
    (
        "information_object",
        include_str!("../../../examples/compiled/information_object.asn"),
    ),
];

fn main() {
    let mut benches = harness::Benches::new("compiler");

    for (name, text) in SPECIFICATIONS {
        let input = Input::String(text.into());

        benches.run(&format!("parse/{name}"), || {
            asnkit_compiler::parse(&input).unwrap()
        });
        benches.run(&format!("compile/{name}"), || {
            asnkit_compiler::compile(&input).unwrap()
        });
        benches.run(&format!("asni/{name}"), || {
            asnkit_compiler::compile_to_asni(&input, Some(0), None, &Protection::default()).unwrap()
        });
    }
}
