// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! The types of a compiled specification, lowered once to what the
//! codecs go through: every type has an id, a reference is an id, the
//! tags that are on the wire are worked out, and the constraints that
//! the codecs look at are numbers.

use alloc::boxed::Box;
use alloc::collections::BTreeMap;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::node::{
    Body, Bound, Builtin, ComponentItem, Const, Constraint, Element, ElementItem, EnumItem, Member,
    MemberItem, Spec, TagClass, TagMode, Type,
};
use crate::value::Value;
use crate::{Error, Result};

pub type TypeId = usize;

fn bad<T>(what: &'static str) -> Result<T> {
    Err(Error::BadNodes(what))
}

/// A value or a range of values of the root of a constraint. A bound that
/// is `None` is MIN or MAX.
pub type Part = (Option<i128>, Option<i128>);

/// A range of sizes or values. A bound that is `None` is MIN or MAX.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Range {
    pub min: Option<i128>,
    pub max: Option<i128>,
    /// There is an extension marker.
    pub extensible: bool,
}

/// The constraints a value is checked against, of a component in WITH
/// COMPONENTS or of the elements in WITH COMPONENT.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Checked {
    pub size: Option<Range>,
    pub size_parts: Option<Vec<Part>>,
    pub range: Option<Range>,
    pub range_parts: Option<Vec<Part>>,
    /// The range if a bound of it does not fit in an `i128`, in place of
    /// `range` and `range_parts`.
    #[cfg(feature = "bigint")]
    pub big_range: Option<crate::bigint::BigRange>,
    pub alphabet: Option<Vec<char>>,
    pub inner: Option<Inner>,
}

/// An inner type constraint (X.680 51.8), as the compiler has worked it
/// out.
#[derive(Debug, Clone, PartialEq)]
pub enum Inner {
    /// WITH COMPONENTS: the values that satisfy any of the alternatives.
    Components(Vec<Vec<ComponentCheck>>),
    /// WITH COMPONENT: the values of which each element satisfies the
    /// constraints of any of the alternatives, the same for all.
    Element(Vec<Checked>),
    /// An extensible one, which is not checked.
    Unchecked,
}

/// The constraint of a component in an alternative of WITH COMPONENTS.
#[derive(Debug, Clone, PartialEq)]
pub struct ComponentCheck {
    pub name: String,
    /// `Some(true)` if it has to be present, `Some(false)` if absent.
    pub presence: Option<bool>,
    pub constraint: Checked,
}

