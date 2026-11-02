// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Loads compiled specifications and tells how much of the heap the
//! loaded specification keeps, how much loading takes at its peak, and
//! how much each step holds: the CBOR of the file, its nodes and the
//! lowered types. The figures of the README are made with it.
//!
//!   cargo run --release -p asnkitrt --example load_memory -- \
//!       [--codec CODEC] FILE.asni...
//!
//! The heap is counted in the sizes asked of the allocator, without
//! what the allocator adds to them.

use std::alloc::{GlobalAlloc, Layout, System};
use std::process::ExitCode;
use std::sync::atomic::{AtomicUsize, Ordering};

use asnkitrt::container::{self, Checks};
use asnkitrt::node::Spec;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::{self, Codec};

/// The allocator, counting what is allocated now and at the most.
struct Counting;

static CURRENT: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

fn grow(size: usize) {
    let now = CURRENT.fetch_add(size, Ordering::Relaxed) + size;

    PEAK.fetch_max(now, Ordering::Relaxed);
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        grow(layout.size());

        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, pointer: *mut u8, layout: Layout) {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);

        unsafe { System.dealloc(pointer, layout) }
    }

    unsafe fn realloc(&self, pointer: *mut u8, layout: Layout, size: usize) -> *mut u8 {
        CURRENT.fetch_sub(layout.size(), Ordering::Relaxed);
        grow(size);

        unsafe { System.realloc(pointer, layout, size) }
    }
}

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn now() -> usize {
    CURRENT.load(Ordering::Relaxed)
}

/// What `step` keeps on the heap, and what it takes at its peak over
/// what there was before it.
fn measure<T>(step: impl FnOnce() -> T) -> (T, usize, usize) {
    let before = now();

    PEAK.store(before, Ordering::Relaxed);

    let made = step();

    (made, now() - before, PEAK.load(Ordering::Relaxed) - before)
}

fn kib(bytes: usize) -> String {
    format!("{:.1} KiB", bytes as f64 / 1024.0)
}

fn codec_named(name: &str) -> Option<Codec> {
    Some(match name {
        "ber" => Codec::Ber,
        "der" => Codec::Der,
        "oer" => Codec::Oer,
        "per" => Codec::Per,
        "uper" => Codec::Uper,
        "jer" => Codec::Jer,
        "xer" => Codec::Xer,
        "gser" => Codec::Gser,
        _ => return None,
    })
}

fn main() -> ExitCode {
    let mut arguments = std::env::args().skip(1).peekable();
    let mut codec = Codec::Der;

    if arguments.peek().map(String::as_str) == Some("--codec") {
        arguments.next();

        match arguments.next().as_deref().and_then(codec_named) {
            Some(named) => codec = named,
            None => {
                eprintln!("The codec is one of ber, der, oer, per, uper, jer, xer and gser.");

                return ExitCode::FAILURE;
            }
        }
    }

    let checks = Checks::default();

    for path in arguments {
        let data = match std::fs::read(&path) {
            Ok(data) => data,
            Err(error) => {
                eprintln!("{path}: {error}");

                return ExitCode::FAILURE;
            }
        };

        // The whole of it, as an application loads a specification.
        let (loaded, kept, peak) =
            measure(|| Specification::from_asni(&data, &checks, Options::new(codec)));

        if let Err(error) = loaded {
            eprintln!("{path}: {error}");

            return ExitCode::FAILURE;
        }

        drop(loaded);

        // Each step, with what the steps before it hold.
        let (asni, cbor, _) = measure(|| container::read(&data, &checks).unwrap());
        let (spec, nodes, _) = measure(|| Spec::from_nodes(&asni.nodes).unwrap());
        let (lowered, lowered_kept, lowered_peak) =
            measure(|| types::lower(&spec, codec, false, false).unwrap());

        drop((asni, spec, lowered));

        println!("{path}");
        println!("  .asni             {:>10}", kib(data.len()));
        println!("  kept              {:>10}", kib(kept));
        println!(
            "  peak              {:>10}  ({:.2} times what is kept)",
            kib(peak),
            peak as f64 / kept as f64
        );
        println!("  CBOR              {:>10}", kib(cbor));
        println!("  nodes             {:>10}", kib(nodes));
        println!(
            "  types             {:>10}  ({} at the peak of lowering)",
            kib(lowered_kept),
            kib(lowered_peak)
        );
    }

    ExitCode::SUCCESS
}
