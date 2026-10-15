# asnkit-compiler (Rust)

The asnkit compiler as a library, for an application to compile ASN.1
itself. It parses ASN.1 and writes `.asnc` files, which a runtime loads
to encode and decode without parsing anything itself, and `.asnst`
files, the syntax tree that plugins generate code from, and it runs
plugins. Its output is checked against the cases in `tests/golden`
of the [repository](https://github.com/boxtheta/asnkit).

```rust
use asnkit_compiler::container::Protection;
use asnkit_compiler::{Input, compile_to_asnc};

let input = Input::String(std::fs::read_to_string("foo.asn")?);
let asnc = compile_to_asnc(&input, None, None, &Protection::default())?;

std::fs::write("foo.asnc", asnc)?;
```

The command [`asnkitcc`](https://crates.io/crates/asnkitcc) is made
with it.

Licensed under the Apache License 2.0 (`LICENSE-COMPILER`). What it
produces is yours to use under terms of your choice; see
[`COPYING`](https://github.com/boxtheta/asnkit/blob/main/COPYING).
