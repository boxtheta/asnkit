// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The text of the specifications: without comments, and where in the
//! files an offset in it is.

/// The offset a comment is not closed at, and what is wrong.
pub struct CommentError {
    pub offset: usize,
    pub message: &'static str,
}

/// Replaces comments with spaces, so that an offset is the same with and
/// without them. A single line comment ends where the line does, or at
/// the next `--`, and a multiple line comment, of which there may be one
/// inside another, ends at `*/`.
pub fn ignore_comments(string: &str) -> Result<String, CommentError> {
    let bytes = string.as_bytes();
    let mut out = String::with_capacity(string.len());
    let mut in_single_line_comment = false;
    let mut depth = 0usize;
    let mut start_offset = 0usize;
    let mut non_comment_offset = 0usize;
    let mut offset = 0usize;

    let spaces = |count: usize| " ".repeat(count);

    while offset < bytes.len() {
        let rest = &bytes[offset..];
        let kind: &str = if rest.starts_with(b"/*") {
            "/*"
        } else if rest.starts_with(b"*/") {
            "*/"
        } else if rest.starts_with(b"--") {
            "--"
        } else if rest.starts_with(b"\n") {
            "\n"
        } else {
            offset += 1;
            continue;
        };

        let at = offset;
        offset += kind.len();

        if in_single_line_comment {
            if kind == "--" || kind == "\n" {
                in_single_line_comment = false;
                let end = if kind == "--" { at + 2 } else { at };
                // The text of the comment is as long in characters as in
                // bytes for what matters here: every character of it
                // becomes as many spaces as it has bytes.
                out.push_str(&spaces(end - start_offset));
                non_comment_offset = end;
            }
        } else if depth > 0 {
            if kind == "/*" {
                depth += 1;
            } else if kind == "*/" {
                depth -= 1;

                if depth == 0 {
                    let end = at + 2;
                    out.push_str(&spaces(end - start_offset));
                    non_comment_offset = end;
                }
            }
        } else if kind == "--" {
            in_single_line_comment = true;
            start_offset = at;
            out.push_str(&string[non_comment_offset..start_offset]);
        } else if kind == "/*" {
            depth = 1;
            start_offset = at;
            out.push_str(&string[non_comment_offset..start_offset]);
        }
    }

    if in_single_line_comment {
        return Err(CommentError {
            offset: start_offset,
            message: "Missing newline or -- for single line comment",
        });
    }

    if depth > 0 {
        return Err(CommentError {
            offset: start_offset,
            message: "Missing */ for multi line comment",
        });
    }

    out.push_str(&string[non_comment_offset..]);

    Ok(out)
}

/// Tells where in the files an offset in the string of all of them, one
/// after the other with a line break between, is.
#[derive(Debug)]
pub struct Locator {
    string: String,
    source_offsets: Vec<usize>,
    newline_offsets: Vec<usize>,
}

impl Locator {
    pub fn new(string: &str, source_sizes: &[usize]) -> Locator {
        let mut source_offsets = vec![0];

        for size in &source_sizes[..source_sizes.len().saturating_sub(1)] {
            source_offsets.push(source_offsets.last().unwrap() + size);
        }

        let newline_offsets = string
            .bytes()
            .enumerate()
            .filter(|(_, byte)| *byte == b'\n')
            .map(|(offset, _)| offset)
            .collect();

        Locator {
            string: string.to_string(),
            source_offsets,
            newline_offsets,
        }
    }

    /// The index of the file, the line and the column given offset is at,
    /// with the two last counted from 1, and the column in characters.
    pub fn locate_offset(&self, offset: usize) -> (usize, usize, usize) {
        let source = self.source_offsets.partition_point(|o| *o <= offset) - 1;
        let source_offset = self.source_offsets[source];
        let first_line = self.newline_offsets.partition_point(|o| *o < source_offset);
        let line = self.newline_offsets.partition_point(|o| *o < offset);

        let line_offset = if line > first_line {
            self.newline_offsets[line - 1] + 1
        } else {
            source_offset
        };

        let column = self.string[line_offset..offset].chars().count() + 1;

        (source, line - first_line + 1, column)
    }

    /// Where an error is, and the line it is in with the place marked.
    pub fn locate_error(&self, offset: usize) -> (usize, usize, usize, String) {
        let offset = offset.min(self.string.len());
        let (source, line, column) = self.locate_offset(offset);
        let begin = self.string[..offset]
            .rfind('\n')
            .map(|i| i + 1)
            .unwrap_or(0)
            .max(self.source_offsets[source]);
        let end = self.string[offset..]
            .find('\n')
            .map(|i| offset + i)
            .unwrap_or(self.string.len());
        let marked = format!(
            "{}>!<{}",
            &self.string[begin..offset],
            &self.string[offset..end]
        );

        (source, line, column, marked.trim().to_string())
    }
}
