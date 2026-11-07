# asnkit-compiler (WebAssembly)

The asnkit compiler for browsers and Node.js. It is the Rust library
`asnkit-compiler` compiled to WebAssembly, with a small JavaScript API.
It turns the text of an ASN.1 specification into the contents of an
`.asni` file, which the runtimes load to encode and decode, such as
[`asnkitrt`](../asnkitrt-wasm) on npm.

```js
import { compile, schema } from 'asnkit-compiler';
import { load } from 'asnkitrt';

const asni = await compile(`
  Foo DEFINITIONS ::= BEGIN
    Question ::= SEQUENCE { id INTEGER, question IA5String }
  END
`);
const foo = await load(asni, 'uper');

foo.encode('Question', { id: 1, question: 'Is 1+1=3?' });

// A JSON Schema of the JER of a Question.
const { text } = await schema(asni, { format: 'json-schema', types: 'Question' });
```

It makes the same `.asni` file as `asnkitcc`, byte for byte, but for
the name of the source file, which a text does not have.

## API

- `compile(specification, options)` returns the contents of an `.asni`
  file as a `Uint8Array`. The text has one module or more. There are no
  options yet; those that were removed, as `boundTo`, are refused.
- `compileWithSummary(specification, options)` returns `{ asni,
  modules, warnings }`, with the `name`, `types` and `objectClasses`
  of each module, and the `kind` and `message` of each warning, of
  what is valid ASN.1 but likely to cause trouble, as a type that
  refers to itself (`RecursiveType`).
- `schema(asni, options)` makes what the values of the types of an
  `.asni` file are in a text codec, as `asnkitcc schema` does, and
  returns `{ text, types, skipped }`. The options are `format`:
  `json-schema` (the default) for a JSON Schema of JER, `xsd` for an
  XML Schema of BASIC-XER or `dtd` for a DTD of it; `types`, a name or
  an array of them, as `Type` or `Module.Type`, of the types the
  documents are values of, every type if none; and `skipUnsupported`,
  to leave out what a schema can not tell, as an `ANY`, and what refers
  to it, rather than fail. `skipped` has the `type` and the `reason` of
  each that is left out. A type given in `types` is never left out.
- `parse(specification)` returns the contents of an `.asnst` file, the
  syntax tree that plugins generate code from.
- `compileFile(path, options)` reads the file first, in Node.js only.
- `version()` is the version of the compiler.
- `ready()` loads the WebAssembly, which the other functions do too.

Errors are thrown as `Error`s named `ParseError` and `CompileError`,
with the line and the column, as an `Error` named `SchemaError`, with
where in the types, or as a `TypeError` for wrong arguments. There are no plugins, which are programs to run. TypeScript
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

The build writes `pkg/`, with a `.wasm` file of about 795 KB (285 KB
gzipped). The schemas are made from the library of `asnkitcc`, which
reads the types with the runtime for Rust.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.