/// A tag as it is on the wire.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct WireTag {
    pub class: TagClass,
    pub number: u64,
    pub constructed: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrKind {
    Utf8,
    Numeric,
    Printable,
    Ia5,
    Visible,
    General,
    Bmp,
    Graphic,
    /// ObjectDescriptor: a GraphicString with the tag 7.
    ObjectDescriptor,
    Universal,
    Teletex,
    Videotex,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberInfo {
    pub name: String,
    pub ty: TypeId,
    pub optional: bool,
    pub default: Option<Value>,
    /// A SIZE given with the member itself.
    pub size: Option<Range>,
    default_const: Option<Const>,
}

impl MemberInfo {
    pub fn has_default(&self) -> bool {
        self.default.is_some()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Addition {
    Member(MemberInfo),
    Group(Vec<MemberInfo>),
}

/// The members of a SEQUENCE, a SET or a CHOICE.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Members {
    pub root: Vec<MemberInfo>,
    /// `Some` if there is an extension marker.
    pub additions: Option<Vec<Addition>>,
    /// The root members in the canonical order of their tags (X.680
    /// 8.6), in which PER and OER write the members of a SET and number
    /// the alternatives of a CHOICE. Empty for a SEQUENCE.
    pub order: Vec<usize>,
    /// The additions of a CHOICE, as `all()` gives them after the root,
    /// in the canonical order of their tags. Empty for the others.
    pub addition_order: Vec<usize>,
    /// How many of the root members are after the additions, after the
    /// second extension marker.
    pub root_after_additions: usize,
}

impl Members {
    /// All members, in the order they are encoded in.
    pub fn all(&self) -> impl Iterator<Item = &MemberInfo> {
        self.root
            .iter()
            .chain(
                self.additions
                    .iter()
                    .flatten()
                    .flat_map(|addition| match addition {
                        Addition::Member(member) => core::slice::from_ref(member).iter(),
                        Addition::Group(group) => group.iter(),
                    }),
            )
    }

    /// All members, in the order they are defined in: the additions
    /// between the root members before and after them.
    pub fn defined(&self) -> impl Iterator<Item = &MemberInfo> {
        let (before, after) = self
            .root
            .split_at(self.root.len() - self.root_after_additions);

        before
            .iter()
            .chain(self.all().skip(self.root.len()))
            .chain(after)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Kind {
    /// The type is another one, with a tag or constraints of its own.
    Ref(TypeId),
    Boolean,
    Integer(Vec<(String, i128)>),
    Real,
    Null,
    /// The named bits, and if there are any.
    BitString(Vec<(String, u64)>),
    OctetString,
    ObjectIdentifier,
    Enumerated {
        root: Vec<(String, i64)>,
        /// `Some` if there is an extension marker.
        additions: Option<Vec<(String, i64)>>,
    },
    Sequence(Members),
    Set(Members),
    Choice(Members),
    /// EXTERNAL, a SEQUENCE with the tag 8.
    External(Members),
    SequenceOf(TypeId),
    SetOf(TypeId),
    Str(StrKind),
    Utc,
    Generalized,
    Date,
    TimeOfDay,
    DateTime,
    Duration,
    Any,
    AnyDefinedBy {
        value: String,
        choices: Vec<(Const, TypeId)>,
    },
    OpenType,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeNode {
    pub kind: Kind,
    /// The tag of the type itself, if it has one.
    pub tag: Option<crate::node::Tag>,
    pub size: Option<Range>,
    pub range: Option<Range>,
    /// The range if a bound of it does not fit in an `i128`, in place of
    /// `range` and `range_parts`, and that of the type or the one it
    /// refers to, in place of `effective_range` and
    /// `effective_range_parts`.
    #[cfg(feature = "bigint")]
    pub big_range: Option<crate::bigint::BigRange>,
    #[cfg(feature = "bigint")]
    pub effective_big_range: Option<crate::bigint::BigRange>,
    /// The permitted alphabet, in order.
    pub alphabet: Option<Vec<char>>,
    /// If the type has a permitted alphabet of its own, also one that is
    /// no permitted alphabet as it is extensible, which is then not taken
    /// from the type it refers to.
    pub alphabet_given: bool,
    /// The values and ranges of the size and of the range of the type,
    /// of which those are the least and the greatest.
    pub size_parts: Option<Vec<Part>>,
    pub range_parts: Option<Vec<Part>>,
    /// What a type with a reference is: the type it ends in, and
    /// all of its tags on the wire, the outermost first.
    pub concrete: TypeId,
    pub tags: Vec<WireTag>,
    /// The size, the range and the alphabet of the type, or those of
    /// the type it is a reference to.
    pub effective_size: Option<Range>,
    pub effective_range: Option<Range>,
    pub effective_alphabet: Option<Vec<char>>,
    pub effective_size_parts: Option<Vec<Part>>,
    pub effective_range_parts: Option<Vec<Part>>,
    /// For a REAL that the ranges of its parts make an IEEE 754 number.
    pub real: Option<RealFormat>,
    /// The inner type constraint of the type itself.
    pub inner: Option<Inner>,
    /// The type of which the inner type constraint is that of this one:
    /// itself, or one it refers to.
    pub effective_inner: Option<TypeId>,
    /// For a type with no tag of its own (a CHOICE): the tags it can
    /// begin with, and if it can begin with anything (an ANY).
    pub begins_with: Vec<(TagClass, u64)>,
    pub begins_with_any: bool,
    /// The tag the type is in the place of in the canonical order of
    /// tags: its first tag or, for a CHOICE with no tag of its own, the
    /// least of those of the alternatives of its root (X.691 21 and
    /// 23.3, X.696 22). `None` for a type that may begin with anything,
    /// as an ANY, which is last.
    pub canonical: Option<(TagClass, u64)>,
    /// For a SEQUENCE OF and a SET OF: what the elements are called in
    /// XER. That is the name given to them, or else the name of their
    /// kind, as `INTEGER`, `SEQUENCE` or the name of a type.
    pub element_name: Option<String>,
}

/// The IEEE 754 formats a REAL can be restricted to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RealFormat {
    Binary32,
    Binary64,
}

/// Which codec the types are for, as it changes what can be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    Ber,
    Der,
    Oer,
    Per,
    Uper,
    /// JSON Encoding Rules, with the feature `jer`.
    Jer,
    /// XML Encoding Rules, with the feature `xer`.
    Xer,
    /// Generic String Encoding Rules, with the feature `gser`.
    Gser,
}

/// The types of a specification.
#[derive(Debug, Clone)]
pub struct Types {
    pub nodes: Vec<TypeNode>,
    /// Module name, and the names of its types with their ids.
    pub modules: Vec<(String, Vec<(String, TypeId)>)>,
    pub numeric_enums: bool,
    pub codec: Codec,
    /// DER that decodes what BER allows.
    pub lenient: bool,
    /// The types of each module by name, and those with a name that is
    /// found in one module only.
    index: BTreeMap<String, BTreeMap<String, TypeId>>,
    unique: BTreeMap<String, Option<TypeId>>,
}

impl Types {
    /// The type with `name` in the module `module`.
    pub fn find(&self, module: &str, name: &str) -> Option<TypeId> {
        self.index.get(module)?.get(name).copied()
    }

    /// The type with `name`, if there is one in all of the modules.
    pub fn find_unique(&self, name: &str) -> Option<TypeId> {
        self.unique.get(name).copied().flatten()
    }

    /// If a type begins with a tag of this class and number. A type that
    /// has none, as ANY, accepts any.
    pub fn accepts(&self, id: TypeId, class: TagClass, number: u64) -> bool {
        let node = &self.nodes[id];

        match node.tags.first() {
            Some(tag) => tag.class == class && tag.number == number,
            None => node.begins_with_any || node.begins_with.contains(&(class, number)),
        }
    }

    /// The first tag a type begins with, if it has one.
    pub fn first_tag(&self, id: TypeId) -> Option<(TagClass, u64)> {
        let node = &self.nodes[id];

        match node.tags.first() {
            Some(tag) => Some((tag.class, tag.number)),
            None => node.begins_with.iter().min().copied(),
        }
    }

    pub fn node(&self, id: TypeId) -> &TypeNode {
        &self.nodes[id]
    }

    /// The kind of a type, past the references.
    pub fn kind(&self, id: TypeId) -> &Kind {
        &self.nodes[self.nodes[id].concrete].kind
    }

    /// The type of the value of a member, `id`, that is an ANY DEFINED BY
    /// with choices: that of the choice of the value of the member it
    /// names, among `siblings`. `None` if it is not one, and `Some(None)`
    /// if no choice has that value.
    pub fn defined_by<'v>(
        &self,
        id: TypeId,
        siblings: impl IntoIterator<Item = &'v (String, Value)>,
    ) -> Option<Option<TypeId>> {
        let Kind::AnyDefinedBy { value, choices } = self.kind(id) else {
            return None;
        };

        if choices.is_empty() {
            return None;
        }

        let key = siblings
            .into_iter()
            .find(|(name, _)| name == value)
            .map(|(_, key)| key);

        Some(key.and_then(|key| {
            choices
                .iter()
                .find(|(candidate, _)| const_matches(candidate, key))
                .map(|(_, ty)| *ty)
        }))
    }
}

/// If the value of the member an ANY DEFINED BY is defined by is that of
/// a choice of its types.
pub fn const_matches(key: &Const, value: &Value) -> bool {
    match (key, value) {
        (Const::Int(a), Value::Integer(b)) => a == b,
        (Const::Text(a) | Const::Identifier(a), Value::Text(b)) => a == b,
        (Const::Bool(a), Value::Boolean(b)) => a == b,
        _ => false,
    }
}

/// Lowers the types of `spec`.
pub fn lower(spec: &Spec, codec: Codec, numeric_enums: bool, lenient: bool) -> Result<Types> {
    let mut lowering = Lowering {
        spec,
        codec,
        nodes: Vec::new(),
        ids: Vec::new(),
        module_index: BTreeMap::new(),
        type_index: Vec::new(),
    };

    // Every type of a module has its id before any is lowered, as
    // types refer to each other.
    for module in &spec.modules {
        let mut ids = Vec::new();

        for _ in &module.types {
            ids.push(lowering.nodes.len());
            lowering.nodes.push(placeholder());
        }

        // The first of a name is the one that is found.
        let mut names = BTreeMap::new();

        for (assignment, id) in module.types.iter().zip(&ids) {
            names.entry(assignment.name.as_str()).or_insert(*id);
        }

        lowering
            .module_index
            .entry(module.name.as_str())
            .or_insert(lowering.ids.len());
        lowering.type_index.push(names);
        lowering.ids.push(ids);
    }

    for (module_index, module) in spec.modules.iter().enumerate() {
        for (type_index, assignment) in module.types.iter().enumerate() {
            let node = lowering.lower_node(&assignment.ty, module_index)?;

            lowering.nodes[lowering.ids[module_index][type_index]] = node;
        }
    }

    let modules: Vec<(String, Vec<(String, TypeId)>)> = spec
        .modules
        .iter()
        .zip(&lowering.ids)
        .map(|(module, ids)| {
            (
                module.name.clone(),
                module
                    .types
                    .iter()
                    .zip(ids)
                    .map(|(assignment, id)| (assignment.name.clone(), *id))
                    .collect(),
            )
        })
        .collect();
    let mut index: BTreeMap<String, BTreeMap<String, TypeId>> = BTreeMap::new();
    let mut unique: BTreeMap<String, Option<TypeId>> = BTreeMap::new();

    for (module, types) in &modules {
        let names = index.entry(module.clone()).or_default();

        for (name, id) in types {
            names.entry(name.clone()).or_insert(*id);

            // A name in two modules, or twice in one, is not unique.
            unique
                .entry(name.clone())
                .and_modify(|found| *found = None)
                .or_insert(Some(*id));
        }
    }

    let mut types = Types {
        nodes: lowering.nodes,
        modules,
        numeric_enums,
        codec,
        lenient,
        index,
        unique,
    };

    finish(&mut types)?;

    Ok(types)
}

fn element_name_of(element: &Type, given: Option<&str>) -> String {
    match given {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => element.kind_name().to_string(),
    }
}

fn placeholder() -> TypeNode {
    TypeNode {
        kind: Kind::Null,
        tag: None,
        size: None,
        range: None,
        #[cfg(feature = "bigint")]
        big_range: None,
        #[cfg(feature = "bigint")]
        effective_big_range: None,
        alphabet: None,
        concrete: 0,
        tags: Vec::new(),
        effective_size: None,
        effective_range: None,
        effective_alphabet: None,
        alphabet_given: false,
        size_parts: None,
        range_parts: None,
        effective_size_parts: None,
        effective_range_parts: None,
        real: None,
        inner: None,
        effective_inner: None,
        begins_with: Vec::new(),
        begins_with_any: false,
        canonical: None,
        element_name: None,
    }
}

struct Lowering<'a> {
    spec: &'a Spec,
    codec: Codec,
    nodes: Vec<TypeNode>,
    /// Ids of the types of each module.
    ids: Vec<Vec<usize>>,
    /// The index of each module by name, and of the types of each module.
    module_index: BTreeMap<&'a str, usize>,
    type_index: Vec<BTreeMap<&'a str, usize>>,
}

/// A bound that does not fit in an `i128`, which `bigint` takes.
const TOO_LARGE: &str =
    "A bound of a constraint does not fit in 128 bits, which needs the feature bigint.";

/// The range of an integer: its own, and its values and ranges, or with
/// the feature `bigint` its `BigRange` if a bound does not fit in an
/// `i128`.
#[derive(Default)]
struct IntegerRange {
    range: Option<Range>,
    parts: Option<Vec<Part>>,
    #[cfg(feature = "bigint")]
    big: Option<crate::bigint::BigRange>,
}

fn integer_range(elements: Option<&[Element]>) -> Result<IntegerRange> {
    let Some(elements) = elements else {
        return Ok(IntegerRange::default());
    };

    match (range_of(elements, true), parts_of(elements, true)) {
        (Ok(range), Ok(parts)) => Ok(IntegerRange {
            range: Some(range),
            parts: Some(parts),
            #[cfg(feature = "bigint")]
            big: None,
        }),
        #[cfg(feature = "bigint")]
        (Err(Error::Unsupported(TOO_LARGE, _)), _) => Ok(IntegerRange {
            big: Some(crate::bigint::range_of(elements)?),
            ..IntegerRange::default()
        }),
        (Err(error), _) | (_, Err(error)) => Err(error),
    }
}

fn bound(bound: &Bound, is_range: bool, is_lower: bool) -> Result<Option<i128>> {
    let (value, excluded) = match bound {
        Bound::Value(value) => (value, false),
        Bound::Excluded(value) => (value, true),
    };

    match value {
        Const::Min | Const::Max => Ok(None),
        Const::Int(number) => {
            // An excluded bound of the range of an integer, or of a size,
            // is the next integer in.
            let _ = is_range;
            let adjust = match (excluded, is_lower) {
                (false, _) => 0,
                (true, true) => 1,
                (true, false) => -1,
            };

            Ok(Some(number + adjust))
        }
        // A bound of more than 64 bits, which has to fit in 128.
        Const::Big {
            negative,
            magnitude,
        } => {
            let Value::Integer(number) =
                Value::from_twos_complement(&twos_complement_of(*negative, magnitude))
            else {
                return Err(Error::unsupported(TOO_LARGE));
            };
            let adjust = match (excluded, is_lower) {
                (false, _) => 0,
                (true, true) => 1,
                (true, false) => -1,
            };

            Ok(Some(number + adjust))
        }
        // A number that is not an integer.
        Const::Real(_) | Const::PlusInfinity | Const::MinusInfinity => Ok(None),
        Const::Identifier(_) => bad("A value in a constraint is not resolved."),
        _ => Ok(None),
    }
}

/// The values and the ranges of the root of a constraint, which are
/// those before an extension marker. A bound that is `None` is MIN or MAX.
fn parts_of(elements: &[Element], is_range: bool) -> Result<Vec<Part>> {
    let mut parts = Vec::new();

    for element in elements {
        parts.push(match element {
            Element::ExtensionMarker => break,
            Element::Range(lower, upper) => (
                bound(lower, is_range, true)?,
                bound(upper, is_range, false)?,
            ),
            Element::Value(value) => {
                let value = bound(&Bound::Value(value.clone()), is_range, true)?;

                (value, value)
            }
            Element::Type(_) => (None, None),
        });
    }

    Ok(parts)
}

/// The least and the greatest values of the root of a constraint, which
/// is the union of its elements, and if there is an extension marker
/// (X.691 10.3.10, X.696 8.2.7).
fn range_of(elements: &[Element], is_range: bool) -> Result<Range> {
    let extensible = elements
        .iter()
        .any(|element| matches!(element, Element::ExtensionMarker));
    let parts = parts_of(elements, is_range)?;
    let min = if parts.iter().any(|(lower, _)| lower.is_none()) {
        None
    } else {
        parts.iter().filter_map(|(lower, _)| *lower).min()
    };
    let max = if parts.iter().any(|(_, upper)| upper.is_none()) {
        None
    } else {
        parts.iter().filter_map(|(_, upper)| *upper).max()
    };

    Ok(Range {
        min,
        max,
        extensible,
    })
}

fn alphabet_of(elements: &[Element]) -> Result<Option<Vec<char>>> {
    let mut characters = Vec::new();

    for element in elements {
        match element {
            Element::Value(Const::Text(text)) => characters.extend(text.chars()),
            Element::Range(Bound::Value(Const::Text(lower)), Bound::Value(Const::Text(upper))) => {
                let (Some(lower), Some(upper)) = (lower.chars().next(), upper.chars().next())
                else {
                    return bad("Bad range of a permitted alphabet.");
                };

                characters.extend((lower as u32..=upper as u32).filter_map(char::from_u32));
            }
            // An extensible permitted alphabet is not PER-visible (X.691
            // 10.3.11), and an extensible constraint is not checked.
            Element::ExtensionMarker => return Ok(None),
            _ => return Ok(None),
        }
    }

    characters.sort_unstable();

    Ok(Some(characters))
}

impl Lowering<'_> {
    /// The module of a name, if there is one of it.
    fn module_named(&self, name: &str) -> Option<usize> {
        self.module_index.get(name).copied()
    }

    fn lookup(&self, name: &str, module: usize, module_name: Option<&String>) -> Result<TypeId> {
        let module = module_name
            .and_then(|module_name| self.module_named(module_name))
            .unwrap_or(module);

        match self.type_index[module].get(name) {
            Some(id) => Ok(*id),
            None => bad("A type is not found."),
        }
    }

    fn lower_members(&mut self, items: &[MemberItem], module: usize) -> Result<Members> {
        let mut members = Members::default();
        let mut in_extension = false;

        for item in items {
            match item {
                MemberItem::ExtensionMarker => {
                    in_extension = !in_extension;

                    if in_extension && members.additions.is_none() {
                        members.additions = Some(Vec::new());
                    }
                }
                MemberItem::Member(member) if in_extension => {
                    let info = self.lower_member(member, module)?;
                    members
                        .additions
                        .as_mut()
                        .unwrap()
                        .push(Addition::Member(info));
                }
                MemberItem::Member(member) => {
                    let info = self.lower_member(member, module)?;
                    members.root.push(info);

                    if members.additions.is_some() {
                        members.root_after_additions += 1;
                    }
                }
                MemberItem::Group(group) => {
                    let mut infos = Vec::new();

                    for inner in group {
                        match inner {
                            MemberItem::Member(member) => {
                                infos.push(self.lower_member(member, module)?)
                            }
                            _ => return bad("Expected a member in a group."),
                        }
                    }

                    match members.additions.as_mut() {
                        Some(additions) => additions.push(Addition::Group(infos)),
                        None => return bad("A group of additions is not after a marker."),
                    }
                }
            }
        }

        Ok(members)
    }

    fn lower_member(&mut self, member: &Member, module: usize) -> Result<MemberInfo> {
        let ty = self.lower_type(&member.ty, module)?;
        let size = match &member.ty.size {
            Some(elements) => Some(range_of(elements, false)?),
            None => None,
        };

        Ok(MemberInfo {
            name: member.name.clone(),
            ty,
            optional: member.optional,
            default: None,
            size,
            default_const: member.default.clone(),
        })
    }

    /// The id of a type that is found where it is used. A reference
    /// with nothing of its own is the type it refers to.
    fn lower_type(&mut self, ty: &Type, module: usize) -> Result<TypeId> {
        let module = self.module_of(ty, module);

        if let Body::Referenced(name) = &ty.body {
            let has_own = ty.tag.is_some()
                || ty.size.is_some()
                || ty.restricted_to.is_some()
                || ty.permitted_alphabet.is_some();

            if !has_own && !name.contains('&') {
                return self.lookup(name, module, ty.module_name.as_ref());
            }
        }

        let node = self.lower_node(ty, module)?;
        let id = self.nodes.len();
        self.nodes.push(node);

        Ok(id)
    }

    fn field_type(
        &self,
        name: &str,
        module: usize,
        module_name: Option<&String>,
    ) -> Result<(Type, usize)> {
        let (class_name, field_name) = name.split_once('.').ok_or(Error::BadNodes("Bad field."))?;
        let start = module_name
            .and_then(|module_name| self.module_named(module_name))
            .unwrap_or(module);

        // The class is in the module the type is found in, or in one
        // the specification has.
        for candidate in core::iter::once(start).chain(0..self.spec.modules.len()) {
            if let Some(class) = self.spec.modules[candidate]
                .object_classes
                .iter()
                .find(|class| class.name == class_name)
                && let Some(field) = class.fields.iter().find(|field| field.name == field_name)
            {
                return Ok((field.ty.clone(), candidate));
            }
        }

        bad("A field of an object class is not found.")
    }

    /// The module a type is found in, if it says, else the one it is used in.
    fn module_of(&self, ty: &Type, module: usize) -> usize {
        ty.module_name
            .as_deref()
            .and_then(|name| self.module_named(name))
            .unwrap_or(module)
    }

    fn lower_node(&mut self, ty: &Type, module: usize) -> Result<TypeNode> {
        let module = self.module_of(ty, module);
        let mut node = placeholder();

        node.tag = ty.tag.clone();
        node.size = ty.size.as_deref().map(|e| range_of(e, false)).transpose()?;
        node.size_parts = ty.size.as_deref().map(|e| parts_of(e, false)).transpose()?;
        let range = integer_range(ty.restricted_to.as_deref())?;

        node.range = range.range;
        node.range_parts = range.parts;
        #[cfg(feature = "bigint")]
        {
            node.big_range = range.big;
        }
        node.alphabet = match &ty.permitted_alphabet {
            Some(elements) => alphabet_of(elements)?,
            None => None,
        };
        node.alphabet_given = ty.permitted_alphabet.is_some();
        if matches!(ty.body, Body::Builtin(Builtin::Real)) {
            node.real = real_format(ty.with_components.as_deref())?;
        }
        node.inner = inner_of(ty.with_components.as_deref(), ty.with_component.as_deref())?;

        node.kind = match &ty.body {
            Body::Builtin(builtin) => self.lower_builtin(*builtin)?,
            Body::Referenced(name) if name.contains('&') => {
                // A field of an object class is the type it is, of which
                // the constraints are taken as those of any type it refers
                // to.
                let (mut field, class_module) =
                    self.field_type(name, module, ty.module_name.as_ref())?;

                field.tag = None;
                // The types in the field are lowered first, and take
                // the ids before its own.
                let lowered = self.lower_node(&field, class_module)?;
                let id = self.nodes.len();
                self.nodes.push(lowered);

                Kind::Ref(id)
            }
            Body::Referenced(name) => {
                Kind::Ref(self.lookup(name, module, ty.module_name.as_ref())?)
            }
            Body::Integer(numbers) => Kind::Integer(
                numbers
                    .iter()
                    .filter_map(|number| number.number.as_int().map(|n| (number.name.clone(), n)))
                    .collect(),
            ),
            Body::BitString(bits) => Kind::BitString(
                bits.iter()
                    .filter_map(|bit| {
                        bit.number
                            .as_int()
                            .and_then(|n| u64::try_from(n).ok())
                            .map(|n| (bit.name.clone(), n))
                    })
                    .collect(),
            ),
            Body::Enumerated(items) => {
                let mut root = Vec::new();
                let mut additions: Option<Vec<(String, i64)>> = None;

                for item in items {
                    match item {
                        EnumItem::Value(number) => {
                            let Some(value) =
                                number.number.as_int().and_then(|n| i64::try_from(n).ok())
                            else {
                                return bad("Bad number of an enumerated value.");
                            };

                            match additions.as_mut() {
                                Some(additions) => additions.push((number.name.clone(), value)),
                                None => root.push((number.name.clone(), value)),
                            }
                        }
                        EnumItem::ExtensionMarker => {
                            if additions.is_none() {
                                additions = Some(Vec::new());
                            }
                        }
                    }
                }

                Kind::Enumerated { root, additions }
            }
            Body::Sequence(items) => Kind::Sequence(self.lower_members(items, module)?),
            Body::Set(items) => Kind::Set(self.lower_members(items, module)?),
            Body::Choice(items) => Kind::Choice(self.lower_members(items, module)?),
            Body::SequenceOf {
                element,
                element_name,
            } => {
                node.element_name = Some(element_name_of(element, element_name.as_deref()));

                Kind::SequenceOf(self.lower_type(element, module)?)
            }
            Body::SetOf {
                element,
                element_name,
            } => {
                node.element_name = Some(element_name_of(element, element_name.as_deref()));

                Kind::SetOf(self.lower_type(element, module)?)
            }
            Body::AnyDefinedBy { value, choices } => {
                let mut lowered = Vec::new();

                for choice in choices {
                    lowered.push((choice.value.clone(), self.lower_type(&choice.ty, module)?));
                }

                Kind::AnyDefinedBy {
                    value: value.clone(),
                    choices: lowered,
                }
            }
        };

        Ok(node)
    }

    /// An open type, as the value of a field of an object class that is a
    /// type: in BER and DER the encoding of the value as it is, as an ANY,
    /// and in PER, UPER and OER that encoding after its length (X.691 11.2,
    /// X.696 30). JER and XER need the type to encode it with (X.697 41,
    /// X.693 8.5), and so do not support it.
    fn open_type(&self) -> Kind {
        match self.codec {
            Codec::Ber | Codec::Der => Kind::Any,
            _ => Kind::OpenType,
        }
    }

    /// The members of EXTERNAL, as X.690 8.18.1 and X.691 29.1 say them,
    /// in an environment of EXPLICIT TAGS.
    fn external(&mut self) -> Members {
        let open_type = self.open_type();
        let mut add = |kind: Kind, tag: Option<(u64, TagMode)>| {
            let mut node = placeholder();

            node.kind = kind;
            node.tag = tag.map(|(number, mode)| crate::node::Tag {
                number,
                class: TagClass::ContextSpecific,
                mode: Some(mode),
            });
            self.nodes.push(node);

            self.nodes.len() - 1
        };
        let direct = add(Kind::ObjectIdentifier, None);
        let indirect = add(Kind::Integer(Vec::new()), None);
        let descriptor = add(Kind::Str(StrKind::ObjectDescriptor), None);
        // [0] ABSTRACT-SYNTAX.&Type, an open type.
        let single = add(open_type, Some((0, TagMode::Explicit)));
        let octets = add(Kind::OctetString, Some((1, TagMode::Implicit)));
        let arbitrary = add(Kind::BitString(Vec::new()), Some((2, TagMode::Implicit)));
        let member = |name: &str, ty, optional| MemberInfo {
            name: String::from(name),
            ty,
            optional,
            default: None,
            size: None,
            default_const: None,
        };
        let encoding = add(
            Kind::Choice(Members {
                root: alloc::vec![
                    member("single-ASN1-type", single, false),
                    member("octet-aligned", octets, false),
                    member("arbitrary", arbitrary, false),
                ],
                additions: None,
                order: Vec::new(),
                addition_order: Vec::new(),
                root_after_additions: 0,
            }),
            None,
        );

        Members {
            root: alloc::vec![
                member("direct-reference", direct, true),
                member("indirect-reference", indirect, true),
                member("data-value-descriptor", descriptor, true),
                member("encoding", encoding, false),
            ],
            additions: None,
            order: Vec::new(),
            addition_order: Vec::new(),
            root_after_additions: 0,
        }
    }

    fn lower_builtin(&mut self, builtin: Builtin) -> Result<Kind> {
        Ok(match builtin {
            Builtin::Any => Kind::Any,
            Builtin::BitString => Kind::BitString(Vec::new()),
            Builtin::Bmp => Kind::Str(StrKind::Bmp),
            Builtin::Boolean => Kind::Boolean,
            Builtin::Date => Kind::Date,
            Builtin::DateTime => Kind::DateTime,
            Builtin::General => Kind::Str(StrKind::General),
            Builtin::Generalized => Kind::Generalized,
            Builtin::Graphic => Kind::Str(StrKind::Graphic),
            Builtin::ObjectDescriptor => Kind::Str(StrKind::ObjectDescriptor),
            Builtin::Ia5 | Builtin::Iso646 => Kind::Str(StrKind::Ia5),
            Builtin::Integer => Kind::Integer(Vec::new()),
            Builtin::Null => Kind::Null,
            Builtin::Numeric => Kind::Str(StrKind::Numeric),
            Builtin::ObjectIdentifier => Kind::ObjectIdentifier,
            Builtin::OctetString => Kind::OctetString,
            Builtin::OpenType => self.open_type(),
            Builtin::Printable => Kind::Str(StrKind::Printable),
            Builtin::Real => Kind::Real,
            Builtin::T61 | Builtin::Teletex => Kind::Str(StrKind::Teletex),
            Builtin::Videotex => Kind::Str(StrKind::Videotex),
            Builtin::TimeOfDay => Kind::TimeOfDay,
            Builtin::Duration => Kind::Duration,
            Builtin::Utc => Kind::Utc,
            Builtin::Utf8 => Kind::Str(StrKind::Utf8),
            Builtin::Universal => Kind::Str(StrKind::Universal),
            Builtin::Visible => Kind::Str(StrKind::Visible),
            Builtin::External => Kind::External(self.external()),
            Builtin::CharacterString | Builtin::EmbeddedPdv | Builtin::Unsupported => {
                return bad("A type is not supported.");
            }
        })
    }
}

