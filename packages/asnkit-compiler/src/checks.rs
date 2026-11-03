// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! What is checked of the types besides what the resolver needs: a type
//! defined by itself alone, a type with no finite value and a constraint
//! that permits no value, which are errors, and what is valid but likely
//! to cause trouble, which is a warning.
//!
//! A type defined by itself is one that is a reference to a type that
//! is, through other references, a reference to it, as `A ::= B` and
//! `B ::= [0] A`, or one that takes `COMPONENTS OF` itself. Neither has
//! a type it comes to. The runtimes refuse a specification with the
//! first, and the resolver would follow either without end. A type of
//! which every value would hold another has no value that can be encoded
//! or decoded. Constraints that permit no value are an error too, as the
//! runtimes would take them for no constraint.
//!
//! The warnings are of a type that refers to itself, which is fine but
//! for how deep its values nest.

use super::{CompileResult, Compiler, is_builtin_type_name, is_object_class_type_name};
use crate::Warning;
use crate::ast::*;

/// The type assignments of all the modules, numbered one after the
/// other in the order they are in.
struct Assignments {
    /// The number of the first of each module.
    starts: Vec<usize>,
    /// The module of each and its place in it.
    places: Vec<(usize, usize)>,
}

impl Assignments {
    fn of(compiler: &Compiler) -> Assignments {
        let mut starts = Vec::new();
        let mut places = Vec::new();

        for (m, module) in compiler.modules.iter().enumerate() {
            starts.push(places.len());
            places.extend((0..module.types.len()).map(|t| (m, t)));
        }

        Assignments { starts, places }
    }

    fn len(&self) -> usize {
        self.places.len()
    }

    fn id(&self, (module, index): (usize, usize)) -> usize {
        self.starts[module] + index
    }
}

/// What a type needs for a value of it to be made, one that ends.
enum Needs {
    Nothing,
    /// A value of the type assigned to this number.
    Type(usize),
    /// Each of these.
    All(Vec<Needs>),
    /// One of these. None if there are none.
    Any(Vec<Needs>),
}

impl Needs {
    fn met(&self, has_value: &dyn Fn(usize) -> bool) -> bool {
        match self {
            Needs::Nothing => true,
            Needs::Type(id) => has_value(*id),
            Needs::All(needs) => needs.iter().all(|needs| needs.met(has_value)),
            Needs::Any(needs) => needs.iter().any(|needs| needs.met(has_value)),
        }
    }
}

/// The members of a SEQUENCE, a SET or a CHOICE, those of a group of
/// additions with them, each with whether it is an extension addition.
fn members_with_place(items: &[MemberItem]) -> Vec<(&Member, bool)> {
    let mut members = Vec::new();
    let mut markers = 0;

    for item in items {
        match item {
            MemberItem::Member(member) => members.push((member, markers == 1)),
            MemberItem::Group(group) => {
                for inner in group {
                    if let MemberItem::Member(member) = inner {
                        members.push((member, true));
                    }
                }
            }
            MemberItem::ExtensionMarker => markers += 1,
            MemberItem::ComponentsOf(_) => {}
        }
    }

    members
}

fn is_extensible(items: &[MemberItem]) -> bool {
    items
        .iter()
        .any(|item| matches!(item, MemberItem::ExtensionMarker))
}

/// If an effective size constraint permits no element, which is also
/// the case if it is not known.
fn permits_empty(size: &Option<Vec<Value>>) -> bool {
    let Some(values) = size else {
        return true;
    };

    // No value at all is told of as an empty constraint, and an
    // extensible one permits any size.
    if values.is_empty() || values.contains(&Value::ExtensionMarker) {
        return true;
    }

    let at_most_zero = |value: &Value| match value {
        Value::Min => true,
        Value::Number(number) => number.as_i64().is_some_and(|n| n <= 0),
        _ => false,
    };
    let at_least_zero = |value: &Value| match value {
        Value::Max => true,
        Value::Number(number) => number.as_i64().is_none_or(|n| n >= 0),
        _ => false,
    };

    values.iter().any(|value| match value {
        Value::Range(lower, upper) => at_most_zero(lower) && at_least_zero(upper),
        value => at_most_zero(value) && at_least_zero(value),
    })
}

