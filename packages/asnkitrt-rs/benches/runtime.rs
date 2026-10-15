// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! How long the runtime takes to load a specification, to encode and
//! decode a small message and a certificate with each codec, and to check
//! the constraints of the certificate, to see when that changes. See
//! `harness/mod.rs`.
//!
//!   cargo bench -p asnkitrt --bench runtime [-- FILTER...]

mod harness;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

const FOO: &[u8] = include_bytes!("../../../examples/compiled/foo.asnc");
const RFC5280: &[u8] = include_bytes!("../../../examples/compiled/rfc5280.asnc");
// A certificate with all the extensions OpenSSL makes.
const CERTIFICATE: &[u8] = include_bytes!("../tests/fixtures/x509/extensions_all.der");

// The codecs, and if they do the certificate: JER and XER need the types
// of the values of its ANYs.
const CODECS: [(&str, Codec, bool); 7] = [
    ("ber", Codec::Ber, true),
    ("der", Codec::Der, true),
    ("oer", Codec::Oer, true),
    ("per", Codec::Per, true),
    ("uper", Codec::Uper, true),
    ("jer", Codec::Jer, false),
    ("xer", Codec::Xer, false),
];

fn load(data: &[u8], codec: Codec) -> Specification {
    Specification::from_asnc(data, &Checks::default(), Options::new(codec)).unwrap()
}

fn main() {
    let mut benches = harness::Benches::new("runtime");

    benches.run("load/foo", || load(FOO, Codec::Ber));
    benches.run("load/rfc5280", || load(RFC5280, Codec::Der));

    let question = Value::Map(vec![
        ("id".into(), Value::Integer(1)),
        ("question".into(), Value::text("Is 1+1=3?")),
    ]);
    let certificate = load(RFC5280, Codec::Der)
        .decode("Certificate", CERTIFICATE)
        .unwrap();

    for (name, codec, does_certificate) in CODECS {
        let foo_spec = load(FOO, codec);
        let encoded = foo_spec.encode("Question", &question).unwrap();

        benches.run(&format!("{name}/encode/Question"), || {
            foo_spec.encode("Question", &question).unwrap()
        });
        benches.run(&format!("{name}/decode/Question"), || {
            foo_spec.decode("Question", &encoded).unwrap()
        });

        if !does_certificate {
            continue;
        }

        let rfc5280 = load(RFC5280, codec);
        let encoded = rfc5280.encode("Certificate", &certificate).unwrap();

        benches.run(&format!("{name}/encode/Certificate"), || {
            rfc5280.encode("Certificate", &certificate).unwrap()
        });
        benches.run(&format!("{name}/decode/Certificate"), || {
            rfc5280.decode("Certificate", &encoded).unwrap()
        });
    }

    let rfc5280 = load(RFC5280, Codec::Der);

    benches.run("check_constraints/Certificate", || {
        rfc5280
            .check_constraints("Certificate", &certificate)
            .unwrap()
    });
}
