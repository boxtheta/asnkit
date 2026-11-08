# Plugins

`asnkitcc generate --plugin NAME` compiles the ASN.1 files, parses them
into a syntax tree, and runs the program `asnkit-gen-NAME` on the two.
The plugin writes the files it makes into the output directory (`-o`,
default `.`), and exits with `0` if it succeeded or a non-zero code if
it did not, with any error message on standard error.

Any program on `PATH` whose name begins with `asnkit-gen-` is a plugin.
`--plugin` also takes the path of a program, as `--plugin ./my-plugin`.

## Protocol

The plugin is run with these arguments:

```text
asnkit-gen-NAME --asni FILE --asnst FILE --out FOLDER [--opt KEY=VALUE ...]
```

| Argument | Value |
|---|---|
| `--asni FILE` | The compiled specification, a version 2 `.asni` file |
| `--asnst FILE` | The syntax tree, an `.asnst` file, whose metadata has the source text of each ASN.1 file (`sources[i].text`) |
| `--out FOLDER` | The `-o` folder of `asnkitcc`, which exists, for the files the plugin writes |
| `--opt KEY=VALUE` | One for each `--plugin-opt KEY=VALUE`, in order |

The two files are in a temporary folder of their own, which is removed
when the plugin exits. Nothing is written to the plugin's standard
input, and what it writes to standard output and standard error goes
to those of `asnkitcc`.

## Pragmas

A plugin can be told what the ASN.1 has no way to tell with pragmas,
comments `--@ key = value` on the lines before a module, an
assignment, a member or a field of an object class:

```text
Greeter DEFINITIONS AUTOMATIC TAGS ::= BEGIN

--@ rpc.method = Greet
HelloRequest ::= SEQUENCE {
    --@ json.name = the_name
    name UTF8String
}

END
```

They are in the syntax tree, as `pragmas` of what they are of: pairs
of a key and a value, in the order they are written, or none. They are
not in the compiled specification.

```python
tree = syntax.from_asnst_file(args.asnst)

for module in tree.modules:
    for assignment in module.types:
        pragmas = dict(assignment.pragmas or ())

        if "rpc.method" in pragmas:
            ...  # pragmas["rpc.method"] is "Greet".
```

See [Pragmas](file_formats.md#pragmas) for what one can be and be of.

`asnkitcc --version` runs each plugin it finds on `PATH` with
`--version`, and shows the last word of what it prints as its version.

## Writing a plugin in Python

```python
#!/usr/bin/env python3
import argparse
import os

from asnkitrt import syntax

parser = argparse.ArgumentParser()
parser.add_argument("--asni", required=True)
parser.add_argument("--asnst")
parser.add_argument("--out", required=True)
parser.add_argument("--opt", action="append", default=[])
args = parser.parse_args()

options = dict(item.partition("=")[::2] for item in args.opt)
specification = syntax.from_asni_file(args.asni)

with open(args.asnst, "rb") as fin:
    sources = syntax.read_header(fin.read()).metadata.sources

for module in specification.modules:
    for assignment in module.types:
        ...  # assignment.name, assignment.type

with open(os.path.join(args.out, "out.txt"), "w") as fout:
    ...
```

`syntax.from_asni_file()` gives a `CompiledSpecification`, whose
`modules` each have a `name` and `types`, a tuple of `TypeAssignment`s
with a `name` and a `type`. The classes are in
`packages/asnkitrt/src/asnkitrt/syntax/nodes.py`, and what the nodes
mean is in [File formats](file_formats.md). `syntax.from_asnst()` reads
the whole syntax tree, for a plugin that needs what the compiled
specification leaves out. Each of `sources` has `name`, `sha256` and
`text`, the ASN.1 source text that was compiled.

## asnkit-gen-python

The first plugin, `asnkit-gen-python`, is in `packages/asnkit-gen-python`.
It writes one `.py` file with a data class for each `SEQUENCE` and
`SET`, an enumeration for each `ENUMERATED` and a class for each
alternative of a `CHOICE`, all with `encode()` and `decode()` methods.
The module has the compiled specification in it, unless `embed=false`.

```text
$ pip install asnkit-gen-python
$ asnkitcc generate --plugin python -o out foo.asn    # Writes out/foo.py.
```

```python
>>> from foo import Question
>>> encoded = Question(id=1, question='Is 1+1=3?').encode('uper')
>>> Question.decode(encoded, 'uper')
Question(id=1, question='Is 1+1=3?')
```

Options, passed with `--plugin-opt KEY=VALUE`:

`name`
: The name of the module, without `.py` (default: the name of the first
  ASN.1 file).

`codec`
: The codec of `encode()` and `decode()` when none is given (default
  `ber`).

`embed`
: `false` to leave the compiled specification out of the module, which
  is then given one with `use_asni()` (default `true`). See
  `packages/asnkit-gen-python/README.rst`.

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
