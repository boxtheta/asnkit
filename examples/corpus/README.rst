Corpus of ``.asni`` files
=========================

These files are licensed under the Apache License 2.0, but for
``make.sh`` and ``foo.asn``, which are code and are licensed under the
MIT License. See ``LICENSE-COMPILER`` and ``LICENSE-RUNTIMES`` in the
root of the repository.

The two ``.asni`` files here are ``foo.asn`` compiled, for testing a
reader of the format (see ``docs/file_formats.md``) in any language.
They have the types ``Question`` and ``Answer``, for any codec.

==================  =======
File                CRC-32
==================  =======
``plain.asni``      matches
``corrupt.asni``    does not
==================  =======

``corrupt.asni`` is ``plain.asni`` with the lowest bit of its last byte
flipped, and the CRC-32 in its header left as it was. A reader loads
``plain.asni`` and rejects ``corrupt.asni`` before it reads any of its
CBOR.

``make.sh`` makes the files again, with the same bytes.
