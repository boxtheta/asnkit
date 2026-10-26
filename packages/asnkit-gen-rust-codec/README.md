# asnkit-gen-rust-codec

A plugin for `asnkitcc` that generates Rust code that encodes and
decodes the types of an ASN.1 specification with one selected codec:
BER, DER or UPER. The
code is made for the specification, which it does not load: it needs no
`.asni` file. Buffer-based encoding and borrowed decoding avoid heap
allocation except for boxed recursive values. It uses
[`asnkitrt`](https://crates.io/crates/asnkitrt) for the rules of the
codecs, which are those of the runtime, and builds without `std`.

```text
$ cargo install asnkitcc asnkit-gen-rust-codec
$ asnkitcc generate --plugin rust-codec --plugin-opt codec=uper -o src foo.asn    # Writes src/foo.rs.
```

```rust
mod foo;

use foo::Question;

let question = Question { id: 1, question: "Is 1+1=3?".into() };
let mut buffer = [0u8; 64];
let length = question.encode_uper(&mut buffer)?;

// Borrows the text from the buffer: nothing is allocated.
let decoded = Question::decode_uper(&buffer[..length])?;

assert_eq!(decoded.question, "Is 1+1=3?");
```

The crate that uses the generated code needs `asnkitrt`, which can be
built with `default-features = false`. Generate code with the matching
runtime version; `asnkitrt::wire` may change between minor versions.

## Why

The runtime loads a specification and keeps it on the heap: RFC 5280,
the types of X.509, uses 398.5 KiB in a 32-bit WebAssembly heap probe, and
loading it takes 737 KiB at its peak. The code this plugin makes for
RFC 5280 keeps nothing on the heap, and a certificate decoded with it is
a value of 432 bytes in that probe that borrows the rest from the data.

Built for a Cortex-M4 (`thumbv7em-none-eabi`, `opt-level = "z"`, LTO),
the earlier generator that emitted all three codecs produced the
comparison below for a program using DER and UPER. These are baseline
measurements, not measurements of the current single-codec output. Both
flash probes use `default-features = false`,
`codegen-units = 1`, `panic = "abort"` and `strip = true`; flash counts
`.text`, `.rodata`, `.data` and `.ARM.exidx`. Heap figures are from the
WebAssembly probe, not a Cortex-M4 execution, and exclude application
buffers and stack usage.

| | Flash | Heap |
|---|---:|---:|
| The runtime, with `rfc5280.asni` in it | 210.7 KiB | 737 KiB at its peak, 398.5 KiB kept |
| The code of this plugin for RFC 5280 | 50.8 KiB | none |

The runtime does more: it encodes and decodes any specification it is
given, with all of its codecs, and checks constraints.

## Generated-code benchmarks

Run from the repository root:

```sh
cargo bench -p asnkit-gen-rust-codec --bench codecs
```

The benchmark uses the current separate BER, DER and UPER generated RFC
5280 modules and all 37 DER certificates in `asnkitrt-rs/tests/fixtures/x509`.
The interpreter converts the same certificates to each wire format during
setup. Every generated decode/length/encode result is checked against that
input before timing. Setup, file access, allocation checks and output-buffer
allocation are outside the timed loops.

Measured on Apple M5, `aarch64-apple-darwin`, Rust 1.99.1, using Cargo's
standard optimized bench profile (`opt-level = 3`, no LTO). Times are
microseconds per certificate, with equal weight per fixture, taking the
median of seven batches after at least 30 ms of calibration/warm-up per
operation. These are local throughput measurements, not latency guarantees.

| Selected codec | Decode | Encoded length | Buffer encode | Decode + buffer encode |
|---|---:|---:|---:|---:|
| BER | 0.737 µs | 1.148 µs | 1.515 µs | 2.131 µs |
| DER | 0.741 µs | 1.161 µs | 1.530 µs | 2.165 µs |
| UPER | 0.757 µs | 3.103 µs | 3.642 µs | 4.516 µs |

Encoding and length timings use values decoded before timing. Decoding
returns borrowing views, including lazy `SequenceOf` collections; it does
not materialize an owned tree or visit every element through an application
iterator. Round-trip timing calls decode then encode, without a separate
length call. The harness uses `black_box` on inputs and results.

| Selected codec | Decoded value, inline | Encoded input / required output | Decode + length + buffer encode heap | `to_*()` heap |
|---|---:|---:|---:|---:|
| BER | 768 B | 370–1,101 B each | 0 allocations, 0 B peak | 1 allocation, 370–1,101 B |
| DER | 768 B | 370–1,101 B each | 0 allocations, 0 B peak | 1 allocation, 370–1,101 B |
| UPER | 768 B | 313–1,026 B each | 0 allocations, 0 B peak | 1 allocation, 313–1,026 B |

Heap counts cover all 37 certificates. `to_*()` figures are per certificate;
requested bytes and peak live bytes are equal here and exclude allocator
metadata. The 768 B figure is `size_of::<Certificate>()` on this 64-bit target,
not peak stack usage. The decoded value borrows the input, which must stay
alive. With distinct input and exactly sized output buffers, their combined
storage plus the inline value is 1,508–2,970 B for BER/DER and 1,394–2,820 B
for UPER. These sums exclude call-stack temporaries, allocator overhead and
harness storage; they are not total process RAM or measured peak stack.
The earlier 432 B WebAssembly result above used a different target.

## Encoding and decoding

Each named struct and enum has inherent methods for the selected codec.
For example, `codec=der` generates:

- `encode_der(&mut buffer)`, which returns the number of bytes written,
  or `Error::BufferTooSmall` with the required size.
- `der_len()`, which returns the required buffer size.
- `to_der()`, an allocating convenience method returning a `Vec`.
- `decode_der(&data)`, which checks DER and returns a value borrowing
  the input.

BER and UPER use the corresponding `_ber` and `_uper` names. Methods
and helpers for the other codecs are omitted; generated types no longer
implement the runtime's all-codec `Asn1` trait. To use several codecs,
generate a separate module for each using the `name` option. These
modules define separate Rust types.

A named type that is an alias, such as `Name ::= UTF8String`, has a
module with encoding and decoding functions for the selected codec,
for example `name::encode_der(&value, &mut buffer)` and
`name::decode_der(&data)`.

To avoid allocations, use the buffer-based `encode_*` methods and
borrowed views. Recursive values stored in a `Box` still allocate when
constructed or decoded. The generated code does not attach allocating
field-location paths to errors. `no_std` alone does not mean that an
application needs no allocator: boxed values and the `to_*` methods
still require one.

## Type mapping

| ASN.1 | Rust |
|---|---|
| `BOOLEAN`, `NULL` | `bool`, `()` |
| `INTEGER` | the smallest integer type of its bounds; with none, `i64` or as `integer` says |
| `ENUMERATED` | an enum of unit variants, with `number()` and `from_number()` |
| `OCTET STRING`, `ANY`, open types | `wire::Octets<'a>` |
| `BIT STRING` | `wire::BitString<'a>` |
| `BIT STRING` with named bits | a newtype of `wire::BitString<'a>` that knows the names (see below) |
| `OBJECT IDENTIFIER` | `wire::Oid<'a>` |
| strings | `wire::Text<'a>`, with `chars()`, `as_str()` and `Display` |
| `UTCTime`, `GeneralizedTime` | `wire::DateTime` |
| `SEQUENCE`, `SET` | a struct; `OPTIONAL` members and members after an extension marker are `Option`, and a member with a `DEFAULT` has its default when it is absent, or is an `Option` if its default is of a type that is not one of those above it |
| `CHOICE` | an enum with a variant for each alternative |
| `SEQUENCE OF`, `SET OF` | `wire::SequenceOf<'a, T>` |

A value that is decoded borrows the data it is decoded from, and one
that is encoded borrows what it is made of: `"text".into()` is a
`Text`, `(&bytes[..]).into()` an `Octets`, and `(&items[..]).into()` a
`SequenceOf`. A `SequenceOf` that is decoded holds where its elements
are, which are decoded again as they are iterated, so that it needs no
memory of its own: `iter()` gives them, and `len()` tells how many
there are.

Names are made as `asnkit-gen-rust` makes them: `fooBar` is `foo_bar`,
`Foo-Bar` is `FooBar`, a type with no name is named after the type that
holds it and the member, and a name that is taken gets a number.

A `BIT STRING` with named bits has the constants of the bits and
`NAMED`, `get()`, `has_unknown_bits()` and a method that reads each
bit by name, as with `asnkit-gen-rust`, so that the constants are the
same in both. As it borrows its bytes, it is made from them:
`bytes()` gives the `BYTES` bytes with some bits set, without
allocating and also in a constant, and `from_bytes()` makes the value
of them, without the zero bits at the end, which DER leaves out:

```rust
static FLAGS: [u8; CapsuleFlags::BYTES] =
    CapsuleFlags::bytes(&[CapsuleFlags::KEY_SET, CapsuleFlags::PATCH]);

let flags = CapsuleFlags::from_bytes(&FLAGS);
```

The named numbers of an `INTEGER` are constants as with
`asnkit-gen-rust`: of the struct or the enum for a member, and in the
module of the functions of a type of its own, as `version::V1`.

## Options

Options are given with `--plugin-opt KEY=VALUE`:

`codec`
: The single codec to generate: `ber` (default), `der` or `uper`.
  Other values are rejected. This is independent of `asnkitcc -e`,
  which selects the input ASN.1 source text encoding.

`name`
: The name of the module, without `.rs`. Default: the name of the first
  ASN.1 file.

`no_std`
: `true` for code that uses `alloc` and not `std`, for the `Box` of a
  type that holds itself. The crate then needs `extern crate alloc;`.
  Default: `false`.

`integer`
: The Rust type of an `INTEGER` with no bounds: `i64` (default),
  `i128`, or `big` for `asnkitrt::wire::Integer`, which holds any value
  and borrows those of more than 128 bits, as the serial numbers of
  certificates are. Decoding a value that does not fit returns an error;
  it is never truncated.

`unsupported`
: `skip` to leave out the types that are not supported yet, and those
  that hold them, which the module lists in a comment. Default: `error`,
  which fails.

## Not supported yet

- `REAL`, `EXTERNAL`, `DATE`, `TIME-OF-DAY`, `DATE-TIME` and `DURATION`.
- The table of an `ANY DEFINED BY`: its value is any value, as an
  `ANY` is.
- Aligned PER and OER, JER, XER and GSER.
- In UPER, an open type of 16384 bytes or more, and a `BIT STRING` of
  16384 bits or more without bounds, which return
  `Error::Unsupported`. An `INTEGER` beyond 128 bits with a lower bound
  that is not 0 does too.
- Checking constraints: UPER checks those it encodes with, as the bounds
  of an `INTEGER` and the size of a string, and BER and DER none.

Where it differs from the runtime, on purpose:

- A value of an extensible `ENUMERATED` or an alternative of an
  extensible `CHOICE` that the specification does not know is an error,
  where the runtime decodes it as nothing.
- A `SEQUENCE` that is not extensible and has more in it than its members
  is an error in BER and DER, where the runtime skips what is after
  them.

## How it is tested

`tests/generated.rs` builds the code of RFC 5280 and of other shapes of
types, and checks that what it encodes the runtime decodes and encodes
the same, that the certificates of `asnkitrt` come back as they were,
and that decoding and encoding them allocates nothing.
`tests/vectors` generates a separate module for each supported codec
of each specification, and checks it against the repository's BER, DER
and UPER vectors:

```text
cargo test --release --manifest-path packages/asnkit-gen-rust-codec/tests/vectors/Cargo.toml
```

## License

Apache License 2.0 (`LICENSE-COMPILER`). The generated code, with the
helper code copied into it, is yours to use under terms of your choice,
as the header of each file it writes says.
