# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Basic Octet Encoding Rules (OER) codec."""

import binascii
import datetime
import struct
from copy import copy
import typing

from .duration import dict_to_duration, duration_to_dict
from asnkitrt.codecs.codec import Codec

from ..constants import EXTENSION_MARKER
from . import (
    NamedNumbersMixin,
    BaseType,
    bmp_string_decode,
    bmp_string_encode,
    DecodeError,
    EncodeError,
    ErrorWithLocation,
    OutOfDataError,
    compiler,
    format_bytes,
    format_or,
    generalized_time_from_datetime,
    generalized_time_to_datetime,
    utc_time_from_datetime,
    utc_time_to_datetime,
)
from .ber import (
    Class,
    Tag,
    decode_object_identifier,
    decode_real,
    encode_object_identifier,
    encode_real,
)
from .compiler import constraint_parts, enum_values_as_dict, hull


def encode_tag(number, flags):
    if number < 63:
        tag = bytearray([flags | number])
    else:
        tag = bytearray([flags | 0x3F])
        encoded = bytearray()

        while number > 0:
            encoded.append(0x80 | (number & 0x7F))
            number >>= 7

        encoded[0] &= 0x7F
        encoded.reverse()
        tag.extend(encoded)

    return bytes(tag)


class Encoder(object):
    def __init__(self):
        self.number_of_bits = 0
        self.value = 0

    def __iadd__(self, other):
        self.append_non_negative_binary_integer(other.value, other.number_of_bits)

        return self

    def number_of_bytes(self):
        return (self.number_of_bits + 7) // 8

    def set_bit(self, offset):
        self.value |= 1 << (self.number_of_bits - offset - 1)

    def align(self):
        width = 8 * self.number_of_bytes() - self.number_of_bits
        self.number_of_bits += width
        self.value <<= width

    def append_bit(self, bit):
        """Append given bit."""

        self.number_of_bits += 1
        self.value <<= 1
        self.value |= bit

    def append_bits(self, data, number_of_bits):
        """Append given bits."""

        if number_of_bits == 0:
            return

        value = int(binascii.hexlify(data), 16)
        value >>= 8 * len(data) - number_of_bits

        self.append_non_negative_binary_integer(value, number_of_bits)

    def append_u8(self, value):
        return self.append_non_negative_binary_integer(value, 8)

    def append_bytes(self, data):
        """Append given data."""

        self.append_bits(data, 8 * len(data))

    def as_bytearray(self):
        """Return the bits as a bytearray."""

        if self.number_of_bits == 0:
            return bytearray()

        data = self.value
        data |= 0x80 << self.number_of_bits

        return bytearray(binascii.unhexlify(hex(data)[4:].rstrip("L")))

    def append_length_determinant(self, value):
        if value < 128:
            self.append_non_negative_binary_integer(value, 8)
        else:
            encoded = bytearray()

            while value > 0:
                encoded.append(value & 0xFF)
                value >>= 8

            length = len(encoded)

            if length > 127:
                raise EncodeError("Length determinant {} is too big.".format(value))

            self.append_u8(0x80 | length)
            self.append_bytes(encoded[::-1])

    def append_non_negative_binary_integer(self, value, number_of_bits):
        """Append given integer value."""

        self.number_of_bits += number_of_bits
        self.value <<= number_of_bits
        self.value |= value

    def append_integer(self, value):
        number_of_bits = value.bit_length()

        if value < 0:
            number_of_bytes = (number_of_bits + 7) // 8
            value = (1 << (8 * number_of_bytes)) + value

            if (value & (1 << (8 * number_of_bytes - 1))) == 0:
                value |= 0xFF << (8 * number_of_bytes)
                number_of_bytes += 1
        elif value > 0:
            number_of_bytes = (number_of_bits + 7) // 8

            if number_of_bits == (8 * number_of_bytes):
                number_of_bytes += 1
        else:
            number_of_bytes = 1

        self.append_length_determinant(number_of_bytes)
        self.append_non_negative_binary_integer(value, 8 * number_of_bytes)

    def append_unsigned_integer(self, value):
        number_of_bits = max(value.bit_length(), 1)
        number_of_bytes = (number_of_bits + 7) // 8
        self.append_length_determinant(number_of_bytes)
        self.append_non_negative_binary_integer(value, 8 * number_of_bytes)

    def __repr__(self):
        return format_bytes(self.as_bytearray())


