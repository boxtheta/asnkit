// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Writes the `.asni` and `.asnst` files (see `docs/file_formats.md`) as
//! `asnkitrt.syntax.container` does: the part of CBOR (RFC 8949) they use,
//! in the shortest form, and the header of an `.asni` file, with the
//! CRC-32 of the rest of it.

use num_bigint::BigUint;
use serde_json::Value as Json;
use sha2::{Digest, Sha256};

const MAJOR_UNSIGNED: u8 = 0;
const MAJOR_NEGATIVE: u8 = 1;
const MAJOR_BYTES: u8 = 2;
const MAJOR_TEXT: u8 = 3;
const MAJOR_ARRAY: u8 = 4;
const MAJOR_MAP: u8 = 5;
const MAJOR_TAG: u8 = 6;
const MAJOR_SIMPLE: u8 = 7;

const TAG_POSITIVE_BIGNUM: u64 = 2;
const TAG_NEGATIVE_BIGNUM: u64 = 3;

/// The version of an `.asni` file, of its header and its nodes.
const ASNI_VERSION: (u8, u8) = (2, 0);
/// The version of an `.asnst` file, of its map and its nodes.
const ASNST_VERSION: (u8, u8) = (1, 1);
/// Where the metadata of an `.asni` file begins: after the magic, the
/// version and the CRC-32.
const ASNI_HEADER_SIZE: usize = 10;

/// What the files tell of where they come from.
#[derive(Debug, Clone, Default)]
pub struct Source {
    pub name: Option<String>,
    pub text: Option<String>,
    /// Whether the text is part of the metadata.
    pub include_text: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Metadata {
    pub generator: Option<String>,
    pub sources: Vec<Source>,
}

/// The CRC-32 of `data`, as zlib, gzip and PNG compute it (CRC-32/ISO-HDLC),
/// a bit at a time, which is fast enough for files of this size.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;

    for &byte in data {
        crc ^= u32::from(byte);

        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }

    !crc
}

fn encode_head(major: u8, argument: u64, out: &mut Vec<u8>) {
    if argument < 24 {
        out.push((major << 5) | argument as u8);
    } else if argument < 1 << 8 {
        out.push((major << 5) | 24);
        out.push(argument as u8);
    } else if argument < 1 << 16 {
        out.push((major << 5) | 25);
        out.extend_from_slice(&(argument as u16).to_be_bytes());
    } else if argument < 1 << 32 {
        out.push((major << 5) | 26);
        out.extend_from_slice(&(argument as u32).to_be_bytes());
    } else {
        out.push((major << 5) | 27);
        out.extend_from_slice(&argument.to_be_bytes());
    }
}

fn encode_text(text: &str, out: &mut Vec<u8>) {
    encode_head(MAJOR_TEXT, text.len() as u64, out);
    out.extend_from_slice(text.as_bytes());
}

/// An integer of any size, given as its digits, with a sign.
fn encode_integer(digits: &str, out: &mut Vec<u8>) {
    let negative = digits.starts_with('-');
    let magnitude = digits.trim_start_matches('-');

    // A negative number n is -1 - value. The digits are those of a
    // number the compiler made, so they parse.
    let value: BigUint = magnitude.parse().expect("an integer");
    // -0 is 0, and not -1 - 0, which is no BigUint.
    let negative = negative && value != BigUint::from(0u32);
    let value = if negative { value - 1u32 } else { value };
    let (major, tag) = if negative {
        (MAJOR_NEGATIVE, TAG_NEGATIVE_BIGNUM)
    } else {
        (MAJOR_UNSIGNED, TAG_POSITIVE_BIGNUM)
    };

    match u64::try_from(&value) {
        Ok(value) => encode_head(major, value, out),
        Err(_) => {
            let bytes = value.to_bytes_be();
            encode_head(MAJOR_TAG, tag, out);
            encode_head(MAJOR_BYTES, bytes.len() as u64, out);
            out.extend_from_slice(&bytes);
        }
    }
}

/// Encodes given primitives as CBOR.
pub fn encode(value: &Json, out: &mut Vec<u8>) {
    match value {
        Json::Null => encode_head(MAJOR_SIMPLE, 22, out),
        Json::Bool(true) => encode_head(MAJOR_SIMPLE, 21, out),
        Json::Bool(false) => encode_head(MAJOR_SIMPLE, 20, out),
        Json::Number(number) => {
            let text = number.to_string();

            if text.contains(['.', 'e', 'E']) {
                out.push((MAJOR_SIMPLE << 5) | 27);
                out.extend_from_slice(&text.parse::<f64>().unwrap_or(0.0).to_be_bytes());
            } else {
                encode_integer(&text, out);
            }
        }
        Json::String(text) => encode_text(text, out),
        Json::Array(items) => {
            encode_head(MAJOR_ARRAY, items.len() as u64, out);

            for item in items {
                encode(item, out);
            }
        }
        Json::Object(map) => {
            encode_head(MAJOR_MAP, map.len() as u64, out);

            for (key, item) in map {
                encode_text(key, out);
                encode(item, out);
            }
        }
    }
}

