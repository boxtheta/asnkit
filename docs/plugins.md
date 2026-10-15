# Plugins

> **Note:** This is a draft. The way a plugin is run and given its files may change until it is agreed on, and until then a plugin should expect to be changed with it.

A plugin is a program that makes something from an ASN.1 specification,
such as code in a language. `asnkitcc generate` compiles the
specification and runs the plugin on it:

```text
$ asnkitcc generate --plugin python -o out foo.asn
$ asnkitcc generate --plugin ./my-plugin --plugin-opt name=foo -o out foo.asn
```

The plugin `python` is the program `asnkit-gen-python`, found
where programs are found, as for `git` and its commands. A plugin
given with a `/` in it is the program at that path. A plugin is of
any language, and only needs something that reads CBOR if it reads the
files below.

## How a plugin is run

The plugin is run with these arguments:

- `--asni FILE` — The compiled specification, an `.asni` file. It is not signed,
  encrypted, bound to something or made with a nonce, as it is meant to
  be put in what the plugin makes and loaded from there. `asnkitcc generate` refuses the options for those.

- `--asnst FILE` — The syntax tree of the same specification, an `.asnst` file. It
  has the text of the ASN.1 files and where everything is found in them.

- `--out FOLDER` — Where the plugin writes what it makes. The folder exists.

- `--opt KEY=VALUE` — An option of the plugin, as given to `asnkitcc generate` with
  `--plugin-opt KEY=VALUE`. Once for each, in the order they were
  given. A plugin rejects an option it does not know of.

The files are in a folder of their own, which is removed once the plugin
is done. The plugin reads them while it runs and does not keep them.

A plugin tells how it did with its exit status: 0 is that it made what
was asked for, and anything else is that it did not. What it writes to
standard error is shown to the person running it as it is, and what it
writes to standard output as well.

Most plugins want the compiled specification (`--asni`), where
everything is resolved: `COMPONENTS OF`, tags, parameterized types and
the modules that types come from are all worked out. The syntax tree is
for those that want what is written, such as the names of the files, or
the order of the members as the specification gives it.

What a plugin makes is under the license its author gives it. What
the plugins of this repository make, with the helper code they copy
into it, may be used under terms of your choice, as the header of each
file they write says.

The formats are in [file formats](file_formats.md).

## asnkit-gen-python

The first plugin makes a Python module that imports `asnkitrt`, has the
compiled specification in it as bytes, and has data classes with
`encode()`, `decode()`, `to_value()` and `from_value()`. Its
options are `name`, the name of the module, `codec`, which the
module encodes and decodes with if not told otherwise (default
`ber`), and `embed`, which is `false` for a module without the
compiled specification, which is then given to it with its
`use_asni()` (default `true`). See `packages/asnkit-gen-python/README.rst`.

## asnkit-gen-rust

The second plugin, `asnkit-gen-rust`, makes a Rust module that uses the
crate `asnkitrt` and `serde`, has the compiled specification in it
as bytes, and has a struct for each `SEQUENCE` and `SET` and an enum
for each `ENUMERATED` and `CHOICE`, with `encode()`, `decode()`,
`to_value()`, `from_value()` and `check_constraints()`. Its
options are `name`, the name of the module, `no_std`, which is
`true` for code that needs `alloc` and not `std` (default
`false`), and `integer`, which is what an `INTEGER` with no bounds
is: `i64` (default), `i128` or `big`, which holds any, and `embed`,
which is `false` for a module without the compiled specification and
its `load()`, which the code that uses it loads (default `true`). See
`packages/asnkit-gen-rust/README.md`.
