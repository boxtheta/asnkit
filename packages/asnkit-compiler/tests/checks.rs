// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! A type defined by itself alone is an error, as the runtimes refuse
//! it, and so are a type of which no value ends, and constraints that
//! permit no value, which the runtimes would take for none. What is
//! valid but likely to cause trouble is a warning: a type that refers to
//! itself.

use asnkit_compiler::{CompileOptions, Input, Warning, compile, compile_to_asni_with_warnings};
use std::fs;
use std::path::PathBuf;

/// A module of `types`, one on each line from the second.
fn module(types: &str) -> Input {
    Input::String(format!(
        "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n{types}\nEND\n"
    ))
}

fn warnings_of(types: &str) -> Vec<Warning> {
    compile_to_asni_with_warnings(&module(types), &CompileOptions::default())
        .unwrap()
        .warnings
}

fn kinds_of(types: &str) -> Vec<String> {
    warnings_of(types)
        .into_iter()
        .map(|warning| warning.kind)
        .collect()
}

fn error_of(types: &str) -> String {
    let error = compile(&module(types)).unwrap_err();

    assert_eq!(error.kind, "CompileError");

    error.message
}

#[test]
fn a_type_that_is_only_itself_is_an_error() {
    assert_eq!(
        error_of("A ::= A"),
        "Type 'A' in module 'Foo' is defined by itself, at line 2, column 1."
    );

    // With a tag, or a constraint, it is still nothing else.
    assert_eq!(
        error_of("A ::= [0] A"),
        "Type 'A' in module 'Foo' is defined by itself, at line 2, column 1."
    );

    // Through other types, which are told in the order they refer to
    // one another.
    assert_eq!(
        error_of("X ::= A\nA ::= C (1..2)\nB ::= A\nC ::= B"),
        "Type 'A' in module 'Foo' is defined by itself, through 'C' and 'B', at line 3, \
         column 1."
    );
}

#[test]
fn a_type_defined_by_itself_is_refused_before_it_is_followed() {
    // These were followed without end.
    assert!(error_of("A ::= B (1..2)\nB ::= A").contains("is defined by itself"));
    assert!(
        error_of("S ::= SEQUENCE { a A DEFAULT 1 }\nA ::= B\nB ::= A")
            .contains("is defined by itself")
    );
}

#[test]
fn a_type_that_is_only_itself_once_parameters_are_given_is_an_error() {
    assert_eq!(
        error_of("P{X} ::= X\nA ::= P{A}"),
        "Type 'A' in module 'Foo' is defined by itself, at line 3, column 1."
    );
}

#[test]
fn a_type_defined_by_itself_across_modules_names_the_modules() {
    let error = compile(&Input::String(
        "Foo DEFINITIONS ::= BEGIN IMPORTS B FROM Bar; A ::= B END\n\
         Bar DEFINITIONS ::= BEGIN IMPORTS A FROM Foo; B ::= A END\n"
            .to_string(),
    ))
    .unwrap_err();

    assert_eq!(
        error.message,
        "Type 'Foo.A' is defined by itself, through 'Bar.B', at line 1, column 47."
    );
}

#[test]
fn components_of_itself_is_an_error() {
    assert_eq!(
        error_of("S ::= SEQUENCE { a INTEGER, COMPONENTS OF S }"),
        "Type 'S' in module 'Foo' takes COMPONENTS OF itself, at line 2, column 1."
    );
    assert_eq!(
        error_of("T ::= SEQUENCE { COMPONENTS OF U }\nU ::= SEQUENCE { COMPONENTS OF T }"),
        "Type 'T' in module 'Foo' takes COMPONENTS OF itself, through 'U', at line 2, column 1."
    );
}

#[test]
fn a_type_that_refers_to_itself_is_told_of() {
    let warnings = warnings_of("List ::= SEQUENCE { head INTEGER, tail List OPTIONAL }");

    assert_eq!(
        warnings,
        [Warning {
            kind: "RecursiveType".to_string(),
            message: "Type 'List' in module 'Foo' refers to itself, at line 2, column 1: its \
                      values nest as deep as the data has them, and asnkit decodes no more \
                      than 128 levels of nesting."
                .to_string(),
        }]
    );

    // Through a SEQUENCE OF, which may be empty.
    assert_eq!(
        kinds_of("Tree ::= SEQUENCE { value INTEGER, children SEQUENCE OF Tree }"),
        ["RecursiveType"]
    );
}

#[test]
fn types_that_refer_to_one_another_are_told_of_once() {
    let warnings =
        warnings_of("A ::= CHOICE { b B, n NULL }\nB ::= SEQUENCE { a A }\nC ::= SEQUENCE { b B }");

    assert_eq!(warnings.len(), 1);
    assert_eq!(
        warnings[0].message,
        "Types 'A' and 'B' in module 'Foo' refer to each other, at line 2, column 1: their \
         values nest as deep as the data has them, and asnkit decodes no more than 128 levels \
         of nesting."
    );
}

#[test]
fn a_long_list_of_types_is_cut_short() {
    let types: Vec<String> = (0..8)
        .map(|n| format!("T{n} ::= SEQUENCE {{ next T{} OPTIONAL }}", (n + 1) % 8))
        .collect();
    let warnings = warnings_of(&types.join("\n"));

    assert!(
        warnings[0]
            .message
            .starts_with("Types 'T0', 'T1', 'T2', 'T3', 'T4' and 3 more in module 'Foo' refer")
    );
}

