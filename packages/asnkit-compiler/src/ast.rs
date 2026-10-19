// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The nodes of a syntax tree. `to_tree()` gives a node in the readable
//! form (see `docs/file_formats.md`): a map with the name of its kind
//! as `kind` and its fields by name, where a field that is `None` is
//! left out and any other is there.

use serde_json::{Map, Number, Value as Json};
use std::cell::Cell;

/// What a tree is written for: a syntax tree, with the constraints as
/// they are written, or a compiled specification, with what the compiler
/// made of them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    SyntaxTree,
    Compiled,
}

thread_local! {
    static FORM: Cell<Form> = const { Cell::new(Form::SyntaxTree) };
}

/// Writes trees for `form` while `write` runs.
pub fn with_form<T>(form: Form, write: impl FnOnce() -> T) -> T {
    let before = FORM.with(|cell| cell.replace(form));
    let result = write();

    FORM.with(|cell| cell.set(before));

    result
}

fn form() -> Form {
    FORM.with(Cell::get)
}

/// The index of the file, the line and the column, counted from 0 for the
/// file and from 1 for the others.
pub type Location = (usize, usize, usize);

fn node(kind: &str, fields: Vec<(&str, Option<Json>)>) -> Json {
    let mut map = Map::new();
    map.insert("kind".to_string(), Json::String(kind.to_string()));

    for (name, value) in fields {
        if let Some(value) = value {
            map.insert(name.to_string(), value);
        }
    }

    Json::Object(map)
}

fn text(value: &str) -> Option<Json> {
    Some(Json::String(value.to_string()))
}

fn opt_text(value: &Option<String>) -> Option<Json> {
    value.as_deref().and_then(text)
}

fn list<T>(items: &[T], to_tree: impl Fn(&T) -> Json) -> Option<Json> {
    Some(Json::Array(items.iter().map(to_tree).collect()))
}

fn opt_list<T>(items: &Option<Vec<T>>, to_tree: impl Fn(&T) -> Json) -> Option<Json> {
    items.as_ref().and_then(|items| list(items, to_tree))
}

fn location(location: &Option<Location>) -> Option<Json> {
    location.map(|(file, line, column)| {
        Json::Array(vec![Json::from(file), Json::from(line), Json::from(column)])
    })
}

/// A value, or anything else found where one is: the bound of a range,
/// an element of a constraint, an actual parameter, the setting of a
/// field of an object.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Number(Number),
    Bool(bool),
    /// A character string.
    Str(String),
    List(Vec<Value>),
    Identifier(String),
    BinaryString(String),
    HexString(String),
    Null,
    PlusInfinity,
    MinusInfinity,
    NotANumber,
    UnsupportedValue,
    Min,
    Max,
    ExtensionMarker,
    NamedValue(String, Box<Value>),
    SequenceValue(Vec<(String, Value)>),
    Excluded(Box<Value>),
    Range(Box<Value>, Box<Value>),
    Type(Box<Type>),
    Component(Box<ComponentConstraint>),
    Element(Box<ElementConstraint>),
    Object(Vec<ObjectField>),
    UnresolvedObject(String),
    ObjectSet(Vec<Value>),
}

impl Value {
    pub fn int(value: i64) -> Value {
        Value::Number(Number::from(value))
    }

