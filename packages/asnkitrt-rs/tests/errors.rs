// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! An error with a value tells where in it it is, as the runtime for
//! Python does: `Certificate.tbsCertificate.extensions[1].critical`.

use std::fs;
use std::path::PathBuf;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;
use asnkitrt::{Error, Step};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rfc5280(codec: Codec) -> Specification {
    let data = fs::read(root().join("../../examples/compiled/rfc5280.asni")).unwrap();

    Specification::from_asni(&data, &Checks::default(), Options::new(codec)).unwrap()
}

/// A certificate with extensions, as DER decodes it.
fn certificate() -> Value<'static> {
    let data = fs::read(root().join("tests/fixtures/x509/extensions_all.der")).unwrap();

    rfc5280(Codec::Der)
        .decode("Certificate", &data)
        .unwrap()
        .into_owned()
}

fn member<'a, 'v>(value: &'a mut Value<'v>, name: &str) -> &'a mut Value<'v> {
    let Value::Map(members) = value else {
        panic!("not a map");
    };

    &mut members
        .iter_mut()
        .find(|(candidate, _)| candidate == name)
        .unwrap()
        .1
}

/// The certificate, with `critical` of its second extension set to
/// `critical`.
fn with_critical(critical: Value) -> Value {
    let mut value = certificate();
    let extensions = member(member(&mut value, "tbsCertificate"), "extensions");
    let Value::List(items) = extensions else {
        panic!("not a list");
    };
    let Value::Map(extension) = &mut items[1] else {
        panic!("not a map");
    };

    extension.retain(|(name, _)| name != "critical");
    extension.insert(1, ("critical".into(), critical));

    value
}

const CRITICAL: &str = "Certificate.tbsCertificate.extensions[1].critical";

/// The extensions of `certificate`. JER and XER do not encode a whole
/// certificate, which has an ANY.
fn extensions(mut certificate: Value) -> Value {
    member(member(&mut certificate, "tbsCertificate"), "extensions").clone()
}

#[test]
fn encoding_tells_where() {
    let value = with_critical(Value::text("yes"));

    for codec in [Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper] {
        let error = rfc5280(codec).encode("Certificate", &value).unwrap_err();

        assert!(matches!(error, Error::Encode(..)), "{codec:?}: {error:?}");
        assert!(
            error.to_string().starts_with(&format!("{CRITICAL}: ")),
            "{codec:?}: {error}"
        );
        assert_eq!(
            error.location().unwrap().steps(),
            [
                Step::Name("Certificate".into()),
                Step::Name("tbsCertificate".into()),
                Step::Name("extensions".into()),
                Step::Index(1),
                Step::Name("critical".into()),
            ],
            "{codec:?}"
        );
    }

    let mut codecs = vec![];

    if cfg!(feature = "jer") {
        codecs.push(Codec::Jer);
    }

    if cfg!(feature = "xer") {
        codecs.push(Codec::Xer);
    }

    if cfg!(feature = "gser") {
        codecs.push(Codec::Gser);
    }

    for codec in codecs {
        let error = rfc5280(codec)
            .encode("Extensions", &extensions(value.clone()))
            .unwrap_err();

        assert!(matches!(error, Error::Encode(..)), "{codec:?}: {error:?}");
        assert!(
            error.to_string().starts_with("Extensions[1].critical: "),
            "{codec:?}: {error}"
        );
    }
}

#[test]
fn indented_encoding_and_a_module_tell_where() {
    if cfg!(feature = "jer") {
        let error = rfc5280(Codec::Jer)
            .encode_indented("Extensions", &extensions(with_critical(Value::Null)), 2)
            .unwrap_err();

        assert!(
            error.to_string().starts_with("Extensions[1].critical: "),
            "{error}"
        );
    }

    let error = rfc5280(Codec::Der)
        .encode_in(
            "PKIX1Explicit88",
            "Certificate",
            &with_critical(Value::Null),
        )
        .unwrap_err();

    assert!(error.to_string().starts_with(CRITICAL), "{error}");
}

