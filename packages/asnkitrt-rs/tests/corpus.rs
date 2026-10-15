// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Reads the files of `examples/corpus`, signed, encrypted and bound in
//! every combination, and the same files changed in ways that each
//! check has to catch.

use std::fs;
use std::path::PathBuf;

use asnkitrt::Error;
use asnkitrt::cbor::Cbor;
use asnkitrt::container::{self, Binding, Checks, HEADER_SIZE, KEY_ID_SIZE, Key, Nonce, Signature};

fn corpus(name: &str) -> Vec<u8> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples/corpus")
        .join(name);

    fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()))
}

fn hex(text: &str) -> Vec<u8> {
    let text = text.trim();

    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn id(hex_digits: &str) -> [u8; KEY_ID_SIZE] {
    hex(hex_digits).try_into().unwrap()
}

fn signing_key() -> Key {
    Key {
        id: id("d543cae4811fc8b033704dc6e7896ff7"),
        key: hex(&String::from_utf8(corpus("signing.pub")).unwrap())
            .try_into()
            .unwrap(),
    }
}

fn encryption_key() -> Key {
    Key {
        id: id("16cde2f77598fc5472df7570a510f54d"),
        key: hex(&String::from_utf8(corpus("encryption.key")).unwrap())
            .try_into()
            .unwrap(),
    }
}

const FILES: [(&str, bool, bool, bool); 8] = [
    ("plain.asnc", false, false, false),
    ("signed.asnc", true, false, false),
    ("encrypted.asnc", false, true, false),
    ("bound.asnc", false, false, true),
    ("signed-encrypted.asnc", true, true, false),
    ("signed-bound.asnc", true, false, true),
    ("encrypted-bound.asnc", false, true, true),
    ("signed-encrypted-bound.asnc", true, true, true),
];

/// The checks that read a file that is signed, encrypted and bound as
/// told, and no other.
fn checks<'a>(
    signing: &'a [Key],
    encryption: &'a [Key],
    signed: bool,
    encrypted: bool,
    bound: bool,
) -> Checks<'a> {
    Checks {
        signature: if signed {
            Signature::Keys(signing)
        } else {
            Signature::None
        },
        decryption_keys: encrypted.then_some(encryption),
        bound_to: if bound {
            Binding::To("corpus")
        } else {
            Binding::None
        },
        ..Checks::default()
    }
}

#[test]
fn every_combination() {
    let signing = [signing_key()];
    let encryption = [encryption_key()];
    let mut nodes = Vec::new();

    for (name, signed, encrypted, bound) in FILES {
        let data = corpus(name);
        let asnc = container::read(
            &data,
            &checks(&signing, &encryption, signed, encrypted, bound),
        )
        .unwrap_or_else(|error| panic!("{name}: {error}"));

        assert_eq!(asnc.header.version.0, 1, "{name}");
        assert_eq!(asnc.header.nonce, None, "{name}");
        assert_eq!(
            asnc.header.signed_by,
            signed.then(|| signing[0].id),
            "{name}"
        );
        assert_eq!(
            asnc.header.encrypted_with,
            encrypted.then(|| encryption[0].id),
            "{name}"
        );
        assert_eq!(
            asnc.metadata.bound_to.as_deref(),
            bound.then_some("corpus"),
            "{name}"
        );
        assert!(asnc.metadata.generator.is_some(), "{name}");
        assert_eq!(asnc.metadata.sources.len(), 1, "{name}");
        assert_eq!(
            asnc.metadata.sources[0].name.as_deref(),
            Some("foo.asn"),
            "{name}"
        );

        // The metadata is not known without the key.
        assert_eq!(asnc.header.metadata.is_none(), encrypted, "{name}");

        // A CompiledSpecification, kind 1, with its list of modules.
        let Cbor::Array(top) = &asnc.nodes else {
            panic!("{name}: not a node")
        };
        assert_eq!(top[0], Cbor::Int(1), "{name}");

        nodes.push(asnc.nodes);
    }

    // Whatever it is signed, encrypted or bound, it holds the same.
    assert!(nodes.windows(2).all(|pair| pair[0] == pair[1]));
}

#[test]
fn header_without_checks() {
    for (name, signed, encrypted, bound) in FILES {
        let header = container::read_header(&corpus(name)).unwrap();

        assert_eq!(header.signed_by.is_some(), signed, "{name}");
        assert_eq!(header.encrypted_with.is_some(), encrypted, "{name}");

        match header.metadata {
            None => assert!(encrypted, "{name}"),
            Some(metadata) => assert_eq!(metadata.bound_to.is_some(), bound, "{name}"),
        }
    }
}

