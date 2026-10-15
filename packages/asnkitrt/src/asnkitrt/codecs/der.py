# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Distinguished Encoding Rules (DER) codec."""

from .duration import canonical_duration, validate_duration
from asnkitrt.codecs.codec import Codec

from . import (
    Error,
    NamedNumbersMixin,
    ber,
    generalized_time_to_datetime,
    restricted_generalized_time_from_datetime,
    restricted_generalized_time_to_datetime,
    restricted_utc_time_from_datetime,
    restricted_utc_time_to_datetime,
    utc_time_to_datetime,
)

# These imports are not used in this module but referenced externally
from .ber import (
    Any,
    EncodeError,
    AnyDefinedBy,
    DecodeError,
    Choice,
    Class,
    Encoding,
    ObjectDescriptor,
    Recursive,
    StandardEncodeMixin,
    Tag,
    decode_full_length,
    encode_length_definite,
    encode_signed_integer,
)
from .compiler import clean_bit_string_value


# DER only has definite lengths in as few bytes as possible, unlike
# BER, where these are found with any length. The other rules of X.690
# clause 10 and 11 are checked as given in the classes below.
#
# A reader that is told to be lenient gets what BER allows instead. That
# is a decoder only: what is encoded is DER whatever it is told.
class Lenient:
    lenient = False
    indefinite_allowed = False
    minimal_length_required = True

    def decode_error(self, message, offset):
        return DecodeError(message, offset=offset)


def make_lenient(compiled):
    """Make given compiled type, and no other, accept what BER does."""

    compiled.lenient = True

    for name in (
        "indefinite_allowed",
        "minimal_length_required",
        "members_in_any_order",
    ):
        for class_ in type(compiled).__mro__:
            if class_.__module__ != __name__ and name in vars(class_):
                setattr(compiled, name, vars(class_)[name])

                break


class MembersMixin(Lenient):
    name: str
    members_in_any_order = False

    def check_member(self, member, value, offset):
        # A member with its default is left out in DER.
        if not self.lenient and member.has_default() and member.is_default(value):
            raise self.decode_error(
                "Member '{}' of {} is encoded with its default value, which "
                "DER leaves out.".format(member.name, self.name),
                offset,
            )


class Sequence(MembersMixin, ber.Sequence):
    pass


class Set(MembersMixin, ber.Set):
    ordered_by_tags = True

    def encode_content(self, data, values=None):
        # The members are in the order of the tags they are encoded with,
        # which for an untagged CHOICE is that of the alternative there
        # (X.690 10.3).
        parts = []

        for member in self.root_members:
            encoded = bytearray()
            self.encode_member(member, data, encoded)
            parts.append(encoded)

        if self.additions:
            try:
                for addition in self.additions:
                    group = []

                    for member in (
                        addition if isinstance(addition, list) else [addition]
                    ):
                        encoded = bytearray()
                        self.encode_member(member, data, encoded)
                        group.append(encoded)

                    parts.extend(group)
            except ber.EncodeError:
                pass

        parts = sorted(
            (part for part in parts if part),
            key=lambda part: ber.canonical_tag_key(part, 0),
        )

        return bytearray().join(parts)


class ExplicitTag(Lenient, ber.ExplicitTag):
    pass


class Type(Lenient, ber.StandardDecodeMixin, ber.Type):
    def set_tag(self, number, flags):
        if not Class.APPLICATION & flags:
            flags |= Class.CONTEXT_SPECIFIC

        super().set_tag(number, flags)


def check_integer_is_minimal(self, data, offset, length):
    """An INTEGER, or an ENUMERATED, is in as few bytes as possible."""

    if self.lenient:
        return

    if length == 0:
        raise self.decode_error("Expected at least one byte of an INTEGER.", offset)

    if length > 1 and (
        (data[offset] == 0x00 and data[offset + 1] < 0x80)
        or (data[offset] == 0xFF and data[offset + 1] >= 0x80)
    ):
        raise self.decode_error(
            "Expected an INTEGER in as few bytes as possible, but got a "
            "first byte of 0x{:02x} before 0x{:02x}.".format(
                data[offset], data[offset + 1]
            ),
            offset,
        )


class Boolean(Lenient, ber.Boolean):
    def decode_content(self, data, offset, length):
        decoded, end_offset = super().decode_content(data, offset, length)

        if not self.lenient and data[offset] not in (0x00, 0xFF):
            raise self.decode_error(
                "Expected BOOLEAN contents of 0x00 or 0xff, but got 0x{:02x}.".format(
                    data[offset]
                ),
                offset,
            )

        return decoded, end_offset


