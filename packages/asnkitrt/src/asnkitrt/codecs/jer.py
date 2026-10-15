# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""JSON Encoding Rules (JER) codec."""

import binascii
import datetime
import json
import math
import time
import typing

from .duration import validate_duration
from asnkitrt.codecs.codec import Codec

from ..constants import EXTENSION_MARKER
from . import (
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


class Type(BaseType):
    def set_size_range(self, minimum, maximum, has_extension_marker):
        pass


class StringType(Type):
    def __init__(self, name):
        super(StringType, self).__init__(name, self.__class__.__name__)

    def encode(self, data):
        return data

    def decode(self, data):
        return data


class MembersType(Type):
    def __init__(self, name, members, type_name):
        super(MembersType, self).__init__(name, type_name)
        self.members = members

    def encode(self, data):
        values = {}

        for member in self.members:
            name = member.name

            if name in data:
                try:
                    if isinstance(member, AnyDefinedBy):
                        value = member.encode(data[name], data)
                    else:
                        value = member.encode(data[name])
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

            values[name] = value

        return values

    def decode(self, data):
        values: dict[str, typing.Any] = {}

        for member in self.members:
            name = member.name

            if name in data:
                try:
                    if isinstance(member, AnyDefinedBy):
                        value = member.decode(data[name], values)
                    else:
                        value = member.decode(data[name])
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


class Boolean(Type):
    def __init__(self, name):
        super(Boolean, self).__init__(name, "BOOLEAN")

    def encode(self, data):
        return data

    def decode(self, data):
        return data


class Integer(NamedNumbersMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name, "INTEGER")

    def encode(self, data):
        data = self.to_number(data)

        return data

    def decode(self, data):
        return data


class Real(Type):
    def __init__(self, name):
        super(Real, self).__init__(name, "REAL")

    def encode(self, data):
        if data == float("inf"):
            return "INF"
        elif data == float("-inf"):
            return "-INF"
        elif math.isnan(data):
            return "NaN"
        else:
            return data

    def decode(self, data):
        if isinstance(data, float):
            return data
        else:
            return {
                "INF": float("inf"),
                "-INF": float("-inf"),
                "NaN": float("nan"),
                "0": 0.0,
                "-0": 0.0,
            }[data]


class Null(Type):
    def __init__(self, name):
        super(Null, self).__init__(name, "NULL")

    def encode(self, data):
        return data

    def decode(self, data):
        return data


class BitString(Type):
    def __init__(self, name, minimum, maximum):
        super(BitString, self).__init__(name, "BIT STRING")
        self.set_size_range(minimum, maximum, None)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        if minimum is None and maximum is None:
            self.size = None
        elif minimum == maximum:
            self.size = minimum
        else:
            self.size = None

    def encode(self, data):
        value: typing.Any = format_bytes(data[0]).upper()

        if self.size is None:
            value = {"value": value, "length": data[1]}

        return value

    def decode(self, data):
        if self.size is None:
            return (binascii.unhexlify(data["value"]), data["length"])
        else:
            return (binascii.unhexlify(data), self.size)


class OctetString(Type):
    def __init__(self, name):
        super(OctetString, self).__init__(name, "OCTET STRING")

    def encode(self, data):
        return format_bytes(data).upper()

    def decode(self, data):
        return binascii.unhexlify(data)


class ObjectIdentifier(Type):
    def __init__(self, name):
        super(ObjectIdentifier, self).__init__(name, "OBJECT IDENTIFIER")

    def encode(self, data):
        return data

    def decode(self, data):
        return str(data)


class Enumerated(Type):
    def __init__(self, name, values, numeric):
        super(Enumerated, self).__init__(name, "ENUMERATED")

        if numeric:
            self.values = {k: k for k in enum_values_as_dict(values)}
        else:
            self.values = {v: v for v in enum_values_as_dict(values).values()}

        self.has_extension_marker = EXTENSION_MARKER in values

    def format_values(self):
        return format_or(sorted(list(self.values)))

    def encode(self, data):
        try:
            value = self.values[data]
        except KeyError:
            raise EncodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_values(), data
                )
            )

        return value

    def decode(self, data):
        if data in self.values:
            return self.values[data]
        elif self.has_extension_marker:
            return None
        else:
            raise DecodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_values(), data
                )
            )


class Sequence(MembersType):
    def __init__(self, name, members):
        super(Sequence, self).__init__(name, members, "SEQUENCE")


class SequenceOf(Type):
    def __init__(self, name, element_type):
        super(SequenceOf, self).__init__(name, "SEQUENCE OF")
        self.element_type = element_type

    def encode(self, data):
        values = []

        for entry in data:
            value = self.element_type.encode(entry)
            values.append(value)

        return values

    def decode(self, data):
        values = []

        for element_data in data:
            value = self.element_type.decode(element_data)
            values.append(value)

        return values

    def __repr__(self):
        return "SequenceOf({}, {})".format(self.name, self.element_type)


