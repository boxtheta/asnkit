// Copyright (c) 2026 BoxTheta
// SPDX-License-Identifier: Apache-2.0

//! Run with `cargo bench -p asnkit-gen-rust-codec --bench codecs`.
//! Setup and correctness checks are outside timing/allocation measurements.
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::hint::black_box;
use std::time::{Duration, Instant};

use asnkitrt::container::Checks;
use asnkitrt::runtime::{self, Specification};
use asnkitrt::types::Codec;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../tests/generated/rfc5280_ber.rs"]
mod ber;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../tests/generated/rfc5280.rs"]
mod der;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../tests/generated/rfc5280_uper.rs"]
mod uper;

#[derive(Clone, Copy, Default, Debug)]
struct Memory {
    allocations: usize,
    requested: usize,
    live: usize,
    peak: usize,
}
thread_local! {
    static MEMORY: Cell<Option<Memory>> = const { Cell::new(None) };
}
struct Counting;
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = unsafe { System.alloc(layout) };
        if !ptr.is_null() {
            let _ = MEMORY.try_with(|cell| {
                if let Some(mut m) = cell.get() {
                    m.allocations += 1;
                    m.requested += layout.size();
                    m.live += layout.size();
                    m.peak = m.peak.max(m.live);
                    cell.set(Some(m));
                }
            });
        }
        ptr
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        let _ = MEMORY.try_with(|cell| {
            if let Some(mut m) = cell.get() {
                m.live -= layout.size();
                cell.set(Some(m));
            }
        });
        unsafe { System.dealloc(ptr, layout) }
    }
}
#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn memory(f: impl FnOnce()) -> Memory {
    MEMORY.with(|cell| cell.set(Some(Memory::default())));
    f();
    let result = MEMORY.with(|cell| cell.replace(None).unwrap());
    assert_eq!(result.live, 0);
    result
}

// Each sample processes every fixture equally often. Report ns/certificate,
// with seven samples after a calibration/warm-up of at least 30 ms.
fn timing(mut batch: impl FnMut(), count: usize) -> (f64, f64, f64) {
    let mut rounds = 1;
    loop {
        let start = Instant::now();
        for _ in 0..rounds {
            batch();
        }
        if start.elapsed() >= Duration::from_millis(30) {
            break;
        }
        rounds *= 2;
    }
    let mut samples = [0.0_f64; 7];
    for sample in &mut samples {
        let start = Instant::now();
        for _ in 0..rounds {
            batch();
        }
        *sample = start.elapsed().as_nanos() as f64 / (rounds * count) as f64;
    }
    samples.sort_by(f64::total_cmp);
    (samples[3], samples[0], samples[6])
}

macro_rules! measure {
    ($module:ident, $codec:expr, $decode:ident, $encode:ident, $len:ident, $to:ident, $sources:expr, $der_spec:expr, $asni:expr) => {{
        let spec =
            Specification::from_asni(&$asni, &Checks::default(), runtime::Options::new($codec))
                .unwrap();
        let inputs: Vec<_> = $sources
            .iter()
            .map(|data| {
                let value = $der_spec.decode("Certificate", data).unwrap();
                spec.encode("Certificate", &value).unwrap()
            })
            .collect();
        let values: Vec<_> = inputs
            .iter()
            .map(|data| $module::Certificate::$decode(data).unwrap())
            .collect();
        let mut outputs: Vec<_> = inputs.iter().map(|data| vec![0; data.len()]).collect();
        let mut zero_heap = Memory::default();
        let mut convenience = Vec::new();
        for (input, out) in inputs.iter().zip(&mut outputs) {
            let m = memory(|| {
                let value = $module::Certificate::$decode(black_box(input)).unwrap();
                assert_eq!(value.$len().unwrap(), input.len());
                let n = value.$encode(black_box(out)).unwrap();
                assert_eq!(&out[..n], input);
                black_box(value);
            });
            zero_heap.allocations += m.allocations;
            zero_heap.requested += m.requested;
            zero_heap.peak = zero_heap.peak.max(m.peak);
            let value = $module::Certificate::$decode(input).unwrap();
            convenience.push(memory(|| {
                black_box(value.$to().unwrap());
            }));
        }
        println!(
            "\n{}: {} certificates, encoded bytes min/max={}/{}, inline value={} bytes",
            stringify!($module),
            inputs.len(),
            inputs.iter().map(Vec::len).min().unwrap(),
            inputs.iter().map(Vec::len).max().unwrap(),
            size_of::<$module::Certificate<'_>>()
        );
        println!("decode + length + buffer encode heap (whole corpus): {zero_heap:?}");
        println!(
            "to_* heap per certificate: allocations {}..{}, requested/peak bytes {}..{} / {}..{}",
            convenience.iter().map(|m| m.allocations).min().unwrap(),
            convenience.iter().map(|m| m.allocations).max().unwrap(),
            convenience.iter().map(|m| m.requested).min().unwrap(),
            convenience.iter().map(|m| m.requested).max().unwrap(),
            convenience.iter().map(|m| m.peak).min().unwrap(),
            convenience.iter().map(|m| m.peak).max().unwrap()
        );
        for (name, stats) in [
            (
                "decode",
                timing(
                    || {
                        for data in &inputs {
                            black_box($module::Certificate::$decode(black_box(data)).unwrap());
                        }
                    },
                    inputs.len(),
                ),
            ),
            (
                "length",
                timing(
                    || {
                        for value in &values {
                            black_box(black_box(value).$len().unwrap());
                        }
                    },
                    inputs.len(),
                ),
            ),
            (
                "encode",
                timing(
                    || {
                        for (value, out) in values.iter().zip(&mut outputs) {
                            black_box(black_box(value).$encode(black_box(out)).unwrap());
                        }
                    },
                    inputs.len(),
                ),
            ),
            (
                "roundtrip",
                timing(
                    || {
                        for (data, out) in inputs.iter().zip(&mut outputs) {
                            let value = $module::Certificate::$decode(black_box(data)).unwrap();
                            black_box(value.$encode(black_box(out)).unwrap());
                        }
                    },
                    inputs.len(),
                ),
            ),
        ] {
            println!(
                "{name}: median {:.1} ns/cert (min {:.1}, max {:.1})",
                stats.0, stats.1, stats.2
            );
        }
    }};
}

fn main() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut paths: Vec<_> = std::fs::read_dir(root.join("../asnkitrt-rs/tests/fixtures/x509"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "der"))
        .collect();
    paths.sort();
    let sources: Vec<_> = paths.iter().map(|p| std::fs::read(p).unwrap()).collect();
    let asni = std::fs::read(root.join("../../examples/compiled/rfc5280.asni")).unwrap();
    let der_spec =
        Specification::from_asni(&asni, &Checks::default(), runtime::Options::new(Codec::Der))
            .unwrap();
    measure!(
        ber,
        Codec::Ber,
        decode_ber,
        encode_ber,
        ber_len,
        to_ber,
        sources,
        der_spec,
        asni
    );
    measure!(
        der,
        Codec::Der,
        decode_der,
        encode_der,
        der_len,
        to_der,
        sources,
        der_spec,
        asni
    );
    measure!(
        uper,
        Codec::Uper,
        decode_uper,
        encode_uper,
        uper_len,
        to_uper,
        sources,
        der_spec,
        asni
    );
}
