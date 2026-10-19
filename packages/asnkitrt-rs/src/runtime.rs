// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

//! Loads a compiled specification and encodes and decodes with it.

use alloc::vec::Vec;

use crate::ber;
use crate::cbor::Cbor;
use crate::check;
use crate::container::{self, Checks};
use crate::node::Spec;
use crate::oer;
use crate::per;
use crate::types::{self, Codec, TypeId, Types};
use crate::value::Value;
use crate::{Error, Result};

/// How a specification is loaded.
#[derive(Debug, Clone, Copy)]
pub struct Options {
    pub codec: Codec,
    /// ENUMERATED values are numbers and not names.
    pub numeric_enums: bool,
    /// DER decodes what BER allows. Only DER looks at it.
    pub lenient: bool,
}

impl Options {
    pub fn new(codec: Codec) -> Options {
        Options {
            codec,
            numeric_enums: false,
            lenient: false,
        }
    }
}

/// A compiled specification, loaded for a codec.
#[derive(Debug, Clone)]
pub struct Specification {
    types: Types,
}

impl Specification {
    /// Loads the nodes of a compiled specification.
    pub fn from_nodes(nodes: &Cbor, options: Options) -> Result<Specification> {
        let spec = Spec::from_nodes(nodes)?;

        Ok(Specification {
            types: types::lower(&spec, options.codec, options.numeric_enums, options.lenient)?,
        })
    }

    /// Loads the contents of an `.asni` file, after `checks`.
    pub fn from_asni(data: &[u8], checks: &Checks<'_>, options: Options) -> Result<Specification> {
        Specification::from_nodes(&container::read(data, checks)?.nodes, options)
    }

    pub fn types(&self) -> &Types {
        &self.types
    }

    fn find(&self, name: &str) -> Result<TypeId> {
        self.types.find_unique(name).ok_or(Error::NoSuchType)
    }

    /// Checks `value` against the constraints of the type `name`.
    pub fn check_constraints(&self, name: &str, value: &Value) -> Result<()> {
        check::check(&self.types, self.find(name)?, value).map_err(|error| error.in_type(name))
    }

    /// Checks `value` against the constraints of the type `name` of the
    /// module `module`.
    pub fn check_constraints_in(&self, module: &str, name: &str, value: &Value) -> Result<()> {
        check::check(
            &self.types,
            self.types.find(module, name).ok_or(Error::NoSuchType)?,
            value,
        )
        .map_err(|error| error.in_type(name))
    }

    /// Encodes `value` as the type `name`, which has to be found in one
    /// of the modules only. See [`Specification::encode_in`] for the
    /// others.
    pub fn encode(&self, name: &str, value: &Value) -> Result<Vec<u8>> {
        self.encode_id(name, self.find(name)?, value)
    }

    /// Encodes `value` as the type `name` of the module `module`.
    pub fn encode_in(&self, module: &str, name: &str, value: &Value) -> Result<Vec<u8>> {
        self.encode_id(
            name,
            self.types.find(module, name).ok_or(Error::NoSuchType)?,
            value,
        )
    }

    /// Encodes `value` as the type `name`, found as `id`. An error in the
    /// value is located from `name`, and one of the codec, as a missing
    /// feature, has no location.
    fn encode_id(&self, name: &str, id: TypeId, value: &Value) -> Result<Vec<u8>> {
        let encoded = match self.types.codec {
            Codec::Ber | Codec::Der => ber::encode(&self.types, id, value),
            Codec::Per | Codec::Uper => per::encode(&self.types, id, value),
            Codec::Oer => oer::encode(&self.types, id, value),
            #[cfg(feature = "jer")]
            Codec::Jer => crate::jer::encode(&self.types, id, value),
            #[cfg(not(feature = "jer"))]
            Codec::Jer => return Err(Error::unsupported("JER needs the feature jer.")),
            #[cfg(feature = "xer")]
            Codec::Xer => crate::xer::encode(&self.types, id, value),
            #[cfg(not(feature = "xer"))]
            Codec::Xer => return Err(Error::unsupported("XER needs the feature xer.")),
            #[cfg(feature = "gser")]
            Codec::Gser => crate::gser::encode(&self.types, id, value),
            #[cfg(not(feature = "gser"))]
            Codec::Gser => return Err(Error::unsupported("GSER needs the feature gser.")),
        };

        encoded.map_err(|error| error.in_type(name))
    }

