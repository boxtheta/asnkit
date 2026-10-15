// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Resolves a parsed specification to what is needed to encode and decode
//! its types, as the Python compiler does in `Compiler.pre_process()` and
//! in `resolver.py`, in the same order, so that the first error is the
//! same.
//!
//! In a resolved specification `COMPONENTS OF` is expanded, `EXTENSIBILITY
//! IMPLIED` is applied, every tag has its number and is explicit or
//! implicit, parameterized types are replaced by what they are with their
//! actual parameters, sizes, ranges and numbers are numbers, and a
//! reference to a type of another module tells which module.

use crate::ast::*;
use serde_json::Value as Json;

pub type CompileResult<T> = Result<T, String>;

#[path = "effective.rs"]
mod effective;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Section {
    Types,
    Values,
    ObjectClasses,
}

impl Section {
    fn debug_string(self) -> &'static str {
        match self {
            Section::Types => "type",
            Section::Values => "value",
            Section::ObjectClasses => "object class",
        }
    }
}

fn capitalize(text: &str) -> String {
    let mut chars = text.chars();

    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// The names of the types that are built in, which are not looked up.
fn is_builtin_type_name(name: &str) -> bool {
    const NAMES: [&str; 39] = [
        "ANY",
        "BMPString",
        "BOOLEAN",
        "CHARACTER STRING",
        "DATE",
        "DATE-TIME",
        "DURATION",
        "EMBEDDED PDV",
        "EXTERNAL",
        "GeneralString",
        "GeneralizedTime",
        "GraphicString",
        "IA5String",
        "ISO646String",
        "NULL",
        "NumericString",
        "OBJECT IDENTIFIER",
        "OCTET STRING",
        "ObjectDescriptor",
        "OpenType",
        "PrintableString",
        "REAL",
        "T61String",
        "TIME-OF-DAY",
        "TeletexString",
        "UTCTime",
        "UTF8String",
        "UniversalString",
        "VideotexString",
        "VisibleString",
        "ANY DEFINED BY",
        "BIT STRING",
        "CHOICE",
        "ENUMERATED",
        "INTEGER",
        "SEQUENCE",
        "SEQUENCE OF",
        "SET",
        "SET OF",
    ];

    NAMES.contains(&name)
}

fn is_object_class_type_name(name: &str) -> bool {
    name.contains('&')
}

pub struct Compiler {
    pub modules: Vec<Module>,
}

/// Where an assignment is found: the module and its place among the
/// assignments of its kind.
type Found = (usize, usize);

impl Compiler {
    fn module_index(&self, name: &str) -> Option<usize> {
        self.modules.iter().position(|module| module.name == name)
    }

    fn count(&self, module: usize, section: Section) -> usize {
        match section {
            Section::Types => self.modules[module].types.len(),
            Section::Values => self.modules[module].values.len(),
            Section::ObjectClasses => self.modules[module].object_classes.len(),
        }
    }

    fn position(&self, module: usize, section: Section, name: &str) -> Option<usize> {
        let module = &self.modules[module];

        match section {
            Section::Types => module.types.iter().position(|t| t.name == name),
            Section::Values => module.values.iter().position(|v| v.name == name),
            Section::ObjectClasses => module.object_classes.iter().position(|c| c.name == name),
        }
    }

    /// `lookup_in_modules()`, with its messages.
    fn lookup(&self, section: Section, name: &str, module: usize) -> CompileResult<Found> {
        let _ = self.count(module, section);
        let module_name = &self.modules[module].name;
        let debug_string = section.debug_string();

        if let Some(position) = self.position(module, section, name) {
            return Ok((module, position));
        }

        for import in &self.modules[module].imports {
            if !import.symbols.iter().any(|symbol| symbol == name) {
                continue;
            }

            let Some(from) = self.module_index(&import.module) else {
                return Err(format!(
                    "Module '{}' cannot import {} '{}' from missing module '{}'.",
                    module_name, debug_string, name, import.module
                ));
            };

            return self.lookup(section, name, from).map_err(|_| {
                format!(
                    "{} '{}' imported by module '{}' not found in module '{}'.",
                    capitalize(debug_string),
                    name,
                    module_name,
                    import.module
                )
            });
        }

        Err(format!(
            "{} '{}' not found in module '{}'.",
            capitalize(debug_string),
            name,
            module_name
        ))
    }

    fn lookup_type(&self, name: &str, module: usize) -> CompileResult<Found> {
        self.lookup(Section::Types, name, module)
    }

    fn lookup_value(&self, name: &str, module: usize) -> CompileResult<Value> {
        let (m, v) = self.lookup(Section::Values, name, module)?;

        Ok(self.modules[m].values[v].value.clone())
    }

    fn lookup_object_class(&self, name: &str, module: usize) -> CompileResult<Found> {
        self.lookup(Section::ObjectClasses, name, module)
    }

    /// The type of the field of an object class, `CLASS.&field`.
    fn lookup_object_class_type(
        &self,
        type_name: &str,
        module: usize,
    ) -> CompileResult<Option<(Type, usize)>> {
        let (class_name, member_name) = type_name.split_once('.').unwrap_or((type_name, ""));
        let (m, c) = self.lookup_object_class(class_name, module)?;

        for field in &self.modules[m].object_classes[c].fields {
            if field.name == member_name {
                return Ok(Some((field.type_.clone(), m)));
            }
        }

        Ok(None)
    }

    /// If a type is an untagged CHOICE, an untagged open type or an untagged
    /// ANY, past references that have no tag of their own (X.680 31.2.7 c).
    fn is_untagged_choice_or_open_type(&self, type_: &Type, module: usize) -> bool {
        let mut current = type_.clone();
        let mut module = module;

        for _ in 0..10_000 {
            let type_name = current.type_name();

            if matches!(
                type_name.as_str(),
                "CHOICE" | "OpenType" | "ANY" | "ANY DEFINED BY"
            ) {
                return true;
            }

            let next = if is_object_class_type_name(&type_name) {
                // The tag of the field is not applied.
                match self.lookup_object_class_type(&type_name, module) {
                    Ok(Some((mut field, m))) => {
                        field.common.tag = None;

                        Some((field, m))
                    }
                    _ => None,
                }
            } else {
                self.lookup_type(&type_name, module)
                    .ok()
                    .map(|(m, t)| (self.modules[m].types[t].type_.clone(), m))
            };

            match next {
                Some((next, m)) if next.common.tag.is_none() => {
                    current = next;
                    module = m;
                }
                _ => return false,
            }
        }

        false
    }

    /// The descriptor a type ends in, following references.
    fn resolve_type_descriptor(&self, type_: &Type, module: usize) -> Type {
        self.resolve_type_and_module(type_, module).0
    }

    /// The descriptor a type ends in, following references, and the module
    /// it is found in.
    fn resolve_type_and_module(&self, type_: &Type, module: usize) -> (Type, usize) {
        let mut current = type_.clone();
        let mut module = module;

        loop {
            let type_name = current.type_name();

            if is_object_class_type_name(&type_name) {
                match self.lookup_object_class_type(&type_name, module) {
                    Ok(Some((member_type, m))) => {
                        current = member_type;
                        module = m;
                    }
                    _ => return (current, module),
                }
            } else {
                match self.lookup_type(&type_name, module) {
                    Ok((m, t)) => {
                        current = self.modules[m].types[t].type_.clone();
                        module = m;
                    }
                    Err(_) => return (current, module),
                }
            }
        }
    }
}

/// The members of a type that has them, with the groups kept.
fn members_of(type_: &mut Type) -> Option<&mut Vec<MemberItem>> {
    match &mut type_.body {
        Body::Sequence(members) | Body::Set(members) | Body::Choice(members) => Some(members),
        _ => None,
    }
}

fn members_ref(type_: &Type) -> Option<&Vec<MemberItem>> {
    match &type_.body {
        Body::Sequence(members) | Body::Set(members) | Body::Choice(members) => Some(members),
        _ => None,
    }
}

// ----------------------------------------------------------------------
// COMPONENTS OF

impl Compiler {
    fn expand_members(
        &self,
        members: &[MemberItem],
        module: usize,
    ) -> CompileResult<Vec<MemberItem>> {
        let mut expanded = Vec::new();

        for member in members {
            if let MemberItem::ComponentsOf(name) = member {
                let (m, t) = self.lookup_type(name, module)?;
                let inner = members_ref(&self.modules[m].types[t].type_)
                    .cloned()
                    .unwrap_or_default();
                let inner = self.expand_members(&inner, m)?;

                for item in inner {
                    if matches!(item, MemberItem::ExtensionMarker) {
                        break;
                    }

                    expanded.push(item);
                }
            } else {
                expanded.push(member.clone());
            }
        }

        Ok(expanded)
    }

    fn pre_process_components_of(&mut self, module: usize) -> CompileResult<()> {
        for index in 0..self.modules[module].types.len() {
            let mut type_ = self.modules[module].types[index].type_.clone();

            if let Some(members) = members_ref(&type_).cloned() {
                let expanded = self.expand_members(&members, module)?;
                *members_of(&mut type_).unwrap() = expanded;
                self.modules[module].types[index].type_ = type_;
            }
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------
// EXTENSIBILITY IMPLIED

fn make_extensible(type_: &mut Type) {
    let Some(members) = members_of(type_) else {
        return;
    };

    for member in members.iter_mut() {
        match member {
            MemberItem::ExtensionMarker | MemberItem::ComponentsOf(_) => {}
            MemberItem::Group(group) => {
                for item in group.iter_mut() {
                    if let MemberItem::Member(member) = item {
                        make_extensible(&mut member.type_);
                    }
                }
            }
            MemberItem::Member(member) => make_extensible(&mut member.type_),
        }
    }

    if !members
        .iter()
        .any(|m| matches!(m, MemberItem::ExtensionMarker))
    {
        members.push(MemberItem::ExtensionMarker);
    }
}

impl Compiler {
    fn pre_process_extensibility_implied(&mut self, module: usize) {
        if !self.modules[module].extensibility_implied {
            return;
        }

        for assignment in self.modules[module].types.iter_mut() {
            make_extensible(&mut assignment.type_);
        }
    }
}

// ----------------------------------------------------------------------
// Tags

impl Compiler {
    fn pre_process_tags(&mut self, module: usize) -> CompileResult<()> {
        let module_tags = self.modules[module]
            .tags
            .clone()
            .unwrap_or_else(|| "EXPLICIT".to_string());

        for index in 0..self.modules[module].types.len() {
            let parameters: Vec<String> = self.modules[module].types[index]
                .type_
                .common
                .parameters
                .iter()
                .flatten()
                .map(|p| p.name.clone())
                .collect();
            let mut type_ = self.modules[module].types[index].type_.clone();
            self.pre_process_tags_type(&mut type_, &module_tags, module, &parameters)?;
            self.modules[module].types[index].type_ = type_;
        }

        Ok(())
    }

    fn pre_process_tags_type(
        &self,
        type_: &mut Type,
        module_tags: &str,
        module: usize,
        parameters: &[String],
    ) -> CompileResult<()> {
        let type_name = type_.type_name();

        // A tag of an untagged CHOICE or open type is explicit (X.680 31.2.7
        // c), which is worked out with the type as it is but its tag.
        let untagged = type_.common.tag.is_some() && {
            let mut inner = type_.clone();

            inner.common.tag = None;

            self.is_untagged_choice_or_open_type(&inner, module)
        };

        if let Some(tag) = type_.common.tag.as_mut() {
            if let Value::Identifier(name) = &tag.number {
                // A value by reference.
                let value = self.lookup_value(name, module)?;

                match value {
                    Value::Number(number) if !number.to_string().contains(['.', 'e', 'E']) => {
                        tag.number = Value::Number(number);
                    }
                    _ => {
                        return Err(format!(
                            "Tag number '{}' in module '{}' is not an integer.",
                            name, self.modules[module].name
                        ));
                    }
                }
            }

            if untagged && tag.mode.as_deref() == Some("IMPLICIT") {
                return Err(format!(
                    "IMPLICIT can not be used on a CHOICE, an open type or an ANY \
                     with no tag of its own, in module '{}'.",
                    self.modules[module].name
                ));
            }

            if tag.mode.is_none() {
                tag.mode = Some(if untagged || parameters.contains(&type_name) {
                    "EXPLICIT".to_string()
                } else if module_tags == "IMPLICIT" || module_tags == "EXPLICIT" {
                    module_tags.to_string()
                } else {
                    "IMPLICIT".to_string()
                });
            }
        }

        // SEQUENCE, SET and CHOICE.
        if let Some(members) = members_of(type_) {
            self.pre_process_tags_members(members, module_tags, module, parameters)?;
        }

        // SEQUENCE OF and SET OF.
        match &mut type_.body {
            Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
                self.pre_process_tags_type(element, module_tags, module, parameters)?;
            }
            _ => {}
        }

        Ok(())
    }

    fn pre_process_tags_members(
        &self,
        members: &mut [MemberItem],
        module_tags: &str,
        module: usize,
        parameters: &[String],
    ) -> CompileResult<()> {
        // Groups are flattened, one level.
        let mut flat: Vec<&mut Member> = Vec::new();

        for item in members.iter_mut() {
            match item {
                MemberItem::Member(member) => flat.push(member),
                MemberItem::Group(group) => {
                    for inner in group.iter_mut() {
                        if let MemberItem::Member(member) = inner {
                            flat.push(member);
                        }
                    }
                }
                _ => {}
            }
        }

        // Add tag number to all members if AUTOMATIC TAGS are selected
        // and no member is tagged.
        let mut number: Option<i64> = None;

        if module_tags == "AUTOMATIC" && !flat.iter().any(|m| m.type_.common.tag.is_some()) {
            number = Some(0);
        }

        for member in flat {
            if let Some(n) = number {
                let tag = member.type_.common.tag.get_or_insert(Tag {
                    number: Value::int(n),
                    tag_class: None,
                    mode: None,
                });
                tag.number = Value::int(n);
                number = Some(n + 1);
            }

            self.pre_process_tags_type(&mut member.type_, module_tags, module, parameters)?;
        }

        Ok(())
    }
}

// ----------------------------------------------------------------------
// Default values

fn lowest_set_bit(value: u128) -> u32 {
    if value == 0 {
        0
    } else {
        value.trailing_zeros()
    }
}

/// The bits of a default value of a `BIT STRING`, as the number of bits
/// and the bytes they are in, with the first bit first.
fn bit_string_default(default: &Value, named_bits: &[NamedNumber]) -> Option<(Vec<u8>, u32)> {
    let (mask, number_of_bits): (u128, u32) = match default {
        Value::List(names) => {
            let mut reversed_mask: u128 = 0;

            for name in names {
                let Value::Identifier(name) = name else {
                    return None;
                };
                let bit = named_bits.iter().find(|n| &n.name == name)?;
                let Value::Number(bit) = &bit.number else {
                    return None;
                };
                let bit: u32 = bit.to_string().parse().ok()?;

                if bit >= 127 {
                    return None;
                }

                reversed_mask |= 1u128 << bit;
            }

            let number_of_bits = 128 - reversed_mask.leading_zeros();
            let mut mask: u128 = 0;

            // The bits in the reverse order.
            for position in 0..number_of_bits {
                if reversed_mask & (1u128 << position) != 0 {
                    mask |= 1u128 << (number_of_bits - 1 - position);
                }
            }

            (mask, number_of_bits)
        }
        Value::HexString(digits) => {
            let mut digits = digits.clone();

            // Python's length includes the 0x.
            if (digits.len() + 2) % 2 == 1 {
                digits.push('0');
            }

            if digits.len() > 30 {
                return None;
            }

            let mut mask = u128::from_str_radix(&format!("01{digits}"), 16).ok()?;
            mask >>= lowest_set_bit(mask);
            let number_of_bits = (128 - mask.leading_zeros()) - 1;
            mask ^= 1u128 << number_of_bits;

            (mask, number_of_bits)
        }
        Value::BinaryString(bits) if bits.is_empty() => (0, 0),
        Value::BinaryString(bits) => {
            if bits.len() > 120 {
                return None;
            }

            let mut mask = u128::from_str_radix(bits, 2).ok()?;
            mask >>= lowest_set_bit(mask);

            (mask, bits.len() as u32)
        }
        _ => return None,
    };

    if number_of_bits == 0 {
        return Some((Vec::new(), 0));
    }

    let padding = (8 - number_of_bits % 8) % 8;
    let shifted = mask << padding;
    let number_of_bytes = number_of_bits.div_ceil(8) as usize;
    let bytes = shifted.to_be_bytes();

    Some((bytes[16 - number_of_bytes..].to_vec(), number_of_bits))
}

fn binary_string_node(bytes: &[u8], number_of_bits: u32) -> Value {
    let bits: String = bytes.iter().map(|byte| format!("{byte:08b}")).collect();

    Value::BinaryString(bits[..number_of_bits as usize].to_string())
}

fn octet_string_default(default: &Value) -> Option<Value> {
    let bytes: Vec<u8> = match default {
        Value::BinaryString(bits) => {
            let mut bits = bits.clone();

            while bits.len() % 8 != 0 {
                bits.push('0');
            }

            bits.as_bytes()
                .chunks(8)
                .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 2).unwrap())
                .collect()
        }
        Value::HexString(digits) => {
            let mut digits = digits.clone();

            if digits.len() % 2 == 1 {
                digits.push('0');
            }

            digits
                .as_bytes()
                .chunks(2)
                .map(|chunk| u8::from_str_radix(std::str::from_utf8(chunk).unwrap(), 16).unwrap())
                .collect()
        }
        _ => return None,
    };

    Some(Value::HexString(
        bytes.iter().map(|b| format!("{b:02x}")).collect(),
    ))
}

impl Compiler {
    fn pre_process_defaults_type(&self, type_: &mut Type, module: usize) {
        let is_sequence_or_set = matches!(type_.body, Body::Sequence(_) | Body::Set(_));

        if is_sequence_or_set {
            if let Some(members) = members_of(type_) {
                for item in members.iter_mut() {
                    if let MemberItem::Member(member) = item {
                        self.pre_process_default(member, module);
                    }
                }
            }
        }

        if let Some(members) = members_of(type_) {
            for item in members.iter_mut() {
                match item {
                    MemberItem::Member(member) => {
                        self.pre_process_defaults_type(&mut member.type_, module)
                    }
                    MemberItem::Group(group) => {
                        for inner in group.iter_mut() {
                            if let MemberItem::Member(member) = inner {
                                self.pre_process_defaults_type(&mut member.type_, module);
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        match &mut type_.body {
            Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
                self.pre_process_defaults_type(element, module)
            }
            _ => {}
        }
    }

    fn pre_process_default(&self, member: &mut Member, module: usize) {
        let Some(default) = member.default.clone() else {
            return;
        };

        let resolved = self.resolve_type_descriptor(&member.type_, module);

        // A default given as a named number, or as a reference to a
        // number, a boolean or a string, is what that is.
        if let Value::Identifier(name) = &default {
            if !matches!(resolved.body, Body::Enumerated(_)) {
                let mut replaced = default.clone();

                if let Body::Integer(Some(numbers)) = &resolved.body {
                    if let Some(number) = numbers.iter().find(|n| &n.name == name) {
                        replaced = number.number.clone();
                    }
                }

                let found = if let Value::Identifier(reference) = &replaced {
                    self.lookup_value(reference, module).ok()
                } else {
                    Some(replaced)
                };

                if let Some(value) = found {
                    if matches!(value, Value::Bool(_) | Value::Number(_) | Value::Str(_)) {
                        member.default = Some(value);
                    }
                }
            }
        }

        let default = member.default.clone().unwrap();

        match &resolved.body {
            Body::BitString(named_bits) => {
                if let Some((bytes, number_of_bits)) =
                    bit_string_default(&default, named_bits.as_deref().unwrap_or(&[]))
                {
                    member.default = Some(binary_string_node(&bytes, number_of_bits));
                }
            }
            Body::Builtin(name) if name == "OCTET STRING" => {
                if let Some(value) = octet_string_default(&default) {
                    member.default = Some(value);
                }
            }
            _ => {}
        }

        member.raw_default = None;
    }

    fn pre_process_defaults(&mut self, module: usize) {
        for index in 0..self.modules[module].types.len() {
            let mut type_ = self.modules[module].types[index].type_.clone();
            self.pre_process_defaults_type(&mut type_, module);
            self.modules[module].types[index].type_ = type_;
        }
    }
}

// ----------------------------------------------------------------------
// Parameterization

/// `type_descriptor.update(actual)`: what the actual type has replaces
/// what the type has.
fn update(type_: &mut Type, actual: &Type) {
    type_.body = actual.body.clone();
    let from = &actual.common;
    let to = &mut type_.common;

    if from.tag.is_some() {
        to.tag = from.tag.clone();
    }
    if from.size.is_some() {
        to.size = from.size.clone();
    }
    if from.restricted_to.is_some() {
        to.restricted_to = from.restricted_to.clone();
    }
    if from.permitted_alphabet.is_some() {
        to.permitted_alphabet = from.permitted_alphabet.clone();
    }
    if from.with_components.is_some() {
        to.with_components = from.with_components.clone();
    }
    if from.with_component.is_some() {
        to.with_component = from.with_component.clone();
    }
    if from.table.is_some() {
        to.table = from.table.clone();
    }
    if from.parameters.is_some() {
        to.parameters = from.parameters.clone();
    }
    if from.actual_parameters.is_some() {
        to.actual_parameters = from.actual_parameters.clone();
    }
    if from.module_name.is_some() {
        to.module_name = from.module_name.clone();
    }

    // The constraints of what takes the place of the type, and then those
    // of the type itself, applied one after the other.
    if let Some(constraints) = &from.constraints {
        let mut all = constraints.clone();

        all.extend(to.constraints.take().unwrap_or_default());
        to.constraints = Some(all);
    }
}

fn dummy_to_actual_parameter(parameter: &Value, dummy: &str, actual: &Value) -> Value {
    match parameter {
        Value::Type(type_) => {
            if type_.type_name() == dummy {
                return actual.clone();
            }
        }
        Value::ObjectSet(members) => {
            let is_dummy = |m: &Value| matches!(m, Value::Str(name) if name == dummy);

            if members.iter().any(is_dummy) {
                let mut converted = Vec::new();

                for member in members {
                    if !is_dummy(member) {
                        converted.push(member.clone());
                    } else if let Value::ObjectSet(inner) = actual {
                        converted.extend(inner.iter().cloned());
                    } else {
                        converted.push(actual.clone());
                    }
                }

                return Value::ObjectSet(converted);
            }
        }
        Value::Object(_) | Value::UnresolvedObject(_) => {}
        value => {
            if matches!(value, Value::Identifier(name) if name == dummy) {
                return actual.clone();
            }
        }
    }

    parameter.clone()
}

fn replace_in_constraint(items: &mut [Value], dummy: &str, actual: &Value) {
    let replace = |value: &Value| -> Value {
        if matches!(value, Value::Identifier(name) if name == dummy) {
            actual.clone()
        } else {
            value.clone()
        }
    };

    for item in items.iter_mut() {
        match item {
            Value::Range(lower, upper) => {
                // Excluded ends keep the way they are excluded.
                let convert = |end: &Value| match end {
                    Value::Excluded(inner) => Value::Excluded(Box::new(replace(inner))),
                    other => replace(other),
                };
                *item = Value::Range(Box::new(convert(lower)), Box::new(convert(upper)));
            }
            other => *other = replace(other),
        }
    }
}

fn dummy_to_actual_type(
    type_: &mut Type,
    dummy_parameters: &[String],
    actual_parameters: &[Value],
) {
    if let Some(members) = members_of(type_) {
        for item in members.iter_mut() {
            if let MemberItem::Member(member) = item {
                dummy_to_actual_type(&mut member.type_, dummy_parameters, actual_parameters);
            }
        }
    } else if let Body::SequenceOf(element, _) | Body::SetOf(element, _) = &mut type_.body {
        dummy_to_actual_type(element, dummy_parameters, actual_parameters);
    }

    for (dummy, actual) in dummy_parameters.iter().zip(actual_parameters) {
        if type_.type_name() == *dummy {
            if let Value::Type(actual_type) = actual {
                update(type_, actual_type);
            }
        }

        if let Some(parameters) = type_.common.actual_parameters.as_mut() {
            for parameter in parameters.iter_mut() {
                *parameter = dummy_to_actual_parameter(parameter, dummy, actual);
            }
        }

        if let Some(size) = type_.common.size.as_mut() {
            replace_in_constraint(size, dummy, actual);
        }

        if let Some(restricted_to) = type_.common.restricted_to.as_mut() {
            replace_in_constraint(restricted_to, dummy, actual);
        }

        if let Some(constraints) = type_.common.constraints.as_mut() {
            for constraint in constraints.iter_mut() {
                replace_in_tree(constraint, dummy, actual);
            }
        }
    }
}

/// Puts the actual parameter in the place of a dummy one in a constraint
/// as it is written: a value as a value, and a type as the type of a
/// contained subtype.
fn replace_in_tree(constraint: &mut Constraint, dummy: &str, actual: &Value) {
    fn replace_value(value: &mut Value, dummy: &str, actual: &Value) {
        match value {
            Value::Identifier(name) if name == dummy => *value = actual.clone(),
            Value::Range(lower, upper) => {
                replace_value(lower, dummy, actual);
                replace_value(upper, dummy, actual);
            }
            Value::Excluded(inner) => replace_value(inner, dummy, actual),
            _ => {}
        }
    }

    fn replace_set(set: &mut ElementSet, dummy: &str, actual: &Value) {
        match set {
            ElementSet::Union(sets) | ElementSet::Intersection(sets) => {
                for inner in sets.iter_mut() {
                    replace_set(inner, dummy, actual);
                }
            }
            ElementSet::Except(inner, exclusions) => {
                replace_set(inner, dummy, actual);
                replace_set(exclusions, dummy, actual);
            }
            ElementSet::All(exclusions) => replace_set(exclusions, dummy, actual),
            ElementSet::Element(element) => match element {
                Element::Value(inner) | Element::Pattern(inner) => {
                    replace_value(inner, dummy, actual)
                }
                Element::ContainedSubtype { type_, .. } => {
                    if type_.type_name() == dummy
                        && let Value::Type(actual_type) = actual
                    {
                        update(type_, actual_type);
                    }

                    if let Some(constraints) = type_.common.constraints.as_mut() {
                        for constraint in constraints.iter_mut() {
                            replace_in_tree(constraint, dummy, actual);
                        }
                    }
                }
                Element::Size(constraint)
                | Element::PermittedAlphabet(constraint)
                | Element::InnerType(constraint) => replace_in_tree(constraint, dummy, actual),
                Element::InnerTypes { components, .. } => {
                    for component in components.iter_mut() {
                        for constraint in component.constraints.iter_mut().flatten() {
                            replace_in_tree(constraint, dummy, actual);
                        }
                    }
                }
            },
        }
    }

    if let Constraint::ElementSetSpecs {
        root, additions, ..
    } = constraint
    {
        replace_set(root, dummy, actual);

        if let Some(additions) = additions {
            replace_set(additions, dummy, actual);
        }
    }
}

impl Compiler {
    fn parameterization_step_1_type(
        &self,
        type_: &mut Type,
        type_name: &str,
        module: usize,
        depth: usize,
    ) -> CompileResult<()> {
        // A parameterized type that uses itself never gets to what it is.
        // The Python compiler runs out of stack on it.
        if depth > 100 {
            return Err(format!(
                "Type '{}' in module '{}' is parameterized recursively.",
                type_name, self.modules[module].name
            ));
        }

        // SEQUENCE, SET and CHOICE. A group of members is left as it is.
        if let Some(members) = members_of(type_) {
            for item in members.iter_mut() {
                if let MemberItem::Member(member) = item {
                    self.parameterization_step_1_type(&mut member.type_, type_name, module, depth)?;
                }
            }
        }

        // SEQUENCE OF and SET OF.
        if let Body::SequenceOf(element, _) | Body::SetOf(element, _) = &mut type_.body {
            self.parameterization_step_1_type(element, type_name, module, depth)?;
        }

        // Just return if the type is not using a parameterized type.
        let Some(actual_parameters) = type_.common.actual_parameters.clone() else {
            return Ok(());
        };

        let used_name = type_.type_name();
        let (parameterized_module, index) = self.lookup_type(&used_name, module)?;
        let parameterized = &self.modules[parameterized_module].types[index].type_;
        let parameterized_module_name = self.modules[parameterized_module].name.clone();

        let Some(parameters) = &parameterized.common.parameters else {
            return Err(format!(
                "Type '{}' in module '{}' is not parameterized.",
                used_name, parameterized_module_name
            ));
        };

        let dummy_parameters: Vec<String> = parameters.iter().map(|p| p.name.clone()).collect();

        if dummy_parameters.len() != actual_parameters.len() {
            return Err(format!(
                "Parameterized type '{}' in module '{}' takes {} parameters, but {} are given in type '{}' in module '{}'.",
                used_name,
                parameterized_module_name,
                dummy_parameters.len(),
                actual_parameters.len(),
                type_name,
                self.modules[module].name
            ));
        }

        let mut copy = parameterized.clone();
        dummy_to_actual_type(&mut copy, &dummy_parameters, &actual_parameters);
        self.parameterization_step_1_type(&mut copy, type_name, parameterized_module, depth + 1)?;
        update(type_, &copy);

        if type_.common.module_name.is_none() && module != parameterized_module {
            type_.common.module_name = Some(parameterized_module_name);
        }

        type_.common.parameters = None;
        type_.common.actual_parameters = None;

        Ok(())
    }

    fn parameterization_step_1(&mut self, module: usize) -> CompileResult<()> {
        for index in 0..self.modules[module].types.len() {
            // Skip if the type is parameterized.
            if self.modules[module].types[index]
                .type_
                .common
                .parameters
                .is_some()
            {
                continue;
            }

            let name = self.modules[module].types[index].name.clone();
            let mut type_ = self.modules[module].types[index].type_.clone();
            self.parameterization_step_1_type(&mut type_, &name, module, 0)?;
            self.modules[module].types[index].type_ = type_;
        }

        Ok(())
    }

    /// Removes the parameterized types, as they are no longer needed.
    fn parameterization_step_2(&mut self, module: usize) {
        self.modules[module]
            .types
            .retain(|t| t.type_.common.parameters.is_none());
    }
}

// ----------------------------------------------------------------------
// The resolver

impl Compiler {
    fn resolve_number(
        &self,
        value: &Value,
        module: usize,
        is_strict: bool,
    ) -> CompileResult<Value> {
        let Value::Identifier(name) = value else {
            return Ok(value.clone());
        };

        match self.lookup_value(name, module) {
            Ok(found) => Ok(found),
            Err(error) if is_strict => Err(error),
            Err(_) => Ok(value.clone()),
        }
    }

    fn resolve_constraint(
        &self,
        items: &[Value],
        resolve: &dyn Fn(&Value, bool) -> CompileResult<Value>,
    ) -> CompileResult<Vec<Value>> {
        let mut resolved = Vec::new();

        for (index, item) in items.iter().enumerate() {
            let is_strict = index == 0;
            let item = match item {
                Value::Range(lower, upper) => {
                    let end = |end: &Value| -> CompileResult<Value> {
                        match end {
                            Value::Excluded(inner) => {
                                Ok(Value::Excluded(Box::new(resolve(inner, is_strict)?)))
                            }
                            other => resolve(other, is_strict),
                        }
                    };

                    Value::Range(Box::new(end(lower)?), Box::new(end(upper)?))
                }
                Value::Type(_) => item.clone(),
                other => resolve(other, is_strict)?,
            };

            resolved.push(item);
        }

        Ok(resolved)
    }

    fn resolve_module_name(&self, type_: &Type, module: usize) -> CompileResult<usize> {
        let mut module = match &type_.common.module_name {
            Some(name) => self.module_index(name).unwrap_or(module),
            None => module,
        };
        let type_name = type_.type_name();

        if is_object_class_type_name(&type_name) {
            // A field of an object class. Not all codecs need it to be
            // found.
            let class_name = type_name.split('.').next().unwrap_or("");

            if let Ok((m, _)) = self.lookup_object_class(class_name, module) {
                module = m;
            }
        } else if !is_builtin_type_name(&type_name) {
            let (m, _) = self.lookup_type(&type_name, module)?;
            module = m;
        }

        Ok(module)
    }

    fn resolve_type(&self, type_: &mut Type, module: usize) -> CompileResult<()> {
        let type_module = self.resolve_module_name(type_, module)?;

        if type_module != module {
            type_.common.module_name = Some(self.modules[type_module].name.clone());
        }

        let module = type_module;

        if let Some(size) = type_.common.size.clone() {
            type_.common.size = Some(self.resolve_constraint(&size, &|value, strict| {
                self.resolve_number(value, module, strict)
            })?);
        }

        if type_.common.restricted_to.is_some() {
            self.resolve_restricted_to(type_, module)?;
        }

        match &mut type_.body {
            Body::Integer(Some(numbers)) => {
                for number in numbers.iter_mut() {
                    number.number = self.resolve_number(&number.number, module, false)?;
                }
            }
            Body::BitString(Some(bits)) => {
                for bit in bits.iter_mut() {
                    // The number of a bit is a string in the dictionary,
                    // and a number again as a node.
                    bit.number = self.resolve_number(&bit.number, module, true)?;
                }
            }
            Body::Enumerated(values) => {
                for item in values.iter_mut() {
                    if let EnumItem::Number(number) = item {
                        number.number = self.resolve_number(&number.number, module, true)?;
                    }
                }
            }
            _ => {}
        }

        if let Some(members) = members_of(type_) {
            for item in members.iter_mut() {
                match item {
                    MemberItem::Member(member) => self.resolve_type(&mut member.type_, module)?,
                    MemberItem::Group(group) => {
                        for inner in group.iter_mut() {
                            if let MemberItem::Member(member) = inner {
                                self.resolve_type(&mut member.type_, module)?;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        if let Body::SequenceOf(element, _) | Body::SetOf(element, _) = &mut type_.body {
            self.resolve_type(element, module)?;
        }

        Ok(())
    }

    fn resolve_restricted_to(&self, type_: &mut Type, module: usize) -> CompileResult<()> {
        let named_numbers: Vec<NamedNumber> = match self.resolve_type_descriptor(type_, module).body
        {
            Body::Integer(Some(numbers)) => numbers,
            _ => Vec::new(),
        };

        let resolve = |value: &Value, is_strict: bool| -> CompileResult<Value> {
            let Value::Identifier(name) = value else {
                return Ok(value.clone());
            };

            // A named number may be a reference to a value itself.
            let value = match named_numbers.iter().find(|n| &n.name == name) {
                Some(number) => number.number.clone(),
                None => value.clone(),
            };

            self.resolve_number(&value, module, is_strict)
        };

        let restricted_to = type_.common.restricted_to.clone().unwrap();
        type_.common.restricted_to = Some(self.resolve_constraint(&restricted_to, &resolve)?);

        Ok(())
    }

    fn resolve_all(&mut self) -> CompileResult<()> {
        for module in 0..self.modules.len() {
            for index in 0..self.modules[module].types.len() {
                let mut type_ = self.modules[module].types[index].type_.clone();
                self.resolve_type(&mut type_, module)?;
                self.modules[module].types[index].type_ = type_;
            }

            for index in 0..self.modules[module].object_classes.len() {
                // The syntax is only needed to tell how its objects are
                // given.
                self.modules[module].object_classes[index].syntax = None;

                for field in 0..self.modules[module].object_classes[index].fields.len() {
                    let mut type_ = self.modules[module].object_classes[index].fields[field]
                        .type_
                        .clone();
                    self.resolve_type(&mut type_, module)?;
                    self.modules[module].object_classes[index].fields[field].type_ = type_;
                }
            }
        }

        // An object class is looked up by name in the module it is used in
        // when compiling, so each module gets the ones it imports.
        for module in 0..self.modules.len() {
            let names: Vec<String> = self.modules[module]
                .imports
                .iter()
                .flat_map(|import| import.symbols.iter())
                .filter(|name| {
                    **name == name.to_uppercase()
                        && !self.modules[module]
                            .object_classes
                            .iter()
                            .any(|c| &c.name == *name)
                })
                .cloned()
                .collect();

            for name in names {
                let Ok((class_module, class)) = self.lookup_object_class(&name, module) else {
                    continue;
                };

                let mut copy = self.modules[class_module].object_classes[class].clone();
                let class_module_name = self.modules[class_module].name.clone();

                for field in copy.fields.iter_mut() {
                    if field.type_.common.module_name.is_none() {
                        field.type_.common.module_name = Some(class_module_name.clone());
                    }
                }

                match self.modules[module]
                    .object_classes
                    .iter_mut()
                    .find(|c| c.name == name)
                {
                    Some(existing) => *existing = copy,
                    None => self.modules[module].object_classes.push(copy),
                }
            }
        }

        self.write_all_effective()?;

        for module in self.modules.iter_mut() {
            module.values.clear();
            module.objects.clear();
            module.object_sets.clear();
            module.imports.clear();
        }

        Ok(())
    }

    /// Writes the effective constraints of all the types, while the values
    /// and the imports they may refer to are still there. They are worked
    /// out of the types as they are written, which this does not change.
    fn write_all_effective(&mut self) -> CompileResult<()> {
        let mut memo = effective::Memo::new();
        let mut written: Vec<(usize, usize, Type)> = Vec::new();
        let mut fields: Vec<(usize, usize, usize, Type)> = Vec::new();

        for (m, module) in self.modules.iter().enumerate() {
            for (t, assignment) in module.types.iter().enumerate() {
                let mut type_ = assignment.type_.clone();

                self.write_effective(&mut type_, m, &mut memo)?;
                written.push((m, t, type_));
            }

            for (c, class) in module.object_classes.iter().enumerate() {
                for (f, field) in class.fields.iter().enumerate() {
                    let mut type_ = field.type_.clone();

                    self.write_effective(&mut type_, m, &mut memo)?;
                    fields.push((m, c, f, type_));
                }
            }
        }

        for (m, t, type_) in written {
            self.modules[m].types[t].type_ = type_;
        }

        for (m, c, f, type_) in fields {
            self.modules[m].object_classes[c].fields[f].type_ = type_;
        }

        Ok(())
    }

    /// `pre_process()` and then the resolver.
    pub fn resolve(&mut self) -> CompileResult<()> {
        for module in 0..self.modules.len() {
            self.pre_process_components_of(module)?;
            self.pre_process_extensibility_implied(module);
            self.pre_process_tags(module)?;
            self.pre_process_defaults(module);
        }

        for module in 0..self.modules.len() {
            self.parameterization_step_1(module)?;
        }

        for module in 0..self.modules.len() {
            self.parameterization_step_2(module);
        }

        self.resolve_all()
    }
}

// ----------------------------------------------------------------------
// The compiled specification

fn strip_locations(type_: &mut Type) {
    if let Some(members) = members_of(type_) {
        for item in members.iter_mut() {
            match item {
                MemberItem::Member(member) => {
                    member.location = None;
                    strip_locations(&mut member.type_);
                }
                MemberItem::Group(group) => {
                    for inner in group.iter_mut() {
                        if let MemberItem::Member(member) = inner {
                            member.location = None;
                            strip_locations(&mut member.type_);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    if let Body::SequenceOf(element, _) | Body::SetOf(element, _) = &mut type_.body {
        strip_locations(element);
    }
}

/// The compiled specification, as a node in the readable form.
pub fn compiled_specification(mut modules: Vec<Module>) -> Json {
    for module in modules.iter_mut() {
        for assignment in module.types.iter_mut() {
            assignment.location = None;
            strip_locations(&mut assignment.type_);
        }

        for class in module.object_classes.iter_mut() {
            class.location = None;

            for field in class.fields.iter_mut() {
                field.location = None;
                strip_locations(&mut field.type_);
            }
        }
    }

    // What the compiler made of the constraints, and not how they are
    // written.
    let tree = with_form(Form::Compiled, || SyntaxTree { modules }.to_tree());
    let mut compiled = Vec::new();

    for module in tree["modules"].as_array().unwrap() {
        let mut node = serde_json::Map::new();
        node.insert("kind".into(), Json::String("CompiledModule".into()));
        node.insert("name".into(), module["name"].clone());
        node.insert("types".into(), module["types"].clone());
        node.insert("object_classes".into(), module["object_classes"].clone());
        compiled.push(Json::Object(node));
    }

    let mut specification = serde_json::Map::new();
    specification.insert("kind".into(), Json::String("CompiledSpecification".into()));
    specification.insert("modules".into(), Json::Array(compiled));

    Json::Object(specification)
}