fn sha256_hex(text: &str) -> String {
    Sha256::digest(text.as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// The metadata as a CBOR map, with its keys in the order of the Python
/// writer.
fn encode_metadata(metadata: &Metadata, out: &mut Vec<u8>) {
    let mut entries = 0;

    if metadata.generator.is_some() {
        entries += 1;
    }

    if !metadata.sources.is_empty() {
        entries += 1;
    }

    encode_head(MAJOR_MAP, entries, out);

    if let Some(generator) = &metadata.generator {
        encode_text("generator", out);
        encode_text(generator, out);
    }

    if !metadata.sources.is_empty() {
        encode_text("sources", out);
        encode_head(MAJOR_ARRAY, metadata.sources.len() as u64, out);

        for source in &metadata.sources {
            let text = source.text.as_deref();
            let fields = u64::from(source.name.is_some())
                + u64::from(text.is_some())
                + u64::from(text.is_some() && source.include_text);

            encode_head(MAJOR_MAP, fields, out);

            if let Some(name) = &source.name {
                encode_text("name", out);
                encode_text(name, out);
            }

            if let Some(text) = text {
                encode_text("sha256", out);
                encode_text(&sha256_hex(text), out);

                if source.include_text {
                    encode_text("text", out);
                    encode_text(text, out);
                }
            }
        }
    }
}

/// The contents of an `.asni` file.
pub fn write_asni(metadata: &Metadata, nodes: &Json) -> Vec<u8> {
    let mut out = b"ASNI".to_vec();
    out.push(ASNI_VERSION.0);
    out.push(ASNI_VERSION.1);
    // The CRC-32, once the rest is written.
    out.extend_from_slice(&[0; 4]);
    encode_metadata(metadata, &mut out);
    encode(nodes, &mut out);

    let crc = crc32(&out[ASNI_HEADER_SIZE..]);
    out[6..ASNI_HEADER_SIZE].copy_from_slice(&crc.to_be_bytes());

    out
}

/// The contents of an `.asnst` file.
pub fn write_asnst(metadata: &Metadata, tree: &Json) -> Vec<u8> {
    let mut out = Vec::new();
    encode_head(MAJOR_MAP, 4, &mut out);
    encode_text("format", &mut out);
    encode_text("asnst", &mut out);
    encode_text("version", &mut out);
    encode_head(MAJOR_ARRAY, 2, &mut out);
    encode_head(MAJOR_UNSIGNED, u64::from(ASNST_VERSION.0), &mut out);
    encode_head(MAJOR_UNSIGNED, u64::from(ASNST_VERSION.1), &mut out);
    encode_text("metadata", &mut out);
    encode_metadata(metadata, &mut out);
    encode_text("tree", &mut out);
    encode(tree, &mut out);

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cbor(json: &str) -> Vec<u8> {
        let mut out = Vec::new();
        encode(&serde_json::from_str(json).unwrap(), &mut out);

        out
    }

    #[test]
    fn integers_are_in_the_shortest_form() {
        assert_eq!(cbor("0"), [0x00]);
        assert_eq!(cbor("23"), [0x17]);
        assert_eq!(cbor("24"), [0x18, 24]);
        assert_eq!(cbor("256"), [0x19, 1, 0]);
        assert_eq!(cbor("65536"), [0x1a, 0, 1, 0, 0]);
        assert_eq!(cbor("-1"), [0x20]);
        assert_eq!(cbor("-25"), [0x38, 24]);
        assert_eq!(
            cbor("18446744073709551615"),
            [0x1b, 255, 255, 255, 255, 255, 255, 255, 255]
        );
    }

    #[test]
    fn integers_above_64_bits_are_bignums() {
        assert_eq!(
            cbor("18446744073709551616"),
            [0xc2, 0x49, 1, 0, 0, 0, 0, 0, 0, 0, 0]
        );
        assert_eq!(
            cbor("-18446744073709551617"),
            [0xc3, 0x49, 1, 0, 0, 0, 0, 0, 0, 0, 0]
        );

        // Above 128 bits too, which were written as 2^128 - 1.
        let mut above = vec![0xc2, 0x51, 4];
        above.extend([0; 16]);
        assert_eq!(cbor("1361129467683753853853498429727072845824"), above);

        let mut below = vec![0xc3, 0x51, 4];
        below.extend([0; 16]);
        assert_eq!(cbor("-1361129467683753853853498429727072845825"), below);
    }

    #[test]
    fn floats_are_of_64_bits_and_the_rest_is_plain() {
        assert_eq!(cbor("1.5"), [0xfb, 0x3f, 0xf8, 0, 0, 0, 0, 0, 0]);
        assert_eq!(cbor("null"), [0xf6]);
        assert_eq!(cbor("true"), [0xf5]);
        assert_eq!(cbor(r#"["a", [0]]"#), [0x82, 0x61, b'a', 0x81, 0x00]);
    }

    #[test]
    fn crc32_is_that_of_zlib() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn the_header_of_a_file_is_the_magic_the_version_and_the_crc() {
        let file = write_asni(&Metadata::default(), &Json::Null);

        // An empty map and null, after the header.
        assert_eq!(&file[ASNI_HEADER_SIZE..], [0xa0, 0xf6]);
        assert_eq!(&file[..6], b"ASNI\x02\x00");
        assert_eq!(
            &file[6..ASNI_HEADER_SIZE],
            crc32(&[0xa0, 0xf6]).to_be_bytes()
        );
    }

    #[test]
    fn a_syntax_tree_keeps_its_own_version() {
        let file = write_asnst(&Metadata::default(), &Json::Null);
        let mut version = Vec::new();
        encode_text("version", &mut version);
        version.extend([0x82, 0x01, 0x01]);

        assert!(file.windows(version.len()).any(|window| window == version));
    }
}