    /// Encodes `value` as the type `name`, as [`Specification::encode`]
    /// does, with `indent` spaces for each level of JER, XER and GSER, as
    /// the runtime for Python does. The other codecs are not indented and
    /// return [`Error::Unsupported`].
    pub fn encode_indented(&self, name: &str, value: &Value, indent: usize) -> Result<Vec<u8>> {
        self.encode_indented_id(name, self.find(name)?, value, indent)
    }

    /// Encodes `value` as the type `name` of the module `module`, with
    /// `indent` spaces for each level.
    pub fn encode_in_indented(
        &self,
        module: &str,
        name: &str,
        value: &Value,
        indent: usize,
    ) -> Result<Vec<u8>> {
        self.encode_indented_id(
            name,
            self.types.find(module, name).ok_or(Error::NoSuchType)?,
            value,
            indent,
        )
    }

    #[allow(unused_variables)]
    fn encode_indented_id(
        &self,
        name: &str,
        id: TypeId,
        value: &Value,
        indent: usize,
    ) -> Result<Vec<u8>> {
        // Each maps its own errors, as none may be compiled in.
        match self.types.codec {
            #[cfg(feature = "jer")]
            Codec::Jer => crate::jer::encode_indented(&self.types, id, value, indent)
                .map_err(|error| error.in_type(name)),
            #[cfg(feature = "xer")]
            Codec::Xer => crate::xer::encode_indented(&self.types, id, value, indent)
                .map_err(|error| error.in_type(name)),
            #[cfg(feature = "gser")]
            Codec::Gser => crate::gser::encode_indented(&self.types, id, value, indent)
                .map_err(|error| error.in_type(name)),
            _ => Err(Error::unsupported("Only JER, XER and GSER are indented.")),
        }
    }

    /// Decodes `data` as the type `name`.
    pub fn decode(&self, name: &str, data: &[u8]) -> Result<Value> {
        self.decode_id(name, self.find(name)?, data)
    }

    pub fn decode_in(&self, module: &str, name: &str, data: &[u8]) -> Result<Value> {
        self.decode_id(
            name,
            self.types.find(module, name).ok_or(Error::NoSuchType)?,
            data,
        )
    }

    fn decode_id(&self, name: &str, id: TypeId, data: &[u8]) -> Result<Value> {
        let decoded = match self.types.codec {
            Codec::Ber | Codec::Der => ber::decode(&self.types, id, data),
            Codec::Per | Codec::Uper => per::decode(&self.types, id, data),
            Codec::Oer => oer::decode(&self.types, id, data),
            #[cfg(feature = "jer")]
            Codec::Jer => crate::jer::decode(&self.types, id, data),
            #[cfg(not(feature = "jer"))]
            Codec::Jer => return Err(Error::unsupported("JER needs the feature jer.")),
            #[cfg(feature = "xer")]
            Codec::Xer => crate::xer::decode(&self.types, id, data),
            #[cfg(not(feature = "xer"))]
            Codec::Xer => return Err(Error::unsupported("XER needs the feature xer.")),
            #[cfg(feature = "gser")]
            Codec::Gser => crate::gser::decode(&self.types, id, data),
            #[cfg(not(feature = "gser"))]
            Codec::Gser => return Err(Error::unsupported("GSER needs the feature gser.")),
        };

        decoded.map_err(|error| error.in_type(name))
    }

    /// Decodes the first value of `data` as the type `name`, and
    /// returns it with its length.
    pub fn decode_with_length(&self, name: &str, data: &[u8]) -> Result<(Value, usize)> {
        match self.types.codec {
            Codec::Ber | Codec::Der => ber::decode_with_length(&self.types, self.find(name)?, data)
                .map_err(|error| error.in_type(name)),
            _ => Err(Error::unsupported("This codec is not done yet.")),
        }
    }

    /// The length of the first value of `data`, if its length is
    /// there. Only BER and DER have it.
    pub fn decode_length(&self, data: &[u8]) -> Result<Option<usize>> {
        match self.types.codec {
            Codec::Ber | Codec::Der => Ok(ber::decode_length(data)),
            _ => Err(Error::unsupported("This codec has no length in the data.")),
        }
    }
}
