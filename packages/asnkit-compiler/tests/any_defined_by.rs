// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

use asnkit_compiler::container::Protection;
use asnkit_compiler::{
    CompileOptions, Input, compile, compile_to_asni_with_options, compile_with_options,
};
use asnkitrt::{
    container::Checks,
    runtime::{Options, Specification},
    types::Codec,
    value::Value,
};

fn config(type_name: &str, member: &str, choices: &str) -> CompileOptions {
    CompileOptions::from_toml(&format!(
        r#"
[[any_defined_by]]
type = "{type_name}"
member = "{member}"
[any_defined_by.choices]
{choices}
"#
    ))
    .unwrap()
}

fn container(source: &str, options: &CompileOptions) -> Vec<u8> {
    compile_to_asni_with_options(
        &Input::String(source.into()),
        None,
        None,
        &Protection::default(),
        options,
    )
    .unwrap()
}

fn message<'a>(key: Value<'a>, payload: Value<'a>) -> Value<'a> {
    Value::Map(vec![("kind".into(), key), ("payload".into(), payload)])
}

const INTEGER: &str = "M DEFINITIONS ::= BEGIN
K ::= INTEGER { text(1), number(2) }
one K ::= text
T ::= SEQUENCE { kind K, payload ANY DEFINED BY kind }
END";

#[test]
fn integer_choices_round_trip_in_all_codecs() {
    let config = config("M.T", "payload", "one = 'UTF8String'\nnumber = 'INTEGER'");
    let asni = container(INTEGER, &config);
    for codec in [
        Codec::Ber,
        Codec::Der,
        Codec::Per,
        Codec::Uper,
        Codec::Oer,
        Codec::Jer,
        Codec::Xer,
        Codec::Gser,
    ] {
        let spec =
            Specification::from_asni(&asni, &Checks::default(), Options::new(codec)).unwrap();
        for value in [
            message(Value::Integer(1), Value::text("hello")),
            message(Value::Integer(2), Value::Integer(42)),
        ] {
            let encoded = spec.encode("T", &value).unwrap();
            assert_eq!(spec.decode("T", &encoded).unwrap(), value, "{codec:?}");
        }
        assert!(
            spec.encode("T", &message(Value::Integer(3), Value::Integer(42)))
                .is_err()
        );
    }
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Ber)).unwrap();
    assert!(spec.decode("T", &[0x30, 6, 2, 1, 3, 2, 1, 42]).is_err());
}

#[test]
fn named_and_imported_oid_choices_preserve_tags_and_constraints() {
    let source = "A DEFINITIONS ::= BEGIN
base OBJECT IDENTIFIER ::= { iso(1) 2 840 }
id OBJECT IDENTIFIER ::= { base 113549 }
Payload ::= INTEGER (0..7)
END
M DEFINITIONS ::= BEGIN
IMPORTS id, Payload FROM A;
T ::= SEQUENCE { kind OBJECT IDENTIFIER, payload [0] EXPLICIT ANY DEFINED BY kind }
END";
    let options = config("M.T", "payload", "id = 'Payload'\n'1.2.3' = 'NULL'");
    let asni = container(source, &options);
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Der)).unwrap();
    let rsa: asnkitrt::value::ObjectIdentifier<'static> = "1.2.840.113549".parse().unwrap();
    let value = message(Value::ObjectIdentifier(rsa.clone()), Value::Integer(5));
    let encoded = spec.encode("T", &value).unwrap();
    assert!(encoded.windows(5).any(|part| part == [0xa0, 3, 2, 1, 5]));
    assert_eq!(spec.decode("T", &encoded).unwrap(), value);
    // The selected reference retains the constraints of the assigned type.
    let t = spec.types().find_unique("T").unwrap();
    let asnkitrt::types::Kind::Sequence(members) = spec.types().kind(t) else {
        panic!()
    };
    let asnkitrt::types::Kind::AnyDefinedBy { choices, .. } = spec.types().kind(members.root[1].ty)
    else {
        panic!()
    };
    let selected = choices
        .iter()
        .find(|(key, _)| *key == asnkitrt::node::Const::ObjectIdentifier(rsa.clone()))
        .unwrap()
        .1;
    asnkitrt::check::check(spec.types(), selected, &Value::Integer(5)).unwrap();
    assert!(asnkitrt::check::check(spec.types(), selected, &Value::Integer(8)).is_err());
    let qualified = container(
        source,
        &config("M.T", "payload", "'A.id' = 'A.Payload'\n'1.2.3' = 'NULL'"),
    );
    assert_eq!(asni, qualified);
    assert!(
        compile_with_options(
            &Input::String(source.into()),
            &config(
                "M.T",
                "payload",
                "id = 'NULL'\n'1.2.840.113549' = 'INTEGER'"
            )
        )
        .unwrap_err()
        .message
        .contains("Duplicate selector")
    );
}

