// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

#![no_main]

use libfuzzer_sys::fuzz_target;

// The CBOR reader, with any data: an item or an error, and no more
// memory than the size of the data asks for.
fuzz_target!(|data: &[u8]| {
    let _ = asnkitrt::cbor::parse(data, asnkitrt::cbor::DEFAULT_MAX_DEPTH);
});