#[test]
fn any_signature_reads_both() {
    let encryption = [encryption_key()];

    for (name, _, encrypted, bound) in FILES {
        let checks = Checks {
            signature: Signature::Any,
            decryption_keys: encrypted.then_some(&encryption[..]),
            bound_to: if bound {
                Binding::To("corpus")
            } else {
                Binding::None
            },
            ..Checks::default()
        };

        container::read(&corpus(name), &checks).unwrap_or_else(|error| panic!("{name}: {error}"));
    }
}

#[test]
fn any_binding_reads_both() {
    let checks = Checks {
        bound_to: Binding::Any,
        ..Checks::default()
    };

    container::read(&corpus("plain.asnc"), &checks).unwrap();
    container::read(&corpus("bound.asnc"), &checks).unwrap();
}

#[test]
fn wrong_setup() {
    let signing = [signing_key()];
    let encryption = [encryption_key()];
    let other = [Key {
        id: [1; 16],
        key: [2; 32],
    }];
    let wrong_key = [Key {
        id: encryption[0].id,
        key: [0; 32],
    }];
    let wrong_signer = [Key {
        id: signing[0].id,
        key: other[0].key,
    }];

    let read =
        |name: &str, checks: Checks<'_>| container::read(&corpus(name), &checks).unwrap_err();
    let ok = |signed, encrypted, bound| checks(&signing, &encryption, signed, encrypted, bound);

    // Signature.
    assert_eq!(
        read("signed.asnc", Checks::default()),
        Error::NoVerificationKeys
    );
    assert_eq!(read("plain.asnc", ok(true, false, false)), Error::NotSigned);
    assert_eq!(
        read(
            "signed.asnc",
            Checks {
                signature: Signature::Keys(&other),
                ..Checks::default()
            }
        ),
        Error::UnknownSigner
    );
    assert_eq!(
        read(
            "signed.asnc",
            Checks {
                signature: Signature::Keys(&wrong_signer),
                ..Checks::default()
            }
        ),
        Error::BadSignature
    );

    // Encryption.
    assert_eq!(
        read("encrypted.asnc", Checks::default()),
        Error::NoDecryptionKeys
    );
    assert_eq!(
        read("plain.asnc", ok(false, true, false)),
        Error::NotEncrypted
    );
    assert_eq!(
        read(
            "encrypted.asnc",
            Checks {
                decryption_keys: Some(&other),
                ..Checks::default()
            }
        ),
        Error::UnknownEncryptionKey
    );
    assert_eq!(
        read(
            "encrypted.asnc",
            Checks {
                decryption_keys: Some(&wrong_key),
                ..Checks::default()
            }
        ),
        Error::BadTag
    );

    // Binding.
    assert_eq!(read("bound.asnc", Checks::default()), Error::BadBinding);
    assert_eq!(
        read("plain.asnc", ok(false, false, true)),
        Error::BadBinding
    );
    assert_eq!(
        read(
            "bound.asnc",
            Checks {
                bound_to: Binding::To("corpu"),
                ..Checks::default()
            }
        ),
        Error::BadBinding
    );
    assert_eq!(
        read(
            "bound.asnc",
            Checks {
                bound_to: Binding::To("corpus "),
                ..Checks::default()
            }
        ),
        Error::BadBinding
    );
    // Bound and encrypted: it is not known until it is decrypted.
    assert_eq!(
        read(
            "encrypted-bound.asnc",
            Checks {
                bound_to: Binding::To("other"),
                ..ok(false, true, false)
            }
        ),
        Error::BadBinding
    );

    // A file with no nonce when one is to be checked.
    assert_eq!(
        read(
            "plain.asnc",
            Checks {
                nonce: Nonce::Exact(1),
                ..Checks::default()
            }
        ),
        Error::BadNonce
    );
}

/// The file with the nonce flag and a nonce set. The signature of a
/// signed file is not made again, so it no longer fits.
fn with_nonce(mut data: Vec<u8>, nonce: u32) -> Vec<u8> {
    data[7] |= 0x04;
    data[8..HEADER_SIZE].copy_from_slice(&nonce.to_be_bytes()[1..]);

    data
}

