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

- Two bytes of flags, the most significant first, telling if the
  file is bound to what its metadata tells.

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

HEADER_SIZE = 8

# Of what a file is bound to, in bytes as UTF-8.
BOUND_TO_MAXIMUM_SIZE = 255

# The file is only for what its metadata tells it is bound to.
FLAG_BOUND = 0x0001

SUPPORTED_FLAGS = FLAG_BOUND


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


def _write_asni(metadata: Metadata | None, nodes: Any) -> bytes:
    if metadata is None:
        metadata = Metadata()

    flags = 0

    if metadata.bound_to is not None:
        flags |= FLAG_BOUND

    return b"".join(
        [
            MAGIC_ASNI,
            bytes(ASNI_VERSION),
            flags.to_bytes(2, "big"),
            cbor.dumps(_metadata_to_primitive(metadata)),
            cbor.dumps(nodes),
        ]
    )


def _check_version(version: Tuple[int, int], expected: Tuple[int, int]) -> None:
    if version[0] != expected[0]:
        raise ConvertError(
            "Unsupported version {}.{}, expected {}.x.".format(
                version[0], version[1], expected[0]
            )
        )


def _read_flags(data: bytes) -> int:
    """Returns the flags of an .asni file, after checking its version
    and that they are known.

    """

    if len(data) < HEADER_SIZE:
        raise ConvertError("Expected an .asni or .asnst file.")

    _check_version((data[4], data[5]), ASNI_VERSION)
    flags = int.from_bytes(data[6:HEADER_SIZE], "big")

    if flags & ~SUPPORTED_FLAGS:
        raise ConvertError("Unsupported flags 0x{:04x}.".format(flags))

    return flags


def _read_asni_header(data: bytes) -> Header:
    flags = _read_flags(data)

    return Header(
        kind="asni",
        version=(data[4], data[5]),
        metadata=_metadata_from_primitive(
            cbor.loads_first(data, HEADER_SIZE)[0], bool(flags & FLAG_BOUND)
        ),
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
            metadata=_metadata_from_primitive(contents.get("metadata", {}), False),
        ),
        contents.get("tree"),
    )


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

    bound_to: str | None = None
    accept_any_binding: bool = False


def _read_asni(data: bytes | bytearray, checks: _Checks) -> Any:
    data = bytes(data)

    if data[:4] != MAGIC_ASNI:
        # Tell if it is the other file.
        _read_asnst(data)

        raise ConvertError("Expected an .asni file, but got an .asnst file.")

    flags = _read_flags(data)
    metadata, offset = cbor.loads_first(data, HEADER_SIZE)
    metadata = _metadata_from_primitive(metadata, bool(flags & FLAG_BOUND))

    # Before the nodes are looked at.
    _check_binding(metadata.bound_to, checks.bound_to, checks.accept_any_binding)
    nodes, offset = cbor.loads_first(data, offset)

    if offset != len(data):
        raise ConvertError(
            "Unexpected data after the nodes at offset {}.".format(offset)
        )

    return nodes


def read_header(data: bytes | bytearray) -> Header:
    """Returns the header of given contents of an ``.asni`` or
    ``.asnst`` file, with its metadata. Nothing of an ``.asni`` file
    is checked but its version and its flags, and not what it is
    bound to.

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
) -> bytes:
    """Encode given compiled specification as the contents of an
    ``.asni`` file, with given metadata. The file is bound to what
    the metadata tells, if anything.

    >>> with open('foo.asni', 'wb') as fout:
    ...     fout.write(asnkitrt.syntax.to_asni(compiled))

    """

    return _write_asni(metadata, to_compact(compiled))


def from_asni(
    data: bytes | bytearray,
    *,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
) -> CompiledSpecification:
    """Decode given contents of an ``.asni`` file, as returned by
    :func:`to_asni()`, to a compiled specification.

    `bound_to` is what the file has to be bound to. A file that is
    not bound is then not read. A bound file is not read if `bound_to`
    is not given. With `accept_any_binding` as ``True`` a file is read
    whatever it is bound to, and whether it is bound or not.

    >>> compiled = asnkitrt.syntax.from_asni(data)

    """

    compiled = from_compact(
        _read_asni(
            data,
            _Checks(bound_to=bound_to, accept_any_binding=accept_any_binding),
        )
    )

    if not isinstance(compiled, CompiledSpecification):
        raise ConvertError("Expected a compiled specification.")

    return compiled


def from_asni_file(
    filename: str,
    *,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
) -> CompiledSpecification:
    """Same as :func:`from_asni()`, but for the ``.asni`` file with
    given name.

    >>> compiled = asnkitrt.syntax.from_asni_file('foo.asni')

    """

    with open(filename, "rb") as fin:
        return from_asni(
            fin.read(), bound_to=bound_to, accept_any_binding=accept_any_binding
        )
