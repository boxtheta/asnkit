// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The nodes of a compiled specification (kinds 1 to 40 of
//! `docs/file_formats.md`), read from the compact form.

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

use crate::cbor::Cbor;
use crate::kinds::number;
use crate::{Error, Result};

const COMPILED_SPECIFICATION: u8 = number("CompiledSpecification");
const COMPILED_MODULE: u8 = number("CompiledModule");
const TYPE_ASSIGNMENT: u8 = number("TypeAssignment");
const MEMBER: u8 = number("Member");
const RANGE: u8 = number("Range");
const NAMED_NUMBER: u8 = number("NamedNumber");
const EXTENSION_MARKER: u8 = number("ExtensionMarker");
const EXTENSION_ADDITION_GROUP: u8 = number("ExtensionAdditionGroup");
const BUILTIN_TYPE: u8 = number("BuiltinType");
const REFERENCED_TYPE: u8 = number("ReferencedType");
const INTEGER_TYPE: u8 = number("IntegerType");
const BIT_STRING_TYPE: u8 = number("BitStringType");
const ENUMERATED_TYPE: u8 = number("EnumeratedType");
const SEQUENCE_TYPE: u8 = number("SequenceType");
const SET_TYPE: u8 = number("SetType");
const CHOICE_TYPE: u8 = number("ChoiceType");
const SEQUENCE_OF_TYPE: u8 = number("SequenceOfType");
const SET_OF_TYPE: u8 = number("SetOfType");
const MIN: u8 = number("Min");
const MAX: u8 = number("Max");
const IDENTIFIER: u8 = number("Identifier");
const HEX_STRING: u8 = number("HexString");
const BINARY_STRING: u8 = number("BinaryString");
const NULL: u8 = number("Null");
const PLUS_INFINITY: u8 = number("PlusInfinity");
const MINUS_INFINITY: u8 = number("MinusInfinity");
const NOT_A_NUMBER: u8 = number("NotANumber");
const NAMED_VALUE: u8 = number("NamedValue");
const SEQUENCE_VALUE: u8 = number("SequenceValue");
const UNSUPPORTED_VALUE: u8 = number("UnsupportedValue");
const EXCLUDED: u8 = number("Excluded");
const TABLE_CONSTRAINT: u8 = number("TableConstraint");
const COMPONENT_CONSTRAINT: u8 = number("ComponentConstraint");
const ELEMENT_CONSTRAINT: u8 = number("ElementConstraint");
const ANY_DEFINED_BY_TYPE: u8 = number("AnyDefinedByType");
const ANY_DEFINED_BY_CHOICE: u8 = number("AnyDefinedByChoice");
const OBJECT_CLASS_ASSIGNMENT: u8 = number("ObjectClassAssignment");
const OBJECT_CLASS_FIELD: u8 = number("ObjectClassField");

fn bad<T>(what: &'static str) -> Result<T> {
    Err(Error::BadNodes(what))
}

/// A node: its kind and its fields, as found in the file.
struct Node<'a> {
    kind: u8,
    fields: &'a [Cbor],
}

impl<'a> Node<'a> {
    fn of(item: &'a Cbor) -> Result<Node<'a>> {
        match item {
            Cbor::Array(items) => match items.first() {
                Some(Cbor::Int(kind)) if (1..=255).contains(kind) => Ok(Node {
                    kind: *kind as u8,
                    fields: &items[1..],
                }),
                _ => bad("Expected a node."),
            },
            _ => bad("Expected a node."),
        }
    }

    /// The field at `index`, or `None` if it is null or left out.
    fn get(&self, index: usize) -> Option<&'a Cbor> {
        match self.fields.get(index) {
            None | Some(Cbor::Null) => None,
            Some(item) => Some(item),
        }
    }

    fn required(&self, index: usize) -> Result<&'a Cbor> {
        self.get(index)
            .ok_or(Error::BadNodes("A field is missing."))
    }

    fn text(&self, index: usize) -> Result<String> {
        match self.required(index)? {
            Cbor::Text(text) => Ok(text.clone()),
            _ => bad("Expected a text string."),
        }
    }

    fn optional_text(&self, index: usize) -> Result<Option<String>> {
        match self.get(index) {
            None => Ok(None),
            Some(Cbor::Text(text)) => Ok(Some(text.clone())),
            Some(_) => bad("Expected a text string."),
        }
    }

    fn boolean(&self, index: usize, default: bool) -> Result<bool> {
        match self.get(index) {
            None => Ok(default),
            Some(Cbor::Bool(value)) => Ok(*value),
            Some(_) => bad("Expected a boolean."),
        }
    }
}

