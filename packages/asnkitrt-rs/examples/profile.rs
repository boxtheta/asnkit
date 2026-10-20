// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Encodes or decodes the certificate of `benches/runtime.rs` over and
//! over with one codec, for a profiler to sample, and prints how long
//! a call takes. `examples/profile.py` builds it, runs it under a
//! profiler and tells where the time goes; see there.
//!
//!   cargo run --profile profiling -p asnkitrt --example profile -- \
//!       CODEC encode|decode [ITERATIONS] [--allocs]
//!
//! With `--allocs` it also counts the allocations of a call, and those
//! that the decoded value holds, which a decoder can not do with fewer.

use std::alloc::{GlobalAlloc, Layout, System};
use std::borrow::Cow;
use std::hint::black_box;
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Instant;

use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

const RFC5280: &[u8] = include_bytes!("../../../examples/compiled/rfc5280.asni");
const CERTIFICATE: &[u8] = include_bytes!("../tests/fixtures/x509/extensions_all.der");

/// The allocator, counting allocations once `COUNTING` is set.
struct Counting;

static COUNTING: AtomicBool = AtomicBool::new(false);
static ALLOCATIONS: AtomicUsize = AtomicUsize::new(0);

fn count() {
    if COUNTING.load(Ordering::Relaxed) {
        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        count();

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        unsafe { System.dealloc(pointer, layout) }
    }

    // A reallocation is an allocation as far as the cost goes.
    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        count();

        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

/// What a value holds on the heap: names it does not borrow, text,
/// bytes and containers.
fn held(value: &Value<'_>, counts: &mut [usize; 4]) {
    match value {
        Value::Text(_) => counts[1] += 1,
        Value::Bytes(_) | Value::Bits(..) | Value::BigInteger(_) => counts[2] += 1,
        Value::List(items) => {
            counts[3] += 1;

            for item in items {
                held(item, counts);
            }
        }
        Value::Map(members) => {
            counts[3] += 1;

            for (name, member) in members {
                counts[0] += usize::from(matches!(name, Cow::Owned(_)));
                held(member, counts);
            }
        }
        Value::Choice(name, inner) => {
            counts[0] += usize::from(matches!(name, Cow::Owned(_)));
            counts[3] += 1;
            held(inner, counts);
        }
        _ => {}
    }
}

fn usage() -> ExitCode {
    eprintln!("usage: profile ber|der|oer|per|uper encode|decode [ITERATIONS] [--allocs]");

    ExitCode::FAILURE
}

fn main() -> ExitCode {
    let allocs = std::env::args().any(|argument| argument == "--allocs");
    let arguments: Vec<String> = std::env::args()
        .skip(1)
        .filter(|argument| argument != "--allocs")
        .collect();
    let codec = match arguments.first().map(String::as_str) {
        Some("ber") => Codec::Ber,
        Some("der") => Codec::Der,
        Some("oer") => Codec::Oer,
        Some("per") => Codec::Per,
        Some("uper") => Codec::Uper,
        _ => return usage(),
    };
    let encode = match arguments.get(1).map(String::as_str) {
        Some("encode") => true,
        Some("decode") => false,
        _ => return usage(),
    };
    let Ok(iterations) = arguments
        .get(2)
        .map_or(Ok(1_000_000), |text| text.parse::<usize>())
    else {
        return usage();
    };

    let load =
        |codec| Specification::from_asni(RFC5280, &Checks::default(), Options::new(codec)).unwrap();
    let der = load(Codec::Der);
    let certificate = der.decode("Certificate", CERTIFICATE).unwrap();
    let spec = load(codec);
    let encoded = spec.encode("Certificate", &certificate).unwrap();

    COUNTING.store(allocs, Ordering::Relaxed);

    let started = Instant::now();

    for _ in 0..iterations {
        if encode {
            black_box(spec.encode("Certificate", black_box(&certificate)).unwrap());
        } else {
            black_box(spec.decode("Certificate", black_box(&encoded)).unwrap());
        }
    }

    let elapsed = started.elapsed();

    COUNTING.store(false, Ordering::Relaxed);

    println!(
        "{:.1} ns per call, {} bytes encoded, {iterations} calls",
        elapsed.as_nanos() as f64 / iterations as f64,
        encoded.len(),
    );

    if allocs {
        let mut counts = [0; 4];

        held(&certificate, &mut counts);
        println!(
            "{:.1} allocations per call; a decoded value holds {}: {} names, {} texts, {} byte strings, {} containers",
            ALLOCATIONS.load(Ordering::Relaxed) as f64 / iterations as f64,
            counts.iter().sum::<usize>(),
            counts[0],
            counts[1],
            counts[2],
            counts[3],
        );
    }

    ExitCode::SUCCESS
}
