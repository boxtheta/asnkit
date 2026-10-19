// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Explicit mappings supplied alongside ASN.1 source.

use std::collections::{BTreeMap, HashSet};

use num_bigint::BigInt;
use serde::Deserialize;

use super::{CompileResult, Compiler, Section, is_builtin_type_name};
use crate::Error;
use crate::ast::*;

/// Options shared by the compiler library and the TOML configuration files.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompileOptions {
    #[serde(default)]
    pub any_defined_by: Vec<AnyDefinedBy>,
}

/// Choices for one inline ANY DEFINED BY member of an assigned type.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AnyDefinedBy {
    /// The containing assignment, as Module.Type.
    #[serde(rename = "type")]
    pub type_name: String,
    /// Member names separated by dots; collection elements consume no name.
    pub member: String,
    /// Selector values or ASN.1 value names, mapped to type names.
    pub choices: BTreeMap<String, String>,
}

impl CompileOptions {
    /// Reads one TOML configuration. Append `any_defined_by` to combine files;
    /// compilation rejects duplicate targets and duplicate resolved selectors.
    pub fn from_toml(text: &str) -> Result<Self, Error> {
        toml::from_str(text).map_err(|error| Error {
            kind: "CompileError".into(),
            message: format!("Invalid ANY DEFINED BY configuration: {error}"),
        })
    }
}

fn members(items: &[MemberItem]) -> Vec<&Member> {
    items
        .iter()
        .flat_map(|item| match item {
            MemberItem::Member(member) => vec![member],
            MemberItem::Group(group) => members(group),
            _ => Vec::new(),
        })
        .collect()
}

// Paths only enter inline types. Following a named type here would modify
// other uses of that type or require a new specialization of its constraints.
fn target<'a>(ty: &'a mut Type, path: &[&str]) -> CompileResult<(&'a mut Type, Type)> {
    let record = matches!(ty.body, Body::Sequence(_) | Body::Set(_));
    match &mut ty.body {
        Body::SequenceOf(element, _) | Body::SetOf(element, _) => target(element, path),
        Body::Sequence(items) | Body::Set(items) | Body::Choice(items) => {
            let Some((name, rest)) = path.split_first() else {
                return Err("Expected a member path.".into());
            };
            let all = members(items);
            let member = all
                .iter()
                .find(|member| member.name == *name)
                .ok_or_else(|| format!("Member '{name}' was not found."))?;
            let governor = if rest.is_empty() {
                if !record {
                    return Err("An ANY DEFINED BY member must be in a SEQUENCE or SET.".into());
                }
                let Body::AnyDefinedBy(selector, _) = &member.type_.body else {
                    return Err(format!("Member '{name}' is not an ANY DEFINED BY."));
                };
                Some(
                    all.iter()
                        .find(|member| member.name == *selector)
                        .ok_or_else(|| format!("Defining member '{selector}' was not found."))?
                        .type_
                        .clone(),
                )
            } else {
                None
            };
            fn find<'a>(items: &'a mut [MemberItem], name: &str) -> Option<&'a mut Member> {
                items.iter_mut().find_map(|item| match item {
                    MemberItem::Member(member) if member.name == name => Some(member),
                    MemberItem::Group(group) => find(group, name),
                    _ => None,
                })
            }
            let member = find(items, name).unwrap();
            match governor {
                Some(governor) => Ok((&mut member.type_, governor)),
                None => target(&mut member.type_, rest),
            }
        }
        Body::Referenced(name) => Err(format!(
            "The member path enters referenced type '{name}'; configure that type directly."
        )),
        _ => Err("The member path does not lead through a constructed type.".into()),
    }
}

