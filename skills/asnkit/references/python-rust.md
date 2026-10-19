# Python and Rust runtime integration

Use the `foo.asn`/`foo.asni` fixture from [compiler.md](compiler.md).
Select the codec when loading, and reuse the loaded specification.

## Python

Install `asnkitrt` in the application's environment. In this repository,
`.venv/bin/python` or `uv run python` uses the workspace packages.

```python
import asnkitrt

spec = asnkitrt.load_asni_file("foo.asni", "der")
value = {"id": 1, "question": "Is 1+1=3?"}
encoded = spec.encode("Question", value, check_constraints=True)
assert encoded.hex() == "300e0201011609497320312b313d333f"
assert spec.decode("Question", encoded, check_constraints=True) == value
```

For bytes already in memory, use `asnkitrt.load_asni(data, "der")`.
`load()` accepts the runtime's schema representation; it is not an ASN.1
source parser. There is no `asnkitrt.compile_files()` API.

The high-level `encode` and `decode` methods operate on unique type
names. For collisions select `spec.modules["Foo"]["Question"]` and use
that type object's `encode`/`decode`. Call its `check_types(value)` and
`check_constraints(value)` explicitly as appropriate; these lower-level
calls do not provide the high-level wrapper checks.

`encode` checks Python types by default, but constraints are opt-in.
Decoding constraints are also opt-in. Text codec encodings are still
bytes; use `.decode("utf-8")` for presentation. For GSER input use the
Rust-based runtime; the Python codec only encodes GSER.

## Rust

Add `asnkitrt` to the consuming crate. Default features include text
codecs and serde. This is a complete binary example:

```rust
use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let asni = std::fs::read("foo.asni")?;
    let spec = Specification::from_asni(
        &asni, &Checks::default(), Options::new(Codec::Der),
    )?;
    let value = Value::Map(vec![
        ("id".into(), Value::Integer(1)),
        ("question".into(), Value::text("Is 1+1=3?")),
    ]);
    spec.check_constraints("Question", &value)?;
    let encoded = spec.encode("Question", &value)?;
    let decoded = spec.decode("Question", &encoded)?;
    spec.check_constraints("Question", &decoded)?;
    assert_eq!(decoded, value);
    Ok(())
}
```

For ambiguous names use `encode_in("Foo", "Question", &value)`,
`decode_in("Foo", "Question", &bytes)` and `check_constraints_in`.
`encode_indented(name, &value, indent)` supports JER, XER and GSER.
`serde_value::to_value` and `from_value` convert typed data; use the
runtime's `OctetString`/`BitString` wrappers where ordinary serde
sequences would lose the ASN.1 distinction. Generated models handle
those mappings for you.

For embedded/no-std builds set `default-features = false`; the runtime
still needs `alloc`. Enable `jer`, `gser`, `serde`, `signing` or
`encryption` as required. `xer` and `bigint` pull in `std`. `bigint` is
needed for very large constraint bounds, not merely for an INTEGER value
larger than `i128`. Use `Value::BigInteger` for larger integer values;
its bytes are signed two's-complement, big-endian.

## Value representations

| ASN.1 | Python | Rust `Value` |
|---|---|---|
| BOOLEAN / NULL | `bool` / `None` | `Boolean` / `Null` |
| INTEGER / REAL | `int` / `float` | `Integer` or `BigInteger` / `Real` |
| OCTET STRING / raw ANY | `bytes` | `Bytes(Vec<u8>)` |
| BIT STRING | `(bytes, bit_count)` | `Bits(Vec<u8>, bit_count)` |
| OBJECT IDENTIFIER | dotted `str` | `Text`, e.g. `"1.2.840.113549"` |
| character strings | `str` | `Text` |
| ENUMERATED | name, or integer with `numeric_enums` | `Text`, or `Integer` with `numeric_enums` |
| SEQUENCE / SET | `dict` of present members | `Map(Vec<(String, Value)>)` |
| SEQUENCE OF / SET OF | `list` | `List(Vec<Value>)` |
| CHOICE | `(alternative_name, value)` | `Choice(String, Box<Value>)` |
| date/time types | `datetime` date/time objects | `Date`, `Time`, `DateTime` |

Omit absent OPTIONAL members; do not encode NULL in their place. A BIT
STRING's length is in bits, not bytes. Preserve exact integer values.
Defaults may be materialized during decoding, so a sparse input map and
the decoded map need not be literally identical.

For BER/DER framing use `decode_with_length` for a decoded value and
consumed byte count, or `decode_length` for the total message length when
it can be determined. Do not use these framing helpers for PER/UPER.

Sources in a checkout: `packages/asnkitrt/src/asnkitrt/runtime.py`,
`packages/asnkitrt-rs/src/runtime.rs`, `src/value.rs`, `src/serde_value.rs`
(the last two under `packages/asnkitrt-rs`), and the package READMEs.
