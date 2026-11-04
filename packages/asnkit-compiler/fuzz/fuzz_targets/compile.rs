// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

#![no_main]

mod common;

use libfuzzer_sys::fuzz_target;

// The text of specifications that is anything, as a file read by the
// compiler would be: an error or the files of it, in as long as the
// text, never a panic, a stack that runs out or no end.
fuzz_target!(|data: &[u8]| {
    common::compile_and_load(String::from_utf8_lossy(data).into_owned());
});
