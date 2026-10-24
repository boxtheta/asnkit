# asnkitrt (Rust)

The asnkit runtime for Rust. It loads the `.asni` files produced by
`asnkitcc` and uses them to encode and decode ASN.1. It contains no
parser, needs `alloc` but not `std`, and builds for embedded targets.

The codecs are BER, DER, OER, PER and UPER, plus JER (feature `jer`),
XER (feature `xer`) and GSER (feature `gser`). It can also check values
against their constraints. GSER decoding accepts bare values and the value
assignments produced by the encoder. It supports the generic value forms;
LDAP variant encodings and symbolic object identifier descriptors are not
supported. Unknown SEQUENCE and SET members are rejected.

```rust
use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

let data = std::fs::read("foo.asni")?;
let foo = Specification::from_asni(&data, &Checks::default(), Options::new(Codec::Der))?;
let encoded = foo.encode("Question", &value)?;
let decoded = foo.decode("Question", &encoded)?;
```

- **Values** are `Value`s, matching the Python runtime: a `Map` for a
  `SEQUENCE`, a `Choice` for a `CHOICE`, and so on. An `OBJECT
  IDENTIFIER` is an `ObjectIdentifier`, which is text to the Python
  runtime. With the `serde` feature, `to_value` and `from_value`
  convert your own types to and from values, and `asnkit-gen-rust`
  generates those types from ASN.1.
- **Decoded values borrow**: a `Value<'a>` borrows the names of its
  members from the specification, and its texts, bytes and object
  identifiers from the data where the encoding is the value, so that
  decoding allocates little more than its lists and maps.
  `Value::into_owned()` or `Specification::decode_owned()` give a
  `Value<'static>` to keep past them. `from_value` can borrow too, into
  a `&'a str` or a `&'a [u8]`.
- **Protected files** (signed, encrypted, bound or with a nonce) only
  load when the caller says what to expect; see `container::Checks`.
- **Untrusted input** never causes a panic: decoding returns a value or
  an error, in time and memory proportional to the input, with nesting
  limited. This applies to JSON and XML too. XML with a document type
  declaration is rejected, so there are no entities to expand.
- **Embedded targets** build with `default-features = false`, adding
  `signing` and `encryption` if they receive signed or encrypted files.
  CI builds it for `thumbv7em-none-eabi`.

## Features

| Feature | Purpose | Default |
|---|---|---|
| `std` | `std::error::Error` and similar | yes |
| `signing`, `encryption` | Loading signed or encrypted `.asni` files | yes |
| `serde` | `serde_value::{to_value, from_value}` and related types | yes |
| `jer` | JSON Encoding Rules, with no extra dependency | yes |
| `xer` | XML Encoding Rules, using `quick-xml`; needs `std` | yes |
| `gser` | Generic String Encoding Rules, with no extra dependency | yes |
| `bigint` | Bounds of constraints of more than 128 bits, and `DURATION` time elements of more than 128 bits in PER and OER, using `num-bigint`; needs `std` | yes |

Requesting a codec that is not compiled in returns
`Error::Unsupported`.

## Sizes

BER, DER, OER, PER and UPER are always compiled in, so the smallest
build is the core alone. SLOC are the lines of the crate's own code
that are compiled in, without comments, blank lines or tests; the
dependencies are not counted. The sizes are those of a program that
loads a `.asni` file and decodes, checks and encodes a value with a
codec chosen at run time, so that every codec compiled in is linked,
built with `lto = true`, `codegen-units = 1`, `panic = "abort"` and
`strip = true`. These baseline measurements were taken with 1.2.0
and Rust 1.99, before the `wire` module and generated-code work below;
they are not measurements of the current source tree. KiB means 1024
bytes. Unless the row says `default`, default features are disabled.
The `Runtime` column subtracts the baseline executable size;
`Over the core` is the increase over the first row. Differences are
calculated before rounding.

On a computer (`aarch64-apple-darwin`, `opt-level = 3`, with `std`),
where the same program without the runtime is 295.5 KiB:

