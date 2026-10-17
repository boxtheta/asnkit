# Contributing to asnkit

Thank you for helping. Bug reports, questions and ideas are welcome as
GitHub issues. A vulnerability is not: report it as `SECURITY.md` says.
Everyone who takes part is expected to follow the
[Code of Conduct](CODE_OF_CONDUCT.md).

## Developer Certificate of Origin

asnkit is licensed under the Apache License 2.0 and the MIT License,
each for its own part of the repository. A
contribution is under the license of the files it changes, as section
5 of the Apache License says for the parts under it.

Every commit has to be signed off, to certify that you wrote it or
otherwise have the right to contribute it under those licenses, as the
[Developer Certificate of Origin](https://developercertificate.org/)
(DCO) says. Sign off by committing with `-s`, which adds a line with
the name and email of your Git configuration:

```text
git commit -s -m "Fix the decoding of empty SET OF in OER"
```

```text
Signed-off-by: Your Name <you@example.com>
```

A check on the pull request fails if a commit is not signed off. To
sign off commits you already made, run `git rebase --signoff trunk` and
push again. If you contribute on behalf of your employer, make sure
that you are allowed to.

## Making a change

1. Fork the repository.

2. Install the prerequisites: `uv` for the Python packages, and Rust
   1.88 or later for the crates and the compiler. Then run:

   ```text
   uv sync
   ```

3. Make your change, and add tests that cover it.

4. Run the Python tests and the type checker:

   ```text
   uv run python -m unittest
   uv run mypy
   ```

   The tests use compiled specifications from `tests/fixtures`, so they
   do not need the compiler. If a test uses a specification that has no
   fixture yet, it is compiled when `asnkitcc` is built
   (`cargo build --bin asnkitcc`). Commit the new fixture.

5. Run the Rust tests, and keep the code formatted and free of warnings:

   ```text
   cargo fmt --all
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   ```

   The tests of the C runtime, `packages/asnkitrt-c`, compile C with
   `cc`, or the compiler `CC` names, on Linux and macOS.

   Also test the Rust runtime without its default features, as embedded
   targets build it:

   ```text
   cargo test -p asnkitrt --no-default-features
   ```

   If you changed the Rust runtime, also build and test the WebAssembly
   runtime, as its README describes:

   ```text
   cd packages/asnkitrt-wasm && npm run build && npm test
   ```

   If you changed the compiler, also build and test the WebAssembly
   compiler, whose tests load what it makes with the WebAssembly
   runtime, built as above:

   ```text
   cd packages/asnkit-compiler-wasm && npm run build && npm test
   ```

6. If you changed the documentation, build it. It is written in Markdown
   and built with MkDocs, and warnings count as errors:

   ```text
   uv run --group docs mkdocs build --strict
   ```

7. If you changed the Rust codecs, or the tests of the Python codecs
   that the cases come from, record the vectors again. The cases are
   taken from the Python tests and the results from the Rust runtime,
   and the Python runtime is then tested against them:

   ```text
   uv run python tests/vectors/record.py
   ```

8. If your change could make the runtimes or the compiler slower, run
   the benchmarks before and after it, on the same machine and with as
   little else running as you can. CI does not run them, as its shared
   runners vary too much between runs:

   ```text
   cargo bench -p asnkitrt --bench runtime
   cargo bench -p asnkit-compiler --bench compiler
   uv run python tests/benchmarks.py
   ```

   Each prints the median time of each benchmark and its median
   deviation, and takes names to filter on, as `cargo bench -p asnkitrt
   --bench runtime -- uper`. To keep the results, set `ASNKIT_BENCH_OUT`
   to a directory that exists, and each writes them there as JSON. Give
   an absolute path, as cargo runs each benchmark from its package's
   directory:

   ```text
   git switch trunk
   mkdir -p ../bench/before
   ASNKIT_BENCH_OUT="$PWD/../bench/before" cargo bench -p asnkitrt --bench runtime
   git switch my-branch
   mkdir -p ../bench/after
   ASNKIT_BENCH_OUT="$PWD/../bench/after" cargo bench -p asnkitrt --bench runtime
   ```

   A difference of a few percent is usually noise; run the benchmark
   again before you trust it.

9. Open a pull request, with every commit signed off (see
   [Developer Certificate of Origin](#developer-certificate-of-origin)).

## Things that are easy to get wrong

- **Fixtures:** the Python tests take `asnkitcc` from `target/release`
  before `target/debug`. After a compiler change, run
  `cargo build --release --bin asnkitcc` before making `tests/fixtures`
  again, or the fixtures come from the old compiler.
- **Golden cases:** a compiler change that changes its output on purpose
  updates the cases in `packages/asnkit-compiler/tests/golden/cases`
  in the same commit. Keep their format (compact JSON with sorted keys,
  some gzipped), check that only the intended fields changed, and update
  `sha256` when a source file changes.
- **The table of kinds** of nodes is in `docs/file_formats.md` and,
  copied, in `packages/asnkit-compiler/src/kinds.md`. Change both.
- **`examples/asni.asn`** describes the node tree, so a change to the
  shape of a node changes it, its converter in `tests/test_asni_asn.py`
  and its golden case.
- **Generated code:** after a change to `asnkit-gen-rust`, make its
  files again with
  `UPDATE_GENERATED=1 cargo test -p asnkit-gen-rust --test generated`.

## Releases

All packages are released together with one version; see `RELEASING.md`.
