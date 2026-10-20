# asnkitrt (C)

The asnkit runtime for C (and anything that calls C). It is the Rust
runtime built as a shared and a static library, `libasnkitrt_c`, with
the header [`include/asnkitrt.h`](include/asnkitrt.h). It loads the
`.asni` files produced by `asnkitcc` and uses them to encode and decode
ASN.1, without parsing ASN.1 itself.

The codecs are BER, DER, OER, PER, UPER, JER and XER. It can also check
values against their constraints, and load signed, encrypted and bound
`.asni` files.

```c
#include "asnkitrt.h"

asnkit_spec *foo;
asnkit_value *question = asnkit_value_new_map();
uint8_t *data;
size_t length;

if (asnkit_spec_load_file("foo.asni", "ber", NULL, &foo) != ASNKIT_OK) {
    fprintf(stderr, "%s\n", asnkit_last_error());
    return 1;
}

asnkit_map_set(question, "id", asnkit_value_new_integer(1));
asnkit_map_set(question, "question", asnkit_value_new_text("Is 1+1=3?", 9));

if (asnkit_encode(foo, "Question", question, NULL, &data, &length) == ASNKIT_OK) {
    /* 30 0e 02 01 01 16 09 49 73 20 31 2b 31 3d 33 3f */
    asnkit_bytes_free(data, length);
}

asnkit_value_free(question);
asnkit_spec_free(foo);
```

[`examples/question.c`](examples/question.c) is the whole of it,
decoding too, with JER and an error.

## Building and linking

