# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""XML Encoding Rules (XER) codec."""

import binascii
import datetime
import decimal
import math
import re
import time
import typing
from xml.etree import ElementTree

from .duration import canonical_duration, validate_duration
from asnkitrt.codecs.codec import Codec

from ..constants import EXTENSION_MARKER
from . import (
    check_local_time,
    NamedNumbersMixin,
    BaseType,
    DecodeError,
    EncodeError,
    ErrorWithLocation,
    compiler,
    format_bytes,
    format_or,
    generalized_time_from_datetime,
    generalized_time_to_datetime,
    utc_time_from_datetime,
    utc_time_to_datetime,
)
from .compiler import enum_values_as_dict


# The names of the control characters in an "xmlcstring", by their code
# (X.680 12.15.5, Table 3). Tab, line feed and carriage return are
# written as they are.
CONTROLS = [
    "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "", "", "vt",
    "ff", "", "so", "si", "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb",
    "can", "em", "sub", "esc", "is4", "is3", "is2", "is1",
]  # fmt: skip
CONTROL_CODES = {name: code for code, name in enumerate(CONTROLS) if name}

# A control character is first an element with this before its name,
# which no identifier of ASN.1 has, so that it is told from an element of
# a member, and then written as `<bel/>`, with no space (X.680 12.15.5).
CONTROL_PREFIX = "ctl--"
CONTROL_TAG = re.compile(rb"<ctl--([a-z0-9]+) />")


def add_text(element, text):
    """Set the text of `element` to `text`, with its control characters as
    empty elements.

    """

    last = None

    for part in re.split(r"([\x00-\x08\x0b\x0c\x0e-\x1f])", text):
        if len(part) == 1 and ord(part) < 32:
            last = ElementTree.SubElement(element, CONTROL_PREFIX + CONTROLS[ord(part)])
        elif part:
            if last is None:
                element.text = part
            else:
                last.tail = part


def text_of(element):
    """The text of `element`, in which a control character is an empty
    element, as `<bel/>`.

    """

    parts = [element.text or ""]

    for child in element:
        if child.tag not in CONTROL_CODES or len(child) or child.text:
            raise DecodeError("Expected text in the XML element.")

        parts.append(chr(CONTROL_CODES[child.tag]))
        parts.append(child.tail or "")

    return "".join(parts)


XML_SPACE = re.compile("[ \t\n\r]")


def without_space(text):
    return XML_SPACE.sub("", text or "")


def decode_boolean(element):
    """A BOOLEAN, <true/> or <false/> (X.680 18.3): an empty element and
    nothing else, but space.

    """

    if element.tag not in ("true", "false") or element.text or len(element):
        raise DecodeError("Expected <true/> or <false/>.")

    return element.tag == "true"


def indent_xml(element, indent, level=0):
    i = "\n" + level * indent

    # The control characters of a string are in its text.
    if len(element) and not element[0].tag.startswith(CONTROL_PREFIX):
        if not element.text or not element.text.strip():
            element.text = i + indent

        if not element.tail or not element.tail.strip():
            element.tail = i

        for child in element:
            indent_xml(child, indent, level + 1)

        # The last child ends where its parent does.
        if not child.tail or not child.tail.strip():
            child.tail = i
    else:
        if level and (not element.tail or not element.tail.strip()):
            element.tail = i


class Type(BaseType):
    def __init__(self, name, type_name):
        super().__init__(name.replace(" ", "_"), type_name)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        pass

    def encode_of(self, data):
        # Used by ArrayType
        return self.encode(data)

    def decode_of(self, element):
        # Used by ArrayType
        return self.decode(element)


class StringType(Type):
    def __init__(self, name, type_name=None):
        if type_name is None:
            type_name = self.__class__.__name__

        super(StringType, self).__init__(name, type_name)

    def encode(self, data):
        element = ElementTree.Element(self.name)

        if len(data) > 0:
            add_text(element, data)

        return element

    def decode(self, element):
        return text_of(element)

    def __repr__(self):
        return "{}({})".format(self.__class__.__name__, self.name)