impl Compiler {
    pub fn apply_any_defined_by(&mut self, options: &CompileOptions) -> CompileResult<()> {
        let mut targets = HashSet::new();
        for mapping in &options.any_defined_by {
            let context = format!("ANY DEFINED BY {}.{}", mapping.type_name, mapping.member);
            let apply = |compiler: &mut Self| -> CompileResult<()> {
                let (module_name, name) = mapping
                    .type_name
                    .split_once('.')
                    .filter(|(module, name)| {
                        !module.is_empty() && !name.is_empty() && !name.contains('.')
                    })
                    .ok_or("Expected the containing type as Module.Type.")?;
                let module = compiler
                    .module_index(module_name)
                    .ok_or_else(|| format!("Module '{module_name}' was not found."))?;
                let index = compiler
                    .position(module, Section::Types, name)
                    .ok_or_else(|| {
                        format!("Type '{name}' was not found in module '{module_name}'.")
                    })?;
                if mapping.member.split('.').any(str::is_empty) {
                    return Err("Expected a nonempty member path.".into());
                }
                if mapping.choices.is_empty() {
                    return Err(
                        "Expected at least one choice; fallback types are not supported.".into(),
                    );
                }
                let mut ty = compiler.modules[module].types[index].type_.clone();
                let path: Vec<_> = mapping.member.split('.').collect();
                let (target, governor) = target(&mut ty, &path)?;
                let (governor, governor_module) = compiler.mapping_descriptor(&governor, module)?;
                let mut choices = Vec::new();
                let mut keys = HashSet::new();
                for (key, type_name) in &mapping.choices {
                    let values = compiler.selector(key, &governor, module, governor_module)?;
                    let canonical = serde_json::to_string(&values[0].to_tree()).unwrap();
                    if !keys.insert(canonical) {
                        return Err(format!("Duplicate selector value for key '{key}'."));
                    }
                    let choice = compiler.mapping_type(type_name, module)?;
                    for value in values {
                        choices.push((value, choice.clone()));
                    }
                }
                let Body::AnyDefinedBy(_, target_choices) = &mut target.body else {
                    unreachable!()
                };
                *target_choices = choices;
                compiler.modules[module].types[index].type_ = ty;
                Ok(())
            };
            if !targets.insert((&mapping.type_name, &mapping.member)) {
                return Err(format!("{context}: Duplicate mapping for this member."));
            }
            apply(self).map_err(|error| format!("{context}: {error}"))?;
        }
        Ok(())
    }

    fn mapping_lookup(
        &self,
        name: &str,
        module: usize,
        section: Section,
    ) -> CompileResult<(usize, usize)> {
        match name.split_once('.') {
            Some((module, name)) => {
                let module = self
                    .module_index(module)
                    .ok_or_else(|| format!("Unknown module in '{module}.{name}'."))?;
                self.position(module, section, name)
                    .map(|index| (module, index))
                    .ok_or_else(|| format!("Unknown {} '{name}'.", section.debug_string()))
            }
            None => {
                let mut module = module;
                let mut visited = HashSet::new();
                loop {
                    if !visited.insert(module) {
                        return Err(format!("Cyclic import while looking up '{name}'."));
                    }
                    if let Some(index) = self.position(module, section, name) {
                        return Ok((module, index));
                    }
                    let scope = &self.modules[module];
                    let import = scope
                        .imports
                        .iter()
                        .find(|import| import.symbols.iter().any(|symbol| symbol == name))
                        .ok_or_else(|| {
                            format!(
                                "Unknown {} '{name}' in module '{}'.",
                                section.debug_string(),
                                scope.name
                            )
                        })?;
                    module = self
                        .module_index(&import.module)
                        .ok_or_else(|| format!("Unknown imported module '{}'.", import.module))?;
                }
            }
        }
    }

    fn mapping_descriptor(&self, ty: &Type, module: usize) -> CompileResult<(Type, usize)> {
        let mut ty = ty.clone();
        let mut module = module;
        let mut seen = HashSet::new();
        while let Body::Referenced(name) = &ty.body {
            if let Some(name) = &ty.common.module_name {
                module = self.module_index(name).ok_or("Unknown type module.")?;
            }
            let found = self.mapping_lookup(name, module, Section::Types)?;
            if !seen.insert(found) {
                return Err("Cyclic type reference in mapping.".into());
            }
            module = found.0;
            ty = self.modules[found.0].types[found.1].type_.clone();
        }
        Ok((ty, module))
    }

