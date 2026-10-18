// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! GSER, which is encoded as the runtime for Python encodes it, and not
//! decoded.

#![cfg(feature = "gser")]

mod common;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

use asnkitrt::Error;
use asnkitrt::cbor::Cbor;
use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;
use serde_json::Value as Json;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn compiled(name: &str, codec: Codec) -> Specification {
    let path = root().join("../../examples/compiled").join(name);

    Specification::from_asni(
        &fs::read(path).unwrap(),
        &Checks::default(),
        Options::new(codec),
    )
    .unwrap()
}

/// `M DEFINITIONS ::= BEGIN T ::= <builtin> END`.
fn builtin(builtin: &str) -> Specification {
    let nodes = Cbor::Array(vec![
        Cbor::Int(1),
        Cbor::Array(vec![
            Cbor::Int(0),
            Cbor::Array(vec![
                Cbor::Int(2),
                Cbor::Text("M".into()),
                Cbor::Array(vec![
                    Cbor::Int(0),
                    Cbor::Array(vec![
                        Cbor::Int(3),
                        Cbor::Text("T".into()),
                        Cbor::Array(vec![Cbor::Int(10), Cbor::Text(builtin.into())]),
                    ]),
                ]),
            ]),
        ]),
    ]);

    Specification::from_nodes(&nodes, Options::new(Codec::Gser)).unwrap()
}

fn question(id: i128, text: &str) -> Value {
    Value::Map(vec![
        ("id".into(), Value::Integer(id)),
        ("question".into(), Value::text(text)),
    ])
}

#[test]
fn is_written_as_python_writes_it() {
    let foo = compiled("foo.asni", Codec::Gser);

    assert_eq!(
        foo.encode("Question", &question(1, "Is 1+1=3?")).unwrap(),
        br#"question Question ::= { id 1, question "Is 1+1=3?" }"#
    );
    assert_eq!(
        foo.encode_indented("Question", &question(1, "Is 1+1=3?"), 2)
            .unwrap(),
        b"question Question ::= {\n  id 1,\n  question \"Is 1+1=3?\"\n}"
    );
    assert_eq!(
        foo.encode_in("Foo", "Question", &question(-7, "")).unwrap(),
        br#"question Question ::= { id -7, question "" }"#
    );

    // A missing member.
    assert_eq!(
        foo.encode(
            "Question",
            &Value::Map(vec![("id".into(), Value::Integer(1))])
        )
        .unwrap_err()
        .to_string(),
        "Question: A member is not found in the SEQUENCE or SET."
    );
}

#[test]
fn members_are_in_the_order_they_are_defined_in() {
    // F ::= SEQUENCE { a BOOLEAN, ..., b BOOLEAN, ..., c BOOLEAN }, where
    // the other codecs have b, the addition, last.
    let data = fs::read(common::folder().join("specs/2991335b3c2e374d.asni")).unwrap();
    let spec =
        Specification::from_asni(&data, &Checks::default(), Options::new(Codec::Gser)).unwrap();
    let value = Value::Map(vec![
        ("a".into(), Value::Boolean(true)),
        ("b".into(), Value::Boolean(true)),
        ("c".into(), Value::Boolean(false)),
    ]);

    assert_eq!(
        spec.encode("F", &value).unwrap(),
        b"f F ::= { a TRUE, b TRUE, c FALSE }"
    );
}

