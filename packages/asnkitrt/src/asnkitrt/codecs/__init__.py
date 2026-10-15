# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

import importlib
from datetime import datetime, timedelta, timezone
from typing import Any, List

from ..errors import ConstraintsError as _ConstraintsError
from ..errors import DecodeError as _DecodeError
from ..errors import EncodeError as _EncodeError
from ..errors import Error


class BaseType(object):
    """
    Base Type class containing common functionality between all codecs
    """

    def __init__(self, name: str, type_name: str) -> None:
        self.name = name
        self.type_name = type_name
        self.optional = False
        self.default: Any = None

    def set_default(self, value: Any) -> None:
        self.default = value

    def get_default(self) -> Any:
        return self.default

    def has_default(self) -> bool:
        return self.default is not None

    def is_default(self, value: Any) -> bool:
        return value == self.default

    def encode(self, *args: Any, **kwargs: Any) -> Any:
        raise NotImplementedError("To be implemented by subclasses.")

    def decode(self, *args: Any, **kwargs: Any) -> Any:
        raise NotImplementedError("To be implemented by subclasses.")

    def __repr__(self) -> str:
        return "{}({})".format(self.__class__.__name__, self.name)

    def type_label(self) -> str:
        return (
            "{}({})".format(self.type_name, self.name) if self.name else self.type_name
        )


class ErrorWithLocation(Exception):
    """
    Mixin for Error classes which have location list
    """

    def __init__(self, message: str, location: BaseType | None = None) -> None:
        """

        :param str message:
        :param BaseType location: Type error occured for
        """
        self.message = message
        self.location: List[BaseType] = [location] if location else []

    def add_location(self, element: BaseType) -> None:
        """
        Add location data for element
        :param BaseType element:
        :return:
        """
        # Don't add name if it is blank (for SEQUENCE OF, SET OF etc)
        if not getattr(element, "no_error_location", False) and (
            not self.location or element != self.location[-1]
        ):
            self.location.append(element)

    @property
    def location_str(self) -> str:
        return ".".join(loc.name for loc in self.location[::-1] if loc.name)

    def __str__(self) -> str:
        if self.location:
            return "{}: {}".format(self.location_str, self.message)
        else:
            return self.message


class EncodeError(ErrorWithLocation, _EncodeError):
    """General ASN.1 encode error."""

    pass


class NamedNumbersMixin(object):
    """An INTEGER, which can be given by the name of any of its named
    numbers when encoding. It is always a number when decoded.

    """

    named_numbers: Any = None

    def to_number(self, data: Any) -> Any:
        if not isinstance(data, str):
            return data

        try:
            return self.named_numbers[data]
        except (KeyError, TypeError):
            raise EncodeError(
                "Expected an integer or a named number, but got '{}'.".format(data)
            )

    def is_default(self, value: Any) -> bool:
        return self.to_number(value) == self.default  # type: ignore[attr-defined]


class DecodeError(ErrorWithLocation, _DecodeError):
    """
    General ASN.1 decode error with error location in the message.
    """

    def __init__(
        self,
        message: str,
        offset: int | None = None,
        location: BaseType | None = None,
    ) -> None:
        """

        :param str message: Message for error
        :param int offset: Data offset at which error occurred. Can be bits or bytes depending on codec
        :param BaseType location: Element in which error occurred
        """
        super(DecodeError, self).__init__(message, location=location)
        self.offset = offset

    def __str__(self) -> str:
        if self.location:
            _str = "{}: {}".format(self.location_str, self.message)
        else:
            _str = self.message
        if self.offset is not None:
            _str += self.get_offset_message()
        return _str

    def get_offset_message(self) -> str:
        """
        Get offset details to add to error message
        :return:
        """
        return " (At offset: {})".format(self.offset)


class ConstraintsError(ErrorWithLocation, _ConstraintsError):
    """
    General ASN.1 constraints error with error location in the message.
    """

    pass


