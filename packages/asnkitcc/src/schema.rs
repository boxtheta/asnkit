// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! What the values of the types of a specification are in a text codec:
//! a JSON Schema of JER (X.697), or an XML Schema or a DTD of BASIC-XER
//! (X.693), as the runtimes write them. The command `asnkitcc schema`
//! writes them, and the compiler as WebAssembly
//! (`packages/asnkit-compiler-wasm`) makes them too.
//!
//! ```text
//! asnkitcc schema --format json-schema -t Question -o question.schema.json foo.asn
//! asnkitcc schema -o foo.xsd foo.asni
//! asnkitcc schema -o foo.dtd foo.asni
//! ```
//!
//! What a schema can not tell is refused, with where in the types it is:
//! an ANY and an open type, of which JER and XER need the actual type,
//! in an XML Schema, root elements of one name in two modules, and, in a
//! DTD, which has one declaration for each name of an element, two
//! elements of the same name with other contents.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use asnkitrt::container::{self, Checks};
use asnkitrt::node::Const;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::{Codec, Constraints, Kind, MemberInfo, Members, StrKind, TypeId, Types};

/// How deep types are nested in each other, at most, as the codecs have it.
const MAX_DEPTH: usize = 256;

pub const FORMATS: &str = "json-schema, xsd or dtd";

/// What a schema is written as.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// A JSON Schema, of the draft 2020-12, of JER.
    JsonSchema,
    /// An XML Schema (XSD 1.0) of BASIC-XER.
    Xsd,
    /// A DTD of BASIC-XER.
    Dtd,
}

/// The format of a name, as `dtd` or `json-schema`.
pub fn format_of(name: &str) -> Result<Format, String> {
    match name.to_ascii_lowercase().as_str() {
        "json-schema" | "jsonschema" | "json" | "jer" => Ok(Format::JsonSchema),
        "xsd" | "xml-schema" | "xer" => Ok(Format::Xsd),
        "dtd" => Ok(Format::Dtd),
        _ => Err(format!("unknown format '{name}', expected {FORMATS}.")),
    }
}

/// The format a file is in, told by its extension.
pub fn format_of_path(path: &Path) -> Option<Format> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "json" => Some(Format::JsonSchema),
        "xsd" => Some(Format::Xsd),
        "dtd" => Some(Format::Dtd),
        _ => None,
    }
}

/// What schema to make.
pub struct Request<'a> {
    pub format: Format,
    /// `Type` or `Module.Type`: the types of which the values are the
    /// documents. All of the specification if there are none.
    pub types: &'a [String],
    /// Leave out what can not be told, with why, rather than fail.
    pub skip_unsupported: bool,
    /// The contents of the `.asni` file.
    pub asni: &'a [u8],
    /// What makes the schema, as `asnkitcc 2.0.0`, which it says.
    pub generator: &'a str,
}

/// A schema, as text, and what it leaves out.
pub struct Schema {
    pub text: String,
    /// How many types it is of.
    pub types: usize,
    /// The types that are left out, by their names in the schema, with
    /// why, when `skip_unsupported` is asked for.
    pub skipped: Vec<(String, String)>,
}

impl Format {
    /// Its name, as `JSON Schema`.
    pub fn name(self) -> &'static str {
        match self {
            Format::JsonSchema => "JSON Schema",
            Format::Xsd => "XML Schema",
            Format::Dtd => "DTD",
        }
    }
}

/// A type that a document can be a value of, by the name it is given in
/// the schema, and the name of its assignment.
struct Root {
    key: String,
    name: String,
    id: TypeId,
    /// It was asked for, and is not to be left out.
    given: bool,
}

/// Makes the schema of the types of an `.asni` file. An error tells why,
/// without `error: `, with where in the types for what a schema can not
/// tell.
pub fn generate(request: &Request<'_>) -> Result<Schema, String> {
    let asni = request.asni;
    let checks = Checks::default();
    let sources: Vec<String> = container::read(asni, &checks)
        .map_err(|error| format!("the specification: {error}"))?
        .header
        .metadata
        .sources
        .iter()
        .filter_map(|source| source.name.clone())
        .collect();
    let codec = match request.format {
        Format::JsonSchema => Codec::Jer,
        Format::Xsd | Format::Dtd => Codec::Xer,
    };
    let specification = Specification::from_asni(asni, &checks, Options::new(codec))
        .map_err(|error| format!("the specification: {error}"))?;
    let types = specification.types();
    let roots = roots(types, request.types)?;
    let about = describe(&sources);
    let mut skipped = Vec::new();

    let text = match request.format {
        Format::JsonSchema => json_schema(types, &roots, request, &about, &mut skipped)?,
        Format::Xsd => xml_schema(types, &roots, request, &about, &mut skipped)?,
        Format::Dtd => dtd(types, &roots, request, &about, &mut skipped)?,
    };

    Ok(Schema {
        text,
        types: roots.len() - skipped.len(),
        skipped,
    })
}

/// What the schema is of, as `foo.asn`.
fn describe(sources: &[String]) -> String {
    let names: Vec<&str> = sources
        .iter()
        .map(|source| {
            Path::new(source)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(source)
        })
        .collect();

    match names.as_slice() {
        [] => "a specification".to_string(),
        names => names.join(", "),
    }
}

/// The name of a type in the schema: its own, or with its module if
/// another module has a type of that name.
fn key_of(types: &Types, module: &str, name: &str, id: TypeId) -> String {
    if types.find_unique(name) == Some(id) {
        name.to_string()
    } else {
        format!("{module}.{name}")
    }
}

fn roots(types: &Types, given: &[String]) -> Result<Vec<Root>, String> {
    let mut roots = Vec::new();

    if given.is_empty() {
        for (module, assignments) in &types.modules {
            for (name, id) in assignments {
                // The first of a name in a module is the one that is found.
                if types.find(module, name) == Some(*id) {
                    roots.push(Root {
                        key: key_of(types, module, name, *id),
                        name: name.clone(),
                        id: *id,
                        given: false,
                    });
                }
            }
        }

        return Ok(roots);
    }

    for text in given {
        let (id, module, name) = match text.split_once('.') {
            Some((module, name)) => (types.find(module, name), module.to_string(), name),
            None => {
                let id = types.find_unique(text);
                let module = types
                    .modules
                    .iter()
                    .find(|(_, assignments)| {
                        assignments.iter().any(|(_, other)| Some(*other) == id)
                    })
                    .map(|(module, _)| module.clone())
                    .unwrap_or_default();

                (id, module, text.as_str())
            }
        };
        let id = id.ok_or_else(|| {
            if text.contains('.') {
                format!("no type '{text}' in the specification.")
            } else {
                format!(
                    "no type '{text}' in the specification, or it is in more than one \
                     module: give it as Module.{text}."
                )
            }
        })?;

        if !roots.iter().any(|root: &Root| root.id == id) {
            roots.push(Root {
                key: key_of(types, &module, name, id),
                name: name.to_string(),
                id,
                given: true,
            });
        }
    }

    Ok(roots)
}

/// The least and the greatest of each of the values and ranges of a
/// size or a range that is not extensible, as text. None if it is not
/// one or is extensible, as any value is then in it.
type Bounds = Vec<(Option<String>, Option<String>)>;

