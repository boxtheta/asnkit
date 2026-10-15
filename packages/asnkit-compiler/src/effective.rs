// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The effective constraints of the types: what the constraints as they
//! are written come to, for the encodings and for checking values.
//!
//! A type has four of them, each a set of values that is a root and is
//! extensible or not: the range of an INTEGER, the size of a string, a
//! BIT STRING, an OCTET STRING, a SEQUENCE OF or a SET OF, the
//! permitted alphabet of a string, and the inner type constraint of a
//! SEQUENCE, a SET, a CHOICE, a REAL, a SEQUENCE OF or a SET OF. Each is
//! worked out as X.691 10.3 and B.2.3 and X.696 8.2 say, of the
//! constraints of X.680 49 to 51:
//!
//! - A union or an intersection is that of the sets of each dimension,
//!   and is extensible if one of them is (X.680 50.2, 50.4, X.691
//!   B.2.3.8).
//! - What follows `EXCEPT` is left out (X.691 10.3.21).
//! - A constraint that is not visible, as a single value of a string, a
//!   pattern or a table constraint, is all values in each dimension.
//! - `...` makes every dimension of its constraint extensible (X.691
//!   B.2.3.8), and a contained subtype gives the root of its type and not
//!   that it is extensible (X.680 51.3.3).
//! - A constraint applied to a type that has one is applied to its root,
//!   and that the result is extensible depends on it alone (X.680 50.11).
//!   A general constraint, as a table constraint, changes neither.
//! - A permitted alphabet that is extensible is not visible, and the
//!   constraint is as if it had none (X.691 10.3.11, 10.3.20). So the
//!   effective permitted alphabet is never extensible.
//! - An inner type constraint is not PER-visible, but is checked, and
//!   is kept as a union of alternatives, each of which constrains the
//!   presence and the values of components by name (X.680 51.8). An
//!   intersection is the alternatives of each made into one, and a
//!   `WITH COMPONENT` constrains each element with the union or the
//!   intersection of what each part does. What is not known, as a
//!   component that is not found, is all values, so that no value is
//!   rejected that should not be.
//!
//! What is written in a compiled specification is the root of each, as
//! values and ranges, followed by an extension marker if it is
//! extensible.

use std::collections::HashMap;

use num_bigint::BigInt;
use serde_json::Number;

use super::{CompileResult, Compiler, is_object_class_type_name};
use crate::ast::*;

/// A bound of a range of integers: no bound below, an integer of any
/// size, or no bound above, in that order.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum Bound {
    Min,
    Int(BigInt),
    Max,
}

impl Bound {
    fn int(value: i64) -> Bound {
        Bound::Int(BigInt::from(value))
    }

    /// The bound after this one, which is itself if it is no bound.
    fn next(&self) -> Bound {
        match self {
            Bound::Int(value) => Bound::Int(value + 1),
            other => other.clone(),
        }
    }

    /// The bound before this one, which is itself if it is no bound.
    fn previous(&self) -> Bound {
        match self {
            Bound::Int(value) => Bound::Int(value - 1),
            other => other.clone(),
        }
    }
}

/// A set of integers, as ranges in ascending order that neither overlap
/// nor touch.
#[derive(Debug, Clone, PartialEq)]
struct Ints(Vec<(Bound, Bound)>);

impl Ints {
    fn range(lower: Bound, upper: Bound) -> Ints {
        if lower > upper {
            Ints(Vec::new())
        } else {
            Ints(vec![(lower, upper)])
        }
    }

    fn normalized(mut ranges: Vec<(Bound, Bound)>) -> Ints {
        ranges.retain(|(lower, upper)| lower <= upper);
        ranges.sort();

        let mut merged: Vec<(Bound, Bound)> = Vec::new();

        for (lower, upper) in ranges {
            match merged.last_mut() {
                Some(last) if lower <= last.1.next() => {
                    if upper > last.1 {
                        last.1 = upper;
                    }
                }
                _ => merged.push((lower, upper)),
            }
        }

        Ints(merged)
    }

    fn union(&self, other: &Ints) -> Ints {
        Ints::normalized(self.0.iter().chain(&other.0).cloned().collect())
    }

    fn intersection(&self, other: &Ints) -> Ints {
        let mut ranges = Vec::new();

        for (a_lower, a_upper) in &self.0 {
            for (b_lower, b_upper) in &other.0 {
                ranges.push((a_lower.max(b_lower).clone(), a_upper.min(b_upper).clone()));
            }
        }

        Ints::normalized(ranges)
    }
}

/// A set of characters, by their code points, as `Ints` is.
#[derive(Debug, Clone, PartialEq)]
struct Chars(Vec<(u32, u32)>);

impl Chars {
    fn normalized(mut ranges: Vec<(u32, u32)>) -> Chars {
        ranges.retain(|(lower, upper)| lower <= upper);
        ranges.sort();

        let mut merged: Vec<(u32, u32)> = Vec::new();

        for (lower, upper) in ranges {
            match merged.last_mut() {
                Some(last) if lower <= last.1.saturating_add(1) => last.1 = last.1.max(upper),
                _ => merged.push((lower, upper)),
            }
        }

        Chars(merged)
    }