class OutOfDataError(DecodeError):
    """
    Error for when trying to read data beyond length of type
    """

    def __init__(self, offset_bits: int, location: BaseType | None = None) -> None:
        super(OutOfDataError, self).__init__(
            "out of data", offset=offset_bits, location=location
        )

    def get_offset_message(self) -> str:
        """
        Get offset details to add to error message
        :return:
        """
        return " (At bit offset: {})".format(self.offset)


def _generalized_time_to_datetime(string: str) -> datetime:
    length = len(string)

    if "." in string:
        try:
            return datetime.strptime(string, "%Y%m%d%H%M.%f")
        except ValueError:
            return datetime.strptime(string, "%Y%m%d%H%M%S.%f")
    elif "," in string:
        try:
            return datetime.strptime(string, "%Y%m%d%H%M,%f")
        except ValueError:
            return datetime.strptime(string, "%Y%m%d%H%M%S,%f")
    elif length == 12:
        return datetime.strptime(string, "%Y%m%d%H%M")
    elif length == 14:
        return datetime.strptime(string, "%Y%m%d%H%M%S")
    else:
        raise ValueError


def format_or(items: List[Any]) -> str:
    """Return a string of comma separated items, with the last to items
    separated by "or".

    [1, 2, 3] -> "1, 2 or 3"

    """

    formatted_items = []

    for item in items:
        try:
            item = "'" + item + "'"
        except TypeError:
            item = str(item)

        formatted_items.append(item)

    if len(items) == 1:
        return formatted_items[0]
    else:
        return "{} or {}".format(", ".join(formatted_items[:-1]), formatted_items[-1])


def utc_time_to_datetime(string: str) -> datetime:
    """Convert given ASN.1 UTC time string `string` to a
    ``datetime.datetime`` object.

    """

    length = len(string)

    try:
        if string[-1] == "Z":
            if length == 11:
                return datetime.strptime(string[:-1], "%y%m%d%H%M")
            elif length == 13:
                return datetime.strptime(string[:-1], "%y%m%d%H%M%S")
            else:
                raise ValueError
        elif length == 15:
            return datetime.strptime(string, "%y%m%d%H%M%z")
        elif length == 17:
            return datetime.strptime(string, "%y%m%d%H%M%S%z")
        else:
            raise ValueError
    except (ValueError, IndexError):
        raise Error("Expected a UTC time string, but got '{}'.".format(string))


def utc_time_from_datetime(date: datetime) -> str:
    """Convert given ``datetime.datetime`` object `date` to an ASN.1 UTC
    time string.

    """

    fmt = "%y%m%d%H%M"

    if date.second > 0:
        fmt += "%S"

    if date.tzinfo is None:
        fmt += "Z"
    else:
        fmt += "%z"

    return date.strftime(fmt)


def restricted_utc_time_to_datetime(string: str) -> datetime:
    """Convert given restricted ASN.1 UTC time string `string` to a
    ``datetime.datetime`` object.

    """

    try:
        if string[-1] != "Z":
            raise ValueError

        if len(string) != 13:
            raise ValueError

        return datetime.strptime(string[:-1], "%y%m%d%H%M%S")
    except (ValueError, IndexError):
        raise Error(
            "Expected a restricted UTC time string, but got '{}'.".format(string)
        )


def restricted_utc_time_from_datetime(date: datetime) -> str:
    """Convert given ``datetime.datetime`` object `date` to an restricted
    ASN.1 UTC time string.

    """

    offset = date.utcoffset()

    if offset is not None:
        date -= offset

    return date.strftime("%y%m%d%H%M%S") + "Z"


def generalized_time_to_datetime(string: str) -> datetime:
    """Convert given ASN.1 generalized time string `string` to a
    ``datetime.datetime`` object.

    """

    try:
        if string[-1] == "Z":
            date = _generalized_time_to_datetime(string[:-1])

            return date.replace(tzinfo=timezone(timedelta(hours=0)))
        elif string[-5] in "-+":
            if "." in string:
                try:
                    return datetime.strptime(string, "%Y%m%d%H%M.%f%z")
                except ValueError:
                    return datetime.strptime(string, "%Y%m%d%H%M%S.%f%z")
            elif "," in string:
                try:
                    return datetime.strptime(string, "%Y%m%d%H%M,%f%z")
                except ValueError:
                    return datetime.strptime(string, "%Y%m%d%H%M%S,%f%z")
            else:
                return datetime.strptime(string, "%Y%m%d%H%M%S%z")
        else:
            return _generalized_time_to_datetime(string)
    except (ValueError, IndexError):
        raise Error("Expected a generalized time string, but got '{}'.".format(string))


