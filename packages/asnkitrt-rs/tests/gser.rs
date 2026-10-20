// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! GSER encoding and decoding.

#![cfg(feature = "gser")]

mod common;

use std::fs;
use std::io::Read;
use std::path::PathBuf;

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

fn question(id: i128, text: &str) -> Value<'static> {
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
fn assignments_and_bare_values_are_decoded() {
    let foo = compiled("foo.asni", Codec::Gser);
    let value = question(1, "say \"hi\" Ă");
    for bytes in [
        foo.encode("Question", &value).unwrap(),
        foo.encode_indented("Question", &value, 2).unwrap(),
    ] {
        assert_eq!(foo.decode("Question", &bytes).unwrap(), value);
        assert_eq!(foo.decode_in("Foo", "Question", &bytes).unwrap(), value);
    }
    assert_eq!(
        foo.decode("Question", br#"{ id 1, question "q" }"#)
            .unwrap(),
        question(1, "q")
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
            Value::Bytes(vec![0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef].into()),
            "'0123456789ABCDEF'H",
        ),
        ("OCTET STRING", Value::Bytes(vec![].into()), "''H"),
        ("BIT STRING", Value::Bits(vec![0xf0].into(), 5), "'11110'B"),
        (
            "BIT STRING",
            Value::Bits(vec![0x12, 0x34].into(), 16),
            "'0001001000110100'B",
        ),
        ("BIT STRING", Value::Bits(vec![].into(), 0), "''B"),
        (
            "OBJECT IDENTIFIER",
            Value::ObjectIdentifier("1.2.840.113549".parse().unwrap()),
            "1.2.840.113549",
        ),
        ("UTF8String", Value::text("\u{102}"), "\"\u{102}\""),
        ("NumericString", Value::text("01 23"), "\"01 23\""),
    ] {
        let spec = builtin(builtin_name);
        let bytes = spec.encode("T", &value).unwrap();
        let decoded = spec.decode("T", &bytes).unwrap();
        assert_eq!(spec.encode("T", &decoded).unwrap(), bytes);

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
            .encode("T", &Value::Bits(vec![0xff].into(), 9))
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
    let decoded = gser.decode("Certificate", encoded.as_bytes()).unwrap();
    assert_eq!(der.encode("Certificate", &decoded).unwrap(), data);

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

        if let Ok(bytes) = spec.encode(name, &common::value_of(spec, name, &vector["value"])) {
            let decoded = spec.decode(name, &bytes).unwrap_or_else(|error| {
                panic!("{name}: {error}; {}", String::from_utf8_lossy(&bytes))
            });
            assert!(spec.encode(name, &decoded).is_ok());
            let text = String::from_utf8(bytes).unwrap();

            assert!(text.contains(&format!(" {name} ::= ")), "{text}");
            encoded += 1;
        }
    }

    assert!(encoded > 1000, "{encoded}");
}

#[test]
fn invalid_input_has_offsets_and_locations() {
    let foo = compiled("foo.asni", Codec::Gser);
    for text in [
        "",
        "{",
        "{}",
        "{ id 1 }",
        "{ id 1, question \"unterminated }",
        "{ id 1, id 2, question \"q\" }",
        "{ id 1, question \"q\", }",
        "{ unknown 1, id 1, question \"q\" }",
        "{ question \"q\", id 1 }",
        "{ id 1, question \"q\" } garbage",
        "x Wrong ::= { id 1, question \"q\" }",
    ] {
        assert!(
            matches!(
                foo.decode("Question", text.as_bytes()),
                Err(asnkitrt::Error::Decode(..))
            ),
            "{text}"
        );
    }
    let error = foo
        .decode("Question", b"{ id 1, question 42 }")
        .unwrap_err();
    assert_eq!(error.location().unwrap().to_string(), "Question.question");
    assert!(matches!(error, asnkitrt::Error::Decode(_, 17, _)));
    assert!(matches!(
        builtin("UTF8String").decode("T", &[b'"', 0xff, b'"']),
        Err(asnkitrt::Error::Decode(_, 1, _))
    ));
    for (name, text) in [
        ("BOOLEAN", "TRUEjunk"),
        ("NULL", "NULLx"),
        ("INTEGER", "1.2"),
        ("INTEGER", "+1"),
        ("INTEGER", "01"),
        ("REAL", "NaN"),
        ("REAL", "1.2"),
        ("REAL", "1e2"),
        ("REAL", "1E+2"),
        ("REAL", "1E9999"),
        ("BIT STRING", "'2'B"),
        ("BIT STRING", "'01'"),
        ("OCTET STRING", "'00'B"),
        ("OCTET STRING", "'GG'H"),
        ("OBJECT IDENTIFIER", "1..2"),
    ] {
        assert!(
            builtin(name).decode("T", text.as_bytes()).is_err(),
            "{name}: {text}"
        );
    }
}

