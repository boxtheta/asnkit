// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The ASN.1 grammar, as parsers of winnow. It follows the Python parser,
//! `asnkit_compiler/parser.py`, which is made of the productions of X.680
//! to X.683, but gives the nodes of the syntax tree directly.

use crate::ast::*;
use crate::data::{Data, Where, convert_number, convert_value_text, value_from_data};
use crate::source::Locator;
use serde_json::Number;
use std::rc::Rc;
use winnow::error::{ContextError, ErrMode, ModalResult};
use winnow::stream::{LocatingSlice, Location as _, Stateful, Stream};

/// What is known while parsing.
#[derive(Debug, Default, Clone)]
pub struct State {
    /// The offset the parse got farthest to without matching, and what
    /// was expected there.
    pub farthest: usize,
    pub expected: Vec<&'static str>,
    pub locator: Option<Rc<Locator>>,
    pub text: Option<Rc<str>>,
    /// The message of an error of a conversion, which is not a syntax
    /// error.
    pub conversion_error: Option<String>,
    /// Where the error of a conversion is, if it tells.
    pub conversion_offset: Option<usize>,
}

impl State {
    fn slice(&self, begin: usize, end: usize) -> &str {
        &self.text.as_ref().unwrap()[begin..end]
    }

    fn line_of(&self, offset: usize) -> usize {
        self.text.as_ref().unwrap()[..offset].matches('\n').count() + 1
    }

    fn locate(&self, offset: usize) -> crate::ast::Location {
        self.locator.as_ref().unwrap().locate_offset(offset)
    }

    fn note(&mut self, offset: usize, what: &'static str) {
        LAST_FAILURE.with(|cell| cell.set(offset));

        if offset > self.farthest {
            self.farthest = offset;
            self.expected.clear();
        }

        if offset == self.farthest && !self.expected.contains(&what) {
            self.expected.push(what);
        }
    }
}

pub type Input<'a> = Stateful<LocatingSlice<&'a str>, State>;
type R<T> = ModalResult<T>;

/// Tries each of the parsers in turn, and gives what the first that
/// matches does.
fn first<T>(i: &mut Input, parsers: &[fn(&mut Input) -> R<T>]) -> R<T> {
    for parser in parsers {
        let checkpoint = i.checkpoint();

        match parser(i) {
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            other => return other,
        }
    }

    backtrack()
}

/// An error of a conversion, which ends the parse with its own message.
fn conversion_error<T>(i: &mut Input, message: String) -> R<T> {
    i.state.conversion_error = Some(message);

    cut()
}

/// An error of a conversion at `offset`, which is told with the file,
/// the line and the column.
fn conversion_error_at<T>(i: &mut Input, message: String, offset: usize) -> R<T> {
    i.state.conversion_offset = Some(offset);

    conversion_error(i, message)
}

fn backtrack<T>() -> R<T> {
    Err(ErrMode::Backtrack(ContextError::new()))
}

fn cut<T>() -> R<T> {
    Err(ErrMode::Cut(ContextError::new()))
}

thread_local! {
    /// Where the last failure to match was.
    static LAST_FAILURE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    /// Where the first failure was that was made final.
    static CUT_OFFSET: std::cell::Cell<Option<usize>> = const { std::cell::Cell::new(None) };
}

/// Forgets where failures were, before a parse.
pub fn reset_failures() {
    LAST_FAILURE.with(|cell| cell.set(0));
    CUT_OFFSET.with(|cell| cell.set(None));
}

/// Where a syntax error is: where the first failure that was made final
/// was, or else the farthest the parse got.
pub fn error_offset(farthest: usize) -> usize {
    CUT_OFFSET.with(|cell| cell.get()).unwrap_or(farthest)
}

/// Makes a failure to match final, as the `-` of Pyparsing does.
fn commit<T>(result: R<T>) -> R<T> {
    match result {
        Err(ErrMode::Backtrack(error)) => {
            CUT_OFFSET.with(|cell| {
                if cell.get().is_none() {
                    cell.set(Some(LAST_FAILURE.with(|last| last.get())));
                }
            });

            Err(ErrMode::Cut(error))
        }
        other => other,
    }
}

fn ws(i: &mut Input) {
    while let Some(c) = i.peek_token() {
        if matches!(c, ' ' | '\t' | '\n' | '\r') {
            i.next_token();
        } else {
            break;
        }
    }
}

/// The offset of what is next, once whitespace is skipped.
fn here(i: &mut Input) -> usize {
    ws(i);

    i.current_token_start()
}

fn fail<T>(i: &mut Input, offset: usize, what: &'static str) -> R<T> {
    i.state.note(offset, what);

    backtrack()
}

fn peek_char(i: &Input) -> Option<char> {
    i.peek_token()
}

/// Matches given text after whitespace.
fn lit(i: &mut Input, text: &'static str) -> R<()> {
    let offset = here(i);
    let checkpoint = i.checkpoint();

    for expected in text.chars() {
        match i.next_token() {
            Some(c) if c == expected => {}
            _ => {
                i.reset(&checkpoint);

                return fail(i, offset, text);
            }
        }
    }

    Ok(())
}

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '$'
}

/// Matches given keyword after whitespace, as Pyparsing does: the text
/// as it is, and not followed by a character of a word.
fn kw(i: &mut Input, text: &'static str) -> R<()> {
    let offset = here(i);
    let checkpoint = i.checkpoint();
    lit(i, text)?;

    if peek_char(i).is_some_and(is_ident_char) {
        i.reset(&checkpoint);

        return fail(i, offset, text);
    }

    Ok(())
}

fn take_while_chars(i: &mut Input, f: impl Fn(char) -> bool) -> String {
    let mut out = String::new();

    while let Some(c) = i.peek_token() {
        if f(c) {
            out.push(c);
            i.next_token();
        } else {
            break;
        }
    }

    out
}

/// `[a-z][a-zA-Z0-9-]*`
fn identifier(i: &mut Input) -> R<(String, usize)> {
    let offset = here(i);

    match peek_char(i) {
        Some(c) if c.is_ascii_lowercase() => {
            let name = take_while_chars(i, |c| c.is_ascii_alphanumeric() || c == '-');

            Ok((name, offset))
        }
        _ => fail(i, offset, "identifier"),
    }
}

const RESERVED: [&str; 4] = ["END", "SEQUENCE", "ENUMERATED", "WITH"];

/// What `(END|SEQUENCE|ENUMERATED|WITH)(\s|$)` matches, as that is no
/// reference.
fn is_reserved(i: &mut Input) -> bool {
    let checkpoint = i.checkpoint();
    let mut found = false;

    for word in RESERVED {
        let mut matched = true;

        for expected in word.chars() {
            match i.next_token() {
                Some(c) if c == expected => {}
                _ => {
                    matched = false;
                    break;
                }
            }
        }

        if matched {
            match i.peek_token() {
                None => found = true,
                Some(c) if matches!(c, ' ' | '\t' | '\n' | '\r' | '\x0b' | '\x0c') => {
                    found = true;
                }
                _ => {}
            }
        }

        i.reset(&checkpoint);

        if found {
            break;
        }
    }

    found
}

/// `[A-Z][a-zA-Z0-9-]*`, but not one of the reserved words.
fn type_reference(i: &mut Input) -> R<(String, usize)> {
    let offset = here(i);

    if is_reserved(i) {
        return fail(i, offset, "typereference");
    }

    match peek_char(i) {
        Some(c) if c.is_ascii_uppercase() => {
            let name = take_while_chars(i, |c| c.is_ascii_alphanumeric() || c == '-');

            Ok((name, offset))
        }
        _ => fail(i, offset, "typereference"),
    }
}

/// `[A-Z][A-Z0-9-]*`, but not one of the reserved words.
fn object_class_reference(i: &mut Input) -> R<(String, usize)> {
    let offset = here(i);

    if is_reserved(i) {
        return fail(i, offset, "objectclassreference");
    }

    match peek_char(i) {
        Some(c) if c.is_ascii_uppercase() => {
            let name = take_while_chars(i, |c| {
                c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'
            });

            Ok((name, offset))
        }
        _ => fail(i, offset, "objectclassreference"),
    }
}

/// Digits that are not followed by a dot.
fn number(i: &mut Input) -> R<Number> {
    let offset = here(i);
    let checkpoint = i.checkpoint();
    let digits = take_while_chars(i, |c| c.is_ascii_digit());

    if digits.is_empty() || peek_char(i) == Some('.') {
        i.reset(&checkpoint);

        return fail(i, offset, "number");
    }

    Ok(digits.parse::<Number>().unwrap())
}

/// The module name, or `Name.` of an external reference.
fn module_reference(i: &mut Input) -> R<String> {
    type_reference(i).map(|(name, _)| name)
}

// ----------------------------------------------------------------------
// Values

/// What the tokens of a value start with, which is what the conversion of
/// a value looks at.
#[derive(Debug, Clone)]
enum Tok {
    /// A plain string: a word, `TRUE`, `0b0101`.
    Word(String),
    CString(String),
    Int(String),
    Float(f64),
    /// A bit string value: `0b0101`, `0x1f`, `CONTAINING` or a list of
    /// identifiers.
    BitString(BitStringValue),
    /// A value in braces that is not a list of identifiers.
    Brace,
    /// The type of an open type value, or the alternative of a choice,
    /// then a colon.
    Other,
    /// `NULL`, which the Python parser takes for the type NULL.
    Null,
}

#[derive(Debug, Clone)]
enum BitStringValue {
    Word(String),
    Identifiers(Vec<String>),
}

/// A value, with its source text.
#[derive(Debug, Clone)]
struct ValueTokens {
    first: Tok,
    second_is_colon: bool,
    /// The components of an object identifier in braces, if it is one.
    components: Option<Vec<Vec<Data>>>,
    /// The numbers of a quadruple or a tuple of a character in braces.
    quadruple: Option<Vec<Data>>,
    text: String,
}

fn quoted_string(i: &mut Input) -> R<String> {
    let offset = here(i);

    if peek_char(i) != Some('"') {
        return fail(i, offset, "string");
    }

    let checkpoint = i.checkpoint();
    i.next_token();
    let mut out = String::new();

    loop {
        match i.next_token() {
            Some('"') => return Ok(out),
            Some('\n') | None => {
                i.reset(&checkpoint);

                return fail(i, offset, "string");
            }
            Some(c) => out.push(c),
        }
    }
}

/// `'0101'B` or `'1F'H`, converted as `0b0101` and `0x1f`.
fn bit_or_hex_string(i: &mut Input) -> R<String> {
    let offset = here(i);

    if peek_char(i) != Some('\'') {
        return fail(i, offset, "bstring");
    }

    let checkpoint = i.checkpoint();
    i.next_token();
    let mut body = String::new();

    while let Some(c) = peek_char(i) {
        if c == '\'' {
            break;
        }

        body.push(c);
        i.next_token();
    }

    i.next_token();

    let kind = peek_char(i);
    let valid_bits = body
        .chars()
        .all(|c| c == '0' || c == '1' || c.is_whitespace());
    let valid_hex = body
        .chars()
        .all(|c| c.is_ascii_digit() || ('A'..='F').contains(&c) || c.is_whitespace());

    if kind == Some('B') && valid_bits {
        i.next_token();

        return Ok(format!(
            "0b{}",
            body.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
        ));
    }

    if kind == Some('H') && valid_hex {
        i.next_token();

        return Ok(format!(
            "0x{}",
            body.chars()
                .filter(|c| !c.is_whitespace())
                .collect::<String>()
                .to_lowercase()
        ));
    }

    i.reset(&checkpoint);

    fail(i, offset, "bstring")
}