#[test]
fn enumeration_names_and_numeric_mode() {
    let source = "M DEFINITIONS ::= BEGIN
K ::= ENUMERATED { text(2), number(7) }
one K ::= text
T ::= SEQUENCE { kind K, payload ANY DEFINED BY kind }
END";
    let asni = container(
        source,
        &config("M.T", "payload", "one = 'UTF8String'\nnumber = 'INTEGER'"),
    );
    for numeric in [false, true] {
        for codec in [Codec::Ber, Codec::Per, Codec::Jer, Codec::Gser] {
            let mut options = Options::new(codec);
            options.numeric_enums = numeric;
            let spec = Specification::from_asni(&asni, &Checks::default(), options).unwrap();
            let value = message(
                if numeric {
                    Value::Integer(2)
                } else {
                    Value::text("text")
                },
                Value::text("ok"),
            );
            let encoded = spec.encode("T", &value).unwrap();
            assert_eq!(spec.decode("T", &encoded).unwrap(), value);
        }
    }
}

#[test]
fn nested_collections_and_extension_groups() {
    let source = "M DEFINITIONS ::= BEGIN
T ::= SEQUENCE OF SEQUENCE {
 nested SET OF SEQUENCE { kind INTEGER, ..., [[ payload ANY DEFINED BY kind ]] }
}
END";
    let asni = container(source, &config("M.T", "nested.payload", "'1' = 'BOOLEAN'"));
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Jer)).unwrap();
    let value = Value::List(vec![Value::Map(vec![(
        "nested".into(),
        Value::List(vec![message(Value::Integer(1), Value::Boolean(true))]),
    )])]);
    assert_eq!(
        spec.decode("T", &spec.encode("T", &value).unwrap())
            .unwrap(),
        value
    );
}

#[test]
fn open_type_choice_retains_raw_bytes() {
    let asni = container(INTEGER, &config("M.T", "payload", "'1' = 'OpenType'"));
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Der)).unwrap();
    let value = message(Value::Integer(1), Value::Bytes(vec![5, 0].into()));
    assert_eq!(
        spec.decode("T", &spec.encode("T", &value).unwrap())
            .unwrap(),
        value
    );
}

#[test]
fn configuration_and_resolution_errors() {
    for text in [
        "unknown = 1",
        "[[any_defined_by]]\ntype='M.T'\nmember='payload'",
        "[[any_defined_by]]\ntype='M.T'\nmember='payload'\ndefault='OpenType'\nchoices={}",
        "[[any_defined_by]]\ntype='M.T'\nmember='payload'\nchoices={'1'='NULL', '1'='INTEGER'}",
    ] {
        assert!(CompileOptions::from_toml(text).is_err(), "{text}");
    }
    for (ty, member, choices, error) in [
        ("T", "payload", "'1'='NULL'", "Module.Type"),
        ("Missing.T", "payload", "'1'='NULL'", "not found"),
        ("M.Missing", "payload", "'1'='NULL'", "not found"),
        ("M.T", "absent", "'1'='NULL'", "not found"),
        ("M.T", "kind", "'1'='NULL'", "not an ANY"),
        ("M.T", "payload", "'1'='Missing'", "Missing"),
        (
            "M.T",
            "payload",
            "'1'='SEQUENCE'",
            "needs a type definition",
        ),
        ("M.T", "payload", "'1.5'='INTEGER'", "Unknown"),
        (
            "M.T",
            "payload",
            "'1'='NULL'\none='INTEGER'",
            "Duplicate selector",
        ),
        ("M.T", "payload", "", "at least one"),
    ] {
        let options = config(ty, member, choices);
        let result = compile_with_options(&Input::String(INTEGER.into()), &options).unwrap_err();
        assert!(result.message.contains(error), "{result}");
    }
    let mut duplicate = config("M.T", "payload", "'1'='NULL'");
    duplicate
        .any_defined_by
        .push(duplicate.any_defined_by[0].clone());
    assert!(
        compile_with_options(&Input::String(INTEGER.into()), &duplicate)
            .unwrap_err()
            .message
            .contains("Duplicate mapping")
    );
}

