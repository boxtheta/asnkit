// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads an `.asnc` file: its header, its signature, its nonce, its
//! encryption, what it is bound to, its metadata and its nodes, in the
//! order of `docs/file_formats.md`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::cbor::{self, Cbor};
use crate::{Error, Result};

pub const HEADER_SIZE: usize = 11;
pub const KEY_ID_SIZE: usize = 16;
pub const KEY_SIZE: usize = 32;
pub const SIGNATURE_SIZE: usize = 64;
pub const IV_SIZE: usize = 12;
pub const TAG_SIZE: usize = 16;

/// Of what a file is bound to, in bytes as UTF-8.
pub const BOUND_TO_MAXIMUM_SIZE: usize = 255;

const MAGIC: &[u8; 4] = b"ASNC";
const MAJOR_VERSION: u8 = 1;

const FLAG_SIGNED: u16 = 0x0001;
const FLAG_ENCRYPTED: u16 = 0x0002;
const FLAG_NONCE: u16 = 0x0004;
const FLAG_BOUND: u16 = 0x0008;
const SUPPORTED_FLAGS: u16 = FLAG_SIGNED | FLAG_ENCRYPTED | FLAG_NONCE | FLAG_BOUND;

/// A key with its identifier: an Ed25519 public key to verify with, or
/// an AES-256 key to decrypt with.
#[derive(Clone)]
pub struct Key {
    pub id: [u8; KEY_ID_SIZE],
    pub key: [u8; KEY_SIZE],
}

/// How a reader treats the signature of a file.
#[derive(Clone, Copy, Default)]
pub enum Signature<'a> {
    /// Rejects every file that is signed.
    #[default]
    None,
    /// Rejects every file that is not signed by one of these.
    Keys(&'a [Key]),
    /// Reads a file whether it is signed or not, and does not check
    /// the signature. For looking at a file.
    Any,
}

/// How a reader treats the nonce of a file.
#[derive(Clone, Copy, Default)]
pub enum Nonce<'a> {
    /// Rejects every file with a nonce to check.
    #[default]
    None,
    /// Rejects every file but one with this nonce.
    Exact(u32),
    /// Rejects every file but one with a nonce it returns `true` for.
    Accept(&'a dyn Fn(u32) -> bool),
    /// Reads a file whatever its nonce is.
    Any,
}

/// How a reader treats what a file is bound to.
#[derive(Clone, Copy, Default)]
pub enum Binding<'a> {
    /// Rejects every file that is bound.
    #[default]
    None,
    /// Rejects every file that is not bound to this.
    To(&'a str),
    /// Reads a file whatever it is bound to.
    Any,
}

/// What a file has to be to be read. Reads nothing by default but a
/// file that is not signed, not encrypted, with no nonce and not bound.
#[derive(Clone, Copy)]
pub struct Checks<'a> {
    pub signature: Signature<'a>,
    pub nonce: Nonce<'a>,
    /// The keys to decrypt with. Rejects every file that is not
    /// encrypted if given.
    pub decryption_keys: Option<&'a [Key]>,
    pub bound_to: Binding<'a>,
    /// How deep the CBOR may be nested.
    pub max_depth: usize,
}

impl Default for Checks<'_> {
    fn default() -> Self {
        Checks {
            signature: Signature::None,
            nonce: Nonce::None,
            decryption_keys: None,
            bound_to: Binding::None,
            max_depth: cbor::DEFAULT_MAX_DEPTH,
        }
    }
}

/// An ASN.1 file a compiled specification was made from.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Source {
    pub name: Option<String>,
    pub sha256: Option<String>,
    pub text: Option<String>,
}

/// What is known about a compiled specification.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Metadata {
    pub generator: Option<String>,
    pub sources: Vec<Source>,
    /// Only there in a file that is bound.
    pub bound_to: Option<String>,
    pub other: Option<Cbor>,
}

/// The header of a file. The metadata is not there in an encrypted
/// file, as it is not known without the key.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub version: (u8, u8),
    pub metadata: Option<Metadata>,
    /// There if the nonce is to be checked.
    pub nonce: Option<u32>,
    pub signed_by: Option<[u8; KEY_ID_SIZE]>,
    pub encrypted_with: Option<[u8; KEY_ID_SIZE]>,
}

/// A file that is read: what it says about itself and its nodes in
/// the compact form.
#[derive(Debug, Clone, PartialEq)]
pub struct Asnc {
    pub header: Header,
    pub metadata: Metadata,
    pub nodes: Cbor,
}

struct Prefix<'a> {
    version: (u8, u8),
    flags: u16,
    nonce: Option<u32>,
    signed_by: Option<[u8; KEY_ID_SIZE]>,
    signature: &'a [u8],
    encrypted_with: Option<[u8; KEY_ID_SIZE]>,
    iv: &'a [u8],
    tag: &'a [u8],
    /// Where the metadata is.
    size: usize,
}

