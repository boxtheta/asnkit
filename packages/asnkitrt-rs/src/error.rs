// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

use core::fmt;

pub type Result<T> = core::result::Result<T, Error>;

/// What can go wrong reading a file. It is the same for every
/// failed check, and tells nothing of the contents of the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// Not an `.asnc` file.
    NotAsnc,
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
    /// A value that can not be encoded.
    Encode(&'static str),
    /// Data that can not be decoded, and the offset it was found at:
    /// bytes for BER, DER and OER and bits for PER and UPER.
    Decode(&'static str, usize),
    /// A value that does not fulfil its constraints.
    Constraints(&'static str),
    /// Something the runtime does not do (yet).
    Unsupported(&'static str),
    /// A type that is not in the specification.
    NoSuchType,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Error::OutOfData => "Out of CBOR data.",
            Error::TooDeep => "The CBOR is nested too deeply.",
            Error::UnsupportedCbor => "Unsupported CBOR.",
            Error::BadText => "Bad CBOR text string.",
            Error::TrailingData => "Unexpected data after the nodes.",
            Error::NotAsnc => "Expected an .asnc file.",
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
            Error::BadNodes(what)
            | Error::Encode(what)
            | Error::Constraints(what)
            | Error::Unsupported(what) => what,
            Error::NoSuchType => "The type is not found in the specification.",
            Error::Decode(what, _) => what,
        })
    }
}

#[cfg(feature = "std")]
impl std::error::Error for Error {}