/// `[+-]?\d+\.?\d*([eE][+-]?\d+)?`
fn real_number(i: &mut Input) -> R<Tok> {
    let offset = here(i);
    let checkpoint = i.checkpoint();
    let mut text = String::new();

    if let Some(c) = peek_char(i)
        && (c == '+' || c == '-')
    {
        text.push(c);
        i.next_token();
    }

    let digits = take_while_chars(i, |c| c.is_ascii_digit());

    if digits.is_empty() {
        i.reset(&checkpoint);

        return fail(i, offset, "number");
    }

    text.push_str(&digits);

    if peek_char(i) == Some('.') {
        text.push('.');
        i.next_token();
        text.push_str(&take_while_chars(i, |c| c.is_ascii_digit()));
    }

    let exponent = i.checkpoint();

    if let Some(c) = peek_char(i)
        && (c == 'e' || c == 'E')
    {
        let mut part = String::from(c);
        i.next_token();

        if let Some(sign) = peek_char(i)
            && (sign == '+' || sign == '-')
        {
            part.push(sign);
            i.next_token();
        }

        let exponent_digits = take_while_chars(i, |c| c.is_ascii_digit());

        if exponent_digits.is_empty() {
            i.reset(&exponent);
        } else {
            part.push_str(&exponent_digits);
            text.push_str(&part);
        }
    }

    if text.contains('.') {
        // float() of Python.
        let value: f64 = text.parse().unwrap_or(0.0);

        Ok(Tok::Float(value))
    } else if let Some(number) = text
        .strip_prefix('+')
        .map(str::to_string)
        .or(Some(text.clone()))
        .filter(|t| {
            t.trim_start_matches('-')
                .bytes()
                .all(|b| b.is_ascii_digit())
        })
    {
        Ok(Tok::Int(convert_int_text(&number)))
    } else {
        // An exponent without a fraction is not an integer, which the
        // Python parser fails to convert.
        i.reset(&checkpoint);

        fail(i, offset, "number")
    }
}

fn convert_int_text(text: &str) -> String {
    match convert_number(Data::Word(text.to_string())) {
        Data::Int(number) => number,
        _ => text.to_string(),
    }
}

/// The source text from `begin` to where the parser is, as the Python
/// parser takes it.
fn text_since(i: &mut Input, begin: usize) -> String {
    let end = {
        // Where the last token ended, and not where the next starts.
        i.current_token_start()
    };

    i.state.slice(begin, end).trim().to_string()
}

/// Reads the text of balanced braces, the way `nestedExpr` does: braces
/// inside quoted strings are not counted.
fn balanced_braces(i: &mut Input) -> R<()> {
    let offset = here(i);

    if peek_char(i) != Some('{') {
        return fail(i, offset, "{");
    }

    let checkpoint = i.checkpoint();
    let mut depth = 0usize;

    loop {
        match i.next_token() {
            Some('{') => depth += 1,
            Some('}') => {
                depth -= 1;

                if depth == 0 {
                    return Ok(());
                }
            }
            Some(quote @ ('"' | '\'')) => loop {
                match i.next_token() {
                    Some(c) if c == quote => break,
                    Some(_) => {}
                    None => {
                        i.reset(&checkpoint);

                        return fail(i, offset, "}");
                    }
                }
            },
            Some(_) => {}
            None => {
                i.reset(&checkpoint);

                return fail(i, offset, "}");
            }
        }
    }
}

/// A list of identifiers in braces, as of the bits of a bit string value.
fn identifier_list(i: &mut Input) -> R<Vec<String>> {
    lit(i, "{")?;
    let mut names = Vec::new();

    if lit(i, "}").is_ok() {
        return Ok(names);
    }

    loop {
        names.push(identifier(i)?.0);

        if lit(i, ",").is_err() {
            break;
        }
    }

    lit(i, "}")?;

    Ok(names)
}

/// `{ group, plane, row, cell }` or `{ column, row }`, a character of a
/// character string.
fn character_numbers(i: &mut Input) -> R<Vec<Data>> {
    lit(i, "{")?;
    let mut numbers = vec![Data::Int(number(i)?.to_string())];

    while lit(i, ",").is_ok() {
        numbers.push(Data::Int(number(i)?.to_string()));
    }

    lit(i, "}")?;

    if numbers.len() != 4 && numbers.len() != 2 {
        return backtrack();
    }

    Ok(numbers)
}

/// The components of an object identifier in braces: a name, a number or
/// a name with its number in parentheses.
fn oid_components(i: &mut Input) -> R<Vec<Vec<Data>>> {
    lit(i, "{")?;
    let mut groups = Vec::new();

    loop {
        let checkpoint = i.checkpoint();

        if let Ok(n) = number(i) {
            groups.push(vec![Data::Int(n.to_string())]);
        } else if let Ok((name, _)) = identifier(i) {
            let inner = i.checkpoint();

            if lit(i, "(").is_ok() {
                let value = match number(i) {
                    Ok(n) => n.to_string(),
                    Err(_) => {
                        i.reset(&checkpoint);
                        break;
                    }
                };

                commit(lit(i, ")"))?;
                groups.push(vec![Data::Word(name), Data::Int(value)]);
            } else {
                i.reset(&inner);
                groups.push(vec![Data::Word(name)]);
            }
        } else {
            i.reset(&checkpoint);
            break;
        }
    }

    lit(i, "}")?;

    if groups.is_empty() {
        return backtrack();
    }

    Ok(groups)
}

fn value_tokens(i: &mut Input) -> R<ValueTokens> {
    let begin = here(i);
    let tokens = value_tokens_inner(i)?;
    let text = text_since(i, begin);

    Ok(ValueTokens { text, ..tokens })
}

fn value_tokens_inner(i: &mut Input) -> R<ValueTokens> {
    let simple = |first: Tok| ValueTokens {
        first,
        second_is_colon: false,
        components: None,
        quadruple: None,
        text: String::new(),
    };
    let offset = here(i);

    // An open type value: a type, a colon and a value.
    if peek_char(i).is_some_and(|c| c.is_ascii_uppercase()) {
        let checkpoint = i.checkpoint();

        if parse_type(i).is_ok() && lit(i, ":").is_ok() {
            value_tokens(i)?;

            return Ok(ValueTokens {
                second_is_colon: true,
                ..simple(Tok::Other)
            });
        }

        i.reset(&checkpoint);
    }

    match peek_char(i) {
        Some('\'') => {
            let word = bit_or_hex_string(i)?;

            Ok(simple(Tok::BitString(BitStringValue::Word(word))))
        }
        Some('"') => Ok(simple(Tok::CString(quoted_string(i)?))),
        Some('{') => {
            let checkpoint = i.checkpoint();

            if let Ok(names) = identifier_list(i) {
                return Ok(simple(Tok::BitString(BitStringValue::Identifiers(names))));
            }

            i.reset(&checkpoint);

            if let Ok(numbers) = character_numbers(i) {
                return Ok(ValueTokens {
                    quadruple: Some(numbers),
                    ..simple(Tok::Brace)
                });
            }

            i.reset(&checkpoint);

            if let Ok(groups) = oid_components(i) {
                return Ok(ValueTokens {
                    components: Some(groups),
                    ..simple(Tok::Brace)
                });
            }

            i.reset(&checkpoint);
            balanced_braces(i)?;

            Ok(simple(Tok::Brace))
        }
        Some(c) if c.is_ascii_digit() || c == '-' || c == '+' => Ok(simple(real_number(i)?)),
        Some(c) if c.is_ascii_lowercase() => {
            let (name, _) = identifier(i)?;
            let checkpoint = i.checkpoint();

            if lit(i, ":").is_ok() {
                value_tokens(i)?;

                return Ok(ValueTokens {
                    second_is_colon: true,
                    ..simple(Tok::Other)
                });
            }

            i.reset(&checkpoint);

            Ok(simple(Tok::Word(name)))
        }
        _ => {
            if kw(i, "NULL").is_ok() {
                return Ok(simple(Tok::Null));
            }

            for word in [
                "TRUE",
                "FALSE",
                "PLUS-INFINITY",
                "MINUS-INFINITY",
                "NOT-A-NUMBER",
            ] {
                if kw(i, word).is_ok() {
                    return Ok(simple(Tok::Word(word.to_string())));
                }
            }

            if kw(i, "CONTAINING").is_ok() {
                commit(value_tokens(i))?;

                return Ok(simple(Tok::BitString(BitStringValue::Word(
                    "CONTAINING".to_string(),
                ))));
            }

            fail(i, offset, "value")
        }
    }
}

/// `convert_value()` of the Python parser.
fn convert_value(tokens: &ValueTokens, type_name: Option<&str>) -> Data {
    let is_value_text = |tokens: &ValueTokens| {
        !matches!(tokens.first, Tok::BitString(_))
            && (tokens.text.starts_with('{') || tokens.second_is_colon)
    };

    if type_name != Some("OBJECT IDENTIFIER") && is_value_text(tokens) {
        return convert_value_text(&tokens.text);
    }

    if type_name == Some("OBJECT IDENTIFIER") {
        if let Some(groups) = &tokens.components {
            return Data::List(
                groups
                    .iter()
                    .map(|group| {
                        if group.len() == 2 {
                            let Data::Word(name) = &group[0] else {
                                return Data::None;
                            };

                            Data::Pair(name.clone(), Box::new(group[1].clone()))
                        } else {
                            convert_number(group[0].clone())
                        }
                    })
                    .collect(),
            );
        }
    }

    match (type_name, &tokens.first) {
        (Some("INTEGER"), first) => convert_number(tok_data(first)),
        (Some("BOOLEAN"), Tok::Word(word)) if word == "TRUE" || word == "FALSE" => {
            Data::Bool(word == "TRUE")
        }
        (Some("BOOLEAN"), first) => convert_number(tok_data(first)),
        (_, Tok::BitString(value)) => match value {
            BitStringValue::Word(word) => Data::Word(word.clone()),
            BitStringValue::Identifiers(names) => {
                Data::List(names.iter().cloned().map(Data::Word).collect())
            }
        },
        (_, Tok::CString(text)) => Data::CString(text.clone()),
        (_, Tok::Word(word)) if word == "TRUE" || word == "FALSE" => Data::Bool(word == "TRUE"),
        (_, Tok::Brace) => Data::None,
        (_, Tok::Word(word)) => convert_number(Data::Word(word.clone())),
        (_, Tok::Int(number)) => Data::Int(number.clone()),
        (_, Tok::Float(value)) => Data::Float(*value),
        _ => Data::None,
    }
}

fn tok_data(tok: &Tok) -> Data {
    match tok {
        Tok::Word(word) => Data::Word(word.clone()),
        Tok::CString(text) => Data::CString(text.clone()),
        Tok::Int(number) => Data::Int(number.clone()),
        Tok::Float(value) => Data::Float(*value),
        Tok::BitString(BitStringValue::Word(word)) => Data::Word(word.clone()),
        _ => Data::None,
    }
}

// ----------------------------------------------------------------------
// Types

/// The keywords of the built-in types that have no syntax of their own.
fn builtin_name(i: &mut Input) -> R<&'static str> {
    const NAMES: [&str; 21] = [
        "BMPString",
        "GeneralString",
        "GraphicString",
        "IA5String",
        "ISO646String",
        "NumericString",
        "PrintableString",
        "TeletexString",
        "UTCTime",
        "GeneralizedTime",
        "T61String",
        "UniversalString",
        "UTF8String",
        "VideotexString",
        "VisibleString",
        "CHARACTER STRING",
        "BOOLEAN",
        "NULL",
        "REAL",
        "OCTET STRING",
        "OBJECT IDENTIFIER",
    ];

    for name in NAMES {
        let checkpoint = i.checkpoint();

        if kw(i, name).is_ok() {
            return Ok(name);
        }

        i.reset(&checkpoint);
    }

    backtrack()
}

/// The names of built-in types, which are written as references.
const BUILTIN_TYPE_NAMES: [&str; 30] = [
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
];

fn tag(i: &mut Input) -> R<Option<Tag>> {
    let checkpoint = i.checkpoint();

    if lit(i, "[").is_err() {
        i.reset(&checkpoint);

        return Ok(None);
    }

    let mut class = None;

    for name in ["UNIVERSAL", "APPLICATION", "PRIVATE"] {
        if kw(i, name).is_ok() {
            class = Some(name.to_string());
            break;
        }
    }

    let number = commit(class_number(i))?;
    commit(lit(i, "]"))?;

    let mode = if kw(i, "IMPLICIT").is_ok() {
        Some("IMPLICIT".to_string())
    } else if kw(i, "EXPLICIT").is_ok() {
        Some("EXPLICIT".to_string())
    } else {
        None
    };

    Ok(Some(Tag {
        number,
        tag_class: class,
        mode,
    }))
}

