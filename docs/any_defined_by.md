# ANY DEFINED BY

> **Note:** This is a draft. `asnkitcc` and the runtimes do not read
> these choices yet.

An `ANY DEFINED BY` member has the type that the value of another member
of the same `SEQUENCE` or `SET` says, but the ASN.1 does not say which
type goes with which value. That is in the text of the standard that
uses it. In RFC 5280, for example, the `parameters` of an
`AlgorithmIdentifier` are a `NULL` for RSA, and an `ECParameters` for
elliptic curves.

Without more, an `ANY DEFINED BY` is an `ANY`: its value is the bytes
of its encoding (see [File formats](file_formats.md#built-in-types)).
Telling which types it can hold makes its value one of those types.
This is done when compiling, with a TOML file, or when loading, with
the same choices given to the runtime.

## The TOML file

```text
$ asnkitcc compile --any-defined-by anys.toml rfc5280.asn rfc3279.asn
```

`--any-defined-by` can be given more than once. It is also taken by
`asnkitcc generate`, so that the generated code has the types.

```toml
[[any_defined_by]]
type = "PKIX1Explicit88.AlgorithmIdentifier"
member = "parameters"
default = "OpenType"

[any_defined_by.choices]
"1.2.840.113549.1.1.1" = "NULL"
"PKIX1Algorithms88.id-ecPublicKey" = "PKIX1Algorithms88.ECParameters"
```

Each `[[any_defined_by]]` is one `ANY DEFINED BY` member. Its keys:

`type`
: The type that has the member, as `Module.Type`. Required.

`member`
: The name of the `ANY DEFINED BY` member. When it is in a type
  written in place in `type`, the names of the members down to it,
  separated by `.`, as `content.parameters`. The items of a `SEQUENCE
  OF` or a `SET OF` are gone into with no name. Required.

`choices`
: A table from each value of the member that defines the type, the
  one after `DEFINED BY`, to the type the `ANY DEFINED BY` has when
  the member has that value. Required, and may be empty when there is
  a `default`.

`default`
: The type for a value that is not in `choices`. Without it, such a
  value is rejected when encoding and decoding. Optional.

A key of `choices` is the value as ASN.1 writes it, in quotes when
TOML needs it:

| Type of the member | Key |
|---|---|
| OBJECT IDENTIFIER | `"1.2.840.113549.1.1.1"`, with the numbers only |
| INTEGER | `"3"`, or the name of a named number |
| ENUMERATED | the name of an item |
| any of these | the name of a value assigned in ASN.1, as `PKIX1Algorithms88.id-ecPublicKey` |

A name is looked up in the module of `type`, as if written there, and
may be given as `Module.name`. Two keys for the same value are an
error.

A type, in `choices` and `default`, is one of:

- A built-in type with no constraints, as `NULL`, `INTEGER` or
  `OCTET STRING`.
- A type assigned in ASN.1, as `Module.Type`, or as `Type` when the
  module of `type` has or imports it.
- `OpenType`, for the bytes of the encoding, as an `ANY` has.
  `default = "OpenType"` keeps a value that is not in `choices`
  instead of rejecting it.

`asnkitcc` rejects the file if a type or a member is not found, if the
member is not an `ANY DEFINED BY`, if a key is not a value of the type
of the member that defines it, or if there are keys it does not know
of.

## Giving the choices to a runtime

A runtime can be given the same choices when loading a specification,
so that a compiled specification can be used without compiling it
again. They are given for each member, by its `type` and `member` as
above, and replace what the `.asni` file has for that member. The
values are those the runtime gives for the member that defines the
type, as an OBJECT IDENTIFIER in text, and the types are named as
above, with only the built-in types and the types the specification
has.

In Python:

```python
foo = asnkitrt.load(
    "rfc5280.asni",
    "der",
    any_defined_by={
        ("PKIX1Explicit88.AlgorithmIdentifier", "parameters"): {
            "choices": {
                "1.2.840.113549.1.1.1": "NULL",
                "1.2.840.10045.2.1": "PKIX1Algorithms88.ECParameters",
            },
            "default": "OpenType",
        },
    },
)
```

In Rust, with `Options::any_defined_by`:

```rust
let mut options = Options::new(Codec::Der);
options.any_defined_by(
    "PKIX1Explicit88.AlgorithmIdentifier",
    "parameters",
    AnyDefinedBy::new()
        .choice("1.2.840.113549.1.1.1", "NULL")
        .choice("1.2.840.10045.2.1", "PKIX1Algorithms88.ECParameters")
        .default("OpenType"),
);
```

In JavaScript, with the WebAssembly runtime, as an option of `load`:

```js
const foo = await loadFile('rfc5280.asni', 'der', {
  anyDefinedBy: [{
    type: 'PKIX1Explicit88.AlgorithmIdentifier',
    member: 'parameters',
    choices: { '1.2.840.113549.1.1.1': 'NULL' },
    default: 'OpenType',
  }],
});
```

A name of a value, as `id-ecPublicKey`, is only understood by
`asnkitcc`: a runtime takes the values themselves.

## How it is encoded

The member has the type of the choice, which every codec encodes as
[File formats](file_formats.md#built-in-types) tells for an
`AnyDefinedByType` with choices. A choice of `OpenType` is encoded as
an `ANY`, so JER and XER reject it.

In the `.asni` file, each choice is an `AnyDefinedByChoice`, and the
default is the field `default` of the `AnyDefinedByType`.