fn read_prefix(data: &[u8]) -> Result<Prefix<'_>> {
    if data.len() < HEADER_SIZE || &data[..4] != MAGIC {
        return Err(Error::NotAsnc);
    }

    let flags = u16::from_be_bytes([data[6], data[7]]);

    if data[4] != MAJOR_VERSION {
        return Err(Error::UnsupportedVersion);
    }

    if flags & !SUPPORTED_FLAGS != 0 {
        return Err(Error::UnsupportedFlags);
    }

    let mut offset = HEADER_SIZE;
    let mut take = |size: usize| -> Result<&[u8]> {
        let bytes = data.get(offset..offset + size).ok_or(Error::OutOfData)?;
        offset += size;

        Ok(bytes)
    };
    let id = |bytes: &[u8]| -> [u8; KEY_ID_SIZE] { bytes.try_into().unwrap() };
    let mut prefix = Prefix {
        version: (data[4], data[5]),
        flags,
        nonce: None,
        signed_by: None,
        signature: &[],
        encrypted_with: None,
        iv: &[],
        tag: &[],
        size: 0,
    };

    if flags & FLAG_NONCE != 0 {
        prefix.nonce = Some(u32::from_be_bytes([0, data[8], data[9], data[10]]));
    }

    if flags & FLAG_SIGNED != 0 {
        prefix.signed_by = Some(id(take(KEY_ID_SIZE)?));
        prefix.signature = take(SIGNATURE_SIZE)?;
    }

    if flags & FLAG_ENCRYPTED != 0 {
        prefix.encrypted_with = Some(id(take(KEY_ID_SIZE)?));
        prefix.iv = take(IV_SIZE)?;
        prefix.tag = take(TAG_SIZE)?;
    }

    prefix.size = offset;

    Ok(prefix)
}

fn find_key<'a>(keys: &'a [Key], id: &[u8; KEY_ID_SIZE]) -> Option<&'a Key> {
    keys.iter().find(|key| &key.id == id)
}

fn read_text(map: &Cbor, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        None => Ok(None),
        Some(Cbor::Text(text)) => Ok(Some(text.clone())),
        Some(_) => Err(Error::BadMetadata),
    }
}

fn read_metadata(primitive: &Cbor, is_bound: bool) -> Result<Metadata> {
    if !matches!(primitive, Cbor::Map(_)) {
        return Err(Error::BadMetadata);
    }

    let mut metadata = Metadata::default();

    // Only looked at in a bound file.
    if is_bound {
        let bound_to = read_text(primitive, "bound_to")?.ok_or(Error::BadMetadata)?;

        if bound_to.len() > BOUND_TO_MAXIMUM_SIZE {
            return Err(Error::BadMetadata);
        }

        metadata.bound_to = Some(bound_to);
    }

    metadata.generator = read_text(primitive, "generator")?;

    match primitive.get("sources") {
        None => {}
        Some(Cbor::Array(sources)) => {
            for source in sources {
                if !matches!(source, Cbor::Map(_)) {
                    return Err(Error::BadMetadata);
                }

                metadata.sources.push(Source {
                    name: read_text(source, "name")?,
                    sha256: read_text(source, "sha256")?,
                    text: read_text(source, "text")?,
                });
            }
        }
        Some(_) => return Err(Error::BadMetadata),
    }

    // Anything not known is that of a later minor version.
    metadata.other = primitive.get("other").cloned();

    Ok(metadata)
}

/// Returns the header of `data`, with its metadata unless the file is
/// encrypted. Nothing of the file is checked: not its signature, not
/// its nonce and not what it is bound to.
pub fn read_header(data: &[u8]) -> Result<Header> {
    read_header_with(data, cbor::DEFAULT_MAX_DEPTH)
}

fn read_header_with(data: &[u8], max_depth: usize) -> Result<Header> {
    let prefix = read_prefix(data)?;
    let mut metadata = None;

    if prefix.encrypted_with.is_none() {
        let (primitive, _) = cbor::parse_first(data, prefix.size, max_depth)?;
        metadata = Some(read_metadata(&primitive, prefix.flags & FLAG_BOUND != 0)?);
    }

    Ok(header_of(&prefix, metadata))
}

fn header_of(prefix: &Prefix<'_>, metadata: Option<Metadata>) -> Header {
    Header {
        version: prefix.version,
        metadata,
        nonce: prefix.nonce,
        signed_by: prefix.signed_by,
        encrypted_with: prefix.encrypted_with,
    }
}

fn check_signature(data: &[u8], prefix: &Prefix<'_>, signature: Signature<'_>) -> Result<()> {
    let keys = match signature {
        Signature::Any => return Ok(()),
        Signature::None if prefix.signed_by.is_none() => return Ok(()),
        Signature::None => return Err(Error::NoVerificationKeys),
        Signature::Keys(_) if prefix.signed_by.is_none() => return Err(Error::NotSigned),
        Signature::Keys(keys) => keys,
    };
    let signed_by = prefix.signed_by.ok_or(Error::NotSigned)?;
    let key = find_key(keys, &signed_by).ok_or(Error::UnknownSigner)?;

    verify(data, prefix, key)
}

