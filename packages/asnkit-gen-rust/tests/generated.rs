// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The Rust code asnkit-gen-rust makes from compiled specifications:
//! the files in `examples/generated` and `tests/generated` are what it
//! makes now, and they are built here and used.
//!
//! To make them again after a change of the plugin:
//!
//!   UPDATE_GENERATED=1 cargo test -p asnkit-gen-rust --test generated

use std::fs;
use std::path::PathBuf;

use asnkit_gen_rust::{Integer, Options, generate};
use asnkitrt::types::Codec;
use shapes::Asn1Type;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../../examples/generated/foo.rs"]
mod foo;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/rfc5280.rs"]
mod rfc5280;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes.rs"]
mod shapes;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes_not_embedded.rs"]
mod shapes_not_embedded;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_current(asni: &str, generated: &str, options: Options) {
    let data = fs::read(root().join(asni)).unwrap();
    let (text, _) = generate(&data, options).unwrap();
    let path = root().join(generated);

    if std::env::var_os("UPDATE_GENERATED").is_some() {
        fs::write(&path, &text).unwrap();
    }

    assert!(
        fs::read_to_string(&path).unwrap() == text,
        "{generated} is not what the plugin makes: run the tests with UPDATE_GENERATED=1."
    );
}

#[test]
fn the_generated_files_are_current() {
    is_current(
        "../../examples/compiled/foo.asni",
        "../../examples/generated/foo.rs",
        Options::default(),
    );
    is_current(
        "../../examples/compiled/rfc5280.asni",
        "tests/generated/rfc5280.rs",
        Options {
            integer: Integer::Big,
            ..Options::default()
        },
    );
    is_current(
        "tests/generated/shapes.asni",
        "tests/generated/shapes.rs",
        Options::default(),
    );
    is_current(
        "tests/generated/shapes.asni",
        "tests/generated/shapes_no_std.rs",
        Options {
            no_std: true,
            ..Options::default()
        },
    );
    is_current(
        "tests/generated/shapes.asni",
        "tests/generated/shapes_not_embedded.rs",
        Options {
            embed: false,
            ..Options::default()
        },
    );
}

/// Without the specification in it, a module is used with one that is
/// loaded from a file, and encodes and decodes as one with it in does.
#[test]
fn a_module_without_the_specification_uses_one_that_is_loaded() {
    use shapes_not_embedded::Asn1Type as _;

    let text = fs::read_to_string(root().join("tests/generated/shapes_not_embedded.rs")).unwrap();

    assert!(!text.contains("const ASNI") && !text.contains("pub fn load"));

    let data = fs::read(root().join("tests/generated/shapes.asni")).unwrap();
    let value = shapes_not_embedded::Keywords {
        r#type: 255,
        self_: true,
        r#match: Some("m".into()),
        foo_bar: -100,
        foo_bar_2: 65535,
        crate_: (),
    };

    for codec in CODECS {
        let specification = asnkitrt::runtime::Specification::from_asni(
            &data,
            &asnkitrt::container::Checks::default(),
            asnkitrt::runtime::Options::new(codec),
        )
        .unwrap();
        let encoded = value.encode(&specification).unwrap();
        let embedded = shapes::Keywords {
            r#type: 255,
            self_: true,
            r#match: Some("m".into()),
            foo_bar: -100,
            foo_bar_2: 65535,
            crate_: (),
        };

        assert_eq!(
            encoded,
            embedded.encode(&shapes::load(codec).unwrap()).unwrap()
        );
        assert_eq!(
            shapes_not_embedded::Keywords::decode(&specification, &encoded).unwrap(),
            value
        );
    }
}

#[test]
fn foo_is_encoded_and_decoded() {
    use foo::Asn1Type;

    let question = foo::Question {
        id: 1,
        question: "Is 1+1=3?".into(),
    };

    for (codec, encoded) in [
        (Codec::Ber, &b"0\x0e\x02\x01\x01\x16\x09Is 1+1=3?"[..]),
        (Codec::Der, &b"0\x0e\x02\x01\x01\x16\x09Is 1+1=3?"[..]),
    ] {
        let specification = foo::load(codec).unwrap();

        assert_eq!(question.encode(&specification).unwrap(), encoded);
        assert_eq!(
            foo::Question::decode(&specification, encoded).unwrap(),
            question
        );
    }

    // The others come back as what they were.
    for codec in [Codec::Oer, Codec::Per, Codec::Uper] {
        let specification = foo::load(codec).unwrap();
        let encoded = question.encode(&specification).unwrap();

        assert_eq!(
            foo::Question::decode(&specification, &encoded).unwrap(),
            question
        );
    }

    // It is the Answer that it is not.
    let specification = foo::load(Codec::Ber).unwrap();

    assert!(
        foo::Answer::decode(&specification, &question.encode(&specification).unwrap()).is_err()
    );
    assert!(question.check_constraints(&specification).is_ok());
}

