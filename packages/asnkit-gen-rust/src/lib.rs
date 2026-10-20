// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Makes the text of a Rust module from a compiled ASN.1 specification.
//!
//! The module imports `asnkitrt` and `serde`, has the `.asni` file in it
//! as bytes and has a struct for each SEQUENCE and SET, an enum for each
//! ENUMERATED and each CHOICE, and a type for the rest, with methods to
//! encode and decode them with the specification.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use asnkitrt::container::{self, Checks};
use asnkitrt::runtime::{self, Specification};
use asnkitrt::types::{
    Addition, Codec, Kind, MemberInfo, Members, Range, RealFormat, TypeId, Types,
};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// How deep types are nested in each other, at most.
const MAX_DEPTH: usize = 256;

/// What the generated code uses, which a type or a function is not
/// called.
const TAKEN: &[&str] = &[
    "Asn1Type",
    "Box",
    "Err",
    "None",
    "Ok",
    "Option",
    "Result",
    "Self",
    "Some",
    "String",
    "Vec",
    "ASNI",
    "load",
    "load_with",
];

const KEYWORDS: &[&str] = &[
    "Self", "abstract", "as", "async", "await", "become", "box", "break", "const", "continue",
    "crate", "do", "dyn", "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if",
    "impl", "in", "let", "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub",
    "ref", "return", "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof",
    "unsafe", "unsized", "use", "virtual", "where", "while", "yield",
];

/// Keywords that can not be raw identifiers.
const NOT_RAW: &[&str] = &["Self", "crate", "self", "super"];

/// What goes wrong when the code is made.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

impl From<asnkitrt::Error> for Error {
    fn from(error: asnkitrt::Error) -> Error {
        Error(format!(
            "Expected a compiled specification that is not signed, encrypted, bound to something or has a nonce, but: {error}"
        ))
    }
}

fn fail<T>(message: String) -> Result<T, Error> {
    Err(Error(message))
}

/// What an INTEGER with no bounds is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Integer {
    #[default]
    I64,
    I128,
    /// `asnkitrt::serde_value::BigInt`, which holds any.
    Big,
}