#[test]
fn nonce() {
    let data = with_nonce(corpus("plain.asnc"), 0x010203);
    let read = |nonce| {
        container::read(
            &data,
            &Checks {
                nonce,
                ..Checks::default()
            },
        )
    };

    assert_eq!(read(Nonce::None).unwrap_err(), Error::BadNonce);
    assert_eq!(
        read(Nonce::Exact(0x010203)).unwrap().header.nonce,
        Some(0x010203)
    );
    assert_eq!(read(Nonce::Exact(0x010204)).unwrap_err(), Error::BadNonce);
    assert!(read(Nonce::Accept(&|nonce| nonce > 5)).is_ok());
    assert_eq!(
        read(Nonce::Accept(&|nonce| nonce < 5)).unwrap_err(),
        Error::BadNonce
    );
    assert!(read(Nonce::Any).is_ok());

    // Any nonce also reads a file with none.
    assert!(
        container::read(
            &corpus("plain.asnc"),
            &Checks {
                nonce: Nonce::Any,
                ..Checks::default()
            }
        )
        .is_ok()
    );
}

#[test]
fn signed_nonce_is_signed() {
    let signing = [signing_key()];
    let data = with_nonce(corpus("signed.asnc"), 7);
    let checks = Checks {
        signature: Signature::Keys(&signing),
        nonce: Nonce::Exact(7),
        ..Checks::default()
    };

    assert_eq!(
        container::read(&data, &checks).unwrap_err(),
        Error::BadSignature
    );
}

#[test]
fn every_byte_of_a_signed_encrypted_file_counts() {
    let signing = [signing_key()];
    let encryption = [encryption_key()];
    let checks = checks(&signing, &encryption, true, true, true);
    let data = corpus("signed-encrypted-bound.asnc");

    assert!(container::read(&data, &checks).is_ok());

    for index in 0..data.len() {
        let mut changed = data.clone();
        changed[index] ^= 0x01;

        assert!(
            container::read(&changed, &checks).is_err(),
            "byte {index} changed"
        );
    }
}

#[test]
fn every_byte_of_an_encrypted_file_counts() {
    let encryption = [encryption_key()];
    let checks = checks(&[], &encryption, false, true, false);
    let data = corpus("encrypted.asnc");

    for index in 0..data.len() {
        let mut changed = data.clone();
        changed[index] ^= 0x01;

        assert!(
            container::read(&changed, &checks).is_err(),
            "byte {index} changed"
        );
    }
}

#[test]
fn bad_files() {
    let plain = corpus("plain.asnc");
    let read = |data: &[u8]| container::read(data, &Checks::default()).unwrap_err();

    assert_eq!(read(b""), Error::NotAsnc);
    assert_eq!(read(b"ASN"), Error::NotAsnc);
    assert_eq!(read(b"{\"format\": \"asnst\"}"), Error::NotAsnc);

    let mut data = plain.clone();
    data[4] = 2;
    assert_eq!(read(&data), Error::UnsupportedVersion);

    // A later minor version is read.
    let mut data = plain.clone();
    data[5] = 9;
    assert!(container::read(&data, &Checks::default()).is_ok());

    for flags in [0x0010u16, 0x8000, 0xfff0] {
        let mut data = plain.clone();
        data[6..8].copy_from_slice(&flags.to_be_bytes());
        assert_eq!(read(&data), Error::UnsupportedFlags, "{flags:x}");
    }

    // Data after the nodes, and the nodes cut short.
    let mut data = plain.clone();
    data.push(0);
    assert_eq!(read(&data), Error::TrailingData);
    assert_eq!(read(&plain[..plain.len() - 1]), Error::OutOfData);

    // Cut anywhere, it is an error and not a panic.
    for size in 0..plain.len() {
        assert!(
            container::read(&plain[..size], &Checks::default()).is_err(),
            "size {size}"
        );
    }

    for name in ["signed.asnc", "encrypted.asnc", "signed-encrypted.asnc"] {
        let data = corpus(name);

        for size in 0..data.len() {
            let any = Checks {
                signature: Signature::Any,
                ..Checks::default()
            };
            assert!(
                container::read(&data[..size], &any).is_err(),
                "{name} size {size}"
            );
        }
    }
}

#[test]
fn nesting_is_limited() {
    let checks = Checks {
        max_depth: 3,
        ..Checks::default()
    };

    assert_eq!(
        container::read(&corpus("plain.asnc"), &checks).unwrap_err(),
        Error::TooDeep
    );
}