#[test]
fn decoding_tells_where() {
    let value = with_critical(Value::Boolean(true));

    // A BOOLEAN of 0x01, which DER does not allow.
    let der = rfc5280(Codec::Der);
    let mut encoded = der.encode("Certificate", &value).unwrap();
    let at = encoded
        .windows(3)
        .position(|window| window == [0x01, 0x01, 0xff])
        .unwrap();

    encoded[at + 2] = 0x01;

    let error = der.decode("Certificate", &encoded).unwrap_err();

    // The first extension that is critical.
    assert!(matches!(error, Error::Decode(..)), "{error:?}");
    assert!(
        error
            .to_string()
            .starts_with("Certificate.tbsCertificate.extensions["),
        "{error}"
    );
    assert!(
        error
            .to_string()
            .contains("].critical: Expected BOOLEAN contents of 0x00 or 0xff."),
        "{error}"
    );

    // Data that ends in a member tells which. (BER and DER find that the
    // certificate is longer than the data first.)
    for codec in [Codec::Oer, Codec::Per, Codec::Uper] {
        let spec = rfc5280(codec);
        let encoded = spec.encode("Certificate", &value).unwrap();
        let error = spec
            .decode("Certificate", &encoded[..encoded.len() / 2])
            .unwrap_err();

        assert!(
            error.to_string().starts_with("Certificate.tbsCertificate"),
            "{codec:?}: {error}"
        );
    }

    if cfg!(feature = "jer") {
        let jer = rfc5280(Codec::Jer);
        let mut items = extensions(value.clone());

        // Only the second is critical.
        if let Value::List(items) = &mut items {
            items.truncate(2);
            *member(&mut items[0], "critical") = Value::Boolean(false);
        }

        let text = String::from_utf8(jer.encode("Extensions", &items).unwrap()).unwrap();
        let text = text.replacen("\"critical\":true", "\"critical\":1", 1);
        let error = jer.decode("Extensions", text.as_bytes()).unwrap_err();

        assert_eq!(
            error.to_string(),
            "Extensions[1].critical: Expected a JSON boolean."
        );
    }

    if cfg!(feature = "xer") {
        let xer = rfc5280(Codec::Xer);
        let text =
            String::from_utf8(xer.encode("Extensions", &extensions(value)).unwrap()).unwrap();
        let text = text.replacen("<critical><true /></critical>", "<critical>1</critical>", 1);
        let error = xer.decode("Extensions", text.as_bytes()).unwrap_err();

        assert!(error.to_string().starts_with("Extensions["), "{error}");
        assert!(error.to_string().contains("].critical: "), "{error}");
    }
}

#[test]
fn constraints_tell_where() {
    let mut value = certificate();

    *member(member(&mut value, "tbsCertificate"), "extensions") = Value::List(vec![]);

    let error = rfc5280(Codec::Der)
        .check_constraints("Certificate", &value)
        .unwrap_err();

    assert!(matches!(error, Error::Constraints(..)), "{error:?}");
    assert_eq!(
        error.to_string(),
        "Certificate.tbsCertificate.extensions: Expected a list of a length in the range."
    );
}

#[test]
fn errors_that_are_not_of_a_value_tell_nothing_more() {
    let error = rfc5280(Codec::Der)
        .encode("Nothing", &Value::Null)
        .unwrap_err();

    assert_eq!(error, Error::NoSuchType);
    assert_eq!(error.location(), None);
    assert_eq!(
        Error::encode("Expected data of type bool.").to_string(),
        "Expected data of type bool."
    );
}

#[test]
fn what_is_not_supported_in_a_value_tells_where() {
    if cfg!(feature = "jer") {
        let error = rfc5280(Codec::Jer)
            .encode("Certificate", &certificate())
            .unwrap_err();

        assert!(matches!(error, Error::Unsupported(..)), "{error:?}");
        assert_eq!(
            error.to_string(),
            "Certificate.tbsCertificate.issuer.rdnSequence[0][0].value: JER needs the type of the value of an ANY or an open type."
        );
        assert_eq!(error.location().unwrap().steps().len(), 7);
    }

    // What the codec does not do is not about the value.
    let error = rfc5280(Codec::Der)
        .encode_indented("Certificate", &certificate(), 2)
        .unwrap_err();

    assert_eq!(
        error,
        Error::unsupported("Only JER, XER and GSER are indented.")
    );
    assert_eq!(error.location(), None);
    assert_eq!(error.to_string(), "Only JER, XER and GSER are indented.");
}