class Decoder(object):
    def __init__(self, encoded):
        self.number_of_bits = 8 * len(encoded)
        self.total_number_of_bits = self.number_of_bits

        if len(encoded) > 0:
            self.value = int(binascii.hexlify(encoded), 16)
        else:
            self.value = 0

    def align(self):
        width = self.number_of_bits & 0x7
        self.number_of_bits -= width

    def number_of_read_bits(self):
        return self.total_number_of_bits - self.number_of_bits

    def skip_bits(self, number_of_bits):
        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        self.number_of_bits -= number_of_bits

    def peek_bit(self):
        return (self.value >> (self.number_of_bits - 1)) & 1

    def clear_bit(self):
        self.value &= (1 << (self.number_of_bits - 1)) - 1

    def read_bit(self):
        """Read a bit."""

        if self.number_of_bits == 0:
            raise OutOfDataError(self.number_of_read_bits())

        self.number_of_bits -= 1

        return (self.value >> self.number_of_bits) & 1

    def read_bits(self, number_of_bits):
        """Read given number of bits."""

        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        self.number_of_bits -= number_of_bits
        mask = (1 << number_of_bits) - 1
        value = (self.value >> self.number_of_bits) & mask
        value &= mask
        value |= 0x80 << number_of_bits

        return binascii.unhexlify(hex(value)[4:].rstrip("L"))

    def read_byte(self):
        return self.read_non_negative_binary_integer(8)

    def read_bytes(self, number_of_bytes):
        return self.read_bits(8 * number_of_bytes)

    def read_non_negative_binary_integer(self, number_of_bits):
        """Read an integer value of given number of bits."""

        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        self.number_of_bits -= number_of_bits
        mask = (1 << number_of_bits) - 1

        return (self.value >> self.number_of_bits) & mask

    def read_length_determinant(self):
        value = self.read_byte()

        if value & 0x80:
            length = value & 0x7F
            value = self.read_non_negative_binary_integer(8 * length)

        return value

    def read_integer(self):
        number_of_bytes = self.read_length_determinant()
        number_of_bits = 8 * number_of_bytes
        value = self.read_non_negative_binary_integer(number_of_bits)

        if value & (1 << (number_of_bits - 1)):
            value -= (1 << number_of_bits) - 1
            value -= 1

        return value

    def read_unsigned_integer(self):
        number_of_bytes = self.read_length_determinant()
        number_of_bits = 8 * number_of_bytes

        return self.read_non_negative_binary_integer(number_of_bits)

    def read_tag(self):
        byte = self.read_byte()
        tag = bytearray([byte])

        if byte & 0x3F == 0x3F:
            while True:
                byte = self.read_byte()
                tag.append(byte)

                if byte & 0x80 == 0:
                    break

        return bytes(tag)


class Type(BaseType):
    def __init__(self, name, type_name, number, flags=0):
        super().__init__(name, type_name)
        self.module_name = None

        if number is None:
            self.tag = None
        else:
            self.tag = encode_tag(number, flags)

    def set_tag(self, number, flags):
        if not Class.APPLICATION & flags:
            flags |= Class.CONTEXT_SPECIFIC

        self.tag = encode_tag(number, flags)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        pass

    def set_restricted_to_range(self, minimum, maximum, has_extension_marker):
        pass


class StringType(Type):
    """A restricted character string, which is a length and its octets."""

    TAG: int
    ENCODING: str
    # The octets of a character of a string of a known multiplier (X.696
    # 27.1), or None for the other strings.
    WIDTH: int | None = None

    def __init__(self, name, minimum=None, maximum=None, has_extension_marker=False):
        super(StringType, self).__init__(name, self.__class__.__name__, self.TAG)
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.number_of_characters = None

        # A string of a known multiplier with a size of one value has no
        # length (X.696 27.2).
        if (
            self.WIDTH is not None
            and not has_extension_marker
            and minimum not in [None, "MIN"]
            and minimum == maximum
        ):
            self.number_of_characters = minimum

    def encode_text(self, data):
        return data.encode(self.ENCODING)

    def decode_text(self, encoded):
        return encoded.decode(self.ENCODING)

    def encode(self, data, encoder):
        encoded = self.encode_text(data)

        if self.number_of_characters is None:
            encoder.append_length_determinant(len(encoded))
        elif len(data) != self.number_of_characters:
            raise EncodeError(
                "Expected a string of {} characters, but got {}.".format(
                    self.number_of_characters, len(data)
                )
            )

        encoder.append_bytes(encoded)

    def decode(self, decoder):
        if self.number_of_characters is None:
            number_of_bytes = decoder.read_length_determinant()
        else:
            number_of_bytes = self.WIDTH * self.number_of_characters

        return self.decode_text(decoder.read_bytes(number_of_bytes))


class KnownMultiplierStringType(StringType):
    WIDTH = 1


