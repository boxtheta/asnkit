// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The values of the dictionaries of the Python parser, which a value of
//! the syntax tree is made from, and the conversions between them.
//!
//! Python tells a character string (`CString`) from any other string
//! with its class, and an integer from a float. `Data` does the same.

use crate::ast::Value;
use serde_json::Number;

#[derive(Debug, Clone, PartialEq)]
pub enum Data {
    /// `None`: NULL, or a value that is not supported.
    None,
    Bool(bool),
    /// An integer, as its digits.
    Int(String),
    Float(f64),
    /// A word: a reference to a value, an identifier, TRUE, `0b0101`.
    Word(String),
    /// A character string given in quotes.
    CString(String),
    List(Vec<Data>),
    /// A tuple of the name of an alternative of a choice and its value.
    Pair(String, Box<Data>),
    /// A dictionary, in the order its keys were added.
    Dict(Vec<(String, Data)>),
}

/// What a string of a dictionary is taken for, given where it is found.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Where {
    Value,
    Constraint,
    PermittedAlphabet,
}

pub fn int(value: i64) -> Data {
    Data::Int(value.to_string())
}

/// `int()` of Python, for a string of digits with a sign.
fn parse_int(text: &str) -> Option<String> {
    let digits = text.strip_prefix(['-', '+']).unwrap_or(text);

    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }

    let negative = text.starts_with('-');
    let trimmed = digits.trim_start_matches('0');
    let trimmed = if trimmed.is_empty() { "0" } else { trimmed };

    Some(if negative && trimmed != "0" {
        format!("-{trimmed}")
    } else {
        trimmed.to_string()
    })
}

/// `convert_number()` of the Python parser: an integer if it can be one.
pub fn convert_number(data: Data) -> Data {
    match data {
        Data::Word(text) => match parse_int(&text) {
            Some(number) => Data::Int(number),
            None => Data::Word(text),
        },
        // int() of a float drops the fraction.
        Data::Float(value) => Data::Int(format!("{}", value.trunc() as i128)),
        Data::Bool(value) => Data::Int(if value { "1" } else { "0" }.to_string()),
        other => other,
    }
}

fn is_identifier(text: &str) -> bool {
    let mut chars = text.chars();

    match chars.next() {
        Some(c) if c.is_ascii_lowercase() => {}
        _ => return false,
    }

    // [a-z][a-zA-Z0-9]*(-[a-zA-Z0-9]+)*
    let mut previous_hyphen = false;
    let mut last = 'a';

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

        last = c;
    }

    last != '-'
}

fn float_value(value: f64) -> Value {
    let text = format!("{value:?}");

    Value::Number(text.parse::<Number>().unwrap())
}

/// `_value_from_data()` of the Python converter.
pub fn value_from_data(
    data: &Data,
    where_: Where,
    type_name: Option<&str>,
    is_nested: bool,
) -> Value {
    match data {
        Data::None => {
            if is_nested || type_name == Some("NULL") {
                Value::Null
            } else {
                Value::UnsupportedValue
            }
        }
        Data::Bool(value) => Value::Bool(*value),
        Data::Int(digits) => Value::Number(digits.parse::<Number>().unwrap()),
        Data::Float(value) => float_value(*value),
        Data::List(items) => Value::List(
            items
                .iter()
                .map(|item| value_from_data(item, where_, None, true))
                .collect(),
        ),
        Data::Pair(name, value) => Value::NamedValue(
            name.clone(),
            Box::new(value_from_data(value, where_, None, true)),
        ),
        Data::Dict(members) => Value::SequenceValue(
            members
                .iter()
                .map(|(name, value)| (name.clone(), value_from_data(value, where_, None, true)))
                .collect(),
        ),
        Data::CString(text) => Value::Str(text.clone()),
        Data::Word(text) => {
            if where_ == Where::PermittedAlphabet {
                return Value::Str(text.clone());
            }

            if where_ == Where::Constraint {
                match text.as_str() {
                    "MIN" => return Value::Min,
                    "MAX" => return Value::Max,
                    "TRUE" => return Value::Bool(true),
                    "FALSE" => return Value::Bool(false),
                    _ => {}
                }
            }

            match text.as_str() {
                "PLUS-INFINITY" => Value::PlusInfinity,
                "MINUS-INFINITY" => Value::MinusInfinity,
                "NOT-A-NUMBER" => Value::NotANumber,
                _ => {
                    if let Some(bits) = text.strip_prefix("0b")
                        && bits.bytes().all(|b| b == b'0' || b == b'1')
                    {
                        Value::BinaryString(bits.to_string())
                    } else if let Some(digits) = text.strip_prefix("0x")
                        && digits.bytes().all(|b| b.is_ascii_hexdigit())
                    {
                        Value::HexString(digits.to_string())
                    } else if is_identifier(text) {
                        Value::Identifier(text.clone())
                    } else {
                        Value::Str(text.clone())
                    }
                }
            }
        }
    }
}