#[test]
fn literals_and_schema_names() {
    use asnkitrt::{gser, types::Kind};
    assert_eq!(
        builtin("OCTET STRING").decode("T", b"'ABC'H").unwrap(),
        Value::Bytes(vec![0xab, 0xc0].into())
    );
    assert_eq!(
        builtin("BIT STRING").decode("T", b"'ABC'H").unwrap(),
        Value::Bits(vec![0xab, 0xc0].into(), 12)
    );
    let spec = builtin("INTEGER");
    let mut types = spec.types().clone();
    let id = types.find_unique("T").unwrap();
    types.nodes[id].kind = Kind::Integer(vec![("negative".into(), -3)]);
    assert_eq!(
        gser::decode(&types, id, b"negative").unwrap(),
        Value::Integer(-3)
    );
    types.nodes[id].kind = Kind::Enumerated {
        root: vec![("red".into(), 0)],
        additions: Some(vec![("blue".into(), 1)]),
    };
    assert_eq!(
        gser::decode(&types, id, b"blue").unwrap(),
        Value::text("blue")
    );
    types.numeric_enums = true;
    assert_eq!(
        gser::decode(&types, id, b"blue").unwrap(),
        Value::Integer(1)
    );
    assert!(gser::decode(&types, id, b"green").is_err());
    types.nodes[id].kind = Kind::BitString(vec![("first".into(), 0), ("last".into(), 9)]);
    assert_eq!(
        gser::decode(&types, id, b"{ last, first }").unwrap(),
        Value::Bits(vec![0x80, 0x40].into(), 10)
    );
    assert!(gser::decode(&types, id, b"{ first, first }").is_err());
    assert!(gser::decode(&types, id, b"{ unknown }").is_err());
    types.nodes[id].kind = Kind::SequenceOf(id);
    let nested = format!("{}{}", "{".repeat(140), "}".repeat(140));
    assert!(gser::decode(&types, id, nested.as_bytes()).is_err());
}

#[test]
fn truncated_values_never_panic() {
    let foo = compiled("foo.asni", Codec::Gser);
    let data = foo.encode("Question", &question(1, "a\"b Ă")).unwrap();
    for end in 0..data.len() {
        assert!(foo.decode("Question", &data[..end]).is_err(), "{end}");
    }
}

#[test]
fn real_component_notation() {
    let spec = builtin("REAL");
    for (text, expected) in [
        ("{ mantissa 5, base 2, exponent -3 }", 0.625),
        ("{ mantissa -5, base 2, exponent -1 }", -2.5),
        ("{ mantissa 1, base 2, exponent -1074 }", f64::from_bits(1)),
        ("{ mantissa 1, base 2, exponent -1075 }", 0.0),
        ("{ mantissa 0, base 2, exponent 0 }", 0.0),
        ("{ mantissa 625, base 10, exponent -3 }", 0.625),
    ] {
        assert_eq!(
            spec.decode("T", text.as_bytes()).unwrap(),
            Value::Real(expected)
        );
    }
    for text in [
        "{ mantissa 1, base 3, exponent 1 }",
        "{ mantissa 1, base 2, exponent 1024 }",
    ] {
        assert!(spec.decode("T", text.as_bytes()).is_err());
    }
}