impl Integer {
    pub fn parse(text: &str) -> Option<Integer> {
        match text {
            "i64" => Some(Integer::I64),
            "i128" => Some(Integer::I128),
            "big" => Some(Integer::Big),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Options {
    /// The code needs `alloc` and not `std`.
    pub no_std: bool,
    pub integer: Integer,
    /// The compiled specification is in the module, with `load()` and
    /// `load_with()`. Without it, the specification is loaded by the code
    /// that uses the module, as from a file that is signed or encrypted.
    pub embed: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            no_std: false,
            integer: Integer::default(),
            embed: true,
        }
    }
}

/// `foo-bar` and `foo_bar` are `FooBar`.
pub fn camel(name: &str) -> String {
    let mut result = String::new();

    for part in name.split(['-', '_']).filter(|part| !part.is_empty()) {
        let mut chars = part.chars();

        if let Some(first) = chars.next() {
            result.extend(first.to_uppercase());
            result.push_str(chars.as_str());
        }
    }

    if result
        .chars()
        .next()
        .is_none_or(|first| first.is_ascii_digit())
    {
        result.insert(0, 'T');
    }

    result
}

/// `fooBar` and `foo-bar` are `foo_bar`, and `IA5String` is `ia5_string`.
pub fn snake(name: &str) -> String {
    let chars: Vec<char> = name.replace('-', "_").chars().collect();
    let mut result = String::new();

    for (index, &c) in chars.iter().enumerate() {
        if c.is_uppercase() && index > 0 {
            let before = chars[index - 1];
            let after = chars.get(index + 1);

            // A word starts at a capital after a lower case letter or
            // a digit, and at the last capital of a run before a lower
            // case letter.
            if before.is_lowercase()
                || before.is_ascii_digit()
                || (before.is_uppercase() && after.is_some_and(|after| after.is_lowercase()))
            {
                result.push('_');
            }
        }

        result.extend(c.to_lowercase());
    }

    if result
        .chars()
        .next()
        .is_none_or(|first| first.is_ascii_digit())
    {
        result.insert(0, 'n');
    }

    result
}

/// A name of a field or a function as Rust has it.
fn identifier(name: String) -> String {
    if NOT_RAW.contains(&name.as_str()) {
        format!("{name}_")
    } else if KEYWORDS.contains(&name.as_str()) {
        format!("r#{name}")
    } else {
        name
    }
}

/// The components of the graph of what is held in what, in which a
/// type that is held by itself, directly or not, can not be held by
/// value. A SEQUENCE OF is not an edge, as a list is on the heap.
fn components(types: &Types) -> Vec<usize> {
    let count = types.nodes.len();
    let edges: Vec<Vec<usize>> = types
        .nodes
        .iter()
        .map(|node| match &node.kind {
            Kind::Sequence(members) | Kind::Set(members) | Kind::Choice(members) => members
                .all()
                .map(|member| types.nodes[member.ty].concrete)
                .collect(),
            _ => Vec::new(),
        })
        .collect();

    let mut index = vec![usize::MAX; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut component = vec![usize::MAX; count];
    let mut stack = Vec::new();
    let mut next_index = 0;
    let mut next_component = 0;

    for root in 0..count {
        if index[root] != usize::MAX {
            continue;
        }

        let mut calls = vec![(root, 0)];

        index[root] = next_index;
        low[root] = next_index;
        next_index += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&mut (node, ref mut position)) = calls.last_mut() {
            if let Some(&target) = edges[node].get(*position) {
                *position += 1;

                if index[target] == usize::MAX {
                    index[target] = next_index;
                    low[target] = next_index;
                    next_index += 1;
                    stack.push(target);
                    on_stack[target] = true;
                    calls.push((target, 0));
                } else if on_stack[target] {
                    low[node] = low[node].min(index[target]);
                }
            } else {
                calls.pop();

                if let Some(&(parent, _)) = calls.last() {
                    low[parent] = low[parent].min(low[node]);
                }

                if low[node] == index[node] {
                    while let Some(member) = stack.pop() {
                        on_stack[member] = false;
                        component[member] = next_component;

                        if member == node {
                            break;
                        }
                    }

                    next_component += 1;
                }
            }
        }
    }

    component
}

struct Generator<'a> {
    types: &'a Types,
    options: Options,
    /// The Rust name of a type that has one.
    names: BTreeMap<TypeId, String>,
    taken: BTreeSet<String>,
    functions_taken: BTreeSet<String>,
    component: Vec<usize>,
    blocks: Vec<String>,
    functions: Vec<String>,
}

impl<'a> Generator<'a> {
    fn new(types: &'a Types, options: Options) -> Generator<'a> {
        Generator {
            types,
            options,
            names: BTreeMap::new(),
            taken: TAKEN.iter().map(|name| (*name).to_string()).collect(),
            functions_taken: TAKEN.iter().map(|name| (*name).to_string()).collect(),
            component: components(types),
            blocks: Vec::new(),
            functions: Vec::new(),
        }
    }

    /// `name`, or the first of `name2`, `name3` and so on that is not
    /// taken.
    fn claim(&mut self, name: String) -> String {
        let mut candidate = name.clone();
        let mut number = 2;

        while self.taken.contains(&candidate) {
            candidate = format!("{name}{number}");
            number += 1;
        }

        self.taken.insert(candidate.clone());

        candidate
    }

    fn claim_function(&mut self, name: String) -> String {
        let mut candidate = name.clone();
        let mut number = 2;

        while self.functions_taken.contains(&candidate) {
            candidate = format!("{name}_{number}");
            number += 1;
        }

        self.functions_taken.insert(candidate.clone());

        candidate
    }

    fn integer_type(&self, range: Option<Range>) -> String {
        if let Some(Range {
            min: Some(min),
            max: Some(max),
            extensible: false,
        }) = range
        {
            let fits = |name: &'static str, low: i128, high: i128| {
                (min >= low && max <= high).then(|| name.to_string())
            };

            let found = if min >= 0 {
                fits("u8", 0, u8::MAX.into())
                    .or_else(|| fits("u16", 0, u16::MAX.into()))
                    .or_else(|| fits("u32", 0, u32::MAX.into()))
                    .or_else(|| fits("u64", 0, u64::MAX.into()))
                    .or_else(|| fits("i128", i128::MIN, i128::MAX))
            } else {
                fits("i8", i8::MIN.into(), i8::MAX.into())
                    .or_else(|| fits("i16", i16::MIN.into(), i16::MAX.into()))
                    .or_else(|| fits("i32", i32::MIN.into(), i32::MAX.into()))
                    .or_else(|| fits("i64", i64::MIN.into(), i64::MAX.into()))
                    .or_else(|| fits("i128", i128::MIN, i128::MAX))
            };

            if let Some(found) = found {
                return found;
            }
        }

        match self.options.integer {
            Integer::I64 => "i64".to_string(),
            Integer::I128 => "i128".to_string(),
            Integer::Big => "asnkitrt::serde_value::BigInt".to_string(),
        }
    }

