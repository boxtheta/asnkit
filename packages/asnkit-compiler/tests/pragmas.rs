// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Pragmas, `--@ key = value` on the lines before a module, an
//! assignment, a member or a field of an object class, are in the syntax
//! tree as pairs, for plugins, and not in a compiled specification.

use asnkit_compiler::{Input, compile, compile_to_asni, parse};
use serde_json::{Value, json};
use std::fs;

fn tree_of(text: &str) -> Value {
    parse(&Input::String(text.to_string())).unwrap()
}

fn error_of(text: &str) -> String {
    let error = parse(&Input::String(text.to_string())).unwrap_err();

    assert_eq!(error.kind, "ParseError");

    error.message
}

const SPECIFICATION: &str = "\
--@ rpc.package = greeter.v1
Greeter DEFINITIONS AUTOMATIC TAGS ::= BEGIN

-- What is asked.
--@ rpc.message
--@ go-name = HelloRequest
HelloRequest ::= SEQUENCE {
    --@ json.name = the_name
    name UTF8String,
    /* A comment between. */
    --@ deprecated = true
    age INTEGER OPTIONAL,
    inner SEQUENCE {
        --@ x = 1
        deep BOOLEAN
    },
    ...,
    [[
        --@ since = 2
        late BOOLEAN
    ]]
}

Reply ::= CHOICE {
    --@ k = v
    text UTF8String,
    list SEQUENCE OF SEQUENCE {
        --@ in = list
        item INTEGER
    }
}

--@ rpc.method = Greet(HelloRequest) returns (Reply)
greet INTEGER ::= 1

--@ class = yes
METHOD ::= CLASS {
    --@ field = id
    &id INTEGER UNIQUE
}

--@ object = yes
hello METHOD ::= { &id 1 }

--@ set = yes
Methods METHOD ::= { hello }

END
";

#[test]
fn pragmas_are_pairs_of_what_they_come_before() {
    let tree = tree_of(SPECIFICATION);
    let module = &tree["modules"][0];

    assert_eq!(module["pragmas"], json!([["rpc.package", "greeter.v1"]]));

    let request = &module["types"][0];
    assert_eq!(request["name"], "HelloRequest");
    assert_eq!(
        request["pragmas"],
        json!([["rpc.message", ""], ["go-name", "HelloRequest"]])
    );

    let members = &request["type"]["members"];
    assert_eq!(members[0]["pragmas"], json!([["json.name", "the_name"]]));
    assert_eq!(members[1]["pragmas"], json!([["deprecated", "true"]]));
    assert_eq!(members[2].get("pragmas"), None);
    assert_eq!(
        members[2]["type"]["members"][0]["pragmas"],
        json!([["x", "1"]])
    );
    assert_eq!(members[4]["members"][0]["pragmas"], json!([["since", "2"]]));

    let reply = &module["types"][1]["type"]["members"];
    assert_eq!(reply[0]["pragmas"], json!([["k", "v"]]));
    assert_eq!(
        reply[1]["type"]["element"]["members"][0]["pragmas"],
        json!([["in", "list"]])
    );

    assert_eq!(
        module["values"][0]["pragmas"],
        json!([["rpc.method", "Greet(HelloRequest) returns (Reply)"]])
    );
    assert_eq!(
        module["object_classes"][0]["pragmas"],
        json!([["class", "yes"]])
    );
    assert_eq!(
        module["object_classes"][0]["fields"][0]["pragmas"],
        json!([["field", "id"]])
    );
    assert_eq!(module["objects"][0]["pragmas"], json!([["object", "yes"]]));
    assert_eq!(module["object_sets"][0]["pragmas"], json!([["set", "yes"]]));
}

#[test]
fn a_compiled_specification_has_no_pragmas() {
    let input = Input::String(SPECIFICATION.to_string());
    let without = Input::String(
        SPECIFICATION
            .lines()
            .filter(|line| !line.trim_start().starts_with("--@"))
            .collect::<Vec<_>>()
            .join("\n"),
    );

    assert!(!compile(&input).unwrap().to_string().contains("pragma"));
    assert_eq!(compile(&input).unwrap(), compile(&without).unwrap());
    assert_eq!(
        compile_to_asni(&input).unwrap()[10..].len(),
        compile_to_asni(&without).unwrap()[10..].len()
    );
}

#[test]
fn a_tree_without_pragmas_has_no_field_of_them() {
    let tree = tree_of("Foo DEFINITIONS ::= BEGIN\n-- @ not a pragma\nA ::= INTEGER\nEND\n");

    assert!(!tree.to_string().contains("pragmas"));
}

