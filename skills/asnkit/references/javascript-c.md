# JavaScript/WebAssembly and C

Both runtimes wrap the Rust runtime. Use the same compiled `.asni` schema
and the exact codec used by the peer.

## JavaScript / TypeScript

Install runtime package `asnkitrt`. In Node.js:

```js
import { loadFile } from 'asnkitrt';

const spec = await loadFile('foo.asni', 'der');
try {
  const value = { id: 1, question: 'Is 1+1=3?' };
  const encoded = spec.encode('Question', value, { checkConstraints: true });
  const decoded = spec.decode('Question', encoded, { checkConstraints: true });
  if (decoded.id !== value.id || decoded.question !== value.question) {
    throw new Error('Round-trip mismatch');
  }
} finally {
  spec.free();
}
```

In a browser use `load` with fetched schema bytes:

```js
import { load } from 'asnkitrt';
const response = await fetch('foo.asni');
if (!response.ok) throw new Error(`Schema fetch failed: ${response.status}`);
const spec = await load(new Uint8Array(await response.arrayBuffer()), 'uper');
```

Pass `{ module: 'Foo' }` in encode/decode options to disambiguate types.
Use `spec.checkConstraints(name, value, options)` for standalone checks.
All encoded output is `Uint8Array`, including text codecs; use
`new TextDecoder().decode(encoded)` for display. Use bytes for decode
when in doubt about a particular release's text-input convenience API.

Represent OCTET STRING and raw ANY as `Uint8Array`, BIT STRING as
`[Uint8Array, bitCount]`, CHOICE as `[name, value]`, SEQUENCE/SET as
objects and collections as arrays. Use `bigint` outside JavaScript's
safe integer range. ENUMERATED uses names unless `numericEnums` is set
at load time. Dates/times have explicit fields; offset is minutes east
of UTC or `null` for local time. See `packages/asnkitrt-wasm/js/index.d.ts`
for the exact shape. Class getters are not own enumerable properties;
convert such objects before encoding.

For applications compiling ASN.1 in the browser, the separate
`asnkit-compiler` WASM package exposes:

```js
import { compile } from 'asnkit-compiler';
import { load } from 'asnkitrt';
const asni = await compile(`Foo DEFINITIONS ::= BEGIN
  Question ::= SEQUENCE { id INTEGER, question IA5String }
END`);
const spec = await load(asni, 'uper');
```

This compiler package may need building from the checkout; do not assume
it is published wherever the runtime is. It cannot run executable code
generation plugins. Use its `compileWithSummary` when the UI needs module
and type names. See `packages/asnkit-compiler-wasm/README.md` for builds.
For WASM source builds, match the `wasm-bindgen-cli` version to the crate
version in the project's lockfile.

## C

Build the runtime with `cargo build --release -p asnkitrt-c`. Include
`packages/asnkitrt-c/include/asnkitrt.h`, link `libasnkitrt_c`, and arrange
the platform's runtime library search path for a shared build. Use the
complete checked example at `packages/asnkitrt-c/examples/question.c`
as the integration starting point; it demonstrates loading, encoding,
decoding and error handling. The package README covers static linking.

Core calls are `asnkit_spec_load_file`, `asnkit_value_new_*`,
`asnkit_map_set`, `asnkit_encode` and `asnkit_decode`. Consult the header
for exact arguments and option structs rather than guessing signatures.

Ownership is part of the API:

- Free owned specifications, values and output buffers with
  `asnkit_spec_free`, `asnkit_value_free`, `asnkit_bytes_free` respectively.
- Inserting a value into a map/list/choice transfers ownership, even if
  the call fails. Do not free the child separately afterward.
- Accessors return borrowed data valid only while the parent remains
  alive and unmodified. Value text has a length and is not NUL-terminated.
- Check statuses/NULL handles and capture `asnkit_last_error()` and
  `asnkit_last_error_offset()` when diagnosing failures.

Use the load options for expected container protections and the codec
options for constraint checks. Do not pass allocator-owned buffers to
C's `free`; use the library's matching destructor.