class Enumerated(Lenient, ber.Enumerated):
    def decode_content(self, data, offset, length):
        check_integer_is_minimal(self, data, offset, length)

        return super().decode_content(data, offset, length)


class Null(Lenient, ber.Null):
    pass


class Real(Lenient, ber.Real):
    def decode_content(self, data, offset, length):
        end_offset = offset + length

        try:
            decoded = ber.decode_real(data[offset:end_offset], not self.lenient)
        except DecodeError as error:
            raise self.decode_error(str(error.message), offset) from None

        return decoded, end_offset


class Date(Lenient, ber.Date):
    pass


class TimeOfDay(Lenient, ber.TimeOfDay):
    pass


class DateTime(Lenient, ber.DateTime):
    pass


class Duration(Lenient, ber.Duration):
    def encode_content(self, data, values=None):
        validate_duration(data)
        canonical = canonical_duration(data)
        return canonical[1:].encode("ascii")

    def decode_content(self, data, offset, length):
        end_offset = offset + length
        raw = ber.decode_ascii(data, offset, end_offset)
        text = "P" + raw
        try:
            validate_duration(text)
        except EncodeError as e:
            raise self.decode_error(str(e.message), offset) from None

        if not self.lenient:
            canonical = canonical_duration(text)
            if canonical != text:
                raise self.decode_error(
                    "The DURATION is not in canonical form.", offset
                )

        return text, end_offset


class ObjectIdentifier(Lenient, ber.ObjectIdentifier):
    def decode_content(self, data, offset, length):
        if not self.lenient:
            start = True

            for index in range(offset, offset + length):
                # A subidentifier is in as few bytes as possible.
                if start and data[index] == 0x80:
                    raise self.decode_error(
                        "Expected an OBJECT IDENTIFIER with subidentifiers in as "
                        "few bytes as possible, but got a leading 0x80.",
                        index,
                    )

                start = not data[index] & 0x80

        return super().decode_content(data, offset, length)


class StringType(StandardEncodeMixin, Type):
    TAG: int
    ENCODING: str

    def __init__(self, name):
        super(StringType, self).__init__(name, self.__class__.__name__, self.TAG)

    def encode_content(self, data, values=None):
        return data.encode(self.ENCODING)

    def decode_content(self, data, offset, length):
        end_offset = offset + length
        return data[offset:end_offset].decode(self.ENCODING), end_offset


class ArrayType(StandardEncodeMixin, Type):
    def __init__(self, name, tag_name, tag, element_type):
        super(ArrayType, self).__init__(name, tag_name, tag, Encoding.CONSTRUCTED)
        self.element_type = element_type

    def set_tag(self, number, flags):
        super(ArrayType, self).set_tag(number, flags | Encoding.CONSTRUCTED)

    def encode_content(self, data, values=None):
        encoded_elements = bytearray()

        for entry in data:
            self.element_type.encode(entry, encoded_elements)

        return encoded_elements

    def decode_content(self, data, offset, length):
        decoded = []
        start_offset = offset

        while (offset - start_offset) < length:
            decoded_element, offset = self.element_type.decode(data, offset)
            decoded.append(decoded_element)

        return decoded, offset

    def __repr__(self):
        return "{}({}, {})".format(
            self.__class__.__name__, self.name, self.element_type
        )


class Integer(NamedNumbersMixin, StandardEncodeMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name, "INTEGER", Tag.INTEGER)

    def encode_content(self, data, values=None):
        return encode_signed_integer(self.to_number(data))

    def decode_content(self, data, offset, length):
        check_integer_is_minimal(self, data, offset, length)
        end_offset = offset + length

        return (
            int.from_bytes(data[offset:end_offset], byteorder="big", signed=True),
            end_offset,
        )