    /// The Rust type of the type with the id `id`, which is `context` if
    /// it needs a type of its own that has no name.
    fn ty(&mut self, id: TypeId, context: &str, depth: usize) -> Result<String, Error> {
        self.ty_from(id, id, context, false, depth)
    }

    /// As `ty`, with the constraints of `first` and, if `expanding`,
    /// not the name `first` has itself.
    fn ty_from(
        &mut self,
        id: TypeId,
        first: TypeId,
        context: &str,
        expanding: bool,
        depth: usize,
    ) -> Result<String, Error> {
        if depth > MAX_DEPTH {
            return fail(format!(
                "The types are nested deeper than {MAX_DEPTH} in {context}."
            ));
        }

        let mut current = id;

        for _ in 0..=self.types.nodes.len() {
            if !(expanding && current == first)
                && let Some(name) = self.names.get(&current)
            {
                return Ok(name.clone());
            }

            match self.types.nodes[current].kind {
                Kind::Ref(target) => current = target,
                _ => return self.shape(current, first, context, depth),
            }
        }

        fail(format!("The type in {context} refers to itself."))
    }

    fn shape(
        &mut self,
        id: TypeId,
        first: TypeId,
        context: &str,
        depth: usize,
    ) -> Result<String, Error> {
        let node = &self.types.nodes[first];
        let kind = self.types.nodes[id].kind.clone();

        Ok(match kind {
            Kind::Ref(_) => unreachable!("references are followed"),
            Kind::Boolean => "bool".to_string(),
            Kind::Integer(_) => self.integer_type(node.effective_range),
            Kind::Real => match self.types.nodes[id].real.or(node.real) {
                Some(RealFormat::Binary32) => "f32".to_string(),
                _ => "f64".to_string(),
            },
            Kind::Null => "()".to_string(),
            Kind::BitString(_) => "asnkitrt::serde_value::BitString".to_string(),
            Kind::OctetString => "asnkitrt::serde_value::OctetString".to_string(),
            Kind::ObjectIdentifier => "asnkitrt::value::ObjectIdentifier<'static>".to_string(),
            Kind::Str(_) | Kind::Duration => "String".to_string(),
            Kind::Utc | Kind::Generalized | Kind::DateTime => {
                "asnkitrt::value::DateTime".to_string()
            }
            Kind::Date => "asnkitrt::value::Date".to_string(),
            Kind::TimeOfDay => "asnkitrt::value::Time".to_string(),
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType | Kind::External(_) => {
                "asnkitrt::value::Value<'static>".to_string()
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                let element = self.ty(element, &format!("{context}Item"), depth + 1)?;

                format!("Vec<{element}>")
            }
            Kind::Sequence(members) | Kind::Set(members) => {
                let name = self.claim(context.to_string());

                self.names.insert(id, name.clone());
                self.structure(id, &name, &members, None, depth)?;

                name
            }
            Kind::Choice(members) => {
                let name = self.claim(context.to_string());

                self.names.insert(id, name.clone());
                self.choice(id, &name, &members, None, depth)?;

                name
            }
            Kind::Enumerated { root, additions } => {
                let name = self.claim(context.to_string());

                self.names.insert(id, name.clone());
                self.enumeration(&name, &root, additions.as_deref(), None);

                name
            }
        })
    }

