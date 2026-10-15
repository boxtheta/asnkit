// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Encode and decode ASN.1 with compiled specifications.
//!
//! This is the runtime for Rust of the formats in
//! `docs/file_formats.md`. It needs the `alloc` crate but not `std`,
//! and it has no parser: it loads the `.asni` files `asnkitcc` makes.
//!
//! It does BER, DER, OER, PER and UPER, and checks the constraints of
//! a value, and it does what the runtime for Python does: the vectors
//! in `tests/vectors` are what that did, and all of them agree.
//!
//! ```
//! use asnkitrt::container::Checks;
//! use asnkitrt::runtime::{Options, Specification};
//! use asnkitrt::types::Codec;
//! use asnkitrt::value::Value;
//!
//! # fn main() -> asnkitrt::Result<()> {
//! // A compiled specification, loaded for a codec.
//! let data = include_bytes!("../../../examples/compiled/foo.asni");
//! let foo = Specification::from_asni(data, &Checks::default(), Options::new(Codec::Ber))?;
//!
//! let question = Value::Map(vec![
//!     ("id".into(), Value::Integer(1)),
//!     ("question".into(), Value::text("Is 1+1=3?")),
//! ]);
//! let encoded = foo.encode("Question", &question)?;
//!
//! assert_eq!(encoded, b"0\x0e\x02\x01\x01\x16\x09Is 1+1=3?");
//! assert_eq!(foo.decode("Question", &encoded)?, question);
//! # Ok(())
//! # }
//! ```
//!
//! # Features
//!
//! - `std`, on by default.
//! - `signing` and `encryption`, on by default, to load `.asni` files
//!   that are signed or encrypted. A device that is sent its
//!   specifications builds with `default-features = false` and these.
//! - `serde`, on by default, for [`serde_value::to_value`] and
//!   [`serde_value::from_value`].
//!
//! # What is not done
//!
//! `ANY` in JER and XER, which need its type, and the types `EMBEDDED
//! PDV`, `CHARACTER STRING` and `DURATION`.

#![no_std]
#![forbid(unsafe_code)]

extern crate alloc;
#[cfg(feature = "std")]
extern crate std;

pub mod ber;
#[cfg(feature = "bigint")]
pub mod bigint;
pub mod cbor;
pub mod check;
pub mod container;
#[cfg(any(feature = "jer", feature = "xer"))]
mod decimal;
mod error;
#[cfg(feature = "jer")]
pub mod jer;
#[cfg(feature = "jer")]
mod json;
pub mod kinds;
pub mod node;
pub mod oer;
pub mod per;
pub mod runtime;
#[cfg(feature = "serde")]
pub mod serde_value;
pub mod time;
pub mod types;
pub mod value;
#[cfg(feature = "xer")]
pub mod xer;
#[cfg(feature = "xer")]
mod xml;

pub use error::{Error, Result};
