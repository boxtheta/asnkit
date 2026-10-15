# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Unaligned Packed Encoding Rules (UPER) codec."""

import typing

from asnkitrt.codecs.codec import Codec

from . import (
    DecodeError,
    ErrorWithLocation,
    per,
    restricted_generalized_time_from_datetime,
    restricted_generalized_time_to_datetime,
    restricted_utc_time_from_datetime,
    restricted_utc_time_to_datetime,
)
from .per import (
    Any,
    AnyDefinedBy,
    Boolean,
    Enumerated,
    GeneralString,
    GraphicString,
    Null,
    ObjectDescriptor,
    ObjectIdentifier,
    OpenType,
    Real,
    Recursive,
    Sequence,
    Set,
    TeletexString,
    UTF8String,
    VideotexString,
)


class Encoder(per.Encoder):
    def align(self):
        pass


class Decoder(per.Decoder):
    def align(self):
        pass


class Unaligned:
    """The length of a value of a size of more than one value, in as few
    bits as there are values (X.691 11.5.6, 11.9.4.1).

    """

    minimum: typing.Any
    number_of_bits: typing.Any

    def encode_length(self, length, encoder):
        encoder.append_non_negative_binary_integer(
            length - self.minimum, self.number_of_bits
        )

    def decode_length(self, decoder):
        return self.minimum + decoder.read_non_negative_binary_integer(
            self.number_of_bits
        )


class KnownMultiplierStringType(Unaligned, per.KnownMultiplierStringType):
    ALIGNED = False


class ArrayType(Unaligned, per.ArrayType):
    pass


class Integer(per.Integer):
    def encode_constrained(self, data, encoder):
        encoder.append_non_negative_binary_integer(
            data - self.minimum, self.number_of_bits
        )

    def decode_constrained(self, decoder):
        value = decoder.read_non_negative_binary_integer(self.number_of_bits)

        return value + self.minimum


class BitString(Unaligned, per.BitString):
    pass


class OctetString(Unaligned, per.OctetString):
    pass


class SequenceOf(ArrayType):
    def __init__(self, name, element_type, minimum, maximum, has_extension_marker):
        super(SequenceOf, self).__init__(
            name, element_type, minimum, maximum, has_extension_marker, "SEQUENCE OF"
        )


class SetOf(ArrayType):
    def __init__(self, name, element_type, minimum, maximum, has_extension_marker):
        super(SetOf, self).__init__(
            name, element_type, minimum, maximum, has_extension_marker, "SET OF"
        )


class Choice(per.Choice):
    def encode_root_index(self, index, encoder):
        encoder.append_non_negative_binary_integer(index, self.root_number_of_bits)

    def decode_root_index(self, decoder):
        return decoder.read_non_negative_binary_integer(self.root_number_of_bits)


class NumericString(KnownMultiplierStringType):
    CHARACTERS = per.NumericString.CHARACTERS


class PrintableString(KnownMultiplierStringType):
    CHARACTERS = per.PrintableString.CHARACTERS


class IA5String(KnownMultiplierStringType):
    CHARACTERS = per.IA5String.CHARACTERS


class BMPString(KnownMultiplierStringType):
    CHARACTERS = per.BMPString.CHARACTERS


class VisibleString(KnownMultiplierStringType):
    CHARACTERS = per.VisibleString.CHARACTERS


class UniversalString(KnownMultiplierStringType):
    CHARACTERS = per.UniversalString.CHARACTERS


class UTCTime(VisibleString):
    def encode(self, data, encoder):
        encoded = restricted_utc_time_from_datetime(data)

        return super(UTCTime, self).encode(encoded, encoder)

    def decode(self, decoder):
        decoded = super(UTCTime, self).decode(decoder)

        return restricted_utc_time_to_datetime(decoded)


class GeneralizedTime(VisibleString):
    def encode(self, data, encoder):
        enceded = restricted_generalized_time_from_datetime(data)

        return super(GeneralizedTime, self).encode(enceded, encoder)

    def decode(self, decoder):
        decoded = super(GeneralizedTime, self).decode(decoder)

        return restricted_generalized_time_to_datetime(decoded)


class Date(per.Date):
    def __init__(self, name):
        super(Date, self).__init__(name)
        immediate = Integer("immediate")
        near_future = Integer("near_future")
        near_past = Integer("near_past")
        reminder = Integer("reminder")
        immediate.set_restricted_to_range(2005, 2020, False)
        near_future.set_restricted_to_range(2021, 2276, False)
        near_past.set_restricted_to_range(1749, 2004, False)
        # INTEGER (MIN..1748 | 2277..MAX), which has no bounds (X.691 32).
        year = Choice("year", [immediate, near_future, near_past, reminder], None)
        month = Integer("month")
        day = Integer("day")
        month.set_restricted_to_range(1, 12, False)
        day.set_restricted_to_range(1, 31, False)
        self._inner = Sequence("DATE-ENCODING", [year, month, day], None)


