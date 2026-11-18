# asnkitrt (C and C++)

The asnkit runtime for C and C++ (and anything that calls C). It is the
Rust runtime built as a shared and a static library, `libasnkitrt_c`,
with the header [`include/asnkitrt.h`](include/asnkitrt.h), and
[`include/asnkitrt.hpp`](include/asnkitrt.hpp) for C++. It loads the
`.asni` files produced by `asnkitcc` and uses them to encode and decode
ASN.1, without parsing ASN.1 itself.

The codecs are BER, DER, OER, PER, UPER, JER, XER and GSER. It can also check
values against their constraints.

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

In C++, values and specifications free themselves and errors are
thrown:

```cpp
#include "asnkitrt.hpp"

try {
    auto foo = asnkit::spec::load_file("foo.asni", "ber");
    auto question = asnkit::value::map({{"id", 1}, {"question", "Is 1+1=3?"}});
    std::vector<std::uint8_t> data = foo.encode("Question", question);
    // 30 0e 02 01 01 16 09 49 73 20 31 2b 31 3d 33 3f

    asnkit::value decoded = foo.decode("Question", data);
    std::cout << decoded["question"].as_text() << "\n";
} catch (const asnkit::error &error) {
    std::cerr << error.what() << "\n";
    return 1;
}
```

[`examples/question.cpp`](examples/question.cpp) is the same example
as `question.c`, in C++.

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

And in C++:

```text
c++ -std=c++17 -I packages/asnkitrt-c/include packages/asnkitrt-c/examples/question.cpp \
    -L target/release -lasnkitrt_c -Wl,-rpath,"$PWD/target/release" -o question
./question examples/compiled/foo.asni
```

`asnkitrt.h` is C99, and C++ can include it as it is. `asnkitrt.hpp`
is C++17 or later, and only a header: it includes `asnkitrt.h`, calls
its functions and links the same library.

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
  `asnkit_last_error()`, of the thread it happened on, and its status
  `asnkit_last_error_status()`, which tells which error a function
  that returned `NULL` had. `asnkit_last_error_offset()` tells where a
  decoding error is, in bytes, or in bits for PER and UPER. The
  statuses are as the errors of the other runtimes:
  `ASNKIT_ERROR_ENCODE`, `_DECODE`, `_CONSTRAINTS`, `_UNSUPPORTED`,
  `_NO_SUCH_TYPE` and `_LOAD`, and
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

## C++

`asnkitrt.hpp` is `asnkitrt.h` for C++17 and later, in the namespace
`asnkit`, and its comments are its documentation. A specification is
an `asnkit::spec`, from `spec::load()` or `spec::load_file()`, with
`encode()`, `decode()`, `encode_text()` and `decode_text()` for JER,
XER and GSER, `decode_with_length()`, `decode_length()` and
`check_constraints()`. They take an `asnkit::options`, of `module`,
`check_constraints` and `indent`, and loading takes an
`asnkit::load_options`, of `numeric_enums` and `lenient`.

A value is an `asnkit::value`. It is made from what C++ has, which
picks its kind, or with the function named for the kind, and read with
the accessor of its kind; `kind()` and `is_text()` and the like tell
what it is.

| Kind | Made with | Read with |
|---|---|---|
| `kind::null` | `value()`, `value::null()` | |
| `kind::boolean` | `value(true)`, `value::boolean()` | `as_boolean()` |
| `kind::integer` | `value(1)`, of any integral type, `value::integer()`, `value::integer_bytes()` for any size | `as_integer()`, `as_unsigned()`, `as_integer_bytes()` |
| `kind::real` | `value(0.5)`, `value::real()` | `as_real()` |
| `kind::text` | `value("text")`, of a `std::string_view` or a `std::string` too, `value::text()` | `as_text()` |
| `kind::object_identifier` | `value::object_identifier("1.2.3")` | `as_object_identifier()`, `as_object_identifier_bytes()` |
| `kind::bytes` | `value::bytes()` | `as_bytes()` |
| `kind::bits` | `value::bits()` | `as_bits()` |
| `kind::list` | `value::list({1, 2})`, `value::list().push(1)` | `size()`, `at()`, `[]`, `items()` |
| `kind::map` | `value::map({{"id", 1}})`, `value::map().set("id", 1)` | `at()`, `[]`, `find()`, `contains()`, `size()`, `member()`, `members()` |
| `kind::choice` | `value::choice()` | `as_choice()` |
| `kind::date_time` | `value::date_time()` | `as_date_time()` |
| `kind::date` | `value::date()` | `as_date()` |
| `kind::time` | `value::time()` | `as_time()` |