fn size_bounds(constraints: &Constraints, factor: i128) -> Bounds {
    let Some(size) = constraints.size.filter(|size| !size.extensible) else {
        return Vec::new();
    };
    let scale = |bound: Option<i128>| bound.map(|bound| (bound * factor).to_string());

    constraints
        .size_parts
        .clone()
        .unwrap_or_else(|| vec![(size.min, size.max)])
        .into_iter()
        .map(|(min, max)| (scale(min), scale(max)))
        .collect()
}

fn range_bounds(constraints: &Constraints) -> Bounds {
    let text = |bound: Option<i128>| bound.map(|bound| bound.to_string());

    if let Some(range) = constraints.range {
        if range.extensible {
            return Vec::new();
        }

        return constraints
            .range_parts
            .clone()
            .unwrap_or_else(|| vec![(range.min, range.max)])
            .into_iter()
            .map(|(min, max)| (text(min), text(max)))
            .collect();
    }

    match &constraints.big_range {
        Some(big) if !big.extensible => {
            let text = |bound: &Option<_>| bound.as_ref().map(ToString::to_string);

            if big.parts.is_empty() {
                vec![(text(&big.min), text(&big.max))]
            } else {
                big.parts
                    .iter()
                    .map(|(min, max)| (text(min), text(max)))
                    .collect()
            }
        }
        _ => Vec::new(),
    }
}

/// The names of the control characters in an `xmlcstring`, by their
/// code (X.680 12.15.5), as XER writes them. Tab, line feed and carriage
/// return have none, as they are written as they are.
const CONTROLS: [&str; 32] = [
    "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "", "", "vt", "ff", "", "so",
    "si", "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb", "can", "em", "sub", "esc", "is4",
    "is3", "is2", "is1",
];

// JSON Schema.

/// JSON, as a schema needs it, of which an object keeps its order.
#[derive(Debug, Clone, PartialEq)]
enum Json {
    Bool(bool),
    /// The digits of an integer of any size.
    Number(String),
    Str(String),
    Array(Vec<Json>),
    Object(Vec<(String, Json)>),
}

fn text(value: &str) -> Json {
    Json::Str(value.to_string())
}

fn number(value: impl ToString) -> Json {
    Json::Number(value.to_string())
}

fn object<const N: usize>(members: [(&str, Json); N]) -> Json {
    Json::Object(
        members
            .into_iter()
            .map(|(name, value)| (name.to_string(), value))
            .collect(),
    )
}

fn reference_to(key: &str) -> Json {
    object([("$ref", Json::Str(format!("#/$defs/{key}")))])
}

impl Json {
    fn push(&mut self, name: &str, value: Json) {
        if let Json::Object(members) = self {
            members.push((name.to_string(), value));
        }
    }

    /// Adds the bounds as `minimum` and `maximum`, or the names given, or
    /// as an `anyOf` of them if there are more than one.
    fn bound(&mut self, bounds: &Bounds, minimum: &str, maximum: &str) {
        let one = |(min, max): &(Option<String>, Option<String>)| {
            let mut part = Json::Object(Vec::new());

            if let Some(min) = min {
                part.push(minimum, Json::Number(min.clone()));
            }

            if let Some(max) = max {
                part.push(maximum, Json::Number(max.clone()));
            }

            part
        };

        match bounds.as_slice() {
            [] => {}
            [only] => {
                if let (Json::Object(members), Json::Object(more)) = (&mut *self, one(only)) {
                    members.extend(more);
                }
            }
            parts => self.push("anyOf", Json::Array(parts.iter().map(one).collect())),
        }
    }

    fn write(&self, out: &mut String, level: usize) {
        let indent = |out: &mut String, level: usize| out.extend(std::iter::repeat_n("  ", level));

        match self {
            Json::Bool(value) => out.push_str(if *value { "true" } else { "false" }),
            Json::Number(digits) => out.push_str(digits),
            Json::Str(value) => write_string(out, value),
            Json::Array(items)
                if items
                    .iter()
                    .all(|item| !matches!(item, Json::Array(_) | Json::Object(_))) =>
            {
                out.push('[');

                for (index, item) in items.iter().enumerate() {
                    if index > 0 {
                        out.push_str(", ");
                    }

                    item.write(out, level);
                }

                out.push(']');
            }
            Json::Array(items) => {
                out.push('[');

                for (index, item) in items.iter().enumerate() {
                    out.push_str(if index > 0 { ",\n" } else { "\n" });
                    indent(out, level + 1);
                    item.write(out, level + 1);
                }

                out.push('\n');
                indent(out, level);
                out.push(']');
            }
            Json::Object(members) if members.is_empty() => out.push_str("{}"),
            Json::Object(members) => {
                out.push('{');

                for (index, (name, value)) in members.iter().enumerate() {
                    out.push_str(if index > 0 { ",\n" } else { "\n" });
                    indent(out, level + 1);
                    write_string(out, name);
                    out.push_str(": ");
                    value.write(out, level + 1);
                }

                out.push('\n');
                indent(out, level);
                out.push('}');
            }
        }
    }
}

fn write_string(out: &mut String, value: &str) {
    out.push('"');

    for c in value.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            c if (c as u32) < 0x20 || c == '\u{7f}' => {
                out.push_str(&format!("\\u{:04x}", c as u32))
            }
            c => out.push(c),
        }
    }

    out.push('"');
}

/// A character in a class of a regular expression of ECMA-262, as JSON
/// Schema has them.
fn class_char(out: &mut String, c: char) {
    match c {
        '\\' | ']' | '[' | '^' | '-' => {
            out.push('\\');
            out.push(c);
        }
        ' '..='~' => out.push(c),
        c if (c as u32) <= 0xffff => out.push_str(&format!("\\u{:04x}", c as u32)),
        // Beyond the BMP, as it is: `\u` has four digits only, and a pair
        // of surrogates is not one character of a class but in Unicode mode.
        c => out.push(c),
    }
}

/// A class of the characters, with the ranges of those that follow each
/// other.
fn char_class(chars: &[char]) -> String {
    let mut sorted: Vec<char> = chars.to_vec();

    sorted.sort_unstable();
    sorted.dedup();

    let mut out = String::new();
    let mut index = 0;

    while index < sorted.len() {
        let first = sorted[index];
        let mut last = first;

        while index + 1 < sorted.len() && sorted[index + 1] as u32 == last as u32 + 1 {
            index += 1;
            last = sorted[index];
        }

        class_char(&mut out, first);

        if last != first {
            if last as u32 > first as u32 + 1 {
                out.push('-');
            }

            class_char(&mut out, last);
        }

        index += 1;
    }

    out
}

const HEX: &str = "^(?:[0-9A-Fa-f]{2})*$";

/// A number of a DURATION, of which only the last may have a fraction,
/// which this does not tell.
const DURATION_NUMBER: &str = "(?:0|[1-9][0-9]*)(?:[.,][0-9]+)?";

fn duration_pattern() -> String {
    let n = DURATION_NUMBER;

    format!("^P(?:{n}W|(?=.)(?:{n}Y)?(?:{n}M)?(?:{n}D)?(?:T(?=.)(?:{n}H)?(?:{n}M)?(?:{n}S)?)?)$")
}