/// The bounds of the range a part of a REAL is restricted to in an
/// alternative, if it has both and is not extensible.
fn component_range(
    alternative: &[crate::node::ComponentConstraint],
    name: &str,
) -> Result<Option<(i128, i128)>> {
    let Some(component) = alternative.iter().find(|component| component.name == name) else {
        return Ok(None);
    };
    let Some(elements) = &component.constraint.restricted_to else {
        return Ok(None);
    };
    let range = range_of(elements, true)?;

    Ok(match (range.extensible, range.min, range.max) {
        (false, Some(min), Some(max)) => Some((min, max)),
        _ => None,
    })
}

/// If the WITH COMPONENTS of a REAL make it an IEEE 754 number. Each part
/// is restricted to the least range that has those of all the
/// alternatives (X.696 12.1), and an extensible constraint is not
/// OER-visible (X.696 8.2.2).
fn real_format(components: Option<&[ComponentItem]>) -> Result<Option<RealFormat>> {
    let Some(components) = components else {
        return Ok(None);
    };
    let mut hulls: [Option<(i128, i128)>; 3] = [None; 3];

    for item in components {
        let ComponentItem::Alternative(alternative) = item else {
            return Ok(None);
        };

        for (hull, name) in hulls.iter_mut().zip(["mantissa", "base", "exponent"]) {
            let Some((lower, upper)) = component_range(alternative, name)? else {
                return Ok(None);
            };

            *hull = Some(match *hull {
                Some((min, max)) => (min.min(lower), max.max(upper)),
                None => (lower, upper),
            });
        }
    }

    let [Some(mantissa), Some(base), Some(exponent)] = hulls else {
        return Ok(None);
    };

    if base != (2, 2) {
        return Ok(None);
    }

    let fits = |(lower, upper): (i128, i128), low: i128, high: i128| {
        low <= lower && lower <= upper && upper <= high
    };

    Ok(
        if fits(mantissa, -16777215, 16777215) && fits(exponent, -149, 104) {
            Some(RealFormat::Binary32)
        } else if fits(mantissa, -9007199254740991, 9007199254740991) && fits(exponent, -1074, 971)
        {
            Some(RealFormat::Binary64)
        } else {
            None
        },
    )
}

