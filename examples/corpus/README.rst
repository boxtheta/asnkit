Corpus of ``.asni`` files
=========================

These files are licensed under the Apache License 2.0, but for
``make.sh`` and ``foo.asn``, which are code and are licensed under the
MIT License. See ``LICENSE-COMPILER`` and ``LICENSE-RUNTIMES`` in the
root of the repository.

The eight ``.asni`` files here are ``foo.asn`` compiled with every
combination of being signed, encrypted and bound, for testing a reader
of the format (see ``docs/file_formats.md``) in any language. None
has a nonce. They have the types ``Question`` and ``Answer``, for any
codec.

=================================  ======  =========  =====
File                               Signed  Encrypted  Bound
=================================  ======  =========  =====
``plain.asni``
``signed.asni``                    yes
``encrypted.asni``                         yes
``bound.asni``                                        yes
``signed-encrypted.asni``          yes     yes
``signed-bound.asni``              yes                yes
``encrypted-bound.asni``                   yes        yes
``signed-encrypted-bound.asni``    yes     yes        yes
=================================  ======  =========  =====

Bound files are bound to ``corpus``.

The keys are for these files only and secret in no way:

- ``signing.key`` is the Ed25519 private key and ``signing.pub`` its
  public key, as hexadecimal digits. Its identifier is
  ``d543cae4811fc8b033704dc6e7896ff7``.

- ``encryption.key`` is the AES-256 key, as hexadecimal digits. Its
  identifier is ``16cde2f77598fc5472df7570a510f54d``.

The identifiers are the first 16 bytes of the SHA-256 of the public
key and of the AES key, as ``asnkitcc compile`` makes them when not
given one.

``make.sh`` makes the files again. The initialization vector of an
encrypted file is random, so those are not the same bytes the next
time.