/// The forms of dates and times the decoders read, as
/// `docs/file_formats.md` tells them.
const UTC_TIME: &str = "^[0-9]{10}(?:[0-9]{2})?(?:Z|[+-][0-9]{4})$";
const GENERALIZED_TIME: &str =
    "^[0-9]{10}(?:[0-9]{2}(?:[0-9]{2})?)?(?:[.,][0-9]+)?(?:Z|[+-][0-9]{2}(?:[0-9]{2})?)?$";
const DATE: &str = "^[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}$";
const TIME_OF_DAY: &str = "^[0-9]{1,2}:[0-9]{1,2}:[0-9]{1,2}$";
const DATE_TIME: &str = "^[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}T[0-9]{1,2}:[0-9]{1,2}:[0-9]{1,2}$";

fn string_pattern(pattern: &str) -> Json {
    object([("type", text("string")), ("pattern", text(pattern))])
}

struct JsonSchema<'a> {
    types: &'a Types,
    /// The name in `$defs` of each type that has one.
    keys: BTreeMap<TypeId, String>,
    /// Those that what is made refers to.
    refs: BTreeSet<String>,
}

impl JsonSchema<'_> {
    /// The schema of a type in another one: a reference to it if it has a
    /// name.
    fn reference(&mut self, id: TypeId, path: &str, depth: usize) -> Result<Json, String> {
        if let Some(key) = self.keys.get(&id) {
            self.refs.insert(key.clone());

            return Ok(reference_to(key));
        }

        self.definition(id, path, depth)
    }

    /// The schema of a type itself.
    fn definition(&mut self, id: TypeId, path: &str, depth: usize) -> Result<Json, String> {
        if depth > MAX_DEPTH {
            return Err(format!("{path}: the types are nested too deeply."));
        }

        // A reference with a tag of its own is the type it refers to, in
        // JER, which has no tags.
        if let Kind::Ref(target) = &self.types.node(id).kind
            && self.types.constraints(id) == self.types.constraints(*target)
        {
            return self.reference(*target, path, depth + 1);
        }

        self.body(id, path, depth)
    }

    fn body(&mut self, id: TypeId, path: &str, depth: usize) -> Result<Json, String> {
        let types = self.types;
        let constraints = types.constraints(id);

        Ok(match types.kind(id) {
            Kind::Ref(_) => return Err(format!("{path}: the type has no contents of its own.")),
            Kind::Boolean => object([("type", text("boolean"))]),
            Kind::Integer(_) => {
                let mut schema = object([("type", text("integer"))]);

                schema.bound(&range_bounds(constraints), "minimum", "maximum");
                schema
            }
            // Any number, and the strings of minus zero, the infinities and
            // NaN (X.697 23.2).
            Kind::Real => object([(
                "anyOf",
                Json::Array(vec![
                    object([("type", text("number"))]),
                    object([(
                        "enum",
                        Json::Array(["-0", "INF", "-INF", "NaN"].map(text).to_vec()),
                    )]),
                ]),
            )]),
            Kind::Null => object([("type", text("null"))]),
            Kind::BitString(_) => match constraints.size {
                // The bytes of the bits, with no length (X.697 24.2).
                Some(size) if size.min.is_some() && size.min == size.max => {
                    let bytes = (size.min.unwrap_or_default() + 7) / 8;

                    string_pattern(&format!("^(?:[0-9A-Fa-f]{{2}}){{{bytes}}}$"))
                }
                _ => {
                    let mut length = object([("type", text("integer")), ("minimum", number(0))]);

                    length.bound(&size_bounds(constraints, 1), "minimum", "maximum");

                    object([
                        ("type", text("object")),
                        (
                            "properties",
                            object([("value", string_pattern(HEX)), ("length", length)]),
                        ),
                        ("required", Json::Array(vec![text("value"), text("length")])),
                        ("additionalProperties", Json::Bool(false)),
                    ])
                }
            },
            Kind::OctetString => {
                let mut schema = string_pattern(HEX);

                schema.bound(&size_bounds(constraints, 2), "minLength", "maxLength");
                schema
            }
            Kind::ObjectIdentifier => string_pattern("^[0-9]+(?:\\.[0-9]+)+$"),
            Kind::Str(kind) => {
                let mut schema = object([("type", text("string"))]);
                let class = match (&constraints.alphabet, kind) {
                    (Some(chars), _) => Some(char_class(chars)),
                    (None, StrKind::Numeric) => Some("0-9 ".to_string()),
                    (None, StrKind::Printable) => Some("A-Za-z0-9 '()+,\\-./:=?".to_string()),
                    (None, StrKind::Ia5) => Some("\\u0000-\\u007f".to_string()),
                    (None, StrKind::Visible) => Some(" -~".to_string()),
                    (None, StrKind::Bmp) => Some("\\u0000-\\ud7ff\\ue000-\\uffff".to_string()),
                    (None, _) => None,
                };

                schema.bound(&size_bounds(constraints, 1), "minLength", "maxLength");

                if let Some(class) = class {
                    schema.push("pattern", Json::Str(format!("^[{class}]*$")));
                }

                schema
            }
            Kind::Enumerated { root, additions } => {
                let names = Json::Array(
                    root.iter()
                        .chain(additions.iter().flatten())
                        .map(|(name, _)| text(name))
                        .collect(),
                );

                // An item of a later version is any other name.
                if additions.is_some() {
                    object([(
                        "anyOf",
                        Json::Array(vec![
                            object([("enum", names)]),
                            object([("type", text("string"))]),
                        ]),
                    )])
                } else {
                    object([("enum", names)])
                }
            }
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                self.members(members, path, depth)?
            }
            Kind::SequenceOf(item) | Kind::SetOf(item) => {
                let mut schema = object([
                    ("type", text("array")),
                    (
                        "items",
                        self.reference(*item, &format!("{path}[]"), depth + 1)?,
                    ),
                ]);

                schema.bound(&size_bounds(constraints, 1), "minItems", "maxItems");
                schema
            }
            Kind::Choice(members) => self.choice(members, path, depth)?,
            Kind::Utc => string_pattern(UTC_TIME),
            Kind::Generalized => string_pattern(GENERALIZED_TIME),
            Kind::Date => string_pattern(DATE),
            Kind::TimeOfDay => string_pattern(TIME_OF_DAY),
            Kind::DateTime => string_pattern(DATE_TIME),
            Kind::Duration => string_pattern(&duration_pattern()),
            kind @ (Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType) => {
                return Err(unsupported(kind, path));
            }
        })
    }

    /// A SEQUENCE, a SET or an EXTERNAL: an object with a member of each
    /// of theirs, in which a member of a later version is any other.
    fn members(&mut self, members: &Members, path: &str, depth: usize) -> Result<Json, String> {
        let mut properties = Vec::new();
        let mut required = Vec::new();
        let mut conditions = Vec::new();

        for member in members.all() {
            let inner = format!("{path}.{}", member.name);
            let schema = match self.types.kind(member.ty) {
                Kind::AnyDefinedBy { value, choices } if !choices.is_empty() => {
                    conditions
                        .extend(self.defined_by(members, member, value, choices, &inner, depth)?);

                    Json::Bool(true)
                }
                _ => self.reference(member.ty, &inner, depth + 1)?,
            };

            properties.push((member.name.clone(), schema));

            if !member.optional && !member.has_default() {
                required.push(text(&member.name));
            }
        }

        let mut schema = object([
            ("type", text("object")),
            ("properties", Json::Object(properties)),
        ]);

        if !required.is_empty() {
            schema.push("required", Json::Array(required));
        }

        if members.additions.is_none() {
            schema.push("additionalProperties", Json::Bool(false));
        }

        if !conditions.is_empty() {
            schema.push("allOf", Json::Array(conditions));
        }

        Ok(schema)
    }

    /// The type of a member that is an ANY DEFINED BY with choices, which
    /// the value of the member it is defined by tells: one condition for
    /// each choice, and one that the member is of one of them.
    fn defined_by(
        &mut self,
        members: &Members,
        member: &MemberInfo,
        key: &str,
        choices: &[(Const, TypeId)],
        path: &str,
        depth: usize,
    ) -> Result<Vec<Json>, String> {
        let key_type = members
            .all()
            .find(|candidate| candidate.name == key)
            .map(|candidate| candidate.ty)
            .ok_or_else(|| {
                format!("{path}: no member {key} that the ANY DEFINED BY is defined by.")
            })?;
        let mut conditions = Vec::new();
        let mut values = Vec::new();

        for (constant, ty) in choices {
            let value = self.key_value(constant, key_type, path)?;
            let schema = self.reference(*ty, path, depth + 1)?;

            values.push(value.clone());
            conditions.push(object([
                (
                    "if",
                    object([
                        (
                            "properties",
                            Json::Object(vec![(key.to_string(), object([("const", value)]))]),
                        ),
                        ("required", Json::Array(vec![text(key)])),
                    ]),
                ),
                (
                    "then",
                    object([(
                        "properties",
                        Json::Object(vec![(member.name.clone(), schema)]),
                    )]),
                ),
            ]));
        }

        conditions.push(object([
            (
                "if",
                object([("required", Json::Array(vec![text(&member.name)]))]),
            ),
            (
                "then",
                object([
                    (
                        "properties",
                        Json::Object(vec![(
                            key.to_string(),
                            object([("enum", Json::Array(values))]),
                        )]),
                    ),
                    ("required", Json::Array(vec![text(key)])),
                ]),
            ),
        ]));

        Ok(conditions)
    }

    /// A value of the member an ANY DEFINED BY is defined by, as JER
    /// writes it.
    fn key_value(&self, constant: &Const, key_type: TypeId, path: &str) -> Result<Json, String> {
        Ok(match constant {
            Const::Int(value) => number(value),
            Const::Text(value) => text(value),
            Const::Identifier(name) => match self.types.kind(key_type) {
                // A named number is written as its number.
                Kind::Integer(named) => match named.iter().find(|(candidate, _)| candidate == name)
                {
                    Some((_, value)) => number(value),
                    None => return Err(format!("{path}: no number named {name}.")),
                },
                _ => text(name),
            },
            Const::ObjectIdentifier(oid) => Json::Str(oid.to_string()),
            Const::Bool(value) => Json::Bool(*value),
            _ => {
                return Err(format!(
                    "{path}: a value of a choice of an ANY DEFINED BY that a schema does not have."
                ));
            }
        })
    }

    /// A CHOICE: an object with the name of one alternative, or of any
    /// other of a later version.
    fn choice(&mut self, members: &Members, path: &str, depth: usize) -> Result<Json, String> {
        let mut alternatives = Vec::new();
        let mut names = Vec::new();

        for member in members.all() {
            let schema =
                self.reference(member.ty, &format!("{path}.{}", member.name), depth + 1)?;

            names.push(text(&member.name));
            alternatives.push(object([
                ("type", text("object")),
                (
                    "properties",
                    Json::Object(vec![(member.name.clone(), schema)]),
                ),
                ("required", Json::Array(vec![text(&member.name)])),
                ("additionalProperties", Json::Bool(false)),
            ]));
        }

        if members.additions.is_some() {
            alternatives.push(object([
                ("type", text("object")),
                ("minProperties", number(1)),
                ("maxProperties", number(1)),
                (
                    "propertyNames",
                    object([("not", object([("enum", Json::Array(names))]))]),
                ),
            ]));
        }

        Ok(object([("anyOf", Json::Array(alternatives))]))
    }
}