    fn mapping_type(&self, name: &str, module: usize) -> CompileResult<Type> {
        let body = match name {
            "INTEGER" => Body::Integer(None),
            "BIT STRING" => Body::BitString(None),
            "SEQUENCE" | "SET" | "CHOICE" | "ENUMERATED" | "SEQUENCE OF" | "SET OF"
            | "ANY DEFINED BY" => {
                return Err(format!(
                    "'{name}' needs a type definition; use an assigned type name."
                ));
            }
            name if is_builtin_type_name(name) => Body::Builtin(name.into()),
            _ => {
                let (module, index) = self.mapping_lookup(name, module, Section::Types)?;
                let assignment = &self.modules[module].types[index];
                if assignment
                    .type_
                    .common
                    .parameters
                    .as_ref()
                    .is_some_and(|parameters| !parameters.is_empty())
                {
                    return Err(format!(
                        "'{name}' needs actual parameters; use an instantiated type."
                    ));
                }
                let mut ty = Type::new(Body::Referenced(assignment.name.clone()));
                ty.common.module_name = Some(self.modules[module].name.clone());
                return Ok(ty);
            }
        };
        Ok(Type::new(body))
    }

    fn selector(
        &self,
        key: &str,
        governor: &Type,
        module: usize,
        governor_module: usize,
    ) -> CompileResult<Vec<Value>> {
        match &governor.body {
            Body::Integer(numbers) => {
                let named = numbers.iter().flatten().find(|number| number.name == key);
                let value = match named {
                    Some(number) => self.mapping_integer(&number.number, governor_module, 0)?,
                    None => self.mapping_integer(&Value::Identifier(key.into()), module, 0)?,
                };
                // The existing runtime selector representation is signed i128.
                let integer = value
                    .to_string()
                    .parse::<i128>()
                    .map_err(|_| "An INTEGER selector must fit in signed 128 bits.")?;
                Ok(vec![Value::Number(integer.to_string().parse().unwrap())])
            }
            Body::Enumerated(items) => {
                let names: Vec<_> = items
                    .iter()
                    .filter_map(|item| match item {
                        EnumItem::Number(number) => Some(number),
                        _ => None,
                    })
                    .collect();
                let mut name = key.to_string();
                let mut scope = module;
                let mut depth = 0;
                while !names.iter().any(|number| number.name == name) {
                    if depth == 128 {
                        return Err("Cyclic or too deep ENUMERATED value reference.".into());
                    }
                    let (m, v) = self.mapping_lookup(&name, scope, Section::Values)?;
                    let assignment = &self.modules[m].values[v];
                    let assigned = self.mapping_type(&assignment.type_, m)?;
                    let (assigned, _) = self.mapping_descriptor(&assigned, m)?;
                    if assigned.body != governor.body {
                        return Err(format!(
                            "'{key}' is not a value of the defining ENUMERATED type."
                        ));
                    }
                    match &assignment.value {
                        Value::Identifier(value) => name = value.clone(),
                        _ => return Err(format!("'{key}' is not an ENUMERATED item.")),
                    }
                    scope = m;
                    depth += 1;
                }
                let number = names.iter().find(|number| number.name == name).unwrap();
                let number = self.mapping_integer(&number.number, governor_module, 0)?;
                // Runtime ENUMERATED values can be either names or numbers.
                Ok(vec![
                    Value::Str(name),
                    Value::Number(number.to_string().parse().unwrap()),
                ])
            }
            Body::Builtin(name) if name == "OBJECT IDENTIFIER" => {
                let arcs = if key.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
                    key.split('.')
                        .map(|arc| {
                            arc.parse::<BigInt>()
                                .map_err(|_| "Invalid OID arc.".to_string())
                        })
                        .collect::<CompileResult<Vec<_>>>()?
                } else {
                    self.mapping_oid(&Value::Identifier(key.into()), module, 0)?
                };
                if arcs.len() < 2
                    || arcs.iter().any(|arc| arc < &BigInt::from(0))
                    || arcs[0] > BigInt::from(2)
                    || (arcs[0] < BigInt::from(2) && arcs[1] >= BigInt::from(40))
                {
                    return Err(format!("'{key}' is not a valid OBJECT IDENTIFIER."));
                }
                Ok(vec![Value::Str(
                    arcs.iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join("."),
                )])
            }
            Body::Builtin(name) if name == "BOOLEAN" => match key {
                "TRUE" => Ok(vec![Value::Bool(true)]),
                "FALSE" => Ok(vec![Value::Bool(false)]),
                _ => Err("A BOOLEAN selector must be TRUE or FALSE.".into()),
            },
            _ => Err(
                "The defining member must be INTEGER, ENUMERATED, OBJECT IDENTIFIER or BOOLEAN."
                    .into(),
            ),
        }
    }

    fn mapping_integer(&self, value: &Value, module: usize, depth: usize) -> CompileResult<BigInt> {
        if depth > 128 {
            return Err("Cyclic or too deep INTEGER value reference.".into());
        }
        match value {
            Value::Number(number) => number
                .to_string()
                .parse()
                .map_err(|_| "Expected an integer selector.".into()),
            Value::Identifier(name) => {
                if let Ok(number) = name.parse::<BigInt>() {
                    return Ok(number);
                }
                let (m, v) = self.mapping_lookup(name, module, Section::Values)?;
                let assignment = &self.modules[m].values[v];
                let ty = self.mapping_type(&assignment.type_, m)?;
                let (ty, scope) = self.mapping_descriptor(&ty, m)?;
                let Body::Integer(numbers) = ty.body else {
                    return Err(format!("'{name}' is not an INTEGER value."));
                };
                if let Value::Identifier(item) = &assignment.value {
                    if let Some(number) =
                        numbers.iter().flatten().find(|number| number.name == *item)
                    {
                        return self.mapping_integer(&number.number, scope, depth + 1);
                    }
                }
                self.mapping_integer(&assignment.value, m, depth + 1)
            }
            _ => Err("Expected an integer selector.".into()),
        }
    }

    fn mapping_oid(
        &self,
        value: &Value,
        module: usize,
        depth: usize,
    ) -> CompileResult<Vec<BigInt>> {
        if depth > 128 {
            return Err("Cyclic or too deep OBJECT IDENTIFIER value reference.".into());
        }
        match value {
            Value::List(items) => {
                let mut arcs = Vec::new();
                for item in items {
                    arcs.extend(self.mapping_oid(item, module, depth + 1)?);
                }
                Ok(arcs)
            }
            Value::NamedValue(_, value) => self.mapping_oid(value, module, depth + 1),
            Value::Number(number) => Ok(vec![
                number
                    .to_string()
                    .parse()
                    .map_err(|_| "Expected an integer OID arc.")?,
            ]),
            Value::Identifier(name) => {
                let root = match name.as_str() {
                    "itu-t" | "ccitt" => Some(0),
                    "iso" => Some(1),
                    "joint-iso-itu-t" | "joint-iso-ccitt" => Some(2),
                    _ => None,
                };
                if let Some(root) = root {
                    return Ok(vec![BigInt::from(root)]);
                }
                let (m, v) = self.mapping_lookup(name, module, Section::Values)?;
                let assignment = &self.modules[m].values[v];
                let ty = self.mapping_type(&assignment.type_, m)?;
                let (ty, _) = self.mapping_descriptor(&ty, m)?;
                if ty.type_name() != "OBJECT IDENTIFIER" && ty.type_name() != "INTEGER" {
                    return Err(format!(
                        "'{name}' is not an OBJECT IDENTIFIER or integer arc."
                    ));
                }
                self.mapping_oid(&assignment.value, m, depth + 1)
            }
            _ => Err("Expected an OBJECT IDENTIFIER value.".into()),
        }
    }
}
