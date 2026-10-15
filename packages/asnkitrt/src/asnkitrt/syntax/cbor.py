# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Encode and decode the part of CBOR (RFC 8949) a syntax tree as
primitives needs: integers, floats, text strings, booleans, null,
arrays and maps with text string keys.

"""

import struct
from typing import Any, List, Tuple

from .convert import ConvertError

MAJOR_UNSIGNED = 0
MAJOR_NEGATIVE = 1
MAJOR_BYTES = 2
MAJOR_TEXT = 3
MAJOR_ARRAY = 4
MAJOR_MAP = 5
MAJOR_TAG = 6
MAJOR_SIMPLE = 7

TAG_POSITIVE_BIGNUM = 2
TAG_NEGATIVE_BIGNUM = 3

SIMPLE_FALSE = 20
SIMPLE_TRUE = 21
SIMPLE_NULL = 22
FLOAT_16 = 25
FLOAT_32 = 26
FLOAT_64 = 27


def _encode_head(major: int, argument: int, encoded: List[bytes]) -> None:
    if argument < 24:
        encoded.append(bytes([(major << 5) | argument]))
    elif argument < 2**8:
        encoded.append(bytes([(major << 5) | 24, argument]))
    elif argument < 2**16:
        encoded.append(bytes([(major << 5) | 25]) + struct.pack(">H", argument))
    elif argument < 2**32:
        encoded.append(bytes([(major << 5) | 26]) + struct.pack(">I", argument))
    else:
        encoded.append(bytes([(major << 5) | 27]) + struct.pack(">Q", argument))


def _encode_integer(value: int, encoded: List[bytes]) -> None:
    major = MAJOR_UNSIGNED
    tag = TAG_POSITIVE_BIGNUM

    if value < 0:
        major = MAJOR_NEGATIVE
        tag = TAG_NEGATIVE_BIGNUM
        value = -1 - value

    if value < 2**64:
        _encode_head(major, value, encoded)
    else:
        data = value.to_bytes((value.bit_length() + 7) // 8, "big")
        _encode_head(MAJOR_TAG, tag, encoded)
        _encode_head(MAJOR_BYTES, len(data), encoded)
        encoded.append(data)


def _encode(value: Any, encoded: List[bytes]) -> None:
    if value is None:
        _encode_head(MAJOR_SIMPLE, SIMPLE_NULL, encoded)
    elif value is True:
        _encode_head(MAJOR_SIMPLE, SIMPLE_TRUE, encoded)
    elif value is False:
        _encode_head(MAJOR_SIMPLE, SIMPLE_FALSE, encoded)
    elif isinstance(value, int):
        _encode_integer(value, encoded)
    elif isinstance(value, float):
        encoded.append(bytes([(MAJOR_SIMPLE << 5) | FLOAT_64]))
        encoded.append(struct.pack(">d", value))
    elif isinstance(value, str):
        data = value.encode("utf-8")
        _encode_head(MAJOR_TEXT, len(data), encoded)
        encoded.append(data)
    elif isinstance(value, (list, tuple)):
        _encode_head(MAJOR_ARRAY, len(value), encoded)

        for item in value:
            _encode(item, encoded)
    elif isinstance(value, dict):
        _encode_head(MAJOR_MAP, len(value), encoded)

        for key, item in value.items():
            if not isinstance(key, str):
                raise ConvertError("Unsupported map key {!r}.".format(key))

            _encode(key, encoded)
            _encode(item, encoded)
    else:
        raise ConvertError("Unsupported value {!r}.".format(value))


def dumps(value: Any) -> bytes:
    """Encode given primitives as CBOR."""

    encoded: List[bytes] = []
    _encode(value, encoded)

    return b"".join(encoded)


def _decode_head(data: bytes, offset: int) -> Tuple[int, int, int, int]:
    """Returns the major type, the additional information, the
    argument and the offset of what follows.

    """

    initial = data[offset]
    major = initial >> 5
    info = initial & 0x1F
    offset += 1

    if info < 24:
        return major, info, info, offset

    if info > 27:
        raise ConvertError(
            "Unsupported CBOR additional information {} at offset {}.".format(
                info, offset - 1
            )
        )

    size = 1 << (info - 24)
    end = offset + size

    if end > len(data):
        raise IndexError()

    return major, info, int.from_bytes(data[offset:end], "big"), end


def _decode_bytes(data: bytes, offset: int, size: int) -> Tuple[bytes, int]:
    end = offset + size

    if end > len(data):
        raise IndexError()

    return data[offset:end], end


def _decode(data: bytes, offset: int) -> Tuple[Any, int]:
    major, info, argument, offset = _decode_head(data, offset)
    value: Any

    if major == MAJOR_UNSIGNED:
        return argument, offset
    elif major == MAJOR_NEGATIVE:
        return -1 - argument, offset
    elif major == MAJOR_TEXT:
        encoded, offset = _decode_bytes(data, offset, argument)

        return encoded.decode("utf-8"), offset
    elif major == MAJOR_ARRAY:
        items = []

        for _ in range(argument):
            value, offset = _decode(data, offset)
            items.append(value)

        return items, offset
    elif major == MAJOR_MAP:
        members = {}

        for _ in range(argument):
            key, offset = _decode(data, offset)

            if not isinstance(key, str):
                raise ConvertError("Unsupported CBOR map key {!r}.".format(key))

            value, offset = _decode(data, offset)
            members[key] = value

        return members, offset
    elif major == MAJOR_TAG and argument in [TAG_POSITIVE_BIGNUM, TAG_NEGATIVE_BIGNUM]:
        major, _, size, offset = _decode_head(data, offset)

        if major != MAJOR_BYTES:
            raise ConvertError("Expected a CBOR byte string as bignum.")

        encoded, offset = _decode_bytes(data, offset, size)
        value = int.from_bytes(encoded, "big")

        if argument == TAG_NEGATIVE_BIGNUM:
            value = -1 - value

        return value, offset
    elif major == MAJOR_SIMPLE and info == SIMPLE_FALSE:
        return False, offset
    elif major == MAJOR_SIMPLE and info == SIMPLE_TRUE:
        return True, offset
    elif major == MAJOR_SIMPLE and info == SIMPLE_NULL:
        return None, offset
    elif major == MAJOR_SIMPLE and info in [FLOAT_16, FLOAT_32, FLOAT_64]:
        size = 1 << (info - 24)
        fmt = {FLOAT_16: ">e", FLOAT_32: ">f", FLOAT_64: ">d"}[info]

        return struct.unpack(fmt, data[offset - size : offset])[0], offset
    else:
        raise ConvertError(
            "Unsupported CBOR major type {} with additional information {}.".format(
                major, info
            )
        )


def loads_first(data: bytes, offset: int = 0) -> Tuple[Any, int]:
    """Decode the CBOR found at given offset to primitives. Returns
    them and the offset of what follows.

    """

    try:
        return _decode(data, offset)
    except (IndexError, struct.error):
        raise ConvertError("Out of CBOR data.")
    except UnicodeDecodeError:
        raise ConvertError("Bad CBOR text string.")


def loads(data: bytes) -> Any:
    """Decode given CBOR to primitives."""

    value, offset = loads_first(data)

    if offset != len(data):
        raise ConvertError(
            "Unexpected data after the CBOR data at offset {}.".format(offset)
        )

    return value
