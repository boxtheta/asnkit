# Releasing asnkit

All packages are released together, with one version, by pushing a tag
`v1.2.3`. `.github/workflows/release.yml` then tests everything,
publishes the Python packages to PyPI, the crates to crates.io and the
WebAssembly runtime and compiler to npm, and makes a release on GitHub with the
notes of `CHANGELOG.md`. Nothing is published unless all of it builds
and passes its tests.

Release candidates use tags such as `v2.0.0-rc1`. Cargo and npm use
`2.0.0-rc1`; Python package metadata and `__version__` use the equivalent
PEP 440 spelling `2.0.0rc1`. The version tests compare these spellings.
The workflow publishes prereleases to npm's `next` tag and marks the
GitHub release as a prerelease. Stable releases use npm's `latest` tag.

`asnkit-gen-rust-codec` is included after `asnkit-gen-rust` in the publish
list. Before its first automated release, publish it once and configure
its crates.io trusted publisher as below. For this new crate, its
`asnkitrt` and `asnkit-gen-rust` dependency versions must already be
published. If the first release workflow stops at this crate, publish
it with `cargo publish --locked -p asnkit-gen-rust-codec` from the tagged
checkout using your crates.io credentials, configure its trusted
publisher, then rerun the failed jobs. The workflow skips versions
already published.

## Once, before the first release

- **GitHub environments:** create `pypi`, `crates-io` and `npm` in the
  settings of the repository, and let only tags `v*` deploy to them.
  Add required reviewers to have each release approved before it is
  published.
- **PyPI:** add a pending trusted publisher for `asnkitrt` and for
  `asnkit-gen-python`: owner `boxtheta`, repository `asnkit`, workflow
  `release.yml`, environment `pypi`. No token is needed.
- **crates.io:** publish the crates once by hand, then add a trusted
  publisher to each of `asnkitrt`, `asnkit-compiler`, `asnkit-gen-rust`,
  `asnkit-gen-rust-codec` and `asnkitcc`: owner `boxtheta`, repository `asnkit`, workflow
  `release.yml`, environment `crates-io`. No token is needed.
- **npm:** publish `asnkitrt` and `asnkit-compiler` once by hand, then
  add a trusted publisher to each: GitHub Actions, owner `boxtheta`, repository `asnkit`, workflow
  `release.yml`, environment `npm`. No token is needed.
- **DCO:** install the [DCO app](https://github.com/apps/dco) on the
  repository, and make its check required on `trunk`, so that every
  commit of a pull request is signed off (see `CONTRIBUTING.md`).

## Each release

1. Choose the version, as `docs/versioning.md` says, from what the
   section `[Unreleased]` of `CHANGELOG.md` lists: a major release if
   it breaks the public API, a minor release if it changes what is
   encoded or decoded or adds to the API, and a patch release
   otherwise.

2. Set the version in every package. `tests/test_versions.py` lists
   the files and fails until they all agree:

   - `packages/asnkitrt/pyproject.toml` and
     `packages/asnkitrt/src/asnkitrt/version.py`
   - `packages/asnkit-gen-python/pyproject.toml`, its dependency on
     `asnkitrt`, and `packages/asnkit-gen-python/src/asnkit_gen_python/version.py`
   - the `Cargo.toml` of each crate, and the versions of the
     dependencies of `asnkitcc`, `asnkit-gen-rust` and
     `asnkit-gen-rust-codec` on the others
   - `packages/asnkitrt-wasm/package.json` and
     `packages/asnkit-compiler-wasm/package.json`

3. Make everything again that names the version of the compiler or of a
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

   - `examples/compiled`: the `.asni` and `.asnst` files next to each
     `.asn`, which `test_examples` in `tests/test_files.py` holds to
     what the compiler makes.
   - `examples/generated/foo.py`, as `tests/test_gen_python.py` tells
     when it fails.
   - The Rust code: `UPDATE_GENERATED=1 cargo test -p asnkit-gen-rust --test generated`,
     and the same with `-p asnkit-gen-rust-codec`.
   - `packages/asnkit-gen-rust/tests/no_std/Cargo.lock`,
     `packages/asnkit-gen-rust-codec/tests/no_std/Cargo.lock` and
     `packages/asnkit-gen-rust-codec/tests/vectors/Cargo.lock`, crates
     of their own: `cargo update -p asnkitrt` in each folder, and
     `-p asnkit-gen-rust-codec` too in the last.

4. In `CHANGELOG.md`, give the section of the version its date
   (`## [1.2.3] - 2026-10-15`) and add the link at the end.

5. Run all the checks of `CONTRIBUTING.md`, commit, and merge to `trunk`.

6. Tag the merge commit and push the tag:

   ```text
   git tag -s v1.2.3 -m "asnkit 1.2.3"
   git push origin v1.2.3
   ```

7. Follow the workflow in the Actions tab, and approve the environments
   if they need it. If a registry fails, fix it and run the failed jobs
   again: each leaves out what is already published.