#[test]
fn the_certificates_of_openssl_come_back_as_they_were() {
    use rfc5280::Asn1Type;

    let specification = rfc5280::load(Codec::Der).unwrap();
    let folder = root().join("../asnkitrt-rs/tests/fixtures/x509");
    let mut count = 0;

    for entry in fs::read_dir(folder).unwrap() {
        let path = entry.unwrap().path();

        if path.extension().is_none_or(|extension| extension != "der") {
            continue;
        }

        let data = fs::read(&path).unwrap();
        let certificate = rfc5280::Certificate::decode(&specification, &data)
            .unwrap_or_else(|error| panic!("{}: {error}", path.display()));

        assert_eq!(
            certificate.encode(&specification).unwrap(),
            data,
            "{}",
            path.display()
        );
        count += 1;
    }

    assert!(count >= 30);
}

const CODECS: [Codec; 5] = [Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper];

/// `value` comes back as it was with each codec, and the
/// constraints hold.
fn comes_back<T: shapes::Asn1Type + PartialEq + std::fmt::Debug>(value: &T) {
    for codec in CODECS {
        let specification = shapes::load(codec).unwrap();
        let encoded = value
            .encode(&specification)
            .unwrap_or_else(|error| panic!("{codec:?}: {error}"));

        assert_eq!(
            &T::decode(&specification, &encoded).unwrap(),
            value,
            "{codec:?}"
        );
        value.check_constraints(&specification).unwrap();
    }
}

#[test]
fn names_are_those_of_rust() {
    comes_back(&shapes::Keywords {
        r#type: 255,
        self_: true,
        r#match: Some("m".into()),
        foo_bar: -100,
        foo_bar_2: 65535,
        crate_: (),
    });
    comes_back(&shapes::Keywords {
        r#type: 0,
        self_: false,
        r#match: None,
        foo_bar: 100,
        foo_bar_2: 0,
        crate_: (),
    });
}

#[test]
fn integers_are_those_of_their_bounds() {
    comes_back(&shapes::Sizes {
        a: 1,
        b: 70000,
        c: 5_000_000_000,
        d: -1,
        e: -30000,
        f: -3_000_000_000,
        g: i64::MIN,
        h: i64::MAX,
    });
}

#[test]
fn built_in_types_come_back() {
    use asnkitrt::serde_value::{BitString, OctetString};
    use asnkitrt::value::{Date, DateTime, Time};

    let date = Date {
        year: 2024,
        month: 2,
        day: 29,
    };
    let time = Time {
        hour: 23,
        minute: 59,
        second: 58,
        microsecond: 0,
        offset: None,
    };

    comes_back(&shapes::Builtins {
        flag: true,
        real: 1.5,
        nothing: (),
        octets: OctetString(vec![0, 1, 255]),
        bits: BitString {
            data: vec![0b1010_0000],
            bits: 4,
        },
        oid: "1.2.840.113549".parse().unwrap(),
        text: "Grüße".into(),
        printable: "ABC 123".into(),
        utc: DateTime { date, time },
        // In UTC, the one time DER and PER have.
        general: DateTime {
            date,
            time: Time {
                offset: Some(0),
                ..time
            },
        },
        day: date,
        clock: Time {
            offset: None,
            ..time
        },
        both: DateTime {
            date,
            time: Time {
                offset: None,
                ..time
            },
        },
    });
}

