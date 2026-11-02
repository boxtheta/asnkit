Corpus of ``.asni`` files
=========================

These files are licensed under the Apache License 2.0, but for
``make.sh`` and ``foo.asn``, which are code and are licensed under the
MIT License. See ``LICENSE-COMPILER`` and ``LICENSE-RUNTIMES`` in the
root of the repository.

The two ``.asni`` files here are ``foo.asn`` compiled bound and not,
for testing a reader of the format (see ``docs/file_formats.md``) in
any language. They have the types ``Question`` and ``Answer``, for any
codec.

=================  =====
File               Bound
=================  =====
``plain.asni``
``bound.asni``     yes
=================  =====

``bound.asni`` is bound to ``corpus``.

``make.sh`` makes the files again, with the same bytes.