/// `_number_from_data()`: a number of a tag or a named number, which is
/// an identifier if given by reference.
pub fn number_from_data(data: &Data) -> Value {
    match data {
        Data::Word(text) => {
            if !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit()) {
                Value::Number(parse_int(text).unwrap().parse::<Number>().unwrap())
            } else {
                Value::Identifier(text.clone())
            }
        }
        Data::Int(digits) => Value::Number(digits.parse::<Number>().unwrap()),
        Data::Float(value) => float_value(*value),
        other => value_from_data(other, Where::Value, None, false),
    }
}

// ----------------------------------------------------------------------
// The text of a value in braces, or of a choice

#[derive(Debug, Clone)]
enum TextToken {
    CString(String),
    BString(String),
    HString(String),
    Number(String),
    Word(String),
    Symbol(char),
}

struct TextError;

fn tokenize(text: &str) -> Result<Vec<TextToken>, TextError> {
    let chars: Vec<char> = text.trim_end().chars().collect();
    let mut tokens = Vec::new();
    let mut at = 0;

    while at < chars.len() {
        while at < chars.len() && chars[at].is_whitespace() {
            at += 1;
        }

        if at >= chars.len() {
            break;
        }

        let c = chars[at];

        if c == '"' {
            let mut end = at + 1;

            loop {
                if end >= chars.len() {
                    return Err(TextError);
                }

                if chars[end] == '"' {
                    if end + 1 < chars.len() && chars[end + 1] == '"' {
                        end += 2;
                        continue;
                    }

                    break;
                }

                end += 1;
            }

            let inner: String = chars[at + 1..end].iter().collect();
            tokens.push(TextToken::CString(inner.replace("\"\"", "\"")));
            at = end + 1;
        } else if c == '\'' {
            let mut end = at + 1;

            while end < chars.len() && chars[end] != '\'' {
                end += 1;
            }

            if end + 1 >= chars.len() {
                return Err(TextError);
            }

            let inner: String = chars[at + 1..end].iter().collect();
            let suffix = chars[end + 1];

            if suffix == 'B'
                && inner
                    .chars()
                    .all(|c| c == '0' || c == '1' || c.is_whitespace())
            {
                tokens.push(TextToken::BString(inner));
            } else if suffix == 'H'
                && inner
                    .chars()
                    .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c) || c.is_whitespace())
            {
                tokens.push(TextToken::HString(inner));
            } else {
                return Err(TextError);
            }

            at = end + 2;
        } else if c.is_ascii_digit()
            || (c == '-' && chars.get(at + 1).is_some_and(|d| d.is_ascii_digit()))
        {
            let mut end = at + 1;

            while end < chars.len() && chars[end].is_ascii_digit() {
                end += 1;
            }

            if end < chars.len() && chars[end] == '.' {
                end += 1;

                while end < chars.len() && chars[end].is_ascii_digit() {
                    end += 1;
                }
            }

            if end < chars.len() && (chars[end] == 'e' || chars[end] == 'E') {
                let mut exponent = end + 1;

                if exponent < chars.len() && (chars[exponent] == '+' || chars[exponent] == '-') {
                    exponent += 1;
                }

                let digits = exponent;

                while exponent < chars.len() && chars[exponent].is_ascii_digit() {
                    exponent += 1;
                }

                if exponent > digits {
                    end = exponent;
                }
            }

            tokens.push(TextToken::Number(chars[at..end].iter().collect()));
            at = end;
        } else if c.is_ascii_alphabetic() {
            let mut end = at + 1;

            while end < chars.len() && (chars[end].is_ascii_alphanumeric() || chars[end] == '-') {
                end += 1;
            }

            tokens.push(TextToken::Word(chars[at..end].iter().collect()));
            at = end;
        } else if "{},:()".contains(c) {
            tokens.push(TextToken::Symbol(c));
            at += 1;
        } else {
            return Err(TextError);
        }
    }

    Ok(tokens)
}

struct TextParser {
    tokens: Vec<TextToken>,
    offset: usize,
}

impl TextParser {
    fn peek(&self, offset: usize) -> Option<&TextToken> {
        self.tokens.get(self.offset + offset)
    }

    fn peek_symbol(&self, offset: usize) -> Option<char> {
        match self.peek(offset) {
            Some(TextToken::Symbol(c)) => Some(*c),
            _ => None,
        }
    }