class MembersType(Type):
    def __init__(self, name, members, type_name):
        super(MembersType, self).__init__(name, type_name)
        self.members = members

    def encode(self, data):
        element = ElementTree.Element(self.name)

        for member in self.members:
            name = member.name

            if name in data:
                try:
                    if isinstance(member, AnyDefinedBy):
                        member_element = member.encode(data[name], data)
                    else:
                        member_element = member.encode(data[name])
                except ErrorWithLocation as e:
                    # Add member location
                    e.add_location(member)
                    raise e

            elif member.optional or member.has_default():
                continue
            else:
                raise EncodeError(
                    "{} member '{}' not found in {}.".format(
                        self.__class__.__name__, name, data
                    )
                )

            element.append(member_element)

        return element

    def decode(self, element):
        values: dict[str, typing.Any] = {}

        for member in self.members:
            name = member.name
            member_element = element.find(name)

            if member_element is not None:
                try:
                    if isinstance(member, AnyDefinedBy):
                        value = member.decode(member_element, values)
                    else:
                        value = member.decode(member_element)
                except ErrorWithLocation as e:
                    # Add member location
                    e.add_location(member)
                    raise e
                values[name] = value
            elif member.optional:
                pass
            elif member.has_default():
                values[name] = member.get_default()

        return values

    def __repr__(self):
        return "{}({}, [{}])".format(
            self.__class__.__name__,
            self.name,
            ", ".join([repr(member) for member in self.members]),
        )


class ArrayType(Type):
    def __init__(self, name, element_type, type_name):
        super(ArrayType, self).__init__(name, type_name)
        self.element_type = element_type

    def encode(self, data):
        element = ElementTree.Element(self.name)

        for entry in data:
            element.append(self.element_type.encode_of(entry))

        return element

    def decode(self, element):
        values = []

        for member_element in list(element):
            value = self.element_type.decode_of(member_element)
            values.append(value)

        return values

    def __repr__(self):
        return "{}({}, {})".format(
            self.__class__.__name__, self.name, self.element_type
        )


class Boolean(Type):
    def __init__(self, name):
        super(Boolean, self).__init__(name, "BOOLEAN")

    def encode(self, data):
        element = ElementTree.Element(self.name)
        ElementTree.SubElement(element, "true" if data else "false")

        return element

    def decode(self, element):
        if (
            len(element) != 1
            or without_space(element.text)
            or without_space(element[0].tail)
        ):
            raise DecodeError("Expected <true/> or <false/>.")

        return decode_boolean(element[0])

    def encode_of(self, data):
        return ElementTree.Element("true" if data else "false")

    def decode_of(self, element):
        return decode_boolean(element)


class Integer(NamedNumbersMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name, "INTEGER")

    def encode(self, data):
        data = self.to_number(data)

        element = ElementTree.Element(self.name)
        element.text = str(data)

        return element

    def decode(self, element):
        return int(element.text)


# A realnumber (X.680 12.9), after a "-" or not (X.680 21.9).
REAL_NUMBER = re.compile(r"-?[0-9]+(\.[0-9]*)?([eE][-+]?[0-9]+)?")


def xml_real(value):
    """A finite float as an XER realnumber with an exponent: as repr()
    has it and E0 if it is 0 or at least 0.0001 and less than 10, else
    its fewest digits with one before the point. The digits are those of
    the float, so it reads back as the same float.

    """

    text = repr(value)

    if value == 0 or (abs(value) < 10 and "e" not in text):
        return text + "E0"

    number = decimal.Decimal(repr(abs(value)))
    digits = "".join(str(digit) for digit in number.as_tuple().digits).rstrip("0")

    return "{}{}.{}E{}".format(
        "-" if value < 0 else "",
        digits[0],
        digits[1:] or "0",
        number.adjusted(),
    )


