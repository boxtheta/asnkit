// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! JSON, as far as JER needs it: a parser that is strict, and the
//! writing of strings as Python writes them.

use alloc::string::String;
use alloc::vec::Vec;

use crate::decimal;
use crate::value::Value;
use crate::{Error, Result};

/// How deep arrays and objects may be nested.
pub(crate) const MAX_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Json {
    Null,
    Bool(bool),
    /// An integer: a `Value::Integer` or a `Value::BigInteger`.
    Int(Value<'static>),
    Float(f64),
    Str(String),
    Array(Vec<Json>),
    /// In the order of the first appearance of each name, with the last
    /// value of a name given twice.
    Object(Vec<(String, Json)>),
}

impl Json {
    pub fn get(&self, name: &str) -> Option<&Json> {
        match self {
            Json::Object(members) => members
                .iter()
                .find(|(candidate, _)| candidate == name)
                .map(|(_, value)| value),
            _ => None,
        }
    }
}

fn error<T>(what: &'static str, offset: usize) -> Result<T> {
    Err(Error::decode(what, offset))
}

struct Parser<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl Parser<'_> {
    fn skip_space(&mut self) {
        while matches!(self.bytes.get(self.at), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.at += 1;
        }
    }

    fn literal(&mut self, word: &str) -> bool {
        if self.bytes[self.at..].starts_with(word.as_bytes()) {
            self.at += word.len();

            true
        } else {
            false
        }
    }

    fn value(&mut self, depth: usize) -> Result<Json> {
        if depth > MAX_DEPTH {
            return error("The JSON is nested too deeply.", self.at);
        }

        self.skip_space();

        match self.bytes.get(self.at) {
            None => error("Expected a JSON value.", self.at),
            Some(b'{') => self.object(depth),
            Some(b'[') => self.array(depth),
            Some(b'"') => self.string().map(Json::Str),
            Some(b't') if self.literal("true") => Ok(Json::Bool(true)),
            Some(b'f') if self.literal("false") => Ok(Json::Bool(false)),
            Some(b'n') if self.literal("null") => Ok(Json::Null),
            // Python reads these too.
            Some(b'N') if self.literal("NaN") => Ok(Json::Float(f64::NAN)),
            Some(b'I') if self.literal("Infinity") => Ok(Json::Float(f64::INFINITY)),
            Some(b'-') if self.literal("-Infinity") => Ok(Json::Float(f64::NEG_INFINITY)),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => error("Expected a JSON value.", self.at),
        }
    }

    fn number(&mut self) -> Result<Json> {
        let start = self.at;
        let digits = |parser: &mut Parser<'_>| {
            let begin = parser.at;

            while matches!(parser.bytes.get(parser.at), Some(b'0'..=b'9')) {
                parser.at += 1;
            }

            parser.at - begin
        };

        if self.bytes.get(self.at) == Some(&b'-') {
            self.at += 1;
        }

        match self.bytes.get(self.at) {
            Some(b'0') => self.at += 1,
            Some(b'1'..=b'9') => {
                digits(self);
            }
            _ => return error("Bad JSON number.", start),
        }

        let mut is_float = false;

        if self.bytes.get(self.at) == Some(&b'.') {
            self.at += 1;
            is_float = true;

            if digits(self) == 0 {
                return error("Bad JSON number.", start);
            }
        }

        if matches!(self.bytes.get(self.at), Some(b'e' | b'E')) {
            self.at += 1;
            is_float = true;

            if matches!(self.bytes.get(self.at), Some(b'+' | b'-')) {
                self.at += 1;
            }

            if digits(self) == 0 {
                return error("Bad JSON number.", start);
            }
        }

        // The bytes were only ASCII digits and signs.
        let text = core::str::from_utf8(&self.bytes[start..self.at]).unwrap_or("0");

        if is_float {
            match text.parse::<f64>() {
                Ok(value) => Ok(Json::Float(value)),
                Err(_) => error("Bad JSON number.", start),
            }
        } else {
            match decimal::decimal_to_value(text) {
                Some(value) => Ok(Json::Int(value)),
                None => error("A JSON integer with too many digits.", start),
            }
        }
    }

    fn hex4(&mut self) -> Result<u32> {
        let part = self.bytes.get(self.at..self.at + 4);
        let value = part
            .and_then(|part| core::str::from_utf8(part).ok())
            .filter(|part| part.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .and_then(|part| u32::from_str_radix(part, 16).ok());

        match value {
            Some(value) => {
                self.at += 4;

                Ok(value)
            }
            None => error("Bad \\u escape in a JSON string.", self.at),
        }
    }

    fn string(&mut self) -> Result<String> {
        // At the quote.
        self.at += 1;

        let mut text = String::new();

        loop {
            // The input is a &str, so what is not an escape is UTF-8.
            let run = self.bytes[self.at..]
                .iter()
                .position(|byte| matches!(byte, b'"' | b'\\' | 0..=0x1f))
                .map(|length| self.at + length);

            let Some(end) = run else {
                return error("A JSON string is not ended.", self.at);
            };

            text.push_str(core::str::from_utf8(&self.bytes[self.at..end]).unwrap_or(""));
            self.at = end;

            match self.bytes[self.at] {
                b'"' => {
                    self.at += 1;

                    return Ok(text);
                }
                b'\\' => {
                    self.at += 1;

                    let Some(&escape) = self.bytes.get(self.at) else {
                        return error("A JSON string is not ended.", self.at);
                    };

                    self.at += 1;

                    match escape {
                        b'"' => text.push('"'),
                        b'\\' => text.push('\\'),
                        b'/' => text.push('/'),
                        b'b' => text.push('\u{8}'),
                        b'f' => text.push('\u{c}'),
                        b'n' => text.push('\n'),
                        b'r' => text.push('\r'),
                        b't' => text.push('\t'),
                        b'u' => {
                            let first = self.hex4()?;
                            let code = if (0xd800..0xdc00).contains(&first) {
                                // A pair.
                                if !self.literal("\\u") {
                                    return error("A lone surrogate in a JSON string.", self.at);
                                }

                                let second = self.hex4()?;

                                if !(0xdc00..0xe000).contains(&second) {
                                    return error("A lone surrogate in a JSON string.", self.at);
                                }

                                0x10000 + ((first - 0xd800) << 10) + (second - 0xdc00)
                            } else {
                                first
                            };

                            match char::from_u32(code) {
                                Some(c) => text.push(c),
                                None => {
                                    return error("A lone surrogate in a JSON string.", self.at);
                                }
                            }
                        }
                        _ => return error("Bad escape in a JSON string.", self.at - 1),
                    }
                }
                _ => return error("A control character in a JSON string.", self.at),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Json> {
        self.at += 1;

        let mut items = Vec::new();

        self.skip_space();

        if self.bytes.get(self.at) == Some(&b']') {
            self.at += 1;

            return Ok(Json::Array(items));
        }

        loop {
            items.push(self.value(depth + 1)?);
            self.skip_space();

            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b']') => {
                    self.at += 1;

                    return Ok(Json::Array(items));
                }
                _ => return error("Expected , or ] in a JSON array.", self.at),
            }
        }
    }

    fn object(&mut self, depth: usize) -> Result<Json> {
        self.at += 1;

        let mut members: Vec<(String, Json)> = Vec::new();

        self.skip_space();

        if self.bytes.get(self.at) == Some(&b'}') {
            self.at += 1;

            return Ok(Json::Object(members));
        }

        loop {
            self.skip_space();

            if self.bytes.get(self.at) != Some(&b'"') {
                return error("Expected a name in a JSON object.", self.at);
            }

            let name = self.string()?;

            self.skip_space();

            if self.bytes.get(self.at) != Some(&b':') {
                return error("Expected : in a JSON object.", self.at);
            }

            self.at += 1;

            let value = self.value(depth + 1)?;

            match members.iter_mut().find(|(candidate, _)| *candidate == name) {
                Some(entry) => entry.1 = value,
                None => members.push((name, value)),
            }

            self.skip_space();

            match self.bytes.get(self.at) {
                Some(b',') => self.at += 1,
                Some(b'}') => {
                    self.at += 1;

                    return Ok(Json::Object(members));
                }
                _ => return error("Expected , or } in a JSON object.", self.at),
            }
        }
    }
}

/// Reads the JSON in `text`, and nothing after it.
pub(crate) fn parse(text: &str) -> Result<Json> {
    let mut parser = Parser {
        bytes: text.as_bytes(),
        at: 0,
    };
    let value = parser.value(0)?;

    parser.skip_space();

    if parser.at != text.len() {
        return error("Data after the JSON value.", parser.at);
    }

    Ok(value)
}

/// JSON written with no spaces, as JER writes it, with `indent` spaces
/// for each level, as `json.dumps` of Python does with `indent`: an
/// item and a member on a line of its own, `": "` after a name, and an
/// empty `{}` and `[]` as they are.
pub(crate) fn indent(compact: &str, indent: usize) -> String {
    let mut out = String::with_capacity(compact.len() * 2);
    let mut level = 0;
    let mut in_string = false;
    let mut escaped = false;
    let mut chars = compact.chars().peekable();

    let new_line = |out: &mut String, level: usize| {
        out.push('\n');
        out.extend(core::iter::repeat_n(' ', level * indent));
    };

    while let Some(c) = chars.next() {
        if in_string {
            out.push(c);

            if escaped {
                escaped = false;
            } else if c == '\\' {
                escaped = true;
            } else if c == '"' {
                in_string = false;
            }

            continue;
        }

        match c {
            '"' => {
                in_string = true;
                out.push(c);
            }
            '{' | '[' => {
                out.push(c);

                if matches!(chars.peek(), Some('}' | ']')) {
                    out.push(chars.next().unwrap_or(c));
                } else {
                    level += 1;
                    new_line(&mut out, level);
                }
            }
            '}' | ']' => {
                level -= 1;
                new_line(&mut out, level);
                out.push(c);
            }
            ',' => {
                out.push(c);
                new_line(&mut out, level);
            }
            ':' => out.push_str(": "),
            _ => out.push(c),
        }
    }

    out
}

/// Writes `text` as a JSON string, with all but ASCII as `\u` escapes,
/// as `json.dumps` of Python does.
pub(crate) fn write_string(out: &mut String, text: &str) {
    use core::fmt::Write;

    out.push('"');

    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            ' '..='~' => out.push(c),
            _ => {
                let mut units = [0u16; 2];

                for unit in c.encode_utf16(&mut units) {
                    let _ = write!(out, "\\u{unit:04x}");
                }
            }
        }
    }

    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_json() {
        assert_eq!(parse(" null ").unwrap(), Json::Null);
        assert_eq!(parse("true").unwrap(), Json::Bool(true));
        assert_eq!(parse("-0").unwrap(), Json::Int(Value::Integer(0)));
        assert_eq!(parse("1.5e1").unwrap(), Json::Float(15.0));
        assert_eq!(
            parse("\"a\\u00e9\\ud83d\\ude00\\n\"").unwrap(),
            Json::Str(String::from("aé😀\n"))
        );
        assert_eq!(
            parse("{\"a\":[1,{\"b\":null}],\"a\":2,\"c\":\"\"}").unwrap(),
            Json::Object(alloc::vec![
                (String::from("a"), Json::Int(Value::Integer(2))),
                (String::from("c"), Json::Str(String::new())),
            ])
        );

        for bad in [
            "",
            "{",
            "[1,]",
            "{\"a\":1,}",
            "01",
            "1.",
            ".5",
            "+1",
            "\"\\x\"",
            "\"a",
            "nul",
            "[] []",
            "\"\u{1}\"",
            "\"\\ud800\"",
            "{1:2}",
            "'a'",
            "1 2",
        ] {
            assert!(parse(bad).is_err(), "{bad:?}");
        }

        assert!(parse(&"[".repeat(MAX_DEPTH + 2)).is_err());
    }

    #[test]
    fn writes_strings_as_python() {
        let mut out = String::new();

        write_string(&mut out, "a\"\\/\n\t\u{1}é\u{7f}😀");

        assert_eq!(
            out,
            "\"a\\\"\\\\/\\n\\t\\u0001\\u00e9\\u007f\\ud83d\\ude00\""
        );
    }
}