Build it in the [repository](https://github.com/boxtheta/asnkit) with
Cargo, which writes the libraries to `target/release`:

```text
cargo build --release -p asnkitrt-c
```

- **Shared**: `libasnkitrt_c.so` on Linux, `libasnkitrt_c.dylib` on
  macOS (its install name is `@rpath/libasnkitrt_c.dylib`) and
  `asnkitrt_c.dll` on Windows. Link with `-lasnkitrt_c`, and give the
  program a runpath to it, for example `-Wl,-rpath,/path/to/lib`.
- **Static**: `libasnkitrt_c.a`, or `asnkitrt_c.lib` on Windows. It
  needs the system libraries that the Rust standard library uses,
  which this prints:

  ```text
  cargo rustc --release -p asnkitrt-c --crate-type staticlib -- --print native-static-libs
  ```

  They are `-lSystem -lc -lm` on macOS, and
  `-lgcc_s -lutil -lrt -lpthread -lm -ldl -lc` on Linux with glibc.

The example, linked with the shared library:

```text
cc -I packages/asnkitrt-c/include packages/asnkitrt-c/examples/question.c \
    -L target/release -lasnkitrt_c -Wl,-rpath,"$PWD/target/release" -o question
./question examples/compiled/foo.asni
```

The header is C99, and C++ can include it.

## Values

A value is an opaque handle, `asnkit_value *`, to a value of the Rust
runtime, which is built with `asnkit_value_new_*()`,
`asnkit_list_push()` and `asnkit_map_set()`, and read with
`asnkit_value_kind()` and an accessor for each kind. The kinds are those
of the Python runtime, and so of the vectors that all the runtimes are
held to, but for `OBJECT IDENTIFIER`, which is a `str` there:

| ASN.1 | Kind | Made with | Read with |
|---|---|---|---|
| `NULL` | `ASNKIT_NULL` | `asnkit_value_new_null()` | |
| `BOOLEAN` | `ASNKIT_BOOLEAN` | `asnkit_value_new_boolean()` | `asnkit_value_boolean()` |
| `INTEGER`, `ENUMERATED` with numeric enums | `ASNKIT_INTEGER` | `asnkit_value_new_integer()`, or `_integer_bytes()` for any size | `asnkit_value_integer()`, `_integer_bytes()` |
| `REAL` | `ASNKIT_REAL` | `asnkit_value_new_real()` | `asnkit_value_real()` |
| character strings, `ENUMERATED` by name, `DURATION` | `ASNKIT_TEXT` | `asnkit_value_new_text()` | `asnkit_value_text()` |
| `OBJECT IDENTIFIER` (`"1.2.3"`) | `ASNKIT_OBJECT_IDENTIFIER` | `asnkit_value_new_object_identifier()` | `asnkit_value_object_identifier()`, `_object_identifier_bytes()` |
| `OCTET STRING`, `ANY`, open types | `ASNKIT_BYTES` | `asnkit_value_new_bytes()` | `asnkit_value_bytes()` |
| `BIT STRING` | `ASNKIT_BITS` | `asnkit_value_new_bits()` | `asnkit_value_bits()` |
| `SEQUENCE OF`, `SET OF` | `ASNKIT_LIST` | `asnkit_value_new_list()`, `asnkit_list_push()` | `asnkit_value_length()`, `asnkit_list_get()` |
| `SEQUENCE`, `SET`, `EXTERNAL` | `ASNKIT_MAP` | `asnkit_value_new_map()`, `asnkit_map_set()` | `asnkit_map_get()`, `asnkit_value_length()`, `asnkit_map_member()` |
| `CHOICE` | `ASNKIT_CHOICE` | `asnkit_value_new_choice()` | `asnkit_choice_get()` |
| `UTCTime`, `GeneralizedTime`, `DATE-TIME` | `ASNKIT_DATE_TIME` | `asnkit_value_new_date_time()` | `asnkit_value_date_time()` |
| `DATE` | `ASNKIT_DATE` | `asnkit_value_new_date()` | `asnkit_value_date()` |
| `TIME-OF-DAY` | `ASNKIT_TIME` | `asnkit_value_new_time()` | `asnkit_value_time()` |

A map holds the members that are present, in order. A time's `offset`
is in minutes east of UTC, and `has_offset` is zero for a local time.

Why handles, and not JSON or structs of C:

- **JSON** in and out would need a JSON library in every program, and
  would lose what JSON can not say without the type at hand: an
  `INTEGER` of any size, the bytes of an `OCTET STRING`, a `BIT STRING`
  of a number of bits, a `CHOICE` or a time. Where JSON is what is
  wanted, it is already there: load the same `.asni` file for `jer`
  too, and decode with one specification and encode with the other.
  That is JSON as X.697 says, and the example does it.
- **Structs of C**, such as a tagged union, would fix the layout of
  every value in the ABI, and would need C to allocate and free
  them as the runtime does. A handle keeps the layout of the runtime
  its own, so that it can change in a minor version.

The handles are cheap: what an accessor gives is borrowed from the value
it is in, with no copy, and a decoded value is built once, by the
codec.

## Errors and memory

- Every function returns an `asnkit_status`, `ASNKIT_OK` or an error,
  or a handle that is `NULL` on an error. The message of the error is
  `asnkit_last_error()`, of the thread it happened on, and
  `asnkit_last_error_offset()` tells where a decoding error is, in
  bytes, or in bits for PER and UPER. The statuses are as the errors of
  the other runtimes: `ASNKIT_ERROR_ENCODE`, `_DECODE`, `_CONSTRAINTS`,
  `_UNSUPPORTED`, `_NO_SUCH_TYPE` and `_LOAD`, and
  `ASNKIT_ERROR_ARGUMENT`, `_KIND`, `_RANGE` and `_IO` for what C
  gives.
- What a function gives is owned by the caller and freed with the
  function named for it: `asnkit_spec_free()`, `asnkit_value_free()`,
  `asnkit_load_options_free()` and `asnkit_bytes_free()`. A value
  given to a list, a map or a choice is owned by it from then on, even
  if the call fails. What an accessor gives is borrowed, and valid as
  long as the value it is from is and is not changed.
- Text that comes from a value has its length and no NUL at its end, as
  it may have one in it; print it with `"%.*s"`. What `asnkit_encode()`
  gives is followed by a NUL that its length does not count, so that
  JER and XER are also a string of C.
- No panic gets into C: one is caught and returned as
  `ASNKIT_ERROR_PANIC`, which is a bug to report. Decoding untrusted
  input never panics, as in the Rust runtime.
- A specification may be used from several threads at once. A value
  may be read from several threads, and changed from one only.

## Signed, encrypted and bound files

By default, only a file that is not signed, not encrypted, with no nonce
and not bound is loaded. An `asnkit_load_options` says what else is
expected: `asnkit_load_options_add_verification_key()` and
`_add_decryption_key()`, which take an id of `ASNKIT_KEY_ID_SIZE` bytes
and a key of `ASNKIT_KEY_SIZE` bytes, `_set_bound_to()`,
`_set_nonce()` or `_set_nonce_callback()`, and the `_accept_any_*()`
functions, as `asnkitrt.load_asni()` of the Python runtime has them. See
[`docs/file_formats.md`](https://github.com/boxtheta/asnkit/blob/trunk/docs/file_formats.md).

## Testing

```text
cargo test -p asnkitrt-c
```

`tests/c.rs` compiles `examples/question.c` and `tests/api.c` with `cc`
(or `CC`) and runs them against the shared library, which needs Linux
or macOS. `tests/header.rs` holds the header, which is written by hand,
to the functions and constants of `src/`.

It supports what the Rust runtime supports; see
[its README](https://github.com/boxtheta/asnkit/blob/trunk/packages/asnkitrt-rs/README.md#limitations) for the
limitations.

Licensed under the MIT License. See `LICENSE-RUNTIMES`.