class Real(Type):
    def __init__(self, name):
        super(Real, self).__init__(name, "REAL")

    def encode(self, data):
        data = float(data)
        element = ElementTree.Element(self.name)

        # The special values are empty elements (X.680 21.9).
        if math.isnan(data):
            ElementTree.SubElement(element, "NOT-A-NUMBER")
        elif math.isinf(data):
            ElementTree.SubElement(
                element, "PLUS-INFINITY" if data > 0 else "MINUS-INFINITY"
            )
        else:
            element.text = xml_real(data)

        return element

    def decode(self, element):
        text = (element.text or "").strip()
        children = list(element)

        if not children:
            if REAL_NUMBER.fullmatch(text):
                return float(text)
        elif len(children) == 1 and not text:
            special = children[0]

            if special.text is None and len(special) == 0:
                if special.tag == "PLUS-INFINITY":
                    return math.inf
                elif special.tag == "MINUS-INFINITY":
                    return -math.inf
                elif special.tag == "NOT-A-NUMBER":
                    return math.nan

        raise DecodeError("Expected a real.")


class Null(Type):
    def __init__(self, name):
        super(Null, self).__init__(name, "NULL")

    def encode(self, data):
        return ElementTree.Element(self.name)

    def decode(self, element):
        return None


class BitString(Type):
    def __init__(self, name):
        super(BitString, self).__init__(name, "BIT STRING")

    def encode(self, data):
        element = ElementTree.Element(self.name)

        if data[1] > 0:
            encoded = int(binascii.hexlify(data[0]), 16)
            encoded |= 0x80 << (8 * len(data[0]))
            element.text = bin(encoded)[10 : 10 + data[1]].upper()

        return element

    def decode(self, element):
        # An xmlbstring may have space in it (X.680 12.10).
        encoded = without_space(element.text)

        if encoded.strip("01"):
            raise DecodeError("Expected 0 and 1 in a BIT STRING.")

        number_of_bits = len(encoded)

        if number_of_bits == 0:
            decoded = b""
        else:
            value = int(encoded, 2)
            value |= 0x80 << number_of_bits
            rest = number_of_bits % 8

            if rest != 0:
                value <<= 8 - rest

            decoded = binascii.unhexlify(hex(value).rstrip("L")[4:])

        return (decoded, number_of_bits)


class OctetString(Type):
    def __init__(self, name):
        super(OctetString, self).__init__(name, "OCTET STRING")

    def encode(self, data):
        element = ElementTree.Element(self.name)

        if len(data) > 0:
            element.text = format_bytes(data).upper()

        return element

    def decode(self, element):
        # An xmlhstring may have space in it (X.680 12.12), and an odd
        # number of digits is ended by a zero (X.680 22.3).
        encoded = without_space(element.text)

        if len(encoded) % 2:
            encoded += "0"

        try:
            return binascii.unhexlify(encoded)
        except (binascii.Error, ValueError):
            raise DecodeError("Expected hex.")


class ObjectIdentifier(StringType):
    def __init__(self, name):
        super(ObjectIdentifier, self).__init__(name, "OBJECT IDENTIFIER")

    def decode(self, element):
        if element.text is None:
            raise DecodeError("Expected an OBJECT IDENTIFIER, but got ''.")

        return element.text


class Enumerated(Type):
    def __init__(self, name, values, numeric):
        super(Enumerated, self).__init__(name, "ENUMERATED")

        if numeric:
            self.data_to_value = enum_values_as_dict(values)
            self.value_to_data = {v: k for k, v in self.data_to_value.items()}
        else:
            self.value_to_data = {k: k for k in enum_values_as_dict(values).values()}
            self.data_to_value = self.value_to_data

        self.has_extension_marker = EXTENSION_MARKER in values

    def format_names(self):
        return format_or(sorted(list(self.value_to_data.values())))

    def format_values(self):
        return format_or(sorted(list(self.value_to_data)))

    def encode(self, data):
        try:
            value = self.data_to_value[data]
        except KeyError:
            raise EncodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_names(), data
                )
            )

        element = ElementTree.Element(self.name)
        element.append(ElementTree.Element(value))

        return element

    def decode(self, element):
        value = element[0].tag

        if value in self.value_to_data:
            return self.value_to_data[value]
        elif self.has_extension_marker:
            return None
        else:
            raise DecodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_values(), value
                )
            )

    def encode_of(self, data):
        try:
            value = self.data_to_value[data]
        except KeyError:
            raise EncodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_names(), data
                )
            )

        return ElementTree.Element(value)

    def decode_of(self, element):
        value = element.tag

        try:
            return self.value_to_data[value]
        except KeyError:
            raise DecodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_values(), value
                )
            )


