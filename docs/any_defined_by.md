# ANY DEFINED BY

The compiler accepts explicit type mappings for `ANY DEFINED BY` members.
The mappings are written into the existing `AnyDefinedByType.choices`
field of the `.asni` format, which the runtimes already understand.
No file-format version change is needed.

An `ANY DEFINED BY` member has the type selected by another member of the
same `SEQUENCE` or `SET`. ASN.1 does not supply the mapping from selector
values to types; that normally comes from the protocol specification.
Without a mapping, the runtime treats the member as raw encoded bytes.
With a mapping, its value has the representation of the selected type.

## Command line

```text
asnkitcc compile --any-defined-by choices.toml -o protocol.asni protocol.asn
asnkitcc generate --any-defined-by choices.toml --plugin python -o out protocol.asn
asnkitcc generate --any-defined-by choices.toml --plugin rust -o src protocol.asn
```

`--any-defined-by` can be repeated to read several TOML files. All ASN.1
modules used by the mappings must be supplied in the compilation. The
option is for `compile` and `generate`, not `parse` or `convert`.
To convert with mappings, compile an `.asni` first and pass it to
`asnkitcc convert`. Generation gives the plugin the mapped `.asni` and
the original `.asnst`; generated models still use dynamic values for
an `ANY DEFINED BY`, with dispatch performed by the embedded schema.

For example:

```asn1
Example DEFINITIONS ::= BEGIN
    Message ::= SEQUENCE {
        kind INTEGER { text(1), number(2) },
        payload ANY DEFINED BY kind
    }
END
```

```toml
[[any_defined_by]]
type = "Example.Message"
member = "payload"

[any_defined_by.choices]
text = "UTF8String"
number = "INTEGER"
```

For `kind = 1`, `payload` is a string; for `kind = 2`, it is an integer.
Other selector values are rejected by encoding and decoding. To keep
raw bytes for a specific known value, map that value to `OpenType`.

## TOML format

Configuration files are UTF-8 TOML. The root accepts only
`any_defined_by`, an array of tables. Each `[[any_defined_by]]` starts a
new mapping. A file may contain several mappings; an empty file (or
`any_defined_by = []`) adds none. Unknown fields at the root or inside a
mapping are errors.

Each `[[any_defined_by]]` entry has three required fields:

| Field | TOML type | Meaning |
|---|---|---|
| `type` | String | The containing type assignment as `Module.Type`, with the exact ASN.1 case. |
| `member` | String | A member name, or a dotted path through inline constructed types. |
| `choices` | Table of string values | A nonempty mapping from selector values or ASN.1 value names to type names. |

### Table styles and quoting

Both an inline table and a table section are supported. These entries
illustrate the two styles for different target members:

```toml
[[any_defined_by]]
type = "Messages.Message"
member = "payload"
choices = { "1" = "UTF8String", "2" = "INTEGER" }

[[any_defined_by]]
type = "Messages.Batch"
member = "entries.parameters"

[any_defined_by.choices]
"1.3.6.1.4.1.99999.1" = "UTF8String"
"Payloads.id-point" = "Payloads.Point"
```

`[any_defined_by.choices]` belongs to the most recent
`[[any_defined_by]]` entry. Put `type` and `member` **before** the choices
section: fields written afterward remain inside that table until another
header starts. Do not define `choices` both inline and as a section for
the same entry.

Quote keys containing dots, including literal OIDs and qualified ASN.1
names. Without quotes, TOML reads `Payloads.id-point` as nested tables,
not as one selector key. Plain keys such as `text`, `id-text`, `TRUE`, and
`FALSE` are allowed; TOML keys are strings even when they look like a
number or boolean. Quoting numeric keys is recommended for clarity.
All **choice values** must be strings naming types: `"NULL"`, not TOML's
boolean `false` or an unquoted word. Single-quoted and double-quoted TOML
strings both work. `#` starts a comment outside a string.

### Member paths

A path such as `content.parameters` enters inline members. A `SEQUENCE
OF` or `SET OF` element consumes no path component. Extension addition
groups are traversed by their member names. If a path reaches a named
type reference, configure that type's assignment directly; this avoids
silently changing other uses or specializing a referenced type.

### Selector keys

Selector keys are strings in TOML:

| Defining member | Keys |
|---|---|
| INTEGER | Decimal integers, named numbers, or assigned INTEGER value names. Selectors must fit signed 128 bits. |
| ENUMERATED | Item names or assigned values of that enumeration. Both runtime name and numeric-enum modes are supported. |
| OBJECT IDENTIFIER | Dotted decimal OIDs, or assigned OBJECT IDENTIFIER value names. |
| BOOLEAN | `TRUE` or `FALSE`. |

Unqualified assigned names are looked up in the module of `type`,
including its imports. Qualified names such as `Algorithms.id-rsa` are
also accepted. Named INTEGER numbers and ENUMERATED items come from the
defining member's type. OID assignments may reference other OID values
and use named arcs. Different keys resolving to the same selector value
are rejected.

### Choice types

