// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

// Assignments that are anything, in a module that is right, so that the
// fuzzer spends its time on the types, values and constraints, where the
// compiler follows references, and not on the header of the module.
fuzz_target!(|data: &[u8]| {
    common::compile_and_load(format!(
        "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n{}\nEND\n",
        String::from_utf8_lossy(data)
    ));
});
