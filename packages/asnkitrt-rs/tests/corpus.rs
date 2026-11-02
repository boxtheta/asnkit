// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads the files of `examples/corpus`, bound and not, and the same
//! files changed in ways that each check has to catch.

use std::fs;
use std::path::PathBuf;

use asnkitrt::Error;
use asnkitrt::cbor::Cbor;
use asnkitrt::container::{self, Binding, Checks, HEADER_SIZE};

fn corpus(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/corpus")
        .join(name);

    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

const FILES: [(&str, bool); 2] = [("plain.asni", false), ("bound.asni", true)];

/// The checks that read a file that is bound as told, and no other.
fn checks(bound: bool) -> Checks<'static> {
    Checks {
        bound_to: if bound {
            Binding::To("corpus")
        } else {
            Binding::None
        },
        ..Checks::default()
    }
}

#[test]
fn both_files() {
    let mut nodes = Vec::new();

    for (name, bound) in FILES {
        let asni = container::read(&corpus(name), &checks(bound))
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let metadata = &asni.header.metadata;

        assert_eq!(asni.header.version.0, 2, "{name}");
        assert_eq!(
            metadata.bound_to.as_deref(),
            bound.then_some("corpus"),
            "{name}"
        );
        assert!(metadata.generator.is_some(), "{name}");
        assert_eq!(metadata.sources.len(), 1, "{name}");
        assert_eq!(
            metadata.sources[0].name.as_deref(),
            Some("foo.asn"),
            "{name}"
        );

        // A CompiledSpecification, kind 1, with its list of modules.
        let Cbor::Array(top) = &asni.nodes else {
            panic!("{name}: not a node")
        };
        assert_eq!(top[0], Cbor::Int(1), "{name}");

        nodes.push(asni.nodes);
    }

    // Whether it is bound or not, it holds the same.
    assert_eq!(nodes[0], nodes[1]);
}

#[test]
fn header_without_checks() {
    for (name, bound) in FILES {
        let header = container::read_header(&corpus(name)).unwrap();

        assert_eq!(header.metadata.bound_to.is_some(), bound, "{name}");
    }
}

#[test]
fn any_binding_reads_both() {
    let checks = Checks {
        bound_to: Binding::Any,
        ..Checks::default()
    };

    container::read(&corpus("plain.asni"), &checks).unwrap();
    container::read(&corpus("bound.asni"), &checks).unwrap();
}

#[test]
fn wrong_binding() {
    let read = |name: &str, bound_to| {
        container::read(
            &corpus(name),
            &Checks {
                bound_to,
                ..Checks::default()
            },
        )
        .unwrap_err()
    };

    assert_eq!(read("bound.asni", Binding::None), Error::BadBinding);
    assert_eq!(read("plain.asni", Binding::To("corpus")), Error::BadBinding);
    assert_eq!(read("bound.asni", Binding::To("corpu")), Error::BadBinding);
    assert_eq!(
        read("bound.asni", Binding::To("corpus ")),
        Error::BadBinding
    );
}

#[test]
fn bad_files() {
    let plain = corpus("plain.asni");
    let read = |data: &[u8]| container::read(data, &Checks::default()).unwrap_err();

    assert_eq!(read(b""), Error::NotAsni);
    assert_eq!(read(b"ASN"), Error::NotAsni);
    assert_eq!(read(b"{\"format\": \"asnst\"}"), Error::NotAsni);

    // Version 1, which had a nonce, a signature and encryption, and
    // any other major version.
    for major in [1, 3] {
        let mut data = plain.clone();
        data[4] = major;
        assert_eq!(read(&data), Error::UnsupportedVersion, "{major}");
    }

    // A later minor version is read.
    let mut data = plain.clone();
    data[5] = 9;
    assert!(container::read(&data, &Checks::default()).is_ok());

    // The flags of version 1 among them, which are no longer known.
    for flags in [0x0002u16, 0x0004, 0x0008, 0x8000, 0xfffe] {
        let mut data = plain.clone();
        data[6..HEADER_SIZE].copy_from_slice(&flags.to_be_bytes());
        assert_eq!(read(&data), Error::UnsupportedFlags, "{flags:x}");
    }

    // Data after the nodes, and the nodes cut short.
    let mut data = plain.clone();
    data.push(0);
    assert_eq!(read(&data), Error::TrailingData);
    assert_eq!(read(&plain[..plain.len() - 1]), Error::OutOfData);

    // Cut anywhere, it is an error and not a panic.
    for (name, bound) in FILES {
        let data = corpus(name);

        for size in 0..data.len() {
            assert!(
                container::read(&data[..size], &checks(bound)).is_err(),
                "{name} size {size}"
            );
        }
    }
}

#[test]
fn the_bound_flag_and_the_metadata_agree() {
    // Without the flag, what the metadata says it is bound to is not
    // looked at.
    let mut data = corpus("bound.asni");
    data[7] = 0;
    let asni = container::read(&data, &Checks::default()).unwrap();
    assert_eq!(asni.header.metadata.bound_to, None);

    // With the flag, the metadata has to say what it is bound to.
    let mut data = corpus("plain.asni");
    data[7] = 1;
    assert_eq!(
        container::read(&data, &checks(true)).unwrap_err(),
        Error::BadMetadata
    );
}

#[test]
fn nesting_is_limited() {
    let checks = Checks {
        max_depth: 3,
        ..Checks::default()
    };

    assert_eq!(
        container::read(&corpus("plain.asni"), &checks).unwrap_err(),
        Error::TooDeep
    );
}
