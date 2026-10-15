# Guidelines for third-party runtimes

This is for those writing a runtime for asnkit in another language,
or for a platform the first-party runtimes do not cover. The
first-party runtimes are `asnkitrt` for Python, `asnkitrt` for Rust,
and `asnkitrt` on npm, which is the one for Rust built as
WebAssembly.

> **Note:** This is a draft, like the [file formats](file_formats.md) it builds on. It is to be settled before the first release of the formats.

## What a runtime is

A runtime loads a compiled specification, an `.asni` file, and
encodes and decodes the types in it. It does not parse ASN.1. That is
what the compiler, `asnkitcc`, does once, so that a runtime
can be small and run where a parser can not.

A runtime reads `.asni` files. It never needs to read `.asnst`
files, which are for plugins of the compiler.

## What to implement

The parts of a runtime, in the order they are best made:

1. The container: the header, the flags, the signature, the nonce,
   the encryption and the binding, as told in [file formats](file_formats.md), with the checks in the order given there.

2. The nodes of an `.asni` file, in the compact form, and the
   types they make.

3. The codecs. Which ones a runtime has is for its implementers to
   decide alone, and none is required. A runtime says which it has, so
   that those using it know.

4. The checks of types and of constraints, if the runtime encodes. A
   runtime that only decodes may leave out the check of constraints
   on what it decodes, but must then say so.

A runtime may leave out signing or encryption, for a device that has
neither, but then it rejects every file with the flag set, as the
format says of a flag that is not supported. It never ignores one.

## Being secure by default

A runtime loads files that may come from anywhere, and is often found
where nobody can update it. So:

- It follows the order of checks, and reads nothing of the metadata or
  the nodes as CBOR before the signature is checked.

- A reader is not set up to accept any signature, nonce or binding
  unless asked to be, with a name that makes it plain. The default is
  to reject.

- A reader that keeps the nonce of the last file it accepted does so
  only once the whole file is checked and loaded.

- It limits what untrusted data can make it use: the depth of
  nesting, the size of anything it allocates from a length it was
  told, and the time spent. A length in the data is never trusted to
  be as long as the data that is left. This holds for the CBOR reader
  and every decoder.

- A decoder is strict. For DER, that means no indefinite lengths, no
  lengths in more bytes than they need, and no data after the value.
  Data that is not the one encoding of a value is an error, so that
  two different byte strings are never the same value.

- It is fuzzed, or tested with data made to break it, and does not
  panic, crash or run without end on any input.

In a language that has unsafe code, keep it out of the decoders, or
keep it small and well tested.

## Testing a runtime

`examples/corpus` has eight `.asni` files, one for every
combination of being signed, encrypted and bound, with the keys to
read them. A runtime has to load them, and has to reject each of them
when it is set up with the wrong key, the wrong binding or the wrong
nonce, or without the key it needs. A signed or encrypted file is
also rejected when any byte of it is changed.

For the codecs, `tests/vectors` has what the Rust runtime, the one of
reference, encodes and decodes for BER, DER, OER, PER, UPER, JER and XER, with the compiled
specifications it used (see its `README.rst`). Encode and decode the same
and the bytes and values must be the same. The types and values in
`tests/files` are more to try. Compare with an
independent implementation too where there is one, as the tests of
the DER codec do with OpenSSL and X.509.

A runtime is correct when it agrees with the formats and the
standards. Where the Rust runtime, and so the vectors, does not agree
with a standard, that is a bug in it, and it should be reported.

## Versions

A runtime tells which major version of the formats it reads, and
reads every minor version of it, as the format says. It rejects a
file of another major version.

It has its own version, which does not have to be that of the
formats nor of the first-party runtimes.

## Licensing

A runtime of your own can have any license you choose. These are the
things to know:

- The first-party runtimes are MIT. A port that is derived
  from one, by translating its code, has to keep its copyright
  notices and the license text. A port of the codecs of the runtime
  for Python also has to keep `packages/asnkitrt/NOTICE`, which holds
  the MIT license of asn1tools that they derive from.

- Writing a runtime from the format and the standards, without
  translating any code, does not need any of this.

- The documentation of the formats and the corpus are licensed under
  Apache-2.0, and the code in the corpus under MIT, so that you can
  copy them into your runtime, its documentation and its tests, under
  any license, as long as you keep the notices those licenses ask for.
  The rest of this repository, such as the test files in `tests`, is
  Apache-2.0 too.
  The section License of `README.md` in the root of the repository
  tells which is which. It is not legal advice.

- The test vectors in `tests/vectors` are not under any of these
  licenses. They are made from ASN.1 modules by other parties (3GPP,
  ETSI, IETF, OMA and others), and are included under the licenses of
  those who wrote the modules. Check those terms before you copy them
  into your runtime or its tests. `record.py` and `values.py`, which
  make them, are MIT, as is the Rust runtime that gives their results.

- The compiler, `asnkitcc`, is Apache-2.0. A runtime does not need
  it to run, and does not link to it. Code copied from it into a
  runtime keeps the Apache License, with its copyright notices.

## Name

Do not name a runtime so that it can be taken for one made by
BoxTheta, as "asnkit-go" could. Say that it is for asnkit, or reads
asnkit files, instead.

## Issues and security

Report a flaw in the formats, in the compiler or in a first-party
runtime as told in `SECURITY.md` if it is a vulnerability, and
otherwise as an issue. A vulnerability in a runtime of your own is
yours to handle, but tell us if it comes from how the format is
written, as other runtimes are then affected too, so that it can be
fixed and told to all at once.

## Asking for changes

The formats are in `docs/file_formats.md`, and the list "Not yet
specified" at the end of it tells what a runtime has to guess today.
If you find something else that is not clear, or two runtimes that
read it differently, open an issue. It will be answered by changing
the text, and not by saying what the Rust runtime does.
