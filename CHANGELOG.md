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

- `org.boxtheta:asnkitrt`: a runtime for the JVM, in Kotlin, in
  `packages/asnkitrt-jvm`. It loads `.asni` files, encodes and decodes
  with BER, DER, JER and XER, and checks constraints, with the same
  results as the runtime for Rust for all the vectors of those codecs.
  It needs Java 17 or later, reads CBOR and JSON with Jackson, and XML
  with Woodstox. It is not published to Maven Central yet.
- `asnkit-compiler`: warnings, of what is valid ASN.1 but likely to
  cause trouble. The first is `RecursiveType`, of types that refer to
  themselves, whose values nest as deep as the data has them, which the
  decoders limit to 128 levels. `compile_to_asni_with_warnings()` gives
  them, with the `.asni` file and the summary of the modules, as a
  `Compiled`.
- `asnkitcc`: `compile` and `generate` print the warnings on standard
  error, as `warning: ...`, and still write what they make.
- `asnkit-compiler` (WebAssembly): `compileWithSummary()` returns the
  warnings too, as `warnings`, each with its `kind` and `message`. The
  playground shows them.
- `asnkit-compiler`: fuzzing, in `packages/asnkit-compiler/fuzz`, of
  specifications that are anything, which are parsed, compiled and
  loaded by the Rust runtime. With `ASNKIT_FUZZ_STRICT=1`, what the
  compiler takes and the runtime refuses fails too. What it found is
  tested in `tests/fuzzed.rs`.

### Changed

- `asnkit-compiler`: a type defined by itself alone, through
  references, as `A ::= B` and `B ::= [0] A`, or through `COMPONENTS
  OF`, is a `CompileError`. The runtimes already refused such a
  specification when loading it.
- `asnkit-compiler`: a type of which every value would hold another,
  as `Never ::= SEQUENCE { next Never }`, is a `CompileError`, as none
  of its values could be encoded or decoded.
- `asnkit-compiler`: constraints that permit no value, as
  `INTEGER (5..1)`, or `(10..20)` on a type that is `(0..5)`, are a
  `CompileError`. They were written as an empty constraint, which the
  runtimes took for no constraint, and so accepted any value, even one
  that the type referred to does not permit. The type of a field of an
  object class, as `&id INTEGER (5..1)`, is one too, as the types that
  use the field take it. A root that permits no value but is
  extensible, as `(5..1, ...)`, is still accepted.
- `asnkit-compiler`: types, constraints and values nested more than 64
  levels deep in one another are a `ParseError`. The parser followed
  them on the stack, which ran out at a few hundred levels and aborted
  the program.
- `asnkit-compiler`: `SET` with no braces, as `P ::= SET`, is a
  `ParseError`, as `SEQUENCE` with none is. It was a reference to a
  type named `SET`, which every runtime refused the file for.
- `asnkit-compiler`: a field of an object class that is not found, as
  `C.&field` where `C` or its `&field` is not defined, is a
  `CompileError`. Every runtime refused the file for it, with every
  codec.
- `asnkit-compiler`: a REAL too large for a double, as `1.0e309`, is a
  `ParseError`, as the runtimes decode REAL to a double.
- `asnkit-compiler`: the number of a tag that is not from 0 to
  2^64 - 1, as `[111111111111111111111]`, or `[n]` where `n` is
  negative, is a `CompileError`. The runtimes for Rust, C and
  WebAssembly read it in 64 bits and refused the file.

### Fixed

- `asnkit-compiler`: a reference defined by itself, with a constraint
  or as the type of a member with a `DEFAULT`, was followed without
  end, and `COMPONENTS OF` itself used up the stack, which aborted the
  program.
- `asnkit-compiler`: a number with zeros before it, as `[00]` or
  `{ 1 04 }`, of a tag, an object identifier, a named number or bit or
  an enumeration, panicked. It is the number, as `007` of a value
  always was.
- `asnkit-compiler`: a named number of `-0` panicked. It is 0.
- `asnkit-compiler`: a REAL too large for a double, as in
  `REAL (0..1.e320)`, panicked.
- `asnkit-compiler`: a module that imports a name from itself, or from
  a module that imports it from the first, where it is not defined,
  used up the stack, which aborted the program. It is a `CompileError`.