class MembersType(Type):
    def __init__(self, name, type_name, tag, root_members, additions):
        super(MembersType, self).__init__(name, type_name, tag)
        self.root_members = root_members
        self.additions = additions
        self.optionals = [
            member
            for member in root_members
            if member.optional or member.default is not None
        ]

    def encode(self, data, encoder):
        if self.additions is not None:
            offset = encoder.number_of_bits
            encoder.append_bit(0)
            self.encode_root(data, encoder)

            if len(self.additions) > 0:
                if self.encode_additions(data, encoder):
                    encoder.set_bit(offset)
        else:
            self.encode_root(data, encoder)

    def encode_root(self, data, encoder):
        for optional in self.optionals:
            if optional.optional:
                encoder.append_bit(optional.name in data)
            elif optional.name in data:
                try:
                    is_default = optional.is_default(data[optional.name])
                except ErrorWithLocation as e:
                    e.add_location(optional)
                    raise e

                encoder.append_bit(not is_default)
            else:
                encoder.append_bit(0)

        encoder.align()

        for member in self.root_members:
            self.encode_member(member, data, encoder)

    def encode_additions(self, data, encoder):
        # Encode extension additions.
        presence_bits = 0
        addition_encoders = []
        number_of_presence_bits = 0

        try:
            for addition in self.additions:
                presence_bits <<= 1
                number_of_presence_bits += 1
                addition_encoder = Encoder()
                self.encode_member(
                    addition, data, addition_encoder, encode_default=True
                )

                if addition_encoder.number_of_bits > 0 or addition.name in data:
                    addition_encoders.append(addition_encoder)
                    presence_bits |= 1
        except EncodeError:
            pass

        # Return false if no extension additions are present.
        if not addition_encoders:
            return False

        # Presence bit field, where the additions after the last are not
        # there.
        number_of_additions = len(self.additions)
        presence_bits <<= number_of_additions - number_of_presence_bits
        number_of_unused_bits = (8 - number_of_additions % 8) % 8
        encoder.append_length_determinant(((number_of_additions + 7) // 8) + 1)
        encoder.append_non_negative_binary_integer(number_of_unused_bits, 8)
        encoder.append_non_negative_binary_integer(presence_bits, number_of_additions)
        encoder.align()

        for addition_encoder in addition_encoders:
            encoder.append_length_determinant(addition_encoder.number_of_bytes())
            encoder += addition_encoder

        return True

    def encode_member(self, member, data, encoder, encode_default=False):
        name = member.name

        if name in data:
            try:
                if isinstance(member, AnyDefinedBy):
                    member.encode(data[name], encoder, data)
                elif member.default is None:
                    member.encode(data[name], encoder)
                elif not member.is_default(data[name]) or encode_default:
                    member.encode(data[name], encoder)
            except ErrorWithLocation as e:
                # Add member location
                e.add_location(member)
                raise e

        elif member.optional or member.has_default():
            pass
        else:
            raise EncodeError(
                "{} member '{}' not found in {}.".format(
                    self.__class__.__name__, name, data
                )
            )

    def decode(self, decoder):
        if self.additions is not None:
            if decoder.read_bit():
                decoded = self.decode_root(decoder)
                decoded.update(self.decode_additions(decoder, decoded))

                return decoded
            else:
                return self.decode_root(decoder)
        else:
            return self.decode_root(decoder)

    def decode_root(self, decoder):
        values: dict[str, typing.Any] = {}
        optionals = {optional: decoder.read_bit() for optional in self.optionals}

        decoder.align()

        for member in self.root_members:
            if optionals.get(member, True):
                try:
                    if isinstance(member, AnyDefinedBy):
                        value = member.decode(decoder, values)
                    else:
                        value = member.decode(decoder)
                except ErrorWithLocation as e:
                    # Add member location
                    e.add_location(member)
                    raise e

                values[member.name] = value
            elif member.has_default():
                values[member.name] = member.default

        return values

    def decode_additions(self, decoder, before):
        # Presence bit field.
        length = decoder.read_length_determinant()
        unused_bits = decoder.read_byte()
        num_additions = ((length - 1) * 8) - unused_bits
        presence_bits = decoder.read_non_negative_binary_integer(num_additions)
        decoder.align()

        decoded: dict[str, typing.Any] = {}

        for i in range(num_additions):
            if presence_bits & (1 << (num_additions - i - 1)):
                member_length = decoder.read_length_determinant()

                if i < len(self.additions):
                    addition = self.additions[i]
                    try:
                        if isinstance(addition, AnyDefinedBy):
                            values = {**before, **decoded}
                            decoded[addition.name] = addition.decode(decoder, values)
                        else:
                            decoded[addition.name] = addition.decode(decoder)
                    except ErrorWithLocation as e:
                        # Add member location
                        e.add_location(addition)
                        raise e
                else:
                    decoder.skip_bits(8 * member_length)

        return decoded

    def __repr__(self):
        return "{}({}, [{}])".format(
            self.__class__.__name__,
            self.name,
            ", ".join([repr(member) for member in self.root_members]),
        )


class ArrayType(Type):
    def __init__(self, name, type_name, tag, element_type):
        super(ArrayType, self).__init__(name, type_name, tag)
        self.element_type = element_type

    def encode(self, data, encoder):
        encoder.append_unsigned_integer(len(data))

        for entry in data:
            self.element_type.encode(entry, encoder)

    def decode(self, decoder):
        length = decoder.read_unsigned_integer()
        decoded = []

        for _ in range(length):
            decoded_element = self.element_type.decode(decoder)
            decoded.append(decoded_element)

        return decoded

    def __repr__(self):
        return "{}({}, {})".format(
            self.__class__.__name__, self.name, self.element_type
        )


class Boolean(Type):
    def __init__(self, name):
        super(Boolean, self).__init__(name, "BOOLEAN", Tag.BOOLEAN)

    def encode(self, data, encoder):
        encoder.append_non_negative_binary_integer(0xFF * data, 8)

    def decode(self, decoder):
        return bool(decoder.read_byte())


class Integer(NamedNumbersMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name, "INTEGER", Tag.INTEGER)
        self.has_extension_marker = False
        self.length = None
        self.fmt = None
        self.signed = True

    def set_restricted_to_range(self, minimum, maximum, has_extension_marker):
        self.has_extension_marker = has_extension_marker

        # An extensible constraint is not OER-visible, so the integer has
        # no bounds (X.696 8.2.2, 10.4 Note 2).
        if has_extension_marker:
            return

        if minimum != "MIN":
            self.signed = minimum < 0

        if minimum == "MIN" or maximum == "MAX":
            return

        if minimum >= 0:
            if maximum < 256:
                self.length = 1
                self.fmt = ">B"
            elif maximum < 65536:
                self.length = 2
                self.fmt = ">H"
            elif maximum < 4294967296:
                self.length = 4
                self.fmt = ">I"
            elif maximum < 18446744073709551616:
                self.length = 8
                self.fmt = ">Q"
        elif minimum >= -128 and maximum < 128:
            self.length = 1
            self.fmt = ">b"
        elif minimum >= -32768 and maximum < 32768:
            self.length = 2
            self.fmt = ">h"
        elif minimum >= -2147483648 and maximum < 2147483648:
            self.length = 4
            self.fmt = ">i"
        elif minimum >= -9223372036854775808 and maximum < 9223372036854775808:
            self.length = 8
            self.fmt = ">q"

    def encode(self, data, encoder):
        data = self.to_number(data)

        if self.fmt:
            encoder.append_bytes(struct.pack(self.fmt, data))
        elif self.signed:
            encoder.append_integer(data)
        else:
            encoder.append_unsigned_integer(data)

    def decode(self, decoder):
        if self.fmt:
            return struct.unpack(self.fmt, decoder.read_bytes(self.length))[0]
        elif self.signed:
            return decoder.read_integer()
        else:
            return decoder.read_unsigned_integer()


class Real(Type):
    def __init__(self, name, with_components):
        super(Real, self).__init__(name, "REAL", Tag.REAL)

        if with_components is None:
            self.length: typing.Any = None
            self.fmt = None
        else:
            mantissa, base, exponent = self.unpack_with_components(with_components)

            if all([mantissa, base, exponent]):
                if self.is_binary32(mantissa, base, exponent):
                    self.length = 4
                    self.fmt = ">f"
                elif self.is_binary64(mantissa, base, exponent):
                    self.length = 8
                    self.fmt = ">d"
                else:
                    self.length = None
                    self.fmt = None
            else:
                self.length = None
                self.fmt = None

    def unpack_with_components(self, with_components):
        """Returns the least ranges of the mantissa, the base and the
        exponent that have those of all the alternatives (X.696 12.1),
        or ``None`` for each if one has no bounds or the constraint is
        extensible, as it is then not OER-visible (X.696 8.2.2).

        """

        hulls: typing.Dict[str, typing.Tuple[int, int]] = {}
        unbounded = (None, None, None)

        for alternative in with_components:
            if alternative == EXTENSION_MARKER:
                return unbounded

            components = {component["name"]: component for component in alternative}

            for name in ["mantissa", "base", "exponent"]:
                restricted_to = components.get(name, {}).get("restricted-to")

                if restricted_to is None or EXTENSION_MARKER in restricted_to:
                    return unbounded

                lower, upper = hull(
                    constraint_parts(restricted_to, lambda bound: bound)
                )

                if not isinstance(lower, int) or not isinstance(upper, int):
                    return unbounded

                if name in hulls:
                    lower = min(lower, hulls[name][0])
                    upper = max(upper, hulls[name][1])

                hulls[name] = (lower, upper)

        if len(hulls) != 3:
            return unbounded

        return (hulls["mantissa"], hulls["base"], hulls["exponent"])

    def is_binary32(self, mantissa, base, exponent):
        return (
            -16777215 <= mantissa[0] <= mantissa[1] <= 16777215
            and base == (2, 2)
            and -149 <= exponent[0] <= exponent[1] <= 104
        )

    def is_binary64(self, mantissa, base, exponent):
        return (
            -9007199254740991 <= mantissa[0] <= mantissa[1] <= 9007199254740991
            and base == (2, 2)
            and -1074 <= exponent[0] <= exponent[1] <= 971
        )

    def encode(self, data, encoder):
        if self.fmt is None:
            encoded = encode_real(data)
            encoder.append_length_determinant(len(encoded))
            encoder.append_bytes(encoded)
        else:
            try:
                encoder.append_bytes(struct.pack(self.fmt, data))
            except (struct.error, OverflowError):
                raise EncodeError(
                    "Expected an IEEE 754 {} bits floating point number, but "
                    "got {}.".format(8 * self.length, data)
                )

    def decode(self, decoder):
        if self.fmt is None:
            length = decoder.read_length_determinant()

            # The contents of CER and DER (X.696 12.4).
            return decode_real(bytearray(decoder.read_bytes(length)), strict=True)
        else:
            return struct.unpack(self.fmt, decoder.read_bytes(self.length))[0]


class Null(Type):
    def __init__(self, name):
        super(Null, self).__init__(name, "NULL", Tag.NULL)

    def encode(self, _data, _encoder):
        return

    def decode(self, _decoder):
        return


class BitString(Type):
    def __init__(self, name, named_bits, minimum, maximum, has_extension_marker):
        super(BitString, self).__init__(name, "BIT STRING", Tag.BIT_STRING)
        self.named_bits = named_bits
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.number_of_bits = None

        if minimum is not None or maximum is not None:
            if not has_extension_marker:
                if minimum == maximum:
                    self.number_of_bits = minimum

    def encode(self, data, encoder):
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
            number_of_bytes += 1

        if self.number_of_bits is None:
            encoder.append_length_determinant(number_of_bytes + 1)
            encoder.append_non_negative_binary_integer(number_of_unused_bits, 8)
            encoder.append_bytes(data)
        else:
            encoder.append_bytes(data)

    def decode(self, decoder):
        if self.number_of_bits is None:
            number_of_bytes = decoder.read_length_determinant()
            number_of_unused_bits = decoder.read_byte()
            number_of_bytes -= 1
            number_of_bits = 8 * number_of_bytes - number_of_unused_bits
        else:
            number_of_bytes = (self.number_of_bits + 7) // 8
            number_of_bits = self.number_of_bits

        return (decoder.read_bytes(number_of_bytes), number_of_bits)


class OctetString(Type):
    def __init__(self, name, minimum, maximum, has_extension_marker):
        super(OctetString, self).__init__(name, "OCTET STRING", Tag.OCTET_STRING)
        self.number_of_bytes = None
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.number_of_bytes = None

        if minimum is not None or maximum is not None:
            if not has_extension_marker:
                if minimum == maximum:
                    self.number_of_bytes = minimum

    def encode(self, data, encoder):
        if self.number_of_bytes is None:
            encoder.append_length_determinant(len(data))
            encoder.append_bytes(data)
        else:
            encoder.append_bytes(data)

    def decode(self, decoder):
        if self.number_of_bytes is None:
            number_of_bytes = decoder.read_length_determinant()
        else:
            number_of_bytes = self.number_of_bytes

        return decoder.read_bytes(number_of_bytes)


class ObjectIdentifier(Type):
    def __init__(self, name):
        super(ObjectIdentifier, self).__init__(
            name, "OBJECT IDENTIFIER", Tag.OBJECT_IDENTIFIER
        )

    def encode(self, data, encoder):
        encoded_subidentifiers = encode_object_identifier(data)
        encoder.append_length_determinant(len(encoded_subidentifiers))
        encoder.append_bytes(bytearray(encoded_subidentifiers))

    def decode(self, decoder):
        length = decoder.read_length_determinant()
        data = decoder.read_bytes(length)

        return decode_object_identifier(bytearray(data), 0, len(data))


class Enumerated(Type):
    def __init__(self, name, values, numeric):
        super(Enumerated, self).__init__(name, "ENUMERATED", Tag.ENUMERATED)

        if numeric:
            self.value_to_data = {k: k for k in enum_values_as_dict(values)}
            self.data_to_value = self.value_to_data
        else:
            self.value_to_data = enum_values_as_dict(values)
            self.data_to_value = {v: k for k, v in self.value_to_data.items()}

        self.has_extension_marker = EXTENSION_MARKER in values

    def format_names(self):
        return format_or(sorted(list(self.value_to_data.values())))

    def format_values(self):
        return format_or(sorted(list(self.value_to_data)))

    def encode(self, data, encoder):
        try:
            value = self.data_to_value[data]
        except KeyError:
            raise EncodeError(
                "Expected enumeration value {}, but got '{}'.".format(
                    self.format_names(), data
                )
            )

        if 0 <= value <= 127:
            encoder.append_non_negative_binary_integer(value, 8)
        else:
            offset = encoder.number_of_bits
            encoder.append_integer(value)
            encoder.set_bit(offset)

    def decode(self, decoder):
        if decoder.peek_bit():
            decoder.clear_bit()
            value = decoder.read_integer()
        else:
            value = decoder.read_byte()

        if value in self.value_to_data:
            return self.value_to_data[value]
        elif self.has_extension_marker:
            return None
        else:
            raise DecodeError(
                "Expected enumeration value {}, but got {}.".format(
                    self.format_values(), value
                )
            )


class Sequence(MembersType):
    def __init__(self, name, root_members, additions):
        super(Sequence, self).__init__(
            name, "SEQUENCE", Tag.SEQUENCE, root_members, additions
        )


class SequenceOf(ArrayType):
    def __init__(self, name, element_type):
        super(SequenceOf, self).__init__(
            name, "SEQUENCE OF", Tag.SEQUENCE, element_type
        )


class Set(MembersType):
    def __init__(self, name, root_members, additions):
        super(Set, self).__init__(name, "SET", Tag.SET, root_members, additions)


class SetOf(ArrayType):
    def __init__(self, name, element_type):
        super(SetOf, self).__init__(name, "SET OF", Tag.SET, element_type)


class Choice(Type):
    def __init__(self, name, root_members, additions):
        super(Choice, self).__init__(name, "CHOICE", None)
        self.root_members = root_members
        self.name_to_root_member = {member.name: member for member in root_members}

        if additions is None:
            additions = []
            self.has_extension_marker = False
        else:
            self.has_extension_marker = True

        self.additions = additions
        self.name_to_addition = {member.name: member for member in additions}
        # Made when first used, as the type of a recursive alternative is
        # not known before.
        self._tag_to_root_member: dict[typing.Any, typing.Any] | None = None
        self._tag_to_addition: dict[typing.Any, typing.Any] = {}

    @property
    def members(self):
        return self.root_members + self.additions

    @property
    def tag_to_root_member(self):
        if self._tag_to_root_member is None:
            self._tag_to_root_member = {}
            self.add_tags(self._tag_to_root_member, self.root_members)
            self.add_tags(self._tag_to_addition, self.additions)

        return self._tag_to_root_member

    @property
    def tag_to_addition(self):
        _ = self.tag_to_root_member

        return self._tag_to_addition

    def tags(self):
        """All the tags the choice can begin with."""

        return list(self.tag_to_root_member) + list(self.tag_to_addition)

    def add_tags(self, tag_to_member, members):
        # An untagged choice has the tags of its alternatives (X.696 20.1).
        for member in members:
            for tag in member_tags(member):
                tag_to_member[tag] = member

    def chosen_tag(self, data):
        """The outermost tag of the type of the chosen alternative, which for
        an untagged choice is that of the alternative chosen in it, and so
        on (X.696 20.1).

        """

        name = data[0]
        member = self.name_to_root_member.get(name) or self.name_to_addition.get(name)

        if member is None:
            return None

        return outer_tag(member, data[1])

    def format_tags(self):
        return format_or(sorted([format_bytes(tag) for tag in self.tags()]))

    def format_names(self):
        return format_or(sorted([member.name for member in self.members]))

    def encode(self, data, encoder):
        name = data[0]

        if name in self.name_to_root_member:
            member = self.name_to_root_member[name]
            encoder.append_bytes(self.chosen_tag(data))
            try:
                member.encode(data[1], encoder)
            except ErrorWithLocation as e:
                # Add member location
                e.add_location(member)
                raise e

        elif name in self.name_to_addition:
            member = self.name_to_addition[name]
            encoder.append_bytes(self.chosen_tag(data))
            addition_encoder = Encoder()
            try:
                member.encode(data[1], addition_encoder)
            except ErrorWithLocation as e:
                # Add member location
                e.add_location(member)
                raise e
            encoder.append_length_determinant(addition_encoder.number_of_bytes())
            encoder += addition_encoder
        else:
            raise EncodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), data[0])
            )

    # def encode_member(self, member, data, encoder):
    #     try:
    #         member.encode(data, encoder)
    #     except EncodeError as e:
    #         e.location.append(member.name)
    #         raise

    def decode(self, decoder):
        tag = decoder.read_tag()

        if tag in self.tag_to_root_member:
            member = self.tag_to_root_member[tag]
            try:
                decoded = member.decode(decoder)
            except ErrorWithLocation as e:
                # Add member location
                e.add_location(member)
                raise e
        elif tag in self.tag_to_addition:
            member = self.tag_to_addition[tag]
            decoder.read_length_determinant()
            try:
                decoded = member.decode(decoder)
            except ErrorWithLocation as e:
                # Add member location
                e.add_location(member)
                raise e
        elif self.has_extension_marker:
            length = decoder.read_length_determinant()
            decoder.skip_bits(8 * length)

            return (None, None)
        else:
            raise DecodeError(
                "Expected choice member tag {}, but got '{}'.".format(
                    self.format_tags(), format_bytes(tag)
                )
            )

        return (member.name, decoded)

    def __repr__(self):
        return "Choice({}, [{}])".format(
            self.name, ", ".join([repr(member) for member in self.members])
        )