#[test]
fn enumerations_and_choices_come_back() {
    use shapes::{Color, Name, Shape, ShapeSquare};

    for color in [Color::Red, Color::Green, Color::DarkBlue] {
        comes_back(&color);
    }

    comes_back(&Shape::Circle(3));
    comes_back(&Shape::Nothing(()));
    comes_back(&Shape::Octets(vec![1, 2].into()));
    comes_back(&Shape::Named(Name::from("n")));
    comes_back(&Shape::Square(ShapeSquare {
        side: 4,
        color: Some(Color::DarkBlue),
    }));
    comes_back(&Shape::Many(vec![
        Shape::Circle(1),
        Shape::Many(vec![Shape::Nothing(()), Shape::Many(vec![])]),
    ]));

    // The names of the ASN.1 are those on the wire.
    let ber = shapes::load(Codec::Ber).unwrap();

    assert_eq!(
        shapes::Shape::Circle(3).encode(&ber).unwrap(),
        [0x80, 0x01, 0x03]
    );
}

#[test]
fn a_type_that_holds_itself_is_on_the_heap() {
    use shapes::{Node, Tree};

    let leaf = |value| Box::new(Node::Leaf(value));
    let tree = Tree {
        value: 1,
        left: Some(Box::new(Tree {
            value: 2,
            left: None,
            right: Some(leaf(3)),
        })),
        right: Some(Box::new(Node::Tree(Box::new(Tree {
            value: 4,
            left: None,
            right: None,
        })))),
    };

    comes_back(&tree);
    comes_back(&Node::Tree(Box::new(tree)));
}

#[test]
fn what_is_optional_and_what_has_a_default() {
    use shapes::{Color, Defaults, Extensible};

    // A default that is left out is the default when it is decoded or
    // not, as the runtime has it, and it is the same as written.
    let none = Defaults {
        count: None,
        enabled: None,
        color: None,
        list: None,
        anything: None,
    };
    let ber = shapes::load(Codec::Ber).unwrap();
    let encoded = none.encode(&ber).unwrap();

    assert_eq!(encoded, [0x30, 0x00]);
    assert!(Defaults::decode(&ber, &encoded).is_ok());

    comes_back(&Defaults {
        count: Some(9),
        enabled: Some(false),
        color: Some(Color::DarkBlue),
        list: Some(vec![1, 2]),
        anything: Some(shapes::Any { x: 1 }),
    });

    comes_back(&Extensible {
        first: 1,
        second: None,
        third: None,
        fourth: None,
    });
    comes_back(&Extensible {
        first: 1,
        second: Some(2),
        third: Some(3),
        fourth: Some("four".into()),
    });
}

#[test]
fn types_that_are_not_classes_have_functions() {
    let specification = shapes::load(Codec::Der).unwrap();
    let encoded = shapes::encode_matrix(&vec![vec![1, 2], vec![]], &specification).unwrap();

    assert_eq!(
        shapes::decode_matrix(&specification, &encoded).unwrap(),
        vec![vec![1, 2], vec![]]
    );

    let name: shapes::Alias = "name".into();
    let encoded = shapes::encode_name(&name, &specification).unwrap();

    assert_eq!(shapes::decode_name(&specification, &encoded).unwrap(), name);
    assert_eq!(
        shapes::decode_name(&specification, &encoded).unwrap(),
        "name"
    );

    // Not what the constraints allow.
    assert!(
        shapes::Shape::Named(String::new())
            .check_constraints(&specification)
            .is_err()
    );
    assert!(
        shapes::Shape::Circle(3)
            .check_constraints(&specification)
            .is_ok()
    );
}

#[test]
fn types_inside_others_have_names_that_do_not_clash() {
    let outer = shapes::Outer {
        inner: shapes::OuterInner2 { value: 1 },
        list: vec![shapes::OuterListItem { value: 2 }],
        choice: shapes::OuterChoice::B(shapes::OuterChoiceB { value: 3 }),
    };
    let specification = shapes::load(Codec::Uper).unwrap();

    assert_eq!(
        shapes::Outer::decode(&specification, &outer.encode(&specification).unwrap()).unwrap(),
        outer
    );
}

#[test]
fn a_value_the_enumeration_does_not_know_is_an_error() {
    let ber = shapes::load(Codec::Ber).unwrap();

    // 5 is green, 9 is not a value of Color, which is extensible.
    assert_eq!(
        shapes::Color::decode(&ber, &[0x0a, 0x01, 0x05]).unwrap(),
        shapes::Color::Green
    );
    assert!(shapes::Color::decode(&ber, &[0x0a, 0x01, 0x09]).is_err());
}
