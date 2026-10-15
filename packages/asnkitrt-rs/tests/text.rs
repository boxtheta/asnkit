// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! JER and XER, written by hand and read by hand, and what they are
//! given by someone who means harm.

#![cfg(all(feature = "jer", feature = "xer"))]

use std::fs;
use std::path::PathBuf;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

fn compiled(name: &str, codec: Codec) -> Specification {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/compiled")
        .join(name);

    Specification::from_asnc(
        &fs::read(path).unwrap(),
        &Checks::default(),
        Options::new(codec),
    )
    .unwrap()
}

fn question(id: i128, text: &str) -> Value {
    Value::Map(vec![
        ("id".into(), Value::Integer(id)),
        ("question".into(), Value::text(text)),
    ])
}

#[test]
fn json_is_written_as_python_writes_it() {
    let foo = compiled("foo.asnc", Codec::Jer);
    let value = question(1, "Is 1+1=3?");
    let encoded = foo.encode("Question", &value).unwrap();

    assert_eq!(encoded, br#"{"id":1,"question":"Is 1+1=3?"}"#);
    assert_eq!(foo.decode("Question", &encoded).unwrap(), value);

    // Only ASCII is written, and the rest as escapes.
    let encoded = foo.encode("Question", &question(2, "é\"\\😀")).unwrap();

    assert_eq!(encoded, br#"{"id":2,"question":"\u00e9\"\\\ud83d\ude00"}"#);
    assert_eq!(
        foo.decode("Question", &encoded).unwrap(),
        question(2, "é\"\\😀")
    );

    // Space and the order of names do not matter.
    assert_eq!(
        foo.decode("Question", br#" { "question" : "q" , "id" : 3 } "#)
            .unwrap(),
        question(3, "q")
    );
}

#[test]
fn xml_is_written_as_python_writes_it() {
    let foo = compiled("foo.asnc", Codec::Xer);
    let value = question(1, "a<b&c>é😀");
    let encoded = foo.encode("Question", &value).unwrap();

    assert_eq!(
        encoded,
        b"<Question><id>1</id><question>a&lt;b&amp;c&gt;&#233;&#128512;</question></Question>"
    );
    assert_eq!(foo.decode("Question", &encoded).unwrap(), value);

    // What is not in the way it is written: space, a comment, another
    // order, the other ways to write a character, an XML declaration.
    let other = r#"<?xml version="1.0" encoding="UTF-8"?>
<!-- a question -->
<Question xmlns="urn:x">
  <question>a&#60;b&#x26;c&gt;<![CDATA[é]]></question>
  <id>1</id>
</Question>"#;

    assert_eq!(
        foo.decode("Question", other.as_bytes()).unwrap(),
        question(1, "a<b&c>é")
    );
}

#[test]
fn integers_of_any_size() {
    let big = "-123456789012345678901234567890123456789012345678901234567890";

    for (codec, written) in [
        (Codec::Jer, format!("{{\"id\":{big},\"question\":\"\"}}")),
        (
            Codec::Xer,
            format!("<Question><id>{big}</id><question /></Question>"),
        ),
    ] {
        let foo = compiled("foo.asnc", codec);
        let value = foo.decode("Question", written.as_bytes()).unwrap();

        assert!(
            matches!(value.get("id"), Some(Value::BigInteger(_))),
            "{codec:?}"
        );
        assert_eq!(
            foo.encode("Question", &value).unwrap(),
            written.as_bytes(),
            "{codec:?}"
        );
    }

    // Digits without end are refused, as Python does.
    let digits = "9".repeat(5000);
    let jer = compiled("foo.asnc", Codec::Jer);
    let xer = compiled("foo.asnc", Codec::Xer);

    assert!(
        jer.decode(
            "Question",
            format!("{{\"id\":{digits},\"question\":\"\"}}").as_bytes()
        )
        .is_err()
    );
    assert!(
        xer.decode(
            "Question",
            format!("<Question><id>{digits}</id></Question>").as_bytes()
        )
        .is_err()
    );
}

#[test]
fn json_that_is_not_json_is_an_error() {
    let foo = compiled("foo.asnc", Codec::Jer);

    for bad in [
        &b""[..],
        b"{",
        b"{\"id\":1,}",
        b"{\"id\":01,\"question\":\"\"}",
        b"{\"id\":1,\"question\":\"\"} {}",
        b"{\"id\":1.5,\"question\":\"\"}",
        b"{\"id\":\"1\",\"question\":\"\"}",
        b"{\"id\":1}",
        b"[]",
        b"\xff\xfe",
        b"{\"id\":1,\"question\":\"\xff\"}",
        b"{\"id\":1,\"question\":\"\\ud800\"}",
    ] {
        assert!(foo.decode("Question", bad).is_err(), "{bad:?}");
    }
}

#[test]
fn xml_that_is_dangerous_is_an_error() {
    let foo = compiled("foo.asnc", Codec::Xer);

    // Entities that are defined by the document, or are somewhere else.
    let laughs = r#"<?xml version="1.0"?>
<!DOCTYPE Question [
  <!ENTITY a "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa">
  <!ENTITY b "&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;">
  <!ENTITY c "&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;">
]>
<Question><id>1</id><question>&c;</question></Question>"#;
    let xxe = r#"<?xml version="1.0"?>
<!DOCTYPE Question [<!ENTITY x SYSTEM "file:///etc/passwd">]>
<Question><id>1</id><question>&x;</question></Question>"#;

    for bad in [
        laughs,
        xxe,
        "<!DOCTYPE Question><Question><id>1</id></Question>",
        "<Question><id>1</id><question>&undefined;</question></Question>",
        "<Question><id>1</id><question>&#0;</question></Question>",
        "<Question><id>1</id><question>&#xD800;</question></Question>",
        "<Question><id>1</id></Quesion>",
        "<Question><id>1</id></Question><Question/>",
        "<Question id=1><id>1</id></Question>",
        "<Question><id>1</id><question>\u{1}</question></Question>",
        "",
        "<",
    ] {
        assert!(foo.decode("Question", bad.as_bytes()).is_err(), "{bad:?}");
    }

    // Nested without end, and many elements.
    assert!(
        foo.decode("Question", "<a>".repeat(1_000_000).as_bytes())
            .is_err()
    );
    assert!(
        foo.decode(
            "Question",
            format!("<Question>{}</Question>", "<x/>".repeat(5_000_000)).as_bytes()
        )
        .is_err()
    );
}

#[test]
fn a_real_is_written_as_python_writes_it() {
    let real = |codec| {
        let nodes = {
            use asnkitrt::cbor::Cbor;

            // R ::= REAL
            Cbor::Array(vec![
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
                                Cbor::Text("R".into()),
                                Cbor::Array(vec![Cbor::Int(10), Cbor::Text("REAL".into())]),
                            ]),
                        ]),
                    ]),
                ]),
            ])
        };

        Specification::from_nodes(&nodes, Options::new(codec)).unwrap()
    };
    let jer = real(Codec::Jer);
    let xer = real(Codec::Xer);

    for (value, json, xml) in [
        (1.5, "1.5", "<R>1.5E0</R>"),
        (-2.0, "-2.0", "<R>-2.0E0</R>"),
        // XER writes the digits of the float, so it reads back the same.
        (12345.0, "12345.0", "<R>1.2345E4</R>"),
        (1e22, "1e+22", "<R>1.0E22</R>"),
        (0.00001, "1e-05", "<R>1.0E-5</R>"),
    ] {
        assert_eq!(
            jer.encode("R", &Value::Real(value)).unwrap(),
            json.as_bytes()
        );
        assert_eq!(
            xer.encode("R", &Value::Real(value)).unwrap(),
            xml.as_bytes()
        );
        assert_eq!(
            jer.decode("R", json.as_bytes()).unwrap(),
            Value::Real(value)
        );
    }

    assert_eq!(
        jer.encode("R", &Value::Real(f64::INFINITY)).unwrap(),
        b"\"INF\""
    );
    assert!(matches!(
        jer.decode("R", b"\"NaN\"").unwrap(),
        Value::Real(value) if value.is_nan()
    ));

    // The special values are empty elements (X.680 21.9).
    for (value, xml) in [
        (f64::INFINITY, "<R><PLUS-INFINITY /></R>"),
        (f64::NEG_INFINITY, "<R><MINUS-INFINITY /></R>"),
    ] {
        assert_eq!(
            xer.encode("R", &Value::Real(value)).unwrap(),
            xml.as_bytes()
        );
        assert_eq!(xer.decode("R", xml.as_bytes()).unwrap(), Value::Real(value));
    }

    assert_eq!(
        xer.encode("R", &Value::Real(f64::NAN)).unwrap(),
        b"<R><NOT-A-NUMBER /></R>"
    );
    assert!(matches!(
        xer.decode("R", b"<R><NOT-A-NUMBER/></R>").unwrap(),
        Value::Real(value) if value.is_nan()
    ));

    // Only a realnumber of X.680 is a real, not what Rust reads as one.
    for xml in [
        "<R>inf</R>",
        "<R>NaN</R>",
        "<R>+1</R>",
        "<R>.5</R>",
        "<R>1<PLUS-INFINITY/></R>",
    ] {
        assert!(xer.decode("R", xml.as_bytes()).is_err(), "{xml}");
    }
}
