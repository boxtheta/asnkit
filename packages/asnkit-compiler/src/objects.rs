// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! What is done once all of a specification is parsed, as the Python
//! parser does it in `ObjectConverter` and the converter of the syntax
//! tree: objects given with the syntax of their class are read, and the
//! value assignments are told from the assignments of objects.

use crate::ast::*;
use crate::data::{Data, Where, convert_value_text, value_from_data};
use crate::grammar::parse_with_syntax;

/// Types that are not those of an object class, although they have no
/// lower case letters.
const NOT_CLASS_NAMES: [&str; 36] = [
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
    "INTEGER",
    "BIT STRING",
    "ENUMERATED",
    "SEQUENCE",
    "SET",
    "CHOICE",
];

fn is_object_class_name(name: &str) -> bool {
    name == name.to_uppercase()
        && !NOT_CLASS_NAMES.contains(&name)
        && !["SEQUENCE OF", "SET OF", "ANY DEFINED BY"].contains(&name)
}

/// The modules of a specification by name, as a dictionary has them.
pub struct Specification {
    pub modules: Vec<Module>,
}

impl Specification {
    fn find_module(&self, name: &str) -> Option<usize> {
        self.modules.iter().position(|module| module.name == name)
    }

    /// Where an assignment of given kind is found, as seen from a module,
    /// following the imports.
    fn find(&self, mut module: Option<usize>, name: &str, kind: Kind) -> Option<(usize, usize)> {
        let mut visited: Vec<usize> = Vec::new();

        while let Some(index) = module {
            if visited.contains(&index) {
                break;
            }

            visited.push(index);
            let found = &self.modules[index];

            let position = match kind {
                Kind::Type => found.types.iter().position(|t| t.name == name),
                Kind::ObjectClass => found.object_classes.iter().position(|c| c.name == name),
            };

            if let Some(position) = position {
                return Some((index, position));
            }

            module = None;

            for import in &found.imports {
                if import.symbols.iter().any(|symbol| symbol == name) {
                    module = self.find_module(&import.module);
                    break;
                }
            }
        }

        None
    }

    fn find_class(&self, module: usize, name: &str) -> Option<&ObjectClassAssignment> {
        self.find(Some(module), name, Kind::ObjectClass)
            .map(|(m, c)| &self.modules[m].object_classes[c])
    }

    /// The name of the built-in type a type is, following references.
    fn find_builtin_type_name(&self, module: usize, type_name: &str) -> Option<String> {
        let mut visited: Vec<(usize, String)> = Vec::new();
        let mut module = module;
        let mut type_name = type_name.to_string();

        while !visited.contains(&(module, type_name.clone())) {
            visited.push((module, type_name.clone()));

            let Some((m, t)) = self.find(Some(module), &type_name, Kind::Type) else {
                // A built-in type has no lower case letters, with a few
                // exceptions that are not of interest.
                return if type_name == type_name.to_uppercase() {
                    Some(type_name)
                } else {
                    None
                };
            };

            module = m;
            let found = &self.modules[m].types[t].type_;

            match &found.body {
                Body::Referenced(name) | Body::Builtin(name) => type_name = name.clone(),
                other => return Some(Type::new(other.clone()).type_name()),
            }
        }

        None
    }
}

#[derive(Clone, Copy)]
enum Kind {
    Type,
    ObjectClass,
}

/// `convert_object_identifier_value()`: a name and a value in braces are
/// two components of an object identifier, or one member of a sequence.
/// Only the type tells which.
fn convert_object_identifier_value(
    specification: &Specification,
    module: usize,
    data: Data,
    type_name: &str,
) -> Data {
    let Data::Dict(members) = &data else {
        return data;
    };

    if members.len() != 1 {
        return data;
    }

    let (name, component) = &members[0];

    if name.starts_with('&')
        || matches!(
            component,
            Data::Bool(_) | Data::Float(_) | Data::List(_) | Data::Dict(_)
        )
    {
        return data;
    }

    let is_object_identifier = match specification.find_builtin_type_name(module, type_name) {
        None => matches!(component, Data::Int(_)),
        Some(builtin) => builtin == "OBJECT IDENTIFIER" || builtin == "RELATIVE-OID",
    };

    if is_object_identifier && !matches!(component, Data::None) {
        Data::List(vec![Data::Word(name.clone()), component.clone()])
    } else {
        data
    }
}

fn convert_members(specification: &Specification, module: usize, members: &mut [MemberItem]) {
    for item in members {
        match item {
            MemberItem::Member(member) => {
                convert_type(specification, module, &mut member.type_);

                if let Some((data, type_name)) = member.raw_default.take() {
                    let data =
                        convert_object_identifier_value(specification, module, data, &type_name);
                    member.default = Some(value_from_data(
                        &data,
                        Where::Value,
                        Some(&type_name),
                        false,
                    ));
                }
            }
            MemberItem::Group(group) => convert_members(specification, module, group),
            _ => {}
        }
    }
}

fn convert_type(specification: &Specification, module: usize, type_: &mut Type) {
    convert_actual_parameters(specification, module, type_);

    match &mut type_.body {
        Body::Sequence(members) | Body::Set(members) | Body::Choice(members) => {
            convert_members(specification, module, members)
        }
        Body::SequenceOf(element, _) | Body::SetOf(element, _) => {
            convert_type(specification, module, element)
        }
        _ => {}
    }
}