#[test]
fn a_type_of_which_every_value_holds_another_is_an_error() {
    assert_eq!(
        error_of("Never ::= SEQUENCE { next Never }"),
        "Type 'Never' in module 'Foo' has no finite value, as each of its values would hold \
         another, at line 2, column 1."
    );

    for types in [
        "C ::= CHOICE { a C, b SEQUENCE { c C } }",
        "L ::= SEQUENCE { items SEQUENCE (SIZE (1..MAX)) OF L }",
        "S ::= SET { a INTEGER, s S }",
        // A member after the extension additions is in the root.
        "S ::= SEQUENCE { a INTEGER, ..., b BOOLEAN OPTIONAL, ..., s S }",
        // Once a parameterized type is given its parameters.
        "P{X} ::= SEQUENCE { x X }\nQ ::= P{Q}",
    ] {
        assert!(error_of(types).contains("has no finite value"), "{types}");
    }

    assert_eq!(
        error_of("Ping ::= SEQUENCE { pong Pong }\nPong ::= SEQUENCE { ping Ping }"),
        "Types 'Ping' and 'Pong' in module 'Foo' have no finite value, as each of their values \
         would hold another, at line 2, column 1."
    );
}

#[test]
fn a_type_with_a_way_out_has_values() {
    for types in [
        "S ::= SEQUENCE { s S OPTIONAL }",
        "S ::= SEQUENCE { a INTEGER DEFAULT 0, s S DEFAULT { a 1 } }",
        "S ::= SEQUENCE { a INTEGER, ..., s S }",
        "S ::= SEQUENCE { a INTEGER, ..., [[ s S ]] }",
        "C ::= CHOICE { c C, n NULL }",
        // An alternative that is not known may be what a value is.
        "C ::= CHOICE { c C, ... }",
        "L ::= SEQUENCE { items SEQUENCE (SIZE (0..2)) OF L }",
        "L ::= SEQUENCE { items SEQUENCE (SIZE (1..2, ...)) OF L }",
    ] {
        assert_eq!(kinds_of(types), ["RecursiveType"], "{types}");
    }
}

#[test]
fn only_the_types_that_hold_themselves_are_told_of() {
    // Message has no value as Never has none, and A refers to itself but
    // would have one if Never did, so Never is what is told.
    assert_eq!(
        error_of(
            "Message ::= SEQUENCE { never Never }\nNever ::= SEQUENCE { next Never }\n\
             A ::= SEQUENCE { a A OPTIONAL, never Never }",
        ),
        "Type 'Never' in module 'Foo' has no finite value, as each of its values would hold \
         another, at line 3, column 1."
    );

    // Of a group, those that have none of themselves.
    assert!(
        error_of("A ::= CHOICE { b B, n NULL }\nB ::= SEQUENCE { a A, b B }")
            .starts_with("Type 'B' in module 'Foo' has no finite value")
    );
}

#[test]
fn constraints_that_permit_no_value_are_an_error() {
    let cases = [
        (
            "I ::= INTEGER (5..1)",
            "Type 'I' in module 'Foo' permits no value, as its constraints have none in \
             common, at line 2, column 1.",
        ),
        // With those of the type it refers to.
        (
            "J ::= INTEGER (0..5)\nK ::= J (10..20)",
            "Type 'K' in module 'Foo' permits no value, as its constraints have none in \
             common, at line 3, column 1.",
        ),
        (
            "S ::= IA5String (SIZE (3..1))",
            "Type 'S' in module 'Foo' permits no value, as its constraints have none in \
             common, at line 2, column 1.",
        ),
        // In a member, where it is.
        (
            "T ::= SEQUENCE {\n  a INTEGER,\n  b SEQUENCE OF INTEGER (1..2 ^ 3..4)\n}",
            "The type of 'T.b[]' in module 'Foo' permits no value, as its constraints have \
             none in common, at line 4, column 3.",
        ),
        // In a field of an object class, which the types that use it take.
        (
            "C ::= CLASS {\n  &Type,\n  &id INTEGER (5..1) UNIQUE\n}\n\
             X ::= SEQUENCE { id C.&id }",
            "The type of 'C.&id' in module 'Foo' permits no value, as its constraints have \
             none in common, at line 4, column 3.",
        ),
    ];

    for (types, message) in cases {
        assert_eq!(error_of(types), message, "{types}");
    }

    // A root that has none but is extensible permits what is not in it,
    // and an empty SEQUENCE OF is a value.
    assert!(warnings_of("I ::= INTEGER (5..1, ...)").is_empty());
    assert!(warnings_of("L ::= SEQUENCE (SIZE (0)) OF INTEGER").is_empty());
}

#[test]
fn a_specification_with_nothing_wrong_has_no_warnings() {
    assert!(
        warnings_of(
            "Question ::= SEQUENCE { id INTEGER (0..255), question IA5String (SIZE (1..64)) }\n\
             Answer ::= CHOICE { yes NULL, no NULL, items SEQUENCE OF Question }"
        )
        .is_empty()
    );
}

#[test]
fn a_warning_tells_the_file_it_is_in() {
    let folder = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("checks");
    fs::create_dir_all(&folder).unwrap();
    let path = folder.join("list.asn");
    fs::write(
        &path,
        "Foo DEFINITIONS ::= BEGIN\n\nList ::= SEQUENCE { tail List OPTIONAL }\nEND\n",
    )
    .unwrap();

    let compiled = compile_to_asni_with_warnings(
        &Input::Files {
            paths: vec![path.clone()],
            encoding: "utf-8".to_string(),
        },
        &CompileOptions::default(),
    )
    .unwrap();

    assert!(compiled.warnings[0].message.starts_with(&format!(
        "Type 'List' in module 'Foo' refers to itself, in '{}' at line 3, column 1:",
        path.display()
    )));
}