/// Why the values of an ANY, an open type or an ANY DEFINED BY with no
/// types given are not in a schema.
fn unsupported(kind: &Kind, path: &str) -> String {
    match kind {
        Kind::AnyDefinedBy { .. } => format!(
            "{path}: an ANY DEFINED BY, of which the codec needs the type, which a schema has \
             if it is compiled with --any-defined-by."
        ),
        _ => format!(
            "{path}: an ANY or an open type, of which the codec needs the type, which no schema has."
        ),
    }
}

/// A content of an element as an error tells it, shortened.
fn shown(content: &str) -> String {
    match content.char_indices().nth(40) {
        Some((at, _)) => format!("{}...", &content[..at]),
        None => content.to_string(),
    }
}

/// The name each type with one has in a schema, by its id.
fn type_keys(types: &Types) -> BTreeMap<TypeId, String> {
    let mut keys = BTreeMap::new();

    for (module, assignments) in &types.modules {
        for (name, id) in assignments {
            if types.find(module, name) == Some(*id) {
                keys.entry(*id)
                    .or_insert_with(|| key_of(types, module, name, *id));
            }
        }
    }

    keys
}

/// The definitions of the types of the roots and of those they refer
/// to, in the order they are made, each by its name in `keys`. `make`
/// makes the definition of a type, and tells the names of those it refers
/// to. A type that can not be told is an error, or with
/// `--skip-unsupported` left out, with what refers to it, but not if it
/// is a root that is given.
fn definitions<T>(
    roots: &[Root],
    keys: &BTreeMap<TypeId, String>,
    request: &Request<'_>,
    skipped: &mut Vec<(String, String)>,
    mut make: impl FnMut(TypeId, &str) -> Result<(T, BTreeSet<String>), String>,
) -> Result<Vec<(String, T)>, String> {
    let ids: BTreeMap<&String, TypeId> = keys.iter().map(|(id, key)| (key, *id)).collect();
    let mut definitions: Vec<(String, T, BTreeSet<String>)> = Vec::new();
    let mut failed: BTreeMap<String, String> = BTreeMap::new();
    let mut pending: Vec<String> = roots.iter().map(|root| root.key.clone()).collect();
    let mut seen: BTreeSet<String> = pending.iter().cloned().collect();
    let mut at = 0;

    while at < pending.len() {
        let key = pending[at].clone();

        at += 1;

        match make(ids[&key], &key) {
            Ok((definition, refs)) => {
                for other in &refs {
                    if seen.insert(other.clone()) {
                        pending.push(other.clone());
                    }
                }

                definitions.push((key, definition, refs));
            }
            Err(why) if request.skip_unsupported => {
                failed.insert(key, why);
            }
            Err(why) => return Err(why),
        }
    }

    // What refers to what is left out is left out too, and which it
    // refers to is kept, to tell why a type that is given can not be.
    let mut refers: BTreeMap<String, String> = BTreeMap::new();

    loop {
        let before = failed.len();

        definitions.retain(|(key, _, refs)| {
            match refs.iter().find(|other| failed.contains_key(*other)) {
                Some(other) => {
                    failed.insert(
                        key.clone(),
                        format!("it refers to {other}, which is left out."),
                    );
                    refers.insert(key.clone(), other.clone());

                    false
                }
                None => true,
            }
        });

        if failed.len() == before {
            break;
        }
    }

    for root in roots.iter().filter(|root| root.given) {
        if failed.contains_key(&root.key) {
            // What refers to what, to the type that can not be told.
            let mut through = Vec::new();
            let mut key = &root.key;

            while let Some(other) = refers.get(key) {
                through.push(other.as_str());
                key = other;
            }

            let why = &failed[key];

            return Err(if through.is_empty() {
                why.clone()
            } else {
                format!(
                    "{} refers to {}, which a schema can not tell: {why}",
                    root.key,
                    through.join(", which refers to ")
                )
            });
        }
    }

    skipped.extend(failed);

    Ok(definitions
        .into_iter()
        .map(|(key, definition, _)| (key, definition))
        .collect())
}

