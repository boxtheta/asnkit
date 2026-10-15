# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Read and write syntax trees and compiled specifications as files.

An ``.asni`` file holds a compiled specification. It is what a
runtime loads, also on small devices, so it is:

- The four bytes ``ASNI``.

- One byte with the major and one with the minor version of the
  layout of its nodes. A reader accepts any minor version of the
  major version it knows, ignoring the fields of nodes it does not
  know of in a later one.

- Two bytes of flags, the most significant first, telling if the
  file is signed, if it is encrypted, if its nonce is to be checked
  and if it is bound to what its metadata tells.

- Three bytes of nonce, which are only looked at if the flag for it
  is set, and zero if not.

- If signed, the 16 bytes of the identifier of the key it is signed
  with and the 64 bytes of an Ed25519 signature of the SHA-256 digest
  of all of the file but the signature.

- If encrypted, the 16 bytes of the identifier of the key it is
  encrypted with, the 12 bytes of the initialization vector and the
  16 bytes of the tag of AES-256 in Galois/Counter Mode.

- The metadata as a CBOR map.

- The nodes as CBOR in the compact form.

The two last are encrypted as one in an encrypted file.

Signing and encrypting need the cryptography package, which is only
imported for a file that is signed or encrypted. See
docs/file_formats.md for all there is to know about the files.

An ``.asnst`` file holds a syntax tree. It is what plugins are given,
to generate code and such, so it is nothing but CBOR: one map with
``'asnst'`` as ``format``, the version as ``version``, the metadata
as ``metadata`` and the nodes in the readable form as ``tree``.

See :mod:`.wire` for the two forms of nodes.

