.. Copyright (c) 2025 BoxTheta
..
.. SPDX-License-Identifier: Apache-2.0

Golden cases
============

What the Python compiler did, frozen in files, so that the compiler
that replaced it is held to it. The Python compiler, and the script
that recorded the cases from its tests, are not part of this
repository. These files are the reference now.

A case is changed by hand, in the commit that changes what the compiler
does on purpose, and then the difference is something to see in review.
``cargo test`` checks that the files named by the cases are the ones that
were frozen, by their SHA-256. A change to such a file that can not change
what is compiled, as one in a comment, is a change of the SHA-256 in the
case too.

A case
------

Each file in ``cases`` is one JSON object, or a gzip of it if it is
large:

``id``
   The name of the file, without its extension.

``string``, ``files`` or ``inline_files``
   The input: the text of a specification, the paths of
   specification files from the root of the repository (with their
   SHA-256 in ``sha256``), or files that a test made in a folder of its
   own, with their ``name`` and ``text``. Messages tell where those
   files were as ``<DIR>``.

``encoding``
   The text encoding of the files.

``origins``
   The Python tests that used the input, or ``(enumerated)`` if the
   input is only there because it is in ``tests/files`` or
   ``examples``.

``parse``
   What parsing the input makes: ``ok`` and the ``tree``, which is the
   syntax tree in the readable form of a node, or ``ok`` as false and
   the ``error``, with its ``type`` and ``message``. Neither the
   metadata nor the text of the files is kept in the tree.

``compile``
   The same for compiling, with ``nodes`` in the compact form of a node
   for a compiled specification.

What is held to
---------------

``cargo test`` runs the compiler on every case and tells how many agree.

- The tree and the nodes have to be equal as JSON values.
- An error has to be of the same kind and tell the same place, which is
  the file, the line and the column. That the message is the same too is
  not decided yet. The tests compare it whole until it is.

Where the cases come from
-------------------------

They were recorded from the tests of the Python compiler that parse and
compile, ``test_parse`` (97 inputs), ``test_resolver`` (40),
``test_compile`` (21) and ``test_two_modules`` (1), and from every
specification in ``tests/files`` and ``examples`` (65 more). The tests
of the codecs that compiled specifications to encode and decode with
now load them from ``tests/fixtures`` instead, which this compiler made.

Licenses
--------

The cases are made by the compiler, which is Apache-2.0. The cases
of the specifications taken from standards and from other organizations
(in the folders of ``tests/files`` named for them) are derived from
text that has terms of its own, which this repository does not change.