- `asnkit-compiler`: a type in a constraint was parsed again for each
  way the constraint could be read, as were those in it, so that
  `E ::= A(A(A(...` took as long as three to the power of its depth:
  more than ten seconds at 12 levels. It is parsed once.
- `asnkit-compiler`: a reference that starts with a keyword and a
  hyphen, as `SEQUENCE-Foo`, `CHOICE-Foo` or `INTEGER-Foo`, was a
  `ParseError` where it was used, as the keyword was read from its
  start. Only `SET-` worked.
- `asnkitrt`: a type with a `size` or a `restricted_to` of no element,
  which an earlier compiler wrote for a constraint that permits no
  value, as `INTEGER (5..1)`, was taken for no constraint, and so any
  value was encoded and decoded. For a reference, it took the place of
  the constraint of the type it refers to, as `K ::= J (10..20)` with
  `J ::= INTEGER (0..5)` encoded 99. The runtimes for Rust, C,
  WebAssembly, Python and the JVM refuse such a file when loading it,
  as the compiler does such a type.
- `asnkitrt`: PER and UPER encoded a value of an extensible constraint
  whose root has no value, as `INTEGER (5..1, ...)` or
  `IA5String (SIZE (3..1, ...))`, as if it was in the root, with the
  extension bit 0, where every value is outside it (X.691 13.1, 16.6,
  17.3, 20.4, 30.4). The bit is 1, so 5 of that INTEGER is `808280` in
  UPER and not `008280`, and data with the bit 0 is rejected when
  decoding, as no value can be in that root. The same for the code
  `asnkit-gen-rust-codec` makes, and in the runtimes for Rust, C,
  WebAssembly and Python.

## [2.0.0-rc5] - 2026-11-02

Release candidates of a new major version make no promise of
compatibility with each other. The `.asni` files of 2.0.0-rc4 are not
read by this release: compile them again.

### Changed

- `.asni` format 2.0: the two bytes of flags at offset 6 are now the
  CRC-32 of the rest of the file (that of zlib, gzip and PNG), in four
  bytes, most significant first, so the header is 10 bytes and the
  metadata begins at offset 10. Every runtime checks it before it
  parses any CBOR, and rejects a file that is corrupt, cut short or
  added to. The magic and the version are not covered. It detects
  corruption, not tampering.
- `asnkit-compiler`: `compile_to_asni` and `compile_to_asni_with_summary`
  take `(input)`, and the options variants `(input, &CompileOptions)`.
- `asnkitrt` (Rust): `Checks` holds only `max_depth`, and has no
  lifetime. `container::HEADER_SIZE` is 10, and `container::crc32()`
  computes the CRC-32. `Error::BadChecksum` is what a corrupt file
  gives, and `container::read_header()` checks the CRC-32 too.
- `asnkitrt` (Python): `syntax.from_asni()` and `syntax.from_asni_file()`
  take only the file, and `load()`, `load_asni()` and `load_asni_file()`
  no longer take `bound_to` or `accept_any_binding`. A corrupt file
  raises `ConvertError`.
- `asnkit-gen-python`: `use_asni()` of a module made with `embed=false`
  takes only the file.
- `examples/corpus` has `plain.asni` and `corrupt.asni`, one bit of it
  flipped, in place of `bound.asni`.
- The specifications of `tests/vectors` have the new header, and so new
  names; the vectors are the same.

### Removed

- Binding of `.asni` files: `asnkitcc compile --bound-to`, the
  `boundTo` option of the WebAssembly compiler, the `boundTo` and
  `acceptAnyBinding` options of the WebAssembly runtime,
  `asnkit_load_options_set_bound_to()` and
  `asnkit_load_options_set_accept_any_binding()` of the C API,
  `container::Binding`, `Metadata::bound_to`, `Error::BadBinding` and
  `Error::UnsupportedFlags` in Rust, and `Metadata.bound_to` in Python.
  The command line and the WebAssembly packages fail on the old options
  instead of ignoring them. A `bound_to` in the metadata of a file is a
  key that is not known, and is ignored.

## [2.0.0-rc4] - 2026-11-02

### Changed

- `.asni` is now format 2.0, with an 8-byte header followed directly by
  metadata and nodes. Binding is flag bit 0; all other bits are rejected.
  Version 1 files must be recompiled from ASN.1 source. `.asnst` remains
  format 1.0.