class UTF8String(StringType):
    TAG = Tag.UTF8_STRING
    ENCODING = "utf-8"


class NumericString(KnownMultiplierStringType):
    TAG = Tag.NUMERIC_STRING
    ENCODING = "ascii"


class PrintableString(KnownMultiplierStringType):
    TAG = Tag.PRINTABLE_STRING
    ENCODING = "ascii"


class IA5String(KnownMultiplierStringType):
    TAG = Tag.IA5_STRING
    ENCODING = "ascii"


class VisibleString(KnownMultiplierStringType):
    TAG = Tag.VISIBLE_STRING
    ENCODING = "ascii"


class GeneralString(StringType):
    TAG = Tag.GENERAL_STRING
    ENCODING = "latin-1"


class BMPString(KnownMultiplierStringType):
    TAG = Tag.BMP_STRING
    ENCODING = "utf-16-be"
    WIDTH = 2

    def encode_text(self, data):
        return bmp_string_encode(data)

    def decode_text(self, encoded):
        return bmp_string_decode(encoded)


class GraphicString(StringType):
    TAG = Tag.GENERAL_STRING
    ENCODING = "latin-1"


class UniversalString(KnownMultiplierStringType):
    TAG = Tag.UNIVERSAL_STRING
    ENCODING = "utf-32-be"
    WIDTH = 4


