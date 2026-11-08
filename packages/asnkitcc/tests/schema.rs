// Copyright (c) 2025 BoxTheta
// SPDX-License-Identifier: Apache-2.0

//! `asnkitcc schema` writes a JSON Schema of JER or a DTD of XER of the
//! types of a specification, and refuses what a schema can not tell.
//! `tests/test_schema.py` holds what it writes to the vectors, with a
//! validator of each.

use std::fs;
use std::path::Path;
use std::process::{Command, Output};

const SHAPES: &str = "Shapes DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    Shape ::= CHOICE { circle Circle, square Square, ... }
    Circle ::= SEQUENCE { radius INTEGER (0..100), label UTF8String (SIZE (1..8)) OPTIONAL }
    Square ::= SEQUENCE { side REAL, color Color DEFAULT red }
    Color ::= ENUMERATED { red, green }
    Shapes ::= SEQUENCE (SIZE (1..10)) OF Shape
    Flags ::= BIT STRING (SIZE (4))
    Code ::= PrintableString (FROM (\"A\"..\"F\"))
    Tree ::= SEQUENCE { value INTEGER, children SEQUENCE OF Tree }
END
";

const OPEN: &str = "Open DEFINITIONS ::= BEGIN
    Known ::= SEQUENCE { kind INTEGER }
    Open ::= SEQUENCE { kind INTEGER, value [0] ANY }
END
";

const CLASH: &str = "Clash DEFINITIONS ::= BEGIN
    A ::= SEQUENCE { x INTEGER }
    B ::= SEQUENCE { x BOOLEAN }
END
";

const PAIRS: &str = "Pairs DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    Pair ::= SET { left INTEGER (-5..5 | 10), right BOOLEAN OPTIONAL }
    Pairs ::= SEQUENCE (SIZE (2..4)) OF Pair
    Bytes ::= OCTET STRING (SIZE (16))
    Name ::= VisibleString (FROM (\"a\"..\"z\" | \"<\"))
    Flags ::= SEQUENCE OF BOOLEAN
END
";

fn run(folder: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_asnkitcc"))
        .current_dir(folder)
        .args(args)
        .env("PATH", "")
        .output()
        .unwrap()
}

/// What the command writes on standard output, which succeeds.
fn schema(folder: &Path, args: &[&str]) -> String {
    let output = run(folder, &[&["schema"][..], args].concat());

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    String::from_utf8(output.stdout).unwrap()
}

/// The error the command fails with.
fn error(folder: &Path, args: &[&str]) -> String {
    let output = run(folder, args);

    assert!(!output.status.success());
    assert!(output.stdout.is_empty());

    String::from_utf8(output.stderr).unwrap()
}

fn folder() -> tempfile::TempDir {
    let folder = tempfile::tempdir().unwrap();

    for (name, text) in [
        ("shapes.asn", SHAPES),
        ("open.asn", OPEN),
        ("clash.asn", CLASH),
        ("pairs.asn", PAIRS),
    ] {
        fs::write(folder.path().join(name), text).unwrap();
    }

    folder
}

/// The text without its spaces and line breaks, to look for what is in
/// it whatever the indentation.
fn compact(text: &str) -> String {
    text.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn a_json_schema_of_jer() {
    let folder = folder();
    let text = schema(folder.path(), &["--format", "json-schema", "shapes.asn"]);
    let json = compact(&text);

    assert!(
        text.starts_with("{\n  \"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n")
    );
    // Every type, and no root.
    assert!(!json.contains("\"$ref\":\"#/$defs/Shape\",\"$defs\""));

    for wanted in [
        // A CHOICE with an extension marker takes any other name.
        r##""Shape":{"anyOf":[{"type":"object","properties":{"circle":{"$ref":"#/$defs/Circle"}},"required":["circle"],"additionalProperties":false}"##,
        r##""propertyNames":{"not":{"enum":["circle","square"]}}"##,
        r##""radius":{"type":"integer","minimum":0,"maximum":100}"##,
        r##""label":{"type":"string","minLength":1,"maxLength":8}"##,
        // A member with a default may be left out.
        r##""required":["radius"],"additionalProperties":false"##,
        r##""required":["side"]"##,
        r##""side":{"anyOf":[{"type":"number"},{"enum":["-0","INF","-INF","NaN"]}]}"##,
        r##""Color":{"enum":["red","green"]}"##,
        r##""Shapes":{"type":"array","items":{"$ref":"#/$defs/Shape"},"minItems":1,"maxItems":10}"##,
        // A BIT STRING of one size is its bytes alone.
        r##""Flags":{"type":"string","pattern":"^(?:[0-9A-Fa-f]{2}){1}$"}"##,
        r##""Code":{"type":"string","pattern":"^[A-F]*$"}"##,
        r##""children":{"type":"array","items":{"$ref":"#/$defs/Tree"}}"##,
    ] {
        assert!(json.contains(&compact(wanted)), "{wanted} not in {text}");
    }
}

#[test]
fn the_types_given_and_those_they_refer_to() {
    let folder = folder();
    let json = compact(&schema(
        folder.path(),
        &["--format", "json-schema", "-t", "Shapes", "shapes.asn"],
    ));

    assert!(json.contains(r##""$ref":"#/$defs/Shapes","$defs":{"Shapes":"##));

    for name in ["Shape", "Circle", "Square", "Color"] {
        assert!(json.contains(&format!("\"{name}\":{{")), "{name}");
    }

    for name in ["Flags", "Tree"] {
        assert!(!json.contains(&format!("\"{name}\":{{")), "{name}");
    }

    let both = compact(&schema(
        folder.path(),
        &[
            "--format",
            "json-schema",
            "-t",
            "Shapes.Flags",
            "-t",
            "Code",
            "shapes.asn",
        ],
    ));

    assert!(both.contains(r##""anyOf":[{"$ref":"#/$defs/Flags"},{"$ref":"#/$defs/Code"}]"##));
    assert!(
        error(
            folder.path(),
            &["schema", "--format", "dtd", "-t", "Nothing", "shapes.asn"]
        )
        .starts_with("error: no type 'Nothing' in the specification")
    );
}

#[test]
fn a_dtd_of_xer() {
    let folder = folder();
    let text = schema(folder.path(), &["--format", "dtd", "shapes.asn"]);

    assert!(text.starts_with("<!-- schema generated by asnkit from shapes.asn -->\n"));
    assert!(text.contains(
        "<!-- The root elements: Shape, Circle, Square, Color, Shapes, Flags, Code, Tree. -->\n"
    ));

    for wanted in [
        // An element before those in it.
        "<!ELEMENT Shape (circle|square)>\n<!ELEMENT circle (radius, label?)>\n",
        "<!ELEMENT radius (#PCDATA)>\n",
        "<!ELEMENT label (#PCDATA|nul|soh|",
        "<!ELEMENT square (side, color?)>\n",
        "<!ELEMENT side (#PCDATA|PLUS-INFINITY|MINUS-INFINITY|NOT-A-NUMBER)*>\n",
        "<!ELEMENT color (red|green)>\n<!ELEMENT red EMPTY>\n<!ELEMENT green EMPTY>\n",
        // The items of a CHOICE have no element of their own (X.693 9.3.6).
        "<!ELEMENT Shapes (circle|square)+>\n",
        "<!ELEMENT Flags (#PCDATA)>\n",
        // A PrintableString has no control characters.
        "<!ELEMENT Code (#PCDATA)>\n",
        "<!ELEMENT Tree (value, children)>\n<!ELEMENT value (#PCDATA)>\n<!ELEMENT children (Tree)*>\n",
    ] {
        assert!(text.contains(wanted), "{wanted} not in {text}");
    }

    // Each element is declared once.
    assert_eq!(text.matches("<!ELEMENT circle ").count(), 1);
}

#[test]
fn the_format_is_told_by_the_extension_of_the_output() {
    let folder = folder();
    let path = folder.path();

    schema(path, &["-o", "shapes.dtd", "shapes.asn"]);
    schema(path, &["-o", "shapes.schema.json", "shapes.asn"]);

    assert!(
        fs::read_to_string(path.join("shapes.dtd"))
            .unwrap()
            .contains("<!ELEMENT Tree ")
    );
    assert!(
        fs::read_to_string(path.join("shapes.schema.json"))
            .unwrap()
            .contains("\"$defs\"")
    );
    assert!(
        error(path, &["schema", "shapes.asn"]).starts_with("error: give the format with --format")
    );
    assert!(
        error(path, &["schema", "--format", "yaml", "shapes.asn"])
            .starts_with("error: unknown format 'yaml'")
    );
    assert!(
        error(path, &["compile", "--format", "dtd", "shapes.asn"])
            .starts_with("error: --format and --skip-unsupported are for schema.")
    );
    assert!(
        error(path, &["compile", "-t", "Shape", "shapes.asn"])
            .starts_with("error: --type is for convert and schema.")
    );
}

#[test]
fn a_choice_that_is_an_item_of_itself_is_a_model_group() {
    let folder = folder();
    let path = folder.path();

    fs::write(
        path.join("tree.asn"),
        "Tree DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    Node ::= CHOICE { leaf INTEGER, group SEQUENCE SIZE (1..MAX) OF Node }
    Forest ::= SEQUENCE OF Node
END
",
    )
    .unwrap();

    let text = schema(path, &["--format", "xsd", "-t", "Forest", "tree.asn"]);

    for wanted in [
        // An item that is a CHOICE has no element of its own: it is the
        // model group of its type, which can have itself in it.
        "<xs:complexType name=\"Forest\">\n    <xs:sequence>\n      <xs:group ref=\"Node\" \
         minOccurs=\"0\" maxOccurs=\"unbounded\"/>\n",
        "<xs:group name=\"Node\">\n    <xs:choice>\n      <xs:element name=\"leaf\" \
         type=\"xs:integer\"/>\n      <xs:element name=\"group\">\n",
        "<xs:group ref=\"Node\" minOccurs=\"1\" maxOccurs=\"unbounded\"/>",
    ] {
        assert!(text.contains(wanted), "{wanted} not in {text}");
    }
}

#[test]
fn a_type_given_tells_why_what_it_refers_to_is_left_out() {
    let folder = folder();
    let path = folder.path();

    fs::write(
        path.join("holder.asn"),
        "Holder DEFINITIONS ::= BEGIN
    Outer ::= SEQUENCE { holder Holder }
    Holder ::= SEQUENCE { open Open }
    Open ::= SEQUENCE { kind INTEGER, value [0] ANY }
END
",
    )
    .unwrap();

    for format in ["json-schema", "xsd"] {
        assert_eq!(
            error(
                path,
                &[
                    "schema",
                    "--format",
                    format,
                    "--skip-unsupported",
                    "-t",
                    "Outer",
                    "holder.asn"
                ]
            ),
            "error: Outer refers to Holder, which refers to Open, which a schema can not tell: \
             Open.value: an ANY or an open type, of which the codec needs the type, which no \
             schema has.\n"
        );
    }
}

#[test]
fn what_a_schema_can_not_tell_is_refused() {
    let folder = folder();
    let path = folder.path();

    for format in ["json-schema", "xsd", "dtd"] {
        assert_eq!(
            error(path, &["schema", "--format", format, "open.asn"]),
            "error: Open.value: an ANY or an open type, of which the codec needs the type, which \
             no schema has.\n"
        );

        // Or left out, with a warning.
        let output = run(
            path,
            &[
                "schema",
                "--format",
                format,
                "--skip-unsupported",
                "open.asn",
            ],
        );
        let text = String::from_utf8(output.stdout).unwrap();

        assert!(output.status.success());
        assert!(text.contains("Known"));
        assert!(!text.contains("Open\""));
        assert_eq!(
            String::from_utf8(output.stderr).unwrap(),
            "warning: Open is left out: Open.value: an ANY or an open type, of which the codec \
             needs the type, which no schema has.\n"
        );

        // But not when it is asked for.
        assert!(
            error(
                path,
                &[
                    "schema",
                    "--format",
                    format,
                    "--skip-unsupported",
                    "-t",
                    "Open",
                    "open.asn"
                ]
            )
            .starts_with("error: Open.value: an ANY")
        );
    }

    // A JSON Schema has no names of elements.
    schema(path, &["--format", "json-schema", "clash.asn"]);

    assert_eq!(
        error(path, &["schema", "--format", "dtd", "clash.asn"]),
        "error: B.x: the element <x> is (true|false) here and (#PCDATA) at A.x, which a DTD can \
         not tell apart, as it has one declaration for each name.\n"
    );
    assert!(
        schema(path, &["--format", "dtd", "-t", "B", "clash.asn"])
            .contains("<!ELEMENT x (true|false)>")
    );
}

#[test]
fn an_xml_schema_of_xer() {
    let folder = folder();
    let path = folder.path();
    let text = schema(path, &["--format", "xsd", "pairs.asn"]);

    assert!(text.starts_with(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- schema generated by asnkit from \
         pairs.asn -->\n"
    ));

    for wanted in [
        "<xs:schema xmlns:xs=\"http://www.w3.org/2001/XMLSchema\">\n  <xs:element name=\"Pair\" \
         type=\"Pair\"/>\n",
        // The members of a SET in any order.
        "<xs:complexType name=\"Pair\">\n    <xs:all>\n      <xs:element name=\"left\">\n",
        "<xs:union>\n            <xs:simpleType>\n              <xs:restriction \
         base=\"xs:integer\">\n                <xs:minInclusive value=\"-5\"/>\n                \
         <xs:maxInclusive value=\"5\"/>\n",
        "<xs:element name=\"right\" minOccurs=\"0\">\n        <xs:complexType>\n          \
         <xs:choice>\n            <xs:element name=\"true\" type=\"_Empty\"/>\n",
        "<xs:element name=\"Pair\" type=\"Pair\" minOccurs=\"2\" maxOccurs=\"4\"/>",
        "<xs:restriction base=\"xs:hexBinary\">\n      <xs:length value=\"16\"/>",
        "<xs:pattern value=\"[&lt;a-z]*\"/>",
        // The items that are BOOLEAN have no element of their own.
        "<xs:complexType name=\"Flags\">\n    <xs:sequence>\n      <xs:choice minOccurs=\"0\" \
         maxOccurs=\"unbounded\">\n        <xs:element name=\"true\" type=\"_Empty\"/>\n",
        "<xs:complexType name=\"_Empty\"/>\n</xs:schema>\n",
    ] {
        assert!(text.contains(wanted), "{wanted} not in {text}");
    }

    // The elements of a member are its own: what a DTD can not tell.
    let clash = schema(path, &["--format", "xsd", "clash.asn"]);

    assert!(clash.contains("<xs:element name=\"x\" type=\"xs:integer\"/>"));
    assert!(
        clash.contains("<xs:element name=\"x\">\n        <xs:complexType>\n          <xs:choice>")
    );

    schema(path, &["-o", "pairs.xsd", "pairs.asn"]);

    assert!(
        fs::read_to_string(path.join("pairs.xsd"))
            .unwrap()
            .contains("<xs:all>")
    );
    assert_eq!(
        error(path, &["schema", "--format", "xsd", "open.asn"]),
        "error: Open.value: an ANY or an open type, of which the codec needs the type, which \
         no schema has.\n"
    );
}

#[test]
fn the_comment_is_what_made_it_or_the_pragma_schema_comment() {
    let folder = folder();
    let path = folder.path();
    let text = schema(
        path,
        &["--format", "json-schema", "shapes.asn", "pairs.asn"],
    );

    assert!(
        text.contains("\"$comment\": \"schema generated by asnkit from shapes.asn, pairs.asn\"")
    );

    fs::write(
        path.join("greeter.asn"),
        "--@ schema_comment = Greeter v1\n\
         Greeter DEFINITIONS ::= BEGIN\nHello ::= INTEGER\nEND\n\
         --@ schema_comment = Other\n\
         Other DEFINITIONS ::= BEGIN\nBye ::= BOOLEAN\nEND\n\
         Plain DEFINITIONS ::= BEGIN\nNone ::= BOOLEAN\nEND\n",
    )
    .unwrap();

    // Those of the modules of the types, a line each.
    let text = schema(path, &["--format", "json-schema", "greeter.asn"]);
    assert!(
        text.contains("\"$comment\": \"Greeter v1\\nOther\""),
        "{text}"
    );

    let text = schema(
        path,
        &["--format", "json-schema", "-t", "Bye", "greeter.asn"],
    );
    assert!(text.contains("\"$comment\": \"Other\""), "{text}");

    let text = schema(
        path,
        &["--format", "json-schema", "-t", "None", "greeter.asn"],
    );
    assert!(text.contains("\"$comment\": \"schema generated by asnkit from greeter.asn\""));

    let text = schema(path, &["--format", "dtd", "-t", "Hello", "greeter.asn"]);
    assert!(text.starts_with("<!-- Greeter v1 -->\n"), "{text}");

    let text = schema(path, &["--format", "xsd", "-t", "Hello", "greeter.asn"]);
    assert!(text.contains("\n<!-- Greeter v1 -->\n"), "{text}");

    // An .asni file has no pragmas.
    let output = run(path, &["compile", "-o", "greeter.asni", "greeter.asn"]);
    assert!(output.status.success());

    let text = schema(path, &["--format", "dtd", "greeter.asni"]);
    assert!(text.starts_with("<!-- schema generated by asnkit from greeter.asn -->\n"));

    // A comment of XML has no `--`, which the name of a file can have.
    fs::write(
        path.join("a---b.asn"),
        "D DEFINITIONS ::= BEGIN\nA ::= INTEGER\nEND\n",
    )
    .unwrap();

    let text = schema(path, &["--format", "dtd", "a---b.asn"]);
    assert!(
        text.starts_with("<!-- schema generated by asnkit from a- - -b.asn -->\n"),
        "{text}"
    );
}