A choice type is an assigned type (local, imported, or `Module.Type`),
a built-in type requiring no component definition, such as `NULL`,
`INTEGER` or `OCTET STRING`, or `OpenType`. Structured or parameterized
types should be assigned/instantiated in ASN.1 and referred to by name.
The selected type retains its tags and constraints. Tags written on the
ANY DEFINED BY member itself are retained too.

### Combining files and diagnosing errors

```sh
asnkitcc compile \
  --any-defined-by messages.toml --any-defined-by batch.toml \
  -o messages.asni payloads.asn messages.asn
```

Mappings are appended in command-line file order. Files do **not**
override earlier entries or merge their `choices` tables. Configure a
particular `(type, member)` pair once across all files. For example,
putting one choice for `Messages.Message.payload` in each of two files
is a duplicate target error; combine those choices into one entry.

Paths supplied to the CLI are relative to its working directory. The
TOML contains ASN.1 names, not file paths, and has no include directive.
All schema files still need to be supplied to the compiler.

| Problem | Correction |
|---|---|
| `default = "OpenType"` | Remove it; fallbacks are not implemented. Map specific known selectors to `OpenType` if raw bytes are intended. |
| `choices = { "1" = 2 }` | Use a quoted type name, such as `"INTEGER"`. |
| Unquoted `1.2.3` or `Module.value` key | Quote the entire key to prevent nested TOML tables. |
| Empty `choices` | Supply at least one mapping, or remove the whole entry to retain unconfigured ANY behavior. |
| Two keys such as `"1"` and a named number resolving to 1 | Keep one; duplicate detection uses resolved selector values. |
| A path enters a named type | Target that type's own `Module.Type` assignment and member path. |
| Unknown selector when encoding or decoding | Add the protocol's mapping and recompile, or correct the message's selector. |

The compiler rejects unknown TOML fields, missing types or members,
non-ANY DEFINED BY targets, unsupported selector types, duplicate target
entries, duplicate selector values, and empty choice tables. Errors
identify the configured type and member; TOML parse errors also identify
the configuration file in the CLI.

For an AlgorithmIdentifier-style schema, a configuration can use:

```toml
[[any_defined_by]]
type = "PKIX1Explicit88.AlgorithmIdentifier"
member = "parameters"

[any_defined_by.choices]
"1.2.840.113549.1.1.1" = "NULL"
"PKIX1Algorithms88.id-ecPublicKey" = "PKIX1Algorithms88.ECParameters"
```

Supply the ASN.1 files defining all these assignments.

## Runnable examples

The repository's
[`examples/any_defined_by`](https://github.com/boxtheta/asnkit/tree/trunk/examples/any_defined_by)
contains complete, matching files:

| Files | Purpose |
|---|---|
| `messages.asn`, `payloads.asn` | A small protocol and imported payload types/OID assignments. |
| `messages.toml` | INTEGER, ENUMERATED and BOOLEAN mappings, using both table styles. |
| `batch.toml` | Imported and qualified OID values, a literal OID, and a path through inline collection elements. |
| `message.json`, `event.json`, `flagged.json`, `batch.json` | JER messages with typed payloads, ready to encode. |
| `raw.der.hex` | A known selector mapped to `OpenType`, carrying encoded DER NULL. |
| `README.md` | Compilation, conversion, expected bytes, and model generation commands. |

The commands in that README run from the repository root and place
output in `target/any-defined-by-example`. They use two repeated
`--any-defined-by` options and show why compiling before conversion
matters. No external ASN.1 specifications are needed.

## Rust compiler library

Existing entry points keep their behavior. Use `CompileOptions` with
`compile_with_options`, `compile_to_asni_with_options`, or
`compile_to_asni_with_summary_and_options` for mappings:

```rust
use asnkit_compiler::{CompileOptions, Input, compile_to_asni_with_options};
use asnkit_compiler::container::Protection;

let options = CompileOptions::from_toml(&std::fs::read_to_string("choices.toml")?)?;
let input = Input::String(std::fs::read_to_string("protocol.asn")?);
let asni = compile_to_asni_with_options(
    &input, None, None, &Protection::default(), &options,
)?;
std::fs::write("protocol.asni", asni)?;
```

Callers can also construct `AnyDefinedBy` entries directly using
`type_name`, `member`, and a `BTreeMap<String, String>` of `choices`.
To combine configuration files, append their `any_defined_by` vectors;
compilation checks for duplicates across the combined configuration.

## Runtime behavior and limitations

Load the compiled file with the ordinary runtime API; no new load option
is needed. An `OpenType` choice uses raw bytes and is not supported in
JER/XER. Raw bytes remain specific to their encoding, so they cannot be
blindly transcoded across codecs.

Choice constraints are preserved in the compiled schema. The existing
runtime constraint walkers do not automatically dispatch through an
ANY DEFINED BY member; check the selected type explicitly when needed.

Use a defining member that is available before the dependent member
when decoding. The existing runtimes do not provide a general deferred
resolution mechanism for selectors that appear later in a message.

## Deferred work

Fallback types (`default = "OpenType"`, for example) and runtime
load-time mapping overrides are **not implemented**. The compiler
rejects `default` rather than accepting it without effect. Version 1.0
of the node format has no fallback field; adding one requires a format
and runtime change. The JavaScript compiler wrapper does not yet expose
mapping options; use the CLI or Rust compiler library.