fn class_number(i: &mut Input) -> R<Value> {
    if let Ok(number) = number(i) {
        return Ok(Value::Number(number));
    }

    let (name, _) = identifier(i)?;

    Ok(Value::Identifier(name))
}

fn base_type(i: &mut Input) -> R<Type> {
    first(
        i,
        &[
            choice_type,
            integer_type,
            enumerated_type,
            sequence_of_type,
            sequence_type,
            set_of_type,
            set_type,
            bit_string_type,
            object_class_field_type,
            keyword_type,
            any_defined_by_type,
            referenced_type,
        ],
    )
}

fn keyword_type(i: &mut Input) -> R<Type> {
    let name = builtin_name(i)?;

    if name == "OBJECT IDENTIFIER" {
        // OBJECT IDENTIFIER ( word | word ), of which nothing is kept.
        let checkpoint = i.checkpoint();

        if lit(i, "(").is_ok() {
            let mut matched = word(i).is_ok();

            while matched && lit(i, "|").is_ok() {
                matched = word(i).is_ok();
            }

            if !(matched && lit(i, ")").is_ok()) {
                i.reset(&checkpoint);
            }
        } else {
            i.reset(&checkpoint);
        }
    }

    Ok(Type::new(Body::Builtin(name.to_string())))
}

/// `Word(printables, excludeChars=',(){}[].:=;"|')`
fn word(i: &mut Input) -> R<String> {
    let offset = here(i);
    let text = take_while_chars(i, |c| c.is_ascii_graphic() && !",(){}[].:=;\"|".contains(c));

    if text.is_empty() {
        return fail(i, offset, "word");
    }

    Ok(text)
}

fn any_defined_by_type(i: &mut Input) -> R<Type> {
    kw(i, "ANY DEFINED BY")?;
    let value = commit(word(i))?;

    Ok(Type::new(Body::AnyDefinedBy(value)))
}

/// `CLASS.&field`
fn object_class_field_type(i: &mut Input) -> R<Type> {
    let checkpoint = i.checkpoint();
    let offset = here(i);
    let class = take_while_chars(i, |c| {
        c.is_ascii_uppercase() || c.is_ascii_digit() || c == '-'
    });

    if !class.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || peek_char(i) != Some('.') {
        i.reset(&checkpoint);

        return fail(i, offset, "objectclassfieldtype");
    }

    i.next_token();

    if peek_char(i) != Some('&') {
        i.reset(&checkpoint);

        return fail(i, offset, "&");
    }

    i.next_token();
    let field = take_while_chars(i, |c| c.is_ascii_alphanumeric() || c == '-');

    if field.is_empty() {
        i.reset(&checkpoint);

        return fail(i, offset, "fieldname");
    }

    Ok(Type::new(Body::Referenced(format!("{class}.&{field}"))))
}

fn referenced_type(i: &mut Input) -> R<Type> {
    let (name, _) = type_reference(i)?;
    let mut type_ = if BUILTIN_TYPE_NAMES.contains(&name.as_str()) {
        Type::new(Body::Builtin(name))
    } else {
        Type::new(Body::Referenced(name))
    };

    // Actual parameters of a parameterized type.
    let checkpoint = i.checkpoint();

    if peek_char_ws(i) == Some('{') {
        match actual_parameters(i) {
            Ok(parameters) => {
                if !parameters.is_empty() {
                    type_.common.actual_parameters = Some(parameters);
                }
            }
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            Err(error) => return Err(error),
        }
    }

    Ok(type_)
}

/// `{ actual, ... }` after the name of a parameterized type.
fn actual_parameters(i: &mut Input) -> R<Vec<Value>> {
    lit(i, "{")?;
    let mut parameters = Vec::new();

    loop {
        parameters.push(actual_parameter(i)?);

        if lit(i, ",").is_err() {
            break;
        }
    }

    lit(i, "}")?;

    Ok(parameters)
}

fn actual_parameter(i: &mut Input) -> R<Value> {
    let checkpoint = i.checkpoint();

    if peek_char_ws(i).is_some_and(|c| c.is_ascii_uppercase()) {
        match parse_type(i) {
            Ok(type_) => return Ok(Value::Type(Box::new(type_))),
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            Err(error) => return Err(error),
        }
    }

    if peek_char_ws(i) == Some('{') {
        let checkpoint = i.checkpoint();

        // A list of identifiers is taken for the value it is, which
        // names the objects of an object set if its parameter is one.
        if let Ok(names) = identifier_list(i) {
            return Ok(Value::List(
                names.into_iter().map(Value::Identifier).collect(),
            ));
        }

        i.reset(&checkpoint);

        if let Ok(members) = object_set_members(i) {
            return Ok(Value::ObjectSet(members));
        }

        i.reset(&checkpoint);

        return match object(i)? {
            RawData::Fields(fields) => Ok(Value::Object(fields)),
            RawData::DefinedSyntax(text) => Ok(Value::UnresolvedObject(text)),
            RawData::Data(_) => Ok(Value::UnsupportedValue),
        };
    }

    let tokens = value_tokens(i)?;
    let data = tok_data(&tokens.first);

    Ok(value_from_data(&data, Where::Value, None, false))
}

fn named_number(i: &mut Input) -> R<NamedNumber> {
    let (name, _) = identifier(i)?;
    lit(i, "(")?;
    let checkpoint = i.checkpoint();
    let negative = lit(i, "-").is_ok();
    let number = match number(i) {
        Ok(number) => {
            let text = if negative {
                format!("-{number}")
            } else {
                number.to_string()
            };

            Value::Number(text.parse::<Number>().unwrap())
        }
        Err(_) => {
            i.reset(&checkpoint);
            let (name, _) = identifier(i)?;

            Value::Identifier(name)
        }
    };
    lit(i, ")")?;

    Ok(NamedNumber { name, number })
}

fn integer_type(i: &mut Input) -> R<Type> {
    kw(i, "INTEGER")?;
    let mut named_numbers = None;
    let checkpoint = i.checkpoint();

    if lit(i, "{").is_ok() {
        let mut items = vec![named_number(i)?];

        while lit(i, ",").is_ok() {
            items.push(named_number(i)?);
        }

        lit(i, "}")?;
        named_numbers = Some(items);
    } else {
        i.reset(&checkpoint);
    }

    Ok(Type::new(Body::Integer(named_numbers)))
}

fn bit_string_type(i: &mut Input) -> R<Type> {
    kw(i, "BIT STRING")?;
    let mut named_bits = None;
    let checkpoint = i.checkpoint();

    if lit(i, "{").is_ok() {
        let mut items = Vec::new();

        loop {
            let (name, _) = identifier(i)?;
            lit(i, "(")?;
            let bit = class_number(i)?;
            lit(i, ")")?;
            items.push(NamedNumber { name, number: bit });

            if lit(i, ",").is_err() {
                break;
            }
        }

        lit(i, "}")?;
        named_bits = Some(items);
    } else {
        i.reset(&checkpoint);
    }

    Ok(Type::new(Body::BitString(named_bits)))
}

/// The number of an item of an enumeration with its number given: a
/// number, which may be negative, or a value by reference.
fn enumeration_number(i: &mut Input) -> R<Data> {
    let negative = lit(i, "-").is_ok();

    if let Ok(n) = number(i) {
        let digits = if negative {
            format!("-{n}")
        } else {
            n.to_string()
        };

        return Ok(convert_number(Data::Word(digits)));
    }

    if negative {
        let offset = here(i);

        return fail(i, offset, "number");
    }

    let (name, _) = identifier(i)?;

    Ok(Data::Word(name))
}

fn enumerated_type(i: &mut Input) -> R<Type> {
    let begin = here(i);
    kw(i, "ENUMERATED")?;
    commit(lit(i, "{"))?;

    // The items before the extension marker, and those after it.
    let mut root: Vec<(String, Option<Data>)> = Vec::new();
    let mut additional: Vec<(String, Option<Data>)> = Vec::new();
    let mut has_extension = false;

    loop {
        if !has_extension && lit(i, "...").is_ok() {
            has_extension = true;

            if lit(i, ",").is_err() {
                break;
            }

            continue;
        }

        let (name, _) = commit(identifier(i))?;
        let checkpoint = i.checkpoint();
        let mut explicit = None;

        if lit(i, "(").is_ok() {
            explicit = Some(commit(enumeration_number(i))?);
            commit(lit(i, ")"))?;
        } else {
            i.reset(&checkpoint);
        }

        if has_extension {
            additional.push((name, explicit));
        } else {
            root.push((name, explicit));
        }

        if lit(i, ",").is_err() {
            break;
        }
    }

    commit(lit(i, "}"))?;

    let line = i.state.line_of(begin);
    let duplicated = |number: &Data| {
        format!(
            "Duplicated ENUMERATED number {} at line {}.",
            match number {
                Data::Int(digits) => digits.clone(),
                Data::Word(word) => word.clone(),
                other => format!("{other:?}"),
            },
            line
        )
    };

    let mut number: i64 = 0;
    let mut values: Vec<EnumItem> = Vec::new();
    let mut used: Vec<Data> = Vec::new();

    for (_, explicit) in &root {
        if let Some(explicit) = explicit {
            if used.contains(explicit) {
                return conversion_error(i, duplicated(explicit));
            }

            used.push(explicit.clone());
        }
    }

    for (name, explicit) in &root {
        let item = match explicit {
            Some(explicit) => explicit.clone(),
            None => {
                while used.contains(&crate::data::int(number)) {
                    number += 1;
                }

                used.push(crate::data::int(number));
                number += 1;

                crate::data::int(number - 1)
            }
        };

        values.push(EnumItem::Number(NamedNumber {
            name: name.clone(),
            number: crate::data::number_from_data(&item),
        }));
    }

    if has_extension {
        values.push(EnumItem::ExtensionMarker);

        for (_, explicit) in &additional {
            if let Some(explicit) = explicit {
                if used.contains(explicit) {
                    return conversion_error(i, duplicated(explicit));
                }

                used.push(explicit.clone());
            }
        }

        for (name, explicit) in &additional {
            let item = match explicit {
                Some(explicit) => {
                    if let Data::Int(digits) = explicit {
                        number = digits.parse().unwrap_or(number);
                    }

                    explicit.clone()
                }
                None => {
                    if used.contains(&crate::data::int(number)) {
                        return conversion_error(i, duplicated(&crate::data::int(number)));
                    }

                    used.push(crate::data::int(number));

                    crate::data::int(number)
                }
            };

            values.push(EnumItem::Number(NamedNumber {
                name: name.clone(),
                number: crate::data::number_from_data(&item),
            }));
            number += 1;
        }
    }

    Ok(Type::new(Body::Enumerated(values)))
}

/// Tries a parser, and gives `None` and does not move if it does not
/// match, as `Optional` does.
fn optional<T>(i: &mut Input, parser: impl FnOnce(&mut Input) -> R<T>) -> R<Option<T>> {
    let checkpoint = i.checkpoint();

    match parser(i) {
        Ok(value) => Ok(Some(value)),
        Err(ErrMode::Backtrack(_)) => {
            i.reset(&checkpoint);

            Ok(None)
        }
        Err(error) => Err(error),
    }
}