class TeletexString(StringType):
    TAG = Tag.T61_STRING
    ENCODING = "iso-8859-1"


class VideotexString(StringType):
    TAG = Tag.VIDEOTEX_STRING
    ENCODING = "iso-8859-1"


class ObjectDescriptor(GraphicString):
    TAG = Tag.OBJECT_DESCRIPTOR


class UTCTime(VisibleString):
    TAG = Tag.UTC_TIME

    def encode(self, data, encoder):
        encoded = utc_time_from_datetime(data)

        return super(UTCTime, self).encode(encoded, encoder)

    def decode(self, decoder):
        decoded = super(UTCTime, self).decode(decoder)

        return utc_time_to_datetime(decoded)


class GeneralizedTime(VisibleString):
    TAG = Tag.GENERALIZED_TIME

    def encode(self, data, encoder):
        encoded = generalized_time_from_datetime(data)

        return super(GeneralizedTime, self).encode(encoded, encoder)

    def decode(self, decoder):
        decoded = super(GeneralizedTime, self).decode(decoder)

        return generalized_time_to_datetime(decoded)


class Date(Type):
    def __init__(self, name):
        super(Date, self).__init__(name, "DATE", None)
        year = Integer("year")
        month = Integer("month")
        day = Integer("day")
        month.set_restricted_to_range(1, 12, False)
        day.set_restricted_to_range(1, 31, False)
        self._inner = Sequence("DATE-ENCODING", [year, month, day], None)

    def encode(self, data, encoder):
        data = {"year": data.year, "month": data.month, "day": data.day}

        self._inner.encode(data, encoder)

    def decode(self, decoder):
        decoded = self._inner.decode(decoder)

        return datetime.date(decoded["year"], decoded["month"], decoded["day"])