    fn take_symbol(&mut self, expected: char) -> Result<(), TextError> {
        if self.peek_symbol(0) == Some(expected) {
            self.offset += 1;

            Ok(())
        } else {
            Err(TextError)
        }
    }

    fn is_value_next(&self, offset: usize) -> bool {
        match self.peek(offset) {
            None => false,
            Some(TextToken::Symbol(c)) => *c == '{',
            Some(_) => true,
        }
    }

    fn parse_value(&mut self) -> Result<Data, TextError> {
        let token = self.tokens.get(self.offset).ok_or(TextError)?.clone();
        self.offset += 1;

        match &token {
            TextToken::CString(text) => Ok(Data::CString(text.clone())),
            TextToken::BString(text) => Ok(Data::Word(format!(
                "0b{}",
                text.chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
            ))),
            TextToken::HString(text) => Ok(Data::Word(format!(
                "0x{}",
                text.chars()
                    .filter(|c| !c.is_whitespace())
                    .collect::<String>()
                    .to_lowercase()
            ))),
            TextToken::Number(text) => {
                if let Some(number) = parse_int(text) {
                    Ok(Data::Int(number))
                } else {
                    text.parse::<f64>().map(Data::Float).map_err(|_| TextError)
                }
            }
            TextToken::Word(word) => {
                if word == "TRUE" || word == "FALSE" {
                    Ok(Data::Bool(word == "TRUE"))
                } else if word == "NULL" {
                    Ok(Data::None)
                } else if self.peek_symbol(0) == Some(':') {
                    self.offset += 1;
                    let value = self.parse_value()?;

                    Ok(Data::Pair(word.clone(), Box::new(value)))
                } else {
                    Ok(Data::Word(word.clone()))
                }
            }
            TextToken::Symbol('{') => {
                let offset = self.offset;

                match self.parse_items() {
                    Ok(value) => Ok(value),
                    Err(TextError) => {
                        self.offset = offset;

                        self.parse_object_identifier()
                    }
                }
            }
            TextToken::Symbol(_) => Err(TextError),
        }
    }

    fn parse_items(&mut self) -> Result<Data, TextError> {
        let mut items = Vec::new();
        let mut names: Vec<String> = Vec::new();

        while self.peek_symbol(0) != Some('}') {
            if !items.is_empty() {
                self.take_symbol(',')?;
            }

            if let Some(TextToken::Word(word)) = self.peek(0)
                && self.is_value_next(1)
            {
                names.push(word.clone());
                self.offset += 1;
            }

            items.push(self.parse_value()?);
        }

        self.take_symbol('}')?;

        if !items.is_empty() && names.len() == items.len() {
            let mut members: Vec<(String, Data)> = Vec::new();

            for (name, value) in names.into_iter().zip(items) {
                match members.iter_mut().find(|(n, _)| *n == name) {
                    Some(existing) => existing.1 = value,
                    None => members.push((name, value)),
                }
            }

            Ok(Data::Dict(members))
        } else if !names.is_empty() {
            Err(TextError)
        } else {
            Ok(Data::List(items))
        }
    }

    fn parse_object_identifier(&mut self) -> Result<Data, TextError> {
        let mut components = Vec::new();

        while self.peek_symbol(0) != Some('}') {
            let token = self.tokens.get(self.offset).ok_or(TextError)?;
            self.offset += 1;

            match token {
                TextToken::Number(text) => {
                    components.push(convert_number(Data::Word(text.clone())));
                }
                TextToken::Word(word) => {
                    if self.peek_symbol(0) == Some('(') {
                        self.offset += 1;

                        let number = match self.tokens.get(self.offset) {
                            Some(TextToken::Number(text)) => text.clone(),
                            Some(TextToken::Word(text)) => text.clone(),
                            _ => return Err(TextError),
                        };

                        self.offset += 1;
                        components.push(Data::Pair(
                            word.clone(),
                            Box::new(convert_number(Data::Word(number))),
                        ));
                        self.take_symbol(')')?;
                    } else {
                        components.push(Data::Word(word.clone()));
                    }
                }
                _ => return Err(TextError),
            }
        }

        self.take_symbol('}')?;

        Ok(Data::List(components))
    }
}

/// `convert_value_text()`: the text of a value in braces, or of a
/// choice, as data. `None` if it is not supported.
pub fn convert_value_text(text: &str) -> Data {
    let Ok(tokens) = tokenize(text) else {
        return Data::None;
    };
    let mut parser = TextParser { tokens, offset: 0 };

    match parser.parse_value() {
        Ok(value) if parser.offset == parser.tokens.len() => match value {
            Data::Word(_) => Data::None,
            other => other,
        },
        _ => Data::None,
    }
}