class TimeOfDay(per.TimeOfDay):
    def __init__(self, name):
        super(TimeOfDay, self).__init__(name)
        hours = Integer("hours")
        minutes = Integer("minutes")
        seconds = Integer("seconds")
        hours.set_restricted_to_range(0, 24, False)
        minutes.set_restricted_to_range(0, 59, False)
        seconds.set_restricted_to_range(0, 60, False)
        self._inner = Sequence("TIME-OF-DAY-ENCODING", [hours, minutes, seconds], None)


class Duration(per.Duration):
    def __init__(self, name):
        super(Duration, self).__init__(name)
        self._inner = self._make_inner(Integer)


class DateTime(per.DateTime):
    def __init__(self, name):
        super(DateTime, self).__init__(name)
        self._inner = Sequence(
            "DATE-TIME-ENCODING", [Date("date"), TimeOfDay("time")], None
        )


class CompiledType(per.CompiledType):
    def encode(self, data):
        encoder = Encoder()
        try:
            self._type.encode(data, encoder)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e

        return encoder.as_bytearray()

    def decode(self, data):
        decoder = Decoder(bytearray(data))
        try:
            return self._type.decode(decoder)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e


class Compiler(per.Compiler):
    def process_type(self, type_name, type_descriptor, module_name):
        compiled_type = self.compile_type(type_name, type_descriptor, module_name)

        return CompiledType(compiled_type)

    def compile_type(self, name, type_descriptor, module_name):
        module_name = self.get_module_name(type_descriptor, module_name)
        type_name = type_descriptor["type"]

        compiled: per.Type
        if type_name == "SEQUENCE":
            compiled = Sequence(
                name, *self.compile_members(type_descriptor["members"], module_name)
            )
        elif type_name == "SEQUENCE OF":
            compiled = SequenceOf(
                name,
                self.compile_type("", type_descriptor["element"], module_name),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "SET":
            compiled = Set(
                name,
                *self.compile_members(
                    type_descriptor["members"], module_name, sort_by_tag=True
                ),
            )
        elif type_name == "SET OF":
            compiled = SetOf(
                name,
                self.compile_type("", type_descriptor["element"], module_name),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "CHOICE":
            # The alternatives are numbered in the canonical order of their
            # tags (X.691 23.2).
            root, additions = self.compile_members(
                type_descriptor["members"],
                module_name,
                sort_by_tag=True,
                flat_additions=True,
            )

            if additions is not None:
                additions = self.sort_additions_by_tag(
                    additions, type_descriptor["members"], module_name
                )

            compiled = Choice(name, root, additions)
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
            compiled = OctetString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "TeletexString":
            compiled = TeletexString(name)
        elif type_name == "VideotexString":
            compiled = VideotexString(name)
        elif type_name == "NumericString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = NumericString(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
        elif type_name == "PrintableString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = PrintableString(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
        elif type_name == "IA5String":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = IA5String(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
        elif type_name == "BMPString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = BMPString(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
        elif type_name == "VisibleString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = VisibleString(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
        elif type_name == "GeneralString":
            compiled = GeneralString(name)
        elif type_name == "UTF8String":
            compiled = UTF8String(name)
        elif type_name == "GraphicString":
            compiled = GraphicString(name)
        elif type_name == "UTCTime":
            compiled = UTCTime(name)
        elif type_name == "UniversalString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = UniversalString(
                name,
                *self.get_size_range(type_descriptor, module_name),
                permitted_alphabet=permitted_alphabet,
            )
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
            compiled = BitString(
                name,
                self.get_named_bits(type_descriptor, module_name),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "ANY":
            compiled = Any(name)
        elif type_name == "ANY DEFINED BY":
            choices = {}

            for key, value in type_descriptor["choices"].items():
                choices[key] = self.compile_type(key, value, module_name)

            compiled = AnyDefinedBy(name, type_descriptor["value"], choices)
        elif type_name == "NULL":
            compiled = Null(name)
        elif type_name == "OpenType":
            compiled = OpenType(name)
        elif type_name == "EXTERNAL":
            compiled = Sequence(
                name,
                *self.compile_members(
                    self.external_type_descriptor()["members"], module_name
                ),
            )
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

        if "tag" in type_descriptor:
            compiled = self.set_compiled_tag(compiled, type_descriptor)

        if "restricted-to" in type_descriptor:
            compiled = self.set_compiled_restricted_to(
                compiled, type_descriptor, module_name
            )

        return compiled


class UPERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums=False, lenient=False):
        return Compiler(specification, numeric_enums).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)


def decode_full_length(_data):
    raise DecodeError("Decode length is not supported for this codec.")