"""

import dataclasses
import hashlib
import os
from typing import Any, Callable, Dict, Mapping, Tuple, Union

from ..version import __version__
from . import cbor
from .convert import ConvertError
from .nodes import SCHEMA_VERSION, CompiledSpecification, SyntaxTree
from .wire import from_compact, from_primitive, to_compact, to_primitive

MAGIC_ASNI = b"ASNI"

# What an .asnst file says it is.
FORMAT_ASNST = "asnst"

HEADER_SIZE = 11

NONCE_SIZE = 3
NONCE_MAXIMUM = 2 ** (8 * NONCE_SIZE) - 1

# Of what a file is bound to, in bytes as UTF-8.
BOUND_TO_MAXIMUM_SIZE = 255

# What the nonce of a file has to be: a number, or a function given
# it that returns True if it will do.
NonceCheck = Union[int, Callable[[int], bool], None]

# The file has a signature, its metadata and nodes are encrypted, its
# nonce is to be checked, and it is only for what its metadata tells
# it is bound to.
FLAG_SIGNED = 0x0001
FLAG_ENCRYPTED = 0x0002
FLAG_NONCE = 0x0004
FLAG_BOUND = 0x0008

SUPPORTED_FLAGS = FLAG_SIGNED | FLAG_ENCRYPTED | FLAG_NONCE | FLAG_BOUND

# Of the identifier of a key, of an Ed25519 private or public key and
# signature, and of an AES-256 key, initialization vector and tag.
KEY_ID_SIZE = 16
KEY_SIZE = 32
SIGNATURE_SIZE = 64
IV_SIZE = 12
TAG_SIZE = 16

# A key to sign or encrypt with: its identifier and the key itself.
Key = Tuple[bytes, bytes]

# The keys to verify or decrypt with: identifier to key.
Keys = Mapping[bytes, bytes]


@dataclasses.dataclass(frozen=True, kw_only=True)
class Source:
    """An ASN.1 file a syntax tree or compiled specification was made
    from: any of its name, the SHA-256 of its contents as hexadecimal
    digits, and its contents.

    """

    name: str | None = None
    sha256: str | None = None
    text: str | None = None

    @classmethod
    def from_text(
        cls, text: str, name: str | None = None, include_text: bool = False
    ) -> "Source":
        """Returns the source with given contents, which are only part
        of it if `include_text` is ``True``.

        """

        return cls(
            name=name,
            sha256=hashlib.sha256(text.encode("utf-8")).hexdigest(),
            text=text if include_text else None,
        )


@dataclasses.dataclass(frozen=True, kw_only=True)
class Metadata:
    """What is known about a syntax tree or compiled specification in
    a file: what made it, the ASN.1 files it was made from, and
    anything else as strings, numbers, lists and dictionaries with
    strings as keys in `other`.

    `bound_to` is what an ``.asni`` file is only for, as any string of
    at most 255 bytes as UTF-8, or ``None`` for a file that is not
    bound. A reader only reads a bound file if told to expect that
    string. An ``.asnst`` file is never bound.

    """

    generator: str | None = "asnkit " + __version__
    sources: Tuple[Source, ...] = ()
    bound_to: str | None = None
    other: Mapping[str, Any] = dataclasses.field(default_factory=dict)


@dataclasses.dataclass(frozen=True, kw_only=True)
class Header:
    """The header of a file: what it holds as ``'asni'`` or
    ``'asnst'``, the version of the layout of its nodes as (major,
    minor) and its metadata, which is ``None`` for an encrypted file.

    Of an ``.asni`` file it also has the nonce if there is one to
    check, the identifier of the key it is signed with if signed, and
    that of the key it is encrypted with if encrypted.

    """

    kind: str
    version: Tuple[int, int]
    metadata: Metadata | None
    nonce: int | None = None
    signed_by: bytes | None = None
    encrypted_with: bytes | None = None


def _metadata_to_primitive(metadata: Metadata) -> Dict[str, Any]:
    primitive: Dict[str, Any] = {}

    if metadata.generator is not None:
        primitive["generator"] = metadata.generator

    if metadata.sources:
        primitive["sources"] = [
            {
                field.name: getattr(source, field.name)
                for field in dataclasses.fields(source)
                if getattr(source, field.name) is not None
            }
            for source in metadata.sources
        ]

    if metadata.bound_to is not None:
        if not isinstance(metadata.bound_to, str):
            raise ConvertError(
                "Expected what the file is bound to as a string, but got {!r}.".format(
                    metadata.bound_to
                )
            )

        if len(metadata.bound_to.encode("utf-8")) > BOUND_TO_MAXIMUM_SIZE:
            raise ConvertError(
                "Expected what the file is bound to as at most {} bytes as "
                "UTF-8.".format(BOUND_TO_MAXIMUM_SIZE)
            )

        primitive["bound_to"] = metadata.bound_to

    if metadata.other:
        primitive["other"] = dict(metadata.other)

    return primitive


def _metadata_from_primitive(primitive: Any, is_bound: bool) -> Metadata:
    if not isinstance(primitive, dict):
        raise ConvertError("Expected the metadata as a map.")

    names = {field.name for field in dataclasses.fields(Source)}
    bound_to = None

    # Only looked at in a bound file.
    if is_bound:
        bound_to = primitive.get("bound_to")

        if not isinstance(bound_to, str):
            raise ConvertError("Expected a bound file to tell what it is bound to.")

        if len(bound_to.encode("utf-8")) > BOUND_TO_MAXIMUM_SIZE:
            raise ConvertError(
                "Expected what the file is bound to as at most {} bytes as "
                "UTF-8.".format(BOUND_TO_MAXIMUM_SIZE)
            )

    # Anything not known is that of a later minor version.
    return Metadata(
        generator=primitive.get("generator"),
        sources=tuple(
            Source(**{name: value for name, value in source.items() if name in names})
            for source in primitive.get("sources", [])
        ),
        bound_to=bound_to,
        other=primitive.get("other", {}),
    )


def _import_cryptography(what: str, extra: str) -> None:
    """Raises an exception telling how to get the cryptography
    package if it is not there.

    """

    try:
        import cryptography  # noqa: F401
    except ImportError:
        raise ConvertError(
            "{} file needs the cryptography package, which is installed with "
            "the extra '{}' of this package.".format(what, extra)
        )


def _check_key(key: Any, what: str) -> Key:
    if (
        not isinstance(key, tuple)
        or len(key) != 2
        or not all(isinstance(item, (bytes, bytearray)) for item in key)
        or len(key[0]) != KEY_ID_SIZE
        or len(key[1]) != KEY_SIZE
    ):
        raise ConvertError(
            "Expected the key to {} with as its identifier of {} bytes and "
            "the key of {} bytes.".format(what, KEY_ID_SIZE, KEY_SIZE)
        )

    return bytes(key[0]), bytes(key[1])


def _find_key(keys: Keys | None, key_id: bytes, what: str, to: str) -> bytes:
    if keys is None:
        raise ConvertError(
            "The file is {}, but no keys to {} it with are given.".format(what, to)
        )

    for candidate_id, key in keys.items():
        if bytes(candidate_id) == key_id:
            if len(key) != KEY_SIZE:
                raise ConvertError(
                    "Expected the key {} as {} bytes.".format(key_id.hex(), KEY_SIZE)
                )

            return bytes(key)

    raise ConvertError(
        "The file is {} with the key {}, which is not among those given.".format(
            what, key_id.hex()
        )
    )


def _write_asni(
    metadata: Metadata | None,
    nodes: Any,
    nonce: int | None,
    signing_key: Key | None,
    encryption_key: Key | None,
) -> bytes:
    if metadata is None:
        metadata = Metadata()

    flags = 0

    if metadata.bound_to is not None:
        flags |= FLAG_BOUND

    if signing_key is not None:
        flags |= FLAG_SIGNED

    if encryption_key is not None:
        flags |= FLAG_ENCRYPTED

    if nonce is None:
        # Nobody looks at it.
        nonce = 0
    elif isinstance(nonce, bool) or not isinstance(nonce, int):
        raise ConvertError(
            "Expected a nonce as an integer, but got {!r}.".format(nonce)
        )
    elif not 0 <= nonce <= NONCE_MAXIMUM:
        raise ConvertError(
            "Expected a nonce of 0 to {}, but got {}.".format(NONCE_MAXIMUM, nonce)
        )
    else:
        flags |= FLAG_NONCE

    header = b"".join(
        [
            MAGIC_ASNI,
            bytes(SCHEMA_VERSION),
            flags.to_bytes(2, "big"),
            nonce.to_bytes(NONCE_SIZE, "big"),
        ]
    )
    contents = cbor.dumps(_metadata_to_primitive(metadata)) + cbor.dumps(nodes)
    encryption = b""
    signature = b""

    # Encrypted first and then signed, so that the signature is
    # checked without the key to decrypt with.
    if encryption_key is not None:
        key_id, key = _check_key(encryption_key, "encrypt")
        _import_cryptography("An encrypted", "encryption")

        from cryptography.hazmat.primitives.ciphers.aead import AESGCM

        iv = os.urandom(IV_SIZE)
        encrypted = AESGCM(key).encrypt(iv, contents, header + key_id + iv)
        contents = encrypted[:-TAG_SIZE]
        encryption = key_id + iv + encrypted[-TAG_SIZE:]

    if signing_key is not None:
        key_id, key = _check_key(signing_key, "sign")
        _import_cryptography("A signed", "signing")

        from cryptography.hazmat.primitives.asymmetric.ed25519 import (
            Ed25519PrivateKey,
        )

        digest = hashlib.sha256(header + key_id + encryption + contents).digest()
        signature = key_id + Ed25519PrivateKey.from_private_bytes(key).sign(digest)

    return header + signature + encryption + contents


def _check_version(version: Tuple[int, int]) -> None:
    if version[0] != SCHEMA_VERSION[0]:
        raise ConvertError(
            "Unsupported version {}.{}, expected {}.x.".format(
                version[0], version[1], SCHEMA_VERSION[0]
            )
        )


@dataclasses.dataclass(frozen=True, kw_only=True)
class _Prefix:
    """What comes before the metadata in an .asni file."""

    version: Tuple[int, int]
    flags: int
    nonce: int | None
    signed_by: bytes | None
    signature: bytes
    encrypted_with: bytes | None
    iv: bytes
    tag: bytes
    size: int


def _read_prefix(data: bytes) -> _Prefix:
    if len(data) < HEADER_SIZE:
        raise ConvertError("Expected an .asni or .asnst file.")

    version = (data[4], data[5])
    flags = int.from_bytes(data[6:8], "big")
    _check_version(version)

    if flags & ~SUPPORTED_FLAGS:
        raise ConvertError("Unsupported flags 0x{:04x}.".format(flags))

    nonce = None
    signed_by = None
    signature = b""
    encrypted_with = None
    iv = b""
    tag = b""
    offset = HEADER_SIZE

    def take(size: int) -> bytes:
        nonlocal offset

        if offset + size > len(data):
            raise ConvertError("Out of data.")

        offset += size

        return data[offset - size : offset]

    if flags & FLAG_NONCE:
        nonce = int.from_bytes(data[8:HEADER_SIZE], "big")

    if flags & FLAG_SIGNED:
        signed_by = take(KEY_ID_SIZE)
        signature = take(SIGNATURE_SIZE)

    if flags & FLAG_ENCRYPTED:
        encrypted_with = take(KEY_ID_SIZE)
        iv = take(IV_SIZE)
        tag = take(TAG_SIZE)

    return _Prefix(
        version=version,
        flags=flags,
        nonce=nonce,
        signed_by=signed_by,
        signature=signature,
        encrypted_with=encrypted_with,
        iv=iv,
        tag=tag,
        size=offset,
    )


def _read_asni_header(data: bytes) -> Header:
    prefix = _read_prefix(data)
    metadata = None

    # Not known without the key of an encrypted file.
    if prefix.encrypted_with is None:
        metadata = _metadata_from_primitive(
            cbor.loads_first(data, prefix.size)[0], bool(prefix.flags & FLAG_BOUND)
        )

    return Header(
        kind="asni",
        version=prefix.version,
        metadata=metadata,
        nonce=prefix.nonce,
        signed_by=prefix.signed_by,
        encrypted_with=prefix.encrypted_with,
    )


def _read_asnst(data: bytes) -> Tuple[Header, Any]:
    """Returns the header of given contents of an .asnst file and its
    nodes.

    """

    try:
        contents = cbor.loads(data)
    except ConvertError:
        contents = None

    if not isinstance(contents, dict) or contents.get("format") != FORMAT_ASNST:
        raise ConvertError("Expected an .asni or .asnst file.")

    version = contents.get("version")

    if (
        not isinstance(version, list)
        or len(version) != 2
        or not all(type(number) is int for number in version)
    ):
        raise ConvertError("Expected the version of an .asnst file as two numbers.")

    _check_version((version[0], version[1]))

    return (
        Header(
            kind="asnst",
            version=(version[0], version[1]),
            metadata=_metadata_from_primitive(contents.get("metadata", {}), False),
        ),
        contents.get("tree"),
    )


def _check_nonce(nonce: int | None, check: NonceCheck, accept_any: bool) -> None:
    """Raises an exception if given nonce of a file, which is None if
    it has none to check, is not what it has to be.

    """

    if accept_any:
        return

    if check is None:
        if nonce is not None:
            raise ConvertError(
                "The file has the nonce {} to check, but no nonce is expected.".format(
                    nonce
                )
            )
    elif nonce is None:
        raise ConvertError("Expected a file with a nonce to check.")
    elif callable(check):
        if not check(nonce):
            raise ConvertError(
                "The nonce {} of the file is not accepted.".format(nonce)
            )
    elif isinstance(check, bool) or nonce != check:
        raise ConvertError("Expected the nonce {}, but got {}.".format(check, nonce))


def _check_binding(
    bound_to: str | None, expected: str | None, accept_any: bool
) -> None:
    """Raises an exception if what a file is bound to, which is None
    if it is not bound, is not what it has to be.

    """

    if accept_any:
        return

    if expected is None:
        if bound_to is not None:
            raise ConvertError(
                "The file is bound to '{}', but no binding is expected.".format(
                    bound_to
                )
            )
    elif bound_to is None:
        raise ConvertError("Expected a file bound to '{}'.".format(expected))
    elif bound_to.encode("utf-8") != expected.encode("utf-8"):
        raise ConvertError(
            "Expected a file bound to '{}', but it is bound to '{}'.".format(
                expected, bound_to
            )
        )


@dataclasses.dataclass(frozen=True, kw_only=True)
class _Checks:
    """What an .asni file has to be to be read."""

    nonce: NonceCheck = None
    accept_any_nonce: bool = False
    bound_to: str | None = None
    accept_any_binding: bool = False
    verification_keys: Keys | None = None
    accept_any_signature: bool = False
    decryption_keys: Keys | None = None


def _check_signature(data: bytes, prefix: _Prefix, checks: _Checks) -> None:
    if checks.accept_any_signature:
        return

    if prefix.signed_by is None:
        if checks.verification_keys is not None:
            raise ConvertError("Expected a signed file.")

        return

    key = _find_key(checks.verification_keys, prefix.signed_by, "signed", "verify")
    _import_cryptography("A signed", "signing")

    from cryptography.exceptions import InvalidSignature
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

    # All of the file but the signature.
    signature_offset = HEADER_SIZE + KEY_ID_SIZE
    digest = hashlib.sha256(
        data[:signature_offset] + data[signature_offset + SIGNATURE_SIZE :]
    ).digest()

    try:
        Ed25519PublicKey.from_public_bytes(key).verify(prefix.signature, digest)
    except InvalidSignature:
        raise ConvertError("The signature of the file is wrong.")


def _decrypt(data: bytes, prefix: _Prefix, checks: _Checks) -> bytes:
    """Returns the metadata and the nodes of given file, decrypted if
    they are encrypted.

    """

    contents = data[prefix.size :]

    if prefix.encrypted_with is None:
        if checks.decryption_keys is not None:
            raise ConvertError("Expected an encrypted file.")

        return contents

    key = _find_key(
        checks.decryption_keys, prefix.encrypted_with, "encrypted", "decrypt"
    )
    _import_cryptography("An encrypted", "encryption")

    from cryptography.exceptions import InvalidTag
    from cryptography.hazmat.primitives.ciphers.aead import AESGCM

    try:
        return AESGCM(key).decrypt(
            prefix.iv,
            contents + prefix.tag,
            data[:HEADER_SIZE] + prefix.encrypted_with + prefix.iv,
        )
    except InvalidTag:
        raise ConvertError(
            "The file is not what was encrypted, or the key is not the one it "
            "was encrypted with."
        )


def _read_asni(data: bytes | bytearray, checks: _Checks) -> Any:
    data = bytes(data)

    if data[:4] != MAGIC_ASNI:
        # Tell if it is the other file.
        _read_asnst(data)

        raise ConvertError("Expected an .asni file, but got an .asnst file.")

    prefix = _read_prefix(data)

    # Before anything else in the file is looked at.
    _check_signature(data, prefix, checks)
    _check_nonce(prefix.nonce, checks.nonce, checks.accept_any_nonce)
    contents = _decrypt(data, prefix, checks)
    metadata, offset = cbor.loads_first(contents, 0)
    metadata = _metadata_from_primitive(metadata, bool(prefix.flags & FLAG_BOUND))
    _check_binding(metadata.bound_to, checks.bound_to, checks.accept_any_binding)
    nodes, offset = cbor.loads_first(contents, offset)

    if offset != len(contents):
        raise ConvertError(
            "Unexpected data after the nodes at offset {}.".format(prefix.size + offset)
        )

    return nodes


def read_header(data: bytes | bytearray) -> Header:
    """Returns the header of given contents of an ``.asni`` or
    ``.asnst`` file, with its metadata. Nothing of an ``.asni`` file
    is checked: not its signature, not its nonce and not what it is
    bound to. The metadata of an encrypted file is ``None``, as it is
    not decrypted.

    >>> asnkitrt.syntax.read_header(data).metadata.sources
    (Source(name='foo.asn', sha256='8c3f...', text=None),)

    """

    data = bytes(data)

    if data[:4] == MAGIC_ASNI:
        return _read_asni_header(data)

    return _read_asnst(data)[0]


def to_asnst(tree: SyntaxTree, metadata: Metadata | None = None) -> bytes:
    """Encode given syntax tree as the contents of an ``.asnst`` file,
    with given metadata.

    >>> with open('foo.asnst', 'wb') as fout:
    ...     fout.write(asnkitrt.syntax.to_asnst(tree))

    """

    if metadata is None:
        metadata = Metadata()

    if metadata.bound_to is not None:
        raise ConvertError("Only an .asni file can be bound to something.")

    return cbor.dumps(
        {
            "format": FORMAT_ASNST,
            "version": list(SCHEMA_VERSION),
            "metadata": _metadata_to_primitive(metadata),
            "tree": to_primitive(tree),
        }
    )


def from_asnst(data: bytes | bytearray) -> SyntaxTree:
    """Decode given contents of an ``.asnst`` file, as returned by
    :func:`to_asnst()`, to a syntax tree.

    >>> tree = asnkitrt.syntax.from_asnst(data)

    """

    data = bytes(data)

    if data[:4] == MAGIC_ASNI:
        raise ConvertError("Expected an .asnst file, but got an .asni file.")

    # Fields of a later minor version are not known.
    tree = from_primitive(_read_asnst(data)[1], is_strict=False)

    if not isinstance(tree, SyntaxTree):
        raise ConvertError("Expected a syntax tree.")

    return tree


def from_asnst_file(filename: str) -> SyntaxTree:
    """Same as :func:`from_asnst()`, but for the ``.asnst`` file with
    given name.

    >>> tree = asnkitrt.syntax.from_asnst_file('foo.asnst')

    """

    with open(filename, "rb") as fin:
        return from_asnst(fin.read())


def to_asni(
    compiled: CompiledSpecification,
    metadata: Metadata | None = None,
    nonce: int | None = None,
    signing_key: Key | None = None,
    encryption_key: Key | None = None,
) -> bytes:
    """Encode given compiled specification as the contents of an
    ``.asni`` file, with given metadata.

    Give `nonce` as a number of at most 24 bits for a file with a
    nonce to check.

    Give `signing_key` for a signed file, as the identifier of the
    key, of 16 bytes, and the Ed25519 private key, of 32 bytes. Give
    `encryption_key` for an encrypted file, as the identifier of the
    key, of 16 bytes, and the AES-256 key, of 32 bytes. Both need the
    cryptography package.

    >>> with open('foo.asni', 'wb') as fout:
    ...     fout.write(asnkitrt.syntax.to_asni(compiled))

    """

    return _write_asni(
        metadata, to_compact(compiled), nonce, signing_key, encryption_key
    )


def from_asni(
    data: bytes | bytearray,
    nonce: NonceCheck = None,
    accept_any_nonce: bool = False,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
    verification_keys: Keys | None = None,
    accept_any_signature: bool = False,
    decryption_keys: Keys | None = None,
) -> CompiledSpecification:
    """Decode given contents of an ``.asni`` file, as returned by
    :func:`to_asni()`, to a compiled specification.

    `nonce` is what the nonce of the file has to be: a number it has
    to be the same as, or a function that is given it and returns
    ``True`` if it will do, as ``lambda nonce: nonce > last``. A file
    without a nonce to check is then not read. A file with a nonce to
    check is not read if `nonce` is not given. With `accept_any_nonce`
    as ``True`` a file is read whatever its nonce, and whether it has
    one to check or not.

    A function given as `nonce` must not keep the nonce it is given,
    as the file may still be found to be bad. Keep that of
    :func:`read_header()` once the file has been read instead.

    `bound_to` is what the file has to be bound to. A file that is
    not bound is then not read. A bound file is not read if `bound_to`
    is not given. With `accept_any_binding` as ``True`` a file is read
    whatever it is bound to, and whether it is bound or not.

    `verification_keys` are the Ed25519 public keys of those trusted
    to sign files, as a dictionary of the identifier of each key, of
    16 bytes, to the key, of 32 bytes. A file is then only read if
    signed with one of them. A signed file is not read if
    `verification_keys` is not given. With `accept_any_signature` as
    ``True`` a file is read without its signature being checked, and
    whether it is signed or not.

    `decryption_keys` are the AES-256 keys to decrypt files with, as
    a dictionary of the identifier of each key, of 16 bytes, to the
    key, of 32 bytes. A file is then only read if encrypted with one
    of them. An encrypted file is not read if `decryption_keys` is not
    given.

    Signed and encrypted files need the cryptography package.

    >>> compiled = asnkitrt.syntax.from_asni(data)

    """

    compiled = from_compact(
        _read_asni(
            data,
            _Checks(
                nonce=nonce,
                accept_any_nonce=accept_any_nonce,
                bound_to=bound_to,
                accept_any_binding=accept_any_binding,
                verification_keys=verification_keys,
                accept_any_signature=accept_any_signature,
                decryption_keys=decryption_keys,
            ),
        )
    )

    if not isinstance(compiled, CompiledSpecification):
        raise ConvertError("Expected a compiled specification.")

    return compiled


def from_asni_file(
    filename: str,
    nonce: NonceCheck = None,
    accept_any_nonce: bool = False,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
    verification_keys: Keys | None = None,
    accept_any_signature: bool = False,
    decryption_keys: Keys | None = None,
) -> CompiledSpecification:
    """Same as :func:`from_asni()`, but for the ``.asni`` file with
    given name.

    >>> compiled = asnkitrt.syntax.from_asni_file('foo.asni')

    """

    with open(filename, "rb") as fin:
        return from_asni(
            fin.read(),
            nonce,
            accept_any_nonce,
            bound_to,
            accept_any_binding,
            verification_keys,
            accept_any_signature,
            decryption_keys,
        )
