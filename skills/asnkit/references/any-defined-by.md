# Configure ANY DEFINED BY

Use this workflow when the protocol specification supplies a mapping
from a sibling selector value to a payload type. Do not invent mappings
from example bytes; get them from the protocol or the user.

## Write the configuration

Identify the type assignment containing the ANY member, its member path,
and the member named after `DEFINED BY`. Verify that the selector is
available before the payload during decoding. Then write one entry per
target, using the exact ASN.1 names:

```toml
[[any_defined_by]]
type = "Messages.Message"
member = "payload"
choices = { "1" = "UTF8String", "2" = "INTEGER" }

[[any_defined_by]]
type = "Messages.Batch"
member = "entries.parameters"

[any_defined_by.choices]
id-text = "UTF8String"
"Payloads.id-point" = "Payloads.Point"
"1.3.6.1.4.1.99999.3" = "NULL"
```

This example matches `examples/any_defined_by/messages.asn` and
`payloads.asn` in an asnkit checkout. For another schema, adapt all names
and selector values to that protocol.

Important TOML rules:

- `[[any_defined_by]]` is an array-of-tables entry, not a single table.
  `type`, `member`, and nonempty `choices` are required; unknown fields fail.
- Put `type` and `member` before `[any_defined_by.choices]`. That section
  belongs to the latest entry, and subsequent fields stay in it.
- Quote an entire dotted key: an OID or `Module.value` without quotes is
  a nested TOML table. Choice values are always strings naming types.
- Use either an inline `choices = { ... }` or a choices section per entry.
- Repeated files append mappings, with no merging or override semantics.
  A `(type, member)` target must occur only once across all files.

## Resolve names and paths

| Selector type | Supported keys |
|---|---|
| INTEGER | Decimal integers within signed 128 bits, named numbers, assigned INTEGER values. |
| ENUMERATED | Item names or assigned values of that enumeration; runtime numeric mode is supported too. |
| OBJECT IDENTIFIER | Quoted dotted decimal, or local/imported/qualified assigned OID values. |
| BOOLEAN | `TRUE` and `FALSE`. |

Named numbers and enumeration items come from the selector type.
Unqualified assigned values and choice types are resolved in the module
of `type`, including imports; use `Module.name` to qualify them.
Different keys resolving to one selector value are an error.

The target `type` is always qualified as `Module.Type`. `member` enters
inline fields with dots and collection elements without a name, index,
`[]` or `*`. For `entries SEQUENCE OF SEQUENCE { ... parameters ANY
DEFINED BY algorithm }`, use `entries.parameters`. If a path crosses a
named reference, configure that referenced type at its own assignment.

Choice types can be built-ins such as `NULL`, `INTEGER`, `UTF8String` or
`OCTET STRING`, or assigned types such as `Payloads.Point`. Define or
instantiate structured/parameterized types in ASN.1 first. `OpenType`
means raw encoded bytes for that particular selector; it is not a catchall.
Do not emit `default`, wildcard selectors, or runtime mapping overrides:
those features are unsupported.

## Compile and verify

In a checkout, the complete fixtures and commands live in
`examples/any_defined_by/README.md`. From the repository root:

```sh
mkdir -p target/any-defined-by-example
cargo run -p asnkitcc -- compile \
  --any-defined-by examples/any_defined_by/messages.toml \
  --any-defined-by examples/any_defined_by/batch.toml \
  -o target/any-defined-by-example/messages.asni \
  examples/any_defined_by/payloads.asn examples/any_defined_by/messages.asn
```

Use the fixture JER messages to round-trip the selected payloads. The
`Message` fixture's DER is `300a0201010c0568656c6c6f`; the raw DER fixture
is `30050201030500`. An unknown selector should fail, and the raw fixture
should fail conversion to JER/XER. In an application, choose representative
values for each actual mapping and verify its required wire codec.

The flag is accepted only by `compile` and `generate`. Pass the mapped
`.asni` to `convert`; passing ASN.1 source again would compile without the
mappings. Pass the same TOML options to `generate` so the embedded schema
contains them. Generated ANY members remain dynamic payload values.

For the Rust compiler library, parse with `CompileOptions::from_toml`
and pass options to `compile_to_asni_with_options`. To combine files,
append their `any_defined_by` vectors before compiling. The JavaScript
compiler wrapper does not expose these options yet.

The existing runtime constraint walkers do not dispatch through ANY
DEFINED BY. Check the selected type explicitly when payload constraint
validation is required. Raw bytes remain codec-specific; avoid blind
transcoding of `OpenType` payloads.

The full maintained format reference is `docs/any_defined_by.md` in the
checkout. Keep mappings and examples consistent with the compiler version
actually used by the application.