/// The items of a list, which is an array of 0 and the items.
fn list(item: &Cbor) -> Result<&[Cbor]> {
    match item {
        Cbor::Array(items) if matches!(items.first(), Some(Cbor::Int(0))) => Ok(&items[1..]),
        _ => bad("Expected a list."),
    }
}

fn optional_list(item: Option<&Cbor>) -> Result<&[Cbor]> {
    item.map_or(Ok(&[]), list)
}

fn map_list<T>(item: Option<&Cbor>, read: impl Fn(&Cbor) -> Result<T>) -> Result<Vec<T>> {
    optional_list(item)?.iter().map(read).collect()
}

/// A value, as the default of a member or a bound of a range.
#[derive(Debug, Clone, PartialEq)]
pub enum Const {
    Int(i128),
    /// An integer of more than 64 bits, as in [`Cbor::Big`].
    Big {
        negative: bool,
        magnitude: Vec<u8>,
    },
    Real(f64),
    Bool(bool),
    Text(String),
    Identifier(String),
    /// Digits of a hexadecimal string.
    Hex(String),
    /// Digits of a binary string.
    Binary(String),
    Null,
    PlusInfinity,
    MinusInfinity,
    NotANumber,
    Min,
    Max,
    Named(String, Box<Const>),
    Sequence(Vec<(String, Const)>),
    List(Vec<Const>),
    Unsupported,
}

impl Const {
    pub fn as_int(&self) -> Option<i128> {
        match self {
            Const::Int(value) => Some(*value),
            _ => None,
        }
    }
}

fn read_const(item: &Cbor) -> Result<Const> {
    Ok(match item {
        Cbor::Int(value) => Const::Int(*value),
        Cbor::Big {
            negative,
            magnitude,
        } => Const::Big {
            negative: *negative,
            magnitude: magnitude.clone(),
        },
        Cbor::Float(value) => Const::Real(*value),
        Cbor::Bool(value) => Const::Bool(*value),
        Cbor::Text(text) => Const::Text(text.clone()),
        Cbor::Array(items) if matches!(items.first(), Some(Cbor::Int(0))) => {
            Const::List(items[1..].iter().map(read_const).collect::<Result<_>>()?)
        }
        Cbor::Array(_) => {
            let node = Node::of(item)?;

            match node.kind {
                MIN => Const::Min,
                MAX => Const::Max,
                IDENTIFIER => Const::Identifier(node.text(0)?),
                HEX_STRING => Const::Hex(node.text(0)?),
                BINARY_STRING => Const::Binary(node.text(0)?),
                NULL => Const::Null,
                PLUS_INFINITY => Const::PlusInfinity,
                MINUS_INFINITY => Const::MinusInfinity,
                NOT_A_NUMBER => Const::NotANumber,
                NAMED_VALUE => {
                    Const::Named(node.text(0)?, Box::new(read_const(node.required(1)?)?))
                }
                SEQUENCE_VALUE => Const::Sequence(map_list(node.get(0), |member| {
                    let member = Node::of(member)?;

                    Ok((member.text(0)?, read_const(member.required(1)?)?))
                })?),
                UNSUPPORTED_VALUE => Const::Unsupported,
                _ => return bad("Expected a value."),
            }
        }
        _ => return bad("Expected a value."),
    })
}

/// A bound of a range.
#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    Value(Const),
    /// The value is not part of the range.
    Excluded(Const),
}

fn read_bound(item: &Cbor) -> Result<Bound> {
    if let Cbor::Array(items) = item
        && matches!(items.first(), Some(Cbor::Int(kind)) if *kind == i128::from(EXCLUDED))
    {
        return Ok(Bound::Excluded(read_const(Node::of(item)?.required(0)?)?));
    }

    read_const(item).map(Bound::Value)
}

