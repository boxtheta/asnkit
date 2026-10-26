// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! What the code that `asnkit-gen-rust-codec` generates is built on: it
//! encodes and decodes the types of one specification, with BER, DER
//! and UPER, without loading it and without allocating.
//!
//! A value is encoded into a buffer that is given, and decoded into a
//! value that borrows the data, as [`Octets`], [`Text`] and
//! [`SequenceOf`] do. The rules of the codecs are those of the
//! interpreter in [`crate::ber`] and [`crate::per`], which share them.
//!
//! This module is for the generated code, which is made for the version
//! of the runtime it is made with. It may change between minor versions.

pub mod ber;
pub mod per;
mod views;

pub use crate::node::TagClass;
pub use crate::types::{Range, StrKind};
pub use crate::value::{Date, DateTime, Time};
pub use crate::{Error, Result};
#[cfg(feature = "serde")]
pub(crate) use views::other_bits;
pub use views::{
    BitString, Bytes, Chars, Integer, Iter, Octets, Oid, SequenceOf, Subidentifiers, Text,
    named_bits,
};

/// A type of a specification, with the code to encode and decode it.
pub trait Asn1<'a>: Sized {
    /// Writes the value with BER, or with DER if `der`.
    fn write_ber(&self, w: &mut ber::Writer<'_>, der: bool) -> Result<()>;

    /// Reads a value from where the reader is.
    fn read_ber(r: &mut ber::Reader<'a>) -> Result<Self>;

    fn write_uper(&self, w: &mut per::Writer<'_>) -> Result<()>;

    fn read_uper(r: &mut per::Reader<'a>) -> Result<Self>;

    /// Encodes the value with BER into `out`, and returns how many bytes
    /// it is, or [`Error::BufferTooSmall`] with how many it would be.
    fn encode_ber(&self, out: &mut [u8]) -> Result<usize> {
        let mut w = ber::Writer::new(out);

        self.write_ber(&mut w, false)?;
        w.finish()
    }

    /// Encodes the value with DER, as [`Asn1::encode_ber`] does.
    fn encode_der(&self, out: &mut [u8]) -> Result<usize> {
        let mut w = ber::Writer::new(out);

        self.write_ber(&mut w, true)?;
        w.finish()
    }

    /// Encodes the value with UPER, as [`Asn1::encode_ber`] does.
    fn encode_uper(&self, out: &mut [u8]) -> Result<usize> {
        let mut w = per::Writer::new(out);

        self.write_uper(&mut w)?;
        w.finish()
    }

    /// How many bytes the value is in BER.
    fn ber_len(&self) -> Result<usize> {
        needed(self.encode_ber(&mut []))
    }

    /// How many bytes the value is in DER.
    fn der_len(&self) -> Result<usize> {
        needed(self.encode_der(&mut []))
    }

    /// How many bytes the value is in UPER.
    fn uper_len(&self) -> Result<usize> {
        needed(self.encode_uper(&mut []))
    }

    /// Decodes a value from `data`, which is all of it, with BER.
    fn decode_ber(data: &'a [u8]) -> Result<Self> {
        let mut r = ber::Reader::new(data, false);
        let value = Self::read_ber(&mut r)?;

        r.finish()?;

        Ok(value)
    }

    /// Decodes a value from `data`, which is all of it and has to be DER.
    fn decode_der(data: &'a [u8]) -> Result<Self> {
        let mut r = ber::Reader::new(data, true);
        let value = Self::read_ber(&mut r)?;

        r.finish()?;

        Ok(value)
    }

    /// Decodes a value from `data` with UPER. What is after the value in
    /// its last byte is not looked at, nor are bytes after it.
    fn decode_uper(data: &'a [u8]) -> Result<Self> {
        Self::read_uper(&mut per::Reader::new(data, false))
    }

    /// The value in BER, in a vector of the size it needs.
    fn to_ber(&self) -> Result<alloc::vec::Vec<u8>> {
        to_vec(self.ber_len()?, |out| self.encode_ber(out))
    }

    fn to_der(&self) -> Result<alloc::vec::Vec<u8>> {
        to_vec(self.der_len()?, |out| self.encode_der(out))
    }

    fn to_uper(&self) -> Result<alloc::vec::Vec<u8>> {
        to_vec(self.uper_len()?, |out| self.encode_uper(out))
    }
}

/// Encodes `value` into `out` with BER, or DER if `der`, as `write`
/// writes it, for a type that is not one of its own, as an INTEGER.
pub fn encode_ber_with<T: ?Sized>(
    value: &T,
    out: &mut [u8],
    der: bool,
    write: impl FnOnce(&T, &mut ber::Writer<'_>, bool) -> Result<()>,
) -> Result<usize> {
    let mut w = ber::Writer::new(out);

    write(value, &mut w, der)?;
    w.finish()
}

/// Decodes a value from `data`, which is all of it, as `read` reads it.
pub fn decode_ber_with<'a, T>(
    data: &'a [u8],
    der: bool,
    read: impl FnOnce(&mut ber::Reader<'a>) -> Result<T>,
) -> Result<T> {
    let mut r = ber::Reader::new(data, der);
    let value = read(&mut r)?;

    r.finish()?;

    Ok(value)
}

pub fn encode_uper_with<T: ?Sized>(
    value: &T,
    out: &mut [u8],
    write: impl FnOnce(&T, &mut per::Writer<'_>) -> Result<()>,
) -> Result<usize> {
    let mut w = per::Writer::new(out);

    write(value, &mut w)?;
    w.finish()
}

pub fn decode_uper_with<'a, T>(
    data: &'a [u8],
    read: impl FnOnce(&mut per::Reader<'a>) -> Result<T>,
) -> Result<T> {
    read(&mut per::Reader::new(data, false))
}

/// How many bytes an encoding is, from one into an empty buffer.
pub fn encoded_len(result: Result<usize>) -> Result<usize> {
    needed(result)
}

/// The length an encoding into an empty buffer tells it needs.
fn needed(result: Result<usize>) -> Result<usize> {
    match result {
        Ok(length) | Err(Error::BufferTooSmall(length)) => Ok(length),
        Err(error) => Err(error),
    }
}

fn to_vec(
    length: usize,
    encode: impl FnOnce(&mut [u8]) -> Result<usize>,
) -> Result<alloc::vec::Vec<u8>> {
    let mut out = alloc::vec![0; length];
    let length = encode(&mut out)?;

    out.truncate(length);

    Ok(out)
}