    pub fn to_tree(&self) -> Json {
        match self {
            Value::Number(number) => Json::Number(number.clone()),
            Value::Bool(value) => Json::Bool(*value),
            Value::Str(value) => Json::String(value.clone()),
            Value::List(items) => Json::Array(items.iter().map(Value::to_tree).collect()),
            Value::Identifier(name) => node("Identifier", vec![("name", text(name))]),
            Value::BinaryString(bits) => node("BinaryString", vec![("bits", text(bits))]),
            Value::HexString(digits) => node("HexString", vec![("digits", text(digits))]),
            Value::Null => node("Null", vec![]),
            Value::PlusInfinity => node("PlusInfinity", vec![]),
            Value::MinusInfinity => node("MinusInfinity", vec![]),
            Value::NotANumber => node("NotANumber", vec![]),
            Value::UnsupportedValue => node("UnsupportedValue", vec![]),
            Value::Min => node("Min", vec![]),
            Value::Max => node("Max", vec![]),
            Value::ExtensionMarker => node("ExtensionMarker", vec![]),
            Value::NamedValue(name, value) => node(
                "NamedValue",
                vec![("name", text(name)), ("value", Some(value.to_tree()))],
            ),
            Value::SequenceValue(members) => node(
                "SequenceValue",
                vec![(
                    "members",
                    list(members, |(name, value)| {
                        node(
                            "NamedValue",
                            vec![("name", text(name)), ("value", Some(value.to_tree()))],
                        )
                    }),
                )],
            ),
            Value::Excluded(value) => node("Excluded", vec![("value", Some(value.to_tree()))]),
            Value::Range(lower, upper) => node(
                "Range",
                vec![
                    ("lower", Some(lower.to_tree())),
                    ("upper", Some(upper.to_tree())),
                ],
            ),
            Value::Type(type_) => type_.to_tree(),
            Value::Component(component) => component.to_tree(),
            Value::Element(element) => element.to_tree(),
            Value::Object(fields) => node(
                "Object",
                vec![("fields", list(fields, ObjectField::to_tree))],
            ),
            Value::UnresolvedObject(source) => {
                node("UnresolvedObject", vec![("source", text(source))])
            }
            Value::ObjectSet(members) => node(
                "ObjectSet",
                vec![("members", list(members, Value::to_tree))],
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectField {
    pub name: String,
    pub setting: Value,
}

impl ObjectField {
    pub fn to_tree(&self) -> Json {
        node(
            "ObjectField",
            vec![
                ("name", text(&self.name)),
                ("setting", Some(self.setting.to_tree())),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Tag {
    pub number: Value,
    pub tag_class: Option<String>,
    pub mode: Option<String>,
}

impl Tag {
    pub fn to_tree(&self) -> Json {
        node(
            "Tag",
            vec![
                ("number", Some(self.number.to_tree())),
                ("tag_class", opt_text(&self.tag_class)),
                ("mode", opt_text(&self.mode)),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedNumber {
    pub name: String,
    pub number: Value,
}

impl NamedNumber {
    pub fn to_tree(&self) -> Json {
        node(
            "NamedNumber",
            vec![
                ("name", text(&self.name)),
                ("number", Some(self.number.to_tree())),
            ],
        )
    }
}

/// The constraints any type has, and what a parameterized type is.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Common {
    pub tag: Option<Tag>,
    pub size: Option<Vec<Value>>,
    pub restricted_to: Option<Vec<Value>>,
    pub permitted_alphabet: Option<Vec<Value>>,
    pub with_components: Option<Vec<Value>>,
    pub with_component: Option<Vec<Value>>,
    pub table: Option<TableConstraint>,
    pub parameters: Option<Vec<Parameter>>,
    pub actual_parameters: Option<Vec<Value>>,
    pub module_name: Option<String>,
    /// The constraints as they are written, each applied to the type with
    /// those before it (X.680 49.1, 50.11). Only in a syntax tree: a compiled
    /// specification has what the compiler made of them in the fields
    /// above.
    pub constraints: Option<Vec<Constraint>>,
}

impl Common {
    fn fields(&self) -> Vec<(&'static str, Option<Json>)> {
        if form() == Form::SyntaxTree {
            return vec![
                ("tag", self.tag.as_ref().map(Tag::to_tree)),
                ("parameters", opt_list(&self.parameters, Parameter::to_tree)),
                (
                    "actual_parameters",
                    opt_list(&self.actual_parameters, Value::to_tree),
                ),
                ("module_name", opt_text(&self.module_name)),
                (
                    "constraints",
                    opt_list(&self.constraints, Constraint::to_tree),
                ),
            ];
        }

        vec![
            ("tag", self.tag.as_ref().map(Tag::to_tree)),
            ("size", opt_list(&self.size, Value::to_tree)),
            (
                "restricted_to",
                opt_list(&self.restricted_to, Value::to_tree),
            ),
            (
                "permitted_alphabet",
                opt_list(&self.permitted_alphabet, Value::to_tree),
            ),
            (
                "with_components",
                opt_list(&self.with_components, Value::to_tree),
            ),
            (
                "with_component",
                opt_list(&self.with_component, Value::to_tree),
            ),
            ("table", self.table.as_ref().map(TableConstraint::to_tree)),
            ("parameters", opt_list(&self.parameters, Parameter::to_tree)),
            (
                "actual_parameters",
                opt_list(&self.actual_parameters, Value::to_tree),
            ),
            ("module_name", opt_text(&self.module_name)),
        ]
    }
}

/// A constraint, as written in parentheses after a type (X.680 49.6).
#[derive(Debug, Clone, PartialEq)]
pub enum Constraint {
    /// Sets of values: the root, and if it is extensible, the additions.
    ElementSetSpecs {
        root: ElementSet,
        extensible: bool,
        additions: Option<ElementSet>,
    },
    /// `{ObjectSet}` or `{ObjectSet}{@component}` (X.682 10).
    Table(TableConstraint),
    /// `CONTAINING Type` and `ENCODED BY value` (X.682 11).
    Contents {
        type_: Option<Box<Type>>,
        encoded_by: Option<Value>,
    },
    /// `CONSTRAINED BY { ... }` (X.682 9).
    UserDefined,
    /// A table constraint of a set of objects that is not given by name,
    /// which is not supported.
    Unsupported,
}

impl Constraint {
    pub fn to_tree(&self) -> Json {
        match self {
            Constraint::ElementSetSpecs {
                root,
                extensible,
                additions,
            } => node(
                "ElementSetSpecs",
                vec![
                    ("root", Some(root.to_tree())),
                    ("extensible", Some(Json::Bool(*extensible))),
                    ("additions", additions.as_ref().map(ElementSet::to_tree)),
                ],
            ),
            Constraint::Table(table) => table.to_tree(),
            Constraint::Contents { type_, encoded_by } => node(
                "ContentsConstraint",
                vec![
                    ("type", type_.as_ref().map(|type_| type_.to_tree())),
                    ("encoded_by", encoded_by.as_ref().map(Value::to_tree)),
                ],
            ),
            Constraint::UserDefined => node("UserDefinedConstraint", vec![]),
            Constraint::Unsupported => node("UnsupportedConstraint", vec![]),
        }
    }
}

/// A set of values of a constraint (X.680 50).
#[derive(Debug, Clone, PartialEq)]
pub enum ElementSet {
    Union(Vec<ElementSet>),
    Intersection(Vec<ElementSet>),
    /// The values of the first set but those of the second.
    Except(Box<ElementSet>, Box<ElementSet>),
    /// `ALL EXCEPT` the values of a set.
    All(Box<ElementSet>),
    Element(Element),
}

impl ElementSet {
    pub fn to_tree(&self) -> Json {
        match self {
            ElementSet::Union(sets) => {
                node("Union", vec![("elements", list(sets, ElementSet::to_tree))])
            }
            ElementSet::Intersection(sets) => node(
                "Intersection",
                vec![("elements", list(sets, ElementSet::to_tree))],
            ),
            ElementSet::Except(set, exclusions) => node(
                "Except",
                vec![
                    ("elements", Some(set.to_tree())),
                    ("exclusions", Some(exclusions.to_tree())),
                ],
            ),
            ElementSet::All(exclusions) => {
                node("All", vec![("exclusions", Some(exclusions.to_tree()))])
            }
            ElementSet::Element(element) => element.to_tree(),
        }
    }
}

/// An element of a set of values of a constraint (X.680 51).
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    /// A single value, or a range as a `Range` value.
    Value(Value),
    /// A contained subtype, with `INCLUDES` or not.
    ContainedSubtype {
        type_: Box<Type>,
        includes: bool,
    },
    Size(Box<Constraint>),
    PermittedAlphabet(Box<Constraint>),
    Pattern(Value),
    /// `WITH COMPONENT`.
    InnerType(Box<Constraint>),
    /// `WITH COMPONENTS`, of which a partial one starts with `...`.
    InnerTypes {
        partial: bool,
        components: Vec<NamedConstraint>,
    },
}

impl Element {
    pub fn to_tree(&self) -> Json {
        match self {
            Element::Value(value) => value.to_tree(),
            Element::ContainedSubtype { type_, includes } => node(
                "ContainedSubtype",
                vec![
                    ("type", Some(type_.to_tree())),
                    ("includes", Some(Json::Bool(*includes))),
                ],
            ),
            Element::Size(constraint) => node(
                "SizeConstraint",
                vec![("constraint", Some(constraint.to_tree()))],
            ),
            Element::PermittedAlphabet(constraint) => node(
                "PermittedAlphabet",
                vec![("constraint", Some(constraint.to_tree()))],
            ),
            Element::Pattern(value) => {
                node("PatternConstraint", vec![("value", Some(value.to_tree()))])
            }
            Element::InnerType(constraint) => node(
                "InnerTypeConstraint",
                vec![("constraint", Some(constraint.to_tree()))],
            ),
            Element::InnerTypes {
                partial,
                components,
            } => node(
                "InnerTypeConstraints",
                vec![
                    ("partial", Some(Json::Bool(*partial))),
                    ("components", list(components, NamedConstraint::to_tree)),
                ],
            ),
        }
    }
}

/// A component in `WITH COMPONENTS`, with its constraints, each applied
/// with those before it, and its presence.
#[derive(Debug, Clone, PartialEq)]
pub struct NamedConstraint {
    pub name: String,
    pub constraints: Option<Vec<Constraint>>,
    pub presence: Option<String>,
}

impl NamedConstraint {
    pub fn to_tree(&self) -> Json {
        node(
            "NamedConstraint",
            vec![
                ("name", text(&self.name)),
                (
                    "constraints",
                    opt_list(&self.constraints, Constraint::to_tree),
                ),
                ("presence", opt_text(&self.presence)),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TableConstraint {
    pub object_set: String,
    pub component_ids: Option<Vec<String>>,
}

impl TableConstraint {
    pub fn to_tree(&self) -> Json {
        node(
            "TableConstraint",
            vec![
                ("object_set", text(&self.object_set)),
                (
                    "component_ids",
                    opt_list(&self.component_ids, |id| Json::String(id.clone())),
                ),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub name: String,
    pub governor: Option<Type>,
}

impl Parameter {
    pub fn to_tree(&self) -> Json {
        node(
            "Parameter",
            vec![
                ("name", text(&self.name)),
                ("governor", self.governor.as_ref().map(Type::to_tree)),
            ],
        )
    }
}

/// The constraints of the elements of a `SEQUENCE OF` in `WITH
/// COMPONENT`, and of a component in `WITH COMPONENTS`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ElementConstraint {
    pub size: Option<Vec<Value>>,
    pub restricted_to: Option<Vec<Value>>,
    pub permitted_alphabet: Option<Vec<Value>>,
    pub with_components: Option<Vec<Value>>,
    pub with_component: Option<Vec<Value>>,
    pub containing: Option<Box<Type>>,
}

impl ElementConstraint {
    fn fields(&self) -> Vec<(&'static str, Option<Json>)> {
        vec![
            ("size", opt_list(&self.size, Value::to_tree)),
            (
                "restricted_to",
                opt_list(&self.restricted_to, Value::to_tree),
            ),
            (
                "permitted_alphabet",
                opt_list(&self.permitted_alphabet, Value::to_tree),
            ),
            (
                "with_components",
                opt_list(&self.with_components, Value::to_tree),
            ),
            (
                "with_component",
                opt_list(&self.with_component, Value::to_tree),
            ),
            ("containing", self.containing.as_ref().map(|t| t.to_tree())),
        ]
    }

    pub fn to_tree(&self) -> Json {
        node("ElementConstraint", self.fields())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentConstraint {
    pub name: String,
    pub presence: Option<String>,
    pub constraint: ElementConstraint,
}

impl ComponentConstraint {
    pub fn to_tree(&self) -> Json {
        let mut fields = vec![
            ("name", text(&self.name)),
            ("presence", opt_text(&self.presence)),
        ];
        fields.extend(self.constraint.fields());

        node("ComponentConstraint", fields)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Builtin(String),
    Referenced(String),
    Integer(Option<Vec<NamedNumber>>),
    BitString(Option<Vec<NamedNumber>>),
    Enumerated(Vec<EnumItem>),
    Sequence(Vec<MemberItem>),
    Set(Vec<MemberItem>),
    Choice(Vec<MemberItem>),
    SequenceOf(Box<Type>, Option<String>),
    SetOf(Box<Type>, Option<String>),
    AnyDefinedBy(String, Vec<(Value, Type)>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumItem {
    Number(NamedNumber),
    ExtensionMarker,
}

#[derive(Debug, Clone, PartialEq)]
pub enum MemberItem {
    Member(Member),
    ComponentsOf(String),
    ExtensionMarker,
    Group(Vec<MemberItem>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub body: Body,
    pub common: Common,
}

impl Type {
    /// The name of the type as the Python parser gives it.
    pub fn type_name(&self) -> String {
        match &self.body {
            Body::Builtin(name) | Body::Referenced(name) => name.clone(),
            Body::Integer(_) => "INTEGER".to_string(),
            Body::BitString(_) => "BIT STRING".to_string(),
            Body::Enumerated(_) => "ENUMERATED".to_string(),
            Body::Sequence(_) => "SEQUENCE".to_string(),
            Body::Set(_) => "SET".to_string(),
            Body::Choice(_) => "CHOICE".to_string(),
            Body::SequenceOf(..) => "SEQUENCE OF".to_string(),
            Body::SetOf(..) => "SET OF".to_string(),
            Body::AnyDefinedBy(..) => "ANY DEFINED BY".to_string(),
        }
    }

    pub fn new(body: Body) -> Type {
        Type {
            body,
            common: Common::default(),
        }
    }

    pub fn to_tree(&self) -> Json {
        let mut fields: Vec<(&str, Option<Json>)> = Vec::new();

        let kind = match &self.body {
            Body::Builtin(name) => {
                fields.push(("name", text(name)));
                "BuiltinType"
            }
            Body::Referenced(name) => {
                fields.push(("name", text(name)));
                "ReferencedType"
            }
            Body::Integer(numbers) => {
                fields.push(("named_numbers", opt_list(numbers, NamedNumber::to_tree)));
                "IntegerType"
            }
            Body::BitString(bits) => {
                fields.push(("named_bits", opt_list(bits, NamedNumber::to_tree)));
                "BitStringType"
            }
            Body::Enumerated(items) => {
                fields.push((
                    "values",
                    list(items, |item| match item {
                        EnumItem::Number(number) => number.to_tree(),
                        EnumItem::ExtensionMarker => node("ExtensionMarker", vec![]),
                    }),
                ));
                "EnumeratedType"
            }
            Body::Sequence(members) => {
                fields.push(("members", list(members, MemberItem::to_tree)));
                "SequenceType"
            }
            Body::Set(members) => {
                fields.push(("members", list(members, MemberItem::to_tree)));
                "SetType"
            }
            Body::Choice(members) => {
                fields.push(("members", list(members, MemberItem::to_tree)));
                "ChoiceType"
            }
            Body::SequenceOf(element, name) => {
                fields.push(("element", Some(element.to_tree())));
                fields.push(("element_name", opt_text(name)));
                "SequenceOfType"
            }
            Body::SetOf(element, name) => {
                fields.push(("element", Some(element.to_tree())));
                fields.push(("element_name", opt_text(name)));
                "SetOfType"
            }
            Body::AnyDefinedBy(value, choices) => {
                fields.push(("value", text(value)));
                fields.push((
                    "choices",
                    list(choices, |(value, type_)| {
                        node(
                            "AnyDefinedByChoice",
                            vec![
                                ("value", Some(value.to_tree())),
                                ("type", Some(type_.to_tree())),
                            ],
                        )
                    }),
                ));
                "AnyDefinedByType"
            }
        };

        fields.extend(self.common.fields());

        node(kind, fields)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub name: String,
    pub type_: Type,
    pub optional: bool,
    pub default: Option<Value>,
    /// What the default was before it was made a value, and the name of
    /// the type it is a default of, for the conversions that need to know
    /// the types of the whole specification.
    pub raw_default: Option<(crate::data::Data, String)>,
    pub location: Option<Location>,
}

impl MemberItem {
    pub fn to_tree(&self) -> Json {
        match self {
            MemberItem::Member(member) => node(
                "Member",
                vec![
                    ("name", text(&member.name)),
                    ("type", Some(member.type_.to_tree())),
                    ("optional", Some(Json::Bool(member.optional))),
                    ("default", member.default.as_ref().map(Value::to_tree)),
                    ("location", location(&member.location)),
                ],
            ),
            MemberItem::ComponentsOf(name) => node("ComponentsOf", vec![("type_name", text(name))]),
            MemberItem::ExtensionMarker => node("ExtensionMarker", vec![]),
            MemberItem::Group(members) => node(
                "ExtensionAdditionGroup",
                vec![("members", list(members, MemberItem::to_tree))],
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAssignment {
    pub name: String,
    pub type_: Type,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ValueAssignment {
    pub name: String,
    pub type_: String,
    pub value: Value,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectAssignment {
    pub name: String,
    pub class_name: String,
    pub object: Value,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectClassField {
    pub name: String,
    pub type_: Type,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxToken {
    Word(String),
    Optional(Vec<SyntaxToken>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectClassAssignment {
    pub name: String,
    pub fields: Vec<ObjectClassField>,
    pub syntax: Option<Vec<SyntaxToken>>,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectSetAssignment {
    pub name: String,
    pub class_name: String,
    pub members: Vec<Value>,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub module: String,
    pub symbols: Vec<String>,
}

/// A value assignment, or an object assignment, as it was found. Which it
/// is is told once the whole specification is known.
#[derive(Debug, Clone, PartialEq)]
pub struct RawValue {
    pub name: String,
    pub type_name: String,
    pub value: RawData,
    pub location: Option<Location>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum RawData {
    Data(crate::data::Data),
    /// An object given with the fields of its class, `{&id 1, &Type X}`.
    Fields(Vec<ObjectField>),
    /// An object given with the syntax of its class, or anything else in
    /// braces: its text.
    DefinedSyntax(String),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct Module {
    pub name: String,
    pub raw_values: Vec<RawValue>,
    pub tags: Option<String>,
    pub extensibility_implied: bool,
    pub imports: Vec<Import>,
    pub types: Vec<TypeAssignment>,
    pub values: Vec<ValueAssignment>,
    pub objects: Vec<ObjectAssignment>,
    pub object_classes: Vec<ObjectClassAssignment>,
    pub object_sets: Vec<ObjectSetAssignment>,
    pub location: Option<Location>,
}

fn syntax_tree(tokens: &[SyntaxToken]) -> Json {
    Json::Array(
        tokens
            .iter()
            .map(|token| match token {
                SyntaxToken::Word(word) => Json::String(word.clone()),
                SyntaxToken::Optional(tokens) => {
                    node("OptionalGroup", vec![("tokens", Some(syntax_tree(tokens)))])
                }
            })
            .collect(),
    )
}

impl Module {
    pub fn to_tree(&self) -> Json {
        node(
            "Module",
            vec![
                ("name", text(&self.name)),
                ("tags", opt_text(&self.tags)),
                (
                    "extensibility_implied",
                    Some(Json::Bool(self.extensibility_implied)),
                ),
                (
                    "imports",
                    list(&self.imports, |import| {
                        node(
                            "Import",
                            vec![
                                ("module", text(&import.module)),
                                (
                                    "symbols",
                                    list(&import.symbols, |s| Json::String(s.clone())),
                                ),
                            ],
                        )
                    }),
                ),
                (
                    "types",
                    list(&self.types, |t| {
                        node(
                            "TypeAssignment",
                            vec![
                                ("name", text(&t.name)),
                                ("type", Some(t.type_.to_tree())),
                                ("location", location(&t.location)),
                            ],
                        )
                    }),
                ),
                (
                    "values",
                    list(&self.values, |v| {
                        node(
                            "ValueAssignment",
                            vec![
                                ("name", text(&v.name)),
                                ("type", text(&v.type_)),
                                ("value", Some(v.value.to_tree())),
                                ("location", location(&v.location)),
                            ],
                        )
                    }),
                ),
                (
                    "objects",
                    list(&self.objects, |o| {
                        node(
                            "ObjectAssignment",
                            vec![
                                ("name", text(&o.name)),
                                ("class_name", text(&o.class_name)),
                                ("object", Some(o.object.to_tree())),
                                ("location", location(&o.location)),
                            ],
                        )
                    }),
                ),
                (
                    "object_classes",
                    list(&self.object_classes, |c| {
                        node(
                            "ObjectClassAssignment",
                            vec![
                                ("name", text(&c.name)),
                                (
                                    "fields",
                                    list(&c.fields, |f| {
                                        node(
                                            "ObjectClassField",
                                            vec![
                                                ("name", text(&f.name)),
                                                ("type", Some(f.type_.to_tree())),
                                                ("location", location(&f.location)),
                                            ],
                                        )
                                    }),
                                ),
                                ("syntax", c.syntax.as_ref().map(|s| syntax_tree(s))),
                                ("location", location(&c.location)),
                            ],
                        )
                    }),
                ),
                (
                    "object_sets",
                    list(&self.object_sets, |s| {
                        node(
                            "ObjectSetAssignment",
                            vec![
                                ("name", text(&s.name)),
                                ("class_name", text(&s.class_name)),
                                ("members", list(&s.members, Value::to_tree)),
                                ("location", location(&s.location)),
                            ],
                        )
                    }),
                ),
                ("location", location(&self.location)),
            ],
        )
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct SyntaxTree {
    pub modules: Vec<Module>,
}

impl SyntaxTree {
    pub fn to_tree(&self) -> Json {
        node(
            "SyntaxTree",
            vec![("modules", list(&self.modules, Module::to_tree))],
        )
    }
}