/// `item, item, ...`: a comma that is not followed by an item is not
/// part of the list.
fn delimited<T>(i: &mut Input, item: fn(&mut Input) -> R<T>) -> R<Vec<T>> {
    let mut items = vec![item(i)?];

    loop {
        let checkpoint = i.checkpoint();

        if lit(i, ",").is_err() {
            break;
        }

        match item(i) {
            Ok(value) => items.push(value),
            Err(ErrMode::Backtrack(_)) => {
                i.reset(&checkpoint);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(items)
}

/// A member of a sequence or a set, or `COMPONENTS OF type`.
fn component_type(i: &mut Input) -> R<MemberItem> {
    if kw(i, "COMPONENTS OF").is_ok() {
        let type_ = commit(parse_type(i))?;

        return match type_.body {
            Body::Referenced(name) | Body::Builtin(name) => Ok(MemberItem::ComponentsOf(name)),
            _ => Ok(MemberItem::ExtensionMarker),
        };
    }

    Ok(MemberItem::Member(member(i)?))
}

/// `[[ 2: member, member ]]`
fn extension_addition_group(i: &mut Input, item: fn(&mut Input) -> R<MemberItem>) -> R<MemberItem> {
    lit(i, "[[")?;

    // The version number.
    let checkpoint = i.checkpoint();

    if !(number(i).is_ok() && lit(i, ":").is_ok()) {
        i.reset(&checkpoint);
    }

    let members = delimited(i, item)?;
    lit(i, "]]")?;

    Ok(MemberItem::Group(members))
}

fn extension_addition(i: &mut Input) -> R<MemberItem> {
    let checkpoint = i.checkpoint();

    match component_type(i) {
        Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
        other => return other,
    }

    extension_addition_group(i, component_type)
}

/// The `...` of an extension.
fn extension_marker(i: &mut Input) -> R<MemberItem> {
    lit(i, "...")?;

    Ok(MemberItem::ExtensionMarker)
}

/// `, ...`
fn comma_extension_marker(i: &mut Input) -> R<MemberItem> {
    lit(i, ",")?;

    extension_marker(i)
}

/// What follows the extension marker of a sequence or a set: the additions,
/// and then either the end of the extension and the members after it or an
/// optional marker.
fn extension_part(i: &mut Input, items: &mut Vec<MemberItem>) -> R<()> {
    items.push(extension_marker(i)?);

    if let Some(additions) = optional(i, |i| {
        lit(i, ",")?;

        delimited(i, extension_addition)
    })? {
        items.extend(additions);
    }

    // `, ... , members` or an optional `, ...`.
    let checkpoint = i.checkpoint();
    let after = optional(i, |i| {
        let marker = comma_extension_marker(i)?;
        lit(i, ",")?;
        let members = delimited(i, component_type)?;

        Ok((marker, members))
    })?;

    match after {
        Some((marker, members)) => {
            items.push(marker);
            items.extend(members);
        }
        None => {
            i.reset(&checkpoint);

            if let Some(marker) = optional(i, comma_extension_marker)? {
                items.push(marker);
            }
        }
    }

    Ok(())
}

/// The contents of the braces of a sequence or a set.
fn component_type_lists(i: &mut Input) -> R<Vec<MemberItem>> {
    let mut items = Vec::new();

    if let Some(root) = optional(i, |i| delimited(i, component_type))? {
        items.extend(root);

        if let Some(extension) = optional(i, |i| {
            lit(i, ",")?;
            let mut extension = Vec::new();
            extension_part(i, &mut extension)?;

            Ok(extension)
        })? {
            items.extend(extension);
        }

        return Ok(items);
    }

    extension_part(i, &mut items)?;

    Ok(items)
}

/// The contents of the braces of a choice.
fn alternative_type_lists(i: &mut Input) -> R<Vec<MemberItem>> {
    let mut items: Vec<MemberItem> = delimited(i, alternative)?;

    if let Some(extension) = optional(i, |i| {
        lit(i, ",")?;
        let mut extension = vec![extension_marker(i)?];

        if let Some(additions) = optional(i, |i| {
            lit(i, ",")?;

            delimited(i, extension_alternative)
        })? {
            extension.extend(additions);
        }

        if let Some(marker) = optional(i, comma_extension_marker)? {
            extension.push(marker);
        }

        Ok(extension)
    })? {
        items.extend(extension);
    }

    Ok(items)
}

fn alternative(i: &mut Input) -> R<MemberItem> {
    Ok(MemberItem::Member(member(i)?))
}

fn extension_alternative(i: &mut Input) -> R<MemberItem> {
    if lit(i, "[[").is_ok() {
        let checkpoint = i.checkpoint();

        if !(number(i).is_ok() && lit(i, ":").is_ok()) {
            i.reset(&checkpoint);
        }

        let members = commit(delimited(i, alternative))?;
        commit(lit(i, "]]"))?;

        return Ok(MemberItem::Group(members));
    }

    alternative(i)
}

fn member(i: &mut Input) -> R<Member> {
    let (name, offset) = identifier(i)?;
    let tag = commit(tag(i))?;
    let mut type_ = commit(parse_type(i))?;

    if let Some(tag) = tag {
        type_.common.tag = Some(tag);
    }

    let mut optional = false;
    let mut default = None;
    let mut raw_default = None;

    if kw(i, "OPTIONAL").is_ok() {
        optional = true;
    } else if kw(i, "DEFAULT").is_ok() {
        let tokens = value_tokens(i)?;
        let type_name = type_.type_name();
        let data = convert_value(&tokens, Some(&type_name));
        default = Some(value_from_data(
            &data,
            Where::Value,
            Some(&type_name),
            false,
        ));
        raw_default = Some((data, type_name));
    }

    Ok(Member {
        name,
        type_,
        optional,
        default,
        raw_default,
        location: offset_location(i, offset),
    })
}

fn offset_location(i: &Input, offset: usize) -> Option<crate::ast::Location> {
    i.state.locator.as_ref().map(|_| i.state.locate(offset))
}

/// `{ members }` of a sequence or a set.
fn braced_members(i: &mut Input, committed: bool) -> R<Vec<MemberItem>> {
    if committed {
        commit(lit(i, "{"))?;
    } else {
        lit(i, "{")?;
    }

    let members = optional(i, |i| {
        let checkpoint = i.checkpoint();

        match component_type_lists(i) {
            Err(ErrMode::Backtrack(_)) => {
                i.reset(&checkpoint);
                let mut items = Vec::new();
                items.push(extension_marker(i)?);

                if let Some(marker) = optional(i, comma_extension_marker)? {
                    items.push(marker);
                }

                Ok(items)
            }
            other => other,
        }
    })?
    .unwrap_or_default();

    commit(lit(i, "}"))?;

    Ok(members)
}

fn sequence_type(i: &mut Input) -> R<Type> {
    kw(i, "SEQUENCE")?;

    Ok(Type::new(Body::Sequence(braced_members(i, true)?)))
}

fn set_type(i: &mut Input) -> R<Type> {
    kw(i, "SET")?;

    Ok(Type::new(Body::Set(braced_members(i, false)?)))
}

fn choice_type(i: &mut Input) -> R<Type> {
    kw(i, "CHOICE")?;
    commit(lit(i, "{"))?;
    let members = commit(alternative_type_lists(i))?;
    commit(lit(i, "}"))?;

    Ok(Type::new(Body::Choice(members)))
}

type OfType = (Vec<RawConstraint>, Option<String>, Type);

fn of_type(i: &mut Input, keyword: &'static str) -> R<OfType> {
    kw(i, keyword)?;
    let mut constraints = Vec::new();

    // SIZE (1..2) or (SIZE (1..2))
    if kw(i, "SIZE").is_ok() {
        let size = commit(constraint(i))?;

        constraints.push(RawConstraint::Specs {
            root: RawSet::Element(RawElement::Size(Box::new(size))),
            extensible: false,
            additions: None,
        });
    } else {
        let checkpoint = i.checkpoint();

        match constraint(i) {
            Ok(constraint) => constraints.push(constraint),
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            Err(error) => return Err(error),
        }
    }

    kw(i, "OF")?;
    let element_name = {
        let checkpoint = i.checkpoint();

        match identifier(i) {
            Ok((name, _)) => Some(name),
            Err(_) => {
                i.reset(&checkpoint);
                None
            }
        }
    };
    let tag = commit(tag(i))?;
    let mut element = commit(parse_type(i))?;

    if let Some(tag) = tag {
        element.common.tag = Some(tag);
    }

    Ok((constraints, element_name, element))
}

fn sequence_of_type(i: &mut Input) -> R<Type> {
    let (constraints, name, element) = of_type(i, "SEQUENCE")?;
    let mut type_ = Type::new(Body::SequenceOf(Box::new(element), name));
    apply_of_constraints(&mut type_, items_of(&constraints));
    type_.common.constraints = tree_of(&constraints);

    Ok(type_)
}

/// The size of a `SEQUENCE OF` or a `SET OF` is that of its first
/// constraint, and an extension marker after it.
fn apply_of_constraints(type_: &mut Type, items: Vec<Item>) {
    if let Some(Item::Size(values)) = items.first() {
        let mut size = values.clone();

        if matches!(items.get(1), Some(Item::Ellipsis)) {
            size.push(Value::ExtensionMarker);
        }

        type_.common.size = Some(size);
    }
}

fn set_of_type(i: &mut Input) -> R<Type> {
    let (constraints, name, element) = of_type(i, "SET")?;
    let mut type_ = Type::new(Body::SetOf(Box::new(element), name));
    apply_of_constraints(&mut type_, items_of(&constraints));
    type_.common.constraints = tree_of(&constraints);

    Ok(type_)
}

// ----------------------------------------------------------------------
// Constraints

/// A bound of a range and how far the range goes, as the tokens of a
/// range tell it.
#[derive(Debug, Clone)]
pub struct RangeItem {
    lower: Data,
    upper: Data,
    lower_excluded: bool,
    upper_excluded: bool,
}

/// What a constraint is made of, once the parentheses are gone and the
/// unions and the intersections are flattened, as the Python parser does.
#[derive(Debug, Clone)]
pub enum Item {
    Ellipsis,
    Size(Vec<Value>),
    From(Vec<Value>),
    WithComponents(Vec<Value>),
    WithComponent(ElementConstraint),
    Single(Data),
    Range(RangeItem),
    Type(Type),
    Containing(Type),
    /// Constraints in parentheses within a constraint, of which a
    /// component has the constraints, and a type has nothing.
    Nested(Vec<Item>),
    /// `({Set})` or `({Set}{@id})`, of the field of an object class.
    Table(TableConstraint),
    /// Anything in braces, which takes all the values of a constraint
    /// with it.
    Junk,
    Ignored,
}

fn bound_value(data: &Data, excluded: bool, where_: Where) -> Value {
    let value = value_from_data(data, where_, None, false);

    if excluded {
        Value::Excluded(Box::new(value))
    } else {
        value
    }
}

fn range_value(range: &RangeItem, where_: Where) -> Value {
    Value::Range(
        Box::new(bound_value(&range.lower, range.lower_excluded, where_)),
        Box::new(bound_value(&range.upper, range.upper_excluded, where_)),
    )
}

/// A constraint as it is parsed. The items the compiler takes are made of
/// it, as the Python parser made them, and so is the tree of the
/// constraint in a syntax tree, as it is written.
#[derive(Debug, Clone)]
pub enum RawConstraint {
    Specs {
        root: RawSet,
        extensible: bool,
        additions: Option<RawSet>,
    },
    Table(TableConstraint),
    /// A table constraint of a set of objects that is not given by name.
    Unsupported,
    UserDefined,
    Contents {
        type_: Option<Type>,
        encoded_by: bool,
    },
}

#[derive(Debug, Clone)]
pub enum RawSet {
    Union(Vec<RawSet>),
    Intersection(Vec<RawSet>),
    Except(Box<RawSet>, Box<RawSet>),
    All(Box<RawSet>),
    /// A set in parentheses.
    Nested(Box<RawSet>),
    Element(RawElement),
    /// An object in braces, as an element of a set of objects.
    Object,
}

#[derive(Debug, Clone)]
pub enum RawElement {
    Ellipsis,
    Size(Box<RawConstraint>),
    From(Box<RawConstraint>),
    WithComponents {
        partial: bool,
        components: Vec<RawComponent>,
    },
    WithComponent(Box<RawConstraint>),
    Single(Data),
    /// A value in braces.
    Braced,
    /// `NULL`, which the Python parser took for the type.
    Null,
    Range(RangeItem),
    Type {
        type_: Type,
        includes: bool,
    },
    Pattern(Data),
}

#[derive(Debug, Clone)]
pub struct RawComponent {
    name: String,
    constraints: Vec<RawConstraint>,
    presence: Option<String>,
}

/// The items of the constraints of a type, one after the other.
fn items_of(constraints: &[RawConstraint]) -> Vec<Item> {
    constraints.iter().flat_map(RawConstraint::items).collect()
}

/// The tree of the constraints of a type, if it has any.
fn tree_of(constraints: &[RawConstraint]) -> Option<Vec<Constraint>> {
    if constraints.is_empty() {
        None
    } else {
        Some(constraints.iter().map(RawConstraint::to_ast).collect())
    }
}

/// The values of a size, as the Python parser made them of its items.
fn size_values(items: &[Item]) -> Vec<Value> {
    items
        .iter()
        .map(|item| match item {
            Item::Ellipsis => Value::ExtensionMarker,
            Item::Range(range) => range_value(range, Where::Constraint),
            Item::Single(data) => value_from_data(
                &convert_number(data.clone()),
                Where::Constraint,
                None,
                false,
            ),
            _ => Value::UnsupportedValue,
        })
        .collect()
}

/// The values of a permitted alphabet, as the Python parser made them of
/// its items.
fn alphabet_values(items: Vec<Item>) -> Vec<Value> {
    let mut values = Vec::new();

    for item in items {
        match item {
            Item::Single(Data::CString(text)) | Item::Single(Data::Word(text)) => {
                for c in text.chars() {
                    values.push(Value::Range(
                        Box::new(Value::Str(c.to_string())),
                        Box::new(Value::Str(c.to_string())),
                    ));
                }
            }
            Item::Range(range) => values.push(range_value(&range, Where::PermittedAlphabet)),
            // An extensible permitted alphabet is not PER-visible (X.691
            // 10.3.11), so the marker is kept.
            Item::Ellipsis => values.push(Value::ExtensionMarker),
            _ => {}
        }
    }

    values
}

impl RawConstraint {
    fn items(&self) -> Vec<Item> {
        match self {
            RawConstraint::Specs {
                root,
                extensible,
                additions,
            } => {
                let mut items = root.items();

                if *extensible {
                    items.push(Item::Ellipsis);

                    if let Some(additions) = additions {
                        items.extend(additions.items());
                    }
                }

                items
            }
            RawConstraint::Table(table) => vec![Item::Table(table.clone())],
            RawConstraint::Unsupported | RawConstraint::UserDefined => vec![Item::Junk],
            RawConstraint::Contents {
                type_: Some(type_), ..
            } => vec![Item::Containing(type_.clone())],
            RawConstraint::Contents { type_: None, .. } => vec![Item::Ignored],
        }
    }

    fn to_ast(&self) -> Constraint {
        match self {
            RawConstraint::Specs {
                root,
                extensible,
                additions,
            } => Constraint::ElementSetSpecs {
                root: root.to_ast(),
                extensible: *extensible,
                additions: additions.as_ref().map(RawSet::to_ast),
            },
            RawConstraint::Table(table) => Constraint::Table(table.clone()),
            RawConstraint::Unsupported => Constraint::Unsupported,
            RawConstraint::UserDefined => Constraint::UserDefined,
            RawConstraint::Contents { type_, encoded_by } => Constraint::Contents {
                type_: type_.clone().map(Box::new),
                encoded_by: encoded_by.then_some(Value::UnsupportedValue),
            },
        }
    }
}

impl RawSet {
    fn items(&self) -> Vec<Item> {
        match self {
            RawSet::Union(sets) | RawSet::Intersection(sets) => {
                sets.iter().flat_map(RawSet::items).collect()
            }
            // What is left out is not looked at.
            RawSet::Except(set, _) => set.items(),
            RawSet::All(_) => vec![Item::Ignored],
            RawSet::Nested(set) => vec![Item::Nested(set.items())],
            RawSet::Element(element) => vec![element.item()],
            RawSet::Object => vec![Item::Junk],
        }
    }

    fn to_ast(&self) -> ElementSet {
        match self {
            RawSet::Union(sets) => ElementSet::Union(sets.iter().map(RawSet::to_ast).collect()),
            RawSet::Intersection(sets) => {
                ElementSet::Intersection(sets.iter().map(RawSet::to_ast).collect())
            }
            RawSet::Except(set, exclusions) => {
                ElementSet::Except(Box::new(set.to_ast()), Box::new(exclusions.to_ast()))
            }
            RawSet::All(exclusions) => ElementSet::All(Box::new(exclusions.to_ast())),
            RawSet::Nested(set) => set.to_ast(),
            RawSet::Element(element) => ElementSet::Element(element.to_ast()),
            RawSet::Object => ElementSet::Element(Element::Value(Value::UnsupportedValue)),
        }
    }
}

impl RawElement {
    fn item(&self) -> Item {
        match self {
            RawElement::Ellipsis => Item::Ellipsis,
            RawElement::Size(constraint) => Item::Size(size_values(&constraint.items())),
            RawElement::From(constraint) => Item::From(alphabet_values(constraint.items())),
            RawElement::WithComponents {
                partial,
                components,
            } => {
                let mut values = Vec::new();

                if *partial {
                    values.push(Value::ExtensionMarker);
                }

                for component in components {
                    values.push(Value::Component(Box::new(ComponentConstraint {
                        name: component.name.clone(),
                        presence: component.presence.clone(),
                        constraint: inner_constraint(items_of(&component.constraints)),
                    })));
                }

                Item::WithComponents(values)
            }
            RawElement::WithComponent(constraint) => {
                Item::WithComponent(inner_constraint(constraint.items()))
            }
            RawElement::Single(data) => Item::Single(data.clone()),
            RawElement::Braced => Item::Junk,
            RawElement::Null => Item::Type(Type::new(Body::Builtin("NULL".to_string()))),
            RawElement::Range(range) => Item::Range(range.clone()),
            RawElement::Type { type_, .. } => Item::Type(type_.clone()),
            RawElement::Pattern(_) => Item::Ignored,
        }
    }

    /// The element as it is written. A word is a reference to a value, also
    /// in a permitted alphabet.
    fn to_ast(&self) -> Element {
        match self {
            RawElement::Ellipsis => Element::Value(Value::ExtensionMarker),
            RawElement::Size(constraint) => Element::Size(Box::new(constraint.to_ast())),
            RawElement::From(constraint) => {
                Element::PermittedAlphabet(Box::new(constraint.to_ast()))
            }
            RawElement::WithComponents {
                partial,
                components,
            } => Element::InnerTypes {
                partial: *partial,
                components: components
                    .iter()
                    .map(|component| NamedConstraint {
                        name: component.name.clone(),
                        constraints: tree_of(&component.constraints),
                        presence: component.presence.clone(),
                    })
                    .collect(),
            },
            RawElement::WithComponent(constraint) => {
                Element::InnerType(Box::new(constraint.to_ast()))
            }
            RawElement::Single(data) => {
                Element::Value(value_from_data(data, Where::Constraint, None, false))
            }
            RawElement::Braced => Element::Value(Value::UnsupportedValue),
            RawElement::Null => Element::Value(Value::Null),
            RawElement::Range(range) => Element::Value(range_value(range, Where::Constraint)),
            RawElement::Type { type_, includes } => Element::ContainedSubtype {
                type_: Box::new(type_.clone()),
                includes: *includes,
            },
            RawElement::Pattern(data) => {
                Element::Pattern(value_from_data(data, Where::Constraint, None, false))
            }
        }
    }
}

/// A constraint in parentheses.
fn constraint(i: &mut Input) -> R<RawConstraint> {
    lit(i, "(")?;
    let constraint = commit(constraint_spec(i))?;
    commit(lit(i, ")"))?;

    Ok(constraint)
}

fn constraint_spec(i: &mut Input) -> R<RawConstraint> {
    let checkpoint = i.checkpoint();

    if let Some(constraint) = general_constraint(i)? {
        return Ok(constraint);
    }

    i.reset(&checkpoint);

    element_set_specs(i)
}

/// A user defined constraint, a table constraint or a contents
/// constraint (X.682 9, 10, 11).
fn general_constraint(i: &mut Input) -> R<Option<RawConstraint>> {
    if kw(i, "CONSTRAINED BY").is_ok() {
        commit(balanced_braces(i))?;

        return Ok(Some(RawConstraint::UserDefined));
    }

    if kw(i, "CONTAINING").is_ok() {
        let type_ = commit(parse_type(i))?;
        let mut encoded_by = false;

        if kw(i, "ENCODED BY").is_ok() || kw(i, "ENCODED_BY").is_ok() {
            commit(value_tokens(i))?;
            encoded_by = true;
        }

        return Ok(Some(RawConstraint::Contents {
            type_: Some(type_),
            encoded_by,
        }));
    }

    if kw(i, "ENCODED BY").is_ok() || kw(i, "ENCODED_BY").is_ok() {
        commit(value_tokens(i))?;

        return Ok(Some(RawConstraint::Contents {
            type_: None,
            encoded_by: true,
        }));
    }

    if peek_char_ws(i) == Some('{') {
        let checkpoint = i.checkpoint();

        if let Some(table) = table_constraint(i)? {
            return Ok(Some(RawConstraint::Table(table)));
        }

        i.reset(&checkpoint);

        if object_set_members(i).is_ok() {
            // A component relation constraint has a second pair of
            // braces.
            let inner = i.checkpoint();

            if lit(i, "{").is_ok() {
                i.reset(&inner);
                commit(balanced_braces(i))?;
            } else {
                i.reset(&inner);
            }

            return Ok(Some(RawConstraint::Unsupported));
        }

        i.reset(&checkpoint);
    }

    Ok(None)
}

/// `{Set}` or `{Set}{@id, @id}`.
fn table_constraint(i: &mut Input) -> R<Option<TableConstraint>> {
    let checkpoint = i.checkpoint();

    if lit(i, "{").is_err() {
        return Ok(None);
    }

    let Ok((object_set, _)) = type_reference(i) else {
        i.reset(&checkpoint);

        return Ok(None);
    };

    if lit(i, "}").is_err() {
        i.reset(&checkpoint);

        return Ok(None);
    }

    let inner = i.checkpoint();

    if lit(i, "{").is_err() {
        i.reset(&inner);

        return Ok(Some(TableConstraint {
            object_set,
            component_ids: None,
        }));
    }

    let mut ids = Vec::new();

    loop {
        commit(lit(i, "@"))?;
        let mut id = String::new();

        while peek_char(i) == Some('.') {
            id.push('.');
            i.next_token();
        }

        id.push_str(&commit(identifier(i))?.0);
        ids.push(id);

        if lit(i, ",").is_err() {
            break;
        }
    }

    commit(lit(i, "}"))?;

    Ok(Some(TableConstraint {
        object_set,
        component_ids: Some(ids),
    }))
}

fn peek_char_ws(i: &mut Input) -> Option<char> {
    ws(i);

    peek_char(i)
}

fn element_set_specs(i: &mut Input) -> R<RawConstraint> {
    let root = element_set_spec(i)?;
    let mut extensible = false;
    let mut additions = None;

    if lit(i, ",").is_ok() {
        commit(lit(i, "..."))?;
        extensible = true;

        if lit(i, ",").is_ok() {
            additions = Some(commit(element_set_spec(i))?);
        }
    }

    Ok(RawConstraint::Specs {
        root,
        extensible,
        additions,
    })
}

/// `ALL EXCEPT` a set, or unions of intersections of elements, of which an
/// intersection binds closer than a union and an exclusion closer than
/// both (X.680 50.1).
fn element_set_spec(i: &mut Input) -> R<RawSet> {
    if kw(i, "ALL").is_ok() {
        commit(kw(i, "EXCEPT"))?;
        let exclusions = commit(elements(i))?;

        return Ok(RawSet::All(Box::new(exclusions)));
    }

    unions(i)
}

/// Sets with a mark between them, of which a mark that is not followed by
/// a set is not part of them.
fn marked_sets(
    i: &mut Input,
    marks: &[&'static str],
    keyword: &'static str,
    set: fn(&mut Input) -> R<RawSet>,
) -> R<Vec<RawSet>> {
    let mut sets = vec![set(i)?];

    loop {
        let checkpoint = i.checkpoint();

        if marks.iter().any(|mark| lit(i, mark).is_ok()) || kw(i, keyword).is_ok() {
            match set(i) {
                Ok(more) => sets.push(more),
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&checkpoint);
                    break;
                }
                Err(error) => return Err(error),
            }
        } else {
            i.reset(&checkpoint);
            break;
        }
    }

    Ok(sets)
}

fn unions(i: &mut Input) -> R<RawSet> {
    let mut sets = marked_sets(i, &["|"], "UNION", intersections)?;

    Ok(if sets.len() == 1 {
        sets.pop().unwrap()
    } else {
        RawSet::Union(sets)
    })
}

fn intersections(i: &mut Input) -> R<RawSet> {
    let mut sets = marked_sets(i, &["^"], "INTERSECTION", intersection_elements)?;

    Ok(if sets.len() == 1 {
        sets.pop().unwrap()
    } else {
        RawSet::Intersection(sets)
    })
}

fn intersection_elements(i: &mut Input) -> R<RawSet> {
    let set = elements(i)?;
    let checkpoint = i.checkpoint();

    if kw(i, "EXCEPT").is_ok() {
        match elements(i) {
            Ok(exclusions) => return Ok(RawSet::Except(Box::new(set), Box::new(exclusions))),
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            Err(error) => return Err(error),
        }
    } else {
        i.reset(&checkpoint);
    }

    Ok(set)
}

fn elements(i: &mut Input) -> R<RawSet> {
    let checkpoint = i.checkpoint();

    if lit(i, "(").is_ok() {
        let nested = commit(element_set_spec(i))?;
        commit(lit(i, ")"))?;

        return Ok(RawSet::Nested(Box::new(nested)));
    }

    i.reset(&checkpoint);

    let before = i.checkpoint();

    match subtype_element(i) {
        Ok(element) => return Ok(RawSet::Element(element)),
        Err(ErrMode::Backtrack(_)) => i.reset(&before),
        Err(error) => return Err(error),
    }

    // An object, as an element of an object set.
    if peek_char_ws(i) == Some('{') {
        balanced_braces(i)?;

        return Ok(RawSet::Object);
    }

    let offset = here(i);

    fail(i, offset, "constraint")
}

/// A constraint in parentheses within a constraint, as that of a size or
/// of an inner constraint.
fn nested_constraint(i: &mut Input) -> R<RawConstraint> {
    constraint(i)
}

fn subtype_element(i: &mut Input) -> R<RawElement> {
    if lit(i, "...").is_ok() {
        return Ok(RawElement::Ellipsis);
    }

    if kw(i, "SIZE").is_ok() {
        return Ok(RawElement::Size(Box::new(commit(nested_constraint(i))?)));
    }

    if kw(i, "FROM").is_ok() {
        return Ok(RawElement::From(Box::new(commit(nested_constraint(i))?)));
    }

    if let Some(range) = value_range(i)? {
        return Ok(RawElement::Range(range));
    }

    if kw(i, "WITH COMPONENTS").is_ok() {
        let (partial, components) = commit(components_specification(i))?;

        return Ok(RawElement::WithComponents {
            partial,
            components,
        });
    }

    if kw(i, "WITH COMPONENT").is_ok() {
        return Ok(RawElement::WithComponent(Box::new(commit(
            nested_constraint(i),
        )?)));
    }

    let checkpoint = i.checkpoint();

    if let Ok(tokens) = value_tokens(i) {
        if !tokens.second_is_colon {
            return Ok(match tokens.first {
                Tok::Brace => RawElement::Braced,
                Tok::Null => RawElement::Null,
                _ => RawElement::Single(convert_number(tok_data(&tokens.first))),
            });
        }
    }

    i.reset(&checkpoint);

    if kw(i, "PATTERN").is_ok() {
        let tokens = commit(value_tokens(i))?;

        return Ok(RawElement::Pattern(tok_data(&tokens.first)));
    }

    let includes = kw(i, "INCLUDES").is_ok();
    let type_ = parse_type(i)?;

    Ok(RawElement::Type { type_, includes })
}

/// `lower..upper`, with the ends that are not part of the range marked
/// by `<`.
fn value_range(i: &mut Input) -> R<Option<RangeItem>> {
    let checkpoint = i.checkpoint();

    // Digits and a dot, and then the separator: 1...2
    let digits = {
        let offset = here(i);
        let text = take_while_chars(i, |c| c.is_ascii_digit() || c == '-');

        if text.is_empty() {
            None
        } else {
            Some((text, offset))
        }
    };

    let mut lower: Option<(Data, bool)> = None;

    if let Some((text, _)) = &digits {
        let after_digits = i.checkpoint();

        if peek_char(i) == Some('.') && text.bytes().all(|b| b.is_ascii_digit()) {
            i.next_token();

            if lit(i, "..").is_ok() {
                lower = Some((Data::Float(text.parse::<f64>().unwrap_or(0.0)), false));
            }
        }

        if lower.is_none() {
            i.reset(&after_digits);

            if lit(i, "..").is_ok()
                && convert_number(Data::Word(text.clone())) != Data::Word(text.clone())
            {
                lower = Some((convert_number(Data::Word(text.clone())), false));
            }
        }
    }

    if lower.is_none() {
        i.reset(&checkpoint);

        let bound = lower_endpoint(i);

        match bound {
            Ok(bound) => {
                if lit(i, "..").is_ok() {
                    lower = Some(bound);
                }
            }
            Err(ErrMode::Backtrack(_)) => {}
            Err(error) => return Err(error),
        }
    }

    let Some((lower, lower_excluded)) = lower else {
        i.reset(&checkpoint);

        return Ok(None);
    };

    // The separator is there, so the upper end has to be.
    let upper_excluded = lit(i, "<").is_ok();
    let upper = commit(upper_end_value(i))?;

    Ok(Some(RangeItem {
        lower,
        upper,
        lower_excluded,
        upper_excluded,
    }))
}

fn lower_endpoint(i: &mut Input) -> R<(Data, bool)> {
    let data = if kw(i, "MIN").is_ok() {
        Data::Word("MIN".to_string())
    } else {
        let tokens = value_tokens(i)?;

        if tokens.second_is_colon {
            return backtrack();
        }

        endpoint(&tokens)
    };

    let excluded = lit(i, "<").is_ok();

    Ok((data, excluded))
}

fn upper_end_value(i: &mut Input) -> R<Data> {
    if kw(i, "MAX").is_ok() {
        return Ok(Data::Word("MAX".to_string()));
    }

    let checkpoint = i.checkpoint();

    match value_tokens(i) {
        Ok(tokens) if !tokens.second_is_colon => Ok(endpoint(&tokens)),
        _ => {
            i.reset(&checkpoint);

            backtrack()
        }
    }
}

/// `convert_value_range_endpoint()` of the Python parser.
fn endpoint(tokens: &ValueTokens) -> Data {
    if let Some(numbers) = &tokens.quadruple {
        return Data::List(numbers.clone());
    }

    match &tokens.first {
        Tok::Word(text)
            if text.ends_with('.')
                && text
                    .trim_start_matches('-')
                    .trim_end_matches('.')
                    .bytes()
                    .all(|b| b.is_ascii_digit()) =>
        {
            Data::Float(text.trim_end_matches('.').parse::<f64>().unwrap_or(0.0))
        }
        first => tok_data(first),
    }
}

/// `{ name constraint presence, ... }`
/// The components of `WITH COMPONENTS`, and if it is partial, which it
/// is when it starts with `...`.
fn components_specification(i: &mut Input) -> R<(bool, Vec<RawComponent>)> {
    lit(i, "{")?;
    let mut components = Vec::new();
    let partial = lit(i, "...").is_ok();

    if partial {
        commit(lit(i, ","))?;
    }

    loop {
        let (name, _) = commit(identifier(i))?;
        let mut constraints = Vec::new();

        loop {
            let checkpoint = i.checkpoint();

            match constraint(i) {
                Ok(constraint) => constraints.push(constraint),
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&checkpoint);
                    break;
                }
                Err(error) => return Err(error),
            }
        }

        let mut presence = None;

        for word in ["PRESENT", "ABSENT", "OPTIONAL"] {
            if kw(i, word).is_ok() {
                presence = Some(word.to_string());
                break;
            }
        }

        components.push(RawComponent {
            name,
            constraints,
            presence,
        });

        if lit(i, ",").is_err() {
            break;
        }
    }

    commit(lit(i, "}"))?;

    Ok((partial, components))
}