#[test]
fn a_pragma_in_a_comment_of_several_lines_is_none() {
    let tree = tree_of("Foo DEFINITIONS ::= BEGIN\n/* --@ a = 1 */\nA ::= INTEGER\nEND\n");

    assert!(!tree.to_string().contains("pragmas"));
}

#[test]
fn a_pragma_ends_where_a_comment_does() {
    let tree = tree_of("Foo DEFINITIONS ::= BEGIN\n--@ a = 1 -- A ::= INTEGER\nEND\n");

    assert_eq!(
        tree["modules"][0]["types"][0]["pragmas"],
        json!([["a", "1"]])
    );
}

#[test]
fn pragmas_of_files_are_of_their_own_file() {
    let folder = tempfile::tempdir().unwrap();
    let a = folder.path().join("a.asn");
    let b = folder.path().join("b.asn");

    fs::write(&a, "A DEFINITIONS ::= BEGIN\nX ::= INTEGER\nEND\n").unwrap();
    fs::write(
        &b,
        "--@ file = b\nB DEFINITIONS ::= BEGIN\n--@ y\nY ::= INTEGER\nEND\n",
    )
    .unwrap();

    let tree = parse(&Input::Files {
        paths: vec![a, b.clone()],
        encoding: "utf-8".to_string(),
    })
    .unwrap();

    assert_eq!(tree["modules"][0].get("pragmas"), None);
    assert_eq!(tree["modules"][1]["pragmas"], json!([["file", "b"]]));
    assert_eq!(
        tree["modules"][1]["types"][0]["pragmas"],
        json!([["y", ""]])
    );

    // A pragma at the end of a file comes before nothing of its own.
    fs::write(
        &b,
        "B DEFINITIONS ::= BEGIN\nY ::= INTEGER\nEND\n--@ last\n",
    )
    .unwrap();

    let error = parse(&Input::Files {
        paths: vec![b],
        encoding: "utf-8".to_string(),
    })
    .unwrap_err();

    assert!(
        error.message.starts_with("Invalid pragma in '")
            && error.message.contains("at line 4, column 1"),
        "{}",
        error.message
    );
}

#[test]
fn a_pragma_after_something_on_its_line_is_an_error() {
    let message = error_of(
        "Foo DEFINITIONS ::= BEGIN\nA ::= SEQUENCE {\n    a INTEGER, --@ k = v\n    b BOOLEAN\n}\nEND\n",
    );

    assert_eq!(
        message,
        "Invalid pragma at line 3, column 16: 'a INTEGER, >!<--@ k = v': A pragma is on a line of \
         its own, before what it is of."
    );
}

#[test]
fn a_pragma_before_what_has_none_is_an_error() {
    for (text, line, column) in [
        (
            "Foo DEFINITIONS ::= BEGIN\nA ::= INTEGER\n--@ k = v\nEND\n",
            3,
            1,
        ),
        (
            "Foo DEFINITIONS ::= BEGIN\nA ::= ENUMERATED {\n--@ k = v\n  a\n}\nEND\n",
            3,
            1,
        ),
        (
            "Foo DEFINITIONS ::= BEGIN\nA ::= SEQUENCE {\n  a INTEGER,\n  --@ k = v\n  ...\n}\nEND\n",
            4,
            3,
        ),
        (
            "Foo DEFINITIONS ::= BEGIN\nA ::= INTEGER\nEND\n--@ k = v\n",
            4,
            1,
        ),
    ] {
        let message = error_of(text);

        assert!(
            message.starts_with(&format!("Invalid pragma at line {line}, column {column}: ")),
            "{message}"
        );
        assert!(
            message.ends_with(
                "A pragma comes on the lines before a module, an assignment, a member or a field \
                 of an object class, and this one does not."
            ),
            "{message}"
        );
    }
}

#[test]
fn a_key_given_twice_is_an_error() {
    let message = error_of("Foo DEFINITIONS ::= BEGIN\n--@ k = 1\n--@ k = 2\nA ::= INTEGER\nEND\n");

    assert_eq!(
        message,
        "Invalid pragma at line 3, column 1: '>!<--@ k = 2': The pragma 'k' is given twice to \
         'A'."
    );
}

#[test]
fn a_key_is_a_letter_and_more() {
    for pragma in ["--@", "--@ = 1", "--@ 1 = 1", "--@ a b = 1", "--@ a/b"] {
        let message = error_of(&format!(
            "Foo DEFINITIONS ::= BEGIN\n{pragma}\nA ::= INTEGER\nEND\n"
        ));

        assert!(
            message.starts_with("Invalid pragma at line 2, column 1: "),
            "{message}"
        );
    }
}