class TimeOfDay(Type):
    def __init__(self, name):
        super(TimeOfDay, self).__init__(name, "TIME-OF-DAY", None)
        hours = Integer("hours")
        minutes = Integer("minutes")
        seconds = Integer("seconds")
        hours.set_restricted_to_range(0, 24, False)
        minutes.set_restricted_to_range(0, 59, False)
        seconds.set_restricted_to_range(0, 60, False)
        self._inner = Sequence("TIME-OF-DAY-ENCODING", [hours, minutes, seconds], None)

    def encode(self, data, encoder):
        data = {"hours": data.hour, "minutes": data.minute, "seconds": data.second}

        self._inner.encode(data, encoder)

    def decode(self, decoder):
        decoded = self._inner.decode(decoder)

        return datetime.time(decoded["hours"], decoded["minutes"], decoded["seconds"])


class Duration(Type):
    def __init__(self, name):
        super(Duration, self).__init__(name, "DURATION", None)

        def uint(field_name, opt=True):
            i = copy(Integer(field_name))
            i.set_restricted_to_range(0, "MAX", False)
            i.optional = opt
            return i

        years = uint("years")
        months = uint("months")
        weeks = uint("weeks")
        days = uint("days")
        hours = uint("hours")
        minutes = uint("minutes")
        seconds = uint("seconds")

        nod = uint("number-of-digits", False)
        fv = uint("fractional-value", False)
        fp = copy(Sequence("fractional-part", [nod, fv], None))
        fp.optional = True

        self._inner = Sequence(
            "DURATION-INTERVAL-ENCODING",
            [years, months, weeks, days, hours, minutes, seconds, fp],
            None,
        )

    def encode(self, data, encoder):
        fields = duration_to_dict(data)
        self._inner.encode(fields, encoder)

    def decode(self, decoder):
        fields = self._inner.decode(decoder)
        return dict_to_duration(fields)