/// `convert_inner_constraint()` of the Python parser: the constraints of
/// a component or of the elements of a `SEQUENCE OF`, just as those of
/// a type.
fn inner_constraint(items: Vec<Item>) -> ElementConstraint {
    let mut constrained = ElementConstraint::default();
    let mut restricted_to: Vec<Value> = Vec::new();

    fn flatten(items: Vec<Item>, out: &mut Vec<Item>) {
        for item in items {
            match item {
                Item::Nested(nested) => flatten(nested, out),
                other => out.push(other),
            }
        }
    }

    let mut flat = Vec::new();
    flatten(items, &mut flat);

    for item in flat {
        match item {
            Item::Ellipsis => restricted_to.push(Value::ExtensionMarker),
            Item::Size(values) => constrained.size = Some(values),
            Item::From(values) => constrained.permitted_alphabet = Some(values),
            Item::WithComponents(values) => constrained.with_components = Some(values),
            Item::WithComponent(element) => {
                constrained.with_component = Some(vec![Value::Element(Box::new(element))])
            }
            Item::Containing(type_) => constrained.containing = Some(Box::new(type_)),
            Item::Single(data) => {
                restricted_to.push(value_from_data(&data, Where::Constraint, None, false))
            }
            Item::Range(range) => restricted_to.push(range_value(&range, Where::Constraint)),
            Item::Type(type_) => restricted_to.push(Value::Type(Box::new(type_))),
            Item::Junk | Item::Ignored | Item::Table(_) | Item::Nested(_) => {}
        }
    }

    if !restricted_to.is_empty() {
        constrained.restricted_to = Some(restricted_to);
    }

    constrained
}

