// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Makes the text of a Rust module that encodes and decodes the types of
//! a compiled ASN.1 specification with BER, DER and UPER, with code of
//! its own for each type, without the specification and without
//! allocating.
//!
//! The module has a struct for each SEQUENCE and SET, an enum for each
//! CHOICE and ENUMERATED, and a type for the rest. A value borrows what
//! it is decoded from, as `asnkitrt::wire` has it.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::{self, Write};

use asnkit_gen_rust::{camel, components, identifier, snake};
use asnkitrt::container::{self, Checks};
use asnkitrt::node::TagClass;
use asnkitrt::per::{Characters, Known};
use asnkitrt::runtime::{self, Specification};
use asnkitrt::types::{Addition, Codec, Kind, MemberInfo, Members, Range, StrKind, TypeId, Types};
use asnkitrt::value::Value;

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// What the generated code uses, which a type is not called.
const TAKEN: &[&str] = &[
    "Asn1", "Box", "Err", "None", "Ok", "Option", "Result", "Self", "Some", "String", "Vec",
];

/// What the generated code uses, which a function or a module is not
/// called.
const FUNCTIONS_TAKEN: &[&str] = &["alloc", "asnkitrt", "ber", "core", "per", "std", "wire"];

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

/// What an INTEGER with no bounds is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Integer {
    #[default]
    I64,
    I128,
    /// `asnkitrt::wire::Integer`, which holds any value, and borrows
    /// those of more than 128 bits.
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

#[derive(Debug, Clone, Copy, Default)]
pub struct Options {
    /// The code needs `alloc` and not `std`.
    pub no_std: bool,
    pub integer: Integer,
    /// Types that are not supported, and those that hold them, are left
    /// out, rather than being an error.
    pub skip_unsupported: bool,
}

/// What is made of the type of a member: what it is if it is absent.
#[derive(Debug, Clone)]
enum Presence {
    Required,
    Optional,
    /// Its default, as Rust.
    Default(String),
}

struct Field {
    name: String,
    member: MemberInfo,
    presence: Presence,
    boxed: bool,
}

struct Variant {
    name: String,
    member: MemberInfo,
    boxed: bool,
}

enum Definition {
    Struct {
        id: TypeId,
        name: String,
        fields: Vec<Field>,
    },
    Choice {
        id: TypeId,
        name: String,
        variants: Vec<Variant>,
    },
    Enum {
        name: String,
        root: Vec<(String, i64)>,
        additions: Option<Vec<(String, i64)>>,
        variants: Vec<String>,
    },
}

/// A type of a module, as the generated code has it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GeneratedType {
    pub module: String,
    /// Its name in ASN.1.
    pub name: String,
    /// The name of its Rust type.
    pub rust: String,
    /// The module of its functions, if it is not a struct or an enum of
    /// its own, which implement `Asn1`.
    pub functions: Option<String>,
}

/// A named type of a module.
struct Named {
    id: TypeId,
    module: String,
    asn1: String,
    name: String,
    /// The name of the module of functions of a type that is not a
    /// struct or an enum of its own.
    functions: Option<String>,
}

struct Generator<'a> {
    types: &'a Types,
    options: Options,
    /// The Rust name of a SEQUENCE, SET, CHOICE or ENUMERATED, by the id
    /// of the type.
    names: BTreeMap<TypeId, String>,
    /// The names of the variants of an ENUMERATED, by its id.
    variants: BTreeMap<TypeId, Vec<String>>,
    taken: BTreeSet<String>,
    functions_taken: BTreeSet<String>,
    component: Vec<usize>,
    definitions: Vec<Definition>,
    /// The structs and enums of CHOICEs that hold a value that borrows.
    lifetime: BTreeSet<TypeId>,
    /// If a type is supported, by its id, as far as it is known.
    support: BTreeMap<TypeId, Result<(), &'static str>>,
    /// The types of which the functions are made, and those to make.
    emitted: BTreeSet<TypeId>,
    queue: Vec<TypeId>,
    contents: BTreeSet<TypeId>,
    content_queue: Vec<TypeId>,
    /// The permitted alphabets of UPER, by their Rust, and their names.
    knowns: BTreeMap<String, String>,
    boxes: bool,
}

fn fail<T>(message: String) -> Result<T, Error> {
    Err(Error(message))
}

/// The name of a variant, which is not `Self`.
fn variant_name(name: &str) -> String {
    match camel(name).as_str() {
        "Self" => "Self_".to_string(),
        other => other.to_string(),
    }
}

/// If a type is encoded with a tag of its own, and not that of what it
/// holds, as a CHOICE is.
fn has_base_tag(kind: &Kind) -> bool {
    !matches!(
        kind,
        Kind::Choice(_) | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType | Kind::Ref(_)
    )
}

/// If a string is of a known multiplier in PER (X.691 30.5).
fn is_known(kind: StrKind) -> bool {
    matches!(
        kind,
        StrKind::Numeric
            | StrKind::Printable
            | StrKind::Ia5
            | StrKind::Visible
            | StrKind::Bmp
            | StrKind::Universal
    )
}

fn class(class: TagClass) -> &'static str {
    match class {
        TagClass::Universal => "wire::TagClass::Universal",
        TagClass::Application => "wire::TagClass::Application",
        TagClass::ContextSpecific => "wire::TagClass::ContextSpecific",
        TagClass::Private => "wire::TagClass::Private",
    }
}

fn range(range: Option<Range>) -> String {
    match range {
        None => "None".to_string(),
        Some(range) => format!(
            "Some(wire::Range {{ min: {:?}, max: {:?}, extensible: {} }})",
            range.min, range.max, range.extensible
        ),
    }
}

fn bytes(data: &[u8]) -> String {
    let items: Vec<String> = data.iter().map(|byte| format!("0x{byte:02x}")).collect();

    format!("&[{}]", items.join(", "))
}

/// The members, as fields, in the order the codecs have them.
fn fields(members: &Members) -> Vec<(MemberInfo, bool)> {
    let mut fields: Vec<(MemberInfo, bool)> = members
        .root
        .iter()
        .map(|member| (member.clone(), false))
        .collect();

    for addition in members.additions.iter().flatten() {
        match addition {
            Addition::Member(member) => fields.push((member.clone(), true)),
            Addition::Group(group) => {
                fields.extend(group.iter().map(|member| (member.clone(), true)));
            }
        }
    }

    fields
}

impl<'a> Generator<'a> {
    fn new(types: &'a Types, options: Options) -> Generator<'a> {
        Generator {
            types,
            options,
            names: BTreeMap::new(),
            variants: BTreeMap::new(),
            taken: TAKEN.iter().map(|name| (*name).to_string()).collect(),
            functions_taken: FUNCTIONS_TAKEN
                .iter()
                .map(|name| (*name).to_string())
                .collect(),
            component: components(types),
            definitions: Vec::new(),
            lifetime: BTreeSet::new(),
            support: BTreeMap::new(),
            emitted: BTreeSet::new(),
            queue: Vec::new(),
            contents: BTreeSet::new(),
            content_queue: Vec::new(),
            knowns: BTreeMap::new(),
            boxes: false,
        }
    }

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

        identifier(candidate)
    }

    fn concrete(&self, id: TypeId) -> TypeId {
        self.types.nodes[id].concrete
    }

