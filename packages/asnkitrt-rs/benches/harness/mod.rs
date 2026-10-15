// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! A small harness for the benchmarks, with nothing but std, so that the
//! crates need no more dependencies for them. The benchmarks of the
//! compiler use it too.
//!
//! Each benchmark is run for a while to know how long it takes, then in
//! samples of about `SAMPLE` each, and the median of the time an
//! iteration took in the samples is what it took. What is printed is
//! that, and the median deviation from it.
//!
//! The arguments that are not flags select the benchmarks whose names
//! have one of them, as `cargo bench -- der`. If `ASNKIT_BENCH_OUT` is
//! a directory, the results are also written there as JSON, as
//! `benchmark-action/github-action-benchmark` reads them
//! (`customSmallerIsBetter`), to the file named after the benchmarks.

use std::fmt::Write as _;
use std::time::{Duration, Instant};

const WARM_UP: Duration = Duration::from_millis(300);
const SAMPLE: Duration = Duration::from_millis(10);
const SAMPLES: usize = 50;

pub struct Benches {
    group: &'static str,
    filters: Vec<String>,
    results: Vec<(String, f64, f64)>,
}

impl Benches {
    pub fn new(group: &'static str) -> Benches {
        Benches {
            group,
            filters: std::env::args()
                .skip(1)
                .filter(|argument| !argument.starts_with('-'))
                .collect(),
            results: Vec::new(),
        }
    }

    pub fn run<R>(&mut self, name: &str, mut f: impl FnMut() -> R) {
        let name = format!("{}/{name}", self.group);

        if !self.filters.is_empty() && !self.filters.iter().any(|filter| name.contains(filter)) {
            return;
        }

        let mut iterations = 0u64;
        let start = Instant::now();

        while start.elapsed() < WARM_UP {
            std::hint::black_box(f());
            iterations += 1;
        }

        let per_iteration = start.elapsed().as_secs_f64() / iterations as f64;
        let batch = ((SAMPLE.as_secs_f64() / per_iteration) as u64).max(1);
        let mut samples: Vec<f64> = (0..SAMPLES)
            .map(|_| {
                let start = Instant::now();

                for _ in 0..batch {
                    std::hint::black_box(f());
                }

                start.elapsed().as_secs_f64() * 1e9 / batch as f64
            })
            .collect();
        let time = median(&mut samples);
        let mut deviations: Vec<f64> = samples.iter().map(|s| (s - time).abs()).collect();
        let deviation = median(&mut deviations);

        eprintln!("{name:<40} {time:>14.1} ns  ± {deviation:.1}");
        self.results.push((name, time, deviation));
    }
}

impl Drop for Benches {
    fn drop(&mut self) {
        let Some(directory) = std::env::var_os("ASNKIT_BENCH_OUT") else {
            return;
        };
        let mut json = String::from("[");

        for (i, (name, median, deviation)) in self.results.iter().enumerate() {
            if i > 0 {
                json.push(',');
            }

            write!(
                json,
                "\n  {{\"name\": \"{name}\", \"unit\": \"ns\", \"value\": {median:.1}, \
                 \"range\": \"± {deviation:.1}\"}}"
            )
            .unwrap();
        }

        json.push_str("\n]\n");
        std::fs::write(
            std::path::Path::new(&directory).join(format!("{}.json", self.group)),
            json,
        )
        .unwrap();
    }
}

fn median(values: &mut [f64]) -> f64 {
    values.sort_by(f64::total_cmp);

    let middle = values.len() / 2;

    if values.len().is_multiple_of(2) {
        (values[middle - 1] + values[middle]) / 2.0
    } else {
        values[middle]
    }
}