class Sequence(MembersType):
    def __init__(self, name, members):
        super(Sequence, self).__init__(name, members, "SEQUENCE")


class SequenceOf(ArrayType):
    def __init__(self, name, element_type):
        super(SequenceOf, self).__init__(name, element_type, "SEQUENCE OF")


class Set(MembersType):
    def __init__(self, name, members):
        super(Set, self).__init__(name, members, "SET")


class SetOf(ArrayType):
    def __init__(self, name, element_type):
        super(SetOf, self).__init__(name, element_type, "SET OF")


class Choice(Type):
    def __init__(self, name, members, has_extension_marker):
        super(Choice, self).__init__(name, "CHOICE")
        self.members = members
        self.name_to_member = {member.name: member for member in self.members}
        self.has_extension_marker = has_extension_marker

    def format_names(self):
        return format_or(sorted([member.name for member in self.members]))

    def encode(self, data):
        try:
            member = self.name_to_member[data[0]]
        except KeyError:
            raise EncodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), data[0])
            )

        element = ElementTree.Element(self.name)

        try:
            element.append(member.encode(data[1]))
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

        return element

    def decode(self, element):
        member_element = element[0]
        name = member_element.tag

        if name in self.name_to_member:
            member = self.name_to_member[name]
        elif self.has_extension_marker:
            return (None, None)
        else:
            raise DecodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), name)
            )
        try:
            return (name, member.decode(member_element))
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def encode_of(self, data):
        try:
            member = self.name_to_member[data[0]]
        except KeyError:
            e = EncodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), data[0])
            )
            # e.add_location(self.name)
            raise e
        try:
            return member.encode(data[1])
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def decode_of(self, element):
        name = element.tag

        try:
            member = self.name_to_member[name]
        except KeyError:
            raise DecodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), name)
            )
        try:
            return (name, member.decode(element))
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def __repr__(self):
        return "Choice({}, [{}])".format(
            self.name, ", ".join([repr(member) for member in self.members])
        )


class UTF8String(StringType):
    pass


class NumericString(StringType):
    pass


class PrintableString(StringType):
    pass


class IA5String(StringType):
    pass


class VisibleString(StringType):
    pass


class GeneralString(StringType):
    pass


class BMPString(StringType):
    pass


class GraphicString(StringType):
    pass


class UniversalString(StringType):
    pass


class TeletexString(StringType):
    pass


class VideotexString(StringType):
    pass


class ObjectDescriptor(GraphicString):
    pass


class UTCTime(Type):
    def __init__(self, name):
        super(UTCTime, self).__init__(name, "UTCTime")

    def encode(self, data):
        element = ElementTree.Element(self.name)
        element.text = utc_time_from_datetime(data)

        return element

    def decode(self, element):
        return utc_time_to_datetime(element.text)


class GeneralizedTime(Type):
    def __init__(self, name):
        super(GeneralizedTime, self).__init__(name, "GeneralizedTime")

    def encode(self, data):
        element = ElementTree.Element(self.name)
        element.text = generalized_time_from_datetime(data)

        return element

    def decode(self, element):
        return generalized_time_to_datetime(element.text)


class Date(StringType):
    def encode(self, data):
        element = ElementTree.Element(self.name)
        element.text = str(data)

        return element

    def decode(self, element):
        return datetime.date(*time.strptime(element.text, "%Y-%m-%d")[:3])


