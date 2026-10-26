# Plugins

`asnkitcc generate --plugin NAME` compiles the ASN.1 files and sends the
compiled specification, with the source text in it, to `asnkit-gen-NAME`
on standard input. The plugin writes the files it makes into the output
directory (`-o`, default `.`), and exits with `0` if it succeeded or a
non-zero code if it did not, with any error message on standard error.

Any program on `PATH` whose name begins with `asnkit-gen-` is a plugin.

## Protocol

Standard input is a CBOR map (RFC 8949):

| Key | Value |
|---|---|
| `"asni"` | Byte string: the compiled `.asni` file, with `sources[i].text` set |
| `"options"` | Map of string to string: the `--plugin-opt KEY=VALUE` flags |

Nothing is read from standard output; everything the plugin writes goes
to files in the current working directory, which `asnkitcc` sets to the
`-o` directory before it starts the plugin.

## Writing a plugin in Python

```python
#!/usr/bin/env python3
import sys
import cbor2
import asnkitrt.syntax

request = cbor2.load(sys.stdin.buffer)
header, specification = asnkitrt.syntax.load_asni(request["asni"])
options = request["options"]

for module_name, module in specification.modules.items():
    for type_name, type_descriptor in module.types.items():
        ...
```

The schema of `specification` is in `docs/file_formats.md`, and the
classes that `load_asni` returns are in `asnkitrt/syntax.py`. Each
`header.metadata.sources[i]` has `name`, `sha256` and `text`, the ASN.1
source text that was compiled.

## asnkit-gen-python

The first plugin, `asnkit-gen-python`, is in `packages/asnkit-gen-python`.
It writes one `.py` file with a `TypedDict` for each `SEQUENCE` and
`SET`, a `Literal` for each `ENUMERATED`, a union of single-key
`TypedDict`s for each `CHOICE`, a type alias for each other named type,
and a class for each type with `encode()`, `decode()` and
`check_constraints()` methods that check the type of their argument with
mypy and pyright.

```text
$ pip install asnkit-gen-python
$ asnkitcc generate --plugin python -o src/asn foo.asn
```

```python
from asn import foo

encoded = foo.Question.encode({"id": 1, "question": "Is 1+1=3?"})
decoded = foo.Question.decode(encoded)
# decoded is foo.QuestionDict, so mypy knows decoded["id"] is int.
```

Options, passed with `--plugin-opt KEY=VALUE`:

`codec`
: The default codec of `encode()` and `decode()` (default `ber`). Every
  method also accepts `codec=` to use another at the call.

`module`
: The name of the `.py` file to write, without `.py` (default: the stem
  of the first `.asn` file, converted to `snake_case`).

## asnkit-gen-rust

The second plugin, `asnkit-gen-rust`, makes a Rust module that uses the
crate `asnkitrt` and `serde`, has the compiled specification in it
as bytes, and has a struct for each `SEQUENCE` and `SET` and an enum
for each `ENUMERATED` and `CHOICE`, with `encode()`, `decode()`,
`to_value()`, `from_value()` and `check_constraints()`. A `BIT STRING`
with named bits is a type with a constant and methods for each bit,
and the named numbers of an `INTEGER` are constants. Its
options are `name`, the name of the module, `no_std`, which is
`true` for code that needs `alloc` and not `std` (default
`false`), and `integer`, which is what an `INTEGER` with no bounds
is: `i64` (default), `i128` or `big`, which holds any, and `embed`,
which is `false` for a module without the compiled specification and
its `load()`, which the code that uses it loads (default `true`). See
`packages/asnkit-gen-rust/README.md`.

## asnkit-gen-rust-codec

The third plugin, `asnkit-gen-rust-codec`, makes a Rust module that
encodes and decodes the types of the specification with BER, DER and
UPER, selected with `--plugin-opt codec=ber|der|uper` (default `ber`),
with code of its own for each type using `asnkitrt::wire`,
without the compiled specification: a value that is decoded borrows
the data. Buffer-based encoding and borrowed decoding avoid allocations
except for boxed recursive values; the `to_*` convenience methods
allocate. It is for embedded targets, where the
heap the runtime keeps a specification on is too much. Its options are
`codec`, `name`, `no_std`, `integer` and
`unsupported`, which is `skip` to leave out the types it does not
support yet rather than to fail. Only the selected codec's methods and
helpers are emitted. It has the same constants of named bits and named
numbers as `asnkit-gen-rust`. See `packages/asnkit-gen-rust-codec/README.md`.
