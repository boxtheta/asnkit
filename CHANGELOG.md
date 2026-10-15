# Changelog

All packages of asnkit are released together, with one version. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions [Semantic Versioning](https://semver.org/): the `.asni`
and `.asnst` formats, the runtime APIs and the command line keep
working within a major version. `SECURITY.md` tells which releases get
security fixes.

## [Unreleased]

### Added

- The option `embed` of the plugins `asnkit-gen-python` and
  `asnkit-gen-rust`: with `embed=false`, the module made does not have
  the compiled specification in it, and is given one that is loaded
  from a file, as one that is signed, encrypted, bound or has a nonce.
  The module of Python then has `use_asni()`.

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

[1.0.1]: https://github.com/boxtheta/asnkit/releases/tag/v1.0.1
[1.0.0]: https://crates.io/crates/asnkitrt/1.0.0