class DateTime(Type):
    def __init__(self, name):
        super(DateTime, self).__init__(name, "DATE-TIME", None)
        self._date = Date("date")
        self._time = TimeOfDay("time")

    def encode(self, data, encoder):
        self._date.encode(data, encoder)
        self._time.encode(data, encoder)

    def decode(self, decoder):
        decoded_date = self._date.decode(decoder)
        decoded_time = self._time.decode(decoder)

        return datetime.datetime(
            decoded_date.year,
            decoded_date.month,
            decoded_date.day,
            decoded_time.hour,
            decoded_time.minute,
            decoded_time.second,
        )


def encode_open_type(data, encoder):
    """The encoding of the value of an open type, after its length (X.696
    30).

    """

    encoder.append_length_determinant(len(data))
    encoder.append_bytes(data)


def decode_open_type(decoder):
    length = decoder.read_length_determinant()

    return decoder.read_bytes(length)


class Any(Type):
    """An ANY, encoded as the open type that took its place, with the
    encoding of its value.

    """

    def __init__(self, name):
        super(Any, self).__init__(name, "ANY", None)

    def encode(self, data, encoder):
        encode_open_type(data, encoder)

    def decode(self, decoder):
        return decode_open_type(decoder)


class OpenType(Type):
    def __init__(self, name):
        super(OpenType, self).__init__(name, "OpenType", None)

    def encode(self, data, encoder):
        encode_open_type(data, encoder)

    def decode(self, decoder):
        return decode_open_type(decoder)