    fn union(&self, other: &Chars) -> Chars {
        Chars::normalized(self.0.iter().chain(&other.0).copied().collect())
    }

    fn intersection(&self, other: &Chars) -> Chars {
        let mut ranges = Vec::new();

        for (a_lower, a_upper) in &self.0 {
            for (b_lower, b_upper) in &other.0 {
                ranges.push((*a_lower.max(b_lower), *a_upper.min(b_upper)));
            }
        }

        Chars::normalized(ranges)
    }
}

trait Set: Clone {
    fn union(&self, other: &Self) -> Self;
    fn intersection(&self, other: &Self) -> Self;
}

impl Set for Ints {
    fn union(&self, other: &Self) -> Self {
        Ints::union(self, other)
    }

    fn intersection(&self, other: &Self) -> Self {
        Ints::intersection(self, other)
    }
}

impl Set for Chars {
    fn union(&self, other: &Self) -> Self {
        Chars::union(self, other)
    }

    fn intersection(&self, other: &Self) -> Self {
        Chars::intersection(self, other)
    }
}

/// An effective constraint: its root, which is all values if `None`, and
/// if it is extensible.
#[derive(Debug, Clone, PartialEq)]
struct Dim<S> {
    root: Option<S>,
    extensible: bool,
}

impl<S: Set> Dim<S> {
    fn all() -> Dim<S> {
        Dim {
            root: None,
            extensible: false,
        }
    }

    fn of(set: S) -> Dim<S> {
        Dim {
            root: Some(set),
            extensible: false,
        }
    }

    fn union(&self, other: &Dim<S>) -> Dim<S> {
        Dim {
            root: match (&self.root, &other.root) {
                (Some(a), Some(b)) => Some(a.union(b)),
                _ => None,
            },
            extensible: self.extensible || other.extensible,
        }
    }

    fn intersection(&self, other: &Dim<S>) -> Dim<S> {
        Dim {
            root: intersect_roots(&self.root, &other.root),
            extensible: self.extensible || other.extensible,
        }
    }

    /// `next` applied to this one, as a constraint of a type that has one
    /// already (X.680 50.11).
    fn serial(&self, next: &Dim<S>) -> Dim<S> {
        Dim {
            root: intersect_roots(&self.root, &next.root),
            extensible: next.extensible,
        }
    }

    /// The root alone, as a contained subtype gives it (X.680 51.3.3).
    fn root_only(&self) -> Dim<S> {
        Dim {
            root: self.root.clone(),
            extensible: false,
        }
    }
}

fn intersect_roots<S: Set>(a: &Option<S>, b: &Option<S>) -> Option<S> {
    match (a, b) {
        (Some(a), Some(b)) => Some(a.intersection(b)),
        (Some(a), None) => Some(a.clone()),
        (None, b) => b.clone(),
    }
}

/// The constraint of a component in an alternative of `WITH COMPONENTS`.
#[derive(Debug, Clone, PartialEq)]
struct Component {
    name: String,
    /// `Some(true)` if it has to be present, `Some(false)` if absent.
    presence: Option<bool>,
    effective: Effective,
    /// What the constraints of the component can constrain.
    kind: Kind,
}

/// More alternatives than this are all values, as too many to check.
const MAX_ALTERNATIVES: usize = 64;

/// An inner type constraint.
#[derive(Debug, Clone, PartialEq)]
enum Inner {
    /// `WITH COMPONENTS`: the values that satisfy any of the alternatives,
    /// each of which has the constraints of components with different
    /// names, all of which hold.
    Components(Vec<Vec<Component>>),
    /// `WITH COMPONENT`: the values of which each element has the
    /// constraints of any of the alternatives, the same for all.
    Element(Vec<Effective>, Kind),
}

impl Inner {
    /// No constraint, which is the alternative that constrains nothing.
    fn none() -> Inner {
        Inner::Components(vec![Vec::new()])
    }

    fn components(mut alternatives: Vec<Vec<Component>>) -> Inner {
        let mut unique: Vec<Vec<Component>> = Vec::new();

        for alternative in alternatives.drain(..) {
            if !unique.contains(&alternative) {
                unique.push(alternative);
            }
        }

        if unique.len() > MAX_ALTERNATIVES || unique.iter().any(Vec::is_empty) {
            Inner::none()
        } else {
            Inner::Components(unique)
        }
    }

    fn elements(mut alternatives: Vec<Effective>, kind: Kind) -> Inner {
        let mut unique: Vec<Effective> = Vec::new();

        for alternative in alternatives.drain(..) {
            if !unique.contains(&alternative) {
                unique.push(alternative);
            }
        }

        if unique.len() > MAX_ALTERNATIVES || unique.contains(&Effective::all()) {
            Inner::Element(vec![Effective::all()], kind)
        } else {
            Inner::Element(unique, kind)
        }
    }
}

