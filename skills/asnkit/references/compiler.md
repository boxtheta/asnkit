# Compiler and generated code

## Command line

Use the project's compiler. An installed command is `asnkitcc`; in an
asnkit checkout use `cargo run -p asnkitcc --` followed by the same arguments.
Install with `cargo install asnkitcc` when an installation is needed.
Python runtimes require Python 3.13+, and this checkout's Rust crates
require Rust 1.88+; consult package manifests for the target release.

For a reproducible example, save this as `foo.asn`:

```asn1
Foo DEFINITIONS ::= BEGIN
    Question ::= SEQUENCE { id INTEGER, question IA5String }
END
```

```sh
asnkitcc compile -o foo.asni foo.asn
asnkitcc compile -o protocol.asni common.asn protocol.asn
asnkitcc parse -o foo.asnst foo.asn
```

Supply the files defining imported modules in the same invocation. Use
`-o` explicitly in automation. `parse` produces a syntax tree, whereas
`compile` produces the file runtimes load. `-e ENCODING` selects source
text encoding; `-v` prints additional compiler information.

Convert a message with a known type and codec:

```sh
asnkitcc convert -t Foo.Question --from jer --to der -i q.json -o q.der foo.asni
asnkitcc convert -t Foo.Question --from der --to jer --indent 2 -i q.der -o decoded.json foo.asni
```

`q.json` for this example is `{"id":1,"question":"Is 1+1=3?"}`.
The expected DER is `300e0201011609497320312b313d333f`.
`convert` also accepts ASN.1 source files and compiles them first.
Without `-i`/`-o`, it uses standard input/output. Use `--input-hex` or
`--output-hex` for hex text; do not send raw binary through text decoding.
Codecs can be inferred from extensions (`.json` → JER, `.xml` → XER),
but explicit `--from` and `--to` are clearer in pipelines. Use
`Module.Type` when names are ambiguous.

Conversion is not a constraint validator or signature verifier. For
protected schemas or explicit constraint validation, use the runtime API.

## Compiler inside a Rust application

Cargo package `asnkit-compiler` is imported as `asnkit_compiler`.
Use it only if the application needs to compile source itself; otherwise
ship a precompiled schema and depend only on `asnkitrt`.

```rust
use asnkit_compiler::{Input, compile_to_asni};
use asnkit_compiler::container::Protection;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source = std::fs::read_to_string("foo.asn")?;
    let asni = compile_to_asni(
        &Input::String(source), None, None, &Protection::default(),
    )?;
    std::fs::write("foo.asni", asni)?;
    Ok(())
}
```

For multiple files use `Input::Files { paths, encoding: "utf-8".into() }`.
The two `None` arguments above are nonce and binding. The result is
container bytes; `compile()` instead returns the compiled node structure.
See `packages/asnkit-compiler/src/lib.rs` for exact signatures and
`docs/file_formats.md` for protected-container requirements.

## Generated models

The compiler locates executable `asnkit-gen-NAME` on `PATH`. Generators
are separate installs; installing the compiler does not install them.

```sh
pip install asnkit-gen-python
asnkitcc generate --plugin python -o out foo.asn

cargo install asnkit-gen-rust
asnkitcc generate --plugin rust -o src foo.asn
```

Generation takes ASN.1 source files, compiles them and runs the plugin.
Models embed the schema by default. Inspect the generated API before
using transformed names or anonymous types.

Python, with `out` on the import path:

```python
from foo import Question

encoded = Question(id=1, question="Is 1+1=3?").encode("uper")
assert Question.decode(encoded, "uper").id == 1
```

Rust consumers need `asnkitrt` with `serde` support and `serde` with
`derive`. The generated module exposes the `Asn1Type` trait:

```rust
mod foo;
use asnkitrt::types::Codec;
use foo::{Asn1Type, Question};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spec = foo::load(Codec::Uper)?;
    let value = Question { id: 1, question: "Is 1+1=3?".into() };
    let encoded = value.encode(&spec)?;
    assert_eq!(Question::decode(&spec, &encoded)?.id, 1);
    Ok(())
}
```

Useful `--plugin-opt KEY=VALUE` options:

- Both: `name=...`, `embed=false`.
- Python: `codec=uper` changes the default codec.
- Rust: `integer=i128` or `integer=big` changes unbounded INTEGER from
  the default `i64`; `no_std=true` generates code requiring only `alloc`.

With `embed=false`, compile the same schema separately. Python requires
`use_asni(data, ...)` before model use. Rust omits `load()`; pass a
`Specification::from_asni(...)` result to the models. This supports
protected schemas without embedding an unchecked copy.

For embedded Rust, the not-yet-published plugin can be installed from
the repository with `cargo install --path packages/asnkit-gen-rust-codec`;
use `asnkitcc generate --plugin rust-codec --plugin-opt codec=der` to
generate DER codecs that do not load the schema. The `codec` option
selects only `ber` (default), `der` or `uper`; other codecs are not emitted.
Buffer-based methods avoid allocations
except for boxed recursive values; decoded values
borrow the input, and `encode_der(&mut buffer)` writes into a given
buffer. Only the matching `asnkitrt` is needed, with
`default-features = false`; use a path dependency on
`packages/asnkitrt-rs` until this runtime support is released.

For generator options and value mappings consult
`packages/asnkit-gen-python/README.rst`, `packages/asnkit-gen-rust/README.md`,
`packages/asnkit-gen-rust-codec/README.md`;
for custom plugin implementation consult `docs/plugins.md`.

## ANY DEFINED BY mappings

Read [ANY DEFINED BY configuration](any-defined-by.md) when a schema's
payload type is selected by another member. It covers the TOML format,
name resolution, nested paths, and the repository's runnable examples.
Pass mappings to `compile` or `generate`; convert using the resulting
`.asni` so that the mappings are retained.