    fn kind(&self, id: TypeId) -> &'a Kind {
        self.types.kind(id)
    }

    // What is supported.

    /// If the type `id` and those in it are supported, or why not.
    fn supported(&mut self, id: TypeId) -> Result<(), &'static str> {
        let concrete = self.concrete(id);

        if let Some(known) = self.support.get(&concrete) {
            return *known;
        }

        // A type that holds itself is supported if the rest of it is.
        self.support.insert(concrete, Ok(()));

        let result = match self.kind(concrete) {
            Kind::Real => Err("REAL"),
            Kind::External(_) => Err("EXTERNAL"),
            Kind::Date | Kind::TimeOfDay | Kind::DateTime => Err("DATE, TIME-OF-DAY and DATE-TIME"),
            Kind::Duration => Err("DURATION"),
            Kind::Sequence(members) | Kind::Set(members) | Kind::Choice(members) => members
                .all()
                .map(|member| member.ty)
                .collect::<Vec<_>>()
                .into_iter()
                .try_for_each(|member| self.supported(member)),
            Kind::SequenceOf(element) | Kind::SetOf(element) => self.supported(*element),
            _ => Ok(()),
        };

        self.support.insert(concrete, result);

        result
    }

    // The types.

    /// Names the types in the type `id` that have no name, after
    /// `context`, and makes their definitions.
    fn visit(&mut self, id: TypeId, context: &str) {
        let concrete = self.concrete(id);

        if self.names.contains_key(&concrete) {
            return;
        }

        match self.kind(concrete) {
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) | Kind::Enumerated { .. } => {
                let name = self.claim(context.to_string());

                self.define(concrete, name);
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                self.visit(*element, &format!("{context}Item"));
            }
            _ => {}
        }
    }

    /// Makes the definition of the SEQUENCE, SET, CHOICE or ENUMERATED
    /// `id`, of the name `name`.
    fn define(&mut self, id: TypeId, name: String) {
        self.names.insert(id, name.clone());

        match self.kind(id) {
            Kind::Sequence(members) | Kind::Set(members) => {
                let mut used = BTreeSet::new();
                let mut result = Vec::new();

                for (member, addition) in fields(members) {
                    let base = identifier(snake(&member.name));
                    let mut field = base.clone();
                    let mut number = 2;

                    while !used.insert(field.clone()) {
                        field = identifier(format!("{}_{number}", snake(&member.name)));
                        number += 1;
                    }

                    self.visit(member.ty, &format!("{name}{}", camel(&member.name)));

                    let boxed = self.is_boxed(id, member.ty);
                    let presence = if addition || member.optional {
                        Presence::Optional
                    } else if member.has_default() {
                        match self.default_value(&member) {
                            Some(literal) if !boxed => Presence::Default(literal),
                            _ => Presence::Optional,
                        }
                    } else {
                        Presence::Required
                    };

                    result.push(Field {
                        name: field,
                        member,
                        presence,
                        boxed,
                    });
                }

                self.definitions.push(Definition::Struct {
                    id,
                    name,
                    fields: result,
                });
            }
            Kind::Choice(members) => {
                let mut used = BTreeSet::new();
                let mut result = Vec::new();

                for member in members.all() {
                    let mut variant = variant_name(&member.name);
                    let mut number = 2;

                    while !used.insert(variant.clone()) {
                        variant = format!("{}{number}", variant_name(&member.name));
                        number += 1;
                    }

                    self.visit(member.ty, &format!("{name}{variant}"));
                    result.push(Variant {
                        name: variant,
                        member: member.clone(),
                        boxed: self.is_boxed(id, member.ty),
                    });
                }

                self.definitions.push(Definition::Choice {
                    id,
                    name,
                    variants: result,
                });
            }
            Kind::Enumerated { root, additions } => {
                let mut used = BTreeSet::new();
                let mut variants = Vec::new();

                for (value, _) in root.iter().chain(additions.iter().flatten()) {
                    let mut variant = variant_name(value);
                    let mut number = 2;

                    while !used.insert(variant.clone()) {
                        variant = format!("{}{number}", variant_name(value));
                        number += 1;
                    }

                    variants.push(variant);
                }

                self.variants.insert(id, variants.clone());
                self.definitions.push(Definition::Enum {
                    name,
                    root: root.clone(),
                    additions: additions.clone(),
                    variants,
                });
            }
            _ => unreachable!("only those have definitions"),
        }
    }

    /// If a member of `parent` has to be on the heap, as it holds
    /// `parent` in turn.
    fn is_boxed(&self, parent: TypeId, member: TypeId) -> bool {
        let target = self.concrete(member);

        self.component[target] != usize::MAX && self.component[target] == self.component[parent]
    }

    /// The structs and the CHOICEs that hold something that borrows.
    fn find_lifetimes(&mut self) {
        loop {
            let mut changed = false;

            for definition in &self.definitions {
                let (id, members): (TypeId, Vec<TypeId>) = match definition {
                    Definition::Struct { id, fields, .. } => {
                        (*id, fields.iter().map(|field| field.member.ty).collect())
                    }
                    Definition::Choice { id, variants, .. } => (
                        *id,
                        variants.iter().map(|variant| variant.member.ty).collect(),
                    ),
                    Definition::Enum { .. } => continue,
                };

                if !self.lifetime.contains(&id)
                    && members.iter().any(|member| self.borrows(*member))
                {
                    self.lifetime.insert(id);
                    changed = true;
                }
            }

            if !changed {
                break;
            }
        }
    }

    /// If the Rust type of the type `id` borrows, and has a lifetime.
    fn borrows(&self, id: TypeId) -> bool {
        let concrete = self.concrete(id);

        match self.kind(concrete) {
            Kind::Integer(_) => self.is_big(id),
            Kind::Boolean
            | Kind::Null
            | Kind::Enumerated { .. }
            | Kind::Utc
            | Kind::Generalized => false,
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) => self.lifetime.contains(&concrete),
            _ => true,
        }
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
            Integer::Big => "wire::Integer<'a>".to_string(),
        }
    }

    /// If an INTEGER of the type `id` is a `wire::Integer`.
    fn is_big(&self, id: TypeId) -> bool {
        self.integer_type(self.types.nodes[id].effective_range)
            .starts_with("wire::")
    }

    /// The Rust type of a value of the type `id`.
    fn rust_type(&self, id: TypeId) -> String {
        let node = &self.types.nodes[id];
        let concrete = node.concrete;

        match self.kind(concrete) {
            Kind::Boolean => "bool".to_string(),
            Kind::Integer(_) => self.integer_type(node.effective_range),
            Kind::Null => "()".to_string(),
            Kind::BitString(_) => "wire::BitString<'a>".to_string(),
            Kind::OctetString | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                "wire::Octets<'a>".to_string()
            }
            Kind::ObjectIdentifier => "wire::Oid<'a>".to_string(),
            Kind::Str(_) => "wire::Text<'a>".to_string(),
            Kind::Utc | Kind::Generalized => "wire::DateTime".to_string(),
            Kind::Enumerated { .. } => self.names[&concrete].clone(),
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) => {
                if self.lifetime.contains(&concrete) {
                    format!("{}<'a>", self.names[&concrete])
                } else {
                    self.names[&concrete].clone()
                }
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                format!("wire::SequenceOf<'a, {}>", self.rust_type(*element))
            }
            Kind::Real
            | Kind::External(_)
            | Kind::Date
            | Kind::TimeOfDay
            | Kind::DateTime
            | Kind::Duration
            | Kind::Ref(_) => unreachable!("not supported, or followed"),
        }
    }

    /// The same, with `'_` for the lifetime, as an argument has it.
    fn argument_type(&self, id: TypeId) -> String {
        self.rust_type(id).replace("'a", "'_")
    }

    /// The default of a member as Rust, if it can be one.
    fn default_value(&self, member: &MemberInfo) -> Option<String> {
        let default = member.default.as_ref()?;
        let concrete = self.concrete(member.ty);

        match (self.kind(concrete), default) {
            (Kind::Boolean, Value::Boolean(value)) => Some(value.to_string()),
            (Kind::Integer(_), Value::Integer(value)) if self.is_big(member.ty) => {
                Some(format!("wire::Integer::new({value})"))
            }
            (Kind::Integer(named), Value::Text(name)) if self.is_big(member.ty) => named
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, value)| format!("wire::Integer::new({value})")),
            (Kind::Integer(_), Value::Integer(value)) => {
                let ty = self.integer_type(self.types.nodes[member.ty].effective_range);
                let fits = match ty.as_str() {
                    "u8" => u8::try_from(*value).is_ok(),
                    "u16" => u16::try_from(*value).is_ok(),
                    "u32" => u32::try_from(*value).is_ok(),
                    "u64" => u64::try_from(*value).is_ok(),
                    "i8" => i8::try_from(*value).is_ok(),
                    "i16" => i16::try_from(*value).is_ok(),
                    "i32" => i32::try_from(*value).is_ok(),
                    "i64" => i64::try_from(*value).is_ok(),
                    _ => true,
                };

                fits.then(|| value.to_string())
            }
            (Kind::Integer(named), Value::Text(name)) => named
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, value)| value.to_string()),
            (Kind::Enumerated { root, additions }, Value::Text(name)) => {
                let index = root
                    .iter()
                    .chain(additions.iter().flatten())
                    .position(|(candidate, _)| candidate == name)?;
                let variants = self.enum_variants(concrete);

                Some(format!(
                    "{}::{}",
                    self.names.get(&concrete)?,
                    variants.get(index)?
                ))
            }
            (Kind::Str(_), Value::Text(text)) => Some(format!("wire::Text::new({text:?})")),
            (Kind::OctetString, Value::Bytes(data)) => {
                Some(format!("wire::Octets::new({})", bytes(data)))
            }
            (Kind::BitString(_), Value::Bits(data, bits)) => {
                Some(format!("wire::BitString::new({}, {bits})", bytes(data)))
            }
            (Kind::ObjectIdentifier, Value::ObjectIdentifier(oid)) => {
                Some(format!("wire::Oid::from_static({})", bytes(oid.as_bytes())))
            }
            _ => None,
        }
    }

    fn enum_variants(&self, id: TypeId) -> Vec<String> {
        if let Some(variants) = self.variants.get(&id) {
            return variants.clone();
        }

        // Not defined yet: named as `define` names them.
        let Kind::Enumerated { root, additions } = self.kind(id) else {
            return Vec::new();
        };
        let mut used = BTreeSet::new();

        root.iter()
            .chain(additions.iter().flatten())
            .map(|(value, _)| {
                let mut variant = variant_name(value);
                let mut number = 2;

                while !used.insert(variant.clone()) {
                    variant = format!("{}{number}", variant_name(value));
                    number += 1;
                }

                variant
            })
            .collect()
    }

    /// If `value` (an expression of a reference to the value of a member)
    /// is not the default of the member, as Rust.
    fn not_default(&self, member: &MemberInfo, value: &str, default: &str) -> String {
        // The value itself: a field of `&v.field`, or what a reference
        // `x` is.
        let (place, value) = match value.strip_prefix('&') {
            Some(field) => (field.to_string(), field.to_string()),
            None => (value.to_string(), format!("*{value}")),
        };

        match (self.kind(member.ty), default) {
            (Kind::BitString(named), _) => {
                format!("!{place}.same_as(&{default}, {})", !named.is_empty())
            }
            (Kind::Boolean, "true") => format!("!{value}"),
            (Kind::Boolean, "false") => value,
            _ => format!("{value} != {default}"),
        }
    }

    // The code of the codecs.

    /// Asks for the functions of the type `id`.
    fn need(&mut self, id: TypeId) -> TypeId {
        if self.emitted.insert(id) {
            self.queue.push(id);
        }

        id
    }

    fn need_content(&mut self, id: TypeId) -> TypeId {
        if self.contents.insert(id) {
            self.content_queue.push(id);
        }

        id
    }

    /// The name of the permitted alphabet of a string of a known
    /// multiplier of the type `id`, which is made once.
    fn known(&mut self, id: TypeId, kind: StrKind) -> String {
        let known = Known::new(kind, &self.types.nodes[id].effective_alphabet, false);
        let characters = match &known.alphabet.characters {
            Characters::Numeric => "per::Characters::Numeric".to_string(),
            Characters::Printable => "per::Characters::Printable".to_string(),
            Characters::Ia5 => "per::Characters::Ia5".to_string(),
            Characters::Visible => "per::Characters::Visible".to_string(),
            Characters::Bmp => "per::Characters::Bmp".to_string(),
            Characters::Universal => "per::Characters::Universal".to_string(),
            Characters::Custom(codes) => {
                let codes: Vec<String> = codes.iter().map(u32::to_string).collect();

                format!("per::Characters::custom(&[{}])", codes.join(", "))
            }
        };
        let literal = format!(
            "per::Known {{\n    alphabet: per::Alphabet {{\n        characters: {characters},\n        by_index: {},\n    }},\n    bits_per_character: {},\n}}",
            known.alphabet.by_index, known.bits_per_character
        );
        let count = self.knowns.len();

        self.knowns
            .entry(literal)
            .or_insert_with(|| format!("KNOWN_{count}"))
            .clone()
    }

    /// The pattern of the tags a value of the type `id` may begin with,
    /// as `peek_tag` gives them.
    fn first_tags(&self, id: TypeId) -> String {
        let node = &self.types.nodes[id];

        if let Some(tag) = node.tags.first() {
            return format!("({}, {})", class(tag.class), tag.number);
        }

        if node.begins_with_any || node.begins_with.is_empty() {
            return "_".to_string();
        }

        node.begins_with
            .iter()
            .map(|(tag_class, number)| format!("({}, {number})", class(*tag_class)))
            .collect::<Vec<_>>()
            .join(" | ")
    }

    /// The functions of the type `id`: its BER and UPER, both ways.
    fn node_functions(&mut self, id: TypeId, out: &mut String) {
        let node = &self.types.nodes[id];
        let concrete = node.concrete;
        let kind = self.kind(concrete);
        let ty = self.rust_type(id);
        let argument = self.argument_type(id);
        let tags = node.tags.clone();
        let (explicit, base) = if has_base_tag(kind) {
            match tags.split_last() {
                Some((base, explicit)) => (explicit.to_vec(), Some(*base)),
                None => (Vec::new(), None),
            }
        } else {
            (tags.clone(), None)
        };

        // BER, writing.
        let mut body = String::new();

        for (index, tag) in explicit.iter().enumerate() {
            let _ = writeln!(
                body,
                "    let t{index} = w.open({}, true, {});",
                class(tag.class),
                tag.number
            );
        }

        if let Some(base) = base {
            let _ = writeln!(
                body,
                "    let b = w.open({}, {}, {});",
                class(base.class),
                base.constructed,
                base.number
            );
        }

        let contents = match kind {
            Kind::Boolean => "w.boolean(*v);".to_string(),
            Kind::Integer(_) if self.is_big(id) => "w.big_integer(v);".to_string(),
            Kind::Integer(_) if ty == "i128" => "w.integer(*v);".to_string(),
            Kind::Integer(_) => "w.integer(i128::from(*v));".to_string(),
            Kind::Null => "let _ = v;".to_string(),
            Kind::BitString(named) => format!("w.bit_string(v, {}, der);", !named.is_empty()),
            Kind::OctetString | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                "w.octets(v);".to_string()
            }
            Kind::ObjectIdentifier => "w.oid(v);".to_string(),
            Kind::Enumerated { .. } => "w.integer(i128::from(v.number()));".to_string(),
            Kind::Str(string) => format!("w.text(v, wire::StrKind::{string:?})?;"),
            Kind::Utc => "w.utc_time(v, der);".to_string(),
            Kind::Generalized => "w.generalized_time(v, der);".to_string(),
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) => {
                format!(
                    "ber_write_contents_{}(v, w, der)?;",
                    self.need_content(concrete)
                )
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => format!(
                "v.write_ber(w, der, {}, ber_write_{})?;",
                matches!(kind, Kind::SetOf(_)),
                self.need(*element)
            ),
            _ => unreachable!("not supported"),
        };

        let _ = writeln!(body, "    {contents}");

        if base.is_some() {
            body.push_str("    w.close(b);\n");
        }

        for index in (0..explicit.len()).rev() {
            let _ = writeln!(body, "    w.close(t{index});");
        }

        let _ = write!(
            out,
            "#[allow(unused_variables)]\nfn ber_write_{id}(v: &{argument}, w: &mut ber::Writer<'_>, der: bool) -> wire::Result<()> {{\n{body}    Ok(())\n}}\n\n"
        );

        // BER, reading.
        let mut body = String::new();

        for (index, tag) in explicit.iter().enumerate() {
            let _ = writeln!(
                body,
                "    let t{index} = r.constructed({}, {})?;",
                class(tag.class),
                tag.number
            );
        }

        let read = match (kind, base) {
            (Kind::Choice(_), _) => {
                format!("ber_read_contents_{}(r)?", self.need_content(concrete))
            }
            (Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType, _) => {
                "wire::Octets::new(r.any()?)".to_string()
            }
            (_, Some(base)) => {
                let tag = format!("{}, {}", class(base.class), base.number);
                let primitive = |read: &str| {
                    format!("{{\n        let c = r.primitive({tag})?;\n        {read}\n    }}")
                };

                match kind {
                    Kind::Boolean => primitive("ber::boolean(r, c)?"),
                    Kind::Integer(_) if self.is_big(id) => primitive("ber::big_integer(r, c)?"),
                    Kind::Integer(_) => primitive("ber::fit(r, ber::integer(r, c)?)?"),
                    Kind::Null => primitive("ber::null(r, c)?"),
                    Kind::BitString(named) => {
                        format!("r.bit_string({tag}, {})?", !named.is_empty())
                    }
                    Kind::OctetString => format!("r.octet_string({tag})?"),
                    Kind::ObjectIdentifier => primitive("ber::oid(r, c)?"),
                    Kind::Enumerated { .. } => primitive(&format!(
                        "{}::from_number(ber::integer(r, c)?)\n            .ok_or_else(|| wire::Error::decode(\"Expected an enumeration value.\", r.position()))?",
                        self.names[&concrete]
                    )),
                    Kind::Str(string) => format!("r.text({tag}, wire::StrKind::{string:?})?"),
                    Kind::Utc => primitive("ber::utc_time(r, c)?"),
                    Kind::Generalized => primitive("ber::generalized_time(r, c)?"),
                    Kind::Sequence(members) | Kind::Set(members) => {
                        let close = if members.additions.is_some() {
                            "skip_rest"
                        } else {
                            "close"
                        };

                        format!(
                            "{{\n        let g = r.constructed({tag})?;\n        let v = ber_read_contents_{}(r, &g)?;\n        r.{close}(g)?;\n        v\n    }}",
                            self.need_content(concrete)
                        )
                    }
                    Kind::SequenceOf(element) | Kind::SetOf(element) => format!(
                        "{{\n        let g = r.constructed({tag})?;\n        let v = wire::SequenceOf::read_ber(r, &g, {}, ber_read_{})?;\n        r.close(g)?;\n        v\n    }}",
                        matches!(kind, Kind::SetOf(_)),
                        self.need(*element)
                    ),
                    _ => unreachable!("not supported"),
                }
            }
            _ => unreachable!("a type with a tag of its own has one"),
        };

        let _ = writeln!(body, "    let v = {read};");

        for index in (0..explicit.len()).rev() {
            let _ = writeln!(body, "    r.close(t{index})?;");
        }

        let _ = write!(
            out,
            "#[allow(unused_variables, clippy::let_unit_value)]\nfn ber_read_{id}<'a>(r: &mut ber::Reader<'a>) -> wire::Result<{ty}> {{\n{body}    Ok(v)\n}}\n\n"
        );

        // UPER, writing.
        let size = range(node.effective_size);
        let write = match kind {
            Kind::Boolean => "w.bit(*v);".to_string(),
            Kind::Integer(_) if self.is_big(id) => {
                format!("per::big_integer(w, v, {})?;", range(node.effective_range))
            }
            Kind::Integer(_) if ty == "i128" => {
                format!("per::integer(w, *v, {})?;", range(node.effective_range))
            }
            Kind::Integer(_) => format!(
                "per::integer(w, i128::from(*v), {})?;",
                range(node.effective_range)
            ),
            Kind::Null => "let _ = (v, w);".to_string(),
            Kind::BitString(named) => {
                format!("per::bit_string(w, v, {}, {size})?;", !named.is_empty())
            }
            Kind::OctetString => format!("per::octet_string(w, v, {size})?;"),
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                "per::octet_string(w, v, None)?;".to_string()
            }
            Kind::ObjectIdentifier => "per::oid(w, v);".to_string(),
            Kind::Enumerated { root, additions } => format!(
                "per::enumerated(w, {}, {}, v.uper_index());",
                root.len(),
                additions.is_some()
            ),
            Kind::Str(string) if is_known(*string) => {
                let known = self.known(id, *string);

                format!("per::known_string(w, v, {size}, &{known})?;")
            }
            Kind::Str(string) => format!("per::other_string(w, v, wire::StrKind::{string:?})?;"),
            Kind::Utc => "per::utc_time(w, v)?;".to_string(),
            Kind::Generalized => "per::generalized_time(w, v)?;".to_string(),
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) => {
                format!(
                    "uper_write_contents_{}(v, w)?;",
                    self.need_content(concrete)
                )
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => {
                format!(
                    "v.write_uper(w, {size}, uper_write_{})?;",
                    self.need(*element)
                )
            }
            _ => unreachable!("not supported"),
        };

        let _ = write!(
            out,
            "#[allow(unused_variables)]\nfn uper_write_{id}(v: &{argument}, w: &mut per::Writer<'_>) -> wire::Result<()> {{\n    {write}\n    Ok(())\n}}\n\n"
        );

        // UPER, reading.
        let read = match kind {
            Kind::Boolean => "r.bit()?".to_string(),
            Kind::Integer(_) if self.is_big(id) => {
                format!("per::read_big_integer(r, {})?", range(node.effective_range))
            }
            Kind::Integer(_) => format!(
                "{{\n        let n = per::read_integer(r, {})?;\n        per::fit(r, n)?\n    }}",
                range(node.effective_range)
            ),
            Kind::Null => "{\n        let _ = r;\n    }".to_string(),
            Kind::BitString(_) => format!("per::read_bit_string(r, {size})?"),
            Kind::OctetString => format!("per::read_octet_string(r, {size})?"),
            Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType => {
                "per::read_unbound_octets(r)?".to_string()
            }
            Kind::ObjectIdentifier => "per::read_oid(r)?".to_string(),
            Kind::Enumerated { root, additions } => format!(
                "{}::from_uper_index(per::read_enumerated(r, {}, {})?)\n        .ok_or_else(|| wire::Error::decode(\"Expected an enumeration value.\", r.position()))?",
                self.names[&concrete],
                root.len(),
                additions.is_some()
            ),
            Kind::Str(string) if is_known(*string) => {
                let known = self.known(id, *string);

                format!("per::read_known_string(r, {size}, &{known})?")
            }
            Kind::Str(string) => format!("per::read_other_string(r, wire::StrKind::{string:?})?"),
            Kind::Utc => "per::read_utc_time(r)?".to_string(),
            Kind::Generalized => "per::read_generalized_time(r)?".to_string(),
            Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) => {
                format!("uper_read_contents_{}(r)?", self.need_content(concrete))
            }
            Kind::SequenceOf(element) | Kind::SetOf(element) => format!(
                "wire::SequenceOf::read_uper(r, {size}, uper_read_{})?",
                self.need(*element)
            ),
            _ => unreachable!("not supported"),
        };

        let _ = write!(
            out,
            "#[allow(unused_variables, clippy::let_unit_value)]\nfn uper_read_{id}<'a>(r: &mut per::Reader<'a>) -> wire::Result<{ty}> {{\n    let v = {read};\n\n    Ok(v)\n}}\n\n"
        );
    }

    /// A member's value of the type `id`, read by `read`, on the heap if
    /// it is `boxed`.
    fn boxed(read: String, boxed: bool) -> String {
        if boxed {
            format!("Box::new({read})")
        } else {
            read
        }
    }

    /// The functions of the contents of the SEQUENCE, SET or CHOICE `id`.
    fn content_functions(&mut self, id: TypeId, out: &mut String) {
        let name = self.names[&id].clone();
        let (argument, ty) = if self.lifetime.contains(&id) {
            (format!("{name}<'_>"), format!("{name}<'a>"))
        } else {
            (name.clone(), name.clone())
        };
        let definition = self
            .definitions
            .iter()
            .position(|definition| match definition {
                Definition::Struct { id: other, .. } | Definition::Choice { id: other, .. } => {
                    *other == id
                }
                Definition::Enum { .. } => false,
            });
        let Some(definition) = definition else {
            return;
        };

        match (self.kind(id), &self.definitions[definition]) {
            (Kind::Sequence(members) | Kind::Set(members), Definition::Struct { fields, .. }) => {
                let fields: Vec<(String, MemberInfo, Presence, bool)> = fields
                    .iter()
                    .map(|field| {
                        (
                            field.name.clone(),
                            field.member.clone(),
                            field.presence.clone(),
                            field.boxed,
                        )
                    })
                    .collect();
                let is_set = matches!(self.kind(id), Kind::Set(_));

                self.struct_functions(id, &name, &argument, &ty, members, is_set, &fields, out);
            }
            (Kind::Choice(members), Definition::Choice { variants, .. }) => {
                let variants: Vec<(String, MemberInfo, bool)> = variants
                    .iter()
                    .map(|variant| (variant.name.clone(), variant.member.clone(), variant.boxed))
                    .collect();

                self.choice_functions(id, &name, &argument, &ty, members, &variants, out);
            }
            _ => {}
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn struct_functions(
        &mut self,
        id: TypeId,
        name: &str,
        argument: &str,
        ty: &str,
        members: &Members,
        is_set: bool,
        fields: &[(String, MemberInfo, Presence, bool)],
        out: &mut String,
    ) {
        let root = members.root.len();
        let field_of = |member: &MemberInfo| {
            fields
                .iter()
                .find(|(_, candidate, _, _)| {
                    candidate.name == member.name && candidate.ty == member.ty
                })
                .expect("each member is a field")
        };
        // The root in the order of the codecs: that of the tags for a SET.
        let ordered: Vec<&MemberInfo> = if is_set {
            members
                .order
                .iter()
                .map(|index| &members.root[*index])
                .collect()
        } else {
            members.root.iter().collect()
        };
        let additions = &fields[root..];
        // The name of the variable of a field, which is not that of one of
        // the function.
        let local = |field: &str| format!("f_{}", field.trim_start_matches("r#"));

        // BER, writing.
        let mut body = String::new();

        if is_set {
            body.push_str("    let start = w.len();\n");
        }

        for member in ordered
            .iter()
            .copied()
            .chain(additions.iter().map(|field| &field.1))
        {
            let (field, member, presence, _) = field_of(member);
            let function = self.need(member.ty);

            match presence {
                Presence::Required => {
                    let _ = writeln!(body, "    ber_write_{function}(&v.{field}, w, der)?;");
                }
                Presence::Optional => {
                    let _ = writeln!(
                        body,
                        "    if let Some(x) = &v.{field} {{\n        ber_write_{function}(x, w, der)?;\n    }}"
                    );
                }
                Presence::Default(default) => {
                    let _ = writeln!(
                        body,
                        "    if {} {{\n        ber_write_{function}(&v.{field}, w, der)?;\n    }}",
                        self.not_default(member, &format!("&v.{field}"), default)
                    );
                }
            }
        }

        if is_set {
            body.push_str("    if der {\n        w.sort_members(start);\n    }\n");
        }

        let _ = write!(
            out,
            "#[allow(unused_variables)]\nfn ber_write_contents_{id}(v: &{argument}, w: &mut ber::Writer<'_>, der: bool) -> wire::Result<()> {{\n{body}    Ok(())\n}}\n\n"
        );

        // BER, reading.
        let mut body = String::new();
        let missing =
            "wire::Error::decode(\"A member is missing and has no default value.\", r.position())";
        let encoded_default = "wire::Error::decode(\"A member is encoded with its default value, which DER leaves out.\", r.position())";

        if is_set {
            // In any order, which DER has as that of the tags.
            for (field, _, _, _) in fields {
                let field = &local(field);
                let _ = writeln!(body, "    let mut {field} = None;");
            }

            if fields.is_empty() {
                // Nothing, or what a later version adds to it.
                if members.additions.is_some() {
                    body.push_str(
                        "    while r.peek_in(g)?.is_some() {\n        r.any()?;\n    }\n",
                    );
                } else {
                    let _ = writeln!(
                        body,
                        "    if r.peek_in(g)?.is_some() {{\n        return Err(wire::Error::decode(\"Expected the tag of a member.\", r.position()));\n    }}"
                    );
                }
            } else {
                body.push_str("    let mut last = None;\n");
                body.push_str("    while let Some(tag) = r.peek_in(g)? {\n");
                body.push_str("        if r.der() {\n            r.check_member_order(&mut last, tag)?;\n        }\n\n");
                body.push_str("        match tag {\n");

                let mut any = None;

                for (field, member, _, boxed) in fields {
                    let field = &local(field);
                    let function = self.need(member.ty);
                    let pattern = self.first_tags(member.ty);
                    let read = Generator::boxed(format!("ber_read_{function}(r)?"), *boxed);

                    if pattern == "_" {
                        any = Some(format!(
                            "            _ if {field}.is_none() => {field} = Some({read}),\n"
                        ));
                    } else {
                        let _ = writeln!(
                            body,
                            "            {pattern} if {field}.is_none() => {field} = Some({read}),"
                        );
                    }
                }

                if let Some(any) = any {
                    body.push_str(&any);
                }

                if members.additions.is_some() {
                    body.push_str("            _ => {\n                r.any()?;\n            }\n");
                } else {
                    let _ = writeln!(
                        body,
                        "            _ => return Err(wire::Error::decode(\"Expected the tag of a member.\", r.position())),"
                    );
                }

                body.push_str("        }\n    }\n\n");
            }

            for (field, member, presence, _) in fields {
                let field = &local(field);
                match presence {
                    Presence::Required => {
                        let _ =
                            writeln!(body, "    let {field} = {field}.ok_or_else(|| {missing})?;");
                    }
                    Presence::Optional => {}
                    Presence::Default(default) => {
                        let _ = writeln!(
                            body,
                            "    let {field} = match {field} {{\n        Some(x) if r.der() && !({}) => return Err({encoded_default}),\n        Some(x) => x,\n        None => {default},\n    }};",
                            self.not_default(member, "&x", default)
                        );
                    }
                }
            }
        } else {
            for (field, member, presence, boxed) in ordered
                .iter()
                .copied()
                .chain(additions.iter().map(|field| &field.1))
                .map(field_of)
            {
                let field = &local(field);
                let function = self.need(member.ty);
                let read = Generator::boxed(format!("ber_read_{function}(r)?"), *boxed);
                let found = match self.first_tags(member.ty).as_str() {
                    "_" => "r.peek_in(g)?.is_some()".to_string(),
                    tags => format!("matches!(r.peek_in(g)?, Some({tags}))"),
                };

                match presence {
                    Presence::Required => {
                        let _ = writeln!(
                            body,
                            "    if !r.more(g)? {{\n        return Err({missing});\n    }}\n    let {field} = {read};"
                        );
                    }
                    Presence::Optional => {
                        let _ = writeln!(
                            body,
                            "    let {field} = if {found} {{\n        Some({read})\n    }} else {{\n        None\n    }};"
                        );
                    }
                    Presence::Default(default) => {
                        let _ = writeln!(
                            body,
                            "    let {field} = if {found} {{\n        let x = {read};\n\n        if r.der() && !({}) {{\n            return Err({encoded_default});\n        }}\n\n        x\n    }} else {{\n        {default}\n    }};",
                            self.not_default(member, "&x", default)
                        );
                    }
                }
            }
        }

        let names: Vec<String> = fields
            .iter()
            .map(|(field, ..)| format!("{field}: {}", local(field)))
            .collect();
        let _ = write!(
            out,
            "#[allow(unused_variables, unreachable_code, clippy::let_unit_value)]\nfn ber_read_contents_{id}<'a>(r: &mut ber::Reader<'a>, g: &ber::Region) -> wire::Result<{ty}> {{\n{body}\n    Ok({name} {{ {} }})\n}}\n\n",
            names.join(", ")
        );

        // UPER, writing.
        let mut body = String::new();
        let extensible = members.additions.is_some();
        // What tells if each addition is there.
        let mut addition_presence = Vec::new();
        let mut field_index = root;

        for addition in members.additions.iter().flatten() {
            let count = match addition {
                Addition::Member(_) => 1,
                Addition::Group(group) => group.len(),
            };
            let present: Vec<String> = fields[field_index..field_index + count]
                .iter()
                .map(|(field, ..)| format!("v.{field}.is_some()"))
                .collect();

            addition_presence.push(present.join(" || "));
            field_index += count;
        }

        if extensible {
            for (index, present) in addition_presence.iter().enumerate() {
                let _ = writeln!(body, "    let e{index} = {present};");
            }

            let any = if addition_presence.is_empty() {
                "false".to_string()
            } else {
                (0..addition_presence.len())
                    .map(|index| format!("e{index}"))
                    .collect::<Vec<_>>()
                    .join(" || ")
            };

            let _ = writeln!(body, "    let extended = {any};\n    w.bit(extended);");
        }

        for member in &ordered {
            let (field, member, presence, _) = field_of(member);

            match presence {
                Presence::Required => {}
                Presence::Optional => {
                    let _ = writeln!(body, "    w.bit(v.{field}.is_some());");
                }
                Presence::Default(default) => {
                    let _ = writeln!(
                        body,
                        "    w.bit({});",
                        self.not_default(member, &format!("&v.{field}"), default)
                    );
                }
            }
        }

        for member in &ordered {
            let (field, member, presence, _) = field_of(member);
            let function = self.need(member.ty);

            match presence {
                Presence::Required => {
                    let _ = writeln!(body, "    uper_write_{function}(&v.{field}, w)?;");
                }
                Presence::Optional => {
                    let _ = writeln!(
                        body,
                        "    if let Some(x) = &v.{field} {{\n        uper_write_{function}(x, w)?;\n    }}"
                    );
                }
                Presence::Default(default) => {
                    let _ = writeln!(
                        body,
                        "    if {} {{\n        uper_write_{function}(&v.{field}, w)?;\n    }}",
                        self.not_default(member, &format!("&v.{field}"), default)
                    );
                }
            }
        }

        if extensible && !addition_presence.is_empty() {
            let count = addition_presence.len();
            let _ = writeln!(
                body,
                "    if extended {{\n        w.normally_small_length({count})?;"
            );

            for index in 0..count {
                let _ = writeln!(body, "        w.bit(e{index});");
            }

            body.push_str("        w.align();\n");

            let mut field_index = root;

            for (index, addition) in members.additions.iter().flatten().enumerate() {
                match addition {
                    Addition::Member(member) => {
                        let field = &fields[field_index].0;
                        let function = self.need(member.ty);

                        let _ = writeln!(
                            body,
                            "        if let Some(x) = &v.{field} {{\n            w.open_type(|w| uper_write_{function}(x, w))?;\n        }}"
                        );
                        field_index += 1;
                    }
                    Addition::Group(group) => {
                        let mut inner = String::new();

                        for (offset, member) in group.iter().enumerate() {
                            if member.optional || member.has_default() {
                                let field = &fields[field_index + offset].0;

                                let _ =
                                    writeln!(inner, "                w.bit(v.{field}.is_some());");
                            }
                        }

                        for (offset, member) in group.iter().enumerate() {
                            let field = &fields[field_index + offset].0;
                            let function = self.need(member.ty);

                            if member.optional || member.has_default() {
                                let _ = writeln!(
                                    inner,
                                    "                if let Some(x) = &v.{field} {{\n                    uper_write_{function}(x, w)?;\n                }}"
                                );
                            } else {
                                let _ = writeln!(
                                    inner,
                                    "                match &v.{field} {{\n                    Some(x) => uper_write_{function}(x, w)?,\n                    None => return Err(wire::Error::encode(\"A member is not found in the SEQUENCE or SET.\")),\n                }}"
                                );
                            }
                        }

                        let _ = writeln!(
                            body,
                            "        if e{index} {{\n            w.open_type(|w| {{\n{inner}                Ok(())\n            }})?;\n        }}"
                        );
                        field_index += group.len();
                    }
                }
            }

            body.push_str("    }\n");
        }

        let _ = write!(
            out,
            "#[allow(unused_variables)]\nfn uper_write_contents_{id}(v: &{argument}, w: &mut per::Writer<'_>) -> wire::Result<()> {{\n{body}    Ok(())\n}}\n\n"
        );

        // UPER, reading.
        let mut body = String::from("    r.enter()?;\n");

        if extensible {
            body.push_str("    let extended = r.bit()?;\n");
        }

        let mut bit = 0;
        let mut presence_bits = BTreeMap::new();

        for member in &ordered {
            let (field, _, presence, _) = field_of(member);
            let field = &local(field);

            if !matches!(presence, Presence::Required) {
                let _ = writeln!(body, "    let p{bit} = r.bit()?;");
                presence_bits.insert(field.clone(), bit);
                bit += 1;
            }
        }

        for member in &ordered {
            let (field, member, presence, boxed) = field_of(member);
            let field = &local(field);
            let function = self.need(member.ty);
            let read = Generator::boxed(format!("uper_read_{function}(r)?"), *boxed);

            match presence {
                Presence::Required => {
                    let _ = writeln!(body, "    let {field} = {read};");
                }
                Presence::Optional => {
                    let _ = writeln!(
                        body,
                        "    let {field} = if p{} {{\n        Some({read})\n    }} else {{\n        None\n    }};",
                        presence_bits[field]
                    );
                }
                Presence::Default(default) => {
                    let _ = writeln!(
                        body,
                        "    let {field} = if p{} {{\n        {read}\n    }} else {{\n        {default}\n    }};",
                        presence_bits[field]
                    );
                }
            }
        }

        if extensible {
            for (field, ..) in additions {
                let field = local(field);
                let _ = writeln!(body, "    let mut {field} = None;");
            }

            // Those of a later version are skipped.
            let skip = if additions.is_empty() {
                "            per::read_unbound_octets(r)?;\n"
            } else {
                "            match index {\n"
            };

            let _ = write!(
                body,
                "\n    if extended {{\n        let count = r.normally_small_length()?;\n        let presence = r.uint(count)?;\n\n        r.align();\n\n        for index in 0..count {{\n            if presence >> (count - index - 1) & 1 == 0 {{\n                continue;\n            }}\n\n{skip}"
            );

            let mut field_index = root;

            for (index, addition) in members.additions.iter().flatten().enumerate() {
                match addition {
                    Addition::Member(member) => {
                        let (field, _, _, boxed) = &fields[field_index];
                        let field = &local(field);
                        let function = self.need(member.ty);
                        let read = Generator::boxed(
                            format!("per::read_open_type(r, uper_read_{function})?"),
                            *boxed,
                        );

                        let _ =
                            writeln!(body, "                {index} => {field} = Some({read}),");
                        field_index += 1;
                    }
                    Addition::Group(group) => {
                        let mut inner = String::new();
                        let mut bit = 0;
                        let mut bits = Vec::new();

                        for member in group {
                            if member.optional || member.has_default() {
                                let _ = writeln!(
                                    inner,
                                    "                        let q{bit} = r.bit()?;"
                                );
                                bits.push(Some(bit));
                                bit += 1;
                            } else {
                                bits.push(None);
                            }
                        }

                        for (offset, member) in group.iter().enumerate() {
                            let (field, _, _, boxed) = &fields[field_index + offset];
                            let field = &local(field);
                            let function = self.need(member.ty);
                            let read =
                                Generator::boxed(format!("uper_read_{function}(r)?"), *boxed);

                            match bits[offset] {
                                Some(bit) => {
                                    let _ = writeln!(
                                        inner,
                                        "                        if q{bit} {{\n                            {field} = Some({read});\n                        }}"
                                    );
                                }
                                None => {
                                    let _ = writeln!(
                                        inner,
                                        "                        {field} = Some({read});"
                                    );
                                }
                            }
                        }

                        let _ = writeln!(
                            body,
                            "                {index} => per::read_open_type(r, |r| {{\n{inner}                        Ok(())\n                    }})?,"
                        );
                        field_index += group.len();
                    }
                }
            }

            if additions.is_empty() {
                body.push_str("        }\n    }\n");
            } else {
                body.push_str(
                    "                _ => {\n                    per::read_unbound_octets(r)?;\n                }\n            }\n        }\n    }\n",
                );
            }
        }

        body.push_str("\n    r.leave();\n");

        let _ = write!(
            out,
            "#[allow(unused_variables, unreachable_code, clippy::let_unit_value)]\nfn uper_read_contents_{id}<'a>(r: &mut per::Reader<'a>) -> wire::Result<{ty}> {{\n{body}\n    Ok({name} {{ {} }})\n}}\n\n",
            names.join(", ")
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn choice_functions(
        &mut self,
        id: TypeId,
        name: &str,
        argument: &str,
        ty: &str,
        members: &Members,
        variants: &[(String, MemberInfo, bool)],
        out: &mut String,
    ) {
        let root = members.root.len();
        let extensible = members.additions.is_some();

        // BER, writing.
        let mut arms = String::new();

        for (variant, member, _) in variants {
            let function = self.need(member.ty);

            let _ = writeln!(
                arms,
                "        {name}::{variant}(x) => ber_write_{function}(x, w, der),"
            );
        }

        let _ = write!(
            out,
            "fn ber_write_contents_{id}(v: &{argument}, w: &mut ber::Writer<'_>, der: bool) -> wire::Result<()> {{\n    match v {{\n{arms}    }}\n}}\n\n"
        );

        // BER, reading: the alternative of the tag.
        let mut arms = String::new();
        let mut any = None;

        for (variant, member, boxed) in variants {
            let function = self.need(member.ty);
            let pattern = self.first_tags(member.ty);
            let read = Generator::boxed(format!("ber_read_{function}(r)?"), *boxed);
            let arm = format!("        Some({pattern}) => Ok({name}::{variant}({read})),\n");

            if pattern == "_" {
                any = Some(arm);
            } else {
                arms.push_str(&arm);
            }
        }

        arms.push_str(&any.unwrap_or_else(|| {
            "        _ => Err(wire::Error::decode(\"Expected the tag of an alternative of the CHOICE.\", r.position())),\n".to_string()
        }));

        let _ = write!(
            out,
            "#[allow(unused_variables, unreachable_code, clippy::let_unit_value)]\nfn ber_read_contents_{id}<'a>(r: &mut ber::Reader<'a>) -> wire::Result<{ty}> {{\n    match r.peek_tag() {{\n{arms}    }}\n}}\n\n"
        );

        // UPER: an alternative of the root is its index in the order of
        // the tags, and an addition is in an open type.
        let additions: Vec<&MemberInfo> = members.all().skip(root).collect();
        let mut write_arms = String::new();
        let mut root_arms = String::new();
        let mut addition_arms = String::new();

        for (variant, member, boxed) in variants {
            let function = self.need(member.ty);
            let read = Generator::boxed(format!("uper_read_{function}(r)?"), *boxed);
            let root_index = members
                .root
                .iter()
                .position(|candidate| candidate == member);

            match root_index {
                Some(index) => {
                    let index = members
                        .order
                        .iter()
                        .position(|candidate| *candidate == index)
                        .unwrap_or(index);
                    let bit = if extensible {
                        "w.bit(false);\n            "
                    } else {
                        ""
                    };

                    let _ = writeln!(
                        write_arms,
                        "        {name}::{variant}(x) => {{\n            {bit}per::choice_index(w, {root}, {index});\n            uper_write_{function}(x, w)\n        }}"
                    );
                    let _ = writeln!(root_arms, "        {index} => {name}::{variant}({read}),");
                }
                None => {
                    let position = additions
                        .iter()
                        .position(|candidate| *candidate == member)
                        .unwrap_or_default();
                    let index = members
                        .addition_order
                        .iter()
                        .position(|candidate| *candidate == position)
                        .unwrap_or(position);
                    let read = Generator::boxed(
                        format!("per::read_open_type(r, uper_read_{function})?"),
                        *boxed,
                    );

                    let _ = writeln!(
                        write_arms,
                        "        {name}::{variant}(x) => {{\n            w.bit(true);\n            w.normally_small_whole_number({index});\n            w.open_type(|w| uper_write_{function}(x, w))\n        }}"
                    );
                    let _ = writeln!(
                        addition_arms,
                        "            {index} => {name}::{variant}({read}),"
                    );
                }
            }
        }

        let _ = write!(
            out,
            "fn uper_write_contents_{id}(v: &{argument}, w: &mut per::Writer<'_>) -> wire::Result<()> {{\n    match v {{\n{write_arms}    }}\n}}\n\n"
        );

        let unknown = "wire::Error::decode(\"Expected the index of an alternative of the CHOICE.\", r.position())";
        let mut body = String::from("    r.enter()?;\n\n");

        if extensible && addition_arms.is_empty() {
            let _ = write!(
                body,
                "    if r.bit()? {{\n        r.normally_small_whole_number()?;\n\n        return Err({unknown});\n    }}\n\n"
            );
        } else if extensible {
            let _ = write!(
                body,
                "    if r.bit()? {{\n        let v = match r.normally_small_whole_number()? {{\n{addition_arms}            _ => return Err({unknown}),\n        }};\n\n        r.leave();\n\n        return Ok(v);\n    }}\n\n"
            );
        }

        let _ = write!(
            body,
            "    let v = match per::read_choice_index(r, {root})? {{\n{root_arms}        _ => return Err({unknown}),\n    }};\n\n    r.leave();\n\n    Ok(v)\n"
        );

        let _ = write!(
            out,
            "#[allow(unused_variables, unreachable_code, clippy::let_unit_value)]\nfn uper_read_contents_{id}<'a>(r: &mut per::Reader<'a>) -> wire::Result<{ty}> {{\n{body}}}\n\n"
        );
    }

    // The text.

    fn definition_text(&self, definition: &Definition) -> String {
        let mut text = String::new();

        match definition {
            Definition::Struct { id, name, fields } => {
                let lifetime = if self.lifetime.contains(id) {
                    "<'a>"
                } else {
                    ""
                };

                let _ = writeln!(
                    text,
                    "#[derive(Debug, Clone, PartialEq)]\npub struct {name}{lifetime} {{"
                );

                for field in fields {
                    let mut ty = self.rust_type(field.member.ty);

                    if field.boxed {
                        ty = format!("Box<{ty}>");
                    }

                    if matches!(field.presence, Presence::Optional) {
                        ty = format!("Option<{ty}>");
                    }

                    let _ = writeln!(text, "    pub {}: {ty},", field.name);
                }

                text.push('}');
            }
            Definition::Choice { id, name, variants } => {
                let lifetime = if self.lifetime.contains(id) {
                    "<'a>"
                } else {
                    ""
                };

                let _ = writeln!(
                    text,
                    "#[derive(Debug, Clone, PartialEq)]\n#[allow(clippy::large_enum_variant, clippy::enum_variant_names)]\npub enum {name}{lifetime} {{"
                );

                for variant in variants {
                    let mut ty = self.rust_type(variant.member.ty);

                    if variant.boxed {
                        ty = format!("Box<{ty}>");
                    }

                    let _ = writeln!(text, "    {}({ty}),", variant.name);
                }

                text.push('}');
            }
            Definition::Enum {
                name,
                root,
                additions,
                variants,
            } => {
                let _ = writeln!(
                    text,
                    "#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]\npub enum {name} {{"
                );

                for variant in variants {
                    let _ = writeln!(text, "    {variant},");
                }

                text.push_str("}\n\n");

                let all: Vec<(&String, &(String, i64))> = variants
                    .iter()
                    .zip(root.iter().chain(additions.iter().flatten()))
                    .collect();
                let mut sorted: Vec<usize> = (0..root.len()).collect();

                sorted.sort_by_key(|index| root[*index].1);

                let _ = writeln!(
                    text,
                    "impl {name} {{\n    /// The number of the value.\n    pub fn number(self) -> i64 {{\n        match self {{"
                );

                for (variant, (_, number)) in &all {
                    let _ = writeln!(text, "            {name}::{variant} => {number},");
                }

                let _ = writeln!(
                    text,
                    "        }}\n    }}\n\n    /// The value of a number, if there is one.\n    pub fn from_number(number: i128) -> Option<{name}> {{\n        match number {{"
                );

                for (variant, (_, number)) in &all {
                    let _ = writeln!(text, "            {number} => Some({name}::{variant}),");
                }

                let _ = writeln!(
                    text,
                    "            _ => None,\n        }}\n    }}\n\n    /// The index of PER: of the root in the order of the numbers, or of\n    /// the additions.\n    fn uper_index(self) -> Result<usize, usize> {{\n        match self {{"
                );

                for (index, variant) in variants.iter().enumerate() {
                    let per_index = if index < root.len() {
                        format!(
                            "Ok({})",
                            sorted
                                .iter()
                                .position(|candidate| *candidate == index)
                                .unwrap_or(0)
                        )
                    } else {
                        format!("Err({})", index - root.len())
                    };

                    let _ = writeln!(text, "            {name}::{variant} => {per_index},");
                }

                let _ = writeln!(
                    text,
                    "        }}\n    }}\n\n    fn from_uper_index(index: Result<usize, usize>) -> Option<{name}> {{\n        match index {{"
                );

                for (index, variant) in variants.iter().enumerate() {
                    let per_index = if index < root.len() {
                        format!(
                            "Ok({})",
                            sorted
                                .iter()
                                .position(|candidate| *candidate == index)
                                .unwrap_or(0)
                        )
                    } else {
                        format!("Err({})", index - root.len())
                    };

                    let _ = writeln!(text, "            {per_index} => Some({name}::{variant}),");
                }

                text.push_str("            _ => None,\n        }\n    }\n}");
            }
        }

        text
    }

    /// The trait for a named struct or enum, or the functions of another
    /// named type.
    fn api_text(&self, named: &Named) -> String {
        let id = named.id;
        let ty = self.rust_type(id);
        let lifetime = if ty.contains("'a") { "<'a>" } else { "" };

        match &named.functions {
            None => format!(
                "impl<'a> Asn1<'a> for {ty} {{\n    fn write_ber(&self, w: &mut ber::Writer<'_>, der: bool) -> wire::Result<()> {{\n        ber_write_{id}(self, w, der)\n    }}\n\n    fn read_ber(r: &mut ber::Reader<'a>) -> wire::Result<Self> {{\n        ber_read_{id}(r)\n    }}\n\n    fn write_uper(&self, w: &mut per::Writer<'_>) -> wire::Result<()> {{\n        uper_write_{id}(self, w)\n    }}\n\n    fn read_uper(r: &mut per::Reader<'a>) -> wire::Result<Self> {{\n        uper_read_{id}(r)\n    }}\n}}"
            ),
            Some(module) => {
                let name = &named.name;
                let argument = self.argument_type(id);
                // The lifetime of what is decoded, if it borrows.
                let (generic, data) = if lifetime.is_empty() {
                    ("", "&[u8]")
                } else {
                    ("<'a>", "&'a [u8]")
                };

                format!(
                    "pub type {name}{lifetime} = {ty};\n\n/// The functions of [`{name}`], as the trait `Asn1` has them for the\n/// others.\npub mod {module} {{\n    use super::*;\n\n    pub fn encode_ber(value: &{argument}, out: &mut [u8]) -> wire::Result<usize> {{\n        wire::encode_ber_with(value, out, false, ber_write_{id})\n    }}\n\n    pub fn encode_der(value: &{argument}, out: &mut [u8]) -> wire::Result<usize> {{\n        wire::encode_ber_with(value, out, true, ber_write_{id})\n    }}\n\n    pub fn encode_uper(value: &{argument}, out: &mut [u8]) -> wire::Result<usize> {{\n        wire::encode_uper_with(value, out, uper_write_{id})\n    }}\n\n    pub fn decode_ber{generic}(data: {data}) -> wire::Result<{ty}> {{\n        wire::decode_ber_with(data, false, ber_read_{id})\n    }}\n\n    pub fn decode_der{generic}(data: {data}) -> wire::Result<{ty}> {{\n        wire::decode_ber_with(data, true, ber_read_{id})\n    }}\n\n    pub fn decode_uper{generic}(data: {data}) -> wire::Result<{ty}> {{\n        wire::decode_uper_with(data, uper_read_{id})\n    }}\n}}"
                )
            }
        }
    }

    fn run(mut self, sources: &[String]) -> Result<(String, Vec<GeneratedType>), Error> {
        let types = self.types;
        let mut count: BTreeMap<String, usize> = BTreeMap::new();

        for (_, assignments) in &types.modules {
            for (name, _) in assignments {
                *count.entry(camel(name)).or_default() += 1;
            }
        }

        // The named types that are supported, with their names.
        let mut skipped = Vec::new();
        let mut assigned = Vec::new();

        for (module, assignments) in &types.modules {
            for (type_name, id) in assignments {
                if let Err(what) = self.supported(*id) {
                    if !self.options.skip_unsupported {
                        return fail(format!(
                            "The type {module}.{type_name} is or holds {what}, which the plugin does not support yet."
                        ));
                    }

                    skipped.push(format!("{module}.{type_name} ({what})"));

                    continue;
                }

                let mut name = camel(type_name);

                if count[&name] > 1 {
                    name = format!("{}{name}", camel(module));
                }

                assigned.push((self.claim(name), *id, module.clone(), type_name.clone()));
            }
        }

        // The types of their own first, so that they have the names.
        let mut named = Vec::new();

        for (name, id, module, asn1) in &assigned {
            let own = self.types.nodes[*id].concrete == *id
                && matches!(
                    self.kind(*id),
                    Kind::Sequence(_) | Kind::Set(_) | Kind::Choice(_) | Kind::Enumerated { .. }
                )
                && !self.names.contains_key(id);

            if own {
                self.names.insert(*id, name.clone());
            }

            named.push((name.clone(), *id, own, module.clone(), asn1.clone()));
        }

        for (name, id, own, _, _) in &named {
            if *own {
                self.names.remove(id);
                self.define(*id, name.clone());
            } else {
                self.visit(*id, name);
            }
        }

        let mut named: Vec<Named> = named
            .into_iter()
            .map(|(name, id, own, module, asn1)| Named {
                id,
                module,
                asn1,
                functions: (!own).then(|| self.claim_function(snake(&name))),
                name,
            })
            .collect();

        self.find_lifetimes();
        self.boxes = self.definitions.iter().any(|definition| match definition {
            Definition::Struct { fields, .. } => fields.iter().any(|field| field.boxed),
            Definition::Choice { variants, .. } => variants.iter().any(|variant| variant.boxed),
            Definition::Enum { .. } => false,
        });

        // The functions of the named types and of all in them.
        for named in &named {
            self.need(named.id);
        }

        let mut functions = String::new();

        loop {
            if let Some(id) = self.queue.pop() {
                self.node_functions(id, &mut functions);
            } else if let Some(id) = self.content_queue.pop() {
                self.content_functions(id, &mut functions);
            } else {
                break;
            }
        }

        named.sort_by_key(|named| named.id);

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
                .replace(
                    "{alloc}",
                    if self.options.no_std && self.boxes {
                        "\nuse alloc::boxed::Box;"
                    } else {
                        ""
                    },
                ),
        ];

        if !skipped.is_empty() {
            parts.push(format!(
                "// Left out, as they are or hold what the plugin does not support yet:\n//\n{}",
                skipped
                    .iter()
                    .map(|skipped| format!("//   {skipped}"))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }

        let mut definitions = std::mem::take(&mut self.definitions);

        definitions.sort_by_key(|definition| match definition {
            Definition::Struct { id, .. } | Definition::Choice { id, .. } => *id,
            Definition::Enum { name, .. } => self
                .names
                .iter()
                .find(|(_, candidate)| *candidate == name)
                .map_or(0, |(id, _)| *id),
        });

        for definition in &definitions {
            parts.push(self.definition_text(definition));
        }

        self.definitions = definitions;

        for named in &named {
            parts.push(self.api_text(named));
        }

        for (literal, name) in &self.knowns {
            parts.push(format!("static {name}: per::Known = {literal};"));
        }

        // A program uses some of the types, and so of the functions.
        let mut lines: Vec<String> = Vec::new();

        for line in functions.trim_end().lines() {
            if line.starts_with("fn ") {
                match lines.last_mut() {
                    Some(previous) if previous.starts_with("#[allow(") => {
                        *previous = previous.replacen("#[allow(", "#[allow(dead_code, ", 1);
                    }
                    _ => lines.push("#[allow(dead_code)]".to_string()),
                }
            }

            lines.push(line.to_string());
        }

        parts.push(lines.join("\n"));

        let generated = named
            .iter()
            .map(|named| GeneratedType {
                module: named.module.clone(),
                name: named.asn1.clone(),
                rust: named.name.clone(),
                functions: named.functions.clone(),
            })
            .collect();

        Ok((parts.join("\n\n") + "\n", generated))
    }
}

const HEADER: &str = r#"// Generated by asnkit-gen-rust-codec {version} from {sources}. Do not edit.
//
// This file is made from the specification it names, and needs the asnkitrt
// crate (MIT) to run. It may be used under terms of your choice: the code that
// asnkit-gen-rust-codec copies into it is not under the Apache License of the
// plugin.
{alloc}
#[allow(unused_imports)]
use asnkitrt::wire::per::BitSink;
#[allow(unused_imports)]
use asnkitrt::wire::{self, Asn1, ber, per};"#;

/// The Rust module for the contents of an `.asni` file, which must not
/// be signed, encrypted, bound to something or have a nonce to check.
/// Returns it with the names of the files it was made from.
pub fn generate(asni: &[u8], options: Options) -> Result<(String, Vec<String>), Error> {
    generate_types(asni, options).map(|(text, sources, _)| (text, sources))
}

/// The same, with the types that are in the module.
pub fn generate_types(
    asni: &[u8],
    options: Options,
) -> Result<(String, Vec<String>, Vec<GeneratedType>), Error> {
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
    let (text, generated) = Generator::new(specification.types(), options).run(&sources)?;

    Ok((text, sources, generated))
}
