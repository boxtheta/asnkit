// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Rust types as values.

use std::fs;
use std::path::PathBuf;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::serde_value::{BitString, from_value, to_value};
use asnkitrt::types::Codec;
use asnkitrt::value::{ObjectIdentifier, Value};
use serde::{Deserialize, Serialize};

fn compiled(name: &str, codec: Codec) -> Specification {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/compiled")
        .join(name);

    Specification::from_asni(
        &fs::read(path).unwrap(),
        &Checks::default(),
        Options::new(codec),
    )
    .unwrap()
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct Question {
    id: u32,
    question: String,
}

#[test]
fn a_sequence_is_a_struct() {
    let foo = compiled("foo.asni", Codec::Ber);
    let question = Question {
        id: 1,
        question: "Is 1+1=3?".into(),
    };
    let encoded = foo
        .encode("Question", &to_value(&question).unwrap())
        .unwrap();

    assert_eq!(encoded, b"0\x0e\x02\x01\x01\x16\x09Is 1+1=3?");
    assert_eq!(
        from_value::<Question>(&foo.decode("Question", &encoded).unwrap()).unwrap(),
        question
    );

    // Not the type.
    assert!(from_value::<Question>(&Value::Boolean(true)).is_err());

    let missing = Value::Map(vec![("id".into(), Value::Integer(1))]);

    assert!(from_value::<Question>(&missing).is_err());
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct AlgorithmIdentifier {
    algorithm: ObjectIdentifier<'static>,
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        with = "optional_bytes"
    )]
    parameters: Option<Vec<u8>>,
}