class AnyDefinedBy(Type):
    """An ANY DEFINED BY. With choices, it is an open type of the type of
    the choice of the value of the member it names, which the SEQUENCE
    or SET it is a member of gives as `values`. Without, it is an ANY.

    """

    def __init__(self, name, type_member, choices):
        super(AnyDefinedBy, self).__init__(name, "ANY DEFINED BY", None, None)
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

    def encode(self, data, encoder, values=None):
        if not self.choices:
            return encode_open_type(data, encoder)

        choice = self.choice(values)

        if choice is None:
            raise EncodeError(self.bad_choice(values))

        inner = Encoder()
        choice.encode(data, inner)
        encode_open_type(inner.as_bytearray(), encoder)

    def decode(self, decoder, values=None):
        if not self.choices:
            return decode_open_type(decoder)

        choice = self.choice(values)

        if choice is None:
            raise DecodeError(
                self.bad_choice(values), offset=decoder.number_of_read_bits()
            )

        offset = decoder.number_of_read_bits()
        data = decode_open_type(decoder)
        inner = Decoder(bytearray(data))
        decoded = choice.decode(inner)

        # It fills the length it is in.
        if inner.number_of_read_bits() != 8 * len(data):
            raise DecodeError("The value does not fill its open type.", offset=offset)

        return decoded


def resolved(member):
    """The type of given member, past a recursive reference."""

    while isinstance(member, Recursive) and member.tag is None:
        member = member.inner

    return member


def member_tags(member):
    """The tags a member can begin with."""

    member = resolved(member)

    if member.tag is None and isinstance(member, Choice):
        return member.tags()

    return [member.tag]


def outer_tag(member, value):
    """The outermost tag of given member with given value."""

    member = resolved(member)

    if member.tag is None and isinstance(member, Choice):
        tag = member.chosen_tag(value)

        if tag is None:
            raise EncodeError(
                "Expected choice {}, but got '{}'.".format(
                    member.format_names(), value[0]
                )
            )

        return tag

    return member.tag


class Recursive(compiler.Recursive, Type):
    def __init__(self, name, type_name, module_name):
        super(Recursive, self).__init__(name, "RECURSIVE", None)
        self.type_name = type_name
        self.module_name = module_name
        self.tag_number = None
        self.tag_flags = None
        self.inner: typing.Any = None

    def set_inner_type(self, inner):
        self.inner = copy(inner)

        if self.tag_number is not None:
            self.inner.set_tag(self.tag_number, self.tag_flags)

    def encode(self, data, encoder):
        self.inner.encode(data, encoder)

    def decode(self, decoder):
        return self.inner.decode(decoder)


class CompiledType(compiler.CompiledType):
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


class Compiler(compiler.Compiler):
    def process_type(self, type_name, type_descriptor, module_name):
        compiled_type = self.compile_type(type_name, type_descriptor, module_name)

        return CompiledType(compiled_type)

    def compile_type(self, name, type_descriptor, module_name):
        module_name = self.get_module_name(type_descriptor, module_name)
        type_name = type_descriptor["type"]

        compiled: Type
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
            compiled = Real(name, self.get_with_components(type_descriptor))
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
            compiled = NumericString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "PrintableString":
            compiled = PrintableString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "IA5String":
            compiled = IA5String(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "VisibleString":
            compiled = VisibleString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "GeneralString":
            compiled = GeneralString(name)
        elif type_name == "UTF8String":
            compiled = UTF8String(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "BMPString":
            compiled = BMPString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "GraphicString":
            compiled = GraphicString(name)
        elif type_name == "UTCTime":
            compiled = UTCTime(name)
        elif type_name == "UniversalString":
            compiled = UniversalString(
                name, *self.get_size_range(type_descriptor, module_name)
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
        elif type_name == "OpenType":
            compiled = OpenType(name)
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
                compiled = self.with_own_constraints(
                    compiled, type_descriptor, module_name
                )

        if "tag" in type_descriptor:
            compiled = self.copy(compiled)
            tag = type_descriptor["tag"]
            class_ = tag.get("class", None)

            if class_ == "APPLICATION":
                flags = Class.APPLICATION
            elif class_ == "PRIVATE":
                flags = Class.PRIVATE
            else:
                flags = 0

            compiled.set_tag(tag["number"], flags)

        if "restricted-to" in type_descriptor:
            compiled = self.set_compiled_restricted_to(
                compiled, type_descriptor, module_name
            )

        return compiled

    def compile_members(self, members, module_name, sort_by_tag=False):
        compiled_members: list[typing.Any] = []
        in_extension = False
        additions: list[typing.Any] | None = None

        for member in members:
            if member == EXTENSION_MARKER:
                in_extension = not in_extension

                if in_extension:
                    additions = []
            elif in_extension:
                self.compile_extension_member(member, module_name, additions)
            else:
                self.compile_root_member(member, module_name, compiled_members)

        if sort_by_tag:
            compiled_members = self.sort_members_by_tag(
                compiled_members, members, module_name
            )

        return compiled_members, additions

    def compile_extension_member(self, member, module_name, additions):
        if isinstance(member, list):
            for memb in member:
                compiled_member = self.compile_member(memb, module_name)
                additions.append(compiled_member)
        else:
            compiled_member = self.compile_member(member, module_name)
            additions.append(compiled_member)


class OERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums=False, lenient=False):
        return Compiler(specification, numeric_enums).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)


def decode_full_length(_data):
    raise DecodeError("Decode length is not supported for this codec.")
