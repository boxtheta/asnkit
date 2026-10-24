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

use asnkit_gen_rust_codec::{Encoding, Integer, Options, generate};
use asnkitrt::container::Checks;
use asnkitrt::runtime::{self, Specification};
use asnkitrt::types::Codec;
use asnkitrt::wire;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/rfc5280.rs"]
mod rfc5280;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes.rs"]
mod shapes;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/rfc5280_ber.rs"]
mod rfc5280_ber;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/rfc5280_uper.rs"]
mod rfc5280_uper;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes_ber.rs"]
mod shapes_ber;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "generated/shapes_uper.rs"]
mod shapes_uper;

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
    for codec in [Encoding::Ber, Encoding::Der, Encoding::Uper] {
        let suffix = if codec == Encoding::Der {
            String::new()
        } else {
            format!("_{}", codec.name())
        };
        is_current(
            "../../examples/compiled/rfc5280.asni",
            &format!("tests/generated/rfc5280{suffix}.rs"),
            Options {
                codec,
                integer: Integer::Big,
                ..Options::default()
            },
        );
        is_current(
            "../asnkit-gen-rust/tests/generated/shapes.asni",
            &format!("tests/generated/shapes{suffix}.rs"),
            Options {
                codec,
                skip_unsupported: true,
                ..Options::default()
            },
        );
    }
    is_current(
        "../asnkit-gen-rust/tests/generated/shapes.asni",
        "tests/generated/shapes_no_std.rs",
        Options {
            codec: Encoding::Uper,
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
fn the_certificates_are_what_the_runtime_makes_of_them() {
    let asni = fs::read(root().join("../../examples/compiled/rfc5280.asni")).unwrap();
    let der =
        Specification::from_asni(&asni, &Checks::default(), runtime::Options::new(Codec::Der))
            .unwrap();
    for codec in [Codec::Ber, Codec::Der, Codec::Uper] {
        let specification =
            Specification::from_asni(&asni, &Checks::default(), runtime::Options::new(codec))
                .unwrap();
        for (name, data) in certificates() {
            let value = der.decode("Certificate", &data).unwrap();
            let encoded = specification.encode("Certificate", &value).unwrap();
            let mut out = vec![0; encoded.len()];
            let count = allocations(|| {
                let length = match codec {
                    Codec::Ber => {
                        let v = rfc5280_ber::Certificate::decode_ber(&encoded).unwrap();
                        assert_eq!(v.ber_len().unwrap(), encoded.len());
                        v.encode_ber(&mut out).unwrap()
                    }
                    Codec::Der => {
                        let v = rfc5280::Certificate::decode_der(&encoded).unwrap();
                        assert_eq!(v.der_len().unwrap(), encoded.len());
                        v.encode_der(&mut out).unwrap()
                    }
                    _ => {
                        let v = rfc5280_uper::Certificate::decode_uper(&encoded).unwrap();
                        assert_eq!(v.uper_len().unwrap(), encoded.len());
                        v.encode_uper(&mut out).unwrap()
                    }
                };
                assert_eq!(length, encoded.len());
            });
            assert_eq!(count, 0, "{name} {codec:?}");
            assert_eq!(out, encoded, "{name} {codec:?}");
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

trait RoundTrip<'a>: Sized {
    const CODEC: Codec;
    fn encoded(&self) -> wire::Result<Vec<u8>>;
    fn decoded(data: &'a [u8]) -> wire::Result<Self>;
}

macro_rules! round_trip {
    ($codec:ident, $encode:ident, $decode:ident, $($ty:ty),+ $(,)?) => {
        $(impl<'a> RoundTrip<'a> for $ty {
            const CODEC: Codec = Codec::$codec;
            fn encoded(&self) -> wire::Result<Vec<u8>> { self.$encode() }
            fn decoded(data: &'a [u8]) -> wire::Result<Self> { Self::$decode(data) }
        })+
    };
}

round_trip!(
    Ber,
    to_ber,
    decode_ber,
    shapes_ber::Keywords<'a>,
    shapes_ber::Sizes,
    shapes_ber::Shape<'a>,
    shapes_ber::Tree,
    shapes_ber::Defaults<'a>,
    shapes_ber::Extensible<'a>,
    shapes_ber::Outer<'a>
);
round_trip!(
    Der,
    to_der,
    decode_der,
    shapes::Keywords<'a>,
    shapes::Sizes,
    shapes::Shape<'a>,
    shapes::Tree,
    shapes::Defaults<'a>,
    shapes::Extensible<'a>,
    shapes::Outer<'a>
);
round_trip!(
    Uper,
    to_uper,
    decode_uper,
    shapes_uper::Keywords<'a>,
    shapes_uper::Sizes,
    shapes_uper::Shape<'a>,
    shapes_uper::Tree,
    shapes_uper::Defaults<'a>,
    shapes_uper::Extensible<'a>,
    shapes_uper::Outer<'a>
);

fn comes_back<'a, T: RoundTrip<'a> + PartialEq + std::fmt::Debug>(name: &str, value: &T) {
    let encoded = Box::leak(value.encoded().unwrap().into_boxed_slice());
    assert_eq!(&T::decoded(encoded).unwrap(), value, "{name}");
    let specification = runtime_shapes(T::CODEC);
    let decoded = specification.decode(name, encoded).unwrap();
    assert_eq!(
        specification.encode(name, &decoded).unwrap(),
        encoded,
        "{name}"
    );
}

macro_rules! shape_tests {
    ($name:ident, $module:ident) => {
        #[test]
        fn $name() {
            use $module::*;

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
    };
}
shape_tests!(der_shapes_come_back, shapes);
shape_tests!(ber_shapes_come_back, shapes_ber);
shape_tests!(uper_shapes_come_back, shapes_uper);

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
    let ber_pairs = pairs.map(|p| shapes_ber::PairsItem { a: p.a, b: p.b });
    let ber_value = wire::SequenceOf::new(&ber_pairs);
    let ber = encode(|out| shapes_ber::pairs::encode_ber(&ber_value, out));
    let decoded = pairs::decode_der(&der).unwrap();
    let order: Vec<(u8, bool)> = decoded.iter().map(|pair| (pair.a, pair.b)).collect();

    assert_eq!(order, [(1, false), (1, true), (9, true)]);
    // BER keeps the order, which DER does not decode.
    assert_ne!(ber, der);
    assert!(pairs::decode_der(&ber).is_err());
    assert_eq!(shapes_ber::pairs::decode_ber(&ber).unwrap(), ber_value);

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
        shapes_ber::Defaults::decode_ber(&[0x30, 0x03, 0x80, 0x01, 0x03])
            .unwrap()
            .count,
        value.count
    );
}

#[test]
fn deep_data_is_an_error_and_not_a_crash() {
    // Each byte is a tree on the left of another, as deep as there are.
    let deep = vec![0xff; 100_000];

    assert!(shapes_uper::Tree::decode_uper(&deep).is_err());

    // In BER, a [1] of a [1] of ...
    let mut ber = Vec::new();

    for _ in 0..10_000 {
        ber.extend_from_slice(&[0x30, 0x80, 0xa1, 0x80]);
    }

    assert!(shapes_ber::Tree::decode_ber(&ber).is_err());
}

#[test]
fn a_value_the_enumeration_does_not_know_is_an_error() {
    // A square of the color of the first addition, and of the second, which
    // it does not have: the index of the alternative of Shape (001), if
    // there is a color (1), the side less 1 (0000000), an addition (1),
    // which is small (0) and its index.
    assert_eq!(
        shapes_uper::Shape::decode_uper(&[0x30, 0x10, 0x00]).unwrap(),
        shapes_uper::Shape::Square(shapes_uper::ShapeSquare {
            side: 1,
            color: Some(shapes_uper::Color::Violet),
        })
    );
    assert!(shapes_uper::Shape::decode_uper(&[0x30, 0x10, 0x20]).is_err());
    assert_eq!(shapes_uper::Color::from_number(4), None);
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

#[test]
fn only_the_selected_codec_is_generated() {
    let data = fs::read(root().join("../../examples/compiled/rfc5280.asni")).unwrap();
    for codec in [Encoding::Ber, Encoding::Der, Encoding::Uper] {
        let (text, _) = generate(
            &data,
            Options {
                codec,
                ..Options::default()
            },
        )
        .unwrap();
        assert!(text.contains(&format!("pub fn encode_{}", codec.name())));
        for other in [Encoding::Ber, Encoding::Der, Encoding::Uper] {
            if other != codec {
                assert!(!text.contains(&format!("pub fn encode_{}", other.name())));
                assert!(!text.contains(&format!("pub fn decode_{}", other.name())));
                assert!(!text.contains(&format!("pub fn to_{}", other.name())));
            }
        }
        let absent = if codec == Encoding::Uper {
            "ber"
        } else {
            "uper"
        };
        assert!(!text.contains(&format!("fn {absent}_")));
        assert!(!text.contains("Asn1"));
        if codec != Encoding::Uper {
            assert!(!text.contains(" per::"));
            assert!(!text.contains("wire::per"));
            assert!(!text.contains("uper_index"));
        }
    }
}

#[test]
fn the_cli_selects_one_codec_and_rejects_unsupported_codecs() {
    let folder =
        std::env::temp_dir().join(format!("asnkit-codec-selection-{}", std::process::id()));
    let source = root().join("../../examples/compiled/foo.asni");
    for codec in [
        None,
        Some("ber"),
        Some("der"),
        Some("uper"),
        Some("oer"),
        Some("all"),
    ] {
        let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_asnkit-gen-rust-codec"));
        command.arg("--asni").arg(&source).arg("--out").arg(&folder);
        if let Some(codec) = codec {
            command.arg("--opt").arg(format!("codec={codec}"));
        }
        let output = command.output().unwrap();
        if matches!(codec, Some("oer" | "all")) {
            assert!(!output.status.success());
            assert!(String::from_utf8_lossy(&output.stderr).contains("codec=ber|der|uper"));
        } else {
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let text = fs::read_to_string(folder.join("foo.rs")).unwrap();
            let selected = codec.unwrap_or("ber");
            assert!(text.contains(&format!("pub fn encode_{selected}")));
            for other in ["ber", "der", "uper"] {
                if other != selected {
                    assert!(!text.contains(&format!("pub fn encode_{other}")));
                }
            }
        }
    }
    fs::remove_dir_all(folder).unwrap();
}
