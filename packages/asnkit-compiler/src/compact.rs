// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The compact form of a node (see `docs/file_formats.md`): an array of
//! the number of its kind followed by its fields, in the order of the
//! table of kinds, which is read from a copy of the one of the
//! documentation, `kinds.md`, so that there is only one.

use serde_json::{Map, Value as Json};
use std::collections::HashMap;
use std::sync::OnceLock;

const DOCUMENTATION: &str = include_str!("kinds.md");

/// What a field of a kind of node is when it is not given.
#[derive(Debug, Clone)]
struct Field {
    name: String,
    /// `None` if the field has no default.
    default: Option<Json>,
}

#[derive(Debug)]
struct Kind {
    number: u64,
    fields: Vec<Field>,
}

const TYPE_FIELDS: [&str; 11] = [
    "tag",
    "size",
    "restricted_to",
    "permitted_alphabet",
    "with_components",
    "with_component",
    "table",
    "parameters",
    "actual_parameters",
    "module_name",
    "constraints",
];

const CONSTRAINT_FIELDS: [&str; 6] = [
    "size",
    "restricted_to",
    "permitted_alphabet",
    "with_components",
    "with_component",
    "containing",
];

fn parse_default(text: &str) -> Json {
    match text {
        "null" => Json::Null,
        "false" => Json::Bool(false),
        "true" => Json::Bool(true),
        "[]" => Json::Array(vec![]),
        other => panic!("unknown default {other:?} in the table of kinds"),
    }
}

fn kinds() -> &'static HashMap<String, Kind> {
    static KINDS: OnceLock<HashMap<String, Kind>> = OnceLock::new();

    KINDS.get_or_init(|| {
        let table = DOCUMENTATION
            .split("<!-- kinds-begin -->")
            .nth(1)
            .and_then(|rest| rest.split("<!-- kinds-end -->").next())
            .expect("no table of kinds");
        let mut kinds = HashMap::new();

        for line in table.lines() {
            let columns: Vec<&str> = line
                .trim()
                .trim_matches('|')
                .split('|')
                .map(str::trim)
                .collect();

            if columns.len() < 3 || !line.trim_start().starts_with('|') {
                continue;
            }

            let Ok(number) = columns[0].parse::<u64>() else {
                continue;
            };
            let name = columns[1].to_string();
            let mut fields = Vec::new();
            let rest = columns[2];

            for item in rest.split(", ") {
                let item = item.trim();

                if item == "--" {
                    continue;
                } else if item == "*type fields*" {
                    for name in TYPE_FIELDS {
                        fields.push(Field {
                            name: name.to_string(),
                            default: Some(Json::Null),
                        });
                    }
                } else if item == "*constraint fields*" {
                    for name in CONSTRAINT_FIELDS {
                        fields.push(Field {
                            name: name.to_string(),
                            default: Some(Json::Null),
                        });
                    }
                } else if let Some((name, default)) = item.split_once(" = ") {
                    fields.push(Field {
                        name: name.to_string(),
                        default: Some(parse_default(default.trim())),
                    });
                } else {
                    fields.push(Field {
                        name: item.to_string(),
                        default: None,
                    });
                }
            }

            kinds.insert(name, Kind { number, fields });
        }

        kinds
    })
}

fn is_default(field: &Field, value: &Json) -> bool {
    if value.is_null() {
        return true;
    }

    match &field.default {
        None => false,
        Some(default) => value == default,
    }
}

/// A node, in the readable form, in the compact form.
pub fn to_compact(node: &Json) -> Json {
    match node {
        Json::Object(map) => match map.get("kind").and_then(Json::as_str) {
            Some(name) => {
                let kind = kinds()
                    .get(name)
                    .unwrap_or_else(|| panic!("no kind of node {name}"));
                let mut compact = vec![Json::from(kind.number)];

                for field in &kind.fields {
                    match map.get(&field.name) {
                        Some(value) if !is_default(field, value) => compact.push(to_compact(value)),
                        _ => compact.push(Json::Null),
                    }
                }

                while compact.last() == Some(&Json::Null) {
                    compact.pop();
                }

                Json::Array(compact)
            }
            None => {
                let mut converted = Map::new();

                for (key, value) in map {
                    converted.insert(key.clone(), to_compact(value));
                }

                Json::Object(converted)
            }
        },
        // A list starts with 0, which no kind of node has.
        Json::Array(items) => {
            let mut compact = vec![Json::from(0)];
            compact.extend(items.iter().map(to_compact));

            Json::Array(compact)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(text: &str) -> &str {
        text.split("<!-- kinds-begin -->")
            .nth(1)
            .and_then(|rest| rest.split("<!-- kinds-end -->").next())
            .unwrap()
    }

    /// The copy in `kinds.md` is the table of the specification. It is
    /// read when the test runs, as the published crate does not have
    /// the documentation, and the test is skipped there.
    #[test]
    fn same_as_the_specification() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/file_formats.md");
        let Ok(specification) = std::fs::read_to_string(path) else {
            return;
        };

        assert_eq!(
            table(DOCUMENTATION),
            table(&specification),
            "copy the table of docs/file_formats.md to src/kinds.md"
        );
    }

    #[test]
    fn the_table_has_all_kinds() {
        let kinds = kinds();

        assert_eq!(kinds.len(), 67);
        assert_eq!(kinds["CompiledSpecification"].number, 1);
        assert_eq!(kinds["Parameter"].number, 52);
        assert_eq!(kinds["UnsupportedConstraint"].number, 67);
        assert_eq!(kinds["BuiltinType"].fields.len(), 12);
        assert_eq!(kinds["ComponentConstraint"].fields.len(), 8);
        assert_eq!(kinds["Module"].fields.len(), 10);
    }

    #[test]
    fn a_node_is_an_array_of_its_kind_and_its_fields() {
        let tree: Json =
            serde_json::from_str(r#"{"kind": "Range", "lower": 1, "upper": 5}"#).unwrap();

        assert_eq!(to_compact(&tree).to_string(), "[6,1,5]");

        let tree: Json = serde_json::from_str(
            r#"{"kind": "Member", "name": "b", "optional": true,
                "type": {"kind": "BuiltinType", "name": "BOOLEAN"}}"#,
        )
        .unwrap();

        assert_eq!(
            to_compact(&tree).to_string(),
            r#"[4,"b",[10,"BOOLEAN"],true]"#
        );
    }

    #[test]
    fn defaults_are_left_out_and_lists_start_with_zero() {
        let tree: Json = serde_json::from_str(
            r#"{"kind": "CompiledModule", "name": "Foo", "types": [], "object_classes": []}"#,
        )
        .unwrap();

        assert_eq!(to_compact(&tree).to_string(), r#"[2,"Foo"]"#);
    }
}