/// An element of a constraint.
#[derive(Debug, Clone, PartialEq)]
pub enum Element {
    Range(Bound, Bound),
    ExtensionMarker,
    Type(Box<Type>),
    Value(Const),
}

fn read_element(item: &Cbor) -> Result<Element> {
    if let Cbor::Array(items) = item
        && let Some(Cbor::Int(kind)) = items.first()
    {
        let node = Node::of(item);

        if *kind == i128::from(RANGE) {
            let node = node?;

            return Ok(Element::Range(
                read_bound(node.required(0)?)?,
                read_bound(node.required(1)?)?,
            ));
        } else if *kind == i128::from(EXTENSION_MARKER) {
            return Ok(Element::ExtensionMarker);
        } else if is_type_kind(*kind) {
            return Ok(Element::Type(Box::new(read_type(item)?)));
        }
    }

    read_const(item).map(Element::Value)
}

fn read_elements(item: Option<&Cbor>) -> Result<Option<Vec<Element>>> {
    item.map(|item| list(item)?.iter().map(read_element).collect())
        .transpose()
}

fn is_type_kind(kind: i128) -> bool {
    [
        BUILTIN_TYPE,
        REFERENCED_TYPE,
        INTEGER_TYPE,
        BIT_STRING_TYPE,
        ENUMERATED_TYPE,
        SEQUENCE_TYPE,
        SET_TYPE,
        CHOICE_TYPE,
        SEQUENCE_OF_TYPE,
        SET_OF_TYPE,
        ANY_DEFINED_BY_TYPE,
    ]
    .iter()
    .any(|candidate| i128::from(*candidate) == kind)
}

/// The class of a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum TagClass {
    Universal,
    Application,
    ContextSpecific,
    Private,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagMode {
    Implicit,
    Explicit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    pub number: u64,
    pub class: TagClass,
    /// `None` if the compiler did not say, which it always does for a
    /// tag that is not a universal one.
    pub mode: Option<TagMode>,
}