#[test]
fn a_quote_is_written_twice() {
    let foo = compiled("foo.asni", Codec::Gser);

    assert_eq!(
        foo.encode("Question", &question(2, r#"say "hi""#)).unwrap(),
        br#"question Question ::= { id 2, question "say ""hi""" }"#
    );
}

#[test]
fn is_not_decoded() {
    let foo = compiled("foo.asni", Codec::Gser);

    assert_eq!(
        foo.decode("Question", br#"{ id 1, question "q" }"#),
        Err(Error::unsupported("GSER is encoded only."))
    );
}

#[test]
fn builtins() {
    for (builtin_name, value, encoded) in [
        ("REAL", Value::Real(0.0), "0"),
        ("REAL", Value::Real(-0.0), "0"),
        ("REAL", Value::Real(f64::INFINITY), "PLUS-INFINITY"),
        ("REAL", Value::Real(f64::NEG_INFINITY), "MINUS-INFINITY"),
        ("REAL", Value::Real(1.0), "1.0E0"),
        ("REAL", Value::Real(1.1), "1.1E0"),
        ("REAL", Value::Real(1234.5678), "1234.5678E0"),
        ("REAL", Value::Integer(8), "8.0E0"),
        ("REAL", Value::Real(0.625), "0.625E0"),
        ("REAL", Value::Real(1e22), "1E22"),
        ("REAL", Value::Real(0.00001), "1E-5"),
        ("NULL", Value::Null, "NULL"),
        ("BOOLEAN", Value::Boolean(true), "TRUE"),
        (
            "INTEGER",
            Value::BigInteger(vec![0x01; 17]),
            "341616807575530379006368233343265341697",
        ),
        (
            "OCTET STRING",
            Value::Bytes(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef]),
            "'0123456789ABCDEF'H",
        ),
        ("OCTET STRING", Value::Bytes(vec![]), "''H"),
        ("BIT STRING", Value::Bits(vec![0xf0], 5), "'11110'B"),
        (
            "BIT STRING",
            Value::Bits(vec![0x12, 0x34], 16),
            "'0001001000110100'B",
        ),
        ("BIT STRING", Value::Bits(vec![], 0), "''B"),
        (
            "OBJECT IDENTIFIER",
            Value::text("1.2.840.113549"),
            "1.2.840.113549",
        ),
        ("UTF8String", Value::text("\u{102}"), "\"\u{102}\""),
        ("NumericString", Value::text("01 23"), "\"01 23\""),
    ] {
        let spec = builtin(builtin_name);

        assert_eq!(
            String::from_utf8(spec.encode("T", &value).unwrap()).unwrap(),
            format!("t T ::= {encoded}"),
            "{builtin_name} {value:?}"
        );
    }

    assert_eq!(
        builtin("REAL")
            .encode("T", &Value::Real(f64::NAN))
            .unwrap_err()
            .to_string(),
        "T: Cannot encode floating point number NaN."
    );
    assert_eq!(
        builtin("BIT STRING")
            .encode("T", &Value::Bits(vec![0xff], 9))
            .unwrap_err()
            .to_string(),
        "T: Expected more bits of data."
    );
}

#[test]
fn certificates() {
    let der = compiled("rfc5280.asni", Codec::Der);
    let gser = compiled("rfc5280.asni", Codec::Gser);
    let data = fs::read(root().join("tests/fixtures/x509/serial_01.der")).unwrap();
    let certificate = der.decode("Certificate", &data).unwrap();
    let encoded = String::from_utf8(gser.encode("Certificate", &certificate).unwrap()).unwrap();

    assert!(
        encoded.starts_with("certificate Certificate ::= { tbsCertificate { "),
        "{encoded}"
    );
    assert!(encoded.contains(", serialNumber 1, "), "{encoded}");
    assert!(encoded.contains("algorithm 1.2.840."), "{encoded}");
    assert!(encoded.contains("Time : \""), "{encoded}");
    assert!(encoded.contains("}, signature '"), "{encoded}");
    assert!(encoded.ends_with("'B }"), "{encoded}");

    let indented = String::from_utf8(
        gser.encode_indented("Certificate", &certificate, 4)
            .unwrap(),
    )
    .unwrap();

    assert!(
        indented.starts_with("certificate Certificate ::= {\n    tbsCertificate {\n        "),
        "{indented}"
    );
    assert!(indented.ends_with("'B\n}"), "{indented}");
}

/// Every value of the vectors is encoded with its specification loaded
/// for GSER, or refused with an error, and nothing panics.
#[test]
fn vectors() {
    let mut text = String::new();

    flate2::read::GzDecoder::new(
        fs::File::open(common::folder().join("vectors.jsonl.gz")).unwrap(),
    )
    .read_to_string(&mut text)
    .unwrap();

    let mut specs = std::collections::HashMap::new();
    let mut encoded = 0;

    for line in text.lines() {
        let vector: Json = serde_json::from_str(line).unwrap();

        if vector["operation"] != "encode" || !vector["error"].is_null() {
            continue;
        }

        let (spec, _, numeric_enums, lenient) = common::spec_key(&vector);
        let key = (spec, "gser".to_string(), numeric_enums, lenient);
        let Ok(spec) = specs
            .entry(key.clone())
            .or_insert_with(|| common::load(&key))
        else {
            continue;
        };
        let name = vector["type"].as_str().unwrap();

        if let Ok(bytes) = spec.encode(name, &common::to_value(&vector["value"])) {
            let text = String::from_utf8(bytes).unwrap();

            assert!(text.contains(&format!(" {name} ::= ")), "{text}");
            encoded += 1;
        }
    }

    assert!(encoded > 1000, "{encoded}");
}