class TimeOfDay(StringType):
    def encode(self, data):
        check_local_time(data)

        element = ElementTree.Element(self.name)
        element.text = str(data)

        return element

    def decode(self, element):
        return datetime.time(*time.strptime(element.text, "%H:%M:%S")[3:6])


class Duration(StringType):
    def encode(self, data):
        validate_duration(data)
        element = ElementTree.Element(self.name)
        element.text = canonical_duration(data)
        return element

    def decode(self, element):
        text = element.text or ""
        validate_duration(text)
        return text


class DateTime(StringType):
    def encode(self, data):
        check_local_time(data)

        element = ElementTree.Element(self.name)
        element.text = str(data).replace(" ", "T")

        return element

    def decode(self, element):
        return datetime.datetime(*time.strptime(element.text, "%Y-%m-%dT%H:%M:%S")[:6])


class Any(Type):
    """An ANY or an open type, which is encoded as the value of the type it
    is (X.693 8.5), and so can not be without it.

    """

    def __init__(self, name):
        super(Any, self).__init__(name, "ANY")

    def encode(self, data):
        raise NotImplementedError(
            "XER needs the type of the value of an ANY or an open type."
        )

    def decode(self, element):
        raise NotImplementedError(
            "XER needs the type of the value of an ANY or an open type."
        )


class AnyDefinedBy(Any):
    """An ANY DEFINED BY. With choices, it is the value of the type of the
    choice of the value of the member it names, which the SEQUENCE or SET
    it is a member of gives as `values`. Without, it is an ANY.

    """

    def __init__(self, name, type_member, choices):
        super(AnyDefinedBy, self).__init__(name)
        self.type_name = "ANY DEFINED BY"
        self.type_member = type_member
        self.choices = choices

    def choice(self, values):
        try:
            return self.choices[values[self.type_member]]
        except (KeyError, TypeError):
            return None

    def bad_choice(self, values):
        return "Bad AnyDefinedBy choice {}.".format(
            (values or {}).get(self.type_member)
        )

    def encode(self, data, values=None):
        if not self.choices:
            return super(AnyDefinedBy, self).encode(data)

        choice = self.choice(values)

        if choice is None:
            raise EncodeError(self.bad_choice(values))

        return choice.encode(data)

    def decode(self, element, values=None):
        if not self.choices:
            return super(AnyDefinedBy, self).decode(element)

        choice = self.choice(values)

        if choice is None:
            raise DecodeError(self.bad_choice(values))

        return choice.decode(element)


class Recursive(compiler.Recursive, Type):
    def __init__(self, name, type_name, module_name):
        super(Recursive, self).__init__(name, "RECURSIVE")
        self.type_name = type_name
        self.module_name = module_name
        self._inner: typing.Any = None

    def set_inner_type(self, inner):
        self._inner = inner

    def encode(self, data):
        encoded = self._inner.encode(data)
        encoded.tag = self.name

        return encoded

    def decode(self, element):
        return self._inner.decode(element)


class CompiledType(compiler.CompiledType):
    def encode(self, data, indent=None):
        try:
            element = self._type.encode(data)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e

        if indent is not None:
            indent_xml(element, indent * " ")

        # A carriage return written as it is would be read as a line feed.
        encoded = ElementTree.tostring(element).replace(b"\r", b"&#13;")

        return CONTROL_TAG.sub(rb"<\1/>", encoded)

    def decode(self, data):
        element = ElementTree.fromstring(data.decode("utf-8"))
        try:
            return self._type.decode(element)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e