#[test]
fn omitted_optional_and_default_members() {
    use asnkitrt::{gser, types::Kind};
    let spec = compiled("foo.asni", Codec::Gser);
    let mut types = spec.types().clone();
    let id = types.find_unique("Question").unwrap();
    let Kind::Sequence(mut members) = types.kind(id).clone() else {
        panic!()
    };
    members.root[0].default = Some(Value::Integer(7));
    members.root[1].optional = true;
    let concrete = types.nodes[id].concrete;
    types.nodes[concrete].kind = Kind::Sequence(members);
    assert_eq!(
        gser::decode(&types, id, b"{}").unwrap(),
        Value::Map(vec![("id".into(), Value::Integer(7))])
    );
    assert_eq!(
        gser::decode(&types, id, br#"{ question "q" }"#).unwrap(),
        question(7, "q")
    );
}

#[test]
fn omitted_extension_group_with_trailing_root_members() {
    let data = fs::read(common::folder().join("specs/2991335b3c2e374d.asni")).unwrap();
    let spec =
        Specification::from_asni(&data, &Checks::default(), Options::new(Codec::Gser)).unwrap();
    assert_eq!(
        spec.decode("F", b"{ a TRUE, c FALSE }").unwrap(),
        Value::Map(vec![
            ("a".into(), Value::Boolean(true)),
            ("c".into(), Value::Boolean(false)),
        ])
    );
}

#[test]
fn extension_groups_are_required_only_when_present() {
    use asnkitrt::{
        gser,
        types::{Addition, Kind},
    };

    let data = fs::read(common::folder().join("specs/2991335b3c2e374d.asni")).unwrap();
    let spec =
        Specification::from_asni(&data, &Checks::default(), Options::new(Codec::Gser)).unwrap();
    let mut types = spec.types().clone();
    let id = types.find_unique("F").unwrap();
    let concrete = types.nodes[id].concrete;
    let Kind::Sequence(mut members) = types.kind(id).clone() else {
        panic!()
    };
    let mut b = members.root[0].clone();
    b.name = "b".into();
    let mut d = b.clone();
    d.name = "d".into();
    let mut e = b.clone();
    e.name = "e".into();
    e.optional = true;
    let mut f = b.clone();
    f.name = "f".into();
    f.default = Some(Value::Boolean(false));
    let mut g = b.clone();
    g.name = "g".into();
    members.additions = Some(vec![Addition::Group(vec![b, d, e, f]), Addition::Member(g)]);

    for kind in [Kind::Sequence(members.clone()), Kind::Set(members.clone())] {
        types.nodes[concrete].kind = kind;
        for text in [
            "{ a TRUE, c FALSE }",
            "{ a TRUE, g TRUE, c FALSE }",
            "{ a TRUE, b TRUE, d FALSE, c FALSE }",
            "{ a TRUE, b TRUE, d FALSE, e TRUE, f TRUE, g TRUE, c FALSE }",
        ] {
            let value = gser::decode(&types, id, text.as_bytes()).unwrap();
            let Value::Map(map) = value else { panic!() };
            assert!(map.iter().any(|(name, _)| name == "f"));
        }
        for (text, missing) in [
            ("{ a TRUE, b TRUE, c FALSE }", "d"),
            ("{ a TRUE, d TRUE, c FALSE }", "b"),
            ("{ a TRUE, e TRUE, c FALSE }", "b"),
            ("{ a TRUE, f TRUE, c FALSE }", "b"),
            ("{ c FALSE }", "a"),
            ("{ a TRUE }", "c"),
        ] {
            let error = gser::decode(&types, id, text.as_bytes()).unwrap_err();
            assert_eq!(error.location().unwrap().to_string(), missing, "{text}");
        }

        // Also exercise missing additions at the end of a value.
        let mut trailing = members.clone();
        trailing.root_after_additions = 0;
        types.nodes[concrete].kind = Kind::Sequence(trailing);
        assert!(gser::decode(&types, id, b"{ a TRUE, c FALSE }").is_ok());
        assert!(gser::decode(&types, id, b"{ a TRUE, c FALSE, b TRUE }").is_err());
    }
}