fn read_tag(item: &Cbor) -> Result<Tag> {
    let node = Node::of(item)?;
    let Cbor::Int(number) = node.required(0)? else {
        return bad("Expected the number of a tag.");
    };
    let class = match node.optional_text(1)?.as_deref() {
        None => TagClass::ContextSpecific,
        Some("UNIVERSAL") => TagClass::Universal,
        Some("APPLICATION") => TagClass::Application,
        Some("PRIVATE") => TagClass::Private,
        Some(_) => return bad("Unknown class of a tag."),
    };
    let mode = match node.optional_text(2)?.as_deref() {
        None => None,
        Some("IMPLICIT") => Some(TagMode::Implicit),
        Some("EXPLICIT") => Some(TagMode::Explicit),
        Some(_) => return bad("Unknown mode of a tag."),
    };

    Ok(Tag {
        number: u64::try_from(*number).map_err(|_| Error::BadNodes("Bad number of a tag."))?,
        class,
        mode,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedNumber {
    pub name: String,
    pub number: Const,
}

fn read_named_number(item: &Cbor) -> Result<NamedNumber> {
    let node = Node::of(item)?;

    Ok(NamedNumber {
        name: node.text(0)?,
        number: read_const(node.required(1)?)?,
    })
}

/// A type that is built in, but for those that have a body of their
/// own.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builtin {
    Any,
    BitString,
    Bmp,
    Boolean,
    CharacterString,
    Date,
    DateTime,
    EmbeddedPdv,
    External,
    General,
    Generalized,
    Graphic,
    Ia5,
    Integer,
    Iso646,
    Null,
    Numeric,
    ObjectIdentifier,
    OctetString,
    ObjectDescriptor,
    OpenType,
    Printable,
    Real,
    T61,
    Teletex,
    TimeOfDay,
    Duration,
    Utc,
    Utf8,
    Universal,
    Videotex,
    Visible,
    /// Any other name.
    Unsupported,
}

impl Builtin {
    fn from_name(name: &str) -> Builtin {
        match name {
            "ANY" => Builtin::Any,
            "BIT STRING" => Builtin::BitString,
            "BMPString" => Builtin::Bmp,
            "BOOLEAN" => Builtin::Boolean,
            "CHARACTER STRING" => Builtin::CharacterString,
            "DATE" => Builtin::Date,
            "DATE-TIME" => Builtin::DateTime,
            "EMBEDDED PDV" => Builtin::EmbeddedPdv,
            "EXTERNAL" => Builtin::External,
            "GeneralString" => Builtin::General,
            "GeneralizedTime" => Builtin::Generalized,
            "GraphicString" => Builtin::Graphic,
            "IA5String" => Builtin::Ia5,
            "INTEGER" => Builtin::Integer,
            "ISO646String" => Builtin::Iso646,
            "NULL" => Builtin::Null,
            "NumericString" => Builtin::Numeric,
            "OBJECT IDENTIFIER" => Builtin::ObjectIdentifier,
            "OCTET STRING" => Builtin::OctetString,
            "ObjectDescriptor" => Builtin::ObjectDescriptor,
            "OpenType" => Builtin::OpenType,
            "PrintableString" => Builtin::Printable,
            "REAL" => Builtin::Real,
            "T61String" => Builtin::T61,
            "TeletexString" => Builtin::Teletex,
            "TIME-OF-DAY" => Builtin::TimeOfDay,
            "DURATION" => Builtin::Duration,
            "UTCTime" => Builtin::Utc,
            "UTF8String" => Builtin::Utf8,
            "UniversalString" => Builtin::Universal,
            "VideotexString" => Builtin::Videotex,
            "VisibleString" => Builtin::Visible,
            _ => Builtin::Unsupported,
        }
    }
}

impl Builtin {
    /// The name the type is written with in ASN.1. Empty for a name that
    /// is not known.
    pub fn name(self) -> &'static str {
        match self {
            Builtin::Any => "ANY",
            Builtin::BitString => "BIT STRING",
            Builtin::Bmp => "BMPString",
            Builtin::Boolean => "BOOLEAN",
            Builtin::CharacterString => "CHARACTER STRING",
            Builtin::Date => "DATE",
            Builtin::DateTime => "DATE-TIME",
            Builtin::EmbeddedPdv => "EMBEDDED PDV",
            Builtin::External => "EXTERNAL",
            Builtin::General => "GeneralString",
            Builtin::Generalized => "GeneralizedTime",
            Builtin::Graphic => "GraphicString",
            Builtin::Ia5 => "IA5String",
            Builtin::Integer => "INTEGER",
            Builtin::Iso646 => "ISO646String",
            Builtin::Null => "NULL",
            Builtin::Numeric => "NumericString",
            Builtin::ObjectIdentifier => "OBJECT IDENTIFIER",
            Builtin::OctetString => "OCTET STRING",
            Builtin::ObjectDescriptor => "ObjectDescriptor",
            Builtin::OpenType => "OpenType",
            Builtin::Printable => "PrintableString",
            Builtin::Real => "REAL",
            Builtin::T61 => "T61String",
            Builtin::Teletex => "TeletexString",
            Builtin::TimeOfDay => "TIME-OF-DAY",
            Builtin::Duration => "DURATION",
            Builtin::Utc => "UTCTime",
            Builtin::Utf8 => "UTF8String",
            Builtin::Universal => "UniversalString",
            Builtin::Videotex => "VideotexString",
            Builtin::Visible => "VisibleString",
            Builtin::Unsupported => "",
        }
    }
}

impl Type {
    /// The name of the kind of the type as ASN.1 writes it: `INTEGER`,
    /// `SEQUENCE`, or the name of the type it refers to.
    pub fn kind_name(&self) -> &str {
        match &self.body {
            Body::Builtin(builtin) => builtin.name(),
            Body::Referenced(name) => name,
            Body::Integer(_) => "INTEGER",
            Body::BitString(_) => "BIT STRING",
            Body::Enumerated(_) => "ENUMERATED",
            Body::Sequence(_) => "SEQUENCE",
            Body::Set(_) => "SET",
            Body::Choice(_) => "CHOICE",
            Body::SequenceOf { .. } => "SEQUENCE OF",
            Body::SetOf { .. } => "SET OF",
            Body::AnyDefinedBy { .. } => "ANY DEFINED BY",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Member {
    pub name: String,
    pub ty: Type,
    pub optional: bool,
    pub default: Option<Const>,
}

/// A member of a SEQUENCE, a SET or a CHOICE.
#[derive(Debug, Clone, PartialEq)]
pub enum MemberItem {
    Member(Box<Member>),
    ExtensionMarker,
    /// Members added in a later version that are there all or not at
    /// all, as `[[ ... ]]`.
    Group(Vec<MemberItem>),
}

fn read_member_item(item: &Cbor) -> Result<MemberItem> {
    let node = Node::of(item)?;

    match node.kind {
        MEMBER => Ok(MemberItem::Member(Box::new(Member {
            name: node.text(0)?,
            ty: read_type(node.required(1)?)?,
            optional: node.boolean(2, false)?,
            default: node.get(3).map(read_const).transpose()?,
        }))),
        EXTENSION_MARKER => Ok(MemberItem::ExtensionMarker),
        EXTENSION_ADDITION_GROUP => Ok(MemberItem::Group(map_list(
            Some(node.required(0)?),
            read_member_item,
        )?)),
        _ => bad("Expected a member."),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum EnumItem {
    Value(NamedNumber),
    ExtensionMarker,
}

/// The constraint of a component in WITH COMPONENTS, or of the elements
/// in WITH COMPONENT.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Constraint {
    pub size: Option<Vec<Element>>,
    pub restricted_to: Option<Vec<Element>>,
    pub permitted_alphabet: Option<Vec<Element>>,
    pub with_components: Option<Vec<ComponentItem>>,
    pub with_component: Option<Vec<ElementItem>>,
    pub containing: Option<Box<Type>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ComponentConstraint {
    pub name: String,
    /// `PRESENT` or `ABSENT`.
    pub presence: Option<String>,
    pub constraint: Constraint,
}

/// An item of an effective WITH COMPONENTS: an alternative, with the
/// constraints of components of different names, or the extension marker
/// of one that is extensible.
#[derive(Debug, Clone, PartialEq)]
pub enum ComponentItem {
    Alternative(Vec<ComponentConstraint>),
    ExtensionMarker,
}

fn read_constraint(node: &Node<'_>, first: usize) -> Result<Constraint> {
    Ok(Constraint {
        size: read_elements(node.get(first))?,
        restricted_to: read_elements(node.get(first + 1))?,
        permitted_alphabet: read_elements(node.get(first + 2))?,
        with_components: node.get(first + 3).map(read_components).transpose()?,
        with_component: node.get(first + 4).map(read_elements_of).transpose()?,
        containing: node
            .get(first + 5)
            .map(|item| read_type(item).map(Box::new))
            .transpose()?,
    })
}

/// An item of an effective WITH COMPONENT: an alternative, with the
/// constraints of every element, or the extension marker of one that is
/// extensible.
#[derive(Debug, Clone, PartialEq)]
pub enum ElementItem {
    Alternative(Constraint),
    ExtensionMarker,
}

fn read_elements_of(item: &Cbor) -> Result<Vec<ElementItem>> {
    list(item)?
        .iter()
        .map(|item| {
            let node = Node::of(item)?;

            match node.kind {
                ELEMENT_CONSTRAINT => Ok(ElementItem::Alternative(read_constraint(&node, 0)?)),
                EXTENSION_MARKER => Ok(ElementItem::ExtensionMarker),
                _ => bad("Expected an alternative of WITH COMPONENT."),
            }
        })
        .collect()
}

fn read_components(item: &Cbor) -> Result<Vec<ComponentItem>> {
    list(item)?
        .iter()
        .map(|item| {
            if let Ok(alternative) = list(item) {
                return alternative
                    .iter()
                    .map(|item| {
                        let node = Node::of(item)?;

                        if node.kind != COMPONENT_CONSTRAINT {
                            return bad("Expected a component constraint.");
                        }

                        Ok(ComponentConstraint {
                            name: node.text(0)?,
                            presence: node.optional_text(1)?,
                            constraint: read_constraint(&node, 2)?,
                        })
                    })
                    .collect::<Result<_>>()
                    .map(ComponentItem::Alternative);
            }

            match Node::of(item)?.kind {
                EXTENSION_MARKER => Ok(ComponentItem::ExtensionMarker),
                _ => bad("Expected an alternative of WITH COMPONENTS."),
            }
        })
        .collect()
}

/// A table constraint: `{ObjectSet}{@a, @b}`.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub object_set: String,
    pub component_ids: Option<Vec<String>>,
}

fn read_table(item: &Cbor) -> Result<Table> {
    let node = Node::of(item)?;

    if node.kind != TABLE_CONSTRAINT {
        return bad("Expected a table constraint.");
    }

    Ok(Table {
        object_set: node.text(0)?,
        component_ids: node
            .get(1)
            .map(|ids| {
                list(ids)?
                    .iter()
                    .map(|id| match id {
                        Cbor::Text(text) => Ok(text.clone()),
                        _ => bad("Expected a text string."),
                    })
                    .collect()
            })
            .transpose()?,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnyDefinedByChoice {
    pub value: Const,
    pub ty: Type,
}

/// What is special about a type.
#[derive(Debug, Clone, PartialEq)]
pub enum Body {
    Builtin(Builtin),
    /// A reference to a type by name. See [`Spec::find`].
    Referenced(String),
    Integer(Vec<NamedNumber>),
    BitString(Vec<NamedNumber>),
    Enumerated(Vec<EnumItem>),
    Sequence(Vec<MemberItem>),
    Set(Vec<MemberItem>),
    Choice(Vec<MemberItem>),
    SequenceOf {
        element: Box<Type>,
        element_name: Option<String>,
    },
    SetOf {
        element: Box<Type>,
        element_name: Option<String>,
    },
    AnyDefinedBy {
        value: String,
        choices: Vec<AnyDefinedByChoice>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Type {
    pub body: Body,
    pub tag: Option<Tag>,
    pub size: Option<Vec<Element>>,
    pub restricted_to: Option<Vec<Element>>,
    pub permitted_alphabet: Option<Vec<Element>>,
    pub with_components: Option<Vec<ComponentItem>>,
    pub with_component: Option<Vec<ElementItem>>,
    pub table: Option<Table>,
    /// The module the type is found in if it is not the one it is used in.
    pub module_name: Option<String>,
}

fn read_type(item: &Cbor) -> Result<Type> {
    let node = Node::of(item)?;
    // The fields all types have come after their own.
    let (body, first) = match node.kind {
        BUILTIN_TYPE => (Body::Builtin(Builtin::from_name(&node.text(0)?)), 1),
        REFERENCED_TYPE => (Body::Referenced(node.text(0)?), 1),
        INTEGER_TYPE => (Body::Integer(map_list(node.get(0), read_named_number)?), 1),
        BIT_STRING_TYPE => (
            Body::BitString(map_list(node.get(0), read_named_number)?),
            1,
        ),
        ENUMERATED_TYPE => (
            Body::Enumerated(map_list(node.get(0), |item| {
                let inner = Node::of(item)?;

                match inner.kind {
                    NAMED_NUMBER => read_named_number(item).map(EnumItem::Value),
                    EXTENSION_MARKER => Ok(EnumItem::ExtensionMarker),
                    _ => bad("Expected a value of an enumeration."),
                }
            })?),
            1,
        ),
        SEQUENCE_TYPE => (Body::Sequence(map_list(node.get(0), read_member_item)?), 1),
        SET_TYPE => (Body::Set(map_list(node.get(0), read_member_item)?), 1),
        CHOICE_TYPE => (Body::Choice(map_list(node.get(0), read_member_item)?), 1),
        SEQUENCE_OF_TYPE | SET_OF_TYPE => {
            let element = Box::new(read_type(node.required(0)?)?);
            let element_name = node.optional_text(1)?;

            (
                if node.kind == SEQUENCE_OF_TYPE {
                    Body::SequenceOf {
                        element,
                        element_name,
                    }
                } else {
                    Body::SetOf {
                        element,
                        element_name,
                    }
                },
                2,
            )
        }
        ANY_DEFINED_BY_TYPE => (
            Body::AnyDefinedBy {
                value: node.text(0)?,
                choices: map_list(node.get(1), |item| {
                    let choice = Node::of(item)?;

                    if choice.kind != ANY_DEFINED_BY_CHOICE {
                        return bad("Expected a choice of an ANY DEFINED BY.");
                    }

                    Ok(AnyDefinedByChoice {
                        value: read_const(choice.required(0)?)?,
                        ty: read_type(choice.required(1)?)?,
                    })
                })?,
            },
            2,
        ),
        _ => return bad("Expected a type."),
    };

    Ok(Type {
        body,
        tag: node.get(first).map(read_tag).transpose()?,
        size: read_elements(node.get(first + 1))?,
        restricted_to: read_elements(node.get(first + 2))?,
        permitted_alphabet: read_elements(node.get(first + 3))?,
        with_components: node.get(first + 4).map(read_components).transpose()?,
        with_component: node.get(first + 5).map(read_elements_of).transpose()?,
        table: node.get(first + 6).map(read_table).transpose()?,
        // `parameters` and `actual_parameters` are never in a compiled
        // specification.
        module_name: node.optional_text(first + 9)?,
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeAssignment {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectClassField {
    pub name: String,
    pub ty: Type,
}

/// A token of the syntax of an object class.
#[derive(Debug, Clone, PartialEq)]
pub enum SyntaxToken {
    Token(String),
    Optional(Vec<SyntaxToken>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObjectClass {
    pub name: String,
    pub fields: Vec<ObjectClassField>,
    pub syntax: Option<Vec<SyntaxToken>>,
}

fn read_syntax(item: &Cbor) -> Result<SyntaxToken> {
    match item {
        Cbor::Text(text) => Ok(SyntaxToken::Token(text.clone())),
        _ => {
            let node = Node::of(item)?;

            if node.kind != number("OptionalGroup") {
                return bad("Expected a token of a syntax.");
            }

            Ok(SyntaxToken::Optional(map_list(
                Some(node.required(0)?),
                read_syntax,
            )?))
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub name: String,
    pub types: Vec<TypeAssignment>,
    pub object_classes: Vec<ObjectClass>,
}

/// A compiled specification.
#[derive(Debug, Clone, PartialEq)]
pub struct Spec {
    pub modules: Vec<Module>,
}

impl Spec {
    /// Reads the nodes of an `.asnc` file, as the container gives them.
    pub fn from_nodes(nodes: &Cbor) -> Result<Spec> {
        let top = Node::of(nodes)?;

        if top.kind != COMPILED_SPECIFICATION {
            return bad("Expected a compiled specification.");
        }

        Ok(Spec {
            modules: map_list(Some(top.required(0)?), |item| {
                let module = Node::of(item)?;

                if module.kind != COMPILED_MODULE {
                    return bad("Expected a compiled module.");
                }

                Ok(Module {
                    name: module.text(0)?,
                    types: map_list(module.get(1), |item| {
                        let assignment = Node::of(item)?;

                        if assignment.kind != TYPE_ASSIGNMENT {
                            return bad("Expected a type assignment.");
                        }

                        Ok(TypeAssignment {
                            name: assignment.text(0)?,
                            ty: read_type(assignment.required(1)?)?,
                        })
                    })?,
                    object_classes: map_list(module.get(2), |item| {
                        let class = Node::of(item)?;

                        if class.kind != OBJECT_CLASS_ASSIGNMENT {
                            return bad("Expected an object class.");
                        }

                        Ok(ObjectClass {
                            name: class.text(0)?,
                            fields: map_list(class.get(1), |item| {
                                let field = Node::of(item)?;

                                if field.kind != OBJECT_CLASS_FIELD {
                                    return bad("Expected a field of an object class.");
                                }

                                Ok(ObjectClassField {
                                    name: field.text(0)?,
                                    ty: read_type(field.required(1)?)?,
                                })
                            })?,
                            syntax: class
                                .get(2)
                                .map(|tokens| list(tokens)?.iter().map(read_syntax).collect())
                                .transpose()?,
                        })
                    })?,
                })
            })?,
        })
    }

    /// The type with `name` in the module with `module_name`.
    pub fn find(&self, module_name: &str, name: &str) -> Option<&Type> {
        self.modules
            .iter()
            .find(|module| module.name == module_name)?
            .types
            .iter()
            .find(|assignment| assignment.name == name)
            .map(|assignment| &assignment.ty)
    }
}
