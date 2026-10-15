// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

#![no_main]

use std::sync::OnceLock;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use libfuzzer_sys::fuzz_target;

const RFC5280: &[u8] = include_bytes!("../../../../examples/compiled/rfc5280.asni");

fn specifications() -> &'static Vec<Specification> {
    static SPECIFICATIONS: OnceLock<Vec<Specification>> = OnceLock::new();

    SPECIFICATIONS.get_or_init(|| {
        [Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper, Codec::Jer, Codec::Xer]
            .into_iter()
            .filter_map(|codec| Specification::from_asni(RFC5280, &Checks::default(), Options::new(codec)).ok())
            .collect()
    })
}

// Data that is anything, decoded as X.509 with every codec: a value or
// an error. What is decoded is encoded again, which has to work too.
fuzz_target!(|data: &[u8]| {
    for specification in specifications() {
        for name in ["Certificate", "CertificateList", "Name", "GeneralName"] {
            if let Ok(value) = specification.decode(name, data) {
                let _ = specification.encode(name, &value);
                let _ = specification.check_constraints(name, &value);
            }
        }
    }
});