/// The actual parameters of a parameterized type that are objects or
/// object sets of a class are read with the syntax of the class.
fn convert_actual_parameters(specification: &Specification, module: usize, type_: &mut Type) {
    let Body::Referenced(name) = &type_.body else {
        return;
    };

    let name = name.clone();
    let Some(actual_parameters) = type_.common.actual_parameters.as_mut() else {
        return;
    };

    let found = specification.find(Some(module), &name, Kind::Type);
    let governors: Vec<Option<String>> = match found {
        Some((m, t)) => specification.modules[m].types[t]
            .type_
            .common
            .parameters
            .as_ref()
            .map(|parameters| {
                parameters
                    .iter()
                    .map(|parameter| parameter.governor.as_ref().map(Type::type_name))
                    .collect()
            })
            .unwrap_or_default(),
        None => Vec::new(),
    };
    let parameterized_module = found.map(|(m, _)| m).unwrap_or(module);

    for (index, parameter) in actual_parameters.iter_mut().enumerate() {
        if let Value::Type(inner) = parameter {
            convert_type(specification, module, inner);
        }

        let Some(Some(governor)) = governors.get(index) else {
            continue;
        };

        let Some(class) = specification.find_class(parameterized_module, governor) else {
            continue;
        };

        let class_name = class.name.clone();

        match parameter {
            Value::List(items) => {
                let members = items
                    .iter()
                    .map(|item| match item {
                        Value::Identifier(name) => Value::Str(name.clone()),
                        other => other.clone(),
                    })
                    .collect();

                *parameter = Value::ObjectSet(members);
            }
            Value::UnresolvedObject(text) => {
                if let Some(fields) =
                    convert_object(specification, parameterized_module, &class_name, text)
                {
                    *parameter = Value::Object(fields);
                }
            }
            Value::ObjectSet(members) => {
                for member in members.iter_mut() {
                    if let Value::UnresolvedObject(text) = member {
                        if let Some(fields) =
                            convert_object(specification, parameterized_module, &class_name, text)
                        {
                            *member = Value::Object(fields);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}

/// Reads the objects given with the syntax of their class.
fn convert_object(
    specification: &Specification,
    module: usize,
    class: &str,
    text: &str,
) -> Option<Vec<ObjectField>> {
    let class = specification.find_class(module, class)?;
    let syntax = class.syntax.as_ref()?;

    parse_with_syntax(syntax, text)
}

pub fn finish(modules: Vec<Module>) -> Vec<Module> {
    let mut specification = Specification { modules };
    let count = specification.modules.len();

    for index in 0..count {
        // The members of the object sets.
        let sets = specification.modules[index].object_sets.clone();
        let mut converted_sets = Vec::new();

        for mut set in sets {
            for member in set.members.iter_mut() {
                if let Value::UnresolvedObject(text) = member {
                    if let Some(fields) =
                        convert_object(&specification, index, &set.class_name, text)
                    {
                        *member = Value::Object(fields);
                    }
                }
            }

            converted_sets.push(set);
        }

        specification.modules[index].object_sets = converted_sets;

        // The values and the objects.
        let raw_values = std::mem::take(&mut specification.modules[index].raw_values);
        let mut values = Vec::new();
        let mut objects = Vec::new();

        for raw in raw_values {
            let mut data = raw.value;

            if let RawData::DefinedSyntax(text) = &data {
                let is_class_name = is_object_class_name(&raw.type_name)
                    && specification
                        .find(Some(index), &raw.type_name, Kind::Type)
                        .is_none();

                if is_class_name {
                    if let Some(fields) =
                        convert_object(&specification, index, &raw.type_name, text)
                    {
                        data = RawData::Fields(fields);
                    }
                } else {
                    data = RawData::Data(convert_value_text(&format!("{{{text}}}")));
                }
            }

            if let RawData::Data(value) = data {
                data = RawData::Data(convert_object_identifier_value(
                    &specification,
                    index,
                    value,
                    &raw.type_name,
                ));
            }

            let is_object = match &data {
                RawData::Fields(_) | RawData::DefinedSyntax(_) => true,
                RawData::Data(value) => {
                    let identifier_like = match value {
                        Data::None => true,
                        Data::Word(word) => is_value_identifier(word),
                        _ => false,
                    };

                    identifier_like
                        && specification
                            .find(Some(index), &raw.type_name, Kind::ObjectClass)
                            .is_some()
                }
            };

            if is_object {
                let object = match data {
                    RawData::Fields(fields) => Value::Object(fields),
                    RawData::DefinedSyntax(text) => Value::UnresolvedObject(text),
                    RawData::Data(Data::None) => Value::UnsupportedValue,
                    RawData::Data(Data::Word(word)) => Value::Identifier(word),
                    RawData::Data(_) => Value::UnsupportedValue,
                };

                objects.push(ObjectAssignment {
                    name: raw.name,
                    class_name: raw.type_name,
                    object,
                    location: raw.location,
                });
            } else {
                let RawData::Data(value) = data else {
                    continue;
                };

                values.push(ValueAssignment {
                    name: raw.name,
                    value: value_from_data(&value, Where::Value, Some(&raw.type_name), false),
                    type_: raw.type_name,
                    location: raw.location,
                });
            }
        }

        specification.modules[index].values = values;
        specification.modules[index].objects = objects;

        // The default values of members.
        let mut types = specification.modules[index].types.clone();

        for assignment in types.iter_mut() {
            convert_type(&specification, index, &mut assignment.type_);
        }

        specification.modules[index].types = types;
    }

    specification.modules
}

/// `[a-z][a-zA-Z0-9]*(-[a-zA-Z0-9]+)*`
fn is_value_identifier(text: &str) -> bool {
    let mut chars = text.chars();

    if !chars.next().is_some_and(|c| c.is_ascii_lowercase()) {
        return false;
    }

    let mut previous_hyphen = false;

    for c in text.chars().skip(1) {
        if c == '-' {
            if previous_hyphen {
                return false;
            }

            previous_hyphen = true;
        } else if c.is_ascii_alphanumeric() {
            previous_hyphen = false;
        } else {
            return false;
        }
    }

    !previous_hyphen
}