/// A type, with the constraints it is followed by.
pub fn parse_type(i: &mut Input) -> R<Type> {
    let mut type_ = base_type(i)?;
    let mut constraints = Vec::new();

    loop {
        let checkpoint = i.checkpoint();

        match constraint(i) {
            Ok(constraint) => constraints.push(constraint),
            Err(ErrMode::Backtrack(_)) => {
                i.reset(&checkpoint);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    apply_constraints(&mut type_, items_of(&constraints));

    if !constraints.is_empty() {
        type_.common.constraints = tree_of(&constraints);
    }

    Ok(type_)
}

/// Puts the constraints of a type where they belong, just as
/// `convert_type()` of the Python parser does.
fn apply_constraints(type_: &mut Type, items: Vec<Item>) {
    let mut restricted_to: Vec<Value> = Vec::new();
    let mut has_junk = false;
    // Only a field of an object class has a table.
    let table_wanted = type_.type_name().contains('&');

    for item in items {
        match item {
            Item::Ellipsis => {
                if !restricted_to.is_empty() {
                    restricted_to.push(Value::ExtensionMarker);
                }

                if let Some(size) = &mut type_.common.size {
                    size.push(Value::ExtensionMarker);
                }
            }
            Item::Size(values) => type_.common.size = Some(values),
            Item::From(values) => type_.common.permitted_alphabet = Some(values),
            Item::WithComponents(values) => type_.common.with_components = Some(values),
            Item::WithComponent(element) => {
                type_.common.with_component = Some(vec![Value::Element(Box::new(element))])
            }
            Item::Single(data) => {
                restricted_to.push(value_from_data(&data, Where::Constraint, None, false))
            }
            Item::Range(range) => restricted_to.push(range_value(&range, Where::Constraint)),
            Item::Type(contained) => restricted_to.push(Value::Type(Box::new(contained))),
            Item::Junk => has_junk = true,
            Item::Table(table) => {
                has_junk = true;

                if table_wanted && type_.common.table.is_none() {
                    type_.common.table = Some(table);
                }
            }
            Item::Containing(_) | Item::Ignored | Item::Nested(_) => {}
        }
    }

    if has_junk {
        restricted_to.clear();
    }

    if !restricted_to.is_empty() {
        type_.common.restricted_to = Some(restricted_to);
    }
}

// ----------------------------------------------------------------------
// Modules

/// `{ governor : name, name }` after the name of a parameterized
/// assignment.
fn parameter_list(i: &mut Input) -> R<Option<Vec<Parameter>>> {
    let checkpoint = i.checkpoint();

    if lit(i, "{").is_err() {
        i.reset(&checkpoint);

        return Ok(None);
    }

    let mut parameters = Vec::new();

    loop {
        let mut governor = None;
        let inner = i.checkpoint();

        match parse_type(i) {
            Ok(type_) if lit(i, ":").is_ok() => governor = Some(type_),
            Ok(_) | Err(ErrMode::Backtrack(_)) => i.reset(&inner),
            Err(error) => return Err(error),
        }

        let (name, _) = commit(reference(i))?;
        parameters.push(Parameter { name, governor });

        if lit(i, ",").is_err() {
            break;
        }
    }

    commit(lit(i, "}"))?;

    Ok(Some(parameters))
}

/// Rejects a name that is assigned already in the module: a reference
/// name is assigned once in a module (X.680 13.1).
fn assign_once(i: &mut Input, module: &Module, name: &str, offset: usize) -> R<()> {
    let assigned = module.types.iter().any(|t| t.name == name)
        || module.object_classes.iter().any(|c| c.name == name)
        || module.object_sets.iter().any(|o| o.name == name)
        || module.raw_values.iter().any(|v| v.name == name);

    if assigned {
        return conversion_error_at(
            i,
            format!("'{}' is assigned twice in module '{}'", name, module.name),
            offset,
        );
    }

    Ok(())
}

fn replace_or_push<T>(items: &mut Vec<T>, item: T, same: impl Fn(&T) -> bool) {
    match items.iter_mut().find(|existing| same(existing)) {
        Some(existing) => *existing = item,
        None => items.push(item),
    }
}

// ----------------------------------------------------------------------
// Information objects

/// `&name` or `&Name`, as the name of a field.
fn field_reference(i: &mut Input) -> R<(String, usize)> {
    let offset = here(i);

    if peek_char(i) != Some('&') {
        return fail(i, offset, "&");
    }

    let checkpoint = i.checkpoint();
    i.next_token();

    match peek_char(i) {
        Some(c) if c.is_ascii_alphabetic() => {
            let name = take_while_chars(i, |c| c.is_ascii_alphanumeric() || c == '-');

            Ok((format!("&{name}"), offset))
        }
        _ => {
            i.reset(&checkpoint);

            fail(i, offset, "fieldname")
        }
    }
}

fn class_field(i: &mut Input) -> R<ObjectClassField> {
    let (name, offset) = field_reference(i)?;
    let is_type_field = name.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase());

    let type_ = if is_type_field {
        if kw(i, "OPTIONAL").is_err() && kw(i, "DEFAULT").is_ok() {
            commit(parse_type(i))?;
        }

        Type::new(Body::Builtin("OpenType".to_string()))
    } else {
        let type_ = parse_type(i)?;
        let _ = kw(i, "UNIQUE");

        if kw(i, "OPTIONAL").is_err() && kw(i, "DEFAULT").is_ok() {
            commit(value_tokens(i))?;
        }

        type_
    };

    Ok(ObjectClassField {
        name,
        type_,
        location: offset_location(i, offset),
    })
}

/// The words, commas and field names of `WITH SYNTAX { ... }`, with the
/// optional groups in brackets as groups.
fn syntax_list(i: &mut Input) -> R<Vec<SyntaxToken>> {
    commit(lit(i, "{"))?;
    let mut groups: Vec<Vec<SyntaxToken>> = vec![Vec::new()];
    let mut count = 0;

    loop {
        if lit(i, "[").is_ok() {
            groups.push(Vec::new());
        } else if lit(i, "]").is_ok() {
            if groups.len() == 1 {
                let offset = here(i);

                return commit(fail(i, offset, "]"));
            }

            let group = groups.pop().unwrap();
            groups
                .last_mut()
                .unwrap()
                .push(SyntaxToken::Optional(group));
        } else if lit(i, ",").is_ok() {
            groups
                .last_mut()
                .unwrap()
                .push(SyntaxToken::Word(",".to_string()));
        } else if let Ok(word) = word(i) {
            groups.last_mut().unwrap().push(SyntaxToken::Word(word));
        } else {
            break;
        }

        count += 1;
    }

    if count == 0 || groups.len() != 1 {
        let offset = here(i);

        return commit(fail(i, offset, "syntax"));
    }

    commit(lit(i, "}"))?;

    Ok(groups.pop().unwrap())
}

fn class_definition(i: &mut Input, name: String, offset: usize) -> R<ObjectClassAssignment> {
    kw(i, "CLASS")?;
    commit(lit(i, "{"))?;
    let mut fields = Vec::new();

    loop {
        fields.push(commit(class_field(i))?);

        if lit(i, ",").is_err() {
            break;
        }
    }

    commit(lit(i, "}"))?;
    let mut syntax = None;

    if kw(i, "WITH SYNTAX").is_ok() {
        syntax = Some(syntax_list(i)?);
    }

    Ok(ObjectClassAssignment {
        name,
        fields,
        syntax,
        location: offset_location(i, offset),
    })
}

/// What an object is, as found: the fields of its class, or its text.
fn object(i: &mut Input) -> R<RawData> {
    let begin = here(i);

    if peek_char(i) != Some('{') {
        return fail(i, begin, "{");
    }

    let checkpoint = i.checkpoint();

    // The fields of the class, `{ &id 1, &Type INTEGER }`.
    if let Ok(fields) = default_syntax(i) {
        return Ok(RawData::Fields(fields));
    }

    i.reset(&checkpoint);
    balanced_braces(i)?;
    let text = text_since(i, begin);
    let inner = text[1..text.len() - 1].trim().to_string();

    // Only words, commas, settings and what is in parentheses are an
    // object given with the syntax of its class.
    if !is_defined_syntax(&inner) {
        i.reset(&checkpoint);

        return fail(i, begin, "object");
    }

    Ok(RawData::DefinedSyntax(inner))
}

/// Whether the text of an object is made of the tokens of the syntax of a
/// class: words, commas, strings, things in braces and in parentheses.
fn is_defined_syntax(text: &str) -> bool {
    let chars: Vec<char> = text.chars().collect();
    let mut at = 0;

    let skip_balanced = |at: &mut usize, open: char, close: char| -> bool {
        let mut depth = 0;

        while *at < chars.len() {
            let c = chars[*at];
            *at += 1;

            if c == open {
                depth += 1;
            } else if c == close {
                depth -= 1;

                if depth == 0 {
                    return true;
                }
            } else if c == '"' || c == '\'' {
                while *at < chars.len() && chars[*at] != c {
                    *at += 1;
                }

                *at += 1;
            }
        }

        false
    };

    while at < chars.len() {
        let c = chars[at];

        if c.is_whitespace() || c == ',' {
            at += 1;
        } else if c == '"' || c == '\'' {
            at += 1;

            while at < chars.len() && chars[at] != c {
                at += 1;
            }

            if at >= chars.len() {
                return false;
            }

            at += 1;

            // The B or H of a bit string or a hexadecimal string.
            if c == '\'' && at < chars.len() && (chars[at] == 'B' || chars[at] == 'H') {
                at += 1;
            }
        } else if c == '{' {
            if !skip_balanced(&mut at, '{', '}') {
                return false;
            }
        } else if c == '(' {
            if !skip_balanced(&mut at, '(', ')') {
                return false;
            }
        } else if c.is_ascii_graphic() && !",(){}[].:=;\"|".contains(c) {
            while at < chars.len()
                && chars[at].is_ascii_graphic()
                && !",(){}[].:=;\"|".contains(chars[at])
            {
                at += 1;
            }
        } else {
            return false;
        }
    }

    true
}

fn default_syntax(i: &mut Input) -> R<Vec<ObjectField>> {
    lit(i, "{")?;
    let mut fields = Vec::new();

    loop {
        let (name, _) = field_reference(i)?;
        let setting = if peek_char_ws(i).is_some_and(|c| c.is_ascii_uppercase()) {
            let checkpoint = i.checkpoint();

            match parse_type(i) {
                Ok(type_) => Value::Type(Box::new(type_)),
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&checkpoint);

                    return backtrack();
                }
                Err(error) => return Err(error),
            }
        } else {
            let tokens = value_tokens(i)?;

            if matches!(tokens.first, Tok::Brace) {
                return backtrack();
            }

            value_from_data(
                &convert_number(tok_data(&tokens.first)),
                Where::Value,
                None,
                false,
            )
        };

        fields.push(ObjectField { name, setting });

        if lit(i, ",").is_err() {
            break;
        }
    }

    lit(i, "}")?;

    Ok(fields)
}