#[test]
fn no_options_preserves_existing_compilation() {
    let input = Input::String(INTEGER.into());
    assert_eq!(
        compile(&input).unwrap(),
        compile_with_options(&input, &CompileOptions::default()).unwrap()
    );
}

#[test]
fn selectors_are_checked_and_canonicalized() {
    for (source, choices, expected) in [
        (
            "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { kind OBJECT IDENTIFIER, payload ANY DEFINED BY kind } END",
            "'3.1'='NULL'",
            "valid OBJECT IDENTIFIER",
        ),
        (
            "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { kind OBJECT IDENTIFIER, payload ANY DEFINED BY kind } END",
            "'1.40'='NULL'",
            "valid OBJECT IDENTIFIER",
        ),
        (
            "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { kind OBJECT IDENTIFIER, payload ANY DEFINED BY kind } END",
            "'1.2.3'='NULL'\n'01.02.03'='INTEGER'",
            "Duplicate selector",
        ),
        (INTEGER, "'1'='NULL'\n'01'='INTEGER'", "Duplicate selector"),
        (
            INTEGER,
            "'170141183460469231731687303715884105728'='NULL'",
            "128 bits",
        ),
        (
            "M DEFINITIONS ::= BEGIN K ::= ENUMERATED { red } wrong INTEGER ::= 1 T ::= SEQUENCE { kind K, payload ANY DEFINED BY kind } END",
            "wrong='NULL'",
            "defining ENUMERATED",
        ),
        (
            "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { kind REAL, payload ANY DEFINED BY kind } END",
            "'1'='NULL'",
            "defining member must",
        ),
        (
            "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { payload ANY DEFINED BY missing } END",
            "'1'='NULL'",
            "Defining member",
        ),
    ] {
        let error = compile_with_options(
            &Input::String(source.into()),
            &config("M.T", "payload", choices),
        )
        .unwrap_err();
        assert!(error.message.contains(expected), "{error}");
    }
    let source =
        "M DEFINITIONS ::= BEGIN T ::= SEQUENCE { kind BOOLEAN, payload ANY DEFINED BY kind } END";
    let asni = container(
        source,
        &config("M.T", "payload", "TRUE='NULL'\nFALSE='INTEGER'"),
    );
    let spec =
        Specification::from_asni(&asni, &Checks::default(), Options::new(Codec::Der)).unwrap();
    let value = message(Value::Boolean(true), Value::Null);
    assert_eq!(
        spec.decode("T", &spec.encode("T", &value).unwrap())
            .unwrap(),
        value
    );
}

#[test]
fn cyclic_selector_references_are_errors() {
    for source in [
        "M DEFINITIONS ::= BEGIN IMPORTS x FROM N; T ::= SEQUENCE { kind INTEGER, payload ANY DEFINED BY kind } END N DEFINITIONS ::= BEGIN IMPORTS x FROM M; END",
        "M DEFINITIONS ::= BEGIN x INTEGER ::= y y INTEGER ::= x T ::= SEQUENCE { kind INTEGER, payload ANY DEFINED BY kind } END",
    ] {
        let error = compile_with_options(
            &Input::String(source.into()),
            &config("M.T", "payload", "x='NULL'"),
        )
        .unwrap_err();
        assert!(error.message.contains("Cyclic"), "{error}");
    }
}
