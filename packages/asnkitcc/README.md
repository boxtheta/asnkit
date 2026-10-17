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
another: BER, DER, OER, PER, UPER, JER or XER. The specification is an
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
hexadecimal digits, `--lenient` has DER decode what BER allows, and
`--encryption-key-file` is the key to decrypt an encrypted `.asni` file
with. Its signature, nonce and binding are not checked. An `ANY` is
copied as the bytes it was decoded from, so it can only be converted
between BER and DER.

The compiler can also add a nonce to an `.asni` file, bind it to a
string, and sign and encrypt it. Run `asnkitcc` without arguments to see
the options. The plugin `NAME` is the program `asnkit-gen-NAME`, looked
up on the `PATH`; see [`docs/plugins.md`](https://github.com/boxtheta/asnkit/blob/trunk/docs/plugins.md). Install
the plugins with `cargo install asnkit-gen-rust` or
`pip install asnkit-gen-python`.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.