fn json_schema(
    types: &Types,
    roots: &[Root],
    request: &Request<'_>,
    about: &str,
    skipped: &mut Vec<(String, String)>,
) -> Result<String, String> {
    let keys = type_keys(types);
    let mut schema = JsonSchema {
        types,
        keys: keys.clone(),
        refs: BTreeSet::new(),
    };
    let definitions = definitions(roots, &keys, request, skipped, |id, key| {
        schema.refs.clear();

        let json = schema.definition(id, key, 0)?;

        Ok((json, std::mem::take(&mut schema.refs)))
    })?;

    let mut document = object([
        (
            "$schema",
            text("https://json-schema.org/draft/2020-12/schema"),
        ),
        (
            "$comment",
            Json::Str(format!(
                "The JSON Encoding Rules (X.697) of the types of {about}, as asnkit writes them. \
                 Made by {}.",
                request.generator
            )),
        ),
    ]);
    let given: Vec<&Root> = roots.iter().filter(|root| root.given).collect();

    match given.as_slice() {
        [] => {}
        [root] => document.push("$ref", Json::Str(format!("#/$defs/{}", root.key))),
        roots => document.push(
            "anyOf",
            Json::Array(roots.iter().map(|root| reference_to(&root.key)).collect()),
        ),
    }

    document.push("$defs", Json::Object(definitions));

    let mut out = String::new();

    document.write(&mut out, 0);
    out.push('\n');

    Ok(out)
}

// DTD.

#[derive(Clone)]
struct Dtd<'a> {
    types: &'a Types,
    /// The names of the elements, in the order they are first found, an
    /// element before those in it.
    order: Vec<String>,
    mentioned: BTreeSet<String>,
    /// The content of each element, and where it was found first.
    declared: BTreeMap<String, (String, String)>,
    /// The elements and types that are declared, or being declared.
    visiting: BTreeSet<(String, TypeId)>,
}

impl Dtd<'_> {
    fn mention(&mut self, name: &str) {
        if self.mentioned.insert(name.to_string()) {
            self.order.push(name.to_string());
        }
    }

    /// Declares the element `name` of a value of the type `id`.
    fn declare(&mut self, name: &str, id: TypeId, path: &str, depth: usize) -> Result<(), String> {
        let name = name.replace(' ', "_");

        if !self.visiting.insert((name.clone(), id)) {
            return Ok(());
        }

        self.mention(&name);

        let content = self.content(id, path, depth + 1)?;

        self.put(&name, content, path)
    }

    fn put(&mut self, name: &str, content: String, path: &str) -> Result<(), String> {
        match self.declared.get(name) {
            Some((existing, _)) if *existing == content => Ok(()),
            Some((existing, other)) => Err(format!(
                "{path}: the element <{name}> is {} here and {} at {other}, which a DTD can not \
                 tell apart, as it has one declaration for each name.",
                shown(&content),
                shown(existing)
            )),
            None => {
                self.declared
                    .insert(name.to_string(), (content, path.to_string()));
                self.mention(name);

                Ok(())
            }
        }
    }

    /// The choice of empty elements, as `(true|false)`, which are declared.
    fn empties<'n>(
        &mut self,
        names: impl IntoIterator<Item = &'n str>,
        path: &str,
    ) -> Result<String, String> {
        let mut choice = Vec::new();

        for name in names {
            self.put(name, "EMPTY".to_string(), path)?;
            choice.push(name);
        }

        Ok(choice.join("|"))
    }

    /// The control characters a string of a type can have, each an empty
    /// element in XER (X.680 12.15.5).
    fn controls(kind: StrKind, alphabet: Option<&Vec<char>>) -> Vec<&'static str> {
        let can_have = |code: usize| match alphabet {
            Some(chars) => chars.iter().any(|c| *c as usize == code),
            None => !matches!(
                kind,
                StrKind::Numeric | StrKind::Printable | StrKind::Visible
            ),
        };

        (0..CONTROLS.len())
            .filter(|code| !CONTROLS[*code].is_empty() && can_have(*code))
            .map(|code| CONTROLS[code])
            .collect()
    }

    /// The content of the element of a value of the type `id`, after the
    /// elements in it are declared.
    fn content(&mut self, id: TypeId, path: &str, depth: usize) -> Result<String, String> {
        if depth > MAX_DEPTH {
            return Err(format!("{path}: the types are nested too deeply."));
        }

        let types = self.types;

        Ok(match types.kind(id) {
            Kind::Ref(_) => return Err(format!("{path}: the type has no contents of its own.")),
            Kind::Boolean => format!("({})", self.empties(["true", "false"], path)?),
            Kind::Integer(_)
            | Kind::ObjectIdentifier
            | Kind::BitString(_)
            | Kind::OctetString
            | Kind::Utc
            | Kind::Generalized
            | Kind::Date
            | Kind::TimeOfDay
            | Kind::DateTime
            | Kind::Duration => "(#PCDATA)".to_string(),
            // A number, or the empty element of a special value (X.680 21.9).
            Kind::Real => format!(
                "(#PCDATA|{})*",
                self.empties(["PLUS-INFINITY", "MINUS-INFINITY", "NOT-A-NUMBER"], path)?
            ),
            Kind::Null => "EMPTY".to_string(),
            Kind::Str(kind) => {
                let controls = Dtd::controls(*kind, types.constraints(id).alphabet.as_ref());

                if controls.is_empty() {
                    "(#PCDATA)".to_string()
                } else {
                    format!("(#PCDATA|{})*", self.empties(controls, path)?)
                }
            }
            Kind::Enumerated { root, additions } => format!(
                "({})",
                self.empties(
                    root.iter()
                        .chain(additions.iter().flatten())
                        .map(|(name, _)| name.as_str()),
                    path
                )?
            ),
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                let mut particles = Vec::new();

                for member in members.all() {
                    self.declare(
                        &member.name,
                        member.ty,
                        &format!("{path}.{}", member.name),
                        depth,
                    )?;

                    particles.push(if member.optional || member.has_default() {
                        format!("{}?", member.name)
                    } else {
                        member.name.clone()
                    });
                }

                if particles.is_empty() {
                    "EMPTY".to_string()
                } else {
                    format!("({})", particles.join(", "))
                }
            }
            Kind::SequenceOf(item) | Kind::SetOf(item) => {
                let size = types.constraints(id).size;
                let repeat = match size {
                    Some(size) if !size.extensible && size.min.is_some_and(|min| min > 0) => '+',
                    _ => '*',
                };
                let name = types
                    .node(types.node(id).concrete)
                    .element_name
                    .clone()
                    .unwrap_or_default();

                format!(
                    "({}){repeat}",
                    self.items(*item, &name, &format!("{path}[]"), depth)?
                )
            }
            Kind::Choice(members) => format!("({})", self.alternatives(members, path, depth)?),
            // The member it is defined by tells the type, but the element
            // is the same: it is told if all of them have one content.
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                let mut contents = BTreeSet::new();

                for (_, ty) in choices {
                    contents.insert(self.content(*ty, path, depth + 1)?);
                }

                if contents.len() > 1 {
                    return Err(format!(
                        "{path}: the types of the ANY DEFINED BY have other contents, which a \
                         DTD can not tell apart."
                    ));
                }

                contents.pop_first().unwrap_or_default()
            }
            kind @ (Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType) => {
                return Err(unsupported(kind, path));
            }
        })
    }

    /// The elements of the items of a SEQUENCE OF or a SET OF: a BOOLEAN,
    /// an ENUMERATED and a CHOICE have no element of their own (X.693
    /// 9.3.6).
    fn items(
        &mut self,
        item: TypeId,
        name: &str,
        path: &str,
        depth: usize,
    ) -> Result<String, String> {
        let types = self.types;

        match types.kind(item) {
            Kind::Boolean | Kind::Enumerated { .. } => {
                let content = self.content(item, path, depth + 1)?;

                Ok(content[1..content.len() - 1].to_string())
            }
            Kind::Choice(members) => self.alternatives(members, path, depth),
            _ => {
                self.declare(name, item, path, depth)?;

                Ok(name.replace(' ', "_"))
            }
        }
    }

    fn alternatives(
        &mut self,
        members: &Members,
        path: &str,
        depth: usize,
    ) -> Result<String, String> {
        let mut names = Vec::new();

        for member in members.all() {
            self.declare(
                &member.name,
                member.ty,
                &format!("{path}.{}", member.name),
                depth,
            )?;
            names.push(member.name.clone());
        }

        Ok(names.join("|"))
    }
}

