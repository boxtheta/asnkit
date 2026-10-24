# asnkit-gen-rust-codec

A plugin for `asnkitcc` that generates Rust code that encodes and
decodes the types of an ASN.1 specification with BER, DER and UPER. The
code is made for the specification, which it does not load: it needs no
`.asni` file. Buffer-based encoding and borrowed decoding avoid heap
allocation except for boxed recursive values. It uses
[`asnkitrt`](https://crates.io/crates/asnkitrt) for the rules of the
codecs, which are those of the runtime, and builds without `std`.

The plugin is not yet published to crates.io. From this repository,
install the compiler and plugin with:

```text
$ cargo install --path packages/asnkitcc
$ cargo install --path packages/asnkit-gen-rust-codec
$ asnkitcc generate --plugin rust-codec -o src foo.asn    # Writes src/foo.rs.
```

```rust
mod foo;

use asnkitrt::wire::Asn1;
use foo::Question;

let question = Question { id: 1, question: "Is 1+1=3?".into() };
let mut buffer = [0u8; 64];
let length = question.encode_uper(&mut buffer)?;

// Borrows the text from the buffer: nothing is allocated.
let decoded = Question::decode_uper(&buffer[..length])?;

assert_eq!(decoded.question, "Is 1+1=3?");
```

The crate that uses the generated code needs `asnkitrt`, which can be
built with `default-features = false`. Until the plugin and its runtime
support are released, use `packages/asnkitrt-rs` from this checkout as
a path dependency. Generate code with the matching runtime version;
`asnkitrt::wire` may change between minor versions.

## Why

The runtime loads a specification and keeps it on the heap: RFC 5280,
the types of X.509, uses 398.5 KiB in a 32-bit WebAssembly heap probe, and
loading it takes 737 KiB at its peak. The code this plugin makes for
RFC 5280 keeps nothing on the heap, and a certificate decoded with it is
a value of 432 bytes in that probe that borrows the rest from the data.

Built for a Cortex-M4 (`thumbv7em-none-eabi`, `opt-level = "z"`, LTO),
a program that decodes a certificate and encodes it again in DER or in
UPER is shown below. Both flash probes use `default-features = false`,
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

## Encoding and decoding

Each struct and enum of a named type implements `asnkitrt::wire::Asn1`:

- `encode_ber()`, `encode_der()` and `encode_uper()` write the value
  into a buffer that is given, and return how many bytes it is. If the
  buffer is too small they return `Error::BufferTooSmall` with the size
  it needs, which `ber_len()`, `der_len()` and `uper_len()` also tell.
  `to_ber()`, `to_der()` and `to_uper()` are allocating convenience
  methods that return a `Vec`; there is no separate `alloc` feature.
- `decode_ber()`, `decode_der()` and `decode_uper()` decode a value that
  borrows the data. `decode_der()` checks that the data is DER.

A named type that is not a struct or an enum of its own, as
`Name ::= UTF8String`, is a `type` with a module of the same functions:
`name::encode_der(&value, &mut buffer)`.

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

## Options

Options are given with `--plugin-opt KEY=VALUE`:

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
`tests/vectors` holds the code of each specification of
`tests/vectors` of the repository to the vectors of BER, DER and UPER:

```text
cargo test --release --manifest-path packages/asnkit-gen-rust-codec/tests/vectors/Cargo.toml
```

## License

Apache License 2.0 (`LICENSE-COMPILER`). The generated code, with the
helper code copied into it, is yours to use under terms of your choice,
as the header of each file it writes says.