class BitString(Type):
    def __init__(self, name, has_named_bits):
        super(BitString, self).__init__(name, "BIT STRING", Tag.BIT_STRING)
        self.has_named_bits = has_named_bits

    def is_default(self, value):
        if self.default is None:
            return False

        clean_value = clean_bit_string_value(value, self.has_named_bits)
        clean_default = clean_bit_string_value(self.default, self.has_named_bits)

        return clean_value == clean_default

    def encode(self, data, encoded, values=None):
        # X.690 11.2.2: Trailing zero bits of a named bit list are left out.
        if self.has_named_bits:
            data = clean_bit_string_value(data, True)

        number_of_bytes, number_of_rest_bits = divmod(data[1], 8)
        data = bytearray(data[0])

        if number_of_rest_bits == 0:
            data = data[:number_of_bytes]
            number_of_unused_bits = 0
        else:
            last_byte = data[number_of_bytes]
            last_byte &= (0xFF >> number_of_rest_bits) ^ 0xFF
            data = data[:number_of_bytes]
            data.append(last_byte)
            number_of_unused_bits = 8 - number_of_rest_bits

        encoded.extend(self.tag)
        encoded.extend(encode_length_definite(len(data) + 1))
        encoded.append(number_of_unused_bits)
        encoded.extend(data)

    def decode_content(self, data, offset, length):
        end_offset = offset + length

        if length == 0:
            raise self.decode_error(
                "Expected at least one byte of a BIT STRING.", offset
            )

        number_of_unused_bits = data[offset]

        if number_of_unused_bits > 7:
            raise self.decode_error(
                "Expected at most 7 unused bits of a BIT STRING, but got {}.".format(
                    number_of_unused_bits
                ),
                offset,
            )

        number_of_bits = 8 * (length - 1) - number_of_unused_bits
        offset += 1

        if not self.lenient:
            if number_of_bits < 0 or (
                number_of_unused_bits
                and data[end_offset - 1] & ((1 << number_of_unused_bits) - 1)
            ):
                raise self.decode_error(
                    "Expected the unused bits of a BIT STRING to be zero.", offset - 1
                )

            if self.has_named_bits and number_of_bits > 0:
                if not data[end_offset - 1] >> number_of_unused_bits & 1:
                    raise self.decode_error(
                        "Expected a BIT STRING with named bits without trailing "
                        "zero bits.",
                        offset - 1,
                    )

        return (bytes(data[offset:end_offset]), number_of_bits), end_offset


class OctetString(StandardEncodeMixin, Type):
    def __init__(self, name):
        super(OctetString, self).__init__(name, "OCTET STRING", Tag.OCTET_STRING)

    def encode_content(self, data, values=None):
        return data

    def decode_content(self, data, offset, length):
        end_offset = offset + length

        return bytes(data[offset:end_offset]), end_offset


class SequenceOf(ArrayType):
    def __init__(self, name, element_type):
        super(SequenceOf, self).__init__(
            name, "SEQUENCE OF", Tag.SEQUENCE, element_type
        )


# X.690 11.6: The encodings of a SET OF are sorted as octet strings, with
# the shorter padded with zero octets.
def padded(encoded, size):
    return encoded.ljust(size, b"\x00")


class SetOf(ArrayType):
    def __init__(self, name, element_type):
        super(SetOf, self).__init__(name, "SET OF", Tag.SET, element_type)

    def encode_content(self, data, values=None):
        elements = []

        for entry in data:
            encoded = bytearray()
            self.element_type.encode(entry, encoded)
            elements.append(bytes(encoded))

        longest = max((len(element) for element in elements), default=0)

        return b"".join(sorted(elements, key=lambda item: padded(item, longest)))

    def decode_content(self, data, offset, length):
        decoded = []
        start_offset = offset
        previous = None

        while (offset - start_offset) < length:
            element_offset = offset
            decoded_element, offset = self.element_type.decode(data, offset)
            decoded.append(decoded_element)

            if not self.lenient:
                encoded = bytes(data[element_offset:offset])

                if previous is not None:
                    longest = max(len(previous), len(encoded))

                    if padded(previous, longest) > padded(encoded, longest):
                        raise self.decode_error(
                            "Expected the elements of a SET OF in ascending order "
                            "of their encodings.",
                            element_offset,
                        )

                previous = encoded

        return decoded, offset


class UTF8String(StringType):
    TAG = Tag.UTF8_STRING
    ENCODING = "utf-8"


class NumericString(StringType):
    TAG = Tag.NUMERIC_STRING
    ENCODING = "ascii"


class PrintableString(StringType):
    TAG = Tag.PRINTABLE_STRING
    ENCODING = "ascii"


class IA5String(StringType):
    TAG = Tag.IA5_STRING
    ENCODING = "ascii"


class VisibleString(StringType):
    TAG = Tag.VISIBLE_STRING
    ENCODING = "ascii"


class GeneralString(StringType):
    TAG = Tag.GENERAL_STRING
    ENCODING = "latin-1"


class BMPString(StringType):
    TAG = Tag.BMP_STRING
    ENCODING = "utf-16-be"


class UniversalString(StringType):
    TAG = Tag.UNIVERSAL_STRING
    ENCODING = "utf-32-be"