/// The constraints of a component or of the elements, to check values
/// against.
fn checked_of(constraint: &Constraint) -> Result<Checked> {
    let range = integer_range(constraint.restricted_to.as_deref())?;

    Ok(Checked {
        size: constraint
            .size
            .as_deref()
            .map(|e| range_of(e, false))
            .transpose()?,
        size_parts: constraint
            .size
            .as_deref()
            .map(|e| parts_of(e, false))
            .transpose()?,
        range: range.range,
        range_parts: range.parts,
        #[cfg(feature = "bigint")]
        big_range: range.big,
        alphabet: match &constraint.permitted_alphabet {
            Some(elements) => alphabet_of(elements)?,
            None => None,
        },
        inner: inner_of(
            constraint.with_components.as_deref(),
            constraint.with_component.as_deref(),
        )?,
    })
}

/// The inner type constraint of a type, a component or the elements.
fn inner_of(
    components: Option<&[ComponentItem]>,
    element: Option<&[ElementItem]>,
) -> Result<Option<Inner>> {
    if let Some(items) = element {
        let mut alternatives = Vec::new();

        for item in items {
            match item {
                ElementItem::Alternative(constraint) => alternatives.push(checked_of(constraint)?),
                ElementItem::ExtensionMarker => return Ok(Some(Inner::Unchecked)),
            }
        }

        return Ok(Some(Inner::Element(alternatives)));
    }

    let Some(items) = components else {
        return Ok(None);
    };
    let mut alternatives = Vec::new();

    for item in items {
        match item {
            ComponentItem::Alternative(alternative) => alternatives.push(
                alternative
                    .iter()
                    .map(|component| {
                        Ok(ComponentCheck {
                            name: component.name.clone(),
                            presence: match component.presence.as_deref() {
                                Some("PRESENT") => Some(true),
                                Some("ABSENT") => Some(false),
                                _ => None,
                            },
                            constraint: checked_of(&component.constraint)?,
                        })
                    })
                    .collect::<Result<Vec<_>>>()?,
            ),
            ComponentItem::ExtensionMarker => return Ok(Some(Inner::Unchecked)),
        }
    }

    Ok(Some(Inner::Components(alternatives)))
}