- `asnkit-compiler`: `compile_to_asni` and its summary variant take
  `(input, bound_to)`; the options variants take an additional
  `&CompileOptions`. `Protection` and `derive_key_id` are removed.
- `asnkitrt` (Rust): `Checks` holds only `bound_to` and `max_depth`.
  Metadata is always at `Asni.header.metadata`; `Asni.metadata` is removed.
  Container parsing reads the input in place instead of copying it.
- `asnkitrt` (Python): load options after `numeric_enums` are keyword-only;
  syntax loading options after the data are keyword-only. Generated
  `use_asni` accepts only `bound_to` and `accept_any_binding` options.

- `asnkitrt`: a loaded specification keeps about a fifth of the heap it
  did, and loading takes about a third at its peak: RFC 5280 keeps
  132.5 KiB where it kept 615.4 KiB, and takes 337.3 KiB at its peak
  where it took 936.9 KiB, as `examples/load_memory.rs` measures.
  `Specification::from_asni` lets go of the file as CBOR before it
  lowers the types, and the types are counted before they are lowered,
  so that their vector is made once. The constraints of
  the types are kept once in `Types`, and a `TypeNode` holds no more of
  them: `Types::constraints(id)` gives the `Constraints` of a type,
  which are its `size`, `range`, `big_range`, `alphabet`, `size_parts`
  and `range_parts`, in place of the `effective_*` fields of its node.
  The fields of the constraints of the type itself, `size`, `range`,
  `big_range`, `alphabet`, `size_parts` and `range_parts`, are no more,
  as nothing but loading looks at them. `TypeNode::inner` is a
  `Box<Inner>`, the `Members` of `Kind::Sequence`, `Kind::Set`,
  `Kind::Choice` and `Kind::External` are a `Box<Members>`, and
  `MemberInfo::default` is a `Box<Value>`. `MemberInfo::size` is no
  more, as the type of the member has it.

### Removed

- Signing, encryption and nonces in `.asni` files, their dependencies,
  Rust features and Python extras. Cryptographic protection moves to the
  firmware update and patching project. Binding remains a label check,
  not authentication.
- Nonce, key and signature options in the CLI and WASM packages. Old
  options fail explicitly instead of being ignored. The C API removes
  the corresponding setters and `ASNKIT_KEY_*` constants; Rust removes
  the corresponding check types and error variants.

### Fixed

- `asnkitrt`: a specification in which fields of object classes hold
  each other, as `&x` holding `C.&y` and `&y` holding `C.&x`, or a field
  that is itself, used up the stack while it was loaded, which aborted
  the program. Types are now lowered at most 128 deep, and loading such
  a specification is an error.

## [2.0.0-rc3] - 2026-10-26

### Added