```cpp
asnkit::value list = asnkit::value::list().push(1).push(2);

for (asnkit::value_view item : list.items()) {
    std::cout << item.as_integer() << "\n";
}

for (auto [name, member] : decoded.members()) {
    std::cout << name << " is of kind " << static_cast<int>(member.kind()) << "\n";
}
```

- An `asnkit::value` owns what it holds, and frees it when it is
  destroyed. A copy is deep, and independent of what it is copied
  from; a move is cheap, and leaves what it is moved from empty. An
  `asnkit::spec` is only moved. What is empty may be destroyed,
  assigned to and, for a value, copied, which gives an empty value too:
  anything else throws `asnkit::argument_error`.
- What an accessor gives, an `asnkit::value_view`, a
  `std::string_view`, an `asnkit::bytes_view` or an
  `asnkit::bits_view`, is borrowed from the value it is in, and valid as
  long as that value is and is not changed, as in C. Keep a decoded
  value in a variable while what is borrowed from it is used: what is
  borrowed from a temporary is gone at the end of the statement, and
  `items()` and `members()` of one do not compile. `clone()` of a view
  is an owned copy.
- `push()`, `set()`, `value::choice()`, `value::list({...})` and
  `value::map({...})` take their values as `asnkit::value`, and keep
  them with no further copy: give a value with `std::move()` for it not
  to be copied at all.
- A function that fails throws an `asnkit::error`, which is a
  `std::runtime_error` with the message of the runtime and `code()`,
  its `asnkit_status`. It is of the class of its status:
  `load_error`, `encode_error`, `decode_error`, `constraints_error`,
  `unsupported_error`, `no_such_type_error`, `argument_error`,
  `kind_error`, `asn_range_error` (so named not to be
  `std::range_error`), `io_error` or `panic_error`.
  `decode_error::offset()` tells where the error is, as a
  `std::optional<std::size_t>`. An accessor of another kind throws
  `kind_error`, `at()` and `[]` throw `asn_range_error` for an item or a
  member that is not there, and `find()` gives `std::nullopt`.
  Allocations throw `std::bad_alloc`.
- Names, of types, modules, codecs, members and alternatives, are
  taken as `std::string_view`, and may not have a NUL in them, which
  throws `argument_error`. Text and bytes may have one.
- `get()` gives the handle of a value or a specification to C, which
  it still owns; `value::release()` gives it away, and `value::adopt()`
  takes one that C gave.

## Corrupt files

A file whose CRC-32 does not match is corrupt, and loading it returns
`ASNKIT_ERROR_LOAD`, whatever the options. See
[`docs/file_formats.md`](https://github.com/boxtheta/asnkit/blob/trunk/docs/file_formats.md).

## Testing

```text
cargo test -p asnkitrt-c
```

`tests/c.rs` compiles `examples/question.c` and `tests/api.c` with `cc`
(or `CC`), and `examples/question.cpp` and `tests/api.cpp` as C++17
with `c++` (or `CXX`), and runs them against the shared library, which
needs Linux or macOS. It also compiles the C++ as C++20 and C++23,
without running it. `tests/header.rs` holds the headers, which are
written by hand, to the functions and constants of `src/`:
`asnkitrt.h` has to declare every function, and `asnkitrt.hpp` to use
every one.

It supports what the Rust runtime supports; see
[its README](https://github.com/boxtheta/asnkit/blob/trunk/packages/asnkitrt-rs/README.md#limitations) for the
limitations.

Licensed under the MIT License. See `LICENSE-RUNTIMES`.
