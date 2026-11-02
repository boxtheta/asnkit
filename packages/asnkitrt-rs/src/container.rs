// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads an `.asni` file: its header, its checksum, its metadata and its
//! nodes, in the order of `docs/file_formats.md`.

use alloc::string::String;
use alloc::vec::Vec;

use crate::cbor::{self, Cbor};
use crate::{Error, Result};

/// The magic, the version and the CRC-32 of the rest of the file.
pub const HEADER_SIZE: usize = 10;

const MAGIC: &[u8; 4] = b"ASNI";
const MAJOR_VERSION: u8 = 2;

/// The table of CRC-32 as zlib, gzip and PNG compute it (CRC-32/ISO-HDLC:
/// the polynomial 0x04C11DB7 reflected, starting from and ended with all
/// ones), a byte at a time.
const CRC32_TABLE: [u32; 256] = {
    let mut table = [0u32; 256];
    let mut index = 0;

    while index < 256 {
        let mut crc = index as u32;
        let mut bit = 0;

        while bit < 8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
            bit += 1;
        }

        table[index] = crc;
        index += 1;
    }

    table
};

/// The CRC-32 of `data`, as zlib, gzip and PNG compute it: that of
/// `b"123456789"` is `0xcbf43926`.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;

    for &byte in data {
        crc = CRC32_TABLE[((crc ^ u32::from(byte)) & 0xff) as usize] ^ (crc >> 8);
    }

    !crc
}

/// What a file has to be to be read.
#[derive(Clone, Copy)]
pub struct Checks {
    /// How deep the CBOR may be nested.
    pub max_depth: usize,
}

impl Default for Checks {
    fn default() -> Self {
        Checks {
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

/// Checks the header of `data` and the CRC-32 of the rest, and returns
/// the version.
fn read_prefix(data: &[u8]) -> Result<(u8, u8)> {
    if data.len() < HEADER_SIZE || &data[..4] != MAGIC {
        return Err(Error::NotAsni);
    }

    if data[4] != MAJOR_VERSION {
        return Err(Error::UnsupportedVersion);
    }

    let expected = u32::from_be_bytes([data[6], data[7], data[8], data[9]]);

    if crc32(&data[HEADER_SIZE..]) != expected {
        return Err(Error::BadChecksum);
    }

    Ok((data[4], data[5]))
}

fn read_text(map: &Cbor, key: &str) -> Result<Option<String>> {
    match map.get(key) {
        None => Ok(None),
        Some(Cbor::Text(text)) => Ok(Some(text.clone())),
        Some(_) => Err(Error::BadMetadata),
    }
}

fn read_metadata(primitive: &Cbor) -> Result<Metadata> {
    if !matches!(primitive, Cbor::Map(_)) {
        return Err(Error::BadMetadata);
    }

    let mut metadata = Metadata {
        generator: read_text(primitive, "generator")?,
        ..Metadata::default()
    };

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

/// Returns the header of `data`, after checking its version and the
/// CRC-32 of the file. The nodes are not read.
pub fn read_header(data: &[u8]) -> Result<Header> {
    let version = read_prefix(data)?;
    let (primitive, _) = cbor::parse_first(data, HEADER_SIZE, cbor::DEFAULT_MAX_DEPTH)?;

    Ok(Header {
        version,
        metadata: read_metadata(&primitive)?,
    })
}

/// Reads the `.asni` file in `data`, after the checks.
///
/// Nothing of the file is read as CBOR before its CRC-32 is checked.
pub fn read(data: &[u8], checks: &Checks) -> Result<Asni> {
    let version = read_prefix(data)?;
    let (primitive, offset) = cbor::parse_first(data, HEADER_SIZE, checks.max_depth)?;
    let metadata = read_metadata(&primitive)?;
    let (nodes, offset) = cbor::parse_first(data, offset, checks.max_depth)?;

    if offset != data.len() {
        return Err(Error::TrailingData);
    }

    Ok(Asni {
        header: Header { version, metadata },
        nodes,
    })
}

#[cfg(test)]
mod tests {
    use super::crc32;

    #[test]
    fn crc32_is_that_of_zlib() {
        assert_eq!(crc32(b""), 0);
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
        assert_eq!(
            crc32(b"The quick brown fox jumps over the lazy dog"),
            0x414f_a339
        );
    }
}
