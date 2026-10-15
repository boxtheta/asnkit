# Changelog

All packages of asnkit are released together, with one version. The
format follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and the versions [Semantic Versioning](https://semver.org/): the `.asnc`
and `.asnst` formats, the runtime APIs and the command line keep
working within a major version. `SECURITY.md` tells which releases get
security fixes.

## [1.0.0] - unreleased

The first release of asnkit.

### Added

- The compiler `asnkitcc` (crate `asnkit-compiler` as a library), in
  Rust: ASN.1 to compiled specifications (`.asnc`, version 1.0 of the
  format) and syntax trees (`.asnst`), with signing, encryption, a
  nonce and binding of `.asnc` files.
- Runtimes that load `.asnc` files, for Python (`asnkitrt` on PyPI),
  Rust (`asnkitrt` on crates.io, also `no_std` with `alloc`) and
  browsers and Node.js (`asnkitrt` on npm, the Rust runtime as
  WebAssembly). The Rust runtime is the one of reference.
- Codecs: BER, DER, OER, PER, UPER, JER and XER in every runtime, and
  GSER in Python. Constraint checks in every runtime.
- Plugins that generate code from a compiled specification:
  `asnkit-gen-python` (data classes) and `asnkit-gen-rust` (structs and
  enums with `serde`).
- The specification of the file formats, with the media types
  `application/vnd.asnkit.asnc` and
  `application/vnd.asnkit.asnst+cbor`, guidelines for third-party
  runtimes, a corpus of `.asnc` files and more than 5,300 test vectors.

[1.0.0]: https://github.com/boxtheta/asnkit/releases/tag/v1.0.0