mod optional_bytes {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S: Serializer>(
        value: &Option<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(bytes) => asnkitrt::serde_value::bytes::serialize(bytes, serializer),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Vec<u8>>, D::Error> {
        Option::<Vec<u8>>::deserialize(deserializer)
    }

    #[allow(dead_code)]
    fn unused<T: Serialize>(_: T) {}
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
struct SubjectPublicKeyInfo {
    algorithm: AlgorithmIdentifier,
    #[serde(rename = "subjectPublicKey")]
    subject_public_key: BitString,
}

#[test]
fn bit_strings_and_optional_members() {
    let der = compiled("rfc5280.asni", Codec::Der);
    let data = fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/x509/key_p256.der"),
    )
    .unwrap();
    let certificate = der.decode("Certificate", &data).unwrap();
    let tbs = certificate.get("tbsCertificate").unwrap();
    let key = tbs.get("subjectPublicKeyInfo").unwrap();
    let typed: SubjectPublicKeyInfo = from_value(key).unwrap();

    assert_eq!(typed.algorithm.algorithm.to_string(), "1.2.840.10045.2.1");
    assert!(typed.algorithm.parameters.is_some());
    assert_eq!(typed.subject_public_key.bits, 520);
    assert_eq!(to_value(&typed).unwrap(), *key);

    // No parameters at all.
    let plain = AlgorithmIdentifier {
        algorithm: "1.2.3".parse().unwrap(),
        parameters: None,
    };

    assert_eq!(
        to_value(&plain).unwrap(),
        Value::Map(vec![(
            "algorithm".into(),
            Value::ObjectIdentifier("1.2.3".parse().unwrap())
        )])
    );
    assert_eq!(
        from_value::<AlgorithmIdentifier>(&to_value(&plain).unwrap()).unwrap(),
        plain
    );
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Pick {
    #[serde(rename = "a")]
    A(i64),
    #[serde(rename = "b")]
    B { x: bool, y: Vec<String> },
    #[serde(rename = "c")]
    C(()),
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]
enum Colour {
    #[serde(rename = "red")]
    Red,
    #[serde(rename = "green")]
    Green,
}

#[test]
fn choices_and_enumerations() {
    for pick in [
        Pick::A(-5),
        Pick::B {
            x: true,
            y: vec!["p".into(), "q".into()],
        },
        Pick::C(()),
    ] {
        let value = to_value(&pick).unwrap();

        assert!(matches!(value, Value::Choice(..)), "{value:?}");
        assert_eq!(from_value::<Pick>(&value).unwrap(), pick);
    }

    assert_eq!(
        to_value(&Pick::A(7)).unwrap(),
        Value::Choice("a".into(), Box::new(Value::Integer(7)))
    );
    assert_eq!(
        to_value(&Colour::Green).unwrap(),
        Value::Text("green".into())
    );
    assert_eq!(
        from_value::<Colour>(&Value::Text("red".into())).unwrap(),
        Colour::Red
    );
    assert!(from_value::<Colour>(&Value::Text("blue".into())).is_err());
}

#[test]
fn numbers_and_lists() {
    assert_eq!(
        to_value(&(1u8, -2i64, 3.5f64)).unwrap(),
        Value::List(vec![
            Value::Integer(1),
            Value::Integer(-2),
            Value::Real(3.5),
        ])
    );
    assert_eq!(
        from_value::<Vec<u16>>(&Value::List(vec![Value::Integer(1), Value::Integer(2)])).unwrap(),
        vec![1, 2]
    );
    assert_eq!(
        from_value::<Vec<u8>>(&Value::Bytes(vec![1, 2, 3])).unwrap(),
        vec![1, 2, 3]
    );
    assert_eq!(
        from_value::<i128>(&Value::Integer(i128::MAX)).unwrap(),
        i128::MAX
    );
    assert!(from_value::<u8>(&Value::Integer(300)).is_err());
    assert!(to_value(&u128::MAX).is_err());
    assert_eq!(to_value(&()).unwrap(), Value::Null);
    assert_eq!(
        from_value::<Option<u8>>(&Value::Integer(4)).unwrap(),
        Some(4)
    );
}

#[test]
fn a_value_is_serde_too() {
    let value = Value::Map(vec![
        (
            "a".into(),
            Value::List(vec![Value::Integer(1), Value::Boolean(false)]),
        ),
        ("b".into(), Value::Text("t".into())),
    ]);
    let again: Value = from_value(&value).unwrap();

    assert_eq!(again, value);
}

#[test]
fn octet_strings_and_times_are_values() {
    use asnkitrt::serde_value::OctetString;
    use asnkitrt::value::{Date, DateTime, Time};

    let bytes = Value::Bytes(vec![1, 2, 3]);

    assert_eq!(to_value(&OctetString(vec![1, 2, 3])).unwrap(), bytes);
    assert_eq!(
        from_value::<OctetString>(&bytes).unwrap(),
        OctetString(vec![1, 2, 3])
    );

    // Inside a list and an option, where `with` can not be put.
    let many = vec![Some(OctetString(vec![9])), None];

    assert_eq!(
        to_value(&many).unwrap(),
        Value::List(vec![Value::Bytes(vec![9]), Value::Null])
    );

    let date = Date {
        year: 2024,
        month: 2,
        day: 29,
    };
    let time = Time {
        hour: 23,
        minute: 59,
        second: 58,
        microsecond: 7,
        offset: Some(-90),
    };
    let both = DateTime { date, time };

    assert_eq!(to_value(&date).unwrap(), Value::Date(date));
    assert_eq!(to_value(&time).unwrap(), Value::Time(time));
    assert_eq!(to_value(&both).unwrap(), Value::DateTime(both));
    assert_eq!(from_value::<Date>(&Value::Date(date)).unwrap(), date);
    assert_eq!(from_value::<Time>(&Value::Time(time)).unwrap(), time);
    assert_eq!(
        from_value::<DateTime>(&Value::DateTime(both)).unwrap(),
        both
    );

    let naive = Time {
        offset: None,
        ..time
    };

    assert_eq!(
        from_value::<Time>(&to_value(&naive).unwrap()).unwrap(),
        naive
    );
    assert!(from_value::<Date>(&Value::Integer(1)).is_err());
}

#[test]
fn big_integers_are_values() {
    use asnkitrt::serde_value::BigInt;

    // Small ones are integers, so that they are encoded as those.
    assert_eq!(to_value(&BigInt::from(-129)).unwrap(), Value::Integer(-129));
    assert_eq!(
        from_value::<BigInt>(&Value::Integer(-129))
            .unwrap()
            .to_i128(),
        Some(-129)
    );

    let big = BigInt(vec![0x01; 20]);

    assert_eq!(to_value(&big).unwrap(), Value::BigInteger(vec![0x01; 20]));
    assert_eq!(
        from_value::<BigInt>(&Value::BigInteger(vec![0x01; 20])).unwrap(),
        big
    );
    assert_eq!(big.to_i128(), None);
    assert!(from_value::<BigInt>(&Value::Boolean(true)).is_err());
}

#[test]
fn an_object_identifier_is_its_text_elsewhere() {
    let oid: ObjectIdentifier<'static> = "1.2.840.113549".parse().unwrap();

    // To the formats of serde, and to a String, as asnkit 1 had it.
    assert_eq!(serde_json::to_string(&oid).unwrap(), "\"1.2.840.113549\"");
    assert_eq!(
        serde_json::from_str::<ObjectIdentifier>("\"1.2.840.113549\"").unwrap(),
        oid
    );
    assert!(serde_json::from_str::<ObjectIdentifier>("\"1\"").is_err());
    assert_eq!(
        from_value::<String>(&Value::ObjectIdentifier(oid.clone())).unwrap(),
        "1.2.840.113549"
    );
    assert_eq!(
        to_value(&oid).unwrap(),
        Value::ObjectIdentifier(oid.clone())
    );
    assert_eq!(
        from_value::<ObjectIdentifier>(&Value::ObjectIdentifier(oid.clone())).unwrap(),
        oid
    );
}