/// The constraints of the components of two alternatives made into one,
/// or `None` if no value satisfies both.
fn merge(a: &[Component], b: &[Component]) -> Option<Vec<Component>> {
    let mut merged = a.to_vec();

    for component in b {
        match merged.iter_mut().find(|other| other.name == component.name) {
            Some(other) => {
                match (other.presence, component.presence) {
                    (Some(x), Some(y)) if x != y => return None,
                    (None, presence) => other.presence = presence,
                    _ => {}
                }

                other.effective = other.effective.intersection(&component.effective);
            }
            None => merged.push(component.clone()),
        }
    }

    Some(merged)
}

impl Set for Inner {
    fn union(&self, other: &Self) -> Self {
        match (self, other) {
            (Inner::Components(a), Inner::Components(b)) => {
                Inner::components(a.iter().chain(b).cloned().collect())
            }
            // All the elements of a value have the constraints of one
            // alternative or all those of another, which is not the
            // elements having those of either.
            (Inner::Element(a, kind), Inner::Element(b, _)) => {
                Inner::elements(a.iter().chain(b).cloned().collect(), *kind)
            }
            _ => Inner::none(),
        }
    }

    fn intersection(&self, other: &Self) -> Self {
        match (self, other) {
            (Inner::Components(a), Inner::Components(b)) => {
                if a.len() * b.len() > MAX_ALTERNATIVES {
                    return Inner::none();
                }

                Inner::components(
                    a.iter()
                        .flat_map(|a| b.iter().filter_map(move |b| merge(a, b)))
                        .collect(),
                )
            }
            (Inner::Element(a, kind), Inner::Element(b, _)) => {
                if a.len() * b.len() > MAX_ALTERNATIVES {
                    return Inner::Element(vec![Effective::all()], *kind);
                }

                Inner::elements(
                    a.iter()
                        .flat_map(|a| b.iter().map(move |b| a.intersection(b)))
                        .collect(),
                    *kind,
                )
            }
            _ => self.clone(),
        }
    }
}

/// The effective constraints of a type.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Effective {
    range: Dim<Ints>,
    size: Dim<Ints>,
    alphabet: Dim<Chars>,
    inner: Dim<Inner>,
}

impl Effective {
    fn all() -> Effective {
        Effective {
            range: Dim::all(),
            size: Dim::all(),
            alphabet: Dim::all(),
            inner: Dim::all(),
        }
    }

    fn union(&self, other: &Effective) -> Effective {
        Effective {
            range: self.range.union(&other.range),
            size: self.size.union(&other.size),
            alphabet: self.alphabet.union(&other.alphabet),
            inner: self.inner.union(&other.inner),
        }
    }

    fn intersection(&self, other: &Effective) -> Effective {
        Effective {
            range: self.range.intersection(&other.range),
            size: self.size.intersection(&other.size),
            alphabet: self.alphabet.intersection(&other.alphabet),
            inner: self.inner.intersection(&other.inner),
        }
    }

    fn serial(&self, next: &Effective) -> Effective {
        Effective {
            range: self.range.serial(&next.range),
            size: self.size.serial(&next.size),
            alphabet: self.alphabet.serial(&next.alphabet),
            inner: self.inner.serial(&next.inner),
        }
    }

    fn root_only(&self) -> Effective {
        Effective {
            range: self.range.root_only(),
            size: self.size.root_only(),
            alphabet: self.alphabet.root_only(),
            inner: self.inner.root_only(),
        }
    }

    fn extensible(mut self) -> Effective {
        self.range.extensible = true;
        self.size.extensible = true;
        self.alphabet.extensible = true;
        self.inner.extensible = true;

        self
    }
}

/// The type a constraint is applied to, as far as its constraints go.
struct Subject {
    kind: Kind,
    named_numbers: Vec<NamedNumber>,
    /// The type it is, once references are followed, and the module it is
    /// found in.
    resolved: Option<(Type, usize)>,
}

impl Subject {
    /// The sizes of a `SIZE`, which are integers.
    fn sizes() -> Subject {
        Subject {
            kind: Kind::Integer,
            named_numbers: Vec::new(),
            resolved: None,
        }
    }
}

/// A component of a type, as a `WITH COMPONENTS` constrains it.
struct Inside {
    name: String,
    /// If it can be constrained to be absent: a member of a SEQUENCE or a
    /// SET that is OPTIONAL, or an alternative of a CHOICE (X.680
    /// 51.8.10).
    can_be_absent: bool,
    type_: Type,
}

/// What the constraints of a type can constrain.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Kind {
    Integer,
    /// A string of characters, with a size and a permitted alphabet.
    String,
    /// A type with a size only.
    Sized,
    /// A SEQUENCE OF or a SET OF, with a size and the constraint of its
    /// elements.
    List,
    /// A SEQUENCE, a SET or a CHOICE, with constraints of its components.
    Constructed,
    Real,
    /// A DURATION, constrained as its equivalent SEQUENCE (X.680 38.4.4).
    Duration,
    Other,
}