/// The universal tag number of a kind, if it has one.
fn universal_tag(kind: &Kind) -> Option<(u64, bool)> {
    Some(match kind {
        Kind::Boolean => (1, false),
        Kind::Integer(_) => (2, false),
        Kind::BitString(_) => (3, false),
        Kind::OctetString => (4, false),
        Kind::Null => (5, false),
        Kind::ObjectIdentifier => (6, false),
        Kind::Real => (9, false),
        Kind::Enumerated { .. } => (10, false),
        Kind::Sequence(_) | Kind::SequenceOf(_) => (16, true),
        Kind::External(_) => (8, true),
        Kind::Set(_) | Kind::SetOf(_) => (17, true),
        Kind::Str(string) => (
            match string {
                StrKind::ObjectDescriptor => 7,
                StrKind::Utf8 => 12,
                StrKind::Numeric => 18,
                StrKind::Printable => 19,
                StrKind::Teletex => 20,
                StrKind::Videotex => 21,
                StrKind::Ia5 => 22,
                StrKind::Graphic => 25,
                StrKind::Visible => 26,
                StrKind::General => 27,
                StrKind::Universal => 28,
                StrKind::Bmp => 30,
            },
            false,
        ),
        Kind::Utc => (23, false),
        Kind::Generalized => (24, false),
        Kind::Date => (31, false),
        Kind::TimeOfDay => (32, false),
        Kind::DateTime => (33, false),
        Kind::Duration => (34, false),
        // A CHOICE and the types of any kind have no tag of their own.
        Kind::Choice(_) | Kind::Any | Kind::AnyDefinedBy { .. } | Kind::OpenType | Kind::Ref(_) => {
            return None;
        }
    })
}