fn dtd(
    types: &Types,
    roots: &[Root],
    request: &Request<'_>,
    about: &str,
    skipped: &mut Vec<(String, String)>,
) -> Result<String, String> {
    let mut dtd = Dtd {
        types,
        order: Vec::new(),
        mentioned: BTreeSet::new(),
        declared: BTreeMap::new(),
        visiting: BTreeSet::new(),
    };
    let mut names = Vec::new();

    for root in roots {
        let before = dtd.clone();

        match dtd.declare(&root.name, root.id, &root.key, 0) {
            Ok(()) => names.push(root.name.clone()),
            Err(why) if request.skip_unsupported && !root.given => {
                dtd = before;
                skipped.push((root.key.clone(), why));
            }
            Err(why) => return Err(why),
        }
    }

    // A comment may not have `--` in it.
    let comment = |text: String| format!("<!-- {} -->\n", text.replace("--", "- -"));
    let mut out = String::new();

    out.push_str(&comment(format!(
        "BASIC-XER (X.693) of the types of {about}, as asnkit writes them. Made by {}.",
        request.generator
    )));
    out.push_str(&comment(format!(
        "The root elements: {}.",
        names.join(", ")
    )));

    for name in &dtd.order {
        out.push_str(&format!("<!ELEMENT {name} {}>\n", dtd.declared[name].0));
    }

    Ok(out)
}

// XML Schema.

/// An element of XML, as an XML Schema is written of them.
#[derive(Debug, Clone, PartialEq)]
struct Xml {
    name: &'static str,
    attributes: Vec<(&'static str, String)>,
    children: Vec<Xml>,
}

fn xs(name: &'static str) -> Xml {
    Xml {
        name,
        attributes: Vec::new(),
        children: Vec::new(),
    }
}

impl Xml {
    fn with(mut self, name: &'static str, value: impl ToString) -> Xml {
        self.attributes.push((name, value.to_string()));
        self
    }

    fn child(mut self, child: Xml) -> Xml {
        self.children.push(child);
        self
    }

    fn children(mut self, children: impl IntoIterator<Item = Xml>) -> Xml {
        self.children.extend(children);
        self
    }

    /// The names of the model groups it refers to.
    fn groups(&self, out: &mut BTreeSet<String>) {
        if self.name == "group"
            && let Some((_, name)) = self.attributes.iter().find(|(name, _)| *name == "ref")
        {
            out.insert(name.clone());
        }

        for child in &self.children {
            child.groups(out);
        }
    }

    /// If an element in it is of the type `name`.
    fn uses(&self, name: &str) -> bool {
        self.attributes
            .iter()
            .any(|(attribute, value)| *attribute == "type" && value == name)
            || self.children.iter().any(|child| child.uses(name))
    }

