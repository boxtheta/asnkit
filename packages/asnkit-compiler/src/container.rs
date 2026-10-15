// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! Writes the `.asnc` and `.asnst` files (see `docs/file_formats.md`) as
//! `asnkitrt.syntax.container` does: the part of CBOR (RFC 8949) they use,
//! in the shortest form, and the header of an `.asnc` file.

use aes_gcm::aead::{AeadInPlace, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use ed25519_dalek::{Signer, SigningKey};
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

const SCHEMA_VERSION: (u8, u8) = (1, 0);
const FLAG_SIGNED: u16 = 1;
const FLAG_ENCRYPTED: u16 = 1 << 1;
const FLAG_BOUND: u16 = 1 << 3;
const FLAG_NONCE: u16 = 1 << 2;
pub const NONCE_MAXIMUM: u32 = (1 << 24) - 1;
pub const BOUND_TO_MAXIMUM_SIZE: usize = 255;
pub const KEY_ID_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const IV_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;
const HEADER_SIZE: usize = 11;

/// A key with its identifier: an Ed25519 private key to sign with, or an
/// AES-256 key to encrypt with.
#[derive(Debug, Clone)]
pub struct Key {
    pub id: [u8; KEY_ID_SIZE],
    pub key: [u8; KEY_SIZE],
}

/// What a file is signed and encrypted with, if anything.
#[derive(Debug, Clone, Default)]
pub struct Protection {
    pub signing_key: Option<Key>,
    pub encryption_key: Option<Key>,
    /// The initialization vector. Never to be given but to test, as a
    /// vector used twice with a key makes it worthless; a random one is
    /// made if not given.
    pub iv: Option<[u8; IV_SIZE]>,
}

/// The identifier of a key as the compiler of Python makes it: the first
/// bytes of the SHA-256 of an AES key, or of the public key of an Ed25519
/// private key, so that those verifying find it from the key they hold.
pub fn derive_key_id(key: &[u8; KEY_SIZE], signing: bool) -> [u8; KEY_ID_SIZE] {
    let digest = if signing {
        Sha256::digest(SigningKey::from_bytes(key).verifying_key().as_bytes())
    } else {
        Sha256::digest(key)
    };

    let mut id = [0; KEY_ID_SIZE];
    id.copy_from_slice(&digest[..KEY_ID_SIZE]);

    id
}

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
    pub bound_to: Option<String>,
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
fn encode_metadata(metadata: &Metadata, out: &mut Vec<u8>) -> Result<(), String> {
    let mut entries = 0;

    if metadata.generator.is_some() {
        entries += 1;
    }

    if !metadata.sources.is_empty() {
        entries += 1;
    }

    if let Some(bound_to) = &metadata.bound_to {
        if bound_to.len() > BOUND_TO_MAXIMUM_SIZE {
            return Err(format!(
                "Expected what the file is bound to as at most {BOUND_TO_MAXIMUM_SIZE} bytes as UTF-8."
            ));
        }

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

    if let Some(bound_to) = &metadata.bound_to {
        encode_text("bound_to", out);
        encode_text(bound_to, out);
    }

    Ok(())
}

/// The contents of an `.asnc` file that is not signed or encrypted.
pub fn write_asnc(
    metadata: &Metadata,
    nodes: &Json,
    nonce: Option<u32>,
    protection: &Protection,
) -> Result<Vec<u8>, String> {
    let mut flags = 0u16;

    if protection.signing_key.is_some() {
        flags |= FLAG_SIGNED;
    }

    if protection.encryption_key.is_some() {
        flags |= FLAG_ENCRYPTED;
    }

    if metadata.bound_to.is_some() {
        flags |= FLAG_BOUND;
    }

    let nonce_value = match nonce {
        None => 0,
        Some(nonce) if nonce <= NONCE_MAXIMUM => {
            flags |= FLAG_NONCE;

            nonce
        }
        Some(nonce) => {
            return Err(format!(
                "Expected a nonce of 0 to {NONCE_MAXIMUM}, but got {nonce}."
            ));
        }
    };

    let mut out = b"ASNC".to_vec();
    out.push(SCHEMA_VERSION.0);
    out.push(SCHEMA_VERSION.1);
    out.extend_from_slice(&flags.to_be_bytes());
    out.extend_from_slice(&nonce_value.to_be_bytes()[1..]);
    let header = out;
    let mut contents = Vec::new();
    encode_metadata(metadata, &mut contents)?;
    encode(nodes, &mut contents);
    let mut encryption = Vec::new();

    // Encrypted first and then signed, so that the signature is checked
    // without the key to decrypt with.
    if let Some(key) = &protection.encryption_key {
        let iv = match protection.iv {
            Some(iv) => iv,
            None => {
                let mut iv = [0; IV_SIZE];
                getrandom::fill(&mut iv).map_err(|error| format!("No random bytes: {error}."))?;

                iv
            }
        };
        let mut aad = header.clone();
        aad.extend_from_slice(&key.id);
        aad.extend_from_slice(&iv);
        let tag = Aes256Gcm::new((&key.key).into())
            .encrypt_in_place_detached(Nonce::from_slice(&iv), &aad, &mut contents)
            .map_err(|_| "Could not encrypt.".to_string())?;

        encryption.extend_from_slice(&key.id);
        encryption.extend_from_slice(&iv);
        encryption.extend_from_slice(&tag);
        debug_assert_eq!(encryption.len(), KEY_ID_SIZE + IV_SIZE + TAG_SIZE);
    }

    let mut signature = Vec::new();

    if let Some(key) = &protection.signing_key {
        debug_assert_eq!(header.len(), HEADER_SIZE);
        let mut hash = Sha256::new();
        hash.update(&header);
        hash.update(key.id);
        hash.update(&encryption);
        hash.update(&contents);
        let digest = hash.finalize();

        signature.extend_from_slice(&key.id);
        signature.extend_from_slice(&SigningKey::from_bytes(&key.key).sign(&digest).to_bytes());
    }

    let mut file = header;
    file.extend_from_slice(&signature);
    file.extend_from_slice(&encryption);
    file.extend_from_slice(&contents);

    Ok(file)
}

/// The contents of an `.asnst` file.
pub fn write_asnst(metadata: &Metadata, tree: &Json) -> Result<Vec<u8>, String> {
    if metadata.bound_to.is_some() {
        return Err("Only an .asnc file can be bound to something.".to_string());
    }

    let mut out = Vec::new();
    encode_head(MAJOR_MAP, 4, &mut out);
    encode_text("format", &mut out);
    encode_text("asnst", &mut out);
    encode_text("version", &mut out);
    encode_head(MAJOR_ARRAY, 2, &mut out);
    encode_head(MAJOR_UNSIGNED, u64::from(SCHEMA_VERSION.0), &mut out);
    encode_head(MAJOR_UNSIGNED, u64::from(SCHEMA_VERSION.1), &mut out);
    encode_text("metadata", &mut out);
    encode_metadata(metadata, &mut out)?;
    encode_text("tree", &mut out);
    encode(tree, &mut out);

    Ok(out)
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
    fn the_header_of_a_file_is_the_magic_the_version_the_flags_and_the_nonce() {
        let file = write_asnc(
            &Metadata::default(),
            &Json::Null,
            None,
            &Protection::default(),
        )
        .unwrap();

        assert_eq!(&file[..12], b"ASNC\x01\x00\x00\x00\x00\x00\x00\xa0");

        let metadata = Metadata {
            bound_to: Some("corpus".to_string()),
            ..Metadata::default()
        };
        let file = write_asnc(&metadata, &Json::Null, Some(5), &Protection::default()).unwrap();

        assert_eq!(&file[..11], b"ASNC\x01\x00\x00\x0c\x00\x00\x05");
    }

    fn from_hex(text: &str) -> Vec<u8> {
        (0..text.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&text[index..index + 2], 16).unwrap())
            .collect()
    }

    /// Made by `asnkitrt.syntax.container` with the same keys and the same
    /// initialization vector.
    #[test]
    fn a_signed_and_encrypted_file_is_that_of_python() {
        let metadata = Metadata {
            generator: Some("asnkit 1.0.0".to_string()),
            bound_to: Some("x".to_string()),
            ..Metadata::default()
        };
        let protection = Protection {
            signing_key: Some(Key {
                id: [0; KEY_ID_SIZE],
                key: std::array::from_fn(|index| index as u8),
            }),
            encryption_key: Some(Key {
                id: [1; KEY_ID_SIZE],
                key: std::array::from_fn(|index| 100 + index as u8),
            }),
            iv: Some(std::array::from_fn(|index| 1 + index as u8)),
        };
        let nodes: Json = serde_json::from_str("[1, 2, 3]").unwrap();

        assert_eq!(
            write_asnc(&metadata, &nodes, Some(9), &protection).unwrap(),
            from_hex(
                "41534e430100000f0000090000000000000000000000000000000027d9b7896fcbbacfb96d88a51a6331a3d1fe71fed2ef6bef2f1ab9a3765c7d2e0222c003946048aea3ad45997024e3a87cd8ba619e5c04962c4ed1427bfc920b010101010101010101010101010101010102030405060708090a0b0cd24fa64fb8a5a51296ee216e551ca5058204e02bc60b2cc31799248e27dab76a5812e7f93b6b0dce76d21659285768d6b33a15e8487448"
            )
        );
    }

    #[test]
    fn the_iv_is_random_if_not_given() {
        let protection = Protection {
            encryption_key: Some(Key {
                id: [1; KEY_ID_SIZE],
                key: [2; KEY_SIZE],
            }),
            ..Protection::default()
        };
        let write = || write_asnc(&Metadata::default(), &Json::Null, None, &protection).unwrap();

        assert_ne!(
            write()[11 + KEY_ID_SIZE..][..IV_SIZE],
            write()[11 + KEY_ID_SIZE..][..IV_SIZE]
        );
    }

    #[test]
    fn key_ids_are_derived_as_python_does() {
        // The first bytes of the SHA-256 of the key, or of its public key.
        let key = [7; KEY_SIZE];

        assert_eq!(
            derive_key_id(&key, false)[..],
            Sha256::digest(key)[..KEY_ID_SIZE]
        );
        assert_ne!(derive_key_id(&key, true), derive_key_id(&key, false));
    }
}