fn raw_to_value(raw: RawData) -> Value {
    match raw {
        RawData::Fields(fields) => Value::Object(fields),
        RawData::DefinedSyntax(text) => Value::UnresolvedObject(text),
        RawData::Data(_) => Value::UnsupportedValue,
    }
}

/// One element of an object set: an object, or the name of an object or
/// of an object set.
fn object_set_element(i: &mut Input) -> R<Value> {
    if peek_char_ws(i) == Some('{') {
        return Ok(raw_to_value(object(i)?));
    }

    if peek_char_ws(i).is_some_and(|c| c.is_ascii_alphabetic()) {
        let name = if peek_char(i).is_some_and(|c| c.is_ascii_uppercase()) {
            type_reference(i)?.0
        } else {
            identifier(i)?.0
        };

        return Ok(Value::Str(name));
    }

    let offset = here(i);

    fail(i, offset, "object")
}

/// Elements separated by `|`, `^`, `UNION` or `INTERSECTION`.
fn object_set_elements(i: &mut Input) -> R<Vec<Value>> {
    let mut members = vec![object_set_element(i)?];

    loop {
        let checkpoint = i.checkpoint();

        if lit(i, "|").is_ok()
            || lit(i, "^").is_ok()
            || kw(i, "UNION").is_ok()
            || kw(i, "INTERSECTION").is_ok()
        {
            match object_set_element(i) {
                Ok(member) => members.push(member),
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&checkpoint);
                    break;
                }
                Err(error) => return Err(error),
            }
        } else {
            i.reset(&checkpoint);
            break;
        }
    }

    Ok(members)
}

/// `{ member | member, ... }` of an object set.
fn object_set_members(i: &mut Input) -> R<Vec<Value>> {
    lit(i, "{")?;
    let mut members: Vec<Value> = Vec::new();

    if lit(i, "...").is_ok() {
        members.push(Value::ExtensionMarker);
    } else {
        members.extend(object_set_elements(i)?);

        let checkpoint = i.checkpoint();

        if lit(i, ",").is_ok() && lit(i, "...").is_ok() {
            members.push(Value::ExtensionMarker);
        } else {
            i.reset(&checkpoint);
            lit(i, "}")?;

            return Ok(members);
        }
    }

    let checkpoint = i.checkpoint();

    if lit(i, ",").is_ok() {
        match object_set_elements(i) {
            Ok(more) => members.extend(more),
            Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
            Err(error) => return Err(error),
        }
    }

    lit(i, "}")?;

    Ok(members)
}

fn object_set(i: &mut Input) -> R<Vec<Value>> {
    commit(object_set_members(i))
}