/// Works out what is left to do with the lowered nodes: the tags on
/// the wire, the constraints of references and the defaults.
fn finish(types: &mut Types) -> Result<()> {
    let count = types.nodes.len();

    settle_all(types)?;
    compute_beginnings(types)?;
    compute_canonical(types);

    // Defaults go by the kind of their type.
    for id in 0..count {
        let mut kind = core::mem::replace(&mut types.nodes[id].kind, Kind::Null);

        if let Kind::Sequence(members) | Kind::Set(members) | Kind::External(members) = &mut kind {
            fill_defaults(types, members)?;
        }

        types.nodes[id].kind = kind;
    }

    // The canonical order of the members of a SET and the alternatives
    // of a CHOICE.
    for id in 0..count {
        let (order, addition_order) = match &types.nodes[id].kind {
            Kind::Set(members) => (canonical_order(types, &members.root), Vec::new()),
            Kind::Choice(members) => {
                let additions: Vec<MemberInfo> =
                    members.all().skip(members.root.len()).cloned().collect();

                (
                    canonical_order(types, &members.root),
                    canonical_order(types, &additions),
                )
            }
            _ => continue,
        };

        if let Kind::Set(members) | Kind::Choice(members) = &mut types.nodes[id].kind {
            members.order = order;
            members.addition_order = addition_order;
        }
    }

    Ok(())
}

/// The indexes of `members` in the canonical order of their tags. A
/// member with no tag, as ANY, is after those with one. The order of
/// members with the same tag, which X.680 forbids, is kept.
fn canonical_order(types: &Types, members: &[MemberInfo]) -> Vec<usize> {
    let mut order: Vec<usize> = (0..members.len()).collect();

    order.sort_by_key(|index| {
        types.nodes[members[*index].ty]
            .canonical
            .map_or((true, TagClass::Universal, 0), |(class, number)| {
                (false, class, number)
            })
    });

    order
}

