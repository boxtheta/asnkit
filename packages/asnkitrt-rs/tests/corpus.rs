// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads the files of `examples/corpus`, whole and corrupt, and the
//! same files changed in ways that each check has to catch.

use std::fs;
use std::path::PathBuf;

use asnkitrt::Error;
use asnkitrt::cbor::Cbor;
use asnkitrt::container::{self, Checks, HEADER_SIZE, crc32};

fn corpus(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/corpus")
        .join(name);

    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

/// `data` with the CRC-32 of its header made right again, as after a
/// change that is not corruption.
fn with_crc(mut data: Vec<u8>) -> Vec<u8> {
    let crc = crc32(&data[HEADER_SIZE..]);
    data[6..HEADER_SIZE].copy_from_slice(&crc.to_be_bytes());

    data
}

fn read(data: &[u8]) -> Error {
    container::read(data, &Checks::default()).unwrap_err()
}

#[test]
fn plain_file() {
    let asni = container::read(&corpus("plain.asni"), &Checks::default()).unwrap();
    let metadata = &asni.header.metadata;

    assert_eq!(asni.header.version, (2, 0));
    assert!(metadata.generator.is_some());
    assert_eq!(metadata.sources.len(), 1);
    assert_eq!(metadata.sources[0].name.as_deref(), Some("foo.asn"));

    // A CompiledSpecification, kind 1, with its list of modules.
    let Cbor::Array(top) = &asni.nodes else {
        panic!("not a node")
    };
    assert_eq!(top[0], Cbor::Int(1));

    let header = container::read_header(&corpus("plain.asni")).unwrap();
    assert_eq!(header, asni.header);
}

#[test]
fn corrupt_file() {
    let corrupt = corpus("corrupt.asni");

    assert_eq!(read(&corrupt), Error::BadChecksum);
    assert_eq!(
        container::read_header(&corrupt).unwrap_err(),
        Error::BadChecksum
    );

    // It is the plain file, with one bit of it flipped.
    let plain = corpus("plain.asni");
    let differences: Vec<u8> = plain.iter().zip(&corrupt).map(|(a, b)| a ^ b).collect();
    assert_eq!(plain.len(), corrupt.len());
    assert_eq!(
        differences
            .iter()
            .map(|byte| byte.count_ones())
            .sum::<u32>(),
        1
    );
    // Whose CRC-32 is left as it was.
    assert_eq!(corrupt[..HEADER_SIZE], plain[..HEADER_SIZE]);
}

#[test]
fn every_flipped_bit_is_caught() {
    let plain = corpus("plain.asni");

    for index in HEADER_SIZE..plain.len() {
        for bit in 0..8 {
            let mut data = plain.clone();
            data[index] ^= 1 << bit;
            assert_eq!(read(&data), Error::BadChecksum, "byte {index} bit {bit}");
        }
    }

    // So is a bit flipped in the CRC-32 itself.
    for index in 6..HEADER_SIZE {
        let mut data = plain.clone();
        data[index] ^= 0x80;
        assert_eq!(read(&data), Error::BadChecksum, "byte {index}");
    }
}

#[test]
fn bad_files() {
    let plain = corpus("plain.asni");

    assert_eq!(read(b""), Error::NotAsni);
    assert_eq!(read(b"ASN"), Error::NotAsni);
    assert_eq!(read(b"ASNI\x02\x00\x00\x00\x00"), Error::NotAsni);
    assert_eq!(read(b"{\"format\": \"asnst\"}"), Error::NotAsni);

    // Version 1, which had a nonce, a signature and encryption, and
    // any other major version, before the CRC-32 is looked at.
    for major in [1, 3] {
        let mut data = plain.clone();
        data[4] = major;
        assert_eq!(read(&data), Error::UnsupportedVersion, "{major}");
    }

    // A later minor version is read: the CRC-32 does not cover it.
    let mut data = plain.clone();
    data[5] = 9;
    assert!(container::read(&data, &Checks::default()).is_ok());

    // Data after the nodes, and the nodes cut short, are caught by the
    // CRC-32, and by the CBOR when the CRC-32 is made to match.
    let mut data = plain.clone();
    data.push(0);
    assert_eq!(read(&data), Error::BadChecksum);
    assert_eq!(read(&with_crc(data)), Error::TrailingData);
    assert_eq!(read(&plain[..plain.len() - 1]), Error::BadChecksum);
    assert_eq!(
        read(&with_crc(plain[..plain.len() - 1].to_vec())),
        Error::OutOfData
    );

    // Cut anywhere, it is an error and not a panic.
    for size in 0..plain.len() {
        assert!(
            container::read(&plain[..size], &Checks::default()).is_err(),
            "size {size}"
        );
    }
}

#[test]
fn a_binding_is_not_looked_at() {
    // The `bound_to` that a file of 2.0.0-rc4 could have in its metadata
    // is a key a reader does not know of, and ignores.
    let plain = corpus("plain.asni");
    let (_, nodes_at) = asnkitrt::cbor::parse_first(&plain, HEADER_SIZE, 64).unwrap();
    let mut entry = vec![0x68];
    entry.extend_from_slice(b"bound_to");
    entry.push(0x66);
    entry.extend_from_slice(b"corpus");

    let mut data = plain.clone();
    assert_eq!(data[HEADER_SIZE], 0xa2, "a map of generator and sources");
    data[HEADER_SIZE] = 0xa3;
    data.splice(nodes_at..nodes_at, entry);

    let asni = container::read(&with_crc(data), &Checks::default()).unwrap();
    let expected = container::read(&plain, &Checks::default()).unwrap();
    assert_eq!(asni, expected);
}

#[test]
fn nesting_is_limited() {
    let checks = Checks { max_depth: 3 };

    assert_eq!(
        container::read(&corpus("plain.asni"), &checks).unwrap_err(),
        Error::TooDeep
    );
}
