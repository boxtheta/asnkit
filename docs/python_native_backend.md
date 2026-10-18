# A native backend for the runtime for Python

> **Note:** This is a plan. Nothing of it is implemented yet.

The runtime for Python, `asnkitrt` on PyPI, is written in Python, and
the runtime for Rust is the one of reference. The plan is for
`asnkitrt` to encode and decode with the runtime for Rust where it can,
through bindings in a package of their own, and with its own code
everywhere else.

## Why, and why not only the bindings

What this is for:

- Speed, above all for large values, as certificates and 3GPP messages.
- `pip install asnkitrt` keeps working wherever Python runs, with
  nothing to build: Pyodide, PyPy, free-threaded builds, and the
  platforms there are no wheels for.
- The runtime for Python stays an implementation of its own, held to
  the vectors that the runtime for Rust records. Two implementations
  that have to agree find the bugs of each other.

What it is not for: having one implementation to maintain. There are
still two, and the bindings between them. Bindings only, with no
fallback, would have had one, and lost the rest.

## The two packages

- `asnkitrt`: as now, in pure Python, with no dependencies. It has the
  public API, and chooses the backend.
- `asnkitrt-native`: the bindings, with PyO3 and maturin, as wheels for
  `abi3-py313`, one for each platform and every CPython from 3.13. It
  has no public API: only `asnkitrt` imports it.

`asnkitrt` depends on `asnkitrt-native` only where there are wheels, by
environment markers, so that pip never has to build it:

```text
asnkitrt-native == 1.2.0 ; platform_python_implementation == "CPython"
    and ((sys_platform == "linux" and platform_machine in "x86_64 aarch64")
    or (sys_platform == "darwin" and platform_machine in "x86_64 arm64")
    or (sys_platform == "win32" and platform_machine in "AMD64 ARM64"))
```