/// Works out the canonical tag of every type. That of an untagged
/// CHOICE depends on those of its root alternatives, which are worked
/// out first, with a stack of its own and not by recursion, so that a
/// specification can not use up the stack.
fn compute_canonical(types: &mut Types) {
    const NEW: u8 = 0;
    const ON_THE_WAY: u8 = 1;
    const DONE: u8 = 2;

    let count = types.nodes.len();
    let mut state = alloc::vec![NEW; count];

    for start in 0..count {
        if state[start] == DONE {
            continue;
        }

        // (type, if its alternatives are pushed already)
        let mut stack = alloc::vec![(start, false)];

        while let Some((id, expanded)) = stack.pop() {
            if state[id] == DONE {
                continue;
            }

            let node = &types.nodes[id];

            if let Some(tag) = node.tags.first() {
                types.nodes[id].canonical = Some((tag.class, tag.number));
                state[id] = DONE;

                continue;
            }

            let Kind::Choice(members) = &types.nodes[node.concrete].kind else {
                // An ANY or an open type: in no place.
                state[id] = DONE;

                continue;
            };

            if !expanded {
                if state[id] == ON_THE_WAY {
                    // A CHOICE in itself, untagged, which X.680 forbids.
                    state[id] = DONE;

                    continue;
                }

                state[id] = ON_THE_WAY;
                stack.push((id, true));

                for member in &members.root {
                    if state[member.ty] == NEW {
                        stack.push((member.ty, false));
                    }
                }

                continue;
            }

            let least = members
                .root
                .iter()
                .filter_map(|member| types.nodes[member.ty].canonical)
                .min();

            types.nodes[id].canonical = least;
            state[id] = DONE;
        }
    }
}

/// Works out each type that is a reference. It follows a reference to
/// the end of the chain, and then works back, so that a chain of a
/// hundred thousand references is no deeper than one.
fn settle_all(types: &mut Types) -> Result<()> {
    // Not seen, on the way down, and done.
    const NEW: u8 = 0;
    const ON_THE_WAY: u8 = 1;
    const DONE: u8 = 2;

    let mut state = alloc::vec![NEW; types.nodes.len()];

    for start in 0..types.nodes.len() {
        let mut path = Vec::new();
        let mut current = start;

        loop {
            match state[current] {
                DONE => break,
                ON_THE_WAY => return bad("A type is defined by itself."),
                _ => {}
            }

            state[current] = ON_THE_WAY;
            path.push(current);

            match types.nodes[current].kind {
                Kind::Ref(target) => current = target,
                _ => break,
            }
        }

        for id in path.into_iter().rev() {
            settle(types, id);
            state[id] = DONE;
        }
    }

    Ok(())
}

/// Works out one type, of which the type it refers to is done.
fn settle(types: &mut Types, id: TypeId) {
    #[cfg(feature = "bigint")]
    let inherited_big = match types.nodes[id].kind {
        Kind::Ref(target) => types.nodes[target].effective_big_range.clone(),
        _ => None,
    };
    let (concrete, mut tags, inherited, inherited_inner) = match types.nodes[id].kind {
        Kind::Ref(target) => {
            let target = &types.nodes[target];

            (
                target.concrete,
                target.tags.clone(),
                (
                    target.effective_size,
                    target.effective_range,
                    target.effective_alphabet.clone(),
                    target.effective_size_parts.clone(),
                    target.effective_range_parts.clone(),
                ),
                target.effective_inner,
            )
        }
        ref kind => {
            let tags = universal_tag(kind)
                .map(|(number, constructed)| {
                    alloc::vec![WireTag {
                        class: TagClass::Universal,
                        number,
                        constructed
                    }]
                })
                .unwrap_or_default();

            (id, tags, (None, None, None, None, None), None)
        }
    };

    if let Some(tag) = types.nodes[id].tag.clone() {
        let wire = |constructed| WireTag {
            class: tag.class,
            number: tag.number,
            constructed,
        };

        if tag.mode == Some(TagMode::Explicit) {
            tags.insert(0, wire(true));
        } else if let Some(outer) = tags.first_mut() {
            // Implicit: it takes the place of the outermost tag.
            *outer = wire(outer.constructed);
        }
    }

    let node = &mut types.nodes[id];

    node.concrete = concrete;
    node.tags = tags;
    // Its own constraints take the place of those of the type it refers
    // to, as the compiler has worked them out of both.
    // A range of its own is either of `i128` or big, and takes the place
    // of both of the type it refers to.
    #[cfg(feature = "bigint")]
    let own_big = node.big_range.is_some();
    #[cfg(not(feature = "bigint"))]
    let own_big = false;

    node.effective_size = node.size.or(inherited.0);
    node.effective_range = if own_big {
        None
    } else {
        node.range.or(inherited.1)
    };
    node.effective_alphabet = if node.alphabet_given {
        node.alphabet.clone()
    } else {
        inherited.2
    };
    node.effective_size_parts = node.size_parts.clone().or(inherited.3);
    node.effective_range_parts = if own_big {
        None
    } else {
        node.range_parts.clone().or(inherited.4)
    };
    #[cfg(feature = "bigint")]
    {
        node.effective_big_range = match node.range {
            Some(_) => None,
            None => node.big_range.clone().or(inherited_big),
        };
    }
    node.effective_inner = if node.inner.is_some() {
        Some(id)
    } else {
        inherited_inner
    };
}

/// What a type with no tag of its own can begin with: a CHOICE, the tags
/// of all that is in it. Every one is looked at once, and the work there
/// is for all of them is limited.
fn compute_beginnings(types: &mut Types) -> Result<()> {
    const STEPS: usize = 20_000_000;

    let count = types.nodes.len();
    let mut steps = 0usize;
    // Which choice a type was last looked at for, plus one.
    let mut seen = alloc::vec![0usize; count];

    for start in 0..count {
        if !types.nodes[start].tags.is_empty() {
            continue;
        }

        let concrete = types.nodes[start].concrete;

        if !matches!(types.nodes[concrete].kind, Kind::Choice(_)) {
            // An ANY, or something else that has no tag: anything.
            types.nodes[start].begins_with_any = true;

            continue;
        }

        let mut begins = Vec::new();
        let mut any = false;
        let mut stack = alloc::vec![start];

        seen[start] = start + 1;

        while let Some(id) = stack.pop() {
            steps += 1;

            if steps > STEPS {
                return bad("The specification is too complex.");
            }

            let node = &types.nodes[id];

            if let Some(tag) = node.tags.first() {
                begins.push((tag.class, tag.number));

                continue;
            }

            match &types.nodes[node.concrete].kind {
                Kind::Choice(members) => {
                    for member in members.all() {
                        if seen[member.ty] != start + 1 {
                            seen[member.ty] = start + 1;
                            stack.push(member.ty);
                        }
                    }
                }
                _ => any = true,
            }
        }

        begins.sort_unstable();
        begins.dedup();

        types.nodes[start].begins_with = begins;
        types.nodes[start].begins_with_any = any;
    }

    Ok(())
}

