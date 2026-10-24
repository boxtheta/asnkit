// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;
use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

/// A step from a value to one in it: the name of a type, of a member of
/// a SEQUENCE or a SET or of an alternative of a CHOICE, or the index of
/// an element of a SEQUENCE OF or a SET OF.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    Name(String),
    Index(usize),
}

/// Where in a value an error is, from the type that is encoded,
/// decoded or checked, as `Message.payload.reading.flags` or
/// `Message.tags[1]`. Nothing if it is not known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
// A pointer, and none for no location, so that an error, and every
// result of the codecs, is no larger for it.
#[allow(clippy::box_collection)]
pub struct Location(Option<Box<Vec<Step>>>);

impl Location {
    /// The steps, from the outside in.
    pub fn steps(&self) -> &[Step] {
        self.0.as_deref().map_or(&[], Vec::as_slice)
    }

    pub fn is_empty(&self) -> bool {
        self.steps().is_empty()
    }

    fn prepend(&mut self, step: Step) {
        self.0.get_or_insert_default().insert(0, step);
    }
}

impl fmt::Display for Location {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, step) in self.steps().iter().enumerate() {
            match step {
                Step::Name(name) if index == 0 => f.write_str(name)?,
                Step::Name(name) => write!(f, ".{name}")?,
                Step::Index(element) => write!(f, "[{element}]")?,
            }
        }

        Ok(())
    }
}

/// What can go wrong reading a file. It is the same for every
/// failed check, and tells nothing of the contents of the file.
///
/// What goes wrong with a value, encoding, decoding or checking it,
/// also tells where in the value it is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// The CBOR ends before its item does.
    OutOfData,
    /// The CBOR is nested deeper than the limit.
    TooDeep,
    /// The CBOR has something this package does not read.
    UnsupportedCbor,
    /// A CBOR text string is not UTF-8.
    BadText,
    /// There is data after the nodes.
    TrailingData,
    /// Not an `.asni` file.
    NotAsni,
    /// A major version that is not known.
    UnsupportedVersion,
    /// A flag that is not known.
    UnsupportedFlags,
    /// A file that is signed or encrypted, with this package built
    /// without the feature to read it.
    FeatureMissing,
    /// A signed file, and the reader is set up with no keys.
    NoVerificationKeys,
    /// An unsigned file, and the reader is set up with keys.
    NotSigned,
    /// The key of the signature is not among those given.
    UnknownSigner,
    /// The signature is wrong.
    BadSignature,
    /// The nonce is not what the reader expects.
    BadNonce,
    /// An encrypted file, and the reader is set up with no keys.
    NoDecryptionKeys,
    /// An unencrypted file, and the reader is set up with keys.
    NotEncrypted,
    /// The key of the encryption is not among those given.
    UnknownEncryptionKey,
    /// The tag is wrong: the file or the key is not the one.
    BadTag,
    /// What the file is bound to is not what the reader expects.
    BadBinding,
    /// The metadata is not what it has to be.
    BadMetadata,
    /// The nodes are not what a compiled specification has.
    BadNodes(&'static str),
    /// A value that can not be encoded, and where in it.
    Encode(&'static str, Location),
    /// Data that can not be decoded, the offset it was found at: bytes
    /// for BER, DER and OER and bits for PER and UPER, and where in the
    /// value.
    Decode(&'static str, usize, Location),
    /// A value that does not fulfil its constraints, and where in it.
    Constraints(&'static str, Location),
    /// Something the runtime does not do (yet), and where in the value,
    /// if it is about one.
    Unsupported(&'static str, Location),
    /// A type that is not in the specification.
    NoSuchType,
    /// The buffer an encoding is written to is too small: it needs this
    /// many bytes.
    BufferTooSmall(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::OutOfData => "Out of CBOR data.",
            Error::TooDeep => "The CBOR is nested too deeply.",
            Error::UnsupportedCbor => "Unsupported CBOR.",
            Error::BadText => "Bad CBOR text string.",
            Error::TrailingData => "Unexpected data after the nodes.",
            Error::NotAsni => "Expected an .asni file.",
            Error::UnsupportedVersion => "Unsupported major version.",
            Error::UnsupportedFlags => "Unsupported flags.",
            Error::FeatureMissing => {
                "The file is signed or encrypted, which this build can not read."
            }
            Error::NoVerificationKeys => "The file is signed, but no keys to verify it with are given.",
            Error::NotSigned => "Expected a signed file.",
            Error::UnknownSigner => "The file is signed with a key that is not among those given.",
            Error::BadSignature => "The signature of the file is wrong.",
            Error::BadNonce => "The nonce of the file is not accepted.",
            Error::NoDecryptionKeys => "The file is encrypted, but no keys to decrypt it with are given.",
            Error::NotEncrypted => "Expected an encrypted file.",
            Error::UnknownEncryptionKey => {
                "The file is encrypted with a key that is not among those given."
            }
            Error::BadTag => {
                "The file is not what was encrypted, or the key is not the one it was encrypted with."
            }
            Error::BadBinding => "The file is not bound to what is expected.",
            Error::BadMetadata => "Bad metadata.",
            Error::Encode(what, location)
            | Error::Decode(what, _, location)
            | Error::Constraints(what, location)
            | Error::Unsupported(what, location)
                if !location.is_empty() =>
            {
                return write!(f, "{location}: {what}");
            }
            Error::BadNodes(what)
            | Error::Encode(what, _)
            | Error::Constraints(what, _)
            | Error::Unsupported(what, _) => what,
            Error::NoSuchType => "The type is not found in the specification.",
            Error::BufferTooSmall(needed) => {
                return write!(f, "The buffer is too small: the encoding is {needed} bytes.");
            }
            Error::Decode(what, _, _) => what,
        })
    }
}

impl Error {
    /// A value that can not be encoded, where is not known yet.
    pub fn encode(what: &'static str) -> Error {
        Error::Encode(what, Location::default())
    }

    /// Data that can not be decoded at `offset`, where in the value is
    /// not known yet.
    pub fn decode(what: &'static str, offset: usize) -> Error {
        Error::Decode(what, offset, Location::default())
    }

    /// A value that does not fulfil its constraints, where is not known
    /// yet.
    pub fn constraints(what: &'static str) -> Error {
        Error::Constraints(what, Location::default())
    }

    /// Something the runtime does not do, where in the value, if it is
    /// about one, is not known yet.
    pub fn unsupported(what: &'static str) -> Error {
        Error::Unsupported(what, Location::default())
    }

    /// Where in the value the error is, if it is about a value.
    pub fn location(&self) -> Option<&Location> {
        match self {
            Error::Encode(_, location)
            | Error::Decode(_, _, location)
            | Error::Constraints(_, location) => Some(location),
            Error::Unsupported(_, location) if !location.is_empty() => Some(location),
            _ => None,
        }
    }

    /// The error, found in the value at `step` of the one around it.
    pub(crate) fn within(mut self, step: Step) -> Error {
        if let Error::Encode(_, location)
        | Error::Decode(_, _, location)
        | Error::Constraints(_, location)
        | Error::Unsupported(_, location) = &mut self
        {
            location.prepend(step);
        }

        self
    }

    /// The error, found in a value of the type `name`.
    pub(crate) fn in_type(self, name: &str) -> Error {
        self.within(Step::Name(String::from(name)))
    }

    /// The error, found in the member or the alternative `name`.
    pub(crate) fn in_member(self, name: &str) -> Error {
        self.within(Step::Name(String::from(name)))
    }

    /// The error, found in the element `index`.
    pub(crate) fn in_element(self, index: usize) -> Error {
        self.within(Step::Index(index))
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