    fn write(&self, out: &mut String, level: usize) {
        out.extend(std::iter::repeat_n("  ", level));
        out.push_str("<xs:");
        out.push_str(self.name);

        for (name, value) in &self.attributes {
            out.push(' ');
            out.push_str(name);
            out.push_str("=\"");

            for c in value.chars() {
                match c {
                    '&' => out.push_str("&amp;"),
                    '<' => out.push_str("&lt;"),
                    '>' => out.push_str("&gt;"),
                    '"' => out.push_str("&quot;"),
                    ' '..='~' => out.push(c),
                    c => out.push_str(&format!("&#x{:X};", c as u32)),
                }
            }

            out.push('"');
        }

        if self.children.is_empty() {
            out.push_str("/>\n");

            return;
        }

        out.push_str(">\n");

        for child in &self.children {
            child.write(out, level + 1);
        }

        out.extend(std::iter::repeat_n("  ", level));
        out.push_str("</xs:");
        out.push_str(self.name);
        out.push_str(">\n");
    }
}

/// The type of an empty element, which no type of ASN.1 is named, as
/// their names have no `_`.
const EMPTY: &str = "_Empty";

/// An empty element, as `<true />`.
fn empty_element(name: &str) -> Xml {
    xs("element").with("name", name).with("type", EMPTY)
}

/// A restriction of a type of XML Schema, with its facets.
fn restriction(base: &str, facets: impl IntoIterator<Item = Xml>) -> Xml {
    xs("simpleType").child(xs("restriction").with("base", base).children(facets))
}

fn facet(name: &'static str, value: impl ToString) -> Xml {
    xs(name).with("value", value)
}

/// A restriction for each of the bounds, in a union if there are more
/// than one, as `minInclusive` and `maxInclusive` or the facets named.
fn bounded(
    base: &str,
    bounds: &Bounds,
    minimum: &'static str,
    maximum: &'static str,
    more: &[Xml],
) -> Xml {
    let one = |(min, max): &(Option<String>, Option<String>)| {
        let mut facets: Vec<Xml> = Vec::new();

        // One size is a length.
        if minimum == "minLength" && min.is_some() && min == max {
            facets.push(facet("length", min.clone().unwrap_or_default()));
        } else {
            facets.extend(min.iter().map(|min| facet(minimum, min)));
            facets.extend(max.iter().map(|max| facet(maximum, max)));
        }

        facets.extend(more.iter().cloned());

        restriction(base, facets)
    };

    match bounds.as_slice() {
        [] => restriction(base, more.iter().cloned()),
        [only] => one(only),
        parts => xs("simpleType").child(xs("union").children(parts.iter().map(one))),
    }
}

/// A character in a class of a regular expression of XML Schema.
fn xsd_class_char(out: &mut String, c: char) {
    match c {
        '\\' | ']' | '[' | '^' | '-' => {
            out.push('\\');
            out.push(c);
        }
        '\t' => out.push_str("\\t"),
        '\n' => out.push_str("\\n"),
        '\r' => out.push_str("\\r"),
        c => out.push(c),
    }
}

fn xsd_char_class(chars: &[char]) -> String {
    let mut sorted: Vec<char> = chars.to_vec();

    sorted.sort_unstable();
    sorted.dedup();

    let mut out = String::new();
    let mut index = 0;

    while index < sorted.len() {
        let first = sorted[index];
        let mut last = first;

        while index + 1 < sorted.len() && sorted[index + 1] as u32 == last as u32 + 1 {
            index += 1;
            last = sorted[index];
        }

        xsd_class_char(&mut out, first);

        if last != first {
            if last as u32 > first as u32 + 1 {
                out.push('-');
            }

            xsd_class_char(&mut out, last);
        }

        index += 1;
    }

    out
}

/// The forms of dates and times the decoders read, as the regular
/// expressions of XML Schema have them: anchored, with no `(?:` and no
/// lookahead.
const XSD_UTC_TIME: &str = "[0-9]{10}([0-9]{2})?(Z|[+\\-][0-9]{4})";
const XSD_GENERALIZED_TIME: &str =
    "[0-9]{10}([0-9]{2}([0-9]{2})?)?([.,][0-9]+)?(Z|[+\\-][0-9]{2}([0-9]{2})?)?";
const XSD_DATE: &str = "[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}";
const XSD_TIME_OF_DAY: &str = "[0-9]{1,2}:[0-9]{1,2}:[0-9]{1,2}";
const XSD_DATE_TIME: &str = "[0-9]{4}-[0-9]{1,2}-[0-9]{1,2}T[0-9]{1,2}:[0-9]{1,2}:[0-9]{1,2}";

/// A DURATION: weeks alone, or a date with a time or not, or a time,
/// each of them with one element at least.
fn xsd_duration_pattern() -> String {
    let n = "(0|[1-9][0-9]*)([.,][0-9]+)?";
    let date = format!("({n}Y({n}M)?({n}D)?|{n}M({n}D)?|{n}D)");
    let time = format!("T({n}H({n}M)?({n}S)?|{n}M({n}S)?|{n}S)");

    format!("P({n}W|{date}({time})?|{time})")
}

fn string_restriction(base: &str, pattern: &str) -> Xml {
    restriction(base, [facet("pattern", pattern)])
}

struct XmlSchema<'a> {
    types: &'a Types,
    keys: BTreeMap<TypeId, String>,
    refs: BTreeSet<String>,
}