fn assignment(i: &mut Input, module: &mut Module) -> R<()> {
    ws(i);
    let checkpoint = i.checkpoint();

    if peek_char(i).is_some_and(|c| c.is_ascii_uppercase()) {
        let (name, offset) = type_reference(i)?;
        let parameters = parameter_list(i)?;

        if lit(i, "::=").is_err() {
            // An object set: Name CLASS ::= { ... }
            if let Ok((class_name, _)) = object_class_reference(i) {
                if lit(i, "::=").is_ok() {
                    let members = commit(object_set(i))?;
                    let assignment = ObjectSetAssignment {
                        name: name.clone(),
                        class_name,
                        members,
                        location: offset_location(i, offset),
                    };

                    assign_once(i, module, &name, offset)?;
                    module.object_sets.push(assignment);

                    return Ok(());
                }
            }

            i.reset(&checkpoint);
            type_reference(i)?;
            let _ = parameter_list(i)?;
            let offset = here(i);

            return commit(fail(i, offset, "::="));
        }

        let after_assign = i.checkpoint();

        if kw(i, "CLASS").is_ok() {
            i.reset(&after_assign);
            let class = class_definition(i, name.clone(), offset)?;
            assign_once(i, module, &name, offset)?;
            module.object_classes.push(class);

            return Ok(());
        }

        let tag = commit(tag(i))?;
        let mut type_ = commit(parse_type(i))?;

        if let Some(tag) = tag {
            type_.common.tag = Some(tag);
        }

        type_.common.parameters = parameters;

        let assignment = TypeAssignment {
            name: name.clone(),
            type_,
            location: offset_location(i, offset),
        };

        assign_once(i, module, &name, offset)?;
        module.types.push(assignment);

        return Ok(());
    }

    let (name, offset) = value_reference(i)?;
    let _ = parameter_list(i)?;

    // An object: name CLASS ::= { ... }
    let after_name = i.checkpoint();

    if let Ok((class_name, _)) = object_class_reference(i) {
        if lit(i, "::=").is_ok() && peek_char_ws(i) == Some('{') {
            if let Ok(object) = object(i) {
                let assignment = RawValue {
                    name: name.clone(),
                    type_name: class_name,
                    value: object,
                    location: offset_location(i, offset),
                };

                assign_once(i, module, &name, offset)?;
                module.raw_values.push(assignment);

                return Ok(());
            }
        }
    }

    i.reset(&after_name);
    let type_ = commit(parse_type(i))?;
    commit(lit(i, "::="))?;
    let tokens = commit(value_tokens(i))?;
    let type_name = type_.type_name();
    let data = convert_value(&tokens, Some(&type_name));
    let assignment = RawValue {
        name: name.clone(),
        type_name,
        value: RawData::Data(data),
        location: offset_location(i, offset),
    };

    assign_once(i, module, &name, offset)?;
    module.raw_values.push(assignment);

    Ok(())
}

/// `[a-z][a-zA-Z0-9-]*`
fn value_reference(i: &mut Input) -> R<(String, usize)> {
    identifier(i)
}

/// A reference to a type, a value, an object class, an object or an
/// object set, as in a list of symbols.
fn reference(i: &mut Input) -> R<(String, usize)> {
    first(i, &[type_reference, value_reference])
}

/// `Name` or `Name{}`, of which the braces are symbols too.
fn symbol(i: &mut Input) -> R<Vec<String>> {
    let (name, _) = reference(i)?;
    let mut symbols = vec![name];
    let checkpoint = i.checkpoint();

    if lit(i, "{").is_ok() && lit(i, "}").is_ok() {
        symbols.push("{".to_string());
        symbols.push("}".to_string());
    } else {
        i.reset(&checkpoint);
    }

    Ok(symbols)
}

fn symbol_list(i: &mut Input) -> R<Vec<String>> {
    let mut symbols = symbol(i)?;

    loop {
        let checkpoint = i.checkpoint();

        if lit(i, ",").is_err() {
            break;
        }

        match symbol(i) {
            Ok(symbol) => symbols.extend(symbol),
            Err(ErrMode::Backtrack(_)) => {
                i.reset(&checkpoint);
                break;
            }
            Err(error) => return Err(error),
        }
    }

    Ok(symbols)
}

/// The components of an object identifier, in braces.
fn object_identifier_components(i: &mut Input) -> R<()> {
    lit(i, "{")?;
    let mut components = 0;

    loop {
        let checkpoint = i.checkpoint();

        if number(i).is_ok() {
        } else if let Ok(_) = identifier(i) {
            let inner = i.checkpoint();

            if lit(i, "(").is_ok() {
                if number(i).is_err() && identifier(i).is_err() {
                    let offset = here(i);

                    return commit(fail(i, offset, "number"));
                }

                commit(lit(i, ")"))?;
            } else {
                i.reset(&inner);
            }
        } else if let Ok(_) = type_reference(i) {
            // Module.value
            commit(lit(i, "."))?;
            commit(identifier(i))?;
        } else {
            i.reset(&checkpoint);
            break;
        }

        components += 1;
    }

    lit(i, "}")?;

    if components == 0 {
        return backtrack();
    }

    Ok(())
}

/// What follows the name of a module in an import or its definition: an
/// object identifier, or a value reference that is not the start of the
/// next list of symbols.
fn assigned_identifier(i: &mut Input) {
    let checkpoint = i.checkpoint();

    if object_identifier_components(i).is_ok() {
        return;
    }

    i.reset(&checkpoint);

    if identifier(i).is_ok() {
        let after = i.checkpoint();

        if lit(i, ",").is_ok() || kw(i, "FROM").is_ok() {
            i.reset(&checkpoint);
        } else {
            i.reset(&after);
        }

        return;
    }

    i.reset(&checkpoint);
}

fn imports(i: &mut Input, module: &mut Module) -> R<()> {
    if kw(i, "EXPORTS").is_ok() {
        if kw(i, "ALL").is_err() {
            commit(symbol_list(i))?;

            while let Ok(_) = {
                let checkpoint = i.checkpoint();
                let result = symbol_list(i);

                if result.is_err() {
                    i.reset(&checkpoint);
                }

                result
            } {}
        }

        commit(lit(i, ";"))?;
    }

    if kw(i, "IMPORTS").is_err() {
        return Ok(());
    }

    loop {
        let checkpoint = i.checkpoint();
        let symbols = match symbol_list(i) {
            Ok(symbols) => symbols,
            Err(ErrMode::Backtrack(_)) => {
                i.reset(&checkpoint);
                break;
            }
            Err(error) => return Err(error),
        };

        if kw(i, "FROM").is_err() {
            i.reset(&checkpoint);
            break;
        }

        let from = commit(module_reference(i))?;
        assigned_identifier(i);

        if kw(i, "WITH SUCCESSORS").is_err() {
            let _ = kw(i, "WITH DESCENDANTS");
        }

        match module
            .imports
            .iter_mut()
            .find(|import| import.module == from)
        {
            Some(import) => {
                // list(set(...)) of Python: each symbol once.
                for symbol in symbols {
                    if !import.symbols.contains(&symbol) {
                        import.symbols.push(symbol);
                    }
                }

                import.symbols.sort();
            }
            None => {
                let mut symbols = symbols;
                symbols.sort();
                module.imports.push(Import {
                    module: from,
                    symbols,
                });
            }
        }
    }

    commit(lit(i, ";"))?;

    Ok(())
}

pub fn parse_modules(i: &mut Input) -> R<Vec<Module>> {
    let mut modules = Vec::new();

    loop {
        let start = here(i);

        if peek_char(i).is_none() {
            break;
        }

        let (name, offset) = commit(module_reference_at(i))?;

        if modules.iter().any(|module: &Module| module.name == name) {
            return conversion_error_at(i, format!("Module '{name}' is defined twice"), offset);
        }

        if lit(i, "{").is_ok() {
            // The definitive identifier of the module.
            let mut components = 0;

            loop {
                let checkpoint = i.checkpoint();

                if number(i).is_ok() {
                } else if identifier(i).is_ok() {
                    let inner = i.checkpoint();

                    if lit(i, "(").is_ok() {
                        commit(number(i))?;
                        commit(lit(i, ")"))?;
                    } else {
                        i.reset(&inner);
                    }
                } else {
                    i.reset(&checkpoint);
                    break;
                }

                components += 1;
            }

            if components == 0 {
                let offset = here(i);

                return commit(fail(i, offset, "identifier"));
            }

            commit(lit(i, "}"))?;
        }

        commit(kw(i, "DEFINITIONS"))?;
        let mut module = Module {
            name,
            location: offset_location(i, start),
            ..Module::default()
        };

        if kw(i, "AUTOMATIC").is_ok() {
            module.tags = Some("AUTOMATIC".to_string());
            commit(kw(i, "TAGS"))?;
        } else if kw(i, "EXPLICIT").is_ok() {
            module.tags = Some("EXPLICIT".to_string());
            commit(kw(i, "TAGS"))?;
        } else if kw(i, "IMPLICIT").is_ok() {
            module.tags = Some("IMPLICIT".to_string());
            commit(kw(i, "TAGS"))?;
        }

        if kw(i, "EXTENSIBILITY IMPLIED").is_ok() {
            module.extensibility_implied = true;
        }

        commit(lit(i, "::="))?;
        commit(kw(i, "BEGIN"))?;
        imports(i, &mut module)?;

        loop {
            let checkpoint = i.checkpoint();

            match assignment(i, &mut module) {
                Ok(()) => {}
                Err(ErrMode::Backtrack(_)) => {
                    i.reset(&checkpoint);
                    break;
                }
                Err(error) => return Err(error),
            }
        }

        commit(kw(i, "END"))?;
        modules.push(module);
    }

    Ok(modules)
}

fn module_reference_at(i: &mut Input) -> R<(String, usize)> {
    type_reference(i)
}

pub fn make_input<'a>(text: &'a str, locator: Rc<Locator>) -> Input<'a> {
    Stateful {
        input: LocatingSlice::new(text),
        state: State {
            locator: Some(locator),
            text: Some(Rc::from(text)),
            ..State::default()
        },
    }
}

/// Parses all of a specification, which is one or more modules.
pub fn parse_all(i: &mut Input) -> R<SyntaxTree> {
    let modules = parse_modules(i)?;

    if modules.is_empty() {
        let offset = here(i);

        return fail(i, offset, "modulereference");
    }

    Ok(SyntaxTree {
        modules: crate::objects::finish(modules),
    })
}

// ----------------------------------------------------------------------
// The syntax of an object class

/// Matches the text of an object against the tokens of the syntax of its
/// class, and gives the fields it tells, or `None` if it does not match.
pub(crate) fn parse_with_syntax(tokens: &[SyntaxToken], text: &str) -> Option<Vec<ObjectField>> {
    let mut input: Input = Stateful {
        input: LocatingSlice::new(text),
        state: State {
            text: Some(Rc::from(text)),
            ..State::default()
        },
    };
    let mut fields = Vec::new();

    syntax_match(&mut input, tokens, &mut fields).ok()?;

    // Nothing but whitespace is left.
    ws(&mut input);

    if peek_char(&input).is_some() {
        return None;
    }

    // The fields are a dictionary, so a name that is given again takes
    // the place of its first, which keeps its place.
    let mut unique: Vec<ObjectField> = Vec::new();

    for field in fields {
        replace_or_push(&mut unique, field.clone(), |f| f.name == field.name);
    }

    Some(unique)
}

fn syntax_match(i: &mut Input, tokens: &[SyntaxToken], fields: &mut Vec<ObjectField>) -> R<()> {
    for token in tokens {
        match token {
            SyntaxToken::Optional(group) => {
                let checkpoint = i.checkpoint();
                let mut inner = Vec::new();

                match syntax_match(i, group, &mut inner) {
                    Ok(()) => fields.extend(inner),
                    Err(ErrMode::Backtrack(_)) => i.reset(&checkpoint),
                    Err(error) => return Err(error),
                }
            }
            SyntaxToken::Word(word) if word == "," => lit(i, ",")?,
            SyntaxToken::Word(word) if !word.starts_with('&') => syntax_keyword(i, word)?,
            SyntaxToken::Word(word) => {
                let is_type = word.chars().nth(1).is_some_and(|c| c.is_ascii_uppercase());

                if is_type {
                    let type_ = parse_type(i)?;
                    fields.push(ObjectField {
                        name: word.clone(),
                        setting: Value::Type(Box::new(type_)),
                    });
                } else {
                    let tokens = value_tokens(i)?;
                    let data = convert_number(tok_data(&tokens.first));
                    fields.push(ObjectField {
                        name: word.clone(),
                        setting: value_from_data(&data, Where::Value, None, false),
                    });
                }
            }
        }
    }

    Ok(())
}

/// A word of the syntax: the text as it is, not followed by a character
/// of a word, in which a hyphen is one.
fn syntax_keyword(i: &mut Input, text: &str) -> R<()> {
    let offset = here(i);
    let checkpoint = i.checkpoint();

    for expected in text.chars() {
        match i.next_token() {
            Some(c) if c == expected => {}
            _ => {
                i.reset(&checkpoint);

                return fail(i, offset, "keyword");
            }
        }
    }

    if peek_char(i).is_some_and(|c| c.is_ascii_alphanumeric() || c == '-') {
        i.reset(&checkpoint);

        return fail(i, offset, "keyword");
    }

    Ok(())
}