class GraphicString(StringType):
    TAG = Tag.GRAPHIC_STRING
    ENCODING = "latin-1"


class TeletexString(StringType):
    TAG = Tag.T61_STRING
    ENCODING = "iso-8859-1"


class VideotexString(StringType):
    TAG = Tag.VIDEOTEX_STRING
    ENCODING = "iso-8859-1"


def in_utc(date):
    """The time in UTC, with no offset, as DER has it."""

    offset = date.utcoffset()

    if offset is not None:
        date = (date - offset).replace(tzinfo=None)

    return date


# DER has one form of each (X.690 11.7, 11.8). A lenient reader also
# takes any other BER has, as the same time in UTC.
class UTCTime(StandardEncodeMixin, Type):
    def __init__(self, name):
        super(UTCTime, self).__init__(name, "UTCTime", Tag.UTC_TIME)

    def encode_content(self, data, values=None):
        return restricted_utc_time_from_datetime(data).encode("ascii")

    def decode_content(self, data, offset, length):
        end_offset = offset + length
        decoded = ber.decode_ascii(data, offset, end_offset)

        try:
            return restricted_utc_time_to_datetime(decoded), end_offset
        except Error as error:
            if not self.lenient:
                raise self.decode_error(str(error), offset) from None

        try:
            return in_utc(utc_time_to_datetime(decoded)), end_offset
        except Error as error:
            raise self.decode_error(str(error), offset) from None


class GeneralizedTime(StandardEncodeMixin, Type):
    def __init__(self, name):
        super(GeneralizedTime, self).__init__(
            name, "GeneralizedTime", Tag.GENERALIZED_TIME
        )

    def encode_content(self, data, values=None):
        return restricted_generalized_time_from_datetime(data).encode("ascii")

    def decode_content(self, data, offset, length):
        end_offset = offset + length
        decoded = ber.decode_ascii(data, offset, end_offset)

        try:
            return restricted_generalized_time_to_datetime(decoded), end_offset
        except Error as error:
            if not self.lenient:
                raise self.decode_error(str(error), offset) from None

        try:
            return in_utc(generalized_time_to_datetime(decoded)), end_offset
        except Error as error:
            raise self.decode_error(str(error), offset) from None


class Compiler(ber.Compiler):
    explicit_tag_class = ExplicitTag

    def __init__(self, specification, numeric_enums=False, lenient=False):
        super().__init__(specification, numeric_enums)
        self._lenient = lenient

    def compile_type(self, name, type_descriptor, module_name):
        compiled = super().compile_type(name, type_descriptor, module_name)

        if self._lenient:
            make_lenient(compiled)

        return compiled

    def compile_implicit_type(self, name, type_descriptor, module_name):
        compiled = self.compile_der_type(name, type_descriptor, module_name)

        # The type inside an explicit tag does not go through compile_type().
        if self._lenient:
            make_lenient(compiled)

        return compiled

    def compile_der_type(self, name, type_descriptor, module_name):
        type_name = type_descriptor["type"]

        compiled: ber.Type
        if type_name == "SEQUENCE":
            compiled = Sequence(
                name, *self.compile_members(type_descriptor["members"], module_name)
            )
        elif type_name == "SEQUENCE OF":
            compiled = SequenceOf(
                name, self.compile_type("", type_descriptor["element"], module_name)
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
            has_named_bits = "named-bits" in type_descriptor
            compiled = BitString(name, has_named_bits)
        elif type_name in ["ANY", "OpenType"]:
            # An open type is the encoding of its value as it is.
            compiled = Any(name)
        elif type_name == "ANY DEFINED BY":
            choices = {}

            for key, value in type_descriptor["choices"].items():
                choices[key] = self.compile_type(key, value, module_name)

            compiled = AnyDefinedBy(name, type_descriptor["value"], choices)
        elif type_name == "NULL":
            compiled = Null(name)
        elif type_name == "EXTERNAL":
            compiled = Sequence(
                name,
                *self.compile_members(
                    self.external_type_descriptor()["members"], module_name
                ),
            )
            compiled.set_tag(Tag.EXTERNAL, 0)
        elif type_name == "ObjectDescriptor":
            compiled = ObjectDescriptor(name)
        else:
            if type_name in self.types_backtrace:
                compiled = Recursive(name, type_name, module_name)
                self.recursive_types.append(compiled)
            else:
                compiled = self.compile_user_type(name, type_name, module_name)

        return compiled


class DERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums=False, lenient=False):
        return Compiler(specification, numeric_enums, lenient).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)
