# asnkit-compiler (WebAssembly)

The asnkit compiler for browsers and Node.js. It is the Rust library
`asnkit-compiler` compiled to WebAssembly, with a small JavaScript API.
It turns the text of an ASN.1 specification into the contents of an
`.asni` file, which the runtimes load to encode and decode, such as
[`asnkitrt`](../asnkitrt-wasm) on npm.

```js
import { compile } from 'asnkit-compiler';
import { load } from 'asnkitrt';

const asni = await compile(`
  Foo DEFINITIONS ::= BEGIN
    Question ::= SEQUENCE { id INTEGER, question IA5String }
  END
`);
const foo = await load(asni, 'uper');

foo.encode('Question', { id: 1, question: 'Is 1+1=3?' });
```

It makes the same `.asni` file as `asnkitcc`, byte for byte, but for
the name of the source file, which a text does not have.

## API

- `compile(specification, options)` returns the contents of an `.asni`
  file as a `Uint8Array`. The text has one module or more. The options
  are those of `asnkitcc compile`, in camel case: `nonce`, `boundTo`,
  `signingKey`, `signingKeyId`, `encryptionKey` and `encryptionKeyId`.
  Keys are `Uint8Array`s of 32 bytes, and their identifiers of 16 bytes,
  derived from the key as `asnkitcc` does if not given. The
  initialization vector of an encrypted file comes from
  `crypto.getRandomValues()`.
- `compileWithSummary(specification, options)` returns `{ asni,
  modules }`, with the `name`, `types` and `objectClasses` of each
  module.
- `parse(specification)` returns the contents of an `.asnst` file, the
  syntax tree that plugins generate code from.
- `compileFile(path, options)` reads the file first, in Node.js only.
- `version()` is the version of the compiler.
- `ready()` loads the WebAssembly, which the other functions do too.

Errors are thrown as `Error`s named `ParseError` and `CompileError`,
with the line and the column, as `Error` for what the format does not
allow, such as a nonce that is too large, or as a `TypeError` for wrong
arguments. There are no plugins, which are programs to run. TypeScript
types are in `js/index.d.ts`.

## Building and testing

As for the runtime, building needs the `wasm32-unknown-unknown` target
and the `wasm-bindgen` command of the version of the crate. The tests
load what the compiler makes with the runtime, which is built first:

```text
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.129 --locked
(cd ../asnkitrt-wasm && npm run build)
npm run build
npm test
```

The build writes `pkg/`, with a `.wasm` file of about 525 KB (225 KB
gzipped).

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.
