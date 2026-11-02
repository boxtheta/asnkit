# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Read and write syntax trees and compiled specifications as files.

An ``.asni`` file holds a compiled specification. It is what a
runtime loads, also on small devices, so it is:

- The four bytes ``ASNI``.

- One byte with the major and one with the minor version of the
  file. A reader accepts any minor version of the major version it
  knows, ignoring the fields of nodes it does not know of in a later
  one.

- Four bytes of the CRC-32 of the rest of the file, the most
  significant first, as :func:`zlib.crc32` computes it, which tells a
  file that is corrupt.

- The metadata as a CBOR map.

- The nodes as CBOR in the compact form.

See docs/file_formats.md for all there is to know about the files.

An ``.asnst`` file holds a syntax tree. It is what plugins are given,
to generate code and such, so it is nothing but CBOR: one map with
``'asnst'`` as ``format``, the version as ``version``, the metadata
as ``metadata`` and the nodes in the readable form as ``tree``.

See :mod:`.wire` for the two forms of nodes.

"""

import dataclasses
import hashlib
import zlib
from typing import Any, Dict, Mapping, Tuple

from ..version import __version__
from . import cbor
from .convert import ConvertError
from .nodes import SCHEMA_VERSION, CompiledSpecification, SyntaxTree
from .wire import from_compact, from_primitive, to_compact, to_primitive

MAGIC_ASNI = b"ASNI"

# What an .asnst file says it is.
FORMAT_ASNST = "asnst"

# The version of an .asni file, of its header and its nodes, as (major,
# minor). That of an .asnst file is SCHEMA_VERSION.
ASNI_VERSION = (2, 0)

# The magic, the version and the CRC-32 of the rest of the file.
HEADER_SIZE = 10


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

    """

    generator: str | None = "asnkit " + __version__
    sources: Tuple[Source, ...] = ()
    other: Mapping[str, Any] = dataclasses.field(default_factory=dict)


@dataclasses.dataclass(frozen=True, kw_only=True)
class Header:
    """The header of a file: what it holds as ``'asni'`` or
    ``'asnst'``, the version of the file as (major, minor) and its
    metadata.

    """

    kind: str
    version: Tuple[int, int]
    metadata: Metadata


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

    if metadata.other:
        primitive["other"] = dict(metadata.other)

    return primitive


def _metadata_from_primitive(primitive: Any) -> Metadata:
    if not isinstance(primitive, dict):
        raise ConvertError("Expected the metadata as a map.")

    names = {field.name for field in dataclasses.fields(Source)}

    # Anything not known is that of a later minor version.
    return Metadata(
        generator=primitive.get("generator"),
        sources=tuple(
            Source(**{name: value for name, value in source.items() if name in names})
            for source in primitive.get("sources", [])
        ),
        other=primitive.get("other", {}),
    )


def _write_asni(metadata: Metadata | None, nodes: Any) -> bytes:
    if metadata is None:
        metadata = Metadata()

    rest = cbor.dumps(_metadata_to_primitive(metadata)) + cbor.dumps(nodes)

    return b"".join(
        [MAGIC_ASNI, bytes(ASNI_VERSION), zlib.crc32(rest).to_bytes(4, "big"), rest]
    )


def _check_version(version: Tuple[int, int], expected: Tuple[int, int]) -> None:
    if version[0] != expected[0]:
        raise ConvertError(
            "Unsupported version {}.{}, expected {}.x.".format(
                version[0], version[1], expected[0]
            )
        )


def _check_prefix(data: bytes) -> None:
    """Checks the version of an .asni file and the CRC-32 of the rest
    of it, before any of its CBOR is read.

    """

    if len(data) < HEADER_SIZE:
        raise ConvertError("Expected an .asni or .asnst file.")

    _check_version((data[4], data[5]), ASNI_VERSION)

    if zlib.crc32(data[HEADER_SIZE:]) != int.from_bytes(data[6:HEADER_SIZE], "big"):
        raise ConvertError(
            "The CRC-32 of the file does not match: the file is corrupt."
        )


def _read_asni_header(data: bytes) -> Header:
    _check_prefix(data)

    return Header(
        kind="asni",
        version=(data[4], data[5]),
        metadata=_metadata_from_primitive(cbor.loads_first(data, HEADER_SIZE)[0]),
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

    _check_version((version[0], version[1]), SCHEMA_VERSION)

    return (
        Header(
            kind="asnst",
            version=(version[0], version[1]),
            metadata=_metadata_from_primitive(contents.get("metadata", {})),
        ),
        contents.get("tree"),
    )


def _read_asni(data: bytes | bytearray) -> Any:
    data = bytes(data)

    if data[:4] != MAGIC_ASNI:
        # Tell if it is the other file.
        _read_asnst(data)

        raise ConvertError("Expected an .asni file, but got an .asnst file.")

    _check_prefix(data)
    metadata, offset = cbor.loads_first(data, HEADER_SIZE)
    _metadata_from_primitive(metadata)
    nodes, offset = cbor.loads_first(data, offset)

    if offset != len(data):
        raise ConvertError(
            "Unexpected data after the nodes at offset {}.".format(offset)
        )

    return nodes


def read_header(data: bytes | bytearray) -> Header:
    """Returns the header of given contents of an ``.asni`` or
    ``.asnst`` file, with its metadata. Nothing of an ``.asni`` file
    is checked but its version and its CRC-32, and its nodes are not
    read.

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
) -> bytes:
    """Encode given compiled specification as the contents of an
    ``.asni`` file, with given metadata, and the CRC-32 of both in its
    header.

    >>> with open('foo.asni', 'wb') as fout:
    ...     fout.write(asnkitrt.syntax.to_asni(compiled))

    """

    return _write_asni(metadata, to_compact(compiled))


def from_asni(data: bytes | bytearray) -> CompiledSpecification:
    """Decode given contents of an ``.asni`` file, as returned by
    :func:`to_asni()`, to a compiled specification. A file whose
    CRC-32 does not match is corrupt, and is not read.

    >>> compiled = asnkitrt.syntax.from_asni(data)

    """

    compiled = from_compact(_read_asni(data))

    if not isinstance(compiled, CompiledSpecification):
        raise ConvertError("Expected a compiled specification.")

    return compiled


def from_asni_file(filename: str) -> CompiledSpecification:
    """Same as :func:`from_asni()`, but for the ``.asni`` file with
    given name.

    >>> compiled = asnkitrt.syntax.from_asni_file('foo.asni')

    """

    with open(filename, "rb") as fin:
        return from_asni(fin.read())
