// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What is decoded may be anything: cut short, changed, or made up.
//! Whatever it is, decoding ends in a value or in an error, and not in
//! a panic, an endless loop or a stack that is used up.

use std::collections::HashMap;
use std::fs;
use std::io::Read;
use std::path::PathBuf;
use std::time::Instant;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use serde_json::Value as Json;

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/vectors")
}

fn hex(text: &str) -> Vec<u8> {
    (0..text.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&text[i..i + 2], 16).unwrap())
        .collect()
}

fn codec_of(name: &str) -> Codec {
    match name {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        _ => Codec::Uper,
    }
}

/// A small generator of numbers that does not need a crate.
struct Random(u64);

impl Random {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

#[test]
fn changed_data_is_never_a_panic() {
    let mut text = String::new();

    flate2::read::GzDecoder::new(fs::File::open(folder().join("vectors.jsonl.gz")).unwrap())
        .read_to_string(&mut text)
        .unwrap();

    let mut specs: HashMap<(String, String, bool), Option<Specification>> = HashMap::new();
    let mut random = Random(0x2545_f491_4f6c_dd1d);
    let started = Instant::now();
    let mut tried = 0usize;

    for line in text.lines() {
        let vector: Json = serde_json::from_str(line).unwrap();

        if vector["operation"] != "decode" || vector["error"] != Json::Null {
            continue;
        }

        let codec = vector["codec"].as_str().unwrap().to_string();
        let spec_name = vector["spec"].as_str().unwrap().to_string();
        let numeric = vector["numeric_enums"].as_bool().unwrap();
        let spec = specs
            .entry((spec_name.clone(), codec.clone(), numeric))
            .or_insert_with(|| {
                let data =
                    fs::read(folder().join("specs").join(format!("{spec_name}.asnc"))).unwrap();
                let mut options = Options::new(codec_of(&codec));

                options.numeric_enums = numeric;

                Specification::from_asnc(&data, &Checks::default(), options).ok()
            });
        let Some(spec) = spec else { continue };
        let name = vector["type"].as_str().unwrap();
        let encoded = hex(vector["encoded"].as_str().unwrap());

        // Every prefix of the short ones, a few of the long ones.
        let step = (encoded.len() / 48).max(1);

        for size in (0..encoded.len()).step_by(step) {
            let _ = spec.decode(name, &encoded[..size]);
            let _ = spec.decode_with_length(name, &encoded[..size]);
            tried += 2;
        }

        // Changed bytes: one, and a few.
        for _ in 0..24 {
            let mut changed = encoded.clone();

            if changed.is_empty() {
                break;
            }

            for _ in 0..1 + random.next() % 3 {
                let index = (random.next() % changed.len() as u64) as usize;

                changed[index] = match random.next() % 3 {
                    0 => random.next() as u8,
                    1 => changed[index] ^ (1 << (random.next() % 8)),
                    _ => 0xff,
                };
            }

            let _ = spec.decode(name, &changed);
            tried += 1;
        }

        // Made up, as long.
        let mut made: Vec<u8> = (0..encoded.len().min(512))
            .map(|_| random.next() as u8)
            .collect();

        let _ = spec.decode(name, &made);
        made.reverse();
        let _ = spec.decode(name, &made);
        tried += 2;
    }

    assert!(tried > 20_000, "{tried}");
    eprintln!("{tried} tries in {:?}", started.elapsed());
}

/// A type that holds itself, with data nested as deep as can be written.
#[test]
fn nesting_is_limited() {
    use asnkitrt::cbor::Cbor;

    let int = |value: i128| Cbor::Int(value);
    let text = |value: &str| Cbor::Text(value.into());
    // A ::= SEQUENCE OF A
    let nodes = Cbor::Array(vec![
        int(1),
        Cbor::Array(vec![
            int(0),
            Cbor::Array(vec![
                int(2),
                text("M"),
                Cbor::Array(vec![
                    int(0),
                    Cbor::Array(vec![
                        int(3),
                        text("A"),
                        Cbor::Array(vec![int(18), Cbor::Array(vec![int(11), text("A")])]),
                    ]),
                ]),
            ]),
        ]),
    ]);
    let started = Instant::now();

    for codec in all_codecs() {
        let spec = Specification::from_nodes(&nodes, Options::new(codec)).unwrap();
        let data: Vec<u8> = match codec {
            Codec::Jer => "[".repeat(200_000).into_bytes(),
            Codec::Xer => "<A>".repeat(200_000).into_bytes(),
            Codec::Ber => std::iter::repeat_n([0x30, 0x80], 100_000)
                .flatten()
                .collect(),
            Codec::Der => std::iter::repeat_n([0x30, 0x80], 100_000)
                .flatten()
                .collect(),
            _ => std::iter::repeat_n(0x01, 200_000).collect(),
        };

        assert!(spec.decode("A", &data).is_err(), "{codec:?}");

        // And what is made of values.
        let mut value = asnkitrt::value::Value::List(Vec::new());

        for _ in 0..2_000 {
            value = asnkitrt::value::Value::List(vec![value]);
        }

        assert!(spec.encode("A", &value).is_err(), "{codec:?}");

        // A value that is deep and not too deep goes both ways.
        let mut value = asnkitrt::value::Value::List(Vec::new());

        for _ in 0..40 {
            value = asnkitrt::value::Value::List(vec![value]);
        }

        let encoded = spec
            .encode("A", &value)
            .unwrap_or_else(|error| panic!("{codec:?}: {error}"));

        assert_eq!(spec.decode("A", &encoded).unwrap(), value, "{codec:?}");

        // Dropping a value this deep must not use up the stack either.
        std::mem::forget(std::hint::black_box(spec));
    }

    eprintln!("{:?}", started.elapsed());
}

// A specification is as much outside data as what is decoded with it.

use asnkitrt::cbor::Cbor;

fn int(value: i128) -> Cbor {
    Cbor::Int(value)
}

fn text(value: &str) -> Cbor {
    Cbor::Text(value.into())
}

fn list(items: Vec<Cbor>) -> Cbor {
    let mut all = vec![int(0)];

    all.extend(items);

    Cbor::Array(all)
}

fn assignment(name: &str, ty: Cbor) -> Cbor {
    Cbor::Array(vec![int(3), text(name), ty])
}

fn reference(name: &str) -> Cbor {
    Cbor::Array(vec![int(11), text(name)])
}

fn member(name: &str, ty: Cbor) -> Cbor {
    Cbor::Array(vec![int(4), text(name), ty])
}

fn module(assignments: Vec<Cbor>) -> Cbor {
    Cbor::Array(vec![
        int(1),
        list(vec![Cbor::Array(vec![
            int(2),
            text("M"),
            list(assignments),
        ])]),
    ])
}

const BOOLEAN: fn() -> Cbor = || Cbor::Array(vec![int(10), text("BOOLEAN")]);

/// The codecs this is built with.
fn all_codecs() -> Vec<Codec> {
    let mut codecs = vec![Codec::Ber, Codec::Der, Codec::Oer, Codec::Per, Codec::Uper];

    if cfg!(feature = "jer") {
        codecs.push(Codec::Jer);
    }

    if cfg!(feature = "xer") {
        codecs.push(Codec::Xer);
    }

    codecs
}

#[test]
fn a_long_chain_of_references() {
    // A0 ::= A1, A1 ::= A2, and so on to a BOOLEAN.
    let count = 100_000;
    let mut assignments: Vec<Cbor> = (0..count)
        .map(|i| assignment(&format!("A{i}"), reference(&format!("A{}", i + 1))))
        .collect();

    assignments.push(assignment(&format!("A{count}"), BOOLEAN()));

    let nodes = module(assignments);
    let started = Instant::now();

    for codec in all_codecs() {
        let spec = Specification::from_nodes(&nodes, Options::new(codec)).unwrap();
        let encoded = spec
            .encode("A0", &asnkitrt::value::Value::Boolean(true))
            .unwrap();

        assert_eq!(
            spec.decode("A0", &encoded).unwrap(),
            asnkitrt::value::Value::Boolean(true),
            "{codec:?}"
        );
    }

    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
}

#[test]
fn a_cycle_of_references() {
    // A ::= B, B ::= A, and a lot of other types.
    let mut assignments = vec![
        assignment("A", reference("B")),
        assignment("B", reference("A")),
    ];

    assignments.extend((0..20_000).map(|i| assignment(&format!("F{i}"), BOOLEAN())));

    let nodes = module(assignments);

    for codec in all_codecs() {
        assert!(
            Specification::from_nodes(&nodes, Options::new(codec)).is_err(),
            "{codec:?}"
        );
    }

    // A type that is a reference to itself.
    let nodes = module(vec![assignment("A", reference("A"))]);

    assert!(Specification::from_nodes(&nodes, Options::new(Codec::Ber)).is_err());
}

#[test]
fn choices_that_hold_themselves() {
    // A ::= CHOICE { x A, y A }, and a SET that has an A. Untagged, so
    // what begins an A is what begins the ones in it, forever.
    let choice = Cbor::Array(vec![
        int(17),
        list(vec![
            member("x", reference("A")),
            member("y", reference("A")),
        ]),
    ]);
    let set = Cbor::Array(vec![
        int(16),
        list(vec![member("a", reference("A")), member("b", BOOLEAN())]),
    ]);
    let mut assignments = vec![assignment("A", choice), assignment("S", set)];

    // C0 ::= CHOICE { x C1, y C1 }, C1 ::= CHOICE { x C2, y C2 }, and so
    // on: there are 2**40 ways down.
    for i in 0..40 {
        assignments.push(assignment(
            &format!("C{i}"),
            Cbor::Array(vec![
                int(17),
                list(vec![
                    member("x", reference(&format!("C{}", i + 1))),
                    member("y", reference(&format!("C{}", i + 1))),
                ]),
            ]),
        ));
    }

    assignments.push(assignment("C40", BOOLEAN()));

    let nodes = module(assignments);
    let started = Instant::now();

    for codec in all_codecs() {
        let spec = Specification::from_nodes(&nodes, Options::new(codec)).unwrap();

        for data in [
            &[0x01u8, 0x01, 0xff][..],
            &[0x30, 0x03, 0x01, 0x01, 0xff],
            &[0xa0, 0x00],
            &[],
        ] {
            let _ = spec.decode("S", data);
            let _ = spec.decode("A", data);
            let _ = spec.decode("C0", data);
        }
    }

    assert!(started.elapsed().as_secs() < 30, "{:?}", started.elapsed());
}