    /// The members as fields: with whether each is optional.
    fn fields(members: &Members) -> Vec<(&MemberInfo, bool)> {
        let mut fields: Vec<(&MemberInfo, bool)> = members
            .root
            .iter()
            .map(|member| (member, member.optional || member.has_default()))
            .collect();

        for addition in members.additions.iter().flatten() {
            match addition {
                Addition::Member(member) => fields.push((member, true)),
                Addition::Group(group) => fields.extend(group.iter().map(|member| (member, true))),
            }
        }

        fields
    }

    /// If a member of `parent` has to be on the heap, as it holds
    /// `parent` in turn.
    fn is_boxed(&self, parent: TypeId, member: TypeId) -> bool {
        let target = self.types.nodes[member].concrete;
        let parent = self.types.nodes[parent].concrete;

        self.component[target] != usize::MAX && self.component[target] == self.component[parent]
    }

    fn structure(
        &mut self,
        id: TypeId,
        name: &str,
        members: &Members,
        named: Option<(&str, &str)>,
        depth: usize,
    ) -> Result<(), Error> {
        let mut lines = vec![derive(), format!("pub struct {name} {{")];
        let mut used = BTreeSet::new();

        for (member, optional) in Generator::fields(members) {
            let mut field = identifier(snake(&member.name));
            let mut number = 2;

            while !used.insert(field.clone()) {
                field = identifier(format!("{}_{number}", snake(&member.name)));
                number += 1;
            }

            let context = format!("{name}{}", camel(&member.name));
            let mut ty = self.ty(member.ty, &context, depth + 1)?;

            if self.is_boxed(id, member.ty) {
                ty = format!("Box<{ty}>");
            }

            let mut attributes = Vec::new();

            if field.trim_start_matches("r#") != member.name {
                attributes.push(format!("rename = {:?}", member.name));
            }

            if optional {
                attributes.push("default".to_string());
                attributes.push("skip_serializing_if = \"Option::is_none\"".to_string());
                ty = format!("Option<{ty}>");
            }

            if !attributes.is_empty() {
                lines.push(format!("    #[serde({})]", attributes.join(", ")));
            }

            lines.push(format!("    pub {field}: {ty},"));
        }

        lines.push("}".to_string());
        self.finish(lines, name, named);

        Ok(())
    }

    fn choice(
        &mut self,
        id: TypeId,
        name: &str,
        members: &Members,
        named: Option<(&str, &str)>,
        depth: usize,
    ) -> Result<(), Error> {
        let mut lines = vec![
            derive(),
            "#[allow(clippy::large_enum_variant, clippy::enum_variant_names)]".to_string(),
            format!("pub enum {name} {{"),
        ];
        let mut used = BTreeSet::new();

        for (member, _) in Generator::fields(members) {
            let mut variant = camel(&member.name);
            let mut number = 2;

            while !used.insert(variant.clone()) {
                variant = format!("{}{number}", camel(&member.name));
                number += 1;
            }

            let context = format!("{name}{variant}");
            let mut ty = self.ty(member.ty, &context, depth + 1)?;

            if self.is_boxed(id, member.ty) {
                ty = format!("Box<{ty}>");
            }

            if variant != member.name {
                lines.push(format!("    #[serde(rename = {:?})]", member.name));
            }

            lines.push(format!("    {variant}({ty}),"));
        }

        lines.push("}".to_string());
        self.finish(lines, name, named);

        Ok(())
    }

    fn enumeration(
        &mut self,
        name: &str,
        root: &[(String, i64)],
        additions: Option<&[(String, i64)]>,
        named: Option<(&str, &str)>,
    ) {
        let mut lines = vec![
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]"
                .to_string(),
            format!("pub enum {name} {{"),
        ];
        let mut used = BTreeSet::new();

        for (value, _) in root.iter().chain(additions.into_iter().flatten()) {
            let mut variant = camel(value);
            let mut number = 2;

            while !used.insert(variant.clone()) {
                variant = format!("{}{number}", camel(value));
                number += 1;
            }

            if variant != *value {
                lines.push(format!("    #[serde(rename = {value:?})]"));
            }

            lines.push(format!("    {variant},"));
        }

