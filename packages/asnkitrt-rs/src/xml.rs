// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! XML, as far as XER needs it: a tree of elements with the text before
//! their first child, and a writer.
//!
//! The reading is done by `quick-xml`, a reader that takes one event at
//! a time. A document type declaration is refused, so there are no
//! entities to grow and none that are fetched, and how deep elements
//! are and how many events there are is limited. Attributes and
//! namespaces are dropped: a name is the local name of the element.

use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::{Error, Result};

/// How deep elements may be nested.
pub(crate) const MAX_DEPTH: usize = 128;

/// How many events a document may have.
const MAX_EVENTS: usize = 4_000_000;

#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct Element {
    pub name: String,
    /// The text before the first child.
    pub text: Option<String>,
    pub children: Vec<Element>,
    /// The text after the element, before the next one, in the element
    /// it is in.
    pub tail: Option<String>,
}

/// The names of the control characters in an `xmlcstring`, by their
/// code (X.680 12.15.5, Table 3). Tab, line feed and carriage return
/// are written as they are.
const CONTROLS: [&str; 32] = [
    "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "", "", "vt", "ff", "", "so",
    "si", "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb", "can", "em", "sub", "esc", "is4",
    "is3", "is2", "is1",
];

/// The control character an empty element in an `xmlcstring` stands
/// for, as `<bel/>` (X.680 12.15.5).
pub(crate) fn control(name: &str) -> Option<char> {
    CONTROLS
        .iter()
        .position(|candidate| !candidate.is_empty() && *candidate == name)
        .and_then(|code| char::from_u32(code as u32))
}

impl Element {
    pub fn new(name: &str) -> Element {
        Element {
            name: name.replace(' ', "_"),
            text: None,
            children: Vec::new(),
            tail: None,
        }
    }

    pub fn with_text(name: &str, text: &str) -> Element {
        let mut element = Element::new(name);

        if !text.is_empty() {
            element.text = Some(String::from(text));
        }

        element
    }

    pub fn child(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.name == name)
    }
}

fn bad<T>() -> Result<T> {
    Err(Error::decode("Bad XML.", 0))
}

fn is_xml_char(c: char) -> bool {
    matches!(c, '\t' | '\n' | '\r' | ' '..='\u{d7ff}' | '\u{e000}'..='\u{fffd}' | '\u{10000}'..)
}

/// The character a reference is: one of the five names XML has, or a
/// number. Nothing else is, as a document type declaration is refused.
fn reference(name: &str) -> Result<char> {
    let c = match name {
        "lt" => Some('<'),
        "gt" => Some('>'),
        "amp" => Some('&'),
        "quot" => Some('"'),
        "apos" => Some('\''),
        _ => name
            .strip_prefix('#')
            .and_then(|number| match number.strip_prefix(['x', 'X']) {
                Some(hex) => u32::from_str_radix(hex, 16).ok(),
                None => number.parse().ok(),
            })
            .and_then(char::from_u32),
    };

    match c {
        Some(c) if is_xml_char(c) => Ok(c),
        _ => bad(),
    }
}

/// Reads the document in `text`.
///
/// The reader of `quick-xml` takes one event at a time and builds
/// nothing, so how deep elements are is held here, and a document type
/// declaration, which can define entities, is refused. Only the five
/// entities XML has and the references of characters are taken out.
pub(crate) fn parse(text: &str) -> Result<Element> {
    use quick_xml::events::Event;

    let mut reader = quick_xml::Reader::from_str(text);
    let mut stack: Vec<Element> = Vec::new();
    let mut root = None;
    let mut events = 0usize;

    // Text goes to the element that is open, or after its last child.
    fn add(stack: &mut [Element], more: &str) -> Result<()> {
        if !more.chars().all(is_xml_char) {
            return bad();
        }

        match stack.last_mut() {
            Some(open) => {
                if !more.is_empty() {
                    let text = match open.children.last_mut() {
                        Some(child) => &mut child.tail,
                        None => &mut open.text,
                    };

                    text.get_or_insert_with(String::new).push_str(more);
                }

                Ok(())
            }
            // Between the elements and around the root, only space.
            None if more.trim_matches([' ', '\t', '\r', '\n']).is_empty() => Ok(()),
            None => bad(),
        }
    }

    loop {
        events += 1;

        if events > MAX_EVENTS {
            return bad();
        }

        let event = reader.read_event().or_else(|_| bad())?;

        match event {
            Event::Start(ref start) | Event::Empty(ref start) => {
                if root.is_some() || stack.len() >= MAX_DEPTH {
                    return bad();
                }

                // The attributes are dropped, but they have to be well formed.
                for attribute in start.attributes() {
                    attribute.or_else(|_| bad())?;
                }

                let element = Element {
                    name: start.local_name().as_ref().to_string(),
                    text: None,
                    children: Vec::new(),
                    tail: None,
                };

                if matches!(event, Event::Empty(_)) {
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(element),
                        None => root = Some(element),
                    }
                } else {
                    stack.push(element);
                }
            }
            Event::End(_) => {
                // The reader has checked that the name is that of the start.
                let Some(done) = stack.pop() else {
                    return bad();
                };

                match stack.last_mut() {
                    Some(parent) => parent.children.push(done),
                    None => root = Some(done),
                }
            }
            Event::Text(data) => add(&mut stack, &data.xml10_content())?,
            Event::CData(data) => {
                if stack.is_empty() {
                    return bad();
                }

                add(&mut stack, &data.xml10_content())?;
            }
            Event::GeneralRef(name) => {
                let mut buffer = [0u8; 4];

                add(&mut stack, reference(&name)?.encode_utf8(&mut buffer))?;
            }
            Event::DocType(_) => return bad(),
            Event::Decl(_) | Event::PI(_) | Event::Comment(_) => {}
            Event::Eof => break,
        }
    }

    match root {
        Some(root) if stack.is_empty() => Ok(root),
        _ => bad(),
    }
}

