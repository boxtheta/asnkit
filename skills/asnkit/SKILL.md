---
name: asnkit
description: Use asnkitcc and the asnkit compiler and runtime libraries to compile ASN.1 schemas, generate Python or Rust types, encode and decode messages, convert codecs, and check constraints. Use for applications integrating asnkit or diagnosing its schema/value/API errors.
---

# Use asnkit

asnkit separates compilation from message processing:

`ASN.1 source → asnkitcc / asnkit-compiler → .asni → asnkitrt → encoded messages`

The runtime does not parse ASN.1 source. An `.asni` file is a compiled
schema, not an encoded message. An `.asnst` file is a syntax tree for
tooling and plugins, not a runtime schema. Generated Python and Rust
models still depend on their runtime.

## Choose the workflow

Identify the schema (including imported modules), top-level type, wire
codec, and application language from the request and project. Preserve
the protocol's codec; PER and UPER are not interchangeable. If decoding
unknown bytes, obtain the missing schema/type/codec rather than guessing
from a hex dump.

Read only the reference needed for the task:

- [Compiler and code generation](references/compiler.md): command-line
  compilation and conversion, embedded Rust compiler, generated models.
- [ANY DEFINED BY configuration](references/any-defined-by.md): write TOML
  mappings, resolve names and member paths, and run the included examples.
- [Python and Rust runtimes](references/python-rust.md): loading schemas,
  value representations, validation, module disambiguation, embedded builds.
- [JavaScript and C runtimes](references/javascript-c.md): WebAssembly,
  browser compilation, C ownership and linking.

These examples describe this repository's implementation. For an installed
package, check the project lockfile/version and its actual API before using
new features. Prefer matching compiler, runtime and generator releases.
Within a checkout, resolve source paths from the repository root, not the
skill directory; in other projects use the installed package documentation.
Do not treat proposal documents as implemented APIs.

## Integrate and verify

1. Compile all required ASN.1 modules together, or reuse the application's
   existing `.asni` file. Load once per codec and reuse the specification.
2. Use the runtime's ASN.1 value representation. In particular, bytes,
   bit strings, choices and large integers are not ordinary JSON values.
3. Check constraints explicitly when the task requires schema validation.
   Successful encoding or decoding alone is not a full constraint check.
4. Verify a representative message with encode/decode, plus a known wire
   fixture when interoperability matters. Account for inserted defaults
   and representation normalization when comparing values. For binary
   text representations, explicitly convert hex to bytes before decoding.
5. Report the commands/API used and the validation result. Preserve schema,
   type, codec, error path and offset in diagnostics; PER/UPER offsets are
   bits, while the other codecs use bytes.

## Important boundaries

- The Rust runtime and its WASM/C wrappers support GSER decoding in this
  checkout; the Python runtime supports GSER encoding only. Older releases
  may differ. Rust GSER accepts bare values and encoder assignments, but
  rejects unknown SEQUENCE/SET members and does not support LDAP variant
  encodings or symbolic OID descriptors.
- Raw `ANY` and unresolved open-type values contain bytes in their source
  codec. Do not blindly transcode them to a different encoding. JER/XER
  require a resolved type. Use `--any-defined-by` on `compile` or
  `generate` for explicit mappings; fallback types and runtime load-time overrides remain unsupported.
- DER decoding is strict by default. Use lenient mode only when accepting
  BER-compatible input is an intentional requirement.
- Protected `.asni` containers need the expected verification/decryption
  keys, nonce and binding. Do not fix a load failure by silently disabling
  those checks. The CLI converter does not verify signatures, nonce or
  binding; use runtime loading checks when authenticity is required.
- Type support varies by runtime and features. Check the relevant package
  README and API before promising support for unusual ASN.1 constructs.