| Features | SLOC | Binary | Runtime | Over the core |
|---|---:|---:|---:|---:|
| `std` (the core) | 9,557 | 667.5 KiB | 372.0 KiB | |
| `std`, `jer` | 10,697 | 699.8 KiB | 404.3 KiB | +32.3 KiB |
| `std`, `gser` | 10,645 | 716.0 KiB | 420.4 KiB | +48.5 KiB |
| `std`, `xer` | 10,637 | 748.3 KiB | 452.7 KiB | +80.8 KiB |
| `std`, `jer`, `xer`, `gser` | 12,314 | 796.7 KiB | 501.2 KiB | +129.2 KiB |
| default | 13,751 | 861.3 KiB | 565.8 KiB | +193.9 KiB |

`xer` includes `quick-xml`, and the default features add `signing`,
`encryption` and `bigint` with their dependencies. `serde` adds lines
but nothing to a program that does not call `to_value` or
`from_value`.

On a device (`thumbv7em-none-eabi`, a Cortex-M4 or M7, `opt-level =
"z"`, without `std`), as flash (`.text`, `.rodata`, `.data` and
`.ARM.exidx`), where the same program without the runtime is 868 bytes:

| Features | SLOC | Flash | Over the core |
|---|---:|---:|---:|
| none (the core) | 9,553 | 202.6 KiB | |
| `jer` | 10,693 | 229.5 KiB | +26.9 KiB |
| `gser` | 10,641 | 232.3 KiB | +29.7 KiB |
| `jer`, `gser` | 11,513 | 243.1 KiB | +40.5 KiB |
| `signing`, `encryption` | 9,578 | 235.5 KiB | +32.9 KiB |
| `signing`, `encryption`, `jer`, `gser` | 11,538 | 276.1 KiB | +73.4 KiB |

`xer` and `bigint` need `std` and are not available there. These flash
figures cover the probe, not the rest of an application or its firmware.
The interpreter needs a global allocator, and RAM can be the tighter
limit: in these examples, a loaded specification occupies roughly 30 to
42 times its `.asni` size, with a higher peak during loading. The heap
measurements below are from a 64-bit computer; they are not device RAM
requirements:

| Specification | `.asni` | Kept | Peak while loading |
|---|---:|---:|---:|
| `examples/compiled/foo.asni` | 244 B | 9.2 KiB | 15.3 KiB |
| `examples/compiled/information_object.asni` | 2.1 KiB | 59.9 KiB | 126.4 KiB |
| `examples/compiled/rfc5280.asni` | 12.3 KiB | 519.4 KiB | 970.5 KiB |

Decoding, checking and encoding a certificate of RFC 5280 in DER takes
4 to 8 KiB more in that host measurement. Budget separately for the
input, output, stack and application state on the intended target.

For a fixed specification on a memory-constrained target,
[`asnkit-gen-rust-codec`](../asnkit-gen-rust-codec/README.md) generates
BER, DER and UPER codecs using `asnkitrt::wire`, without loading a
`.asni` file. Its buffer-based encoding and borrowed views avoid heap
allocation except for boxed recursive values; its `to_*` convenience
methods allocate. See that plugin for the RFC 5280 flash comparison
and the supported types and constraints.

## Errors

An error with a value, encoding, decoding or checking it, tells where
in the value it is, as `Message.tags[1]: Expected data of type str.`;
`Error::location()` gives the steps. So does an `Error::Unsupported`
found in a value, as an `ANY` in JER, but not one of the codec, as a
codec that is not compiled in.

## Limitations

Not supported: `ANY` in JER and XER, which need its type, and the
types `EMBEDDED PDV` and `CHARACTER STRING`. Without the feature
`bigint`, a specification with a bound of a constraint of more than 128
bits is not loaded, and a `DURATION` with a time element of more than
128 bits is not encoded in PER or OER; an `INTEGER` of any size is
encoded either way. All of these return an error; none produces a wrong
value.

It is the runtime of reference: the more than 5,300 vectors in
[`tests/vectors`](https://github.com/boxtheta/asnkit/blob/trunk/tests/vectors) are what it encodes and decodes,
including constraint checks, and the Python and WebAssembly runtimes
are held to them. `tests/x509.rs` checks it against OpenSSL.

Licensed under the MIT License. See `LICENSE-RUNTIMES`.
