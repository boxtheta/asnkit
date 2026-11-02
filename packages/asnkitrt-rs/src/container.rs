// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads an `.asni` file: its header, what it is bound to, its metadata
//! and its nodes, in the order of `docs/file_formats.md`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::cbor::{self, Cbor};
use crate::{Error, Result};

pub const HEADER_SIZE: usize = 8;

/// Of what a file is bound to, in bytes as UTF-8.
pub const BOUND_TO_MAXIMUM_SIZE: usize = 255;

const MAGIC: &[u8; 4] = b"ASNI";
const MAJOR_VERSION: u8 = 2;

const FLAG_BOUND: u16 = 0x0001;
const SUPPORTED_FLAGS: u16 = FLAG_BOUND;

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
/// file that is not bound.
#[derive(Clone, Copy)]
pub struct Checks<'a> {
    pub bound_to: Binding<'a>,
    /// How deep the CBOR may be nested.
    pub max_depth: usize,
}

impl Default for Checks<'_> {
    fn default() -> Self {
        Checks {
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

/// The header of a file.
#[derive(Debug, Clone, PartialEq)]
pub struct Header {
    pub version: (u8, u8),
    pub metadata: Metadata,
}

/// A file that is read: what it says about itself and its nodes in
/// the compact form.
#[derive(Debug, Clone, PartialEq)]
pub struct Asni {
    pub header: Header,
    pub nodes: Cbor,
}

/// The version and the flags of a file, which are all of its header
/// but its metadata.
struct Prefix {
    version: (u8, u8),
    flags: u16,
}

fn read_prefix(data: &[u8]) -> Result<Prefix> {
    if data.len() < HEADER_SIZE || &data[..4] != MAGIC {
        return Err(Error::NotAsni);
    }

    if data[4] != MAJOR_VERSION {
        return Err(Error::UnsupportedVersion);
    }

    let flags = u16::from_be_bytes([data[6], data[7]]);

    if flags & !SUPPORTED_FLAGS != 0 {
        return Err(Error::UnsupportedFlags);
    }

    Ok(Prefix {
        version: (data[4], data[5]),
        flags,
    })
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

/// Returns the header of `data`. Nothing of the file is checked but
/// its version and its flags, and not what it is bound to.
pub fn read_header(data: &[u8]) -> Result<Header> {
    let prefix = read_prefix(data)?;
    let (primitive, _) = cbor::parse_first(data, HEADER_SIZE, cbor::DEFAULT_MAX_DEPTH)?;

    Ok(Header {
        version: prefix.version,
        metadata: read_metadata(&primitive, prefix.flags & FLAG_BOUND != 0)?,
    })
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

/// Reads the `.asni` file in `data`, after the checks.
///
/// Nothing of the nodes is read as CBOR before what the file is bound
/// to is checked.
pub fn read(data: &[u8], checks: &Checks<'_>) -> Result<Asni> {
    let prefix = read_prefix(data)?;
    let (primitive, offset) = cbor::parse_first(data, HEADER_SIZE, checks.max_depth)?;
    let metadata = read_metadata(&primitive, prefix.flags & FLAG_BOUND != 0)?;

    check_binding(metadata.bound_to.as_deref(), checks.bound_to)?;

    let (nodes, offset) = cbor::parse_first(data, offset, checks.max_depth)?;

    if offset != data.len() {
        return Err(Error::TrailingData);
    }

    Ok(Asni {
        header: Header {
            version: prefix.version,
            metadata,
        },
        nodes,
    })
}
