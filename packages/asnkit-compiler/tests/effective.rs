// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The effective constraints a compiled specification has: the root of
//! the range, the size and the permitted alphabet of each type, as X.680
//! 50 and 51 and X.691 10.3 say, followed by an extension marker if it is
//! extensible.

use asnkit_compiler::{Input, compile};
use serde_json::{Value, json};

/// The size, the range and the permitted alphabet of a type of module
/// `Foo`, in the compact form, which are `null` if it has none of its
/// own.
fn constraints_of(definitions: &str, name: &str) -> (Value, Value, Value) {
    let text = format!("Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n{definitions}\nEND\n");
    let compiled = compile(&Input::String(text)).unwrap();
    let types = compiled[1][1][2].as_array().unwrap();
    let type_ = types
        .iter()
        .skip(1)
        .find(|assignment| assignment[1] == name)
        .map(|assignment| &assignment[2])
        .unwrap();
    // The fields before the type fields (see docs/file_formats.md).
    let first = match type_[0].as_i64().unwrap() {
        10..=12 => 2,
        18 | 19 => 3,
        kind => panic!("A type of kind {kind}."),
    };
    let field = |index: usize| type_.get(first + index).cloned().unwrap_or(Value::Null);

    (field(1), field(2), field(3))
}

fn range_of(definitions: &str, name: &str) -> Value {
    let (size, range, alphabet) = constraints_of(definitions, name);

    assert_eq!((size, alphabet), (Value::Null, Value::Null));

    range
}

fn size_of(definitions: &str, name: &str) -> Value {
    constraints_of(definitions, name).0
}

fn alphabet_of(definitions: &str, name: &str) -> Value {
    constraints_of(definitions, name).2
}

const MARKER: [i64; 1] = [8];

#[test]
fn a_union_has_the_values_of_each() {
    let spec = "A ::= INTEGER (1..2 | 4..5)\n\
                B ::= INTEGER (1 | 3 | 2)\n\
                C ::= INTEGER (1..5 UNION 3..9)";

    assert_eq!(range_of(spec, "A"), json!([0, [6, 1, 2], [6, 4, 5]]));
    // Values next to each other are a range.
    assert_eq!(range_of(spec, "B"), json!([0, [6, 1, 3]]));
    assert_eq!(range_of(spec, "C"), json!([0, [6, 1, 9]]));
}

#[test]
fn an_intersection_has_the_values_of_all() {
    let spec = "A ::= INTEGER ((1..20) ^ (5..30))\n\
                B ::= IA5String (SIZE (1..4) INTERSECTION FROM (\"ABCD\"))";

    assert_eq!(range_of(spec, "A"), json!([0, [6, 5, 20]]));
    assert_eq!(size_of(spec, "B"), json!([0, [6, 1, 4]]));
    assert_eq!(alphabet_of(spec, "B"), json!([0, [6, "A", "D"]]));
}

#[test]
fn what_follows_except_is_left_out() {
    // X.691 10.3.21.
    let spec = "A ::= INTEGER (1..10 EXCEPT 5)\n\
                B ::= INTEGER (ALL EXCEPT 5)";

    assert_eq!(range_of(spec, "A"), json!([0, [6, 1, 10]]));
    assert_eq!(range_of(spec, "B"), Value::Null);
}

#[test]
fn a_constraint_applied_after_another_is_applied_to_its_root() {
    // X.680 50.11: the result is extensible if the last one is.
    let spec = "A ::= INTEGER (1..32)(1..16)\n\
                N ::= INTEGER (1..32, ..., 33..128)\n\
                O ::= N (1..16)\n\
                P ::= N (1..16, ...)";

    assert_eq!(range_of(spec, "A"), json!([0, [6, 1, 16]]));
    assert_eq!(range_of(spec, "N"), json!([0, [6, 1, 32], MARKER]));
    assert_eq!(range_of(spec, "O"), json!([0, [6, 1, 16]]));
    assert_eq!(range_of(spec, "P"), json!([0, [6, 1, 16], MARKER]));
}

#[test]
fn a_general_constraint_leaves_the_constraints_as_they_are() {
    // X.680 50.11 NOTE: a contents constraint does not take away that
    // the size is extensible, which a pattern constraint does.
    let spec = "S ::= IA5String (SIZE (1..4, ...))\n\
                C ::= OCTET STRING (SIZE (1..4, ...))\n\
                D ::= C (CONTAINING INTEGER)\n\
                P ::= S (PATTERN \"a*\")";

    assert_eq!(size_of(spec, "D"), Value::Null);
    assert_eq!(size_of(spec, "P"), json!([0, [6, 1, 4]]));
}

#[test]
fn a_contained_subtype_gives_the_root_of_its_type() {
    // X.680 51.3.3.
    let spec = "N ::= INTEGER (1..32, ..., 33..128)\n\
                A ::= INTEGER (N)\n\
                B ::= INTEGER (N | 40..50)";

    assert_eq!(range_of(spec, "A"), json!([0, [6, 1, 32]]));
    assert_eq!(range_of(spec, "B"), json!([0, [6, 1, 32], [6, 40, 50]]));
}

