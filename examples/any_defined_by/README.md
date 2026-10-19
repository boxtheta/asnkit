# ANY DEFINED BY mappings

These examples compile selector-to-type mappings from two TOML files into
one schema. They cover all supported selector kinds and both TOML table
styles. Run the following commands **from the repository root**, with a
compiler built from this checkout:

```sh
cargo build -p asnkitcc
mkdir -p target/any-defined-by-example
target/debug/asnkitcc compile \
  --any-defined-by examples/any_defined_by/messages.toml \
  --any-defined-by examples/any_defined_by/batch.toml \
  -o target/any-defined-by-example/messages.asni \
  examples/any_defined_by/payloads.asn examples/any_defined_by/messages.asn
```

An installed `asnkitcc` supporting `--any-defined-by` can replace
`target/debug/asnkitcc` in these commands.

| Type | Input | Mapping demonstrated |
|---|---|---|
| `Messages.Message` | `message.json` | INTEGER named numbers and decimal keys; string payload. |
| `Messages.Event` | `event.json` | ENUMERATED item names and an imported structured payload type. |
| `Messages.Flagged` | `flagged.json` | BOOLEAN selectors and a NULL payload. |
| `Messages.Batch` | `batch.json` | OIDs by imported value, qualified value, and dotted decimal; nested collection path and an explicit tag. |

## Encode and decode

Encode the message as DER, then decode it back to JER:

```sh
target/debug/asnkitcc convert -t Messages.Message --from jer --to der \
  -i examples/any_defined_by/message.json \
  -o target/any-defined-by-example/message.der \
  target/any-defined-by-example/messages.asni

target/debug/asnkitcc convert -t Messages.Message --from der --to jer --indent 2 \
  -i target/any-defined-by-example/message.der \
  target/any-defined-by-example/messages.asni
```

The DER bytes are `300a0201010c0568656c6c6f`. The decoded value is
`{"kind":1,"payload":"hello"}`. The payload is an ordinary string,
not the bytes of a separately encoded UTF8String.

To exercise the other fixtures, use the type and input filename from the
table. For example, transcode the nested batch to GSER:

```sh
target/debug/asnkitcc convert -t Messages.Batch --from jer --to gser --indent 2 \
  -i examples/any_defined_by/batch.json \
  target/any-defined-by-example/messages.asni
```

The JSON fixtures also round-trip with BER, OER, PER, UPER, XER and GSER
through the Rust-based CLI. The payloads selected by an OID are a string,
a Point, and NULL respectively. `entries.parameters` in `batch.toml`
skips over the collection element; it does not use indexes or `*`.

## A known selector with raw bytes

`raw.der.hex` contains a Message with `kind = 3` and a payload consisting
of a complete DER NULL encoding (`05 00`). That selector is explicitly
mapped to `OpenType`. Preserve it as DER with:

```sh
target/debug/asnkitcc convert -t Messages.Message --from der --to der \
  --input-hex --output-hex -i examples/any_defined_by/raw.der.hex \
  target/any-defined-by-example/messages.asni
```

Expected output: `30050201030500`. JER and XER cannot represent this
unresolved raw payload, so converting it to either is an error. Changing
`kind` to an unmapped value such as 4 is also an error; there is no
implicit fallback.

## Generate models with the mappings

With `asnkit-gen-python` or `asnkit-gen-rust` installed on `PATH`:

```sh
target/debug/asnkitcc generate --plugin python --plugin-opt name=messages \
  --any-defined-by examples/any_defined_by/messages.toml \
  --any-defined-by examples/any_defined_by/batch.toml \
  -o target/any-defined-by-example/python \
  examples/any_defined_by/payloads.asn examples/any_defined_by/messages.asn
```

Replace `python` with `rust` and choose another output directory for Rust
models. The models embed the mapped schema; the ANY DEFINED BY members
remain dynamic values. For Python, `Message(kind=1, payload="hello")`
encodes with `.encode("der")`.

See the [TOML format reference](../../docs/any_defined_by.md#toml-format)
for name lookup, duplicate detection, unsupported defaults, and runtime
constraint-checking limitations. These OIDs are illustrative identifiers
for this example, not a registry of real algorithms.
