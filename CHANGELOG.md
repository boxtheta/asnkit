# Changelog

All packages of asnkit are released together, with one version. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions [Semantic Versioning](https://semver.org/), as
`docs/versioning.md` says: a major release may break the public API, a
minor release may change what is encoded and decoded, and a patch
release changes neither. `SECURITY.md` tells which releases get
security fixes.

## [Unreleased]

### Added

- Explicit `ANY DEFINED BY` type mappings in the compiler, with repeatable
  `--any-defined-by` TOML files for `asnkitcc compile` and `generate`,
  and `CompileOptions` entry points in the Rust compiler library. The
  existing `.asni` choice nodes carry the mappings to the runtimes;
  fallback types and load-time overrides are not yet supported.

- GSER in the Rust runtime, with the feature `gser`, and so in the
  runtime for WebAssembly, the C API and `asnkitcc convert --to gser`.
  It is encoded only, for debug output, and written as the runtime for
  Python writes it, except that a `"` in a string is written twice and
  a REAL with an exponent is written as `1E20` (RFC 3641).

- `asnkitrt::value::ObjectIdentifier`, the value of an OBJECT
  IDENTIFIER in the Rust runtime: its contents octets of X.690 8.19,
  with `Display` and `FromStr` for its text (`1.2.840.113549`),
  `arcs()` and `as_bytes()`. With serde it is its text.
- `Specification::decode_owned()` and `Value::into_owned()`, which give
  a value that borrows nothing, and `Value::reborrow()`.
- `ASNKIT_OBJECT_IDENTIFIER` in the C API, with
  `asnkit_value_new_object_identifier()`,
  `asnkit_value_object_identifier()` and
  `asnkit_value_object_identifier_bytes()`.

### Changed

- `Value` of the Rust runtime is `Value<'a>`, and borrows what it can:
  the names of the members of a `Map` and of the alternative of a
  `Choice` are `Cow<'a, str>`, `Value::Text` is a `Cow<'a, str>`, and
  `Value::Bytes` and `Value::Bits` a `Cow<'a, [u8]>`.
  `Specification::decode(&'a self, name, data: &'a [u8])` gives a
  `Value<'a>`, which borrows the names from the specification, and the
  texts, bytes and object identifiers from the data where their
  encoding is the value (a primitive string of BER and DER, all of OER,
  and what is on a byte in PER and UPER). Build a value with
  `("id".into(), …)` and `Value::Text("…".into())`. `from_value` can
  give a type that borrows from the value, as a `&'a str`.
- An OBJECT IDENTIFIER is a `Value::ObjectIdentifier` in the Rust
  runtime, and no longer a `Value::Text`, which encoding it no longer
  takes. JER, XER and GSER reject the text of one that is not valid,
  and every codec an arc of 2^127 or more. `asnkit-gen-rust` gives it
  the type `asnkitrt::value::ObjectIdentifier<'static>`, and an `ANY`
  the type `asnkitrt::value::Value<'static>`. The C API has it as
  `ASNKIT_OBJECT_IDENTIFIER`; the runtime for WebAssembly and the
  runtime for Python still have it as a string.

- An error of the Rust runtime with a value tells where in it it is,
  from the type in, as the runtime for Python does:
  `Certificate.tbsCertificate.extensions[1].critical: Expected data of
  type bool.` `Error::Encode`, `Error::Decode` and `Error::Constraints`
  have a `Location` as their last field, `Error::location()` gives it,
  and `Error::encode()`, `Error::decode()` and `Error::constraints()`
  make one without. So does `Error::Unsupported`, made with
  `Error::unsupported()`, which has a location when it is found in a
  value, as an `ANY` in JER, and none when it is of the codec, as a
  codec that is not compiled in. `Error` is no longer `Copy`. The
  messages of the C API and `asnkitcc convert` start with the location,
  and the errors of the runtime for WebAssembly have it as `path`, as
  `['Certificate', 'tbsCertificate', 'extensions', 1, 'critical']`.
- The Rust runtime, and so the runtime for WebAssembly, the C API and
  `asnkitcc`, is faster, with the same bytes encoded and decoded: an
  X.509 certificate is decoded 1.7 to 2 times as fast in BER, DER, OER,
  PER and UPER, and encoded 1.8 to 4.3 times as fast. It no longer
  copies the encoding of each value of BER and DER into the one around
  it, reads and writes the bits of PER and UPER a byte at a time and
  not a bit at a time, converts the time of a UTCTime or a
  GeneralizedTime to UTC without counting the years from year 1, and
  allocates less, as for OBJECT IDENTIFIERs and the members of a
  SEQUENCE or a SET. As a decoded value borrows, a decode of the
  certificate is then twice as fast again, with 42 allocations instead
  of 155, and an encode 1.3 to 1.5 times as fast, as it looks a member
  up in the order of the type.
- GSER writes a REAL given as an integer as the float it is, `8.0E0`
  and not `8E0`, in the runtime for Python too, so that the runtime for
  WebAssembly, where every number is a float, writes the same.

### Removed

- The dictionaries of specifications in the runtime for Python,
  deprecated in 1.2.0: `asnkitrt.runtime.load_dict()` and
  `asnkitrt.syntax.from_dict()`, `to_dict()`, `compiled_from_dict()`
  and `compiled_to_dict()`. Load an `.asni` file or a compiled
  specification with `asnkitrt.load()`, and read the nodes of a syntax
  tree or a compiled specification instead.

## [1.2.0] - 2026-10-18

### Deprecated

- The dictionaries of specifications in the runtime for Python:
  `asnkitrt.runtime.load_dict()` and `asnkitrt.syntax.from_dict()`,
  `to_dict()`, `compiled_from_dict()` and `compiled_to_dict()`. They
  warn with a `DeprecationWarning`, and are removed in 2.0. Load an
  `.asni` file or a compiled specification with `asnkitrt.load()`, and
  read the nodes of a syntax tree or a compiled specification instead.

## [1.1.0] - 2026-10-17

### Added

- `asnkitcc convert`, which decodes a value with one codec and encodes
  it with another, as JER to DER, with an `.asni` file or ASN.1 files
  it compiles first. `asnkitcc` now depends on the runtime `asnkitrt`.
- The compiler for browsers and Node.js, as WebAssembly, in
  `packages/asnkit-compiler-wasm`: it compiles and parses the text of a
  specification, and signs, encrypts, binds and gives a nonce as
  `asnkitcc` does. It is on npm as `asnkit-compiler`.
- The runtime for C, in `packages/asnkitrt-c`: the Rust runtime as a
  shared and a static library, with the header `asnkitrt.h`. It is
  built from source.

### Changed

- `asnkit-compiler` builds for `wasm32-unknown-unknown`, where it has no
  module `plugin`, as WebAssembly runs no programs. `aes-gcm` is used
  without its default features, which the compiler did not use.

- The option `embed` of the plugins `asnkit-gen-python` and
  `asnkit-gen-rust`: with `embed=false`, the module made does not have
  the compiled specification in it, and is given one that is loaded
  from a file, as one that is signed, encrypted, bound or has a nonce.
  The module of Python then has `use_asni()`.

### Fixed

- XER, in every runtime: a BOOLEAN is read only as `<true/>` or
  `<false/>`, and anything else is an error, where it was read as false.
  An OCTET STRING with an odd number of hexadecimal digits is ended by a
  zero (X.680 22.3), so `<A>123</A>` is `12 30`, where it was `01 23`.
  A BIT STRING and an OCTET STRING may have space in them (X.680 12.10,
  12.12), and the runtime for Python no longer takes `0b`, `_` or `+` in
  a BIT STRING. A carriage return in a string is written `&#13;`, as one
  written as it is is read as a line feed.
- The playground no longer tells a byte offset for an error in JER or
  XER, which the runtimes do not give.

## [1.0.1] - 2026-10-15

What the runtimes write and read now follows the standards where it
did not. Values that were encoded otherwise before are listed below.
The WebAssembly runtime 1.0.0 on npm was made after these changes, and
already has them.

### Fixed

- `GeneralizedTime`: a time with a fraction of a second and a second of
  0 is written with its seconds, as a fraction after the minute is one
  of a minute (X.680 46.3). A decoder reads a fraction of a minute or of
  an hour, a time with no minute, an offset of hours only and a
  fraction of any number of digits, rounded down to the microsecond.
- JER: minus zero is the string `"-0"`, and is read as minus zero
  (X.697 23.2). An `ENUMERATED` is always written as its name, also
  when the runtime gives numbers (X.697 22). A `BIT STRING` is written
  with the bytes of its bits only, the bits past them 0, and with named
  bits made the size of a fixed size (X.697 24.2).
- XER: a control character in a string is an empty element, as
  `<bel/>`, and is read back (X.680 12.15.5).
- `TIME-OF-DAY` and `DATE-TIME`: a value with a fraction of a second or
  an offset is rejected when encoding, as these types have neither
  (X.680 38.4). The WebAssembly runtime takes a `Date` for them as its
  time in UTC with no offset.
- A default of a `SEQUENCE`, a `SET`, a `CHOICE`, a `SEQUENCE OF` or a
  `SET OF` has the values of its members and items as their own types.
  The Rust runtime left out a member of an `OCTET STRING` or a `BIT
  STRING` in one.

### Changed

- The WebAssembly runtime gives values to and takes them from the
  WebAssembly in one buffer, which encodes and decodes about twice as
  fast. An object is read by its own enumerable properties.
- The specification of the formats tells more of what a runtime needs:
  the values of the fields of the nodes, the limits of the CBOR, how a
  signature is verified, and what BER, JER and XER write where the
  standards let them choose.

## [1.0.0] - 2026-10-15

The first release of asnkit, of the crates on crates.io and the
WebAssembly runtime on npm.

### Added

- The compiler `asnkitcc` (crate `asnkit-compiler` as a library), in
  Rust: ASN.1 to compiled specifications (`.asni`, version 1.0 of the
  format) and syntax trees (`.asnst`), with signing, encryption, a
  nonce and binding of `.asni` files.
- Runtimes that load `.asni` files, for Python (`asnkitrt` on PyPI),
  Rust (`asnkitrt` on crates.io, also `no_std` with `alloc`) and
  browsers and Node.js (`asnkitrt` on npm, the Rust runtime as
  WebAssembly). The Rust runtime is the one of reference.
- Codecs: BER, DER, OER, PER, UPER, JER and XER in every runtime, and
  GSER in Python. Constraint checks in every runtime.
- Plugins that generate code from a compiled specification:
  `asnkit-gen-python` (data classes) and `asnkit-gen-rust` (structs and
  enums with `serde`).
- The specification of the file formats, with the media types
  `application/vnd.asnkit.asni` and
  `application/vnd.asnkit.asnst+cbor`, guidelines for third-party
  runtimes, a corpus of `.asni` files and more than 5,300 test vectors.

[1.2.0]: https://github.com/boxtheta/asnkit/releases/tag/v1.2.0
[1.1.0]: https://github.com/boxtheta/asnkit/releases/tag/v1.1.0
[1.0.1]: https://github.com/boxtheta/asnkit/releases/tag/v1.0.1
[1.0.0]: https://crates.io/crates/asnkitrt/1.0.0