impl XmlSchema<'_> {
    /// The name of the type `id` in the schema, or of the type it is a
    /// reference to, if it is that type in XER.
    fn key(&self, id: TypeId) -> Option<String> {
        if let Some(key) = self.keys.get(&id) {
            return Some(key.clone());
        }

        match &self.types.node(id).kind {
            Kind::Ref(target) if self.types.constraints(id) == self.types.constraints(*target) => {
                self.key(*target)
            }
            _ => None,
        }
    }

    /// The element of a value of the type `id`: with the name of the type
    /// if it has one, else with the type in it.
    fn element(&mut self, name: &str, id: TypeId, path: &str, depth: usize) -> Result<Xml, String> {
        let element = xs("element").with("name", name.replace(' ', "_"));

        if depth > MAX_DEPTH {
            return Err(format!("{path}: the types are nested too deeply."));
        }

        if let Some(key) = self.keys.get(&id) {
            self.refs.insert(key.clone());

            return Ok(element.with("type", key));
        }

        // A reference with a tag of its own is the type it refers to, in
        // XER, which has no tags.
        if let Kind::Ref(target) = &self.types.node(id).kind
            && self.types.constraints(id) == self.types.constraints(*target)
        {
            return self.element(name, *target, path, depth + 1);
        }

        let definition = self.definition(id, path, depth)?;

        // A type of XML Schema with no facets is that type.
        if let [restriction] = definition.children.as_slice()
            && definition.name == "simpleType"
            && restriction.name == "restriction"
            && restriction.children.is_empty()
        {
            return Ok(element.with("type", &restriction.attributes[0].1));
        }

        Ok(element.child(definition))
    }

    /// An element in another, as a member, which may be left out.
    fn particle(&mut self, member: &MemberInfo, path: &str, depth: usize) -> Result<Xml, String> {
        let inner = format!("{path}.{}", member.name);
        let element = match self.types.kind(member.ty) {
            // The member it is defined by tells the type, but the element
            // is the same: it is told if all of them are one type.
            Kind::AnyDefinedBy { choices, .. } if !choices.is_empty() => {
                let mut elements = Vec::new();

                for (_, ty) in choices {
                    let element = self.element(&member.name, *ty, &inner, depth + 1)?;

                    if !elements.contains(&element) {
                        elements.push(element);
                    }
                }

                if elements.len() > 1 {
                    return Err(format!(
                        "{inner}: the types of the ANY DEFINED BY are not one, which an XML \
                         Schema can not tell apart."
                    ));
                }

                elements.remove(0)
            }
            _ => self.element(&member.name, member.ty, &inner, depth + 1)?,
        };

        Ok(if member.optional || member.has_default() {
            element.with("minOccurs", 0)
        } else {
            element
        })
    }

    /// The `simpleType` or the `complexType` of a type, with no name.
    fn definition(&mut self, id: TypeId, path: &str, depth: usize) -> Result<Xml, String> {
        if depth > MAX_DEPTH {
            return Err(format!("{path}: the types are nested too deeply."));
        }

        let types = self.types;
        let constraints = types.constraints(id);
        let complex = |content: Xml| xs("complexType").child(content);

        Ok(match types.kind(id) {
            Kind::Ref(_) => return Err(format!("{path}: the type has no contents of its own.")),
            Kind::Boolean => {
                complex(xs("choice").children([empty_element("true"), empty_element("false")]))
            }
            Kind::Integer(_) => bounded(
                "xs:integer",
                &range_bounds(constraints),
                "minInclusive",
                "maxInclusive",
                &[],
            ),
            // A number, or the empty element of a special value (X.680
            // 21.9), of which the text is not told.
            Kind::Real => xs("complexType").with("mixed", true).child(
                xs("choice").with("minOccurs", 0).children(
                    ["PLUS-INFINITY", "MINUS-INFINITY", "NOT-A-NUMBER"].map(empty_element),
                ),
            ),
            Kind::Null => xs("complexType"),
            Kind::BitString(_) => bounded(
                "xs:string",
                &size_bounds(constraints, 1),
                "minLength",
                "maxLength",
                &[facet("pattern", "[01]*")],
            ),
            Kind::OctetString => bounded(
                "xs:hexBinary",
                &size_bounds(constraints, 1),
                "minLength",
                "maxLength",
                &[],
            ),
            Kind::ObjectIdentifier => string_restriction("xs:token", "[0-9]+(\\.[0-9]+)+"),
            Kind::Str(kind) => {
                let controls = Dtd::controls(*kind, constraints.alphabet.as_ref());

                if controls.is_empty() {
                    let class = match (&constraints.alphabet, kind) {
                        (Some(chars), _) => Some(xsd_char_class(chars)),
                        (None, StrKind::Numeric) => Some("0-9 ".to_string()),
                        (None, StrKind::Printable) => Some("A-Za-z0-9 '()+,\\-./:=?".to_string()),
                        (None, StrKind::Visible) => Some(" -~".to_string()),
                        (None, _) => None,
                    };
                    let pattern: Vec<Xml> = class
                        .map(|class| facet("pattern", format!("[{class}]*")))
                        .into_iter()
                        .collect();

                    bounded(
                        "xs:string",
                        &size_bounds(constraints, 1),
                        "minLength",
                        "maxLength",
                        &pattern,
                    )
                } else {
                    // A control character is an empty element of its name
                    // (X.680 12.15.5), and the text with them is not told.
                    xs("complexType").with("mixed", true).child(
                        xs("choice")
                            .with("minOccurs", 0)
                            .with("maxOccurs", "unbounded")
                            .children(controls.into_iter().map(empty_element)),
                    )
                }
            }
            Kind::Enumerated { root, additions } => complex(
                xs("choice").children(
                    root.iter()
                        .chain(additions.iter().flatten())
                        .map(|(name, _)| empty_element(name)),
                ),
            ),
            // The members of a SET in any order (X.693 9.3.5).
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) => {
                let group = if matches!(types.kind(id), Kind::Set(_)) {
                    "all"
                } else {
                    "sequence"
                };
                let mut particles = Vec::new();

                for member in members.all() {
                    particles.push(self.particle(member, path, depth)?);
                }

                if particles.is_empty() {
                    xs("complexType")
                } else {
                    complex(xs(group).children(particles))
                }
            }
            Kind::SequenceOf(item) | Kind::SetOf(item) => {
                let (min, max) = match constraints.size {
                    Some(size) if !size.extensible => (size.min.unwrap_or(0), size.max),
                    _ => (0, None),
                };
                let name = types
                    .node(types.node(id).concrete)
                    .element_name
                    .clone()
                    .unwrap_or_default();
                let inner = format!("{path}[]");
                // A BOOLEAN, an ENUMERATED and a CHOICE have no element of
                // their own as an item (X.693 9.3.6): the item is what is in
                // it, as the model group of the type of the same name, which
                // a CHOICE that has itself as an item needs, or in the
                // sequence if the type has no name.
                let particle = match (types.kind(*item), self.key(*item)) {
                    (Kind::Boolean | Kind::Enumerated { .. } | Kind::Choice(_), Some(key)) => {
                        self.refs.insert(key.clone());
                        xs("group").with("ref", key)
                    }
                    (Kind::Boolean | Kind::Enumerated { .. } | Kind::Choice(_), None) => {
                        let mut group = self
                            .definition(*item, &inner, depth + 1)?
                            .children
                            .remove(0);

                        group.attributes.retain(|(name, _)| *name != "minOccurs");
                        group
                    }
                    _ => self.element(&name, *item, &inner, depth + 1)?,
                };
                let max = max.map_or("unbounded".to_string(), |max| max.to_string());

                complex(
                    xs("sequence").child(particle.with("minOccurs", min).with("maxOccurs", max)),
                )
            }
            Kind::Choice(members) => {
                let mut alternatives = Vec::new();

                for member in members.all() {
                    alternatives.push(self.element(
                        &member.name,
                        member.ty,
                        &format!("{path}.{}", member.name),
                        depth + 1,
                    )?);
                }

                complex(xs("choice").children(alternatives))
            }
            Kind::Utc => string_restriction("xs:string", XSD_UTC_TIME),
            Kind::Generalized => string_restriction("xs:string", XSD_GENERALIZED_TIME),
            Kind::Date => string_restriction("xs:string", XSD_DATE),
            Kind::TimeOfDay => string_restriction("xs:string", XSD_TIME_OF_DAY),
            Kind::DateTime => string_restriction("xs:string", XSD_DATE_TIME),
            Kind::Duration => string_restriction("xs:string", &xsd_duration_pattern()),
            kind @ (Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType) => {
                return Err(unsupported(kind, path));
            }
        })
    }
}

fn xml_schema(
    types: &Types,
    roots: &[Root],
    request: &Request<'_>,
    about: &str,
    skipped: &mut Vec<(String, String)>,
) -> Result<String, String> {
    let keys = type_keys(types);
    let mut schema = XmlSchema {
        types,
        keys: keys.clone(),
        refs: BTreeSet::new(),
    };
    let definitions = definitions(roots, &keys, request, skipped, |id, key| {
        schema.refs.clear();

        let definition = schema.definition(id, key, 0)?.with("name", key);

        Ok((definition, std::mem::take(&mut schema.refs)))
    })?;
    let defined: BTreeSet<&String> = definitions.iter().map(|(key, _)| key).collect();
    let mut document = xs("schema").with("xmlns:xs", "http://www.w3.org/2001/XMLSchema");
    // The root elements, named after their types, of which two of one name
    // in two modules have to be the same.
    let mut elements: BTreeMap<&str, &str> = BTreeMap::new();

    for root in roots.iter().filter(|root| defined.contains(&root.key)) {
        match elements.get(root.name.as_str()) {
            Some(other) => {
                let why = format!(
                    "{}: the root element <{}> is also of {other}, which an XML Schema can not \
                     tell apart.",
                    root.key, root.name
                );

                if request.skip_unsupported && !root.given {
                    skipped.push((root.key.clone(), why));
                } else {
                    return Err(why);
                }
            }
            None => {
                elements.insert(&root.name, &root.key);
                document = document.child(
                    xs("element")
                        .with("name", &root.name)
                        .with("type", &root.key),
                );
            }
        }
    }

    // The model groups of the types that are items, with what is in their
    // types, which are defined, as the items refer to them.
    let mut groups = BTreeSet::new();

    for (_, definition) in &definitions {
        definition.groups(&mut groups);
    }

    let groups: Vec<Xml> = groups
        .into_iter()
        .filter_map(|key| {
            let (_, definition) = definitions.iter().find(|(other, _)| *other == key)?;
            let mut content = definition.children.first()?.clone();

            content.attributes.retain(|(name, _)| *name != "minOccurs");

            Some(xs("group").with("name", key).child(content))
        })
        .collect();

    document = document.children(definitions.into_iter().map(|(_, definition)| definition));
    document = document.children(groups);

    if document.uses(EMPTY) {
        document = document.child(xs("complexType").with("name", EMPTY));
    }

    let mut out = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!-- {} -->\n",
        format!(
            "BASIC-XER (X.693) of the types of {about}, as asnkit writes them. Made by {}.",
            request.generator
        )
        .replace("--", "- -")
    );

    document.write(&mut out, 0);

    Ok(out)
}
