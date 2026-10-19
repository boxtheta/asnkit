# asnkit-compiler (Rust)

The asnkit compiler as a library, for an application to compile ASN.1
itself. It parses ASN.1 and writes `.asni` files, which a runtime loads
to encode and decode without parsing anything itself, and `.asnst`
files, the syntax tree that plugins generate code from, and it runs
plugins. Its output is checked against the cases in `tests/golden`
of the [repository](https://github.com/boxtheta/asnkit).

```rust
use asnkit_compiler::container::Protection;
use asnkit_compiler::{Input, compile_to_asni};

let input = Input::String(std::fs::read_to_string("foo.asn")?);
let asni = compile_to_asni(&input, None, None, &Protection::default())?;

std::fs::write("foo.asni", asni)?;
```

The command [`asnkitcc`](https://crates.io/crates/asnkitcc) is made
with it.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice.

## ANY DEFINED BY choices

`CompileOptions::from_toml()` reads explicit selector-to-type mappings.
Pass the options to `compile_with_options()`,
`compile_to_asni_with_options()` or
`compile_to_asni_with_summary_and_options()`. The original functions
continue to compile without mappings. Options can also be constructed
with `AnyDefinedBy` entries directly. See
[ANY DEFINED BY](../../docs/any_defined_by.md) for the configuration and
runtime behavior.