fn kind_of_name(name: &str) -> Kind {
    match name {
        "INTEGER" => Kind::Integer,
        "UTF8String" | "NumericString" | "PrintableString" | "TeletexString" | "T61String"
        | "VideotexString" | "IA5String" | "GraphicString" | "VisibleString" | "ISO646String"
        | "GeneralString" | "UniversalString" | "BMPString" => Kind::String,
        "BIT STRING" | "OCTET STRING" => Kind::Sized,
        "SEQUENCE OF" | "SET OF" => Kind::List,
        "SEQUENCE" | "SET" | "CHOICE" => Kind::Constructed,
        "REAL" => Kind::Real,
        "DURATION" => Kind::Duration,
        _ => Kind::Other,
    }
}

/// The effective constraints of the types worked out already, by their
/// module and their name.
pub(super) type Memo = HashMap<(usize, String), Effective>;

/// How deep references and contained subtypes may go.
const DEPTH: usize = 200;

fn integer_of(number: &Number) -> Option<BigInt> {
    number.to_string().parse::<BigInt>().ok()
}

impl Compiler {
    /// The module a type is found in.
    fn module_of_type(&self, type_: &Type, module: usize) -> usize {
        type_
            .common
            .module_name
            .as_deref()
            .and_then(|name| self.module_index(name))
            .unwrap_or(module)
    }

    /// What the constraints of a type can constrain, and what they are
    /// worked out with.
    fn subject_of(&self, type_: &Type, module: usize) -> Subject {
        let module = self.module_of_type(type_, module);
        let (resolved, resolved_module) = self.resolve_type_and_module(type_, module);
        let resolved_module = self.module_of_type(&resolved, resolved_module);

        Subject {
            kind: kind_of_name(&resolved.type_name()),
            named_numbers: match &resolved.body {
                Body::Integer(Some(numbers)) => numbers.clone(),
                _ => Vec::new(),
            },
            resolved: Some((resolved, resolved_module)),
        }
    }

    /// The components of the type of a subject, if it has any, if it is a
    /// CHOICE, and the module their types are found in.
    fn components_of(&self, subject: &Subject) -> Option<(Vec<Inside>, bool, usize)> {
        let (type_, module) = subject.resolved.as_ref()?;
        let mut components = Vec::new();

        fn add(items: &[MemberItem], choice: bool, components: &mut Vec<Inside>) {
            for item in items {
                match item {
                    MemberItem::Member(member) => components.push(Inside {
                        name: member.name.clone(),
                        can_be_absent: choice || member.optional,
                        type_: member.type_.clone(),
                    }),
                    MemberItem::Group(group) => add(group, choice, components),
                    _ => {}
                }
            }
        }

        let choice = match &type_.body {
            Body::Sequence(members) | Body::Set(members) => {
                add(members, false, &mut components);

                false
            }
            Body::Choice(members) => {
                add(members, true, &mut components);

                true
            }
            // The SEQUENCE a REAL is (X.680 21.5).
            Body::Builtin(name) if name == "REAL" => {
                for name in ["mantissa", "base", "exponent"] {
                    components.push(Inside {
                        name: name.to_string(),
                        can_be_absent: false,
                        type_: Type::new(Body::Integer(None)),
                    });
                }

                false
            }
            // DURATION-EQUIVALENT (X.680 38.4.4.2).
            Body::Builtin(name) if name == "DURATION" => {
                let member = |name: &str| {
                    MemberItem::Member(Member {
                        name: name.to_string(),
                        type_: Type::new(Body::Integer(None)),
                        optional: false,
                        default: None,
                        raw_default: None,
                        location: None,
                    })
                };

                for name in [
                    "years", "months", "weeks", "days", "hours", "minutes", "seconds",
                ] {
                    components.push(Inside {
                        name: name.to_string(),
                        can_be_absent: true,
                        type_: Type::new(Body::Integer(None)),
                    });
                }

                components.push(Inside {
                    name: "fractional-part".to_string(),
                    can_be_absent: true,
                    type_: Type::new(Body::Sequence(vec![
                        member("number-of-digits"),
                        member("fractional-value"),
                    ])),
                });

                false
            }
            _ => return None,
        };

        Some((components, choice, *module))
    }