def generalized_time_from_datetime(date: datetime) -> str:
    """Convert given ``datetime.datetime`` object `date` to an ASN.1
    generalized time string.

    """

    if date.second == 0:
        if date.microsecond > 0:
            string = date.strftime("%Y%m%d%H%M.%f").rstrip("0")
        else:
            string = date.strftime("%Y%m%d%H%M")
    else:
        if date.microsecond > 0:
            string = date.strftime("%Y%m%d%H%M%S.%f").rstrip("0")
        else:
            string = date.strftime("%Y%m%d%H%M%S")

    if date.tzinfo is not None:
        if date.utcoffset():
            string += date.strftime("%z")
        else:
            string += "Z"

    return string


def restricted_generalized_time_to_datetime(string: str) -> datetime:
    """Convert given restricted ASN.1 generalized time string `string` to
    a ``datetime.datetime`` object.

    """

    try:
        if string[-1] != "Z":
            raise ValueError

        if "." in string:
            if string[-2] == "0":
                raise ValueError

            if string[14] != ".":
                raise ValueError

            return datetime.strptime(string[:-1], "%Y%m%d%H%M%S.%f")
        elif len(string) != 15:
            raise ValueError

        return datetime.strptime(string[:-1], "%Y%m%d%H%M%S")
    except (ValueError, IndexError):
        raise Error(
            "Expected a restricted generalized time string, but got '{}'.".format(
                string
            )
        )


def restricted_generalized_time_from_datetime(date: datetime) -> str:
    """Convert given ``datetime.datetime`` object `date` to an restricted
    ASN.1 generalized time string.

    """

    offset = date.utcoffset()

    if offset is not None:
        date -= offset

    if date.microsecond > 0:
        string = date.strftime("%Y%m%d%H%M%S.%f").rstrip("0")
    else:
        string = date.strftime("%Y%m%d%H%M%S")

    return string + "Z"


def bmp_string_encode(text: str) -> bytes:
    """A BMPString in the 2-octet form, of which each character is in the
    BMP (X.690 8.23.8, X.696 27.4).

    """

    for character in text:
        if ord(character) > 0xFFFF:
            raise EncodeError(
                "Expected a character in the BMP, but got U+{:04X}.".format(
                    ord(character)
                )
            )

    return text.encode("utf-16-be")


def bmp_string_decode(data: bytes | bytearray) -> str:
    """The text of a BMPString in the 2-octet form. A surrogate is not a
    character.

    """

    if len(data) % 2 != 0:
        raise DecodeError("Expected a BMPString of an even number of octets.")

    units = [
        int.from_bytes(data[offset : offset + 2], "big")
        for offset in range(0, len(data), 2)
    ]

    for unit in units:
        if 0xD800 <= unit <= 0xDFFF:
            raise DecodeError(
                "Expected a character in the BMP, but got the surrogate "
                "0x{:04X}.".format(unit)
            )

    return "".join(map(chr, units))


def format_bytes(tag: bytes | bytearray) -> str:
    """
    Format tag as hex string
    :param bytes tag:
    :return:
    """
    return tag.hex()


# A codec is imported when first used, by name or as an attribute of
# this package.
_SUBMODULES = [
    "ber",
    "codec",
    "compiler",
    "constraints_checker",
    "der",
    "gser",
    "jer",
    "oer",
    "per",
    "permitted_alphabet",
    "type_checker",
    "uper",
    "xer",
]


def __getattr__(name: str) -> Any:
    if name in _SUBMODULES:
        return importlib.import_module("." + name, __name__)

    raise AttributeError("module '{}' has no attribute '{}'".format(__name__, name))
