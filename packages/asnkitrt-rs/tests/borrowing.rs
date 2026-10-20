// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What a decoded value borrows: the names of its members from the
//! specification, so that a decoder does not allocate them, and what
//! `Value::into_owned` keeps when the specification is gone.

use std::borrow::Cow;
use std::fs;
use std::path::PathBuf;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compiled(name: &str, codec: Codec) -> Specification {
    let data = fs::read(root().join("../../examples/compiled").join(name)).unwrap();

    Specification::from_asni(&data, &Checks::default(), Options::new(codec)).unwrap()
}

fn certificate() -> Vec<u8> {
    fs::read(root().join("tests/fixtures/x509/extensions_all.der")).unwrap()
}

/// How many names `value` has, after checking that all are borrowed.
fn borrowed_names(value: &Value<'_>) -> usize {
    match value {
        Value::Map(members) => members
            .iter()
            .map(|(name, member)| {
                assert!(matches!(name, Cow::Borrowed(_)), "{name} is owned");

                1 + borrowed_names(member)
            })
            .sum(),
        Value::Choice(name, inner) => {
            assert!(matches!(name, Cow::Borrowed(_)), "{name} is owned");

            1 + borrowed_names(inner)
        }
        Value::List(items) => items.iter().map(borrowed_names).sum(),
        _ => 0,
    }
}

/// Whether `value` borrows nothing.
fn is_owned(value: &Value<'_>) -> bool {
    match value {
        Value::Map(members) => members
            .iter()
            .all(|(name, member)| matches!(name, Cow::Owned(_)) && is_owned(member)),
        Value::Choice(name, inner) => matches!(name, Cow::Owned(_)) && is_owned(inner),
        Value::List(items) => items.iter().all(is_owned),
        _ => true,
    }
}

#[test]
fn the_binary_codecs_borrow_the_names() {
    let der = compiled("rfc5280.asni", Codec::Der);
    let decoded = der.decode("Certificate", &certificate()).unwrap();

    assert!(borrowed_names(&decoded) > 50);

    for codec in [Codec::Ber, Codec::Oer, Codec::Per, Codec::Uper] {
        let spec = compiled("rfc5280.asni", codec);
        let encoded = spec.encode("Certificate", &decoded).unwrap();
        let again = spec.decode("Certificate", &encoded).unwrap();

        // DER adds the members left at their default at the end, which
        // the others do not, so it is the encodings that are compared.
        assert_eq!(borrowed_names(&again), borrowed_names(&decoded), "{codec:?}");
        assert_eq!(spec.encode("Certificate", &again).unwrap(), encoded, "{codec:?}");
    }
}

#[cfg(all(feature = "jer", feature = "xer", feature = "gser"))]
#[test]
fn the_text_codecs_borrow_the_names() {
    let question = Value::Map(vec![
        ("id".into(), Value::Integer(1)),
        ("question".into(), Value::text("Is 1+1=3?")),
    ]);

    for codec in [Codec::Jer, Codec::Xer, Codec::Gser] {
        let spec = compiled("foo.asni", codec);
        let encoded = spec.encode("Question", &question).unwrap();
        let decoded = spec.decode("Question", &encoded).unwrap();

        assert_eq!(borrowed_names(&decoded), 2, "{codec:?}");
        assert_eq!(decoded, question, "{codec:?}");
    }
}

#[test]
fn an_owned_value_outlives_the_specification() {
    let data = certificate();
    let owned = {
        let spec = compiled("rfc5280.asni", Codec::Der);

        spec.decode("Certificate", &data).unwrap().into_owned()
    };

    assert!(is_owned(&owned));

    let spec = compiled("rfc5280.asni", Codec::Der);

    assert_eq!(owned, spec.decode("Certificate", &data).unwrap());
    assert_eq!(spec.encode("Certificate", &owned).unwrap(), data);
}
