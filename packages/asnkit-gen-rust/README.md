# asnkit-gen-rust

A plugin for `asnkitcc` that generates Rust code from ASN.1. The
generated module uses [`asnkitrt`](https://crates.io/crates/asnkitrt) and
[`serde`](https://serde.rs), and, by default, embeds the compiled specification as
bytes. It has a struct for each `SEQUENCE` and `SET`, an enum for each
`ENUMERATED` and `CHOICE`, and methods to encode and decode them.

```text
$ cargo install asnkitcc asnkit-gen-rust
$ asnkitcc generate --plugin rust -o src foo.asn    # Writes src/foo.rs.
```

```rust
mod foo;

use asnkitrt::types::Codec;
use foo::{Asn1Type, Question};

let specification = foo::load(Codec::Uper)?;
let encoded = Question { id: 1, question: "Is 1+1=3?".into() }.encode(&specification)?;

assert_eq!(Question::decode(&specification, &encoded)?.id, 1);
```

The crate that uses the generated code needs `asnkitrt`, and `serde`
with its `derive` feature.

Options, given with `--plugin-opt KEY=VALUE`:

`name`
: The name of the module, without `.rs`. Default: the name of the first
  ASN.1 file.

`no_std`
: `true` to generate code that needs only `alloc`, not `std`. The crate
  then needs `extern crate alloc;`, and uses `asnkitrt` and `serde` with
  `default-features = false`. Default: `false`.

`integer`
: The Rust type of an `INTEGER` without bounds: `i64` (default),
  `i128`, or `big` for `asnkitrt::serde_value::BigInt`, which holds any
  value. An `INTEGER` with bounds gets the smallest Rust integer type
  that fits them. Decoding a value that does not fit returns an error;
  it is never truncated.

`embed`
: `false` to leave the compiled specification out of the module, as for
  one sent signed or encrypted. The module then has no `load()` and
  `load_with()`: load the `.asni` file with
  `asnkitrt::runtime::Specification::from_asni()`, with the
  `asnkitrt::container::Checks` it needs, and give it to the methods as
  any other. The file has to be compiled from the same specification.
  Default: `true`.

## Type mapping

| ASN.1 | Rust |
|---|---|
| `BOOLEAN`, `NULL`, `REAL` | `bool`, `()`, `f64` (`f32` if constrained to IEEE 754 binary32) |
| `OCTET STRING`, `BIT STRING` | `asnkitrt::serde_value::OctetString`, `BitString` |
| `OBJECT IDENTIFIER` | `asnkitrt::value::ObjectIdentifier<'static>` |
| strings, `DURATION` | `String` |
| `UTCTime`, `GeneralizedTime`, `DATE-TIME` | `asnkitrt::value::DateTime` |
| `DATE`, `TIME-OF-DAY` | `asnkitrt::value::Date`, `Time` |
| `SEQUENCE`, `SET` | a struct; `OPTIONAL` and `DEFAULT` members, and members after an extension marker, are `Option` |
| `CHOICE` | an enum with a variant for each alternative |
| `ENUMERATED` | an enum of unit variants |
| `SEQUENCE OF`, `SET OF` | `Vec` |
| `ANY`, open types, `EXTERNAL` | `asnkitrt::value::Value` |

An anonymous inner type is named after the type that contains it and the
member. A type that contains itself, directly or through other types, is
wrapped in a `Box`. Names are converted to Rust style (`fooBar` becomes
`foo_bar`, `Foo-Bar` becomes `FooBar`), with `#[serde(rename)]` keeping
the ASN.1 name. Names that collide after conversion get a number
appended.

Decoding a value of an extensible `ENUMERATED` that the specification
does not know returns an error.

## License

Apache License 2.0 (`LICENSE-COMPILER`). The generated code, with the
helper code copied into it, is yours to use under terms of your choice,
as the header of each file it writes says.