    /// The effective constraints of a type, of its own constraints applied
    /// one after the other to those of the type it refers to.
    pub(super) fn effective_of(
        &self,
        type_: &Type,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Effective> {
        if depth > DEPTH {
            return Err("A type is defined by itself, or is too deep.".to_string());
        }

        let module = self.module_of_type(type_, module);
        let mut effective = match &type_.body {
            Body::Referenced(name) if is_object_class_type_name(name) => {
                match self.lookup_object_class_type(name, module) {
                    Ok(Some((field, field_module))) => {
                        self.effective_of(&field, field_module, memo, depth + 1)?
                    }
                    _ => Effective::all(),
                }
            }
            Body::Referenced(name) => match self.lookup_type(name, module) {
                Ok((m, t)) => {
                    let key = (m, name.clone());

                    match memo.get(&key) {
                        Some(effective) => effective.clone(),
                        None => {
                            let target = self.modules[m].types[t].type_.clone();
                            let effective = self.effective_of(&target, m, memo, depth + 1)?;

                            memo.insert(key, effective.clone());

                            effective
                        }
                    }
                }
                Err(_) => Effective::all(),
            },
            _ => Effective::all(),
        };

        if let Some(constraints) = &type_.common.constraints {
            let subject = self.subject_of(type_, module);

            effective =
                self.apply_constraints(effective, constraints, &subject, module, memo, depth + 1)?;
        }

        // A size is in 0..MAX (X.680 51.5.3).
        if let Some(root) = &effective.size.root {
            effective.size.root = Some(root.intersection(&Ints::range(Bound::int(0), Bound::Max)));
        }

        Ok(effective)
    }

    /// Constraints applied one after the other to the effective
    /// constraints of a type.
    fn apply_constraints(
        &self,
        mut effective: Effective,
        constraints: &[Constraint],
        subject: &Subject,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Effective> {
        for constraint in constraints {
            if let Some(mut next) =
                self.eval_constraint(constraint, subject, module, memo, depth)?
            {
                // An extensible permitted alphabet is not visible, and
                // leaves that of the parent as it is (X.691 10.3.11,
                // 10.3.20).
                if next.alphabet.extensible {
                    next.alphabet = effective.alphabet.root_only();
                }

                effective = effective.serial(&next);
            }
        }

        Ok(effective)
    }

    fn eval_constraint(
        &self,
        constraint: &Constraint,
        subject: &Subject,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Option<Effective>> {
        match constraint {
            Constraint::ElementSetSpecs {
                root, extensible, ..
            } => {
                let effective = self.eval_set(root, subject, module, memo, depth)?;

                Ok(Some(if *extensible {
                    effective.extensible()
                } else {
                    effective
                }))
            }
            // A general constraint, which does not change the values nor
            // that the type is extensible (X.680 50.11, X.682 8.1).
            Constraint::Table(_)
            | Constraint::Contents { .. }
            | Constraint::UserDefined
            | Constraint::Unsupported => Ok(None),
        }
    }

    fn eval_set(
        &self,
        set: &ElementSet,
        subject: &Subject,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Effective> {
        Ok(match set {
            ElementSet::Union(sets) => {
                let mut effective: Option<Effective> = None;

                for set in sets {
                    let next = self.eval_set(set, subject, module, memo, depth)?;

                    effective = Some(match effective {
                        Some(effective) => effective.union(&next),
                        None => next,
                    });
                }

                effective.unwrap_or_else(Effective::all)
            }
            ElementSet::Intersection(sets) => {
                let mut effective: Option<Effective> = None;

                for set in sets {
                    let next = self.eval_set(set, subject, module, memo, depth)?;

                    effective = Some(match effective {
                        Some(effective) => effective.intersection(&next),
                        None => next,
                    });
                }

                effective.unwrap_or_else(Effective::all)
            }
            // What is left out is not looked at (X.691 10.3.21).
            ElementSet::Except(set, _) => self.eval_set(set, subject, module, memo, depth)?,
            ElementSet::All(_) => Effective::all(),
            ElementSet::Element(element) => {
                self.eval_element(element, subject, module, memo, depth)?
            }
        })
    }

    fn eval_element(
        &self,
        element: &Element,
        subject: &Subject,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Effective> {
        let mut effective = Effective::all();

        match element {
            // A single value or a range is only visible for an INTEGER
            // (X.691 10.3.17, 10.3.18).
            Element::Value(value) if subject.kind == Kind::Integer => {
                effective.range =
                    Dim::of(self.ints_of(value, &subject.named_numbers, module, depth)?);
            }
            Element::Value(_) => {}
            Element::ContainedSubtype { type_, .. } => {
                effective = self
                    .effective_of(type_, module, memo, depth + 1)?
                    .root_only();
            }
            Element::Size(constraint) => {
                if let Some(size) =
                    self.eval_constraint(constraint, &Subject::sizes(), module, memo, depth + 1)?
                {
                    effective.size = size.range;
                }
            }
            Element::PermittedAlphabet(constraint) => {
                effective.alphabet = self.eval_chars_constraint(constraint, module, depth + 1)?;
            }
            Element::InnerTypes {
                partial,
                components,
            } => {
                if let Some(inner) =
                    self.eval_components(*partial, components, subject, module, memo, depth)?
                {
                    effective.inner = Dim::of(inner);
                }
            }
            Element::InnerType(constraint) => {
                if let Some((resolved, resolved_module)) = &subject.resolved
                    && let Body::SequenceOf(element, _) | Body::SetOf(element, _) = &resolved.body
                {
                    let element = self.subject_of(element, *resolved_module);
                    let constrained = self.apply_constraints(
                        Effective::all(),
                        std::slice::from_ref(constraint.as_ref()),
                        &element,
                        module,
                        memo,
                        depth + 1,
                    )?;

                    effective.inner = Dim::of(Inner::Element(vec![constrained], element.kind));
                }
            }
            Element::Pattern(_) => {}
        }

        Ok(effective)
    }

    /// The alternative of a `WITH COMPONENTS`, or `None` if a component it
    /// names is not one of the type. A full specification is made a partial
    /// one, with the presence it implies (X.680 51.8.7, 51.8.10.3).
    fn eval_components(
        &self,
        partial: bool,
        named: &[NamedConstraint],
        subject: &Subject,
        module: usize,
        memo: &mut Memo,
        depth: usize,
    ) -> CompileResult<Option<Inner>> {
        let Some((components, choice, components_module)) = self.components_of(subject) else {
            return Ok(None);
        };
        let mut alternative = Vec::new();

        for constraint in named {
            let Some(inside) = components
                .iter()
                .find(|inside| inside.name == constraint.name)
            else {
                return Ok(None);
            };
            let component = self.subject_of(&inside.type_, components_module);
            let effective = self.apply_constraints(
                Effective::all(),
                constraint.constraints.as_deref().unwrap_or_default(),
                &component,
                module,
                memo,
                depth + 1,
            )?;
            let presence = match constraint.presence.as_deref() {
                Some("PRESENT") => Some(true),
                Some("ABSENT") => Some(false),
                Some(_) => None,
                None if !partial && !choice => Some(true),
                None => None,
            };

            alternative.push(Component {
                name: constraint.name.clone(),
                // Only what can be absent is constrained in its presence.
                presence: presence.filter(|_| inside.can_be_absent),
                effective,
                kind: component.kind,
            });
        }

        if !partial {
            for inside in &components {
                if inside.can_be_absent && !named.iter().any(|named| named.name == inside.name) {
                    alternative.push(Component {
                        name: inside.name.clone(),
                        presence: Some(false),
                        effective: Effective::all(),
                        kind: Kind::Other,
                    });
                }
            }
        }

        alternative.retain(|component| {
            component.presence.is_some() || component.effective != Effective::all()
        });

        Ok(Some(Inner::components(vec![alternative])))
    }

    /// The integers of a single value or a range.
    fn ints_of(
        &self,
        value: &Value,
        named_numbers: &[NamedNumber],
        module: usize,
        depth: usize,
    ) -> CompileResult<Ints> {
        match value {
            Value::Range(lower, upper) => Ok(Ints::range(
                self.bound_of(lower, true, named_numbers, module, depth)?,
                self.bound_of(upper, false, named_numbers, module, depth)?,
            )),
            other => {
                let number = Bound::Int(self.integer_value(other, named_numbers, module, depth)?);

                Ok(Ints::range(number.clone(), number))
            }
        }
    }

    fn bound_of(
        &self,
        value: &Value,
        is_lower: bool,
        named_numbers: &[NamedNumber],
        module: usize,
        depth: usize,
    ) -> CompileResult<Bound> {
        match value {
            Value::Min => Ok(Bound::Min),
            Value::Max => Ok(Bound::Max),
            // The next integer in (X.680 51.4.3).
            Value::Excluded(inner) => {
                let bound = self.bound_of(inner, is_lower, named_numbers, module, depth)?;

                Ok(match is_lower {
                    true => bound.next(),
                    false => bound.previous(),
                })
            }
            other => Ok(Bound::Int(self.integer_value(
                other,
                named_numbers,
                module,
                depth,
            )?)),
        }
    }

    /// An integer given as a number, the name of a named number or a
    /// reference to a value.
    fn integer_value(
        &self,
        value: &Value,
        named_numbers: &[NamedNumber],
        module: usize,
        depth: usize,
    ) -> CompileResult<BigInt> {
        if depth > DEPTH {
            return Err("A value is defined by itself, or is too deep.".to_string());
        }

        match value {
            Value::Number(number) => integer_of(number)
                .ok_or_else(|| format!("The bound {number} of a constraint is not an integer.")),
            Value::Identifier(name) => {
                if let Some(named) = named_numbers.iter().find(|named| &named.name == name) {
                    return self.integer_value(&named.number, &[], module, depth + 1);
                }

                let found = self.lookup_value(name, module)?;

                self.integer_value(&found, &[], module, depth + 1)
            }
            other => Err(format!(
                "Expected an integer in a constraint, but got {}.",
                serde_json::to_string(&other.to_tree()).unwrap_or_default()
            )),
        }
    }

    /// The effective permitted alphabet of the constraint of a `FROM`.
    fn eval_chars_constraint(
        &self,
        constraint: &Constraint,
        module: usize,
        depth: usize,
    ) -> CompileResult<Dim<Chars>> {
        match constraint {
            Constraint::ElementSetSpecs {
                root, extensible, ..
            } => {
                let mut dim = self.eval_chars_set(root, module, depth)?;

                dim.extensible |= *extensible;

                Ok(dim)
            }
            _ => Ok(Dim::all()),
        }
    }

    fn eval_chars_set(
        &self,
        set: &ElementSet,
        module: usize,
        depth: usize,
    ) -> CompileResult<Dim<Chars>> {
        Ok(match set {
            ElementSet::Union(sets) => {
                let mut dim: Option<Dim<Chars>> = None;

                for set in sets {
                    let next = self.eval_chars_set(set, module, depth)?;

                    dim = Some(match dim {
                        Some(dim) => dim.union(&next),
                        None => next,
                    });
                }

                dim.unwrap_or_else(Dim::all)
            }
            ElementSet::Intersection(sets) => {
                let mut dim: Option<Dim<Chars>> = None;

                for set in sets {
                    let next = self.eval_chars_set(set, module, depth)?;

                    dim = Some(match dim {
                        Some(dim) => dim.intersection(&next),
                        None => next,
                    });
                }

                dim.unwrap_or_else(Dim::all)
            }
            ElementSet::Except(set, _) => self.eval_chars_set(set, module, depth)?,
            ElementSet::All(_) => Dim::all(),
            ElementSet::Element(Element::Value(value)) => match self.chars_of(value, module, depth)
            {
                Some(chars) => Dim::of(chars),
                None => Dim::all(),
            },
            ElementSet::Element(_) => Dim::all(),
        })
    }

    /// The characters of a string, or of a range of characters, of a
    /// permitted alphabet.
    fn chars_of(&self, value: &Value, module: usize, depth: usize) -> Option<Chars> {
        match value {
            Value::Range(lower, upper) => {
                let lower = self.char_bound(lower, true, module, depth)?;
                let upper = self.char_bound(upper, false, module, depth)?;

                Some(Chars::normalized(vec![(lower, upper)]))
            }
            other => {
                let text = self.text_value(other, module, depth)?;

                Some(Chars::normalized(
                    text.chars().map(|c| (c as u32, c as u32)).collect(),
                ))
            }
        }
    }

    fn char_bound(
        &self,
        value: &Value,
        is_lower: bool,
        module: usize,
        depth: usize,
    ) -> Option<u32> {
        match value {
            Value::Min => Some(0),
            Value::Max => Some(char::MAX as u32),
            Value::Excluded(inner) => {
                let bound = self.char_bound(inner, is_lower, module, depth)?;

                Some(if is_lower {
                    bound.saturating_add(1)
                } else {
                    bound.saturating_sub(1)
                })
            }
            other => {
                let text = self.text_value(other, module, depth)?;
                let mut chars = text.chars();
                let first = chars.next()?;

                // A bound is a single character (X.680 51.4.4 NOTE).
                chars.next().is_none().then_some(first as u32)
            }
        }
    }

    fn text_value(&self, value: &Value, module: usize, depth: usize) -> Option<String> {
        if depth > DEPTH {
            return None;
        }

        match value {
            Value::Str(text) => Some(text.clone()),
            Value::Identifier(name) => {
                let found = self.lookup_value(name, module).ok()?;

                self.text_value(&found, module, depth + 1)
            }
            _ => None,
        }
    }

    /// Writes the effective constraints of a type, and of the types in it,
    /// as the compiled specification has them.
    pub(super) fn write_effective(
        &self,
        type_: &mut Type,
        module: usize,
        memo: &mut Memo,
    ) -> CompileResult<()> {
        let module = self.module_of_type(type_, module);

        match &mut type_.body {
            Body::Sequence(members) | Body::Set(members) | Body::Choice(members) => {
                for item in members.iter_mut() {
                    match item {
                        MemberItem::Member(member) => {
                            self.write_effective(&mut member.type_, module, memo)?
                        }
                        MemberItem::Group(group) => {
                            for inner in group.iter_mut() {
                                if let MemberItem::Member(member) = inner {
                                    self.write_effective(&mut member.type_, module, memo)?;
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
                self.write_effective(element, module, memo)?;
            }
            _ => {}
        }

        // What the parser made of the constraints is replaced with what
        // they come to.
        type_.common.with_components = None;
        type_.common.with_component = None;

        if type_.common.constraints.is_none() {
            return Ok(());
        }

        let kind = self.subject_of(type_, module).kind;
        let effective = self.effective_of(type_, module, memo, 0)?;
        // A reference has those of its effective constraints written that
        // are not those of the type it refers to, which are taken from it
        // otherwise.
        let inherited = match type_.body {
            Body::Referenced(_) => {
                let mut bare = type_.clone();

                bare.common.constraints = None;

                Some(self.effective_of(&bare, module, memo, 0)?)
            }
            _ => None,
        };
        let constraint = constraint_values(&effective, inherited.as_ref(), kind);
        let common = &mut type_.common;

        common.size = constraint.size;
        common.permitted_alphabet = constraint.permitted_alphabet;
        common.with_components = constraint.with_components;
        common.with_component = constraint.with_component;

        // The value constraints of a REAL are not visible, and are kept as
        // they are written.
        if kind != Kind::Real {
            common.restricted_to = constraint.restricted_to;
        }

        Ok(())
    }
}

/// Effective constraints as a compiled specification has them, of what
/// a type of `kind` has: each is `None` if it is that of `inherited`, the
/// effective constraints of the type referred to, if any, or else no
/// constraint.
fn constraint_values(
    effective: &Effective,
    inherited: Option<&Effective>,
    kind: Kind,
) -> ElementConstraint {
    let mut constraint = ElementConstraint::default();

    match kind {
        Kind::Integer => {
            constraint.restricted_to = ints_values(
                &effective.range,
                inherited.map(|inherited| &inherited.range),
            );
        }
        Kind::Sized | Kind::String | Kind::List => {
            constraint.size =
                ints_values(&effective.size, inherited.map(|inherited| &inherited.size));
        }
        _ => {}
    }

    if kind == Kind::String {
        constraint.permitted_alphabet = chars_values(
            &effective.alphabet,
            inherited.map(|inherited| &inherited.alphabet),
        );
    }

    let inner = &effective.inner;
    // What is checked of an inner type constraint: nothing if it is all
    // values or extensible.
    let checked = |dim: &Dim<Inner>| match dim.extensible {
        false => dim.root.clone(),
        true => None,
    };

    if checked(inner) == inherited.and_then(|inherited| checked(&inherited.inner)) {
        return constraint;
    }

    match kind {
        Kind::Constructed | Kind::Real | Kind::Duration => {
            let mut values: Vec<Value> = match &inner.root {
                Some(Inner::Components(alternatives)) => alternatives
                    .iter()
                    .map(|alternative| {
                        Value::List(alternative.iter().map(component_value).collect())
                    })
                    .collect(),
                _ => vec![Value::List(Vec::new())],
            };

            if inner.extensible {
                values.push(Value::ExtensionMarker);
            }

            constraint.with_components = Some(values);
        }
        Kind::List => {
            let mut values: Vec<Value> = match &inner.root {
                Some(Inner::Element(alternatives, element_kind)) => alternatives
                    .iter()
                    .map(|alternative| {
                        Value::Element(Box::new(constraint_values(
                            alternative,
                            None,
                            *element_kind,
                        )))
                    })
                    .collect(),
                _ => vec![Value::Element(Box::default())],
            };

            if inner.extensible {
                values.push(Value::ExtensionMarker);
            }

            constraint.with_component = Some(values);
        }
        _ => {}
    }

    constraint
}

/// A constraint of a component, as a compiled specification has it.
fn component_value(component: &Component) -> Value {
    Value::Component(Box::new(ComponentConstraint {
        name: component.name.clone(),
        presence: component
            .presence
            .map(|present| if present { "PRESENT" } else { "ABSENT" }.to_string()),
        constraint: constraint_values(&component.effective, None, component.kind),
    }))
}

fn bound_value(bound: &Bound) -> Value {
    match bound {
        Bound::Min => Value::Min,
        Bound::Max => Value::Max,
        Bound::Int(number) => Value::Number(
            number
                .to_string()
                .parse::<Number>()
                .unwrap_or_else(|_| Number::from(0)),
        ),
    }
}

/// The root of an effective constraint of integers as values and ranges,
/// and an extension marker if it is extensible. `None` if it is that of
/// the type referred to, if any, or else no constraint.
fn ints_values(dim: &Dim<Ints>, inherited: Option<&Dim<Ints>>) -> Option<Vec<Value>> {
    let unchanged = match inherited {
        Some(inherited) => dim == inherited,
        None => dim.root.is_none() && !dim.extensible,
    };

    if unchanged {
        return None;
    }

    let mut values: Vec<Value> = match &dim.root {
        None => vec![Value::Range(Box::new(Value::Min), Box::new(Value::Max))],
        Some(ints) => ints
            .0
            .iter()
            .map(|(lower, upper)| {
                if lower == upper {
                    bound_value(lower)
                } else {
                    Value::Range(Box::new(bound_value(lower)), Box::new(bound_value(upper)))
                }
            })
            .collect(),
    };

    if dim.extensible {
        values.push(Value::ExtensionMarker);
    }

    Some(values)
}

/// The root of an effective permitted alphabet as ranges of characters,
/// or `None` if it is that of the type referred to, if any, or else no
/// permitted alphabet.
fn chars_values(dim: &Dim<Chars>, inherited: Option<&Dim<Chars>>) -> Option<Vec<Value>> {
    if inherited == Some(dim) {
        return None;
    }

    // None is not a reference, as the root of a reference is not more than
    // that of the type it refers to.
    dim.root.as_ref().map(|chars| {
        let range = |lower: u32, upper: u32| {
            Some(Value::Range(
                Box::new(Value::Str(char::from_u32(lower)?.to_string())),
                Box::new(Value::Str(char::from_u32(upper)?.to_string())),
            ))
        };

        // The surrogates are not characters, and are left out.
        chars
            .0
            .iter()
            .flat_map(|(lower, upper)| {
                [
                    (*lower <= 0xD7FF).then(|| range(*lower, (*upper).min(0xD7FF))),
                    (*upper >= 0xE000).then(|| range((*lower).max(0xE000), *upper)),
                ]
            })
            .flatten()
            .flatten()
            .collect()
    })
}
