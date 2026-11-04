// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

use std::sync::OnceLock;

use asnkit_compiler::{CompileOptions, Input, compile_to_asni_with_warnings, parse_to_asnst};
use asnkitrt::container::{self, Checks};
use asnkitrt::node::Spec;
use asnkitrt::types::{Codec, lower};

/// With `ASNKIT_FUZZ_STRICT` set, what the compiler takes and the
/// runtime refuses is a failure too, as the compiler is to refuse what no
/// runtime can use.
fn strict() -> bool {
    static STRICT: OnceLock<bool> = OnceLock::new();

    *STRICT.get_or_init(|| std::env::var_os("ASNKIT_FUZZ_STRICT").is_some())
}

/// A specification that is anything: its syntax tree, its `.asni` file or
/// an error, and the file is loaded by the runtime and lowered for every
/// codec that is binary.
pub fn compile_and_load(text: String) {
    let input = Input::String(text);

    let _ = parse_to_asnst(&input);

    let Ok(compiled) = compile_to_asni_with_warnings(&input, &CompileOptions::default()) else {
        return;
    };

    let loaded = container::read(&compiled.asni, &Checks::default())
        .map_err(|error| format!("{error:?}"))
        .and_then(|file| Spec::from_nodes(&file.nodes).map_err(|error| format!("{error:?}")));

    let spec = match loaded {
        Ok(spec) => spec,
        Err(error) if strict() => panic!("the runtime refuses what the compiler made: {error}"),
        Err(_) => return,
    };

    for codec in [Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper] {
        if let Err(error) = lower(&spec, codec, false, false) {
            let error = format!("{error:?}");

            // CHARACTER STRING and EMBEDDED PDV, which the Rust runtime
            // does not support, as the README tells.
            if strict() && !error.contains("A type is not supported.") {
                panic!("the runtime cannot lower for {codec:?} what the compiler made: {error}");
            }
        }
    }
}
