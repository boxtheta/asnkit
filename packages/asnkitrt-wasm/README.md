# asnkitrt (WebAssembly)

The asnkit runtime for browsers and Node.js. It is the Rust runtime
compiled to WebAssembly, with a small JavaScript API. It loads the
`.asni` files produced by `asnkitcc` and uses them to encode and decode
ASN.1, without parsing ASN.1 itself.

The codecs are BER, DER, OER, PER, UPER, JER and XER. It can also check
values against their constraints, and load signed, encrypted and bound
`.asni` files.

```js
import { load } from 'asnkitrt';

const asni = new Uint8Array(await (await fetch('foo.asni')).arrayBuffer());
const foo = await load(asni, 'uper');

const encoded = foo.encode('Question', { id: 1, question: 'Is 1+1=3?' });
const decoded = foo.decode('Question', encoded);
```

In Node.js, `loadFile('foo.asni', 'uper')` reads the file too. The
package picks its entry point through the `node` condition of
`exports`: the browser entry fetches the `.wasm` file from next to its
module, which bundlers such as Vite and webpack handle, and the Node.js
entry reads it from disk.

## Values

Values are those of the Python runtime, in JavaScript types:

| ASN.1 | JavaScript |
|---|---|
| `BOOLEAN`, `NULL` | `boolean`, `null` |
| `INTEGER` | `number`, or `bigint` outside the safe integers; both are accepted |
| `REAL` | `number` |
| `ENUMERATED` | the name as a string, or a `number` with `numericEnums` |
| `OCTET STRING`, `ANY`, open types | `Uint8Array` |
| `BIT STRING` | `[Uint8Array, number of bits]` |
| `OBJECT IDENTIFIER`, character strings, `DURATION` | `string`, as `'1.2.3'` for an OID |
| `SEQUENCE`, `SET`, `EXTERNAL` | an object with the members that are present |
| `SEQUENCE OF`, `SET OF` | an array |
| `CHOICE` | `[name of the alternative, value]` |
| `DATE` | `{ year, month, day }` |
| `TIME-OF-DAY` | `{ hour, minute, second, microsecond, offset }` |
| `UTCTime`, `GeneralizedTime`, `DATE-TIME` | the fields of both of the above |

`offset` is in minutes east of UTC, or `null` for local time. When
encoding, a JavaScript `Date` is also accepted for dates and times, in
UTC, and a member set to `undefined` counts as absent. Values are
converted according to their ASN.1 type, so an array is a `CHOICE` or a
`SEQUENCE OF` as the type says, and `1` is a valid `REAL`.

`encode()` always returns a `Uint8Array`, also for JER and XER, which
`TextDecoder` turns into text. `decode()` accepts a `Uint8Array`, or a
string for JER and XER.

## API

- `load(asni, codec = 'ber', options)` loads the contents of an `.asni`
  file. The options are those of `asnkitrt.load_asni()` in Python, in
  camel case: `numericEnums`, `lenient`, `nonce` (a number, or a
  function that returns whether a nonce is accepted), `acceptAnyNonce`,
  `boundTo`, `acceptAnyBinding`, `verificationKeys`,
  `acceptAnySignature` and `decryptionKeys`. Keys are `[id, key]` pairs
  of `Uint8Array`s, as a `Map` or an array.
- `spec.encode(name, value, options)` and `spec.decode(name, data,
  options)`. The options are `checkConstraints`, `module` for a type
  name that more than one module defines, and, for `encode()` of JER
  and XER, `indent`, the number of spaces for each level.
- `spec.checkConstraints(name, value, options)`.
- `spec.decodeWithLength(name, data)` and `spec.decodeLength(data)`, for
  BER and DER only.
- `spec.free()` releases the WebAssembly memory of a specification
  right away, instead of when it is garbage collected.

Errors are thrown as `Error`s named `EncodeError`, `DecodeError` (with
the `offset` of the error, in bytes, or in bits for PER and UPER),
`ConstraintsError`, `UnsupportedError`, `NoSuchTypeError` and
`LoadError`, or as a `TypeError` for wrong arguments. TypeScript types
are in `js/index.d.ts`.

## Building and testing

Building needs the `wasm32-unknown-unknown` target and the
`wasm-bindgen` command, of the same version as the `wasm-bindgen` crate
in `Cargo.toml`, run in `packages/asnkitrt-wasm` of the
[repository](https://github.com/boxtheta/asnkit):

```text
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm run build
npm test
```

The build writes `pkg/`, with a `.wasm` file of about 500 KB (190 KB
gzipped). The tests need Node.js 21 or later, and replay all the vectors
of `tests/vectors` through the JavaScript API, as the Rust runtime does
in Rust.

It supports what the Rust runtime supports; see
[its README](https://github.com/boxtheta/asnkit/blob/trunk/packages/asnkitrt-rs/README.md#limitations) for the
limitations.

Licensed under the MIT License. See `LICENSE-RUNTIMES`.