/// The first constraint, in a type and in the types in it, that permits
/// no value, by its path from the assignment, as `T.member[]` for the
/// elements of a member, and where it is.
fn empty_constraint(
    type_: &Type,
    path: &str,
    location: Option<Location>,
) -> Option<(String, Option<Location>)> {
    let empty = |values: &Option<Vec<Value>>| values.as_ref().is_some_and(Vec::is_empty);

    if empty(&type_.common.restricted_to) || empty(&type_.common.size) {
        return Some((path.to_string(), location));
    }

    match &type_.body {
        Body::Sequence(items) | Body::Set(items) | Body::Choice(items) => members_with_place(items)
            .into_iter()
            .find_map(|(member, _)| {
                empty_constraint(
                    &member.type_,
                    &format!("{path}.{}", member.name),
                    member.location.or(location),
                )
            }),
        Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
            empty_constraint(element, &format!("{path}[]"), location)
        }
        _ => None,
    }
}

/// The groups of assignments each of which refers to the others, through
/// `edges`, in no particular order (Tarjan's algorithm, with a stack of
/// its own, as a chain of references may be long).
fn strongly_connected(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    const UNSEEN: usize = usize::MAX;

    let count = edges.len();
    let mut index = vec![UNSEEN; count];
    let mut low = vec![0; count];
    let mut on_stack = vec![false; count];
    let mut stack = Vec::new();
    let mut components = Vec::new();
    let mut next = 0;

    for root in 0..count {
        if index[root] != UNSEEN {
            continue;
        }

        // Each node being visited, and how many of its edges are done.
        let mut work = vec![(root, 0)];
        index[root] = next;
        low[root] = next;
        next += 1;
        stack.push(root);
        on_stack[root] = true;

        while let Some(&(node, done)) = work.last() {
            if let Some(&target) = edges[node].get(done) {
                work.last_mut().unwrap().1 += 1;

                if index[target] == UNSEEN {
                    index[target] = next;
                    low[target] = next;
                    next += 1;
                    stack.push(target);
                    on_stack[target] = true;
                    work.push((target, 0));
                } else if on_stack[target] {
                    low[node] = low[node].min(index[target]);
                }

                continue;
            }

            work.pop();

            if let Some(&(parent, _)) = work.last() {
                low[parent] = low[parent].min(low[node]);
            }

            if low[node] == index[node] {
                let mut component = Vec::new();

                loop {
                    let member = stack.pop().unwrap();
                    on_stack[member] = false;
                    component.push(member);

                    if member == node {
                        break;
                    }
                }

                component.sort_unstable();
                components.push(component);
            }
        }
    }

    components
}

/// The groups of `strongly_connected()` that go round, by their first
/// assignment: those of more than one, and those of one that refers to
/// itself.
fn cycles(edges: &[Vec<usize>]) -> Vec<Vec<usize>> {
    let mut cycles: Vec<Vec<usize>> = strongly_connected(edges)
        .into_iter()
        .filter(|component| component.len() > 1 || edges[component[0]].contains(&component[0]))
        .collect();

    cycles.sort_unstable_by_key(|cycle| cycle[0]);

    cycles
}

/// How many names a message lists, of a group of types that refer to
/// one another, which may be large.
const LISTED: usize = 5;

/// `'B'`, `'B' and 'C'` or `'B', 'C' and 'D'`, and of more than
/// [`LISTED`], the first of them and how many more there are.
fn listed(names: &[String]) -> String {
    let quoted = |names: &[String]| {
        names
            .iter()
            .map(|name| format!("'{name}'"))
            .collect::<Vec<_>>()
            .join(", ")
    };

    match names {
        [] => String::new(),
        [name] => format!("'{name}'"),
        names if names.len() > LISTED => format!(
            "{} and {} more",
            quoted(&names[..LISTED]),
            names.len() - LISTED
        ),
        [first @ .., last] => format!("{} and '{last}'", quoted(first)),
    }
}

impl Compiler {
    /// The module a type refers to the types it has from: the one it
    /// names, as a type of a parameterized type of another module does.
    fn context(&self, type_: &Type, module: usize) -> usize {
        type_
            .common
            .module_name
            .as_deref()
            .and_then(|name| self.module_index(name))
            .unwrap_or(module)
    }

    /// The assignment a reference is to, if it is one that is found.
    fn referred(&self, type_: &Type, module: usize) -> Option<(usize, usize)> {
        let Body::Referenced(name) = &type_.body else {
            return None;
        };

        if is_builtin_type_name(name) || is_object_class_type_name(name) {
            return None;
        }

        self.lookup_type(name, self.context(type_, module)).ok()
    }

    /// Where something is, as a message tells it after what it is.
    fn place(&self, location: Option<Location>) -> String {
        let Some((source, line, column)) = location else {
            return String::new();
        };
        let file = self
            .files
            .as_ref()
            .and_then(|files| files.get(source))
            .map(|name| format!("in '{name}' "))
            .unwrap_or_default();

        format!(", {file}at line {line}, column {column}")
    }

