// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

#![no_main]

use asnkitrt::container::{self, Checks, HEADER_SIZE};
use asnkitrt::node::Spec;
use libfuzzer_sys::fuzz_target;

// An .asni file that is anything, as it is and with the CRC-32 of its
// header made right, so that the CBOR after it is read too, which is as
// far as it can be taken, and the nodes of it, which are lowered for
// every codec.
fuzz_target!(|data: &[u8]| {
    let _ = container::read_header(data);
    let _ = container::read(data, &Checks::default());

    let mut data = data.to_vec();

    if data.len() >= HEADER_SIZE {
        let crc = container::crc32(&data[HEADER_SIZE..]);
        data[6..HEADER_SIZE].copy_from_slice(&crc.to_be_bytes());
    }

    if let Ok(file) = container::read(&data, &Checks::default()) {
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
