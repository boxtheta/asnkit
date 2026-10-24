// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

//! The Rust code asnkit-gen-rust-codec makes from compiled
//! specifications: the files in `tests/generated` are what it makes now,
//! and they are built here and used. The vectors of `tests/vectors` are
//! in `tests/vectors`, which is a crate of its own.
//!
//! To make them again after a change of the plugin:
//!
//!   UPDATE_GENERATED=1 cargo test -p asnkit-gen-rust-codec --test generated

use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::fs;
use std::path::PathBuf;

use asnkit_gen_rust_codec::{Integer, Options, generate};
use asnkitrt::container::Checks;
use asnkitrt::runtime::{self, Specification};
use asnkitrt::types::Codec;
use asnkitrt::wire::{self, Asn1};

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/rfc5280.rs"]
mod rfc5280;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes.rs"]
mod shapes;

/// Counts the allocations of each thread, as the tests run in threads of
/// their own.
struct Counting;

thread_local! {
    static ALLOCATIONS: Cell<usize> = const { Cell::new(0) };
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let _ = ALLOCATIONS.try_with(|count| count.set(count.get() + 1));

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// How many allocations `f` makes on this thread.
fn allocations(f: impl FnOnce()) -> usize {
    let before = ALLOCATIONS.with(Cell::get);

    f();

    ALLOCATIONS.with(Cell::get) - before
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn is_current(asni: &str, generated: &str, options: Options) {
    let data = fs::read(root().join(asni)).unwrap();
    let (text, _) = generate(&data, options).unwrap();
    let path = root().join(generated);

    if std::env::var_os("UPDATE_GENERATED").is_some() {
        fs::write(&path, &text).unwrap();
    }

    assert!(
        fs::read_to_string(&path).unwrap() == text,
        "{generated} is not what the plugin makes: run the tests with UPDATE_GENERATED=1."
    );
}

#[test]
fn the_generated_files_are_current() {
    is_current(
        "../../examples/compiled/rfc5280.asni",
        "tests/generated/rfc5280.rs",
        Options {
            integer: Integer::Big,
            ..Options::default()
        },
    );
    is_current(
        "../asnkit-gen-rust/tests/generated/shapes.asni",
        "tests/generated/shapes.rs",
        Options {
            skip_unsupported: true,
            ..Options::default()
        },
    );
    is_current(
        "../asnkit-gen-rust/tests/generated/shapes.asni",
        "tests/generated/shapes_no_std.rs",
        Options {
            skip_unsupported: true,
            no_std: true,
            ..Options::default()
        },
    );
}

#[test]
fn what_is_not_supported_is_an_error_unless_it_is_skipped() {
    let data = fs::read(root().join("../asnkit-gen-rust/tests/generated/shapes.asni")).unwrap();
    let error = generate(&data, Options::default()).unwrap_err();

    assert!(error.0.contains("Shapes.Builtins"), "{error}");
    assert!(error.0.contains("REAL"), "{error}");

    let (text, _) = generate(
        &data,
        Options {
            skip_unsupported: true,
            ..Options::default()
        },
    )
    .unwrap();

    assert!(text.contains("//   Shapes.Builtins (REAL)"));
    assert!(!text.contains("pub struct Builtins"));
}

fn certificates() -> Vec<(String, Vec<u8>)> {
    let mut found: Vec<_> = fs::read_dir(root().join("../asnkitrt-rs/tests/fixtures/x509"))
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
    assert!(found.len() >= 30);

    found
}

#[test]
fn the_certificates_of_openssl_come_back_as_they_were_without_allocating() {
    let mut out = vec![0u8; 16384];

    for (name, data) in certificates() {
        let mut length = 0;
        let count = allocations(|| {
            let certificate = rfc5280::Certificate::decode_der(&data)
                .unwrap_or_else(|error| panic!("{name}: {error}"));

            length = certificate.encode_der(&mut out).unwrap();
        });

        assert_eq!(&out[..length], &data[..], "{name}");
        assert_eq!(count, 0, "{name}: decoding and encoding allocate");
    }
}

#[test]
fn the_certificates_come_back_through_ber_and_uper() {
    for (name, data) in certificates() {
        let certificate = rfc5280::Certificate::decode_der(&data).unwrap();

        // UPER of the decoded value, without allocating, which is a value
        // that is decoded from its UPER.
        let mut uper = vec![0u8; certificate.uper_len().unwrap()];
        let count = allocations(|| {
            certificate.encode_uper(&mut uper).unwrap();
        });

        assert_eq!(count, 0, "{name}");

        let from_uper = rfc5280::Certificate::decode_uper(&uper).unwrap();

        assert_eq!(from_uper, certificate, "{name}");
        assert_eq!(from_uper.to_der().unwrap(), data, "{name}");

        // DER is BER, of which a time in UTC keeps its offset, where DER has
        // none, as the runtime does.
        let from_ber = rfc5280::Certificate::decode_ber(&data).unwrap();

        assert_eq!(from_ber.to_der().unwrap(), data, "{name}");
    }
}

#[test]
fn the_certificates_are_what_the_runtime_makes_of_them() {
    let asni = fs::read(root().join("../../examples/compiled/rfc5280.asni")).unwrap();

    for codec in [Codec::Der, Codec::Uper] {
        let specification =
            Specification::from_asni(&asni, &Checks::default(), runtime::Options::new(codec))
                .unwrap();

        for (name, data) in certificates() {
            let certificate = rfc5280::Certificate::decode_der(&data).unwrap();
            let generated = match codec {
                Codec::Der => certificate.to_der().unwrap(),
                _ => certificate.to_uper().unwrap(),
            };
            let value = specification.decode("Certificate", &generated).unwrap();

            assert_eq!(
                specification.encode("Certificate", &value).unwrap(),
                generated,
                "{name} {codec:?}"
            );
        }
    }
}

#[test]
fn a_value_tells_the_size_of_the_buffer_it_needs() {
    let data = &certificates()[0].1;
    let certificate = rfc5280::Certificate::decode_der(data).unwrap();
    let mut small = [0u8; 10];

    assert_eq!(
        certificate.encode_der(&mut small),
        Err(wire::Error::BufferTooSmall(data.len()))
    );
    assert_eq!(certificate.der_len().unwrap(), data.len());
}

/// The specification of shapes, loaded by the runtime for a codec.
fn runtime_shapes(codec: Codec) -> Specification {
    let asni = fs::read(root().join("../asnkit-gen-rust/tests/generated/shapes.asni")).unwrap();

    Specification::from_asni(&asni, &Checks::default(), runtime::Options::new(codec)).unwrap()
}

/// `value` of the type `name` comes back as it was with each codec, and
/// the runtime decodes what the code encodes and encodes it the same.
fn comes_back<'a, T: Asn1<'a> + PartialEq + std::fmt::Debug>(name: &str, value: &T) {
    // The encodings live as long as the test, as what is decoded from
    // them borrows them.
    let leak = |bytes: Vec<u8>| -> &'static [u8] { Box::leak(bytes.into_boxed_slice()) };
    let ber = leak(value.to_ber().unwrap());
    let der = leak(value.to_der().unwrap());
    let uper = leak(value.to_uper().unwrap());

    assert_eq!(&T::decode_ber(ber).unwrap(), value, "{name} BER");
    assert_eq!(&T::decode_der(der).unwrap(), value, "{name} DER");
    assert_eq!(&T::decode_uper(uper).unwrap(), value, "{name} UPER");

    for (codec, encoded) in [(Codec::Ber, ber), (Codec::Der, der), (Codec::Uper, uper)] {
        let specification = runtime_shapes(codec);
        let decoded = specification
            .decode(name, encoded)
            .unwrap_or_else(|error| panic!("{name} {codec:?}: {error}"));

        assert_eq!(
            specification.encode(name, &decoded).unwrap(),
            encoded,
            "{name} {codec:?}"
        );
    }
}

#[test]
fn the_shapes_come_back_as_the_runtime_has_them() {
    use shapes::*;

    comes_back(
        "Keywords",
        &Keywords {
            r#type: 7,
            self_: true,
            r#match: Some("übermatch".into()),
            foo_bar: -100,
            foo_bar_2: 65535,
            crate_: (),
        },
    );
    comes_back(
        "Sizes",
        &Sizes {
            a: 1,
            b: 70000,
            c: 5_000_000_000,
            d: -1,
            e: -30000,
            f: -3_000_000_000,
            g: i64::MIN,
            h: i64::MAX,
        },
    );

    let many = [
        Shape::Circle(3),
        Shape::Nothing(()),
        Shape::Named("ring".into()),
    ];

    for shape in [
        Shape::Circle(100),
        Shape::Square(ShapeSquare {
            side: 1,
            color: Some(Color::Violet),
        }),
        Shape::Square(ShapeSquare {
            side: 1,
            color: Some(Color::Green),
        }),
        Shape::Many(wire::SequenceOf::new(&many)),
        Shape::Octets(wire::Octets::new(&[0, 1, 2, 0xff])),
    ] {
        comes_back("Shape", &shape);
    }

    comes_back(
        "Tree",
        &Tree {
            value: 1,
            left: Some(Box::new(Tree {
                value: 2,
                left: None,
                right: Some(Box::new(Node::Leaf(3))),
            })),
            right: Some(Box::new(Node::Tree(Box::new(Tree {
                value: 4,
                left: None,
                right: None,
            })))),
        },
    );

    let list = [1, 2, 9];

    for defaults in [
        Defaults {
            count: 3,
            enabled: true,
            color: Color::Green,
            list: None,
            anything: None,
        },
        Defaults {
            count: 10,
            enabled: false,
            color: Color::DarkBlue,
            list: Some(wire::SequenceOf::new(&list)),
            anything: Some(Any { x: 1 }),
        },
    ] {
        comes_back("Defaults", &defaults);
    }

    for extensible in [
        Extensible {
            first: 1,
            second: None,
            third: None,
            fourth: None,
        },
        Extensible {
            first: 1,
            second: Some(2),
            third: Some(3),
            fourth: Some("four".into()),
        },
    ] {
        comes_back("Extensible", &extensible);
    }

    let list = [OuterListItem { value: 2 }, OuterListItem { value: 0 }];

    comes_back(
        "Outer",
        &Outer {
            inner: OuterInner2 { value: 1 },
            list: wire::SequenceOf::new(&list),
            choice: OuterChoice::B(OuterChoiceB { value: 3 }),
        },
    );
}

#[test]
fn der_sorts_a_set_of() {
    use shapes::*;

    let pairs = [
        PairsItem { a: 9, b: true },
        PairsItem { a: 1, b: false },
        PairsItem { a: 1, b: true },
    ];
    let value = wire::SequenceOf::new(&pairs);
    let der = encode(|out| pairs::encode_der(&value, out));
    let ber = encode(|out| pairs::encode_ber(&value, out));
    let decoded = pairs::decode_der(&der).unwrap();
    let order: Vec<(u8, bool)> = decoded.iter().map(|pair| (pair.a, pair.b)).collect();

    assert_eq!(order, [(1, false), (1, true), (9, true)]);
    // BER keeps the order, which DER does not decode.
    assert_ne!(ber, der);
    assert!(pairs::decode_der(&ber).is_err());
    assert_eq!(pairs::decode_ber(&ber).unwrap(), value);

    let specification = runtime_shapes(Codec::Der);

    assert_eq!(
        specification
            .encode("Pairs", &specification.decode("Pairs", &der).unwrap())
            .unwrap(),
        der
    );
}

fn encode(f: impl Fn(&mut [u8]) -> wire::Result<usize>) -> Vec<u8> {
    let mut out = vec![0; 1024];
    let length = f(&mut out).unwrap();

    out.truncate(length);
    out
}

#[test]
fn der_leaves_out_defaults_and_does_not_decode_them() {
    use shapes::*;

    let value = Defaults {
        count: 3,
        enabled: true,
        color: Color::Green,
        list: None,
        anything: None,
    };

    assert_eq!(value.to_der().unwrap(), [0x30, 0x00]);
    // With the count of its default.
    assert!(Defaults::decode_der(&[0x30, 0x03, 0x80, 0x01, 0x03]).is_err());
    assert_eq!(
        Defaults::decode_ber(&[0x30, 0x03, 0x80, 0x01, 0x03]).unwrap(),
        value
    );
}

#[test]
fn deep_data_is_an_error_and_not_a_crash() {
    // Each byte is a tree on the left of another, as deep as there are.
    let deep = vec![0xff; 100_000];

    assert!(shapes::Tree::decode_uper(&deep).is_err());

    // In BER, a [1] of a [1] of ...
    let mut ber = Vec::new();

    for _ in 0..10_000 {
        ber.extend_from_slice(&[0x30, 0x80, 0xa1, 0x80]);
    }

    assert!(shapes::Tree::decode_ber(&ber).is_err());
}

#[test]
fn a_value_the_enumeration_does_not_know_is_an_error() {
    // A square of the color of the first addition, and of the second, which
    // it does not have: the index of the alternative of Shape (001), if
    // there is a color (1), the side less 1 (0000000), an addition (1),
    // which is small (0) and its index.
    assert_eq!(
        shapes::Shape::decode_uper(&[0x30, 0x10, 0x00]).unwrap(),
        shapes::Shape::Square(shapes::ShapeSquare {
            side: 1,
            color: Some(shapes::Color::Violet),
        })
    );
    assert!(shapes::Shape::decode_uper(&[0x30, 0x10, 0x20]).is_err());
    assert_eq!(shapes::Color::from_number(4), None);
}

#[test]
fn borrowed_values_borrow_the_data() {
    let data = &certificates()[0].1;
    let certificate = rfc5280::Certificate::decode_der(data).unwrap();
    let serial = certificate.tbs_certificate.serial_number;
    let signature = certificate.signature;

    // The bits of the signature are those of the data, not a copy.
    let bytes: Vec<u8> = signature.bytes().collect();
    let at = data
        .windows(bytes.len())
        .position(|window| window == bytes)
        .unwrap();

    assert!(at > 0);
    assert!(!serial.is_negative());
}
