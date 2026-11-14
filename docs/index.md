# asnkit

This is the documentation of the Python runtime and of the formats of the
files. For the repository as a whole (the runtimes in Python, Rust,
WebAssembly, C and the JVM, the compiler, the plugins, the codecs, the commands
and the known limitations) see the
[README](https://github.com/boxtheta/asnkit#readme).
The Rust crates are documented by `cargo doc`. ASN.1 is compiled to `.asni`
files by the compiler `asnkitcc`, which has no Python API.

- [File formats](file_formats.md): the `.asni` and `.asnst` files.
- [ANY DEFINED BY](any_defined_by.md): how to map the types an
  `ANY DEFINED BY` can hold.
- [Guidelines for third-party runtimes](third_party_runtimes.md).
- [Plugins](plugins.md): how they are run, how to write one, and the
  three there are.
- [Versioning](versioning.md): what each kind of release may change.
- [Playground](playground/): compile ASN.1, encode a value with every
  codec and make a JSON Schema, an XML Schema or a DTD of the types, in
  the browser, with the compiler and the runtime as WebAssembly.

## Basic usage

::: asnkitrt.load_asni_file

::: asnkitrt.load_asni

::: asnkitrt.runtime.Specification
    options:
      members: true

## Types

ASN.1 types are mapped to Python types as shown in the table below.

| ASN.1 type | Python type | Example |
|---|---|---|
| BOOLEAN | `bool` | `True` |
| INTEGER | `int` (2) | `87` |
| REAL | `float` | `33.12` |
| NULL | -- | `None` |
| BIT STRING | `tuple(bytes, int)` | `(b'\x50', 4)` |
| OCTET STRING | `bytes` | `b'\x44\x1e\xff'` |
| OBJECT IDENTIFIER | `str` | `'1.33.2'` |
| ENUMERATED | `str` or `int` (1) | `'one'` or `1` |
| SEQUENCE | `dict` | `{'a': 52, 'b': 1}` |
| SEQUENCE OF | `list` | `[1, 3]` |
| SET | `dict` | `{'foo': 'bar'}` |
| SET OF | `list` | `[3, 0, 7]` |
| CHOICE | `tuple(str, object)` | `('a', 5)` |
| UTF8String | `str` | `'hello'` |
| NumericString | `str` | `'234359'` |
| PrintableString | `str` | `'goo'` |
| IA5String | `str` | `'name'` |
| VisibleString | `str` | `'gle'` |
| GeneralString | `str` | `'abc'` |
| BMPString | `str` | `'ko'` |
| GraphicString | `str` | `'a b'` |
| TeletexString | `str` | `'ßø'` |
| UniversalString | `str` | `'åäö'` |
| UTCTime | `datetime.datetime` | `datetime(2018, 6, 11, 11, 4, 59)` |
| GeneralizedTime | `datetime.datetime` | `datetime(2018, 1, 31, 5, 0, 47)` |
| DATE | `datetime.date` | `date(1985, 4, 12)` |
| TIME-OF-DAY | `datetime.time` | `time(15, 27, 46)` |
| DATE-TIME | `datetime.datetime` | `datetime(1985, 4, 12, 15, 27, 30)` |
| DURATION | `str` | `'P1Y2M3DT4H5M6S'` |
| ObjectDescriptor | `str` | `'a b'` |

1. Load with `numeric_enums=True` for numeric enumeration values
   instead of strings.

2. An INTEGER with named numbers can be given as the name of any of them
   when encoding. It is always an `int` when decoded.

## Errors

Every error of the package is an `asnkitrt.Error`:

| Error | When |
|---|---|
| `asnkitrt.EncodeError` | A value can not be encoded as its type. |
| `asnkitrt.DecodeError` | Data is not what the codec writes for the type, whatever is wrong with it. `offset` tells where, in bytes, or in bits for PER and UPER, if it is known. |
| `asnkitrt.ConstraintsError` | A value is not one its constraints allow, when they are checked. |
| `asnkitrt.CompileError` | What is loaded is not a compiled specification that the runtime supports. |
| `asnkitrt.syntax.ConvertError` | A file is not an `.asni` or an `.asnst` file, is of another major version, or its CRC-32 does not match. |

The message of an `EncodeError`, a `DecodeError` and a
`ConstraintsError` starts with where in the value it is, as
`Question.id: ...`. Where one is made from another exception, as one of
`json`, that is its `__cause__`. What a codec does not support, as an
`ANY` in JER, is a `NotImplementedError`, and a file that can not be
read is an `OSError`.

## Advanced usage

::: asnkitrt.load

## Reading and writing the files

::: asnkitrt.syntax.from_asni

::: asnkitrt.syntax.from_asnst

::: asnkitrt.syntax.to_asni

::: asnkitrt.syntax.to_asnst
