// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What a decoded value borrows: the names of its members from the
//! specification and its texts and bytes from the data, so that a
//! decoder does not allocate them, and what `Value::into_owned` keeps
//! when they are gone.

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

/// How many texts, bytes, bits and object identifiers of `value` are
/// borrowed and how many are owned.
fn data(value: &Value<'_>) -> (usize, usize) {
    let one = |borrowed: bool| if borrowed { (1, 0) } else { (0, 1) };
    let sum = |values: &mut dyn Iterator<Item = &Value<'_>>| {
        values
            .map(data)
            .fold((0, 0), |(a, b), (c, d)| (a + c, b + d))
    };

    match value {
        Value::Text(text) => one(matches!(text, Cow::Borrowed(_))),
        Value::Bytes(bytes) | Value::Bits(bytes, _) => one(matches!(bytes, Cow::Borrowed(_))),
        Value::ObjectIdentifier(oid) => one(matches!(oid.clone().into_bytes(), Cow::Borrowed(_))),
        Value::Map(members) => sum(&mut members.iter().map(|(_, member)| member)),
        Value::Choice(_, inner) => data(inner),
        Value::List(items) => sum(&mut items.iter()),
        _ => (0, 0),
    }
}

/// Whether `value` borrows nothing.
fn is_owned(value: &Value<'_>) -> bool {
    let owned = match value {
        Value::Map(members) => members
            .iter()
            .all(|(name, member)| matches!(name, Cow::Owned(_)) && is_owned(member)),
        Value::Choice(name, inner) => matches!(name, Cow::Owned(_)) && is_owned(inner),
        Value::List(items) => items.iter().all(is_owned),
        _ => true,
    };

    owned && data(value).0 == 0
}

#[test]
fn the_binary_codecs_borrow_the_names() {
    let der = compiled("rfc5280.asni", Codec::Der);
    let data = certificate();
    let decoded = der.decode("Certificate", &data).unwrap();

    assert!(borrowed_names(&decoded) > 50);

    for codec in [Codec::Ber, Codec::Oer, Codec::Per, Codec::Uper] {
        let spec = compiled("rfc5280.asni", codec);
        let encoded = spec.encode("Certificate", &decoded).unwrap();
        let again = spec.decode("Certificate", &encoded).unwrap();

        // DER adds the members left at their default at the end, which
        // the others do not, so it is the encodings that are compared.
        assert_eq!(
            borrowed_names(&again),
            borrowed_names(&decoded),
            "{codec:?}"
        );
        assert_eq!(
            spec.encode("Certificate", &again).unwrap(),
            encoded,
            "{codec:?}"
        );
    }
}

#[test]
fn the_binary_codecs_borrow_from_the_data() {
    let der = compiled("rfc5280.asni", Codec::Der);
    let data = certificate();
    let decoded = der.decode("Certificate", &data).unwrap();
    let (borrowed, owned) = self::data(&decoded);

    // The certificate has its texts as PrintableString and UTF8String,
    // whose bytes are their UTF-8.
    assert!(borrowed > 30, "{borrowed}");
    assert_eq!(owned, 0);

    // Most of what UPER has is not on a byte.
    for (codec, all) in [
        (Codec::Ber, true),
        (Codec::Oer, true),
        (Codec::Per, true),
        (Codec::Uper, false),
    ] {
        let spec = compiled("rfc5280.asni", codec);
        let encoded = spec.encode("Certificate", &decoded).unwrap();
        let again = spec.decode("Certificate", &encoded).unwrap();
        let (borrowed, owned) = self::data(&again);

        assert_eq!(borrowed + owned, self::data(&decoded).0, "{codec:?}");

        if all {
            assert_eq!(owned, 0, "{codec:?}");
        } else {
            assert!(owned > 0, "{codec:?}");
        }
    }
}

/// What BER sends in parts is put together, and what is not the UTF-8 of
/// a text is made into it.
#[test]
fn what_is_not_its_encoding_is_owned() {
    let spec = compiled("rfc5280.asni", Codec::Ber);
    // A constructed OCTET STRING in two parts.
    let parts = [0x24, 0x80, 0x04, 0x01, 0x01, 0x04, 0x01, 0x02, 0x00, 0x00];
    let decoded = spec.decode("KeyIdentifier", &parts).unwrap();

    assert_eq!(decoded, Value::Bytes(vec![1, 2].into()));
    assert_eq!(data(&decoded), (0, 1));

    let primitive = [0x04, 0x02, 0x01, 0x02];

    assert_eq!(
        data(&spec.decode("KeyIdentifier", &primitive).unwrap()),
        (1, 0)
    );
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
