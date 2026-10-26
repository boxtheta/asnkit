// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

#![no_std]

extern crate alloc;

// Made by the plugin, see tests/generated.rs: RFC 5280, which needs no
// allocation, and shapes with `--opt no_std=true`, of which the types that
// hold themselves are on the heap.
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../generated/otps.rs"]
pub mod otps;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../generated/rfc5280.rs"]
pub mod rfc5280;
#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../generated/shapes_no_std.rs"]
pub mod shapes;

#[rustfmt::skip]
#[allow(dead_code)]
#[path = "../../generated/rfc5280_ber.rs"]
pub mod rfc5280_ber;
