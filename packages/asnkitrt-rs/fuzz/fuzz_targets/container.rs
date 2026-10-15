// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

#![no_main]

use asnkitrt::container::{self, Binding, Checks, Nonce, Signature};
use asnkitrt::node::Spec;
use libfuzzer_sys::fuzz_target;

// An .asni file that is anything, read by a reader that accepts any
// signature, nonce and binding, which is as far as it can be taken, and
// the nodes of it, which are lowered for every codec.
fuzz_target!(|data: &[u8]| {
    let _ = container::read_header(data);

    let checks = Checks {
        signature: Signature::Any,
        nonce: Nonce::Any,
        bound_to: Binding::Any,
        ..Checks::default()
    };

    if let Ok(file) = container::read(data, &checks) {
        if let Ok(spec) = Spec::from_nodes(&file.nodes) {
            for codec in [
                asnkitrt::types::Codec::Ber,
                asnkitrt::types::Codec::Der,
                asnkitrt::types::Codec::Oer,
                asnkitrt::types::Codec::Per,
                asnkitrt::types::Codec::Uper,
            ] {
                let _ = asnkitrt::types::lower(&spec, codec, false, false);
            }
        }
    }
});
