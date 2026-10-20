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