class Compiler(compiler.Compiler):
    def process_type(self, type_name, type_descriptor, module_name):
        compiled_type = self.compile_type(type_name, type_descriptor, module_name)

        return CompiledType(compiled_type)

    def compile_type(self, name, type_descriptor, module_name):
        module_name = self.get_module_name(type_descriptor, module_name)
        type_name = type_descriptor["type"]

        compiled: Type
        if type_name == "SEQUENCE":
            members, _ = self.compile_members(type_descriptor["members"], module_name)
            compiled = Sequence(name, members)
        elif type_name == "SEQUENCE OF":
            element = type_descriptor["element"]
            compiled = SequenceOf(
                name,
                self.compile_type(
                    type_descriptor.get("element_name") or element["type"],
                    element,
                    module_name,
                ),
            )
        elif type_name == "SET":
            members, _ = self.compile_members(type_descriptor["members"], module_name)
            compiled = Set(name, members)
        elif type_name == "SET OF":
            element = type_descriptor["element"]
            compiled = SetOf(
                name,
                self.compile_type(
                    type_descriptor.get("element_name") or element["type"],
                    element,
                    module_name,
                ),
            )
        elif type_name == "CHOICE":
            compiled = Choice(
                name, *self.compile_members(type_descriptor["members"], module_name)
            )
        elif type_name == "INTEGER":
            compiled = Integer(name)
            compiled.named_numbers = self.get_named_numbers(
                type_descriptor, module_name
            )
        elif type_name == "REAL":
            compiled = Real(name)
        elif type_name == "ENUMERATED":
            compiled = Enumerated(
                name,
                self.get_enum_values(type_descriptor, module_name),
                self._numeric_enums,
            )
        elif type_name == "BOOLEAN":
            compiled = Boolean(name)
        elif type_name == "OBJECT IDENTIFIER":
            compiled = ObjectIdentifier(name)
        elif type_name == "OCTET STRING":
            compiled = OctetString(name)
        elif type_name == "TeletexString":
            compiled = TeletexString(name)
        elif type_name == "VideotexString":
            compiled = VideotexString(name)
        elif type_name == "NumericString":
            compiled = NumericString(name)
        elif type_name == "PrintableString":
            compiled = PrintableString(name)
        elif type_name == "IA5String":
            compiled = IA5String(name)
        elif type_name == "VisibleString":
            compiled = VisibleString(name)
        elif type_name == "GeneralString":
            compiled = GeneralString(name)
        elif type_name == "UTF8String":
            compiled = UTF8String(name)
        elif type_name == "BMPString":
            compiled = BMPString(name)
        elif type_name == "GraphicString":
            compiled = GraphicString(name)
        elif type_name == "UTCTime":
            compiled = UTCTime(name)
        elif type_name == "UniversalString":
            compiled = UniversalString(name)
        elif type_name == "GeneralizedTime":
            compiled = GeneralizedTime(name)
        elif type_name == "DATE":
            compiled = Date(name)
        elif type_name == "TIME-OF-DAY":
            compiled = TimeOfDay(name)
        elif type_name == "DATE-TIME":
            compiled = DateTime(name)
        elif type_name == "DURATION":
            compiled = Duration(name)
        elif type_name == "BIT STRING":
            compiled = BitString(name)
        elif type_name in ["ANY", "OpenType"]:
            # An open type needs the type of its value (X.697 41, X.693 8.5).
            compiled = Any(name)
        elif type_name == "ANY DEFINED BY":
            choices = {}

            # Named after the member, as the element of the value is.
            for key, value in type_descriptor["choices"].items():
                choices[key] = self.compile_type(name, value, module_name)

            compiled = AnyDefinedBy(name, type_descriptor["value"], choices)
        elif type_name == "NULL":
            compiled = Null(name)
        elif type_name == "EXTERNAL":
            members, _ = self.compile_members(
                self.external_type_descriptor()["members"], module_name
            )
            compiled = Sequence(name, members)
        elif type_name == "ObjectDescriptor":
            compiled = ObjectDescriptor(name)
        else:
            if type_name in self.types_backtrace:
                compiled = Recursive(name, type_name, module_name)
                self.recursive_types.append(compiled)
            else:
                compiled = self.compile_user_type(name, type_name, module_name)

        return compiled


class XERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums: bool = False, lenient=False):
        return Compiler(specification, numeric_enums).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)


def decode_full_length(_data):
    raise DecodeError("Decode length is not supported for this codec.")
