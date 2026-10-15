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
```

The compiler can also add a nonce to an `.asni` file, bind it to a
string, and sign and encrypt it. Run `asnkitcc` without arguments to see
the options. The plugin `NAME` is the program `asnkit-gen-NAME`, looked
up on the `PATH`; see [`docs/plugins.md`](https://github.com/boxtheta/asnkit/blob/trunk/docs/plugins.md). Install
the plugins with `cargo install asnkit-gen-rust` or
`pip install asnkit-gen-python`.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.
