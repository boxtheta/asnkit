// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! X.509, which OpenSSL writes and reads and which was not written by the
//! same people, nor read the same way: the certificates in
//! `tests/fixtures/x509` (see `make.py`) are decoded and encoded again
//! with the DER codec, to come out as OpenSSL wrote them.
//!
//! The tests that run `openssl` are skipped if there is none.

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The compiled specification of RFC 5280 that is tried: the one in the
/// repository, or another, as when it is made by the compiler in Rust:
///
///   ASNKIT_RFC5280_ASNI=/tmp/rfc5280.asni cargo test --test x509
fn rfc5280_path() -> PathBuf {
    std::env::var_os("ASNKIT_RFC5280_ASNI").map_or_else(
        || root().join("../../examples/compiled/rfc5280.asni"),
        PathBuf::from,
    )
}

fn rfc5280(codec: Codec) -> Specification {
    let data = fs::read(rfc5280_path()).unwrap();

    Specification::from_asni(&data, &Checks::default(), Options::new(codec)).unwrap()
}

fn certificates() -> Vec<(String, Vec<u8>)> {
    let mut found: Vec<_> = fs::read_dir(root().join("tests/fixtures/x509"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .filter(|path| path.extension().is_some_and(|extension| extension == "der"))
        .map(|path| {
            (
                path.file_stem().unwrap().to_string_lossy().into_owned(),
                fs::read(path).unwrap(),
            )
        })
        .collect();

    found.sort();
    found
}

/// The type of the value of each extension that is decoded.
const EXTENSION_TYPES: [(&str, &str); 15] = [
    ("2.5.29.9", "SubjectDirectoryAttributes"),
    ("2.5.29.14", "SubjectKeyIdentifier"),
    ("2.5.29.15", "KeyUsage"),
    ("2.5.29.17", "SubjectAltName"),
    ("2.5.29.18", "IssuerAltName"),
    ("2.5.29.19", "BasicConstraints"),
    ("2.5.29.20", "CRLNumber"),
    ("2.5.29.30", "NameConstraints"),
    ("2.5.29.31", "CRLDistributionPoints"),
    ("2.5.29.32", "CertificatePolicies"),
    ("2.5.29.35", "AuthorityKeyIdentifier"),
    ("2.5.29.36", "PolicyConstraints"),
    ("2.5.29.37", "ExtKeyUsageSyntax"),
    ("2.5.29.54", "InhibitAnyPolicy"),
    ("1.3.6.1.5.5.7.1.1", "AuthorityInfoAccessSyntax"),
];

fn openssl(arguments: &[&str], input: &[u8]) -> Option<std::process::Output> {
    let mut child = Command::new("openssl")
        .args(arguments)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .ok()?;

    child.stdin.take().unwrap().write_all(input).ok()?;
    child.wait_with_output().ok()
}

#[test]
fn there_are_certificates() {
    let names: Vec<_> = certificates().into_iter().map(|(name, _)| name).collect();

    assert!(names.len() > 20, "{names:?}");

    for name in ["key_rsa2048", "key_p256", "key_ed25519", "extensions_all"] {
        assert!(names.iter().any(|candidate| candidate == name), "{name}");
    }
}

#[test]
fn round_trip() {
    let der = rfc5280(Codec::Der);

    for (name, data) in certificates() {
        let decoded = der
            .decode("Certificate", &data)
            .unwrap_or_else(|e| panic!("{name}: {e}"));

        der.check_constraints("Certificate", &decoded)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(der.encode("Certificate", &decoded).unwrap(), data, "{name}");
    }
}

#[test]
fn what_der_wrote_ber_reads() {
    let ber = rfc5280(Codec::Ber);

    for (name, data) in certificates() {
        // Not the same value, as DER reads a time as UTC, and BER does
        // not write the seconds of a time when they are zero.
        let decoded = ber
            .decode("Certificate", &data)
            .unwrap_or_else(|e| panic!("{name}: {e}"));

        ber.encode("Certificate", &decoded)
            .unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn the_parts() {
    let der = rfc5280(Codec::Der);

    for (name, data) in certificates() {
        let certificate = der.decode("Certificate", &data).unwrap();
        let tbs = certificate.get("tbsCertificate").unwrap();
        let encoded = der.encode("TBSCertificate", tbs).unwrap();

        assert!(
            data.windows(encoded.len()).any(|window| window == encoded),
            "{name}"
        );
        assert_eq!(
            &der.decode("TBSCertificate", &encoded).unwrap(),
            tbs,
            "{name}"
        );
        assert_eq!(
            der.decode_length(&data).unwrap(),
            Some(data.len()),
            "{name}"
        );
        assert_eq!(
            der.decode_length(&data[..5]).unwrap(),
            Some(data.len()),
            "{name}"
        );
        assert_eq!(der.decode_length(&data[..1]).unwrap(), None, "{name}");
    }
}

#[test]
fn extensions() {
    let der = rfc5280(Codec::Der);
    let mut seen = std::collections::BTreeSet::new();

    for (name, data) in certificates() {
        let certificate = der.decode("Certificate", &data).unwrap();
        let Some(Value::List(extensions)) = certificate
            .get("tbsCertificate")
            .and_then(|tbs| tbs.get("extensions"))
        else {
            continue;
        };

        for extension in extensions {
            let Some(Value::Text(id)) = extension.get("extnID") else {
                panic!("{name}")
            };
            let Some(Value::Bytes(value)) = extension.get("extnValue") else {
                panic!("{name}")
            };
            let Some((_, type_name)) = EXTENSION_TYPES.iter().find(|(known, _)| known == id) else {
                continue;
            };

            seen.insert(*type_name);

            let decoded = der
                .decode(type_name, value)
                .unwrap_or_else(|e| panic!("{name} {type_name}: {e}"));

            assert_eq!(
                &der.encode(type_name, &decoded).unwrap(),
                value,
                "{name} {type_name}"
            );
        }
    }

    assert!(seen.len() >= 12, "{seen:?}");
}

/// Changes a value in a certificate and has OpenSSL read it.
#[test]
fn what_is_encoded_openssl_reads() {
    let der = rfc5280(Codec::Der);
    let Some((_, data)) = certificates()
        .into_iter()
        .find(|(name, _)| name == "key_p256")
    else {
        panic!("There is no certificate to start with.");
    };
    let mut certificate = der.decode("Certificate", &data).unwrap();

    // A serial number that is more than 64 bits, and a version.
    let Value::Map(members) = &mut certificate else {
        panic!()
    };
    let Value::Map(tbs) = &mut members
        .iter_mut()
        .find(|(name, _)| name == "tbsCertificate")
        .unwrap()
        .1
    else {
        panic!()
    };

    tbs.iter_mut()
        .find(|(name, _)| name == "serialNumber")
        .unwrap()
        .1 = Value::from_twos_complement(&[
        0x01, 0x23, 0x45, 0x67, 0x89, 0xab, 0xcd, 0xef, 0x01, 0x23, 0x45,
    ]);

    let encoded = der.encode("Certificate", &certificate).unwrap();
    let Some(output) = openssl(&["x509", "-inform", "DER", "-noout", "-serial"], &encoded) else {
        eprintln!("openssl is not there: skipped");
        return;
    };

    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "serial=0123456789ABCDEF012345"
    );

    let parsed = openssl(&["asn1parse", "-inform", "DER"], &encoded).unwrap();

    assert!(
        parsed.status.success(),
        "{}",
        String::from_utf8_lossy(&parsed.stderr)
    );
}

/// Not DER is not read as DER, but BER reads it.
#[test]
fn not_der() {
    let der = rfc5280(Codec::Der);
    let ber = rfc5280(Codec::Ber);
    let lenient = {
        let data = fs::read(rfc5280_path()).unwrap();
        let mut options = Options::new(Codec::Der);

        options.lenient = true;

        Specification::from_asni(&data, &Checks::default(), options).unwrap()
    };

    for (name, data) in certificates().into_iter().take(5) {
        // The outer length in a long form it need not have.
        let mut padded = vec![data[0], 0x84, 0, 0, 0, 0];
        let length = data.len() - 4;

        padded[2] = (length >> 24) as u8;
        padded[3] = (length >> 16) as u8;
        padded[4] = (length >> 8) as u8;
        padded[5] = length as u8;
        padded.extend_from_slice(&data[4..]);

        assert!(der.decode("Certificate", &padded).is_err(), "{name}");
        assert!(ber.decode("Certificate", &padded).is_ok(), "{name}");
        assert!(lenient.decode("Certificate", &padded).is_ok(), "{name}");
    }
}