    /// The names of assignments, and ` in module 'M'` if they are all of
    /// that module, or else the names as `M.Name`.
    fn names(&self, assignments: &Assignments, ids: &[usize]) -> (Vec<String>, String) {
        let module = assignments.places[ids[0]].0;
        let one_module = ids.iter().all(|id| assignments.places[*id].0 == module);
        let names = ids
            .iter()
            .map(|id| {
                let (m, t) = assignments.places[*id];
                let name = &self.modules[m].types[t].name;

                match one_module {
                    true => name.clone(),
                    false => format!("{}.{name}", self.modules[m].name),
                }
            })
            .collect();
        let where_ = match one_module {
            true => format!(" in module '{}'", self.modules[module].name),
            false => String::new(),
        };

        (names, where_)
    }

    fn location_of(&self, assignments: &Assignments, id: usize) -> Option<Location> {
        let (m, t) = assignments.places[id];

        self.modules[m].types[t].location
    }

    /// Fails if a type is defined by itself alone: through references,
    /// as `A ::= B` and `B ::= [0] A`, or through `COMPONENTS OF`. A
    /// parameterized type is not looked into, and a type that is one
    /// with its actual parameters is not followed, as it is not yet what
    /// it is; this is checked again once it is.
    pub(super) fn check_definitions(&self) -> CompileResult<()> {
        let assignments = Assignments::of(self);
        let mut aliases = vec![Vec::new(); assignments.len()];
        let mut components = vec![Vec::new(); assignments.len()];

        for (id, &(m, t)) in assignments.places.iter().enumerate() {
            let type_ = &self.modules[m].types[t].type_;

            if type_.common.parameters.is_some() {
                continue;
            }

            if type_.common.actual_parameters.is_none() {
                if let Some(target) = self.referred(type_, m) {
                    aliases[id].push(assignments.id(target));
                }
            }

            if let Body::Sequence(items) | Body::Set(items) | Body::Choice(items) = &type_.body {
                for item in items {
                    if let MemberItem::ComponentsOf(name) = item {
                        if let Ok(target) = self.lookup_type(name, m) {
                            components[id].push(assignments.id(target));
                        }
                    }
                }
            }
        }

        if let Some(cycle) = cycles(&aliases).first() {
            // In the order they refer to one another, from the first.
            let mut order = vec![cycle[0]];

            while let Some(&next) = aliases[*order.last().unwrap()].first() {
                if next == cycle[0] {
                    break;
                }

                order.push(next);
            }

            return Err(self.defined_by_itself(&assignments, &order, "is defined by itself"));
        }

        if let Some(cycle) = cycles(&components).first() {
            return Err(self.defined_by_itself(&assignments, cycle, "takes COMPONENTS OF itself"));
        }

        Ok(())
    }

    fn defined_by_itself(&self, assignments: &Assignments, cycle: &[usize], what: &str) -> String {
        let (names, where_) = self.names(assignments, cycle);
        let through = match names.len() {
            1 => String::new(),
            _ => format!(", through {}", listed(&names[1..])),
        };

        format!(
            "Type '{}'{where_} {what}{through}{}.",
            names[0],
            self.place(self.location_of(assignments, cycle[0]))
        )
    }

