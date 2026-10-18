# asnkit

[![Rust](https://github.com/boxtheta/asnkit/actions/workflows/rust.yml/badge.svg)](https://github.com/boxtheta/asnkit/actions/workflows/rust.yml)
[![Python package](https://github.com/boxtheta/asnkit/actions/workflows/python-package.yml/badge.svg)](https://github.com/boxtheta/asnkit/actions/workflows/python-package.yml)
[![PyPI](https://img.shields.io/pypi/v/asnkitrt)](https://pypi.org/project/asnkitrt/)
[![crates.io](https://img.shields.io/crates/v/asnkitrt)](https://crates.io/crates/asnkitrt)
[![npm](https://img.shields.io/npm/v/asnkitrt)](https://www.npmjs.com/package/asnkitrt)

An [ASN.1](https://en.wikipedia.org/wiki/Abstract_Syntax_Notation_One)
toolkit that parses a specification once, so that encoding and decoding
can happen anywhere. A compiler turns ASN.1 into a compiled
specification IR, an `.asni` file. A runtime loads that file to encode and
decode. It never parses ASN.1 itself, which keeps it small enough for
embedded devices and makes it behave the same in every language. Plugins
generate code, such as data classes, from the same file.

```text
foo.asn --> compiler --> foo.asni --> runtime: encode, decode
                            |
                            +-------> plugin: Python or Rust code
```

Try it in the browser in the [playground](https://asnkit.boxtheta.org/playground/),
which runs the compiler and the runtime as WebAssembly; the
[documentation](https://asnkit.boxtheta.org/) is on the same site.

asnkit supports a large subset of ASN.1; what it does not support yet is
listed under [Known limitations](#known-limitations), and is rejected
with an error rather than encoded wrongly.

## What is in this repository

| Package | What it does | Location | Registry | License |
|---|---|---|---|---|
| **Runtimes** | Encode and decode with an `.asni` file. | | | |
| `asnkitrt` | Runtime for Python. | [`packages/asnkitrt`](packages/asnkitrt) | [PyPI](https://pypi.org/project/asnkitrt/) | MIT |
| `asnkitrt` | Runtime for Rust. Needs `alloc` but not `std`, so it builds for embedded targets. | [`packages/asnkitrt-rs`](packages/asnkitrt-rs) | [crates.io](https://crates.io/crates/asnkitrt) | MIT |
| `asnkitrt` | Runtime for browsers and Node.js: the Rust runtime as WebAssembly. | [`packages/asnkitrt-wasm`](packages/asnkitrt-wasm) | [npm](https://www.npmjs.com/package/asnkitrt) | MIT |
| `asnkitrt-c` | Runtime for C: the Rust runtime as a shared and a static library, with a header. | [`packages/asnkitrt-c`](packages/asnkitrt-c) | build from source | MIT |
| **Compiler** | Parses ASN.1 and writes `.asni` files. | | | |
| `asnkit-compiler` | The compiler as a Rust library, which also runs the plugins, for an application to compile ASN.1 itself. | [`packages/asnkit-compiler`](packages/asnkit-compiler) | [crates.io](https://crates.io/crates/asnkit-compiler) | Apache-2.0 |
| `asnkitcc` | The command, made with the library. | [`packages/asnkitcc`](packages/asnkitcc) | [crates.io](https://crates.io/crates/asnkitcc) | Apache-2.0 |
| `asnkit-compiler` | The compiler for browsers and Node.js, as WebAssembly. | [`packages/asnkit-compiler-wasm`](packages/asnkit-compiler-wasm) | not yet | Apache-2.0 |
| **Plugins** | Generate code from a compiled specification. | | | |
| `asnkit-gen-python` | Python data classes. | [`packages/asnkit-gen-python`](packages/asnkit-gen-python) | [PyPI](https://pypi.org/project/asnkit-gen-python/) | Apache-2.0 |
| `asnkit-gen-rust` | Rust structs and enums with `serde`. | [`packages/asnkit-gen-rust`](packages/asnkit-gen-rust) | [crates.io](https://crates.io/crates/asnkit-gen-rust) | Apache-2.0 |

All the packages are released together, with one version; see
[CHANGELOG.md](CHANGELOG.md). Within a major version, the `.asni` and
`.asnst` formats, the runtime APIs and the command line keep working.

Also in the repository:

- [`docs/file_formats.md`](docs/file_formats.md): the `.asni` and
  `.asnst` file formats.
- [`docs/third_party_runtimes.md`](docs/third_party_runtimes.md):
  guidelines for writing a runtime in another language.
- [`docs/plugins.md`](docs/plugins.md): how plugins are run, and how to
  write one.
- [`docs/any_defined_by.md`](docs/any_defined_by.md): a draft of how
  to tell which types an `ANY DEFINED BY` can hold.
- `tests/vectors`: encodings and decodings recorded from the Rust
  runtime, the one of reference, for testing a runtime in any language.
- `examples/corpus`: `.asni` files for testing a reader of the format.

## Codecs

| Codec | Python | Rust | WebAssembly |
|---|---|---|---|
| Basic Encoding Rules (BER) | yes | yes | yes |
| Distinguished Encoding Rules (DER) | yes | yes | yes |
| Octet Encoding Rules (OER) | yes | yes | yes |
| Aligned Packed Encoding Rules (PER) | yes | yes | yes |
| Unaligned Packed Encoding Rules (UPER) | yes | yes | yes |
| JSON Encoding Rules (JER) | yes | yes, feature `jer` | yes |
| XML Encoding Rules (XER) | yes | yes, feature `xer` | yes |
| Generic String Encoding Rules (GSER) | encoding only | encoding only, feature `gser` | encoding only |

The Rust runtime is the one of reference: the more than 5,300 vectors
in `tests/vectors` are what it does, including constraint checks, and
the Python runtime gives the same results for all of them. The
WebAssembly runtime is the Rust one, and is tested on the vectors
through its JavaScript API. In Rust,
the text codecs are crate features that are enabled by default. GSER
is for debug output: it is encoded and not decoded. `xer`
uses [quick-xml](https://crates.io/crates/quick-xml) and `bigint` uses
[num-bigint](https://crates.io/crates/num-bigint), and both need `std`,
so embedded builds use `default-features = false`.

The DER decoder rejects anything X.690 does not allow, such as a
`BOOLEAN` other than 0x00 or 0xff, an `INTEGER` that is not minimally
encoded, an unsorted `SET OF`, a `REAL` not in its canonical form, or a
`UTCTime` or `GeneralizedTime` other than the one form DER has. Pass
`lenient=True` to `asnkitrt.load()` (or set `lenient` in the Rust
runtime's options) to accept anything BER allows.

## Known limitations

- `EMBEDDED PDV` is not supported yet.

- `ANY` and `ANY DEFINED BY` are raw encoded bytes, which BER and DER
  write as they are and PER, UPER and OER as an open type. The bytes are
  in the codec's own encoding, so a value decoded with BER can't be
  encoded again with PER as is. JER and XER need the actual type, and
  so reject them. There is no way yet to tell the compiler which types
  an `ANY DEFINED BY` can hold ([`docs/any_defined_by.md`](docs/any_defined_by.md)
  is a draft of how it will be done); when a specification has them, all the
  codecs encode the value with its type.

- The Rust runtime does not support `CHARACTER STRING`. Bounds of
  constraints of more than 128 bits need its feature `bigint`, which is
  on by default and needs `std`. Anything unsupported returns an error;
  it never produces a wrong value.

- A union inside one constraint that mixes kinds, such as
  `WITH COMPONENT (SIZE (1) | FROM ("a"))`, is checked as no
  constraint: the sizes and the alphabets are combined separately, as
  PER does. This accepts more values than the standard allows, but never
  rejects a valid one.

- The Python runtime does not check a `WITH COMPONENTS` on a type that
  refers to itself. The Rust runtime does.

## Installation

The Python packages need Python 3.13 or later, the crates Rust 1.88 or
later, and the WebAssembly runtime Node.js 18 or later (or a browser
with WebAssembly).

```text
pip install asnkitrt           # Encode and decode.
pip install asnkit-gen-python  # Generate Python code from ASN.1 with asnkitcc.
```

Signed and encrypted `.asni` files need the
[cryptography](https://cryptography.io) package. `asnkitrt` installs it
only as an extra, and does not import it until such a file is loaded:

```text
pip install asnkitrt[signing]
pip install asnkitrt[encryption]
```

The compiler, the Rust runtime and the plugin for Rust are on
crates.io, and the WebAssembly runtime is on npm:

```text
cargo install asnkitcc         # The compiler.
cargo install asnkit-gen-rust  # Generate Rust code from ASN.1 with asnkitcc.
cargo add asnkitrt             # Encode and decode in Rust.
npm install asnkitrt           # Encode and decode in browsers and Node.js.
```

## Example

This ASN.1 specification defines the messages of a fictitious Foo
protocol (based on the FooProtocol example on Wikipedia):

```text
Foo DEFINITIONS ::= BEGIN

    Question ::= SEQUENCE {
        id        INTEGER,
        question  IA5String
    }

    Answer ::= SEQUENCE {
        id        INTEGER,
        answer    BOOLEAN
    }

END
```

Compile it to `foo.asni` in the current directory (the file is
`tests/files/foo.asn` in this repository):

```text
$ asnkitcc compile tests/files/foo.asn
```

### Python

Load the file and encode and decode with it. Only `asnkitrt` is needed:

```python
>>> import asnkitrt
>>> foo = asnkitrt.load_asni_file('foo.asni')
>>> encoded = foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
>>> encoded
b'0\x0e\x02\x01\x01\x16\tIs 1+1=3?'
>>> foo.decode('Question', encoded)
{'id': 1, 'question': 'Is 1+1=3?'}
```

BER is the default codec. The same with PER:

```python
>>> foo = asnkitrt.load_asni_file('foo.asni', 'per')
>>> foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
b'\x01\x01\tIs 1+1=3?'
```

### Rust

The Rust runtime loads the same file. Values are `Value`s, with a `Map`
for a `SEQUENCE` and so on, just like the dictionaries in Python:

```rust
use asnkitrt::container::Checks;
use asnkitrt::runtime::{Options, Specification};
use asnkitrt::types::Codec;
use asnkitrt::value::Value;

let data = std::fs::read("foo.asni")?;
let foo = Specification::from_asni(&data, &Checks::default(), Options::new(Codec::Uper))?;

let question = Value::Map(vec![
    ("id".into(), Value::Integer(1)),
    ("question".into(), Value::text("Is 1+1=3?")),
]);
let encoded = foo.encode("Question", &question)?;

assert_eq!(foo.decode("Question", &encoded)?, question);
```

### JavaScript

The WebAssembly runtime loads the same file, in a browser or in
Node.js, with the same values as in Python, in JavaScript types:

```js
import { loadFile } from 'asnkitrt';

const foo = await loadFile('foo.asni', 'uper');
const encoded = foo.encode('Question', { id: 1, question: 'Is 1+1=3?' });

foo.decode('Question', encoded); // { id: 1, question: 'Is 1+1=3?' }
```

### Generated code

A plugin generates native types for a language from the same
specification. `asnkitcc generate` compiles the specification and runs
the plugin on it:

```text
$ asnkitcc generate --plugin python -o out tests/files/foo.asn   # Writes out/foo.py.
$ asnkitcc generate --plugin rust -o src tests/files/foo.asn     # Writes src/foo.rs.
```

```python
>>> from foo import Question
>>> encoded = Question(id=1, question='Is 1+1=3?').encode('uper')
>>> Question.decode(encoded, 'uper')
Question(id=1, question='Is 1+1=3?')
```

```rust
let specification = foo::load(Codec::Uper)?;
let encoded = Question { id: 1, question: "Is 1+1=3?".into() }.encode(&specification)?;

assert_eq!(Question::decode(&specification, &encoded)?.id, 1);
```

See the `examples` folder for more, [`docs/plugins.md`](docs/plugins.md)
for how plugins work and how to write one, and
[`docs/file_formats.md`](docs/file_formats.md) for what an `.asni` file
contains.

## Command line

```text
asnkitcc compile foo.asn    # Writes foo.asni, for a runtime to load.
asnkitcc parse foo.asn      # Writes foo.asnst, a syntax tree for plugins.
asnkitcc generate --plugin NAME [--plugin-opt KEY=VALUE ...] foo.asn

python -m asnkitrt.syntax foo.asni    # Prints a file as JSON.
```

`asnkitcc` prints nothing on success. Its output is checked against the
cases in `packages/asnkit-compiler/tests/golden`, recorded from the
Python compiler it replaced.

The compiler can also add a nonce, bind a file to a string, and sign and
encrypt it. Run `asnkitcc` without arguments to see the options. Keys are
given as files with `--signing-key-file` and `--encryption-key-file`. A
key's identifier defaults to the first 16 bytes of the SHA-256 of the key
(of the public key, for signing), and can be set with `--signing-key-id`
and `--encryption-key-id`.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for
how to build and test a change, and for the sign-off (DCO) that each
commit needs. Everyone who takes part follows the
[Code of Conduct](CODE_OF_CONDUCT.md). To report a vulnerability, see
[SECURITY.md](SECURITY.md).

## Standards

The ASN.1 standards published by the ITU-T and the IETF.

### Notation

- [X.680: Specification of basic notation](https://www.itu.int/rec/T-REC-X.680)
- [X.681: Information object specification](https://www.itu.int/rec/T-REC-X.681)
- [X.682: Constraint specification](https://www.itu.int/rec/T-REC-X.682)
- [X.683: Parameterization of ASN.1 specifications](https://www.itu.int/rec/T-REC-X.683)

### Encoding rules

- [X.690: Basic (BER), Canonical (CER) and Distinguished (DER) Encoding Rules](https://www.itu.int/rec/T-REC-X.690)
- [X.691: Packed Encoding Rules (PER)](https://www.itu.int/rec/T-REC-X.691)
- [X.693: XML Encoding Rules (XER)](https://www.itu.int/rec/T-REC-X.693)
- [X.696: Octet Encoding Rules (OER)](https://www.itu.int/rec/T-REC-X.696)
- [X.697: JSON Encoding Rules (JER)](https://www.itu.int/rec/T-REC-X.697)
- [RFC 3641: Generic String Encoding Rules (GSER)](https://www.rfc-editor.org/rfc/rfc3641)

## Acknowledgements

asnkit began as a fork of [asn1tools](https://github.com/eerimoq/asn1tools)
by Erik Moqvist, and has since become a different project, with its own
compiler, file formats and runtimes. What is left of asn1tools is in
the codecs of the Python runtime, which keep its copyright and its MIT
license (see `packages/asnkitrt/NOTICE`).

## License

The runtimes, such as `asnkitrt`, are licensed under the MIT License
(`LICENSE-RUNTIMES`). The compiler, the plugins, the documentation and
everything else are licensed under the Apache License 2.0
(`LICENSE-COMPILER`). The test vectors are made from ASN.1 modules
written by others, and keep their authors' licenses. What the compiler
and the plugins produce, compiler and first party plugins output is subject
to no licensing claims from us and you may use it under your own terms.

THIS PROJECT IS PROVIDED AS IS WITH NO WARRANTY AS PER THE LICENSE.