class Set(MembersType):
    def __init__(self, name, members):
        super(Set, self).__init__(name, members, "SET")


class SetOf(Type):
    def __init__(self, name, element_type):
        super(SetOf, self).__init__(name, "SET OF")
        self.element_type = element_type

    def encode(self, data):
        values = []

        for entry in data:
            value = self.element_type.encode(entry)
            values.append(value)

        return values

    def decode(self, data):
        values = []

        for element_data in data:
            value = self.element_type.decode(element_data)
            values.append(value)

        return values

    def __repr__(self):
        return "SetOf({}, {})".format(self.name, self.element_type)


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
        try:
            return {member.name: member.encode(data[1])}
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def decode(self, data):
        name, value = list(data.items())[0]

        if name in self.name_to_member:
            member = self.name_to_member[name]
        elif self.has_extension_marker:
            return (None, None)
        else:
            raise DecodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), name)
            )
        try:
            return (name, member.decode(value))
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


class UTCTime(StringType):
    def encode(self, data):
        return utc_time_from_datetime(data)

    def decode(self, data):
        return utc_time_to_datetime(data)


class GeneralizedTime(StringType):
    def encode(self, data):
        return generalized_time_from_datetime(data)

    def decode(self, data):
        return generalized_time_to_datetime(data)


class Date(StringType):
    def encode(self, data):
        return str(data)

    def decode(self, data):
        return datetime.date(*time.strptime(data, "%Y-%m-%d")[:3])


class TimeOfDay(StringType):
    def encode(self, data):
        return str(data)

    def decode(self, data):
        return datetime.time(*time.strptime(data, "%H:%M:%S")[3:6])


class Duration(StringType):
    def encode(self, data):
        validate_duration(data)
        return data

    def decode(self, data):
        validate_duration(data)
        return data


class DateTime(StringType):
    def encode(self, data):
        return str(data).replace(" ", "T")

    def decode(self, data):
        return datetime.datetime(*time.strptime(data, "%Y-%m-%dT%H:%M:%S")[:6])


class Any(Type):
    """An ANY or an open type, which is encoded as the value of the type it
    is (X.697 41), and so can not be without it.

    """

    def __init__(self, name):
        super(Any, self).__init__(name, "ANY")

    def encode(self, _data):
        raise NotImplementedError(
            "JER needs the type of the value of an ANY or an open type."
        )

    def decode(self, _data):
        raise NotImplementedError(
            "JER needs the type of the value of an ANY or an open type."
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

    def decode(self, data, values=None):
        if not self.choices:
            return super(AnyDefinedBy, self).decode(data)

        choice = self.choice(values)

        if choice is None:
            raise DecodeError(self.bad_choice(values))

        return choice.decode(data)


class Recursive(compiler.Recursive, Type):
    def __init__(self, name, type_name, module_name):
        super(Recursive, self).__init__(name, "RECURSIVE")
        self.type_name = type_name
        self.module_name = module_name
        self._inner: typing.Any = None

    def set_inner_type(self, inner):
        self._inner = inner

    def encode(self, data):
        return self._inner.encode(data)

    def decode(self, data):
        return self._inner.decode(data)


class CompiledType(compiler.CompiledType):
    def encode(self, data, indent=None):
        try:
            dictionary = self._type.encode(data)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e

        if indent is None:
            string = json.dumps(dictionary, separators=(",", ":"))
        else:
            string = json.dumps(dictionary, indent=indent)

        return string.encode("utf-8")

    def decode(self, data):
        try:
            return self._type.decode(json.loads(data.decode("utf-8")))
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
            compiled = SequenceOf(
                name, self.compile_type("", type_descriptor["element"], module_name)
            )
        elif type_name == "SET":
            members, _ = self.compile_members(type_descriptor["members"], module_name)
            compiled = Set(name, members)
        elif type_name == "SET OF":
            compiled = SetOf(
                name, self.compile_type("", type_descriptor["element"], module_name)
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
        elif type_name == "DATE":
            compiled = Date(name)
        elif type_name == "TIME-OF-DAY":
            compiled = TimeOfDay(name)
        elif type_name == "DATE-TIME":
            compiled = DateTime(name)
        elif type_name == "DURATION":
            compiled = Duration(name)
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
        elif type_name == "BIT STRING":
            minimum, maximum, _ = self.get_size_range(type_descriptor, module_name)
            compiled = BitString(name, minimum, maximum)
        elif type_name in ["ANY", "OpenType"]:
            # An open type needs the type of its value (X.697 41, X.693 8.5).
            compiled = Any(name)
        elif type_name == "ANY DEFINED BY":
            choices = {}

            for key, value in type_descriptor["choices"].items():
                choices[key] = self.compile_type(key, value, module_name)

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
                compiled = self.with_own_constraints(
                    compiled, type_descriptor, module_name
                )

        return compiled


class JERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums=False, lenient=False):
        return Compiler(specification, numeric_enums).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)


def decode_full_length(_data):
    raise DecodeError("Decode length is not supported for this codec.")
