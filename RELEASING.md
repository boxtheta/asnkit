# Releasing asnkit

All packages are released together, with one version, by pushing a tag
`v1.2.3`. `.github/workflows/release.yml` then tests everything,
publishes the Python packages to PyPI, the crates to crates.io and the
WebAssembly runtime to npm, and makes a release on GitHub with the
notes of `CHANGELOG.md`. Nothing is published unless all of it builds
and passes its tests.

## Once, before the first release

- **GitHub environments:** create `pypi`, `crates-io` and `npm` in the
  settings of the repository, and let only tags `v*` deploy to them.
  Add required reviewers to have each release approved before it is
  published.
- **PyPI:** add a pending trusted publisher for `asnkitrt` and for
  `asnkit-gen-python`: owner `boxtheta`, repository `asnkit`, workflow
  `release.yml`, environment `pypi`. No token is needed.
- **crates.io:** make an API token limited to publishing and to the
  crates `asnkitrt`, `asnkit-compiler`, `asnkit-gen-rust` and
  `asnkitcc`, and store it as the secret `CARGO_REGISTRY_TOKEN` of the
  environment `crates-io`. Once the crates exist, trusted publishing
  can replace the token.
- **npm:** make a granular access token that can publish `asnkitrt`,
  and store it as the secret `NPM_TOKEN` of the environment `npm`.
  Once the package exists, trusted publishing can replace the token.
- **DCO:** install the [DCO app](https://github.com/apps/dco) on the
  repository, and make its check required on `trunk`, so that every
  commit of a pull request is signed off (see `CONTRIBUTING.md`).

## Each release

1. Set the version in every package. `tests/test_versions.py` lists
   the files and fails until they all agree:

   - `packages/asnkitrt/pyproject.toml` and
     `packages/asnkitrt/src/asnkitrt/version.py`
   - `packages/asnkit-gen-python/pyproject.toml`, its dependency on
     `asnkitrt`, and `packages/asnkit-gen-python/src/asnkit_gen_python/version.py`
   - the `Cargo.toml` of each crate, and the versions of the
     dependencies of `asnkitcc` and `asnkit-gen-rust` on the others
   - `packages/asnkitrt-wasm/package.json`

2. Make everything again that names the version of the compiler or of a
   plugin: the fixtures, the compiled examples, and the code generated
   from them. CI checks each of them, but only when it compiles them
   again, which the tests on your machine do not.

   - `tests/fixtures`: compile them into an empty folder and copy them
     over, as the job `fixtures-are-current` does:

     ```text
     cargo build --bin asnkitcc
     mkdir /tmp/fixtures
     ASNKITCC="$PWD/target/debug/asnkitcc" ASNKIT_FIXTURES=/tmp/fixtures uv run python -m unittest
     cp /tmp/fixtures/* tests/fixtures/
     ```

   - `examples/compiled`: the `.asnc` and `.asnst` files next to each
     `.asn`, which `test_examples` in `tests/test_files.py` holds to
     what the compiler makes.
   - `examples/generated/foo.py`, as `tests/test_gen_python.py` tells
     when it fails.
   - The Rust code: `UPDATE_GENERATED=1 cargo test -p asnkit-gen-rust --test generated`.
   - `packages/asnkit-gen-rust/tests/no_std/Cargo.lock`, a crate of its
     own: `cargo update -p asnkitrt` in its folder.

3. In `CHANGELOG.md`, give the section of the version its date
   (`## [1.2.3] - 2026-10-15`) and add the link at the end.

4. Run all the checks of `CONTRIBUTING.md`, commit, and merge to `trunk`.

5. Tag the merge commit and push the tag:

   ```text
   git tag -s v1.2.3 -m "asnkit 1.2.3"
   git push origin v1.2.3
   ```

6. Follow the workflow in the Actions tab, and approve the environments
   if they need it. If a registry fails, fix it and run the failed jobs
   again: each leaves out what is already published.