fn write_text(out: &mut String, text: &str) {
    use core::fmt::Write;

    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            // Written as it is, a reader would make it a line feed.
            '\r' => out.push_str("&#13;"),
            // The control characters XML does not have (X.680 12.15.5).
            c if (c as u32) < 0x20 && !matches!(c, '\t' | '\n' | '\r') => {
                out.push('<');
                out.push_str(CONTROLS[c as usize]);
                out.push_str("/>");
            }
            c if c.is_ascii() => out.push(c),
            c => {
                let _ = write!(out, "&#{};", c as u32);
            }
        }
    }
}

/// Writes `element` as `ElementTree.tostring` of Python does, in ASCII.
pub(crate) fn write(element: &Element, out: &mut String) {
    out.push('<');
    out.push_str(&element.name);

    if element.text.as_ref().is_none_or(String::is_empty) && element.children.is_empty() {
        out.push_str(" />");

        return;
    }

    out.push('>');

    if let Some(text) = &element.text {
        write_text(out, text);
    }

    for child in &element.children {
        write(child, out);
    }

    out.push_str("</");
    out.push_str(&element.name);
    out.push('>');
}

/// Writes `element` as `write()` does, with `indent` spaces for each
/// level, as the runtime for Python does: each child on a line of its
/// own, and a line feed after the root if it has children.
pub(crate) fn write_indented(element: &Element, out: &mut String, indent: usize) {
    fn write_level(element: &Element, out: &mut String, indent: usize, level: usize) {
        if element.children.is_empty() {
            write(element, out);

            return;
        }

        out.push('<');
        out.push_str(&element.name);
        out.push('>');

        if let Some(text) = element.text.as_ref().filter(|text| !text.trim().is_empty()) {
            write_text(out, text);
        }

        for child in &element.children {
            out.push('\n');
            out.extend(core::iter::repeat_n(' ', (level + 1) * indent));
            write_level(child, out, indent, level + 1);
        }

        out.push('\n');
        out.extend(core::iter::repeat_n(' ', level * indent));
        out.push_str("</");
        out.push_str(&element.name);
        out.push('>');
    }

    write_level(element, out, indent, 0);

    if !element.children.is_empty() {
        out.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_xml() {
        let root = parse(
            "<?xml version=\"1.0\"?>\n<!-- c --><a x='1' y=\"&lt;\">t&amp;&#65;&#x42;<![CDATA[<c>]]><b/><c>\r\nz</c>tail</a>\n",
        )
        .unwrap();

        assert_eq!(root.name, "a");
        assert_eq!(root.text.as_deref(), Some("t&AB<c>"));
        assert_eq!(root.children.len(), 2);
        assert_eq!(root.child("b").unwrap().text, None);
        assert_eq!(root.child("c").unwrap().text.as_deref(), Some("\nz"));

        // Names are the local ones.
        let root = parse("<n:a xmlns:n='urn:x'><n:b/></n:a>").unwrap();

        assert_eq!(
            (root.name.as_str(), root.children[0].name.as_str()),
            ("a", "b")
        );

        for bad in [
            "",
            "<a>",
            "<a></b>",
            "<a/><b/>",
            "<a>&nope;</a>",
            "<a>&#0;</a>",
            "<a x=1/>",
            "<!DOCTYPE a [<!ENTITY e 'x'>]><a>&e;</a>",
            "<!DOCTYPE a><a/>",
            "text<a/>",
            "<a>\u{1}</a>",
            "</a>",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }

        assert!(parse(&"<a>".repeat(MAX_DEPTH + 2)).is_err());
    }

    #[test]
    fn writes_xml_as_python() {
        let mut root = Element::new("a b");

        root.children.push(Element::new("empty"));
        root.children.push(Element::with_text("t", "x<&>é😀"));

        let mut out = String::new();

        write(&root, &mut out);

        assert_eq!(
            out,
            "<a_b><empty /><t>x&lt;&amp;&gt;&#233;&#128512;</t></a_b>"
        );
    }
}
