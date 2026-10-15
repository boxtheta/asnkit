// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What the nodes of a compiled specification mean, as
//! `docs/file_formats.md` tells, for what the vectors do not have.

use asnkitrt::cbor::Cbor;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

fn int(value: i128) -> Cbor {
    Cbor::Int(value)
}

fn text(value: &str) -> Cbor {
    Cbor::Text(value.into())
}

fn node(kind: i128, fields: Vec<Cbor>) -> Cbor {
    let mut all = vec![int(kind)];

    all.extend(fields);

    Cbor::Array(all)
}

fn list(items: Vec<Cbor>) -> Cbor {
    node(0, items)
}

fn assignment(name: &str, ty: Cbor) -> Cbor {
    node(3, vec![text(name), ty])
}

fn member(name: &str, ty: Cbor) -> Cbor {
    node(4, vec![text(name), ty])
}

fn sequence(members: Vec<Cbor>) -> Cbor {
    node(15, vec![list(members)])
}

fn reference(name: &str) -> Cbor {
    node(11, vec![text(name)])
}

/// A specification of one module, `M`.
fn module(types: Vec<Cbor>, object_classes: Vec<Cbor>) -> Cbor {
    node(
        1,
        vec![list(vec![node(
            2,
            vec![text("M"), list(types), list(object_classes)],
        )])],
    )
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn map(members: Vec<(&str, Value)>) -> Value {
    Value::Map(
        members
            .into_iter()
            .map(|(name, value)| (name.into(), value))
            .collect(),
    )
}

#[test]
fn a_field_of_an_object_class_is_the_type_of_the_field() {
    // CLS ::= CLASS { &id INTEGER (0..255), &rec SEQUENCE { a BOOLEAN } }
    // S ::= SEQUENCE { id CLS.&id }
    // R ::= SEQUENCE { rec CLS.&rec }
    let range = node(6, vec![int(0), int(255)]);
    let integer = node(
        12,
        vec![Cbor::Null, Cbor::Null, Cbor::Null, list(vec![range])],
    );
    let boolean = node(10, vec![text("BOOLEAN")]);
    let class = node(
        39,
        vec![
            text("CLS"),
            list(vec![
                node(40, vec![text("&id"), integer]),
                node(40, vec![text("&rec"), sequence(vec![member("a", boolean)])]),
            ]),
        ],
    );
    let nodes = module(
        vec![
            assignment("S", sequence(vec![member("id", reference("CLS.&id"))])),
            assignment("R", sequence(vec![member("rec", reference("CLS.&rec"))])),
        ],
        vec![class],
    );
    let spec = Specification::from_nodes(&nodes, Options::new(Codec::Uper)).unwrap();

    // The range of the field is that of the member: 8 bits.
    let s = map(vec![("id", Value::Integer(5))]);
    assert_eq!(hex(&spec.encode("S", &s).unwrap()), "05");

    // A field with types in it, which are lowered before it.
    let r = map(vec![("rec", map(vec![("a", Value::Boolean(true))]))]);
    let encoded = spec.encode("R", &r).unwrap();
    assert_eq!(hex(&encoded), "80");
    assert_eq!(spec.decode("R", &encoded).unwrap(), r);
}

#[test]
fn a_string_outside_an_extensible_size() {
    // E ::= IA5String (SIZE (1..2, ...))
    // M ::= IA5String (SIZE (1..MAX, ...))
    let string = |upper: Cbor| {
        node(
            10,
            vec![
                text("IA5String"),
                Cbor::Null,
                list(vec![node(6, vec![int(1), upper]), node(8, vec![])]),
            ],
        )
    };
    let nodes = module(
        vec![
            assignment("E", string(int(2))),
            assignment("M", string(node(21, vec![]))),
        ],
        vec![],
    );

    // Outside the root, as if there was no size constraint (X.691 30.4).
    for (codec, cases) in [
        (
            Codec::Per,
            [
                ("E", "ab", "406162"),
                ("E", "abc", "8003616263"),
                ("M", "abc", "0003616263"),
                ("M", "", "8000"),
            ],
        ),
        (
            Codec::Uper,
            [
                ("E", "ab", "70e2"),
                ("E", "abc", "81e1c58c"),
                ("M", "abc", "01e1c58c"),
                ("M", "", "8000"),
            ],
        ),
    ] {
        let spec = Specification::from_nodes(&nodes, Options::new(codec)).unwrap();

        for (name, value, expected) in cases {
            let value = Value::Text(value.into());
            let encoded = spec.encode(name, &value).unwrap();

            assert_eq!(hex(&encoded), expected, "{codec:?} {name}");
            assert_eq!(spec.decode(name, &encoded).unwrap(), value);
        }
    }
}

#[test]
fn constraints_of_a_reference_take_the_place_of_those_it_refers_to() {
    // D ::= IA5String (SIZE (1..4) ^ FROM ("AB"))
    // R ::= D, with a size of its own and an extension marker alone as
    // its permitted alphabet, which is none.
    let size = |lower, upper| list(vec![node(6, vec![int(lower), int(upper)])]);
    let d = node(
        10,
        vec![
            text("IA5String"),
            Cbor::Null,
            size(1, 4),
            Cbor::Null,
            list(vec![node(6, vec![text("A"), text("B")])]),
        ],
    );
    let r = node(
        11,
        vec![
            text("D"),
            Cbor::Null,
            size(1, 1),
            Cbor::Null,
            list(vec![node(8, vec![])]),
        ],
    );
    let nodes = module(vec![assignment("D", d), assignment("R", r)], vec![]);
    let spec = Specification::from_nodes(&nodes, Options::new(Codec::Uper)).unwrap();

    // 2 bits for the length and 1 bit for each character.
    let value = Value::Text("BA".into());
    let encoded = spec.encode("D", &value).unwrap();
    assert_eq!(hex(&encoded), "60");
    assert_eq!(spec.decode("D", &encoded).unwrap(), value);

    // No length, and 7 bits for a character of IA5String.
    let value = Value::Text("C".into());
    let encoded = spec.encode("R", &value).unwrap();
    assert_eq!(hex(&encoded), "86");
    assert_eq!(spec.decode("R", &encoded).unwrap(), value);
}