- `asnkit-gen-rust` and `asnkit-gen-rust-codec`: the named numbers of an
  `INTEGER` are constants, of the struct or the enum it is a member of
  (`OneAsymmetricKey::VERSION_V1`) or in the module of a type of its own
  (`version::V1`), and the same in both plugins (#4).
- `asnkitrt`: `serde_value::BitString` has `bit()`, `set_bit()`,
  `trimmed_len()`, `trim()` and `has_bits_other_than()`, and
  `wire::BitString` has `named()` and `has_bits_other_than()`, with
  `wire::named_bits()`, which makes the bytes of some bits in a
  constant.

### Changed

- `asnkit-gen-rust` and `asnkit-gen-rust-codec`: a `BIT STRING` with
  named bits is a newtype of `BitString` (or of `wire::BitString<'a>`)
  with a constant for each bit, `NAMED`, `get()`, `has_unknown_bits()`
  and a method that reads each bit, where it was a `BitString` that
  knew no names. `asnkit-gen-rust` adds `empty()`, `set()` and a setter
  for each bit, and `asnkit-gen-rust-codec` `bytes()` and
  `from_bytes()`, which make a value without allocating. Code that used
  the `BitString` uses `.0` of the newtype (#4).

## [2.0.0-rc2] - 2026-10-26

### Fixed

- `GeneralizedTime`, in every runtime: a time read from the form DER,
  CER, PER and UPER have is in UTC, with an offset of zero, where it was
  read as a local time, with none. Its `Z` is so kept when it is written
  in GSER, JER or XER, and a time in UTC that is encoded and decoded is
  equal to what it was. A lenient DER reader gives an offset of zero too
  to a time in another form, which it reads as the same time in UTC (#2).
- `asnkitcc`: `-h`, `--help` and `help`, also after a command, write the
  usage to standard output and succeed, where they were an unknown
  command or option. `-V`, `--version` and `version` tell the version
  of `asnkitcc` and those of the plugins found where programs are. The
  usage tells `convert` apart from `compile` and `parse` (#3).

## [2.0.0-rc1] - 2026-10-24

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
- `asnkit-gen-rust-codec`, a plugin that makes Rust code that encodes
  and decodes the types of a specification with BER, DER and UPER
  without loading it, for embedded targets: a
  value is encoded into a buffer that is given, and a value that is
  decoded borrows the data. These operations avoid heap allocation
  except for boxed recursive values; `to_*` convenience methods allocate.
  Select one codec with `--plugin-opt codec=ber|der|uper` (default
  `ber`); only that codec's methods and helpers are generated.
  The earlier output with all three codecs for RFC 5280 used 50.8 KiB
  of flash on a Cortex-M4 and no heap, where the runtime with
  `rfc5280.asni` is 210.7 KiB and keeps 398.5 KiB of heap. It does not
  support REAL, EXTERNAL, DATE, TIME-OF-DAY, DATE-TIME, DURATION or the
  table of an `ANY DEFINED BY` yet.
- `asnkitrt::wire`, what that code is built on: the views `Octets`,
  `BitString`, `Text`, `Oid`, `Integer` and `SequenceOf`, which borrow
  what is decoded, the readers and writers of BER and UPER of
  `wire::ber` and `wire::per`, which do not allocate, and the trait
  `Asn1`. The interpreter of PER and UPER uses the same numbers, sizes
  and strings, and `asnkitrt::per::BitSink` is what both write with.
- `Error::BufferTooSmall` in the Rust runtime, with the size an
  encoding needs, and `time::write_utc_time()` and
  `time::write_generalized_time()`, which write to a `fmt::Write`.
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
  `asnkitcc`, is faster, with the same bytes encoded and decoded.
  Compared with 1.2.0, an X.509 certificate of 893 bytes is decoded
  3.1 to 3.7 times as fast in BER, DER, OER, PER and UPER (DER from
  5.5 µs to 1.7 µs), with 42 allocations instead of 155, and encoded
  2.6 to 7.8 times as fast (DER from 12.7 µs to 1.6 µs). A small
  SEQUENCE is decoded 1.5 to 2.5 times as fast. It no longer copies the
  encoding of each value of BER and DER into the one around it, reads
  and writes the bits of PER and UPER a byte at a time and not a bit at
  a time, converts the time of a UTCTime or a GeneralizedTime to UTC
  without counting the years from year 1, borrows the names, texts,
  bytes and object identifiers of a decoded value, and looks a member
  up in the order of the type when encoding.
- PER and UPER in the Rust runtime encode a small SEQUENCE a third
  faster, and an X.509 certificate 8 to 13% faster, as they write the
  bits of an OCTET STRING, a BIT STRING and a string at once, with the
  same bytes.
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

[2.0.0-rc5]: https://github.com/boxtheta/asnkit/releases/tag/v2.0.0-rc5
[2.0.0-rc4]: https://github.com/boxtheta/asnkit/releases/tag/v2.0.0-rc4
[2.0.0-rc3]: https://github.com/boxtheta/asnkit/releases/tag/v2.0.0-rc3
[2.0.0-rc2]: https://github.com/boxtheta/asnkit/releases/tag/v2.0.0-rc2
[2.0.0-rc1]: https://github.com/boxtheta/asnkit/releases/tag/v2.0.0-rc1
[1.2.0]: https://github.com/boxtheta/asnkit/releases/tag/v1.2.0
[1.1.0]: https://github.com/boxtheta/asnkit/releases/tag/v1.1.0
[1.0.1]: https://github.com/boxtheta/asnkit/releases/tag/v1.0.1
[1.0.0]: https://crates.io/crates/asnkitrt/1.0.0
