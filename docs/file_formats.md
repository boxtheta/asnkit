# File formats

This is the specification of the two files a parsed ASN.1
specification is stored in. It is written for those reading or
writing them without this package, in C, Rust or anything else.

> **Note:** This document is licensed under the Creative Commons Attribution 4.0 International License (CC-BY-4.0), but for its code blocks, which are licensed under the MIT License. See `COPYING` and `LICENSE-DOCUMENTATION` in the root of the repository.

> **Note:** This is a draft of version 1.0 of the formats. Nothing has been released with them yet, so anything in here may still change without a new major version. [Not yet specified](#not-yet-specified) lists what is missing.

## The two files

- `.asnc` — A compiled specification: all that is needed to encode and decode
  its types, with nothing left to look up or work out. It is what a
  runtime loads, and it is made to be small and simple to read.

- `.asnst` — A syntax tree: everything found in a specification, as it is
  written. It is what plugins are given, with the text of the ASN.1
  files it was made from and where in them things are found. Its
  size matters less than how easy it is to look at. A runtime never
  loads one. It is compiled to an `.asnc` file first.

Both hold nodes of the same kinds. They differ in which kinds are
found in them, in how a node is written and in what is around the
nodes: an `.asnc` file has a header a small device can check, and
an `.asnst` file is [nothing but CBOR](#syntax-tree-files).

## Media types

| File | Media type | Extension |
|---|---|---|
| Compiled specification | `application/vnd.asnkit.asnc` | `.asnc` |
| Syntax tree | `application/vnd.asnkit.asnst+cbor` | `.asnst` |

These are claimed for the two files, in the vendor tree of RFC 6838,
and are not registered with IANA yet. Neither takes parameters.

An `.asnst` file has the suffix `+cbor`, as it is nothing but CBOR
and anything that reads CBOR can read it. An `.asnc` file has no
suffix: its header comes before the CBOR, and the CBOR is not
readable at all when the file is encrypted.

A file has the same media type whatever its version, and whether or
not it is signed, encrypted, bound or has a nonce: the header tells
all of that. One that sends a file, as over HTTP, gives its media
type. One that receives it does not rely on the media type, nor on
the extension, but on what is in the file: an `.asnc` file begins
with the four bytes `41 53 4E 43` (`ASNC` in ASCII), and an `.asnst`
file is a CBOR map whose `format` is `asnst`.

## Layout

This is the layout of an `.asnc` file. All numbers of more than one
byte have the most significant byte first.

| Offset | Bytes | What |
|---|---|---|
| 0 | 4 | `ASNC` in ASCII. |
| 4 | 1 | Major version, currently 1. |
| 5 | 1 | Minor version, currently 0. |
| 6 | 2 | [Flags](#flags). |
| 8 | 3 | [Nonce](#nonce). Always there, and zero unless the nonce flag is set. |
| 11 | 80 | [Signature](#signature). Only if the signed flag is set. |
|  | 44 | [Encryption](#encryption) parameters. Only if the encrypted flag is set. |
|  | variable | The [metadata](#metadata), as one CBOR map. |
|  | variable | The [nodes](#nodes), as one CBOR item. |

Nothing follows the nodes. A reader rejects a file with anything
after them.

## Flags

| Bit | Name | What |
|---|---|---|
| 0 | signed | The file has a [signature](#signature). |
| 1 | encrypted | The metadata and the nodes are [encrypted](#encryption). |
| 2 | nonce | The [nonce](#nonce) is to be checked. |
| 3 | bound | The file is only for those its metadata tells. See [Bound files](#bound-files). |
| 4-15 | -- | Not in use. Zero. |

Bit 0 is the least significant, so a signed file that is not
encrypted has the flags `00 01`.

A reader rejects a file with a flag set that it does not know of or
does not support. It never ignores one.

Only an `.asnc` file has flags, as it is the one sent to devices.
An `.asnst` file is never signed, encrypted or bound, and has no
nonce.

What the flags are for is giving a device in the field a new or changed
protocol without new firmware: it is sent a compiled specification,
which it only loads if signed by someone it trusts, made for it and
not sent before, and which nobody else can read if encrypted. So all
of it is made to be checked by a small device, as the file is
received and, but for what it is bound to, before anything in it is
read as CBOR.

## Signature

A signed file has 80 bytes at offset 11:

| Offset | Bytes | What |
|---|---|---|
| 11 | 16 | The identifier of the key it is signed with. What it means is up to those giving out keys. |
| 27 | 64 | An Ed25519 signature (RFC 8032). |

What is signed is the SHA-256 digest of the whole file but the 64
bytes of the signature itself. So the flags, the nonce, the
identifier of the key, the encryption parameters, the metadata and
the nodes can not be changed without it showing.

```text
digest    = SHA-256(file[0:27] || file[91:])
signature = Ed25519-Sign(private key, digest)
```

This is plain Ed25519 with the 32 bytes of the digest as its message.
It is not Ed25519ph, which hashes with SHA-512. The digest can be
computed as a file is read, without keeping all of it in memory.

A file that is both signed and encrypted is encrypted first and then
signed, so the signature is checked without the key to decrypt it. A
reader checks the signature before it does anything else with a
file.

A reader is set up with the public keys of those it trusts, each
with its identifier. It rejects a signed file if it has no key with
the identifier in the file, or if the signature is not that of the
file with that key. A reader that is set up with keys rejects every
file that is not signed. A reader that is not set up with keys
rejects every signed file.

A reader can also be set up to accept any signature. It then reads a
file without checking its signature, and whether it is signed or
not. That is for looking at a file, and never how a reader is set up
unless asked to be.

How the public keys are given to a reader is not part of this
specification.

## Nonce

The three bytes at offset 8 are a number of 24 bits. They are there
in every file, so that everything after them is at the same place
whether or not they are in use.

With the nonce flag set, a reader checks the number and rejects the
file if the check fails. What the check is, is not part of the file.
It is how the reader is set up, as one of:

- The number is the one the reader expects. That is for a device
  that makes up a number and sends it when asking for a file, so that
  a file made for an earlier request can not be sent to it again.

- The number is greater than that of the last file the reader
  accepted. That is for a device that is sent files without asking,
  so that an old file can not be sent to it again.

A reader that is not set up to check the nonce rejects every file
with the flag set. A reader that is set up to check it rejects every
file without the flag, as there is nothing to check.

A reader that keeps the nonce of the last file it accepted only does
so once all of a file is checked and loaded, and never as part of
checking the nonce. The nonce comes before the tag of an encrypted
file, so until the tag is checked it may be any number. A reader that
kept it sooner could be made to keep the greatest number there is,
and would then accept no file ever again.

A reader can also be set up to accept any nonce. It then reads a file
whatever the number is, and whether the flag is set or not. That is
never how a reader is set up unless asked to be.

Without the flag, a writer writes zero and a reader does not look at
the number.

The nonce flag can be set in a file that is not signed. The nonce
then keeps a reader from loading a file that is old or not the one it
asked for by mistake, as when files are mixed up or sent twice. It
does not keep anyone from changing the number on purpose. For that
the file has to be signed, as the nonce is part of what is signed.

## Bound files

With the bound flag set, the file is only for those it is bound to,
and its [metadata](#metadata) tells which in `bound_to`: a text string of at
most 255 bytes as UTF-8. What is in it is up to those making the
file. It may be the name of a product, or anything else.

A reader is set up with the string it accepts. It rejects a bound
file if `bound_to` is not that string, byte for byte as UTF-8, or
if there is no `bound_to`. A reader that is not set up with a
string rejects every bound file. A reader that is set up with one
rejects every file that is not bound.

A reader can also be set up to accept any binding. It then reads a
file whatever it is bound to, and whether it is bound or not. That is
never how a reader is set up unless asked to be.

A reader rejects a `bound_to` of more than 255 bytes whatever it is
set up to accept, and compares no more bytes than that.

A reader that reads a file as it receives it never keeps
`bound_to`. The length of a text string comes before its bytes in
CBOR, so it first rejects a `bound_to` of more than 255 bytes, or
of another length than that of its own string, and then compares the
bytes as it reads them. How much memory it uses so never depends on
how long `bound_to` is.

A reader that is given a whole file in memory may read `bound_to`
before it checks its length, as the reader of this package does. It
then uses no more memory for it than the file is long.

It is a flag, and not only something in the metadata, because a
reader ignores what it does not know of in the metadata, but never a
flag. A reader that knows nothing of bound files rejects them all.

The bound flag can be set in a file that is not signed. It then
keeps a reader from loading a file made for something else by
mistake. It does not keep anyone from changing what the file is bound
to on purpose. For that the file has to be signed, as the metadata is
part of what is signed.

In an encrypted file the metadata is encrypted, so what the file is
bound to is not known until it is decrypted.

## Encryption

An encrypted file has 44 bytes of encryption parameters after the
signature, at offset 91, or at offset 11 if it is not signed. The
offsets below are from where they start:

| Offset | Bytes | What |
|---|---|---|
| 0 | 16 | The identifier of the key. What it means is up to those giving out keys. |
| 16 | 12 | The initialization vector. Never used twice with the same key. |
| 28 | 16 | The authentication tag. |

The metadata and the nodes are encrypted as one with AES-256 in
Galois/Counter Mode (NIST SP 800-38D). The encrypted bytes take the
place of the metadata and the nodes in the file, and are as many.

The additional authenticated data is the first 11 bytes of the file
followed by the identifier of the key and the initialization vector,
so that none of them can be changed without the tag failing.

```text
aad             = file[0:11] || key id || iv
ciphertext, tag = AES-256-GCM-Encrypt(key, iv, metadata || nodes, aad)
```

As the metadata is encrypted too, nothing but what comes before it
is known about an encrypted file without its key: the kind of file,
its version, its flags, its nonce and the identifiers of its keys.

A reader is set up with the keys it decrypts with, each with its
identifier. It rejects an encrypted file if it has no key with the
identifier in the file, or if the tag is not that of the file with
that key. A reader that is set up with keys rejects every file that
is not encrypted. A reader that is not set up with keys rejects every
encrypted file.

How keys are given to a reader is not part of this specification.

## Order of checks

A reader checks a file in this order, and stops at the first check
that fails:

1. The four first bytes, the major version and the flags.

2. The signature, if the file is signed or the reader expects it to
   be.

3. The nonce, if the nonce flag is set or the reader expects it to
   be.

4. The tag, as the file is decrypted, if the file is encrypted or the
   reader expects it to be.

5. What the file is bound to, if the bound flag is set or the reader
   expects it to be.

Only then are the nodes read. Nothing of the metadata or the nodes is
read as CBOR before the signature is checked.

## Syntax tree files

An `.asnst` file is one CBOR map and nothing else, so that a plugin
needs nothing but something that reads CBOR to read it. The map has:

- `format` — The text string `asnst`.

- `version` — A list of the major and the minor version, currently `[1, 0]`.
- `metadata` — The [metadata](#metadata).

- `tree` — The [nodes](#nodes), in [the readable form](#the-readable-form).

A reader ignores the keys it does not know of. It rejects a file of
another major version.

## Metadata

The metadata is a CBOR map with text strings as keys. All keys are
optional, and a reader ignores those it does not know of.

- `generator` — A text string naming what wrote the file and its version.

- `sources` — A list of the ASN.1 files the file was made from, in the order
  they were parsed. Each is a map with any of:

  - `name` — The name of the file, without its folder.
  - `sha256` — The SHA-256 digest of its text encoded as UTF-8, as 64
    lowercase hexadecimal digits.

  - `text` — Its text. An `.asnst` file has it unless told otherwise. An
    `.asnc` file never does.

- `bound_to` — What the file is bound to, as a text string of at most 255 bytes
  as UTF-8. Only looked at in a bound `.asnc` file. See [Bound files](#bound-files).

- `other` — A map of anything else those making the file want it to tell.

## CBOR

The metadata and the nodes are CBOR (RFC 8949), of which only this is
used:

- Unsigned and negative integers. Those that do not fit in 64 bits
  are bignums, with tag 2 or 3.

- Floating point numbers. A writer only writes those of 64 bits. A
  reader accepts those of 16, 32 and 64 bits.

- Text strings of a known length, as UTF-8.

- `false`, `true` and `null`.

- Arrays of a known length.

- Maps of a known length, with text strings as keys.

There are no byte strings but those of a bignum, nothing of
indefinite length and no other tags.

## Nodes

What a file holds is a tree of nodes. A node is of a kind and has
fields, which hold text strings, numbers, booleans, other nodes and
lists of any of those. [Kinds of nodes](#kinds-of-nodes) lists them.

The top node of an `.asnc` file is a `CompiledSpecification` and
that of an `.asnst` file is a `SyntaxTree`.

A node is written in one of two forms.

### The compact form

This is the form of an `.asnc` file.

- A node is an array of the number of its kind followed by its
  fields, in the order of [Kinds of nodes](#kinds-of-nodes).

- A field that is what it is by default is `null`. The default of
  each field is in [Kinds of nodes](#kinds-of-nodes).

- Fields that are `null` last in the array are left out. A node
  with nothing but such fields is an array of the number of its kind
  alone.

- A list is an array of 0 followed by its items. No kind of node has
  the number 0, which is how a list is told from a node.

- A text string, a number or a boolean is itself.

A reader takes a field that is `null` or left out as its default.
It ignores fields after those it knows of, which are those of a later
minor version. It rejects a kind of node it does not know of.

This is `A ::= SEQUENCE { a INTEGER (0..7), b BOOLEAN OPTIONAL }`
in a module `Foo`, shown as JSON:

```text
[1,                                    CompiledSpecification
  [0,                                    modules
    [2, "Foo",                           CompiledModule
      [0,                                  types
        [3, "A",                           TypeAssignment
          [15,                               SequenceType
            [0,                                members
              [4, "a",                         Member
                [12, null, null, null,           IntegerType
                  [0, [6, 0, 7]]]],                restricted_to: Range
              [4, "b",                         Member
                [10, "BOOLEAN"],                 BuiltinType
                true]]]]]]]]                     optional
```

It is 50 bytes as CBOR.

### The readable form

This is the form of an `.asnst` file.

- A node is a map with the name of its kind as `kind` and each of
  its fields by name.

- A field that is `null` is left out. Any other field is there,
  also when it is what it is by default.

- A list is an array of its items.

A reader ignores the fields it does not know of, which are those of a
later minor version. It rejects a kind of node it does not know of.

This is `A ::= INTEGER` in a module `Foo`, shown as JSON:

```text
{"kind": "SyntaxTree",
 "modules": [
   {"kind": "Module",
    "name": "Foo",
    "extensibility_implied": false,
    "imports": [],
    "types": [
      {"kind": "TypeAssignment",
       "name": "A",
       "type": {"kind": "IntegerType"},
       "location": [0, 1, 27]}],
    "values": [],
    "objects": [],
    "object_classes": [],
    "object_sets": [],
    "location": [0, 1, 1]}]}
```

## Kinds of nodes

The number of a kind is what tells it in the compact form, and its
fields are in the order they have in that form. A field with a
default has it after `=`.

*type fields* stands for the fields all types have, in this order and
all `null` by default: `tag`, `size`, `restricted_to`,
`permitted_alphabet`, `with_components`, `with_component`,
`table`, `parameters`, `actual_parameters`, `module_name` and
`constraints`.

*constraint fields* stands for `size`, `restricted_to`,
`permitted_alphabet`, `with_components`, `with_component` and
`containing`, all `null` by default.

<!-- kinds-begin -->

| No. | Kind | Fields |
|---|---|---|
| 1 | CompiledSpecification | modules |
| 2 | CompiledModule | name, types = [], object_classes = [] |
| 3 | TypeAssignment | name, type, location = null |
| 4 | Member | name, type, optional = false, default = null, location = null |
| 5 | Tag | number, tag_class = null, mode = null |
| 6 | Range | lower, upper |
| 7 | NamedNumber | name, number |
| 8 | ExtensionMarker | -- |
| 9 | ExtensionAdditionGroup | members |
| 10 | BuiltinType | name, *type fields* |
| 11 | ReferencedType | name, *type fields* |
| 12 | IntegerType | named_numbers = null, *type fields* |
| 13 | BitStringType | named_bits = null, *type fields* |
| 14 | EnumeratedType | values, *type fields* |
| 15 | SequenceType | members, *type fields* |
| 16 | SetType | members, *type fields* |
| 17 | ChoiceType | members, *type fields* |
| 18 | SequenceOfType | element, element_name = null, *type fields* |
| 19 | SetOfType | element, element_name = null, *type fields* |
| 20 | Min | -- |
| 21 | Max | -- |
| 22 | Identifier | name |
| 23 | HexString | digits |
| 24 | BinaryString | bits |
| 25 | Null | -- |
| 26 | PlusInfinity | -- |
| 27 | MinusInfinity | -- |
| 28 | NotANumber | -- |
| 29 | NamedValue | name, value |
| 30 | SequenceValue | members |
| 31 | UnsupportedValue | -- |
| 32 | Excluded | value |
| 33 | TableConstraint | object_set, component_ids = null |
| 34 | ComponentConstraint | name, presence = null, *constraint fields* |
| 35 | ElementConstraint | *constraint fields* |
| 36 | AnyDefinedByType | value, choices = [], *type fields* |
| 37 | AnyDefinedByChoice | value, type |
| 38 | ComponentsOf | type_name |
| 39 | ObjectClassAssignment | name, fields, syntax = null, location = null |
| 40 | ObjectClassField | name, type, location = null |
| 41 | SyntaxTree | modules |
| 42 | Module | name, tags = null, extensibility_implied = false, imports = [], types = [], values = [], objects = [], object_classes = [], object_sets = [], location = null |
| 43 | Import | module, symbols |
| 44 | ValueAssignment | name, type, value, location = null |
| 45 | ObjectAssignment | name, class_name, object, location = null |
| 46 | ObjectSetAssignment | name, class_name, members, location = null |
| 47 | Object | fields |
| 48 | ObjectField | name, setting |
| 49 | UnresolvedObject | source |
| 50 | ObjectSet | members |
| 51 | OptionalGroup | tokens |
| 52 | Parameter | name, governor = null |
| 53 | ElementSetSpecs | root, extensible = false, additions = null |
| 54 | Union | elements |
| 55 | Intersection | elements |
| 56 | Except | elements, exclusions |
| 57 | All | exclusions |
| 58 | SizeConstraint | constraint |
| 59 | PermittedAlphabet | constraint |
| 60 | ContainedSubtype | type, includes = false |
| 61 | PatternConstraint | value |
| 62 | InnerTypeConstraint | constraint |
| 63 | InnerTypeConstraints | partial = false, components |
| 64 | NamedConstraint | name, constraints = null, presence = null |
| 65 | ContentsConstraint | type = null, encoded_by = null |
| 66 | UserDefinedConstraint | -- |
| 67 | UnsupportedConstraint | -- |

<!-- kinds-end -->

What the kinds of a compiled specification mean is told in
[What the nodes mean](#what-the-nodes-mean). The others, which are
only found in syntax trees, are told in `asnkitrt.syntax.nodes` (see
the [reference](index.md)), and those of constraints in
[Constraints in a syntax tree](#constraints-in-a-syntax-tree).

## Constraints in a syntax tree

A syntax tree has the constraints of a type as they are written, in
`constraints`: a list of the constraints in parentheses after the
type, each applied to the type with those before it (X.680 49.1,
50.11). Its `size`, `restricted_to`, `permitted_alphabet`,
`with_components`, `with_component` and `table` are not used, which a
compiled specification has instead, with what the compiler made of the
constraints. A constraint is one of:

- `ElementSetSpecs`, for sets of values (X.680 50): the `root` set, if
  it is `extensible` with `...`, and the `additions` after it.

- `TableConstraint`, for `{ObjectSet}` or `{ObjectSet}{@component}`
  (X.682 10), and `UnsupportedConstraint` for one of a set of objects
  that is not given by name.

- `ContentsConstraint`, for `CONTAINING` a `type` and `ENCODED BY` a
  value (X.682 11), and `UserDefinedConstraint` for `CONSTRAINED BY`
  (X.682 9).

A set of values is a `Union` or an `Intersection` of its `elements`,
which are sets, an `Except` of the values of `elements` but those of
`exclusions`, an `All` but its `exclusions`, or an element (X.680 51):

- A single value, or a `Range` of values.

- `SizeConstraint` and `PermittedAlphabet`, for `SIZE` and `FROM`, with
  the `constraint` of the sizes or of the characters.

- `ContainedSubtype`, the values of a `type`, written with `INCLUDES`
  or not.

- `PatternConstraint`, for `PATTERN` and its `value`.

- `InnerTypeConstraint`, for `WITH COMPONENT` and the `constraint` of
  the elements, and `InnerTypeConstraints`, for `WITH COMPONENTS` and
  its `components`, which is `partial` if it starts with `...`. Each
  component is a `NamedConstraint` with its `name`, its `constraints`
  and its `presence`: `PRESENT`, `ABSENT` or `OPTIONAL`.

An element that is not supported, as an object in braces, is an
`UnsupportedValue`. Parentheses are not kept, as the sets nest.

## Values

A value, as the default of a member or a bound of a range, is one of:

- An integer, a floating point number or a boolean.

- A text string, which is a character string given in quotes. It is
  never the name of something.

- `Identifier`, for a value given by name: a reference to a value,
  or an item of an enumeration, a named number or a named bit.

- `BinaryString` and `HexString`, with the bits and the
  hexadecimal digits as a text string.

- `Null`, `PlusInfinity`, `MinusInfinity` and `NotANumber`.

- `NamedValue`, as the value of a choice.

- `SequenceValue`, for a value in braces with every item named.

- A list of values, for any other value in braces.

- `UnsupportedValue`, for what the parser does not support.

`Min`, `Max` and `Excluded` are only found as the bounds of a
`Range`.

## Compiled specifications

An `.asnc` file only has the kinds of nodes numbered 1 to 40, and of
those `ComponentsOf`, `location`, `parameters`,
`actual_parameters` and `constraints` are never found in one. Unlike in a syntax
tree:

- Every type is what it is with `COMPONENTS OF`, `EXTENSIBILITY IMPLIED`, the tagging of its module and any actual parameters
  applied.

- Every `Tag` of a type has a `mode`. That is the case for the tags of
  members and of elements, and for the tags that `AUTOMATIC TAGS`
  gives. The only tags that may have no mode are those of the types
  of the fields of object classes, which are never applied (see
  [Finding a type](#finding-a-type)).

- Tags, named numbers, named bits and enumeration numbers are
  numbers, never an `Identifier`.

- The `size`, `restricted_to` and `permitted_alphabet` of a type are
  its effective constraints, which the compiler works out of the
  constraints as they were written. See [Constraints](#constraints).

- A default given as a named number, or as a reference to a number, a
  boolean or a character string, is that number, boolean or string.
  The default of a `BIT STRING` is a `BinaryString` and that of an
  `OCTET STRING` a `HexString`, however they were written.

- A reference to a type or an object class of another module tells
  which module in `module_name`, which is always the name of a module
  of the file. There are no imports.

- The names of the modules are all different, and so are the names of
  the types of a module and those of its object classes.

- There are only types and object classes. There are no values and
  no object sets.

A reader does not have to check these. What it does with a file that
does not keep to them is not specified.

Two things are still left to look up by name in version 1.0:

- A default given as a reference to a value that is not a number, a
  boolean or a character string is an `Identifier`, with no value
  to find. See [Defaults](#defaults).

- A field of an object class, as in `ALGORITHM.&id`, is a
  `ReferencedType` with that as its name, and the class is found in
  `object_classes`. See [Finding a type](#finding-a-type).

## What the nodes mean

This is for those writing a runtime. It tells what the runtimes in
this repository do with the nodes of a compiled specification, which
is the same in both of them. `tests/vectors` holds more than 4,000
encodings and decodings by the one for Python, with the compiled
specifications they were made with, for a runtime to be tried against.

The codecs themselves are those of the standards: X.690 for BER and
DER, X.691 for PER and UPER, X.696 for OER, X.697 for JER and X.693 for
XER. This section tells which ASN.1 type a node is, which is what a
codec needs to know. Where the runtimes here do not do what a
standard says, it is told in
[Where the runtimes differ from the standards](#where-the-runtimes-differ-from-the-standards).

A runtime reads all the types of a file when it loads it, and rejects
the file if one of them is not supported, also one that is not used.

### Finding a type

A `ReferencedType` is the type it names. It is found in the module
that `module_name` tells or, without it, in the module of the type it
is part of. That is the module of the nearest type around it that has a
`module_name`, or else the module of the `TypeAssignment` or the
`ObjectClassAssignment` it is found in. A type of a member, of an
element or of a choice of an `ANY DEFINED BY` is part of the type
it is in.

A `ReferencedType` with no tag, `size`, `restricted_to` or
`permitted_alphabet` of its own is the type it names, and nothing
else. One with any of those is a new type: the type it names, with
them applied, as [Tags](#tags) and [Constraints](#constraints) tell.

A type can be part of itself through a member, an element or a choice
of an `ANY DEFINED BY`, as `A ::= SEQUENCE { a A OPTIONAL }`. A type
that is a `ReferencedType` of itself, or of a chain of
`ReferencedType` that comes back to it, is not a type, and a runtime
rejects the file.

A name with a `.` in it, as `ALGORITHM.&id`, is a field of an object
class: the class, a `.`, then the field, with its `&`. The class is
the `ObjectClassAssignment` with that name in `object_classes` of
the module the type is found in, as above. The compiler copies a class
to each module that imports it, so that it is always found there. The
type is the `type` of the `ObjectClassField` with that name, with
its constraints, but without its tag. The names in it are found in the
module of the class. The tag and the constraints of the
`ReferencedType` are applied to it as to any other type.

A field of an object class that holds a type, as `&Type`, has the type
`OpenType`: the value of a type that is not known when compiling. A
runtime gives and takes its value as the bytes of the encoding of that
value. BER and DER write those bytes as they are, as for an `ANY`, and
PER, UPER and OER after their length (X.691 11.2, X.696 30). PER and
UPER write them as an unconstrained `OCTET STRING`, in fragments if
there are 16384 bytes or more (X.691 11.9). JER and XER need the
type of the value to encode it with (X.697 41, X.693 8.5), and reject
a value of an open type.

### Built-in types

A `BuiltinType` is the ASN.1 type with its `name`, and
`IntegerType`, `BitStringType`, `EnumeratedType`,
`SequenceType`, `SetType`, `ChoiceType`, `SequenceOfType`,
`SetOfType` and `AnyDefinedByType` are the types they are named
after. These are the names a `BuiltinType` can have:

| `name` | Universal tag |
|---|---|
| `BOOLEAN` | 1 |
| `INTEGER` | 2 |
| `BIT STRING` | 3 |
| `OCTET STRING` | 4 |
| `NULL` | 5 |
| `OBJECT IDENTIFIER` | 6 |
| `ObjectDescriptor` | 7 |
| `EXTERNAL` | 8, constructed |
| `REAL` | 9 |
| `UTF8String` | 12 |
| `NumericString` | 18 |
| `PrintableString` | 19 |
| `TeletexString`, `T61String` | 20 |
| `VideotexString` | 21 |
| `IA5String`, `ISO646String` | 22 |
| `UTCTime` | 23 |
| `GeneralizedTime` | 24 |
| `GraphicString` | 25 |
| `VisibleString` | 26 |
| `GeneralString` | 27 |
| `UniversalString` | 28 |
| `BMPString` | 30 |
| `DATE` | 31 |
| `TIME-OF-DAY` | 32 |
| `DATE-TIME` | 33 |
| `DURATION` | 34 |
| `ANY` | none |
| `OpenType` | none |

`ENUMERATED` has the universal tag 10, `SEQUENCE` and
`SEQUENCE OF` 16, and `SET` and `SET OF` 17, all constructed. A
`CHOICE` has none.

A `BuiltinType` with any other name, as `CHARACTER STRING` or
`EMBEDDED PDV`, is not supported.

The characters of `TeletexString`, `VideotexString`, `GraphicString`,
`GeneralString` and `ObjectDescriptor` are written as one byte each,
as ISO 8859-1, and so a runtime only supports the characters of ISO
8859-1 in them. Those of `UniversalString` are four bytes each and
those of `BMPString` two, the most significant first. A `BMPString`
has the characters of the BMP only (X.690 8.23.8): a runtime rejects
another one when encoding, and a surrogate when decoding.

A `REAL` is written by BER, DER, PER and OER in the form of DER
(X.690 11.3.1, X.691 15.2, X.696 12.4): in base 2, with no scaling
factor, and an odd mantissa and its exponent each in as few bytes as
they need. Plus zero has no contents, and minus zero is the special
value 0x43 (X.690 8.5.3). A BER decoder reads every form of X.690
8.5, in base 2, 8 or 16 and in decimal, as the float nearest to the
value, with ties to even, and rejects one too large for a float. A
DER, PER or OER decoder rejects any form but that of DER.

A DER decoder reads a `UTCTime` as `YYMMDDHHMMSSZ` only, and a
`GeneralizedTime` as `YYYYMMDDHHMMSSZ`, with a fraction of a second
after a `.` if it is not 0, without trailing zeros (X.690 11.7,
11.8). Both are a time in UTC, with no offset.

`EXTERNAL` is encoded as this type, in an environment of `EXPLICIT
TAGS`, by all the codecs (X.690 8.18.1, X.691 29.1, X.696 26, X.693
8.4, X.697 37):

```text
EXTERNAL ::= [UNIVERSAL 8] IMPLICIT SEQUENCE {
    direct-reference      OBJECT IDENTIFIER OPTIONAL,
    indirect-reference    INTEGER OPTIONAL,
    data-value-descriptor ObjectDescriptor OPTIONAL,
    encoding              CHOICE {
        single-ASN1-type  [0] ABSTRACT-SYNTAX.&Type,
        octet-aligned     [1] IMPLICIT OCTET STRING,
        arbitrary         [2] IMPLICIT BIT STRING
    }
}
```

Its value is one of that type. `single-ASN1-type` is an open type, as
[Finding a type](#finding-a-type) tells.

An `ANY` is encoded as the open type that took its place in ASN.1, as
[Finding a type](#finding-a-type) tells: its value is the bytes of the
encoding of a value, which BER and DER write as they are, PER, UPER
and OER after their length, and JER and XER reject. The bytes are
those of the codec that writes them: a value decoded with BER is not
one to encode with PER.

An `AnyDefinedByType` with no choices is an `ANY`. With choices, it
has the type of the choice of the value of the member it names, which
has to be there before it in the same `SEQUENCE` or `SET`, and a
runtime rejects a value of that member that no choice has. BER and DER
write the value of that type as it is, PER, UPER and OER as the value
of an open type of that type (X.691 11.2, X.696 30), and JER and XER as
the value of that type (X.697 41, X.693 8.5). A PER or UPER decoder
rejects one that does not fill the bytes of the open type, but for the
padding of the last byte and the one byte 0 of an empty encoding
(X.691 11.1), and an OER decoder one that does not fill its length.
Each choice is an `AnyDefinedByChoice`, with the value of that member
and the type of the `ANY` when it has that value.

The `named_numbers` of an `IntegerType` are the names that can be
given instead of a number when encoding. One with a `number` that is
not an integer is left out. The `named_bits` of a `BitStringType`
are only looked at to tell that there are some, which changes how the
value is encoded and compared, as X.680 22.7 says: a value with
trailing zero bits is the same as one without them.

The `values` of an `EnumeratedType` are `NamedNumber` nodes, and
one `ExtensionMarker` if it is extensible. Those after the marker are
the additions. Each has its number, which is what BER, DER, OER, JER
and XER write and how PER and UPER sort the items of the root.

### Tags

Each type has the tags it has on the wire, the outermost first. A type
of a kind that has a universal tag starts with it, as
[Built-in types](#built-in-types) tells. A `CHOICE`, an `ANY` and an
`OpenType` have none of their own.

A `Tag` of a type is then applied. Its `tag_class` is
context-specific when there is none. With the mode `EXPLICIT` it is
put in front of the tags the type has, constructed. With `IMPLICIT`
it takes the place of the first of them, and is constructed if that
one was. The compiler makes a tag of an untagged `CHOICE`, open type
or `ANY` explicit, whatever the tagging of the module (X.680 31.2.7),
and rejects `IMPLICIT` on them (31.2.9).

Only BER and DER write tags, with one exception: OER writes the
outermost tag of the alternative of a `CHOICE` that is chosen, with
the class and the number. That of an untagged `CHOICE` as the
alternative is the tag of the alternative chosen in it, which that
`CHOICE` writes again (X.696 20.1).

A type is told from the others by its first tag when a member of a
`SEQUENCE` or a `SET` is left out, or an alternative of a
`CHOICE` is found. A `CHOICE` has the tags of all of its
alternatives.

The canonical order of tags is the class (universal, application,
context-specific, private) and then the number (X.680 8.6). The place
of an untagged `CHOICE` in it is that of the least tag of the
alternatives of its root, and of those of untagged `CHOICE` types in
them, and a type with no tag, as an `ANY`, is last. PER and OER write
the root members of a `SET` in that order, and its additions in the
order they are declared in (X.691 21, X.696 18.2). PER numbers the
alternatives of a `CHOICE`, those of the root and the additions each
from 0, in that order (X.691 23.2, 23.3). BER writes the members of a
`SET` in that order too, though any order is BER. DER writes them in
the order of the tags they are encoded with, which for an untagged
`CHOICE` is the tag of the alternative chosen (X.690 10.3), and rejects
another order when decoding.

### Members

The members of a `SEQUENCE`, a `SET` or a `CHOICE` are the
`Member` nodes in their list, and the `ExtensionMarker` nodes tell
where the extension is. The first marker begins the extension
additions, and a second one ends them: members after it are root
members again, as the ones before the first are. There is an
extension if there is a marker, also if it has no additions. A list
has no more than two markers.

PER and UPER write each addition of a `SEQUENCE` or a `SET`, and an
alternative of a `CHOICE` that is an addition, in an open type: its
complete encoding, which is one byte 0 if it is empty (X.691 11.1), as
an unconstrained `OCTET STRING`, in fragments if it is 16384 bytes or
more (X.691 11.2). A decoder also reads an empty one.

An `ExtensionAdditionGroup` is a group of additions that are all
there or none are. It is only found between the markers, and it holds
`Member` nodes only. PER and UPER write it as one addition, which
has the presence bits of its own optional members, and does not write
the addition at all if all of them are left out. The others write its
members as the additions they are, one by one.

`optional` is `OPTIONAL`. A `default` is `DEFAULT`, as
[Defaults](#defaults) tells. A member with its default value is left
out when encoding in all the codecs (but in PER, UPER and OER if it is
an extension addition), and DER rejects it when decoding. When
decoding, a member that is not there has its default, if it has one.

### Defaults

A `default` is the value of the type of its member as this table
says. One that is not in it, as an `UnsupportedValue`, is no default,
and the member then has to be there.

| Type of the member | `default` | Value |
|---|---|---|
| any | `Null` | No default. So a `NULL` with a default has none. |
| `BOOLEAN` | `true` or `false` | The boolean. |
| `INTEGER` | an integer, also a bignum | The integer. |
| `REAL` | a floating point number or an integer | The number. |
| `REAL` | `PlusInfinity` or `MinusInfinity` | Infinity, positive or negative. |
| `ENUMERATED` | `Identifier` | The item with that name, or its number if the runtime is told to give numbers. |
| `BIT STRING` | `BinaryString` | The bits, as many as there are digits. |
| `OCTET STRING` | `HexString` | The bytes. An odd number of digits has a `0` added. |
| character strings | a text string | The text. |
| `CHOICE` | `NamedValue` | The alternative with its name, with the value as this table says. |
| `SEQUENCE`, `SET` | `SequenceValue` | The members it names, with their values as this table says. |
| `SEQUENCE OF`, `SET OF` | a list | The items, with their values as this table says. |
| any other | `Identifier` | The name, as text. |

The last row is that of a default given as a reference to a value
that the compiler did not resolve, as an `OBJECT IDENTIFIER` by
name. A value of the type is never equal to it, so the member is
always encoded, and a member that is not there decodes to the name.

A value is the default, and is left out, when it is equal to it. A
`BIT STRING` is compared without the bits past its length, and, if it
has named bits, without its trailing zero bits. An `INTEGER` given
by the name of a named number is compared as its number, and a `REAL`
that is an integer as the number it is.

### Constraints

A type has a size, a range, a permitted alphabet and an inner type
constraint, each of which may be none. They are its effective
constraints, which the compiler has worked out of all the constraints
that apply to it, as X.680 50 and 51 and X.691 10.3 say:

- A union has the values of each of its parts, and an intersection
  those of all of them (X.680 50.2, 50.4). What follows `EXCEPT` is left out
  (X.691 10.3.21).

- A constraint applied to a type that has constraints is applied to
  their root, and the result is extensible only if the last one is
  (X.680 50.11). So the size of `A ::= B (SIZE (1..10))` with
  `B ::= OCTET STRING (SIZE (1..4))` is 1 to 4. It is the same for a
  field of an object class, of which the constraints are applied
  first. A table, contents or user-defined constraint changes
  neither (X.680 50.11 NOTE).

- A contained subtype, as `INTEGER (A)`, has the root of `A`, and is
  not extensible because `A` is (X.680 51.3.3).

- An `...` makes each of the size, the range and the permitted
  alphabet of its constraint extensible (X.691 B.2.3.8).

- A constraint that is not PER-visible is all values: a single value
  of a string, a pattern and an inner type constraint (X.691 10.3.8,
  10.3.17, 10.3.18). So is a union of which a part is not visible (X.691
  10.3.21). A permitted alphabet that is extensible is not visible,
  and is as if it was not there (X.691 10.3.11, 10.3.20), so the
  effective permitted alphabet is never extensible.

The compiler writes the root of each as values and ranges in
ascending order that neither overlap nor touch, followed by an
`ExtensionMarker` if it is extensible: `INTEGER (1..5 | 10..20, ...)`
has `restricted_to` `[Range(1, 5), Range(10, 20), ExtensionMarker]`.
A size or a range with no bound has `Min` or `Max` as one. A size is
never less than 0, so `SIZE (MIN..4)` is `Range(0, 4)`. Only an
`INTEGER` has a `restricted_to`, but a `REAL` has the one it was
written with, which no codec looks at; only a `BIT STRING`, an
`OCTET STRING`, a `SEQUENCE OF`, a `SET OF` and a character string
have a `size`; and only a character string has a
`permitted_alphabet`, as `Range` nodes of single characters.

A type with no `size`, `restricted_to` or `permitted_alphabet` of
its own has those of the type its `ReferencedType` names, if it is
one. A `ReferencedType` has one of its own if its constraints change
that of the type it names, and that one takes its place.

Reading them:

- The elements of a `size` and a `restricted_to` before an
  `ExtensionMarker` are the root, and the type has any value that is
  one of them, or in one of them. A single value `v` is the range `v`
  to `v`. A `Min` or a `Max` bound is no bound. The codecs use the
  least and the greatest values of the root as its bounds (X.691
  10.3.10).

- An `Excluded` bound of a `size`, or of a `restricted_to` that is an
  integer, is the next integer in: `0<..10` is the range 1 to 10
  (X.680 51.4.3). A size with no lower bound has 0 as one (X.691
  16.4, 17.2, 20.2, 30.3). The compiler writes neither, but a reader
  takes them so.

- A bound that is not an integer is no bound for a codec. A bound that
  is a bignum is the integer it is. A runtime that can not hold it
  rejects the specification; that for Rust holds bounds of up to 128
  bits.

- An `ExtensionMarker` anywhere among the elements means the size or
  the range is extensible. What follows it is not looked at.

The permitted alphabet is all the characters of the text strings among
the elements of `permitted_alphabet`, and all the characters from the
first character of the lower bound to that of the upper bound of each
`Range` of text strings. The characters are in the order of their code
points, as PER needs them. An `ExtensionMarker` among the elements
means there is no permitted alphabet, as one that is extensible is not
PER-visible (X.691 10.3.11). There is none either if any other element
is found there. Such a `permitted_alphabet` of a `ReferencedType`
still takes the place of the one of the type it names.

The inner type constraint is `with_components` for a `SEQUENCE`, a
`SET`, a `CHOICE`, a `REAL` and a `DURATION`, and `with_component` for
a `SEQUENCE OF` and a `SET OF` (X.680 51.8). Those of a `DURATION` are
of its equivalent `SEQUENCE` (X.680 38.4.4.2), with the components
`years`, `months`, `weeks`, `days`, `hours`, `minutes`, `seconds` and
`fractional-part`. The compiler works it out as it does
the others, with these rules of its own:

- `with_components` is a list of alternatives, each a list of
  `ComponentConstraint` nodes with different names. A value satisfies
  it if it satisfies all the constraints of any of the alternatives. A
  union is the alternatives of each part, and an intersection, also of
  constraints applied one after the other, has an alternative for each
  pair of alternatives of its parts, made into one. An alternative that
  can not be satisfied, as one with a component both `PRESENT` and
  `ABSENT`, is left out, so a list of no alternatives has no value.

- The `presence` of a `ComponentConstraint` is `PRESENT` or `ABSENT`,
  and only for what can be absent: a member of a `SEQUENCE` or a `SET`
  that is `OPTIONAL`, or an alternative of a `CHOICE` (X.680 51.8.10).
  An alternative of a `CHOICE` is present if it is the one chosen. A
  `WITH COMPONENTS` that is not partial is made one that is, with the
  presence it implies: `ABSENT` for what can be absent and is not
  listed, and `PRESENT` for a listed member that is `OPTIONAL` and has
  no presence (X.680 51.8.7, 51.8.10.3).

- The `size`, `restricted_to` and `permitted_alphabet` of a
  `ComponentConstraint`, and of each `ElementConstraint` of
  `with_component`, are those of the constraint of the component or
  of the elements alone, as those of a type are written. They are
  read in the same way, and are checked with those of the type of the
  component. Its `with_components` and `with_component` are its inner
  type constraint, and `containing` is never there.

- `with_component` is a list of alternatives, each an
  `ElementConstraint`. A value satisfies it if all its elements satisfy
  the constraints of the same alternative: a union of two is not the
  elements having those of either. A union and an intersection are
  made as those of `with_components`, an intersection of two
  alternatives being one with the constraints of both.

- An `ExtensionMarker` at the end of `with_components` or
  `with_component` means the constraint is extensible, and it is not
  checked then. The compiler only writes it to take the place of one
  of the type referred to.

- What the compiler can not work out is all values, so that no value
  is rejected that should not be: a component that is not one of the
  type, and a union or an intersection that would have more than 64
  alternatives.

A `REAL` is an IEEE 754 number of 32 bits, binary32, if
`with_components` is not extensible, and in each of its alternatives
`mantissa`, `base` and `exponent` have a `restricted_to` with two
bounds that is not extensible, and if, of the least ranges that have
those of all the alternatives (X.696 12.1), that of the base is the
single value 2, that of the mantissa is in -16777215 to 16777215 and
that of the exponent in -149 to 104. It is one of 64 bits, binary64,
if instead the mantissa is in -9007199254740991 to 9007199254740991
and the exponent in -1074 to 971. Only OER uses this, as X.696 says.

`containing` and `table` are not looked at by any codec, and
`with_component` and `with_components`, but for a `REAL` in OER,
only by a constraints checker.

This is what each codec uses:

| Codec | What is used |
|---|---|
| BER, DER, XER | Nothing. |
| PER, UPER | The range of an `INTEGER`, with an upper and a lower bound, or a lower bound only (X.691 13.2). The size of a `BIT STRING`, an `OCTET STRING`, a `SEQUENCE OF` and a `SET OF`. The size and the permitted alphabet of the strings of a known multiplier: `NumericString`, `PrintableString`, `VisibleString`, `IA5String`, `BMPString` and `UniversalString` (X.691 30.1). If a size or a range is extensible, and then a value outside its root is encoded as if there was no constraint (13.1, 16.6, 17.3, 20.4, 30.4). A value outside the bounds of a size or a range that is not extensible is rejected; one between them that is not in the root, as 3 of `INTEGER (1..2 \| 4..5)`, is encoded, and only a constraints checker rejects it. |
| OER | The range of an `INTEGER` that is not extensible, which tells how many bytes it takes (X.696 8.2.1, 10). A size of one value only, and not extensible, of a `BIT STRING`, an `OCTET STRING` and the strings of a known multiplier, which is then not written (X.696 27.2). A string of a known multiplier is one of those of PER, which `UTF8String` is not. If a `REAL` is IEEE 754. |
| JER | A size of one value only of a `BIT STRING`, which is then not written. |

A runtime with a constraints checker checks the sizes
of strings, OCTET STRINGs, BIT STRINGs and SEQUENCE OFs, the range of
an INTEGER, the permitted alphabet of a string, that the name of
an alternative is one of the CHOICE, also in a member, and the inner
type constraints of the `SEQUENCE`, `SET`, `CHOICE`, `SEQUENCE OF`
and `SET OF` types. A size or a
range is checked against each of the values and ranges of its root,
and not at all if it is extensible. A string with no permitted
alphabet of its own has to have the characters of its kind:

| Kind | Characters |
|---|---|
| `NumericString` | The digits and the space. |
| `PrintableString` | The letters A to Z and a to z, the digits, the space and `'()+,-./:=?`. |
| `IA5String` | The code points 0 to 127. |
| `VisibleString` | The code points 32 to 126. |
| `BMPString` | The code points 0 to 65535, but those of surrogates. |
| any other | Any. |

### Values

| ASN.1 | Value |
|---|---|
| BOOLEAN | true or false |
| INTEGER | an integer of any size, or the name of a named number when encoding |
| REAL | a floating point number |
| NULL | nothing |
| BIT STRING | bytes and the number of bits |
| OCTET STRING | bytes |
| OBJECT IDENTIFIER | text, as `1.2.3` |
| ENUMERATED | text with the name, or a number if the runtime is told to |
| SEQUENCE, SET and EXTERNAL | the members that are there, by name |
| SEQUENCE OF and SET OF | a list |
| CHOICE | the name of the alternative and its value |
| character strings | text |
| UTCTime, GeneralizedTime and DATE-TIME | a date and a time |
| DATE | a date |
| TIME-OF-DAY | a time |
| DURATION | text, as an ISO 8601 duration |
| ANY and OpenType | the bytes of its encoding |

An `ENUMERATED` that is extensible and has a value that is not one
of its own decodes to nothing, and so does a `CHOICE` with an
alternative of a later version, with no name.

XER writes the items of a `SEQUENCE OF` and a `SET OF` as elements
named `element_name`. Without it, they are named after the type of
the items: the `name` of a `BuiltinType` or a `ReferencedType`, or
the kind of the type, as `INTEGER`, `SEQUENCE` or `CHOICE`. Spaces
in the name are written as `_`, as in `BIT_STRING`.

### Where the runtimes differ from the standards

What the runtimes here write is what `tests/vectors` holds, and a
runtime that is to agree with them does the same. Where that is
known not to be what a standard says, so that a runtime that follows
the standard writes other bytes, it is told here, to be fixed. There
is no such place known now.

## Locations

`location` tells where in the ASN.1 files something is found, as a
list of three numbers:

1. The index of the file among the files parsed, counted from 0. It
   is the index in `sources` of the metadata, unless other metadata
   was given when the file was made.

2. The line, counted from 1.

3. The column, counted from 1, as a number of characters and not of
   bytes.

It is the place of the first character of the name of a module, an
assignment, a member or a field of an object class. Only an
`.asnst` file has locations.

## Versions

The minor version is incremented when a field is added to a kind of
node, which is always last in its fields, or a key to the metadata. A
reader of an earlier minor version reads such a file by ignoring what
it does not know of.

The major version is incremented on any other change, as a new kind
of node, a field that is removed or a field that changes meaning. A
reader rejects a file of another major version.

A number of a kind and a place of a field are never given to
something else.

The flags are not part of the version. A flag is added without a new
version, as a reader rejects a file with a flag it does not know of.

## Not yet specified

- The registration of the [media types](#media-types) with IANA.

- A description of the nodes in CDDL (RFC 8610).

- Files to test a reader of the container with beyond the corpus of
  `examples/corpus`.

- Whether the metadata is to be left readable in an encrypted file.

- Whether the encrypted bytes are to be split in parts, each with a
  tag, for a device that can not keep a whole file in memory until
  its tag is checked.

- Whether the size of the file is to be part of it, for a device
  that is not told by what it receives it over.

- Whether AES-256 in Galois/Counter Mode is right for the devices in
  mind. AES-128, or ChaCha20-Poly1305 where there is no AES hardware,
  are the other candidates.

- Compression. A file of RRC 14.4.0 is 441 kB, and 72 kB with
  Zstandard.
