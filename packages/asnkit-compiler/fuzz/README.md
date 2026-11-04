# Fuzzing the compiler

Specifications that are anything are to be a `ParseError`, a
`CompileError` or compiled, never a panic, a stack that runs out, or no
end. Each run parses the text to an `.asnst`, compiles it to an `.asni`,
and loads that with the Rust runtime, lowered for every binary codec.

- `compile`: the input is the whole text, as a file would be.
- `compile_types`: the input is put in
  `Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN ... END`, so that the fuzzer
  spends its time on the assignments, where the compiler follows
  references.

With `ASNKIT_FUZZ_STRICT=1`, what the compiler takes and the runtime
refuses to load or lower is a failure too.

It needs [cargo-fuzz](https://github.com/rust-fuzz/cargo-fuzz) and a
nightly Rust, or a stable one with `RUSTC_BOOTSTRAP=1`:

```sh
cd packages/asnkit-compiler
python3 fuzz/seed.py
RUSTC_BOOTSTRAP=1 cargo fuzz run -O compile_types fuzz/corpus/compile_types -- \
    -dict=fuzz/asn1.dict -timeout=10 -max_len=32768
```

`-fork=N -ignore_crashes=1 -ignore_timeouts=1` runs on N cores and goes
on after what it finds. What is found is in `fuzz/artifacts/`, and is
run again with the program and the file:

```sh
fuzz/target/x86_64-unknown-linux-gnu/release/compile_types fuzz/artifacts/compile_types/crash-...
```

What it found is tested in `tests/fuzzed.rs`.