#[cfg(feature = "signing")]
fn verify(data: &[u8], prefix: &Prefix<'_>, key: &Key) -> Result<()> {
    use ed25519_dalek::{Signature, VerifyingKey};
    use sha2::{Digest, Sha256};

    // All of the file but the signature.
    let signature_offset = HEADER_SIZE + KEY_ID_SIZE;
    let digest = Sha256::new()
        .chain_update(&data[..signature_offset])
        .chain_update(&data[signature_offset + SIGNATURE_SIZE..])
        .finalize();
    let signature = Signature::from_slice(prefix.signature).map_err(|_| Error::BadSignature)?;

    VerifyingKey::from_bytes(&key.key)
        .and_then(|key| key.verify_strict(&digest, &signature))
        .map_err(|_| Error::BadSignature)
}

#[cfg(not(feature = "signing"))]
fn verify(_: &[u8], _: &Prefix<'_>, _: &Key) -> Result<()> {
    Err(Error::FeatureMissing)
}

fn check_nonce(nonce: Option<u32>, check: Nonce<'_>) -> Result<()> {
    match (check, nonce) {
        (Nonce::Any, _) | (Nonce::None, None) => Ok(()),
        (Nonce::None, Some(_)) | (_, None) => Err(Error::BadNonce),
        (Nonce::Exact(expected), Some(nonce)) if nonce == expected => Ok(()),
        (Nonce::Accept(accept), Some(nonce)) if accept(nonce) => Ok(()),
        _ => Err(Error::BadNonce),
    }
}

fn check_binding(bound_to: Option<&str>, expected: Binding<'_>) -> Result<()> {
    match (expected, bound_to) {
        (Binding::Any, _) | (Binding::None, None) => Ok(()),
        // Byte for byte as UTF-8, and no more of them than there can be.
        (Binding::To(expected), Some(bound_to))
            if expected.len() <= BOUND_TO_MAXIMUM_SIZE && expected == bound_to =>
        {
            Ok(())
        }
        _ => Err(Error::BadBinding),
    }
}

/// The metadata and the nodes of `data`, decrypted if they are
/// encrypted.
fn decrypt(data: &[u8], prefix: &Prefix<'_>, keys: Option<&[Key]>) -> Result<Vec<u8>> {
    let contents = &data[prefix.size..];

    let Some(encrypted_with) = prefix.encrypted_with else {
        return match keys {
            Some(_) => Err(Error::NotEncrypted),
            None => Ok(Vec::from(contents)),
        };
    };
    let keys = keys.ok_or(Error::NoDecryptionKeys)?;
    let key = find_key(keys, &encrypted_with).ok_or(Error::UnknownEncryptionKey)?;

    decrypt_with(data, prefix, key, contents)
}

#[cfg(feature = "encryption")]
fn decrypt_with(data: &[u8], prefix: &Prefix<'_>, key: &Key, contents: &[u8]) -> Result<Vec<u8>> {
    use aes_gcm::aead::{AeadInPlace, KeyInit};
    use aes_gcm::{Aes256Gcm, Nonce, Tag};

    // The first 11 bytes of the file, the identifier of the key and
    // the initialization vector.
    let mut aad = Vec::with_capacity(HEADER_SIZE + KEY_ID_SIZE + IV_SIZE);
    aad.extend_from_slice(&data[..HEADER_SIZE]);
    aad.extend_from_slice(&key.id);
    aad.extend_from_slice(prefix.iv);

    let mut decrypted = Vec::from(contents);

    Aes256Gcm::new((&key.key).into())
        .decrypt_in_place_detached(
            Nonce::from_slice(prefix.iv),
            &aad,
            &mut decrypted,
            Tag::from_slice(prefix.tag),
        )
        .map_err(|_| Error::BadTag)?;

    Ok(decrypted)
}

#[cfg(not(feature = "encryption"))]
fn decrypt_with(_: &[u8], _: &Prefix<'_>, _: &Key, _: &[u8]) -> Result<Vec<u8>> {
    Err(Error::FeatureMissing)
}

/// Reads the `.asnc` file in `data`, after the checks.
///
/// Nothing of the metadata or the nodes is read as CBOR before the
/// signature, the nonce and the tag are checked.
pub fn read(data: &[u8], checks: &Checks<'_>) -> Result<Asnc> {
    let prefix = read_prefix(data)?;

    check_signature(data, &prefix, checks.signature)?;
    check_nonce(prefix.nonce, checks.nonce)?;

    let contents = decrypt(data, &prefix, checks.decryption_keys)?;
    let (primitive, offset) = cbor::parse_first(&contents, 0, checks.max_depth)?;
    let metadata = read_metadata(&primitive, prefix.flags & FLAG_BOUND != 0)?;

    check_binding(metadata.bound_to.as_deref(), checks.bound_to)?;

    let (nodes, offset) = cbor::parse_first(&contents, offset, checks.max_depth)?;

    if offset != contents.len() {
        return Err(Error::TrailingData);
    }

    Ok(Asnc {
        header: header_of(
            &prefix,
            prefix.encrypted_with.is_none().then(|| metadata.clone()),
        ),
        metadata,
        nodes,
    })
}