The version is the same, as every package of asnkit is released with
one version. There is no marker for a free-threaded build, which can
not load an `abi3` wheel, so `asnkitrt` falls back to Python there if
the import fails; see [Open questions](#open-questions).

Making it one package, a pure `py3-none-any` wheel next to the wheels
of the platforms, would also do, but needs maturin and `uv_build` to
make one distribution, and its sdist would need Rust.

## Choosing the backend

For each `Specification`, when it is loaded:

1. Python, if `ASNKIT_PURE=1` is set, or `load(..., backend="python")`
   is given.
2. Else the native backend, if `asnkitrt_native` is imported, and it
   supports the codec.
3. Else Python.

`backend="native"` fails if the native backend can not be had, for
tests and for those who need the speed. `Specification.backend` tells
which one it got, and `asnkitrt.native_version()` which bindings are
installed, or `None`, for bug reports.

What `load()` reads stays in Python: `syntax.from_asni()` checks the
container, the signature, the encryption, the nonce and the binding,
with `cryptography`, as now. The native backend is given the nodes it
returns, as CBOR (`Specification::from_nodes()` of the runtime for
Rust), so that there is one place where files are checked, and keys
are given as they are now.

## The API of the native backend

Small, and private to `asnkitrt`:

```python
spec = asnkitrt_native.load(nodes: bytes, codec: str, numeric_enums: bool, lenient: bool)
spec.encode(name, value, module=None, indent=None, check_constraints=False) -> bytes
spec.decode(name, data, module=None, check_constraints=False) -> object
spec.decode_with_length(name, data) -> tuple[object, int]
spec.decode_length(data) -> int | None
spec.check_constraints(name, value, module=None) -> None
```

### Values

A value goes in and out as the Python backend has it: a `dict` for a
SEQUENCE and a SET, a `tuple` of a name and a value for a CHOICE, a
`tuple` of bytes and a number of bits for a BIT STRING, `int` of any
size, `float`, `str`, `bytes`, `None`, and `datetime.date`,
`datetime.time` and `datetime.datetime`, with an offset from their
`tzinfo`.

The inputs it accepts are those the type checker of the Python backend
(`codecs/type_checker.py`) accepts, and no others: `bytearray` and
`memoryview` as `bytes`, a `list` as a `tuple`, `bool` not as an
`int`, and so on. This is written down as a table, and tested on both
backends.

### Errors

`EncodeError`, `DecodeError` and `ConstraintsError` of
`asnkitrt.errors`, with the offset of a `DecodeError`. Their messages
differ: the Python backend says where the error is, as `A: Sequence
member 'a' not found in {}.`, and the runtime for Rust does not, yet.
See [Before the bindings](#1-before-the-bindings).

## What only the Python backend has

| What | With the native backend |
|---|---|
| `Specification.types`, `.modules` | Made by the Python backend when first asked for, which loads it then |
| `check_types=False` | Accepted, and does nothing: the native backend checks anyway, and fast |
| Messages that say where the error is | The same, with the indexes of elements too, as `extensions[1]` |

What neither has, as decoding GSER, is `Unsupported` on both.

## The rules from then on

- The runtime for Rust leads. A codec or a feature is done in Rust
  first, and in the bindings with it.
- The Python backend may lag, and then raises `Unsupported`, as the
  runtime for Rust does for what it does not do. It never does
  something else instead.
- The vectors are the contract. Both backends pass every vector the
  other supports, as both runtimes do now.
- What may differ between the backends: the text of error messages,
  and speed. Nothing else.

## Steps

### 0. Measure

Before anything is kept: bindings made quickly, to know

- the time of `encode()` and `decode()` on both backends, for small
  values (`Question` of `foo.asni`) and large ones (a certificate of
  `tests/fixtures/x509`, a 3GPP message of the vectors). On small
  values the conversion of the values may cost more than it saves;
- that the vectors pass through them;
- what the table above is missing.

It goes on if the native backend is faster for large values and not
slower for small ones. Else it stops here.

### 1. Before the bindings

In the runtime for Rust, for every runtime made from it:

- Errors that say where they are: the path of members, as the Python
  backend has. `Error` is `Copy` with a `&'static str` now, so this is
  a change of the API, or a type of its own for errors with a path;
  see [Open questions](#open-questions).
- `Codec` and the other public enums `#[non_exhaustive]`, at the next
  major version, so that a codec can be added without breaking those
  who match on them, as adding `Codec::Gser` would have.

### 2. `asnkitrt-native`

- `packages/asnkitrt-native`: the crate with PyO3, and its
  `pyproject.toml` for maturin, with `abi3-py313`.
- The conversion of values, the table of what is accepted, and the
  errors, with tests of their own.
- The whole suite of `asnkitrt`, run with `backend="native"`.

### 3. In `asnkitrt`

- The choice of the backend, `ASNKIT_PURE`, `backend=` and
  `Specification.backend`.
- `.types` and `.modules` that load the Python backend when asked for.
- `tests/test_vectors.py` and the rest of the suite run with each
  backend.

### 4. CI and the release

- `python-package.yml`: the suite twice, with `ASNKIT_PURE=1` and
  without, and the native backend built from the checkout.
- `release.yml`: the wheels, with `maturin-action`, for Linux
  (manylinux and musllinux, x86_64 and aarch64), macOS (x86_64 and
  arm64) and Windows (AMD64 and ARM64), and an sdist for those who
  package it. Each wheel is installed and run on part of the vectors on
  its platform before anything is published.
- `uv build --all-packages` does not build the native package, so it
  is built in a job of its own, and published with `asnkitrt`.

### 5. Out

- First release: `asnkitrt-native` as an extra,
  `pip install asnkitrt[native]`, and Python by default. Bugs are found
  by those who ask for it.
- The next one: the dependency with the markers above, and the native
  backend by default where it is installed.
- The README, the table of codecs, the CHANGELOG and
  [Third-party runtimes](third_party_runtimes.md) say what is
  installed where, and how to have the Python backend.

## Open questions

- **Errors with a path in Rust:** a change of `Error` at 2.0, or a
  second error type and methods that return it in 1.x.
- **Free-threaded Python:** wheels for `cp313t` and `cp314t` too, or
  the Python backend there. Nothing in the markers tells a
  free-threaded build apart.
- **The checks of files in Rust:** the native backend could check the
  container itself, and those who have it would not need
  `cryptography`. Keys would then be given to Rust, as they are to the
  runtime for WebAssembly.
- **How far the Python backend follows:** every new feature, or only
  fixes, with `Unsupported` for the rest.
- **The bar of step 0:** how much faster, for which values.