#[test]
fn an_extension_marker_makes_each_constraint_extensible() {
    // X.691 B.2.3.8.
    let spec = "A ::= IA5String (SIZE (1..4), ...)\n\
                B ::= IA5String (SIZE (1..4) ^ FROM (\"ab\"), ...)";

    assert_eq!(size_of(spec, "A"), json!([0, [6, 1, 4], MARKER]));
    assert_eq!(size_of(spec, "B"), json!([0, [6, 1, 4], MARKER]));
    // An extensible permitted alphabet is not visible (X.691 10.3.11).
    assert_eq!(alphabet_of(spec, "B"), Value::Null);
}

#[test]
fn an_extensible_permitted_alphabet_is_none() {
    // X.691 10.3.20 NOTE 2.
    let spec = "A ::= IA5String (SIZE (1..4))(FROM (\"ABCD\", ...))\n\
                B ::= IA5String (A)\n\
                U ::= IA5String (FROM (\"AB\"))\n\
                V ::= U (FROM (\"A\", ...))";

    assert_eq!(size_of(spec, "A"), json!([0, [6, 1, 4]]));
    assert_eq!(alphabet_of(spec, "A"), Value::Null);
    assert_eq!(size_of(spec, "B"), json!([0, [6, 1, 4]]));
    assert_eq!(alphabet_of(spec, "B"), Value::Null);
    // The permitted alphabet of the type it refers to, which it takes.
    assert_eq!(alphabet_of(spec, "U"), json!([0, [6, "A", "B"]]));
    assert_eq!(alphabet_of(spec, "V"), Value::Null);
}

#[test]
fn a_union_with_a_part_that_is_not_visible_is_not_visible() {
    // X.691 10.3.17 and 10.3.21.
    let spec = "A ::= IA5String (SIZE (1..4) | \"abcdefgh\")\n\
                B ::= IA5String (SIZE (1..4) ^ \"abc\")";

    assert_eq!(size_of(spec, "A"), Value::Null);
    assert_eq!(size_of(spec, "B"), json!([0, [6, 1, 4]]));
}

#[test]
fn a_size_is_not_less_than_zero() {
    // X.680 51.5.3.
    let spec = "A ::= OCTET STRING (SIZE (MIN..4))\n\
                B ::= SEQUENCE (SIZE (0<..2)) OF INTEGER";

    assert_eq!(size_of(spec, "A"), json!([0, [6, 0, 4]]));
    assert_eq!(size_of(spec, "B"), json!([0, [6, 1, 2]]));
}

#[test]
fn bounds_given_by_name() {
    let spec = "maxLen INTEGER ::= 9\n\
                A ::= OCTET STRING (SIZE (1..maxLen))\n\
                B ::= INTEGER { low(2), high(8) } (low..high)\n\
                C ::= INTEGER (MIN..maxLen)";

    assert_eq!(size_of(spec, "A"), json!([0, [6, 1, 9]]));
    assert_eq!(range_of(spec, "B"), json!([0, [6, 2, 8]]));
    assert_eq!(range_of(spec, "C"), json!([0, [6, [20], 9]]));
}

#[test]
fn constraints_of_a_parameterized_type() {
    let spec = "P { INTEGER: n } ::= INTEGER (0..n)\n\
                Q ::= P { 7 }\n\
                R { T } ::= SEQUENCE (SIZE (1..4)) OF T\n\
                S ::= R { INTEGER (0..3) }";

    assert_eq!(range_of(spec, "Q"), json!([0, [6, 0, 7]]));
    assert_eq!(size_of(spec, "S"), json!([0, [6, 1, 4]]));
}

#[test]
fn a_permitted_alphabet_as_ranges() {
    let spec = "A ::= IA5String (FROM (\"A\"..\"C\" | \"x\"))\n\
                B ::= IA5String (FROM (\"cab\"))";

    assert_eq!(
        alphabet_of(spec, "A"),
        json!([0, [6, "A", "C"], [6, "x", "x"]])
    );
    assert_eq!(alphabet_of(spec, "B"), json!([0, [6, "a", "c"]]));

    // Characters are not surrogates, and none is greater than U+10FFFF.
    let spec = "C ::= UTF8String (FROM (\"a\"..MAX))\n\
                D ::= UTF8String (FROM (\"\u{D7FF}\"..<\"\u{E001}\"))";

    assert_eq!(
        alphabet_of(spec, "C"),
        json!([0, [6, "a", "\u{D7FF}"], [6, "\u{E000}", "\u{10FFFF}"]])
    );
    assert_eq!(
        alphabet_of(spec, "D"),
        json!([0, [6, "\u{D7FF}", "\u{D7FF}"], [6, "\u{E000}", "\u{E000}"]])
    );
}

#[test]
fn a_bound_can_be_of_any_size() {
    let spec = "A ::= INTEGER (0..1361129467683753853853498429727072845824)\n\
                B ::= INTEGER (-1361129467683753853853498429727072845824<..<0)\n\
                C ::= INTEGER (170141183460469231731687303715884105727..MAX)\n\
                D ::= INTEGER (MIN..10)";

    let text = |name| range_of(spec, name).to_string();

    assert_eq!(
        text("A"),
        "[0,[6,0,1361129467683753853853498429727072845824]]"
    );
    assert_eq!(
        text("B"),
        "[0,[6,-1361129467683753853853498429727072845823,-1]]"
    );
    // The greatest i128 is a bound, not MAX.
    assert_eq!(
        text("C"),
        "[0,[6,170141183460469231731687303715884105727,[21]]]"
    );
    assert_eq!(text("D"), "[0,[6,[20],10]]");
}
