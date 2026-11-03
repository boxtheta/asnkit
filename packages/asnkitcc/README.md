# asnkitcc

The asnkit compiler as a command. It parses ASN.1 and writes `.asni`
files, which a runtime loads to encode and decode without parsing
anything itself. It also runs plugins that generate code from the same
specification. The work is done by the library `asnkit-compiler`
([`asnkit-compiler`](https://crates.io/crates/asnkit-compiler)), which an
application can also use itself.

```text
$ asnkitcc compile foo.asn                           # Writes foo.asni.
$ asnkitcc parse foo.asn                             # Writes foo.asnst, a syntax tree for plugins.
$ asnkitcc generate --plugin rust -o src foo.asn     # Writes src/foo.rs.
$ asnkitcc convert -t Question -i q.json -o q.der foo.asni   # JER to DER.
```

`convert` decodes a value of a type with one codec and encodes it with
another: BER, DER, OER, PER, UPER, JER, XER or GSER. The specification is an
`.asni` file, or the ASN.1 files, which are compiled first. The codecs
are given with `--from` and `--to`, or told by the extensions of `-i`
and `-o` (`.json` is JER, `.xml` is XER, `.der` is DER and so on), and
the value is read from standard input and written to standard output
when these are not given:

```text
$ asnkitcc convert -t Question --from der --to jer --indent 2 foo.asn < q.der
$ echo 300e0201011609497320312b313d333f \
    | asnkitcc convert -t Foo.Question --from der --input-hex --to uper --output-hex foo.asni
```

A type that is in more than one module is given as `Module.Type`.
`--input-hex` and `--output-hex` read and write the binary codecs as
hexadecimal digits, and `--lenient` has DER decode what BER allows.
An `ANY` is
copied as the bytes it was decoded from, so it can only be converted
between BER and DER.

`compile` and `generate` print warnings on standard error, of what is
valid ASN.1 but likely to cause trouble, as types that refer to
themselves, and still write what they make. A type defined by itself
alone, as `A ::= B` and `B ::= A`, is an error, and so are a type of
which every value would hold another and a constraint that permits no
value, as `INTEGER (5..1)`. See the
[README](https://github.com/boxtheta/asnkit#command-line) of the
repository for the warnings.

The compiler writes version 2 `.asni` files, with the CRC-32 of the
file in their header, which runtimes check. Signing, encryption, nonces
and binding are no longer supported; their old options, as
`--bound-to`, fail with an error. Recompile version 1 files from
source. Run `asnkitcc --help` to see the options.

The plugin `NAME` is the program `asnkit-gen-NAME`, looked up on the
`PATH`; see [`docs/plugins.md`](https://github.com/boxtheta/asnkit/blob/trunk/docs/plugins.md). Install
the plugins with `cargo install asnkit-gen-rust`,
`cargo install asnkit-gen-rust-codec` or `pip install asnkit-gen-python`.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.

## ANY DEFINED BY choices

Use `--any-defined-by choices.toml` with `compile` or `generate` to
supply mappings from selector values to payload types. Repeat the option
for multiple files. A minimal file is:

```toml
[[any_defined_by]]
type = "Example.Message"
member = "payload"
choices = { "1" = "UTF8String", "2" = "INTEGER" }
```

The schema must define `payload ANY DEFINED BY` an INTEGER member.
The resulting `.asni` carries the mappings for all runtimes. Unknown
selectors are rejected. See [ANY DEFINED BY](../../docs/any_defined_by.md)
for the TOML format, nested paths, named values and limitations.
Runnable schemas, mapping files and messages are in
[examples/any_defined_by](../../examples/any_defined_by/README.md).