        lines.push("}".to_string());
        self.finish(lines, name, named);
    }

    /// Adds the type to the code, with what it needs to be encoded if
    /// it has a name in a module.
    fn finish(&mut self, mut lines: Vec<String>, name: &str, named: Option<(&str, &str)>) {
        if let Some((module, type_name)) = named {
            lines.push(String::new());
            lines.push(format!("impl Asn1Type for {name} {{"));
            lines.push(format!("    const MODULE: &'static str = {module:?};"));
            lines.push(format!("    const NAME: &'static str = {type_name:?};"));
            lines.push("}".to_string());
        }

        self.blocks.push(lines.join("\n"));
    }

    fn named(&mut self, module: &str, type_name: &str, id: TypeId) -> Result<(), Error> {
        let name = self.names[&id].clone();
        let named = Some((module, type_name));
        let kind = self.types.nodes[id].kind.clone();

        match kind {
            Kind::Sequence(members) | Kind::Set(members) => {
                self.structure(id, &name, &members, named, 0)
            }
            Kind::Choice(members) => self.choice(id, &name, &members, named, 0),
            Kind::Enumerated { root, additions } => {
                self.enumeration(&name, &root, additions.as_deref(), named);

                Ok(())
            }
            _ => {
                let target = self.ty_from(id, id, &name, true, 0)?;
                let function = self.claim_function(snake(&name));

                self.blocks.push(format!("pub type {name} = {target};"));
                self.functions.push(format!(
                    "pub fn encode_{function}(\n    value: &{name},\n    specification: &asnkitrt::runtime::Specification,\n) -> asnkitrt::Result<Vec<u8>> {{\n    specification.encode_in({module:?}, {type_name:?}, &asnkitrt::serde_value::to_value(value)?)\n}}\n\npub fn decode_{function}(\n    specification: &asnkitrt::runtime::Specification,\n    data: &[u8],\n) -> asnkitrt::Result<{name}> {{\n    asnkitrt::serde_value::from_value(&specification.decode_in({module:?}, {type_name:?}, data)?)\n}}"
                ));

                Ok(())
            }
        }
    }

    fn run(mut self, asni: &[u8], sources: &[String]) -> Result<String, Error> {
        let types = self.types;

        // The names of the types of the modules, which are the same in
        // more than one module with the name of the module before them.
        let mut count: BTreeMap<String, usize> = BTreeMap::new();

        for (_, assignments) in &types.modules {
            for (name, _) in assignments {
                *count.entry(camel(name)).or_default() += 1;
            }
        }

        let mut assigned = Vec::new();

        for (module, assignments) in &types.modules {
            for (type_name, id) in assignments {
                let mut name = camel(type_name);

                if count[&name] > 1 {
                    name = format!("{}{name}", camel(module));
                }

                let name = self.claim(name);

                self.names.insert(*id, name);
                assigned.push((module.clone(), type_name.clone(), *id));
            }
        }

        for (module, type_name, id) in assigned {
            self.named(&module, &type_name, id)?;
        }

        let alloc = if self.options.no_std {
            "\nuse alloc::boxed::Box;\nuse alloc::string::String;\nuse alloc::vec::Vec;\n"
        } else {
            ""
        };
        let mut parts = vec![
            HEADER
                .replace("{version}", VERSION)
                .replace(
                    "{sources}",
                    &if sources.is_empty() {
                        "a specification".to_string()
                    } else {
                        sources.join(", ")
                    },
                )
                .replace("{alloc}", alloc)
                .replace(
                    "{load}",
                    &if self.options.embed {
                        EMBEDDED.replace("{asni}", &format_bytes(asni))
                    } else {
                        NOT_EMBEDDED.to_string()
                    },
                ),
        ];

        parts.extend(self.blocks);
        parts.extend(self.functions);

        Ok(parts.join("\n\n") + "\n")
    }
}

/// The loading of a specification that is in the module.
const EMBEDDED: &str = r#"/// The compiled specification, as the contents of an .asni file.
const ASNI: &[u8] = &[
{asni}
];

