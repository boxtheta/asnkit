// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Pragmas: single line comments `--@ key = value`, which tell a plugin
//! something of the module, the assignment, the member or the field of
//! an object class they come before (see `docs/file_formats.md`). They
//! are in the syntax tree, as pairs of a key and a value, and not in a
//! compiled specification.

use std::collections::BTreeMap;

use crate::ast::{Body, Location, MemberItem, Pragmas, SyntaxTree, Type};
use crate::source::{Locator, PragmaComment};

/// What is wrong with a pragma, and the offset of its `--`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PragmaError {
    pub offset: usize,
    pub message: String,
}

/// A pragma that is yet to be given to what it comes before.
struct Pending {
    offset: usize,
    key: String,
    value: String,
}

/// Whether `key` is a letter, followed by letters, digits, `.`, `-` and
/// `_`, as `rpc.method` and `go-package`.
fn is_key(key: &str) -> bool {
    let mut chars = key.chars();

    chars.next().is_some_and(|c| c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
}

/// The key and the value of a pragma, of the text after its `--@`: the
/// key and the value are around the first `=` and trimmed, and a pragma
/// with no `=` has the value `""`.
fn key_and_value(text: &str) -> Result<(String, String), String> {
    let (key, value) = match text.split_once('=') {
        Some((key, value)) => (key.trim(), value.trim()),
        None => (text.trim(), ""),
    };

    if key.is_empty() {
        return Err("A pragma has a key, as in '--@ key = value'".to_string());
    }

    if !is_key(key) {
        return Err(format!(
            "The key '{key}' of a pragma is not a letter followed by letters, digits, '.', '-' and '_'"
        ));
    }

    Ok((key.to_string(), value.to_string()))
}

/// Gives each of `comments` to the module, assignment, member or field of
/// an object class of `tree` that it comes before, with nothing but
/// whitespace and other comments between them, in `stripped`, the text
/// of the specifications without comments.
pub fn attach(
    tree: &mut SyntaxTree,
    comments: &[PragmaComment],
    stripped: &str,
    locator: &Locator,
) -> Result<(), PragmaError> {
    if comments.is_empty() {
        return Ok(());
    }

    let mut pending: BTreeMap<Location, Vec<Pending>> = BTreeMap::new();

    for comment in comments {
        let error = |message: String| PragmaError {
            offset: comment.offset,
            message,
        };
        let line_start = stripped[..comment.offset]
            .rfind('\n')
            .map_or(0, |index| index + 1);

        // A pragma after something on its line would otherwise be of what
        // comes on the next line, which is not what it looks like.
        if !stripped[line_start..comment.offset].trim().is_empty() {
            return Err(error(
                "A pragma is on a line of its own, before what it is of".to_string(),
            ));
        }

        let (key, value) = key_and_value(&comment.text).map_err(error)?;

        // The comment is spaces in the text without comments.
        let Some(target) = stripped[comment.offset..]
            .char_indices()
            .find(|(_, c)| !c.is_whitespace())
            .map(|(index, _)| comment.offset + index)
        else {
            return Err(error(nothing_after()));
        };

        pending
            .entry(locator.locate_offset(target))
            .or_default()
            .push(Pending {
                offset: comment.offset,
                key,
                value,
            });
    }

    let mut walk = Walk { pending };

    for module in tree.modules.iter_mut() {
        walk.give(&module.location, &mut module.pragmas, &module.name)?;

        for assignment in module.types.iter_mut() {
            walk.give(
                &assignment.location,
                &mut assignment.pragmas,
                &assignment.name,
            )?;
            walk.type_(&mut assignment.type_)?;
        }

        for assignment in module.values.iter_mut() {
            walk.give(
                &assignment.location,
                &mut assignment.pragmas,
                &assignment.name,
            )?;
        }

        for assignment in module.objects.iter_mut() {
            walk.give(
                &assignment.location,
                &mut assignment.pragmas,
                &assignment.name,
            )?;
        }

        for class in module.object_classes.iter_mut() {
            walk.give(&class.location, &mut class.pragmas, &class.name)?;

            for field in class.fields.iter_mut() {
                walk.give(&field.location, &mut field.pragmas, &field.name)?;
                walk.type_(&mut field.type_)?;
            }
        }

        for set in module.object_sets.iter_mut() {
            walk.give(&set.location, &mut set.pragmas, &set.name)?;
        }
    }

    // What is left comes before something that has no pragmas, as the
    // END of a module or an item of an ENUMERATED.
    match walk
        .pending
        .into_values()
        .flatten()
        .min_by_key(|p| p.offset)
    {
        Some(pragma) => Err(PragmaError {
            offset: pragma.offset,
            message: nothing_after(),
        }),
        None => Ok(()),
    }
}

fn nothing_after() -> String {
    "A pragma comes on the lines before a module, an assignment, a member or a field of an \
     object class, and this one does not"
        .to_string()
}

struct Walk {
    pending: BTreeMap<Location, Vec<Pending>>,
}

impl Walk {
    /// Gives what has the location `location`, of name `name`, the
    /// pragmas that come before it.
    fn give(
        &mut self,
        location: &Option<Location>,
        pragmas: &mut Pragmas,
        name: &str,
    ) -> Result<(), PragmaError> {
        let Some(found) = location.and_then(|location| self.pending.remove(&location)) else {
            return Ok(());
        };

        for pragma in found {
            if pragmas.iter().any(|(key, _)| *key == pragma.key) {
                return Err(PragmaError {
                    offset: pragma.offset,
                    message: format!("The pragma '{}' is given twice to '{name}'", pragma.key),
                });
            }

            pragmas.push((pragma.key, pragma.value));
        }

        Ok(())
    }

    /// Gives the members of a type, and those of the types in it, their
    /// pragmas.
    fn type_(&mut self, type_: &mut Type) -> Result<(), PragmaError> {
        match &mut type_.body {
            Body::Sequence(members) | Body::Set(members) | Body::Choice(members) => {
                self.members(members)
            }
            Body::SequenceOf(element, _) | Body::SetOf(element, _) => self.type_(element),
            _ => Ok(()),
        }
    }

    fn members(&mut self, members: &mut [MemberItem]) -> Result<(), PragmaError> {
        for item in members.iter_mut() {
            match item {
                MemberItem::Member(member) => {
                    self.give(&member.location, &mut member.pragmas, &member.name)?;
                    self.type_(&mut member.type_)?;
                }
                MemberItem::Group(group) => self.members(group)?,
                MemberItem::ComponentsOf(_) | MemberItem::ExtensionMarker => {}
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pragma_is_a_key_and_a_value() {
        assert_eq!(
            key_and_value(" rpc.method = Get = 1 "),
            Ok(("rpc.method".to_string(), "Get = 1".to_string()))
        );
        assert_eq!(
            key_and_value("deprecated"),
            Ok(("deprecated".to_string(), String::new()))
        );
        assert_eq!(
            key_and_value("go-package_2=x"),
            Ok(("go-package_2".to_string(), "x".to_string()))
        );
        assert!(key_and_value(" = 1").is_err());
        assert!(key_and_value("1st = 1").is_err());
        assert!(key_and_value("a b = 1").is_err());
    }
}