    /// The types a type refers to, in it and in the types in it.
    fn references(
        &self,
        type_: &Type,
        module: usize,
        assignments: &Assignments,
        out: &mut Vec<usize>,
    ) {
        let module = self.context(type_, module);

        if let Some(target) = self.referred(type_, module) {
            out.push(assignments.id(target));
        }

        match &type_.body {
            Body::Sequence(items) | Body::Set(items) | Body::Choice(items) => {
                for (member, _) in members_with_place(items) {
                    self.references(&member.type_, module, assignments, out);
                }
            }
            Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
                self.references(element, module, assignments, out);
            }
            Body::AnyDefinedBy(_, choices) => {
                for (_, choice) in choices {
                    self.references(choice, module, assignments, out);
                }
            }
            _ => {}
        }
    }

    /// What a value of a type needs. An extension addition is not needed,
    /// as a value of a version of the type before it is one without it,
    /// and a CHOICE that is extensible may have an alternative that is
    /// not known.
    fn needs(&self, type_: &Type, module: usize, assignments: &Assignments) -> Needs {
        let module = self.context(type_, module);

        match &type_.body {
            Body::Referenced(_) => match self.referred(type_, module) {
                Some(target) => Needs::Type(assignments.id(target)),
                None => Needs::Nothing,
            },
            Body::Sequence(items) | Body::Set(items) => Needs::All(
                members_with_place(items)
                    .into_iter()
                    .filter(|(member, addition)| {
                        !addition && !member.optional && member.default.is_none()
                    })
                    .map(|(member, _)| self.needs(&member.type_, module, assignments))
                    .collect(),
            ),
            Body::Choice(items) if !is_extensible(items) => Needs::Any(
                members_with_place(items)
                    .into_iter()
                    .map(|(member, _)| self.needs(&member.type_, module, assignments))
                    .collect(),
            ),
            Body::SequenceOf(element, _) | Body::SetOf(element, _)
                if !permits_empty(&type_.common.size) =>
            {
                self.needs(element, module, assignments)
            }
            _ => Needs::Nothing,
        }
    }

    /// Fails at the first constraint, in the order of the types, that
    /// permits no value, as `INTEGER (5..1)`, or `(10..20)` on a type
    /// that is `(0..5)`: one of which the root has no value and that is
    /// not extensible, as an extensible one permits values that are not
    /// in its root. The runtimes would take it for no constraint.
    pub(super) fn check_constraints(&self) -> CompileResult<()> {
        for module in &self.modules {
            for assignment in &module.types {
                let Some((path, location)) =
                    empty_constraint(&assignment.type_, &assignment.name, assignment.location)
                else {
                    continue;
                };
                let what = match path == assignment.name {
                    true => "Type",
                    false => "The type of",
                };

                return Err(format!(
                    "{what} '{path}' in module '{}' permits no value, as its constraints have \
                     none in common{}.",
                    module.name,
                    self.place(location)
                ));
            }
        }

        Ok(())
    }

    /// The types each assignment refers to, in it and in the types in it.
    fn edges(&self, assignments: &Assignments) -> Vec<Vec<usize>> {
        assignments
            .places
            .iter()
            .map(|&(m, t)| {
                let mut edges = Vec::new();

                self.references(&self.modules[m].types[t].type_, m, assignments, &mut edges);

                edges
            })
            .collect()
    }

    /// Fails at the first group of types that refer to one another, in
    /// the order of the types, of which some have no finite value, as
    /// `Never ::= SEQUENCE { next Never }`: each of their values would
    /// hold another, so none can be encoded or decoded. Of a group, those
    /// are told of that have none if the types outside it that they need
    /// have one, so that a type that has none only as a type it needs has
    /// none is not, but that type is.
    pub(super) fn check_finite(&self) -> CompileResult<()> {
        let assignments = Assignments::of(self);
        let edges = self.edges(&assignments);
        let needs: Vec<Needs> = assignments
            .places
            .iter()
            .map(|&(m, t)| self.needs(&self.modules[m].types[t].type_, m, &assignments))
            .collect();
        // Of the group being looked at. An assignment is in one group at
        // most.
        let mut in_cycle = vec![false; assignments.len()];
        let mut has_value = vec![false; assignments.len()];

        for cycle in cycles(&edges) {
            for id in &cycle {
                in_cycle[*id] = true;
            }

            loop {
                let mut changed = false;

                for id in &cycle {
                    if !has_value[*id]
                        && needs[*id].met(&|other| !in_cycle[other] || has_value[other])
                    {
                        has_value[*id] = true;
                        changed = true;
                    }
                }

                if !changed {
                    break;
                }
            }

            for id in &cycle {
                in_cycle[*id] = false;
            }

            let without: Vec<usize> = cycle.into_iter().filter(|id| !has_value[*id]).collect();

            if without.is_empty() {
                continue;
            }

            let (names, where_) = self.names(&assignments, &without);
            let (subject, has, their) = match without.len() {
                1 => ("Type", "has", "its"),
                _ => ("Types", "have", "their"),
            };

            return Err(format!(
                "{subject} {}{where_} {has} no finite value, as each of {their} values would \
                 hold another{}.",
                listed(&names),
                self.place(self.location_of(&assignments, without[0]))
            ));
        }

        Ok(())
    }

    /// What is valid in the types but likely to cause trouble, in the
    /// order of the types: a warning for each group of types that refer
    /// to one another, by the first of them.
    pub fn warnings(&self) -> Vec<Warning> {
        let assignments = Assignments::of(self);

        cycles(&self.edges(&assignments))
            .into_iter()
            .map(|cycle| {
                let (names, where_) = self.names(&assignments, &cycle);
                let place = self.place(self.location_of(&assignments, cycle[0]));
                let (subject, refers, their) = match cycle.len() {
                    1 => ("Type", "refers to itself", "its"),
                    2 => ("Types", "refer to each other", "their"),
                    _ => ("Types", "refer to one another", "their"),
                };

                Warning {
                    kind: "RecursiveType".to_string(),
                    message: format!(
                        "{subject} {}{where_} {refers}{place}: {their} values nest as deep as \
                         the data has them, and asnkit decodes no more than 128 levels of \
                         nesting.",
                        listed(&names)
                    ),
                }
            })
            .collect()
    }
}
