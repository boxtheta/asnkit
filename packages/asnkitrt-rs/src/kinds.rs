// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The kinds of nodes of `docs/file_formats.md`: the number of each and its
//! fields, in the order of the compact form. A test compares this table with
//! the one in the specification, so that they do not drift apart.

/// The number, the name and the fields of a kind. The fields are as the
/// specification writes them, with `*type fields*` and `*constraint fields*`
/// standing for the fields all types and all constraints have.
pub const KINDS: [(u8, &str, &str); 67] = [
    (1, "CompiledSpecification", "modules"),
    (2, "CompiledModule", "name, types = [], object_classes = []"),
    (3, "TypeAssignment", "name, type, location = null"),
    (
        4,
        "Member",
        "name, type, optional = false, default = null, location = null",
    ),
    (5, "Tag", "number, tag_class = null, mode = null"),
    (6, "Range", "lower, upper"),
    (7, "NamedNumber", "name, number"),
    (8, "ExtensionMarker", ""),
    (9, "ExtensionAdditionGroup", "members"),
    (10, "BuiltinType", "name, *type fields*"),
    (11, "ReferencedType", "name, *type fields*"),
    (12, "IntegerType", "named_numbers = null, *type fields*"),
    (13, "BitStringType", "named_bits = null, *type fields*"),
    (14, "EnumeratedType", "values, *type fields*"),
    (15, "SequenceType", "members, *type fields*"),
    (16, "SetType", "members, *type fields*"),
    (17, "ChoiceType", "members, *type fields*"),
    (
        18,
        "SequenceOfType",
        "element, element_name = null, *type fields*",
    ),
    (
        19,
        "SetOfType",
        "element, element_name = null, *type fields*",
    ),
    (20, "Min", ""),
    (21, "Max", ""),
    (22, "Identifier", "name"),
    (23, "HexString", "digits"),
    (24, "BinaryString", "bits"),
    (25, "Null", ""),
    (26, "PlusInfinity", ""),
    (27, "MinusInfinity", ""),
    (28, "NotANumber", ""),
    (29, "NamedValue", "name, value"),
    (30, "SequenceValue", "members"),
    (31, "UnsupportedValue", ""),
    (32, "Excluded", "value"),
    (33, "TableConstraint", "object_set, component_ids = null"),
    (
        34,
        "ComponentConstraint",
        "name, presence = null, *constraint fields*",
    ),
    (35, "ElementConstraint", "*constraint fields*"),
    (36, "AnyDefinedByType", "value, choices = [], *type fields*"),
    (37, "AnyDefinedByChoice", "value, type"),
    (38, "ComponentsOf", "type_name"),
    (
        39,
        "ObjectClassAssignment",
        "name, fields, syntax = null, location = null",
    ),
    (40, "ObjectClassField", "name, type, location = null"),
    (41, "SyntaxTree", "modules"),
    (
        42,
        "Module",
        "name, tags = null, extensibility_implied = false, imports = [], types = [], values = [], objects = [], object_classes = [], object_sets = [], location = null",
    ),
    (43, "Import", "module, symbols"),
    (44, "ValueAssignment", "name, type, value, location = null"),
    (
        45,
        "ObjectAssignment",
        "name, class_name, object, location = null",
    ),
    (
        46,
        "ObjectSetAssignment",
        "name, class_name, members, location = null",
    ),
    (47, "Object", "fields"),
    (48, "ObjectField", "name, setting"),
    (49, "UnresolvedObject", "source"),
    (50, "ObjectSet", "members"),
    (51, "OptionalGroup", "tokens"),
    (52, "Parameter", "name, governor = null"),
    (
        53,
        "ElementSetSpecs",
        "root, extensible = false, additions = null",
    ),
    (54, "Union", "elements"),
    (55, "Intersection", "elements"),
    (56, "Except", "elements, exclusions"),
    (57, "All", "exclusions"),
    (58, "SizeConstraint", "constraint"),
    (59, "PermittedAlphabet", "constraint"),
    (60, "ContainedSubtype", "type, includes = false"),
    (61, "PatternConstraint", "value"),
    (62, "InnerTypeConstraint", "constraint"),
    (63, "InnerTypeConstraints", "partial = false, components"),
    (
        64,
        "NamedConstraint",
        "name, constraints = null, presence = null",
    ),
    (65, "ContentsConstraint", "type = null, encoded_by = null"),
    (66, "UserDefinedConstraint", ""),
    (67, "UnsupportedConstraint", ""),
];

/// The number of the kind with this name.
pub const fn number(name: &str) -> u8 {
    let mut index = 0;

    while index < KINDS.len() {
        if const_eq(KINDS[index].1, name) {
            return KINDS[index].0;
        }

        index += 1;
    }

    panic!("unknown kind");
}

const fn const_eq(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());

    if a.len() != b.len() {
        return false;
    }

    let mut index = 0;

    while index < a.len() {
        if a[index] != b[index] {
            return false;
        }

        index += 1;
    }

    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::vec::Vec;

    /// The table of the specification is the table here.
    #[test]
    fn same_as_the_specification() {
        let text = include_str!("../../../docs/file_formats.md");
        let table = text
            .split("<!-- kinds-begin -->")
            .nth(1)
            .unwrap()
            .split("<!-- kinds-end -->")
            .next()
            .unwrap();
        let mut found = 0;

        for line in table.lines() {
            let cells: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();

            if !line.trim_start().starts_with('|') || cells.len() < 3 {
                continue;
            }

            let Ok(number) = cells[0].parse::<u8>() else {
                continue;
            };
            let (name, fields) = (cells[1], cells[2]);
            let fields = if fields == "--" { "" } else { fields };

            assert_eq!(KINDS[found], (number, name, fields));
            found += 1;
        }

        assert_eq!(found, KINDS.len());
    }
}