fn fill_defaults(types: &Types, members: &mut Members) -> Result<()> {
    let fill = |member: &mut MemberInfo| -> Result<()> {
        if let Some(constant) = member.default_const.take() {
            member.default = default_value(types, member.ty, &constant)?;
        }

        Ok(())
    };

    for member in &mut members.root {
        fill(member)?;
    }

    for addition in members.additions.iter_mut().flatten() {
        match addition {
            Addition::Member(member) => fill(member)?,
            Addition::Group(group) => {
                for member in group {
                    fill(member)?;
                }
            }
        }
    }

    Ok(())
}

fn hex_digits(digits: &str) -> Option<Vec<u8>> {
    let mut padded = String::from(digits);

    if padded.len() % 2 == 1 {
        padded.push('0');
    }

    (0..padded.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&padded[i..i + 2], 16).ok())
        .collect()
}

fn bits_of(bits: &str) -> Option<Value> {
    let mut bytes = alloc::vec![0u8; bits.len().div_ceil(8)];

    for (index, bit) in bits.bytes().enumerate() {
        match bit {
            b'1' => bytes[index / 8] |= 0x80 >> (index % 8),
            b'0' => {}
            _ => return None,
        }
    }

    Some(Value::Bits(bytes, bits.len()))
}

/// The two's complement of an integer given as its sign and the bytes of
/// its magnitude, most significant first.
pub(crate) fn twos_complement_of(negative: bool, magnitude: &[u8]) -> Vec<u8> {
    let mut bytes = alloc::vec![0u8];

    bytes.extend_from_slice(magnitude);

    if negative {
        for byte in bytes.iter_mut() {
            *byte = !*byte;
        }

        for byte in bytes.iter_mut().rev() {
            let (sum, carry) = byte.overflowing_add(1);

            *byte = sum;

            if !carry {
                break;
            }
        }
    }

    bytes
}

/// What a `DEFAULT` is as a value of a type.
fn default_value(types: &Types, ty: TypeId, constant: &Const) -> Result<Option<Value>> {
    let kind = types.kind(ty);

    Ok(match (kind, constant) {
        (_, Const::Null) => None,
        (Kind::Boolean, Const::Bool(value)) => Some(Value::Boolean(*value)),
        (Kind::Real, Const::Real(value)) => Some(Value::Real(*value)),
        (Kind::Real, Const::Int(value)) => Some(Value::Real(*value as f64)),
        (Kind::Real, Const::PlusInfinity) => Some(Value::Real(f64::INFINITY)),
        (Kind::Real, Const::MinusInfinity) => Some(Value::Real(f64::NEG_INFINITY)),
        (_, Const::Int(value)) => Some(Value::Integer(*value)),
        (
            _,
            Const::Big {
                negative,
                magnitude,
            },
        ) => Some(Value::from_twos_complement(&twos_complement_of(
            *negative, magnitude,
        ))),
        (Kind::Enumerated { root, additions }, Const::Identifier(name) | Const::Text(name)) => {
            if types.numeric_enums {
                root.iter()
                    .chain(additions.iter().flatten())
                    .find(|(candidate, _)| candidate == name)
                    .map(|(_, number)| Value::Integer(i128::from(*number)))
            } else {
                Some(Value::Text(name.clone()))
            }
        }
        (Kind::OctetString, Const::Hex(digits)) => hex_digits(digits).map(Value::Bytes),
        (Kind::BitString(_), Const::Binary(bits)) => bits_of(bits),
        (Kind::BitString(_), Const::Hex(digits)) => {
            hex_digits(digits).map(|bytes| Value::Bits(bytes, digits.len() * 4))
        }
        (_, Const::Bool(value)) => Some(Value::Boolean(*value)),
        (_, Const::Real(value)) => Some(Value::Real(*value)),
        (_, Const::Text(text) | Const::Identifier(text)) => Some(Value::Text(text.clone())),
        // The value of an alternative, a member or an item is one of its
        // own type.
        (Kind::Choice(members), Const::Named(name, value)) => {
            match members.all().find(|member| &member.name == name) {
                Some(member) => default_value(types, member.ty, value)?
                    .map(|value| Value::Choice(name.clone(), Box::new(value))),
                None => None,
            }
        }
        (
            Kind::Sequence(members) | Kind::Set(members) | Kind::External(members),
            Const::Sequence(values),
        ) => {
            let mut converted = Vec::new();

            for (name, value) in values {
                let Some(member) = members.all().find(|member| &member.name == name) else {
                    return Ok(None);
                };

                match default_value(types, member.ty, value)? {
                    Some(value) => converted.push((name.clone(), value)),
                    None => return Ok(None),
                }
            }

            Some(Value::Map(converted))
        }
        (Kind::SequenceOf(element) | Kind::SetOf(element), Const::List(items)) => {
            let mut converted = Vec::new();

            for item in items {
                match default_value(types, *element, item)? {
                    Some(value) => converted.push(value),
                    None => return Ok(None),
                }
            }

            Some(Value::List(converted))
        }
        _ => None,
    })
}

/// A bit string with the bits past the length cleared, and, with named
/// bits, the trailing zero bits dropped.
pub fn clean_bits(bytes: &[u8], bits: usize, named: bool) -> (Vec<u8>, usize) {
    let whole = bits / 8;
    let rest = bits % 8;
    let mut data: Vec<u8> = bytes
        .iter()
        .take(whole + usize::from(rest != 0))
        .copied()
        .collect();

    if rest != 0
        && let Some(last) = data.get_mut(whole)
    {
        *last &= !(0xffu8 >> rest);
    }

    if !named {
        return (data, bits);
    }

    while data.last() == Some(&0) {
        data.pop();
    }

    match data.last() {
        None => (data, 0),
        Some(last) => {
            let count = 8 * data.len() - last.trailing_zeros() as usize;

            (data, count)
        }
    }
}

/// If a value is the default of a member.
pub fn is_default(types: &Types, member: &MemberInfo, value: &Value) -> bool {
    let Some(default) = &member.default else {
        return false;
    };

    match (types.kind(member.ty), value, default) {
        (Kind::Null, _, _) => false,
        (Kind::Integer(named), Value::Text(name), _) => named
            .iter()
            .find(|(candidate, _)| candidate == name)
            .is_some_and(|(_, number)| *default == Value::Integer(*number)),
        // An integer of any size, as few bytes as it can be.
        (Kind::Integer(_), Value::Integer(_) | Value::BigInteger(_), _) => value
            .to_twos_complement()
            .is_some_and(|bytes| Value::from_twos_complement(&bytes) == *default),
        (Kind::BitString(named), Value::Bits(a, a_bits), Value::Bits(b, b_bits)) => {
            a.len() * 8 >= *a_bits
                && b.len() * 8 >= *b_bits
                && clean_bits(a, *a_bits, !named.is_empty())
                    == clean_bits(b, *b_bits, !named.is_empty())
        }
        (Kind::Real, Value::Real(a), Value::Real(b)) => a == b,
        (Kind::Real, Value::Integer(a), Value::Real(b)) => *a as f64 == *b,
        (_, value, default) => value == default,
    }
}
