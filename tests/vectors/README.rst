Test vectors
============

**The test vectors are included under their own licenses.** The
specifications in ``specs/`` are compiled forms of ASN.1 modules
written by other parties (3GPP, ETSI, IETF, OMA, the authors of
asn1tools and others, taken from ``tests/files``), and the values in
``vectors.jsonl.gz`` are made from them. Each module, and what is made
from it, is under the license and the terms of its own authors.
BoxTheta does not license them, and none of the licenses of this
repository applies to them. Check those terms before you use, copy or
redistribute them.

This file is licensed under the Creative Commons Attribution 4.0
International License (CC-BY-4.0), and ``record.py`` and
``values.py``, which are code, under the MIT License. See ``COPYING``,
``LICENSE-DOCUMENTATION`` and ``LICENSE-RUNTIMES`` in the root of the
repository.

What the runtime for Rust, the one of reference, encodes and decodes,
frozen in files, for testing a runtime in any language against it. The
runtime for Python is held to them by ``tests/test_vectors.py``.

``record.py`` makes them. It takes the cases, what to encode and to
decode, from the tests of the codecs of the runtime for Python, and
has the runtime for Rust do each of them
(``packages/asnkitrt-rs/examples/record_vectors.rs``). What it does
not support is left out. ``values.py`` says how a value is written as
JSON.

``specs/`` holds the compiled specifications (``.asnc`` files, not
signed or encrypted) and ``vectors.jsonl.gz`` holds one JSON object
per line with:

``spec``
   The name of the file in ``specs``, without ``.asnc``.

``codec``
   ``ber``, ``der``, ``oer``, ``per`` or ``uper``.

``numeric_enums``, ``lenient``
   How the specification was loaded.

``operation``, ``type``
   ``encode`` or ``decode``, and the name of the type.

``value``
   The value encoded, or the one that was decoded.

``encoded``
   The bytes, as hexadecimal digits: what was made by ``encode``, or
   what was given to ``decode``.

``error``
   ``null`` if it worked, else the kind of the error, as the runtime for
   Rust names it: ``Encode``, ``Decode`` or ``Constraints``. What was
   given, ``value`` or ``encoded``, is then kept, and the other is
   ``null`` or empty. A runtime only has to fail, not with the same
   kind.

``options``
   ``check_types`` and ``check_constraints`` for ``encode``, and
   ``check_constraints`` for ``decode``. An ``encode`` of JER or XER
   may also have ``indent``, the number of spaces for each level.