/// Loads the specification for a codec.
pub fn load(codec: asnkitrt::types::Codec) -> asnkitrt::Result<asnkitrt::runtime::Specification> {
    load_with(asnkitrt::runtime::Options::new(codec))
}

/// Loads the specification with the options given.
pub fn load_with(
    options: asnkitrt::runtime::Options,
) -> asnkitrt::Result<asnkitrt::runtime::Specification> {
    asnkitrt::runtime::Specification::from_asni(
        ASNI,
        &asnkitrt::container::Checks::default(),
        options,
    )
}
"#;

/// What a module says of its specification when it is not in it.
const NOT_EMBEDDED: &str = r#"// The compiled specification is not in this module. Load the .asni file
// it was made from with asnkitrt::runtime::Specification::from_asni(), with
// the checks the file needs, and give it to the functions below.
"#;

fn derive() -> String {
    "#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]".to_string()
}

const HEADER: &str = r#"// Generated by asnkit-gen-rust {version} from {sources}. Do not edit.
//
// This file is made from the specification it names, and needs the asnkitrt
// crate (MIT) and serde to run. It may be used under terms of your choice:
// the code that asnkit-gen-rust copies into it is not under the Apache
// License of the plugin.
{alloc}
{load}
/// What the types made from the types of a module can do.
pub trait Asn1Type: serde::Serialize + serde::de::DeserializeOwned {
    /// The module the type is in.
    const MODULE: &'static str;
    /// The name of the type in the module.
    const NAME: &'static str;

    /// What asnkitrt encodes for this value.
    fn to_value(&self) -> asnkitrt::Result<asnkitrt::value::Value<'static>> {
        asnkitrt::serde_value::to_value(self)
    }

    /// The value for what asnkitrt decoded.
    fn from_value(value: &asnkitrt::value::Value<'_>) -> asnkitrt::Result<Self> {
        asnkitrt::serde_value::from_value(value)
    }

    /// This value encoded with the codec the specification is loaded for.
    fn encode(
        &self,
        specification: &asnkitrt::runtime::Specification,
    ) -> asnkitrt::Result<Vec<u8>> {
        specification.encode_in(Self::MODULE, Self::NAME, &self.to_value()?)
    }

    /// The value that the data is, decoded with the codec the
    /// specification is loaded for.
    fn decode(specification: &asnkitrt::runtime::Specification, data: &[u8]) -> asnkitrt::Result<Self> {
        Self::from_value(&specification.decode_in(Self::MODULE, Self::NAME, data)?)
    }

    /// Checks this value against the constraints of its type.
    fn check_constraints(
        &self,
        specification: &asnkitrt::runtime::Specification,
    ) -> asnkitrt::Result<()> {
        specification.check_constraints_in(Self::MODULE, Self::NAME, &self.to_value()?)
    }
}"#;

fn format_bytes(data: &[u8]) -> String {
    data.chunks(16)
        .map(|chunk| {
            let bytes: Vec<String> = chunk.iter().map(|byte| format!("0x{byte:02x},")).collect();

            format!("    {}", bytes.join(" "))
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// The Rust module for the contents of an `.asni` file, which must not
/// be signed, encrypted, bound to something or have a nonce to check,
/// as it may be embedded in the module and loaded without any of those.
/// Returns it with the names of the files it was made from.
pub fn generate(asni: &[u8], options: Options) -> Result<(String, Vec<String>), Error> {
    let file = container::read(asni, &Checks::default())?;

    if file.header.signed_by.is_some()
        || file.header.encrypted_with.is_some()
        || file.header.nonce.is_some()
        || file.metadata.bound_to.is_some()
    {
        return fail(
            "Expected a compiled specification that is not signed, encrypted, bound to something or has a nonce."
                .to_string(),
        );
    }

    let specification = Specification::from_nodes(&file.nodes, runtime::Options::new(Codec::Ber))?;
    let sources: Vec<String> = file
        .metadata
        .sources
        .iter()
        .filter_map(|source| source.name.clone())
        .collect();
    let text = Generator::new(specification.types(), options).run(asni, &sources)?;

    Ok((text, sources))
}
