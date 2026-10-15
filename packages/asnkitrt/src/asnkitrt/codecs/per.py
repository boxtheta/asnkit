# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Aligned Packed Encoding Rules (PER) codec."""

import binascii
import datetime
import string
from operator import itemgetter
import typing

from copy import copy
from .duration import dict_to_duration, duration_to_dict
from asnkitrt.codecs.codec import Codec

from ..constants import EXTENSION_MARKER
from . import (
    NamedNumbersMixin,
    BaseType,
    DecodeError,
    EncodeError,
    ErrorWithLocation,
    OutOfDataError,
    compiler,
    format_bytes,
    format_or,
    restricted_generalized_time_from_datetime,
    restricted_generalized_time_to_datetime,
    restricted_utc_time_from_datetime,
    restricted_utc_time_to_datetime,
)
from .ber import (
    decode_object_identifier,
    decode_real,
    encode_object_identifier,
    encode_real,
)
from .compiler import (
    clean_bit_string_value,
    enum_values_as_dict,
    enum_values_split,
    rstrip_bit_string_zeros,
)
from .permitted_alphabet import (
    IA5_STRING,
    NUMERIC_STRING,
    PRINTABLE_STRING,
    VISIBLE_STRING,
)


def is_unbound(minimum, maximum):
    return (minimum in [None, "MIN"]) or (maximum in [None, "MAX"]) or (maximum > 65535)


def is_in_size_range(size, minimum, maximum):
    """If given size is in the range of a size. A bound that is missing is
    no bound.

    """

    return (minimum in [None, "MIN"] or size >= minimum) and (
        maximum in [None, "MAX"] or size <= maximum
    )


def size_bounds(minimum, maximum):
    """The bounds of a size, of which a missing lower bound is 0 (X.691 16.4,
    17.2, 20.2, 30.3), and a missing upper bound ``None``.

    """

    return (
        0 if minimum in [None, "MIN"] else minimum,
        None if maximum in [None, "MAX"] else maximum,
    )


class Sized:
    """The length of a value of a size of more than one value, as a
    constrained whole number.

    """

    minimum: typing.Any
    maximum: typing.Any
    number_of_bits: typing.Any

    def check_size(self, size):
        """Rejects a value that is not of the size of its type, which can not
        be encoded.

        """

        if not is_in_size_range(size, self.minimum, self.maximum):
            raise EncodeError(
                "Expected a size between {} and {}, but got {}.".format(
                    self.minimum,
                    "MAX" if self.maximum is None else self.maximum,
                    size,
                )
            )

    def encode_length(self, length, encoder):
        encoder.append_constrained_whole_number(
            length, self.minimum, self.maximum, self.number_of_bits
        )

    def decode_length(self, decoder):
        return decoder.read_constrained_whole_number(
            self.minimum, self.maximum, self.number_of_bits
        )


def to_int(chars):
    if isinstance(chars, int):
        return chars

    num = 0
    byte_array = bytearray(chars)
    while len(byte_array) > 0:
        num <<= 8
        byte = byte_array.pop(0)
        num += byte

    return num


def to_byte_array(num, number_of_bits):
    byte_array = bytearray()

    while number_of_bits > 0:
        byte_array.insert(0, (num & 0xFF))
        num >>= 8
        number_of_bits -= 8

    return byte_array


def integer_as_number_of_bits(size):
    """Returns the minimum number of bits needed to fit given positive
    integer.

    """

    if size == 0:
        return 0
    else:
        return size.bit_length()


def integer_as_number_of_bits_power_of_two(size):
    """Returns the minimum power of two number of bits needed to fit given
    positive integer.

    """

    if size == 0:
        return 0
    else:
        bit_length = integer_as_number_of_bits(size)
        bit_length_pow_2 = 1

        while bit_length > bit_length_pow_2:
            bit_length_pow_2 <<= 1

        return bit_length_pow_2


def size_as_number_of_bytes(size):
    """Returns the minimum number of bytes needed to fit given positive
    integer.

    """

    if size == 0:
        return 1
    else:
        number_of_bits = size.bit_length()
        rest = number_of_bits % 8

        if rest != 0:
            number_of_bits += 8 - rest

        return number_of_bits // 8


CLASS_PRIO = {"UNIVERSAL": 0, "APPLICATION": 1, "CONTEXT_SPECIFIC": 2, "PRIVATE": 3}


class PermittedAlphabet(object):
    def __init__(self, encode_map, decode_map):
        self.encode_map = encode_map
        self.decode_map = decode_map

    def __len__(self):
        return len(self.encode_map)

    def encode(self, value):
        try:
            return self.encode_map[value]
        except KeyError:
            raise EncodeError(
                "Expected a character in '{}', but got '{}' (0x{:02x})'.".format(
                    "".join(sorted([chr(v) for v in self.encode_map])),
                    chr(value) if chr(value) in string.printable else ".",
                    value,
                )
            )

    def decode(self, value):
        try:
            return self.decode_map[value]
        except KeyError:
            raise DecodeError(
                "Expected a value in {}, but got {:d}.".format(
                    list(self.decode_map), value
                )
            )


class Encoder(object):
    def __init__(self):
        self.number_of_bits = 0
        self.value = 0
        self.chunks_number_of_bits = 0
        self.chunks = []

    def __iadd__(self, other):
        for value, number_of_bits in other.chunks:
            self.append_non_negative_binary_integer(value, number_of_bits)

        self.append_non_negative_binary_integer(other.value, other.number_of_bits)

        return self

    def reset(self):
        self.number_of_bits = 0
        self.value = 0
        self.chunks_number_of_bits = 0
        self.chunks = []

    def are_all_bits_zero(self):
        return not (any([value for value, _ in self.chunks]) or self.value)

    def number_of_bytes(self):
        return (self.chunks_number_of_bits + self.number_of_bits + 7) // 8

    def offset(self):
        return (len(self.chunks), self.number_of_bits)

    def set_bit(self, offset):
        chunk_offset, bit_offset = offset

        if len(self.chunks) == chunk_offset:
            self.value |= 1 << (self.number_of_bits - bit_offset - 1)
        else:
            chunk = self.chunks[chunk_offset]
            chunk[0] |= 1 << (chunk[1] - bit_offset - 1)

    def align(self):
        self.align_always()

    def align_always(self):
        width = 8 * self.number_of_bytes()
        width -= self.chunks_number_of_bits
        width -= self.number_of_bits
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

    def append_non_negative_binary_integer(self, value, number_of_bits):
        """Append given integer value."""

        if self.number_of_bits > 4096:
            self.chunks.append([self.value, self.number_of_bits])
            self.chunks_number_of_bits += self.number_of_bits
            self.number_of_bits = 0
            self.value = 0

        self.number_of_bits += number_of_bits
        self.value <<= number_of_bits
        self.value |= value

    def append_bytes(self, data):
        """Append given data."""

        self.append_bits(data, 8 * len(data))

    def as_bytearray(self):
        """Return the bits as a bytearray."""

        value = 0
        number_of_bits = 0

        for chunk_value, chunk_number_of_bits in self.chunks:
            value <<= chunk_number_of_bits
            value |= chunk_value
            number_of_bits += chunk_number_of_bits

        value <<= self.number_of_bits
        value |= self.value
        number_of_bits += self.number_of_bits

        if number_of_bits == 0:
            return bytearray()

        number_of_alignment_bits = 8 - (number_of_bits % 8)

        if number_of_alignment_bits != 8:
            value <<= number_of_alignment_bits
            number_of_bits += number_of_alignment_bits

        value |= 0x80 << number_of_bits

        return bytearray(binascii.unhexlify(hex(value)[4:].rstrip("L")))

    def append_length_determinant(self, length):
        encoded: bytes | bytearray

        if length < 128:
            encoded = bytearray([length])
        elif length < 16384:
            encoded = bytearray([(0x80 | (length >> 8)), (length & 0xFF)])
        elif length < 32768:
            encoded = b"\xc1"
            length = 16384
        elif length < 49152:
            encoded = b"\xc2"
            length = 32768
        elif length < 65536:
            encoded = b"\xc3"
            length = 49152
        else:
            encoded = b"\xc4"
            length = 65536

        self.append_bytes(encoded)

        return length

    def append_length_determinant_chunks(self, length):
        offset = 0
        chunk_length = length

        while True:
            chunk_length = self.append_length_determinant(chunk_length)

            yield offset, chunk_length

            if chunk_length < 16384:
                break

            offset += chunk_length
            chunk_length = length - offset

    def append_normally_small_non_negative_whole_number(self, value):
        if value < 64:
            self.append_non_negative_binary_integer(value, 7)
        else:
            self.append_bit(1)
            length = (value.bit_length() + 7) // 8
            self.append_length_determinant(length)
            self.append_non_negative_binary_integer(value, 8 * length)

    def append_normally_small_length(self, value):
        if value <= 64:
            self.append_non_negative_binary_integer(value - 1, 7)
        elif value <= 127:
            self.append_non_negative_binary_integer(0x100 | value, 9)
        else:
            raise NotImplementedError(
                "Normally small length number >127 is not yet supported."
            )

    def append_constrained_whole_number(self, value, minimum, maximum, number_of_bits):
        _range = maximum - minimum + 1
        value -= minimum

        if _range <= 255:
            self.append_non_negative_binary_integer(value, number_of_bits)
        elif _range == 256:
            self.align_always()
            self.append_non_negative_binary_integer(value, 8)
        elif _range <= 65536:
            self.align_always()
            self.append_non_negative_binary_integer(value, 16)
        else:
            self.align_always()
            self.append_non_negative_binary_integer(value, number_of_bits)

    def append_semi_constrained_whole_number(self, value):
        """A whole number that is not negative, in as few octets as it can be
        and at least one, after its length (X.691 11.7).

        """

        number_of_bytes = max((value.bit_length() + 7) // 8, 1)
        self.append_length_determinant(number_of_bytes)
        self.append_non_negative_binary_integer(value, 8 * number_of_bytes)

    def append_unconstrained_whole_number(self, value):
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

    def __repr__(self):
        return format_bytes(self.as_bytearray())


class Decoder(object):
    def __init__(self, encoded):
        self.number_of_bits = 8 * len(encoded)
        self.total_number_of_bits = self.number_of_bits

        if len(encoded) > 0:
            value = int(binascii.hexlify(encoded), 16)
            value |= 0x80 << self.number_of_bits
            self.value = bin(value)[10:]
        else:
            self.value = ""

    def align(self):
        self.align_always()

    def align_always(self):
        width = self.number_of_bits & 0x7
        self.number_of_bits -= width

    def number_of_read_bits(self):
        return self.total_number_of_bits - self.number_of_bits

    def skip_bits(self, number_of_bits):
        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        self.number_of_bits -= number_of_bits

    def read_bit(self):
        """Read a bit."""

        if self.number_of_bits == 0:
            raise OutOfDataError(self.number_of_read_bits())

        bit = int(self.value[self.number_of_read_bits()])
        self.number_of_bits -= 1

        return bit

    def read_bits(self, number_of_bits):
        """Read given number of bits."""

        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        offset = self.number_of_read_bits()
        value = self.value[offset : offset + number_of_bits]
        self.number_of_bits -= number_of_bits
        value = "10000000" + value
        number_of_alignment_bits = 8 - (number_of_bits % 8)

        if number_of_alignment_bits != 8:
            value += "0" * number_of_alignment_bits

        return binascii.unhexlify(hex(int(value, 2))[4:].rstrip("L"))

    def read_bytes(self, number_of_bytes):
        return self.read_bits(8 * number_of_bytes)

    def read_non_negative_binary_integer(self, number_of_bits):
        """Read an integer value of given number of bits."""

        if number_of_bits > self.number_of_bits:
            raise OutOfDataError(self.number_of_read_bits())

        if number_of_bits == 0:
            return 0

        offset = self.number_of_read_bits()
        value = self.value[offset : offset + number_of_bits]
        self.number_of_bits -= number_of_bits

        return int(value, 2)

    def read_length_determinant(self):
        value = self.read_non_negative_binary_integer(8)

        if (value & 0x80) == 0x00:
            return value
        elif (value & 0xC0) == 0x80:
            return ((value & 0x7F) << 8) | (self.read_non_negative_binary_integer(8))
        else:
            try:
                return {0xC1: 16384, 0xC2: 32768, 0xC3: 49152, 0xC4: 65536}[value]
            except KeyError:
                raise DecodeError(
                    "Bad length determinant fragmentation value 0x{:02x}.".format(value)
                )

    def read_length_determinant_chunks(self):
        while True:
            length = self.read_length_determinant()

            yield length

            if length < 16384:
                break

    def read_normally_small_non_negative_whole_number(self):
        if not self.read_bit():
            decoded = self.read_non_negative_binary_integer(6)
        else:
            length = self.read_length_determinant()
            decoded = self.read_non_negative_binary_integer(8 * length)

        return decoded

    def read_normally_small_length(self):
        if not self.read_bit():
            return self.read_non_negative_binary_integer(6) + 1
        elif not self.read_bit():
            return self.read_non_negative_binary_integer(7)
        else:
            raise NotImplementedError(
                "Normally small length number >64 is not yet supported."
            )

    def read_constrained_whole_number(self, minimum, maximum, number_of_bits):
        _range = maximum - minimum + 1

        if _range <= 255:
            value = self.read_non_negative_binary_integer(number_of_bits)
        elif _range == 256:
            self.align_always()
            value = self.read_non_negative_binary_integer(8)
        elif _range <= 65536:
            self.align_always()
            value = self.read_non_negative_binary_integer(16)
        else:
            self.align_always()
            value = self.read_non_negative_binary_integer(number_of_bits)

        return value + minimum

    def read_semi_constrained_whole_number(self):
        length = self.read_length_determinant()

        return self.read_non_negative_binary_integer(8 * length)

    def read_unconstrained_whole_number(self):
        length = self.read_length_determinant()
        decoded = self.read_non_negative_binary_integer(8 * length)
        number_of_bits = 8 * length

        if decoded & (1 << (number_of_bits - 1)):
            mask = (1 << number_of_bits) - 1
            decoded = decoded - mask
            decoded -= 1

        return decoded


class Type(BaseType):
    def __init__(self, name, type_name):
        super().__init__(name, type_name)
        self.module_name = None
        self.tag = None

    def set_size_range(self, minimum, maximum, has_extension_marker):
        pass

    def set_restricted_to_range(self, minimum, maximum, has_extension_marker):
        pass


class Characters:
    """The characters a string of a known multiplier can have: those of its
    kind, or those of its permitted alphabet, as code points in the
    canonical order (X.680 43).

    """

    def __init__(self, codes):
        self.codes = sorted(set(codes))
        self.positions = {code: index for index, code in enumerate(self.codes)}

    def __len__(self):
        return len(self.codes)

    def __contains__(self, code):
        return code in self.positions

    def greatest(self):
        return self.codes[-1] if self.codes else 0

    def index(self, code):
        return self.positions[code]

    def at(self, index):
        return self.codes[index] if index < len(self.codes) else None


class WideCharacters:
    """The characters of BMPString and UniversalString, which are too many
    to be listed. A character of them is always its code.

    """

    def __init__(self, count, greatest, excluded=range(0)):
        self.count = count
        self.greatest_code = greatest
        self.excluded = excluded

    def __len__(self):
        return self.count

    def __contains__(self, code):
        return 0 <= code <= self.greatest_code and code not in self.excluded

    def greatest(self):
        return self.greatest_code

    def index(self, code):
        raise EncodeError("A character of this kind is not counted.")

    def at(self, index):
        return None


class KnownMultiplierStringType(Sized, Type):
    """A string of a known multiplier (X.691 30.5): NumericString,
    PrintableString, VisibleString, IA5String, BMPString and
    UniversalString.

    """

    ALIGNED = True
    CHARACTERS: typing.Any = Characters([])

    def __init__(
        self,
        name,
        minimum=None,
        maximum=None,
        has_extension_marker=False,
        permitted_alphabet=None,
    ):
        super(KnownMultiplierStringType, self).__init__(name, self.__class__.__name__)
        self.set_size_range(minimum, maximum, has_extension_marker)

        self.set_permitted_alphabet(permitted_alphabet)

    def set_permitted_alphabet(self, permitted_alphabet):
        """Sets the permitted alphabet, as code points, or that of the kind of
        string if ``None``.

        """

        if permitted_alphabet is None:
            self.characters = self.CHARACTERS
        else:
            self.characters = Characters(permitted_alphabet)

        # B bits are enough for the characters, and B2 is the power of two
        # that is the least not less than B (X.691 30.5.2).
        largest = max(len(self.characters) - 1, 0)

        if self.ALIGNED:
            self.bits_per_character = integer_as_number_of_bits_power_of_two(largest)
        else:
            self.bits_per_character = integer_as_number_of_bits(largest)

        # A character is its place in the canonical order, and not its code,
        # when the greatest code does not fit in the bits of a character
        # (X.691 30.5.4).
        self.by_index = self.bits_per_character < 32 and (
            self.characters.greatest() > 2**self.bits_per_character - 1
        )

    def set_size_range(self, minimum, maximum, has_extension_marker):
        # A missing lower bound is 0 (X.691 30.3).
        self.minimum = 0 if minimum in [None, "MIN"] else minimum
        self.maximum = None if maximum in [None, "MAX"] else maximum
        self.has_extension_marker = has_extension_marker

        if self.maximum is None or self.maximum > 65535:
            self.number_of_bits = None
        else:
            self.number_of_bits = integer_as_number_of_bits(self.maximum - self.minimum)

    def encode_character(self, character):
        code = ord(character)

        if code not in self.characters:
            raise EncodeError(
                "Expected a character in the permitted alphabet, but got '{}' "
                "(0x{:02x}).".format(
                    character if character in string.printable else ".", code
                )
            )

        return self.characters.index(code) if self.by_index else code

    def decode_character(self, value):
        code = self.characters.at(value) if self.by_index else value

        if code is None or code not in self.characters:
            raise DecodeError(
                "Expected a value in the permitted alphabet, but got {:d}.".format(
                    value
                )
            )

        return chr(code)

    def encode(self, data, encoder):
        values = [self.encode_character(character) for character in data]
        unbound = self.number_of_bits is None

        if self.has_extension_marker:
            # Outside the root of an extensible size, as if there was no size
            # constraint, with the permitted alphabet still (X.691 30.4).
            if is_in_size_range(len(data), self.minimum, self.maximum):
                encoder.append_bit(0)
            else:
                encoder.append_bit(1)
                unbound = True
        else:
            self.check_size(len(data))

        if unbound:
            return self.encode_unbound(values, encoder)
        elif self.minimum != self.maximum:
            self.encode_length(len(data), encoder)

            # The characters are octet-aligned when the upper bound times the
            # bits of a character is 16 or more, and there are some (X.691
            # 30.5.7, 11.9.3.3).
            if self.maximum * self.bits_per_character >= 16 and len(data) > 0:
                encoder.align()
        elif self.maximum * self.bits_per_character > 16:
            encoder.align()

        for value in values:
            encoder.append_non_negative_binary_integer(value, self.bits_per_character)

    def encode_unbound(self, values, encoder):
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(len(values)):
            for value in values[offset : offset + length]:
                encoder.append_non_negative_binary_integer(
                    value, self.bits_per_character
                )

    def decode(self, decoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker and decoder.read_bit():
            unbound = True

        if unbound:
            return self.decode_unbound(decoder)
        elif self.minimum != self.maximum:
            length = self.decode_length(decoder)

            if self.maximum * self.bits_per_character >= 16 and length > 0:
                decoder.align()
        else:
            length = self.minimum

            if self.maximum * self.bits_per_character > 16:
                decoder.align()

        return "".join(
            self.decode_character(
                decoder.read_non_negative_binary_integer(self.bits_per_character)
            )
            for _ in range(length)
        )

    def decode_unbound(self, decoder):
        decoder.align()
        characters = []

        for length in decoder.read_length_determinant_chunks():
            for _ in range(length):
                characters.append(
                    self.decode_character(
                        decoder.read_non_negative_binary_integer(
                            self.bits_per_character
                        )
                    )
                )

        return "".join(characters)


class StringType(Type):
    ENCODING: str
    LENGTH_MULTIPLIER = 1

    def __init__(self, name):
        super(StringType, self).__init__(name, self.__class__.__name__)

    def encode(self, data, encoder):
        encoded = data.encode(self.ENCODING)
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(len(data)):
            offset *= self.LENGTH_MULTIPLIER
            data = encoded[offset : offset + self.LENGTH_MULTIPLIER * length]
            encoder.append_bytes(data)

    def decode(self, decoder):
        decoder.align()
        encoded = []

        for length in decoder.read_length_determinant_chunks():
            encoded.append(decoder.read_bytes(self.LENGTH_MULTIPLIER * length))

        return b"".join(encoded).decode(self.ENCODING)


class MembersType(Type):
    def __init__(self, name, root_members, additions, type_name):
        super(MembersType, self).__init__(name, type_name)
        self.root_members = root_members
        self.additions = additions
        self.optionals = [
            member
            for member in root_members
            if member.optional or member.default is not None
        ]

    def encode(self, data, encoder):
        if self.additions is not None:
            offset = encoder.offset()
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

        for member in self.root_members:
            self.encode_member(member, data, encoder)

    def encode_additions(self, data, encoder):
        # Encode extension additions.
        presence_bits = 0
        addition_encoders = []
        number_of_precence_bits = 0

        try:
            for addition in self.additions:
                presence_bits <<= 1
                addition_encoder = encoder.__class__()
                number_of_precence_bits += 1

                if isinstance(addition, AdditionGroup):
                    addition.encode_addition_group(data, addition_encoder)
                else:
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

        # Presence bit field.
        number_of_additions = len(self.additions)
        presence_bits <<= number_of_additions - number_of_precence_bits
        encoder.append_normally_small_length(number_of_additions)
        encoder.append_non_negative_binary_integer(presence_bits, number_of_additions)

        # Each extension addition in an open type.
        for addition_encoder in addition_encoders:
            put_open_type(addition_encoder, encoder)

        return True

    def encode_addition_group(self, data, encoder):
        self.encode_root(data, encoder)

        if encoder.are_all_bits_zero() and (
            encoder.number_of_bits == len(self.optionals)
        ):
            encoder.reset()

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
        elif member.optional or member.default is not None:
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
            else:
                decoded = self.decode_root(decoder)
        else:
            decoded = self.decode_root(decoder)

        return decoded

    def decode_root(self, decoder, before=None):
        """The members of the root, after the members `before` of the
        same SEQUENCE or SET, which an ANY DEFINED BY may be defined by.

        """

        values: dict[str, typing.Any] = {}
        optionals = {optional: decoder.read_bit() for optional in self.optionals}

        for member in self.root_members:
            if optionals.get(member, True):
                try:
                    if isinstance(member, AnyDefinedBy):
                        value = member.decode(decoder, {**(before or {}), **values})
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
        length = decoder.read_normally_small_length()
        presence_bits = decoder.read_non_negative_binary_integer(length)
        decoder.align()
        decoded: dict[str, typing.Any] = {}

        for i in range(length):
            if presence_bits & (1 << (length - i - 1)):
                if i < len(self.additions):
                    addition = self.additions[i]
                    values = {**before, **decoded}

                    try:
                        if isinstance(addition, AdditionGroup):
                            decoded.update(
                                get_open_type(decoder, addition.decode_root, values)
                            )
                        elif isinstance(addition, AnyDefinedBy):
                            decoded[addition.name] = get_open_type(
                                decoder, addition.decode, values
                            )
                        else:
                            decoded[addition.name] = get_open_type(
                                decoder, addition.decode
                            )
                    except ErrorWithLocation as e:
                        # Add member location
                        e.add_location(addition)
                        raise e
                else:
                    # One of a later version.
                    decode_open_type(decoder)

        return decoded

    def __repr__(self):
        return "{}({}, [{}])".format(
            self.__class__.__name__,
            self.name,
            ", ".join([repr(member) for member in self.root_members]),
        )


class ArrayType(Sized, Type):
    def __init__(
        self, name, element_type, minimum, maximum, has_extension_marker, type_name
    ):
        super(ArrayType, self).__init__(name, type_name)
        self.element_type = element_type
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.minimum, self.maximum = size_bounds(minimum, maximum)
        self.has_extension_marker = has_extension_marker

        if self.maximum is None or self.maximum > 65535:
            self.number_of_bits = None
        else:
            self.number_of_bits = integer_as_number_of_bits(self.maximum - self.minimum)

    def encode(self, data, encoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker:
            # Outside the root, as if there was no size constraint (X.691
            # 20.4).
            if is_in_size_range(len(data), self.minimum, self.maximum):
                encoder.append_bit(0)
            else:
                encoder.append_bit(1)
                unbound = True
        else:
            self.check_size(len(data))

        if unbound:
            return self.encode_unbound(data, encoder)
        elif self.minimum != self.maximum:
            self.encode_length(len(data), encoder)

        for entry in data:
            self.element_type.encode(entry, encoder)

    def encode_unbound(self, data, encoder):
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(len(data)):
            for entry in data[offset : offset + length]:
                self.element_type.encode(entry, encoder)

    def decode(self, decoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker and decoder.read_bit():
            unbound = True

        if unbound:
            return self.decode_unbound(decoder)
        elif self.minimum != self.maximum:
            length = self.decode_length(decoder)
        else:
            length = self.minimum

        return [self.element_type.decode(decoder) for _ in range(length)]

    def decode_unbound(self, decoder):
        decoder.align()
        decoded = []

        for length in decoder.read_length_determinant_chunks():
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
        super(Boolean, self).__init__(name, "BOOLEAN")

    def encode(self, data, encoder):
        encoder.append_bit(bool(data))

    def decode(self, decoder):
        return bool(decoder.read_bit())


class Integer(NamedNumbersMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name, "INTEGER")
        self.minimum = None
        self.maximum = None
        self.has_extension_marker = False
        self.number_of_bits = None
        self.number_of_indefinite_bits = None

    def set_restricted_to_range(self, minimum, maximum, has_extension_marker):
        self.has_extension_marker = has_extension_marker
        self.minimum = None if minimum in [None, "MIN"] else minimum
        self.maximum = None if maximum in [None, "MAX"] else maximum
        self.number_of_bits = None
        self.number_of_indefinite_bits = None

        if self.minimum is None or self.maximum is None:
            return

        size = self.maximum - self.minimum
        self.number_of_bits = integer_as_number_of_bits(size)

        if size > 65535:
            number_of_bits = ((self.number_of_bits + 7) // 8 - 1).bit_length()
            self.number_of_indefinite_bits = number_of_bits

    def is_in_range(self, data):
        return (self.minimum is None or data >= self.minimum) and (
            self.maximum is None or data <= self.maximum
        )

    def encode(self, data, encoder):
        data = self.to_number(data)
        inside = self.is_in_range(data)

        if self.has_extension_marker:
            encoder.append_bit(0 if inside else 1)

            # Outside the root, as if there was no constraint (X.691 13.1).
            if not inside:
                encoder.align()
                encoder.append_unconstrained_whole_number(data)

                return
        elif not inside:
            raise EncodeError(
                "Expected an integer between {} and {}, but got {}.".format(
                    "MIN" if self.minimum is None else self.minimum,
                    "MAX" if self.maximum is None else self.maximum,
                    data,
                )
            )

        if self.number_of_bits is None:
            encoder.align()

            if self.minimum is None:
                # An unconstrained whole number (X.691 13.2.4, 11.8).
                encoder.append_unconstrained_whole_number(data)
            else:
                # A semi-constrained whole number (X.691 13.2.3, 11.7).
                encoder.append_semi_constrained_whole_number(data - self.minimum)

            return

        self.encode_constrained(data, encoder)

    def encode_constrained(self, data, encoder):
        if self.number_of_indefinite_bits is None:
            number_of_bits = self.number_of_bits
        else:
            number_of_bytes = size_as_number_of_bytes(data - self.minimum)
            number_of_bits = 8 * number_of_bytes
            encoder.append_constrained_whole_number(
                number_of_bytes - 1,
                0,
                2**self.number_of_indefinite_bits,
                self.number_of_indefinite_bits,
            )
            encoder.align()

        encoder.append_constrained_whole_number(
            data, self.minimum, self.maximum, number_of_bits
        )

    def decode(self, decoder):
        if self.has_extension_marker:
            if decoder.read_bit():
                decoder.align()

                return decoder.read_unconstrained_whole_number()

        if self.number_of_bits is None:
            decoder.align()

            if self.minimum is None:
                return decoder.read_unconstrained_whole_number()
            else:
                return self.minimum + decoder.read_semi_constrained_whole_number()

        return self.decode_constrained(decoder)

    def decode_constrained(self, decoder):
        if self.number_of_indefinite_bits is None:
            number_of_bits = self.number_of_bits
        else:
            number_of_bytes = decoder.read_constrained_whole_number(
                0, 2**self.number_of_indefinite_bits, self.number_of_indefinite_bits
            )
            number_of_bytes += 1
            number_of_bits = 8 * number_of_bytes
            decoder.align()

        return decoder.read_constrained_whole_number(
            self.minimum, self.maximum, number_of_bits
        )

    def __repr__(self):
        return "Integer({})".format(self.name)


class Real(Type):
    def __init__(self, name):
        super(Real, self).__init__(name, "REAL")

    def encode(self, data, encoder):
        encoded = encode_real(data)
        encoder.align()
        encoder.append_length_determinant(len(encoded))
        encoder.append_bytes(encoded)

    def decode(self, decoder):
        decoder.align()
        length = decoder.read_length_determinant()

        # The contents of CER and DER (X.691 15.2).
        return decode_real(bytearray(decoder.read_bytes(length)), strict=True)


class Null(Type):
    def __init__(self, name):
        super(Null, self).__init__(name, "NULL")

    def encode(self, _, _encoder):
        pass

    def decode(self, _):
        return None


class BitString(Sized, Type):
    def __init__(self, name, named_bits, minimum, maximum, has_extension_marker):
        super(BitString, self).__init__(name, "BIT STRING")
        self.named_bits = named_bits
        self.has_named_bits = named_bits is not None
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.minimum, self.maximum = size_bounds(minimum, maximum)
        self.has_extension_marker = has_extension_marker

        if self.maximum is None or self.maximum > 65535:
            self.number_of_bits = None
        else:
            self.number_of_bits = integer_as_number_of_bits(self.maximum - self.minimum)

    def is_default(self, value):
        clean_value = clean_bit_string_value(value, self.has_named_bits)
        clean_default = clean_bit_string_value(self.default, self.has_named_bits)

        return clean_value == clean_default

    def rstrip_zeros(self, data, number_of_bits):
        # Trailing zero bits are left out, but for the minimum (X.691 16.2,
        # 16.3).
        data, number_of_bits = rstrip_bit_string_zeros(bytearray(data))

        if number_of_bits < self.minimum:
            number_of_bits = self.minimum
            number_of_bytes = (number_of_bits + 7) // 8
            data += (number_of_bytes - len(data)) * b"\x00"

        return (data, number_of_bits)

    def encode(self, data, encoder):
        data, number_of_bits = data

        if self.has_named_bits:
            data, number_of_bits = self.rstrip_zeros(data, number_of_bits)

        unbound = self.number_of_bits is None

        if self.has_extension_marker:
            # Outside the root, as if there was no size constraint (X.691
            # 16.6).
            if is_in_size_range(number_of_bits, self.minimum, self.maximum):
                encoder.append_bit(0)
            else:
                encoder.append_bit(1)
                unbound = True
        else:
            self.check_size(number_of_bits)

        if unbound:
            return self.encode_unbound(data, number_of_bits, encoder)
        elif self.minimum != self.maximum:
            self.encode_length(number_of_bits, encoder)

            # No padding for nothing (X.691 11.9.3.3).
            if number_of_bits > 0:
                encoder.align()
        elif self.minimum > 16:
            encoder.align()

        encoder.append_bits(data, number_of_bits)

    def encode_unbound(self, data, number_of_bits, encoder):
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(number_of_bits):
            encoder.append_bits(data[offset // 8 : (offset + length + 7) // 8], length)

    def decode(self, decoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker and decoder.read_bit():
            unbound = True

        if unbound:
            return self.decode_unbound(decoder)

        number_of_bits = self.minimum

        if self.minimum != self.maximum:
            number_of_bits = self.decode_length(decoder)

            if number_of_bits > 0:
                decoder.align()
        elif self.minimum > 16:
            decoder.align()

        value = decoder.read_bits(number_of_bits)

        return (value, number_of_bits)

    def decode_unbound(self, decoder):
        decoder.align()
        decoded = []
        number_of_bits = 0

        for length in decoder.read_length_determinant_chunks():
            decoded.append(decoder.read_bits(length))
            number_of_bits += length

        return (b"".join(decoded), number_of_bits)


class OctetString(Sized, Type):
    def __init__(self, name, minimum, maximum, has_extension_marker):
        super(OctetString, self).__init__(name, "OCTET STRING")
        self.set_size_range(minimum, maximum, has_extension_marker)

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.minimum, self.maximum = size_bounds(minimum, maximum)
        self.has_extension_marker = has_extension_marker

        if self.maximum is None or self.maximum > 65535:
            self.number_of_bits = None
        else:
            self.number_of_bits = integer_as_number_of_bits(self.maximum - self.minimum)

    def encode(self, data, encoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker:
            # Outside the root, as if there was no size constraint (X.691
            # 17.3).
            if is_in_size_range(len(data), self.minimum, self.maximum):
                encoder.append_bit(0)
            else:
                encoder.append_bit(1)
                unbound = True
        else:
            self.check_size(len(data))

        if unbound:
            return self.encode_unbound(data, encoder)

        align = True

        if self.minimum != self.maximum:
            self.encode_length(len(data), encoder)
            # No padding for nothing (X.691 11.9.3.3).
            align = len(data) > 0
        elif self.maximum <= 2:
            align = False

        if align:
            encoder.align()

        encoder.append_bytes(data)

    def encode_unbound(self, data, encoder):
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(len(data)):
            encoder.align()
            encoder.append_bytes(data[offset : offset + length])

    def decode(self, decoder):
        unbound = self.number_of_bits is None

        if self.has_extension_marker and decoder.read_bit():
            unbound = True

        if unbound:
            return self.decode_unbound(decoder)

        length = self.minimum
        align = True

        if self.minimum != self.maximum:
            length = self.decode_length(decoder)
            align = length > 0
        elif self.maximum <= 2:
            align = False

        if align:
            decoder.align()

        return decoder.read_bytes(length)

    def decode_unbound(self, decoder):
        decoder.align()
        decoded = []

        for length in decoder.read_length_determinant_chunks():
            decoder.align()
            decoded.append(decoder.read_bytes(length))

        return b"".join(decoded)


class ObjectIdentifier(Type):
    def __init__(self, name):
        super(ObjectIdentifier, self).__init__(name, "OBJECT IDENTIFIER")

    def encode(self, data, encoder):
        encoded_subidentifiers = encode_object_identifier(data)
        encoder.align()
        encoder.append_length_determinant(len(encoded_subidentifiers))
        encoder.append_bytes(bytearray(encoded_subidentifiers))

    def decode(self, decoder):
        decoder.align()
        length = decoder.read_length_determinant()
        data = decoder.read_bytes(length)

        return decode_object_identifier(bytearray(data), 0, len(data))


class Enumerated(Type):
    def __init__(self, name, values, numeric):
        super(Enumerated, self).__init__(name, "ENUMERATED")
        root, additions = enum_values_split(values)
        root = sorted(root, key=itemgetter(1))

        # Root.
        index_to_data, data_to_index = self.create_maps(root, numeric)
        self.root_index_to_data = index_to_data
        self.root_data_to_index = data_to_index
        self.root_number_of_bits = integer_as_number_of_bits(len(index_to_data) - 1)

        if numeric:
            self.root_data_to_value = {k: k for k in enum_values_as_dict(root)}
        else:
            self.root_data_to_value = {
                v: k for k, v in enum_values_as_dict(root).items()
            }

        # Optional additions.
        if additions is None:
            index_to_data = None
            data_to_index = None
        else:
            index_to_data, data_to_index = self.create_maps(additions, numeric)

        self.additions_index_to_data = index_to_data
        self.additions_data_to_index = data_to_index

    def create_maps(self, items, numeric):
        if numeric:
            index_to_data = {index: value[1] for index, value in enumerate(items)}
        else:
            index_to_data = {index: value[0] for index, value in enumerate(items)}

        data_to_index = {data: index for index, data in index_to_data.items()}

        return index_to_data, data_to_index

    def format_root_indexes(self):
        return format_or(sorted(list(self.root_index_to_data)))

    def encode(self, data, encoder):
        if self.additions_index_to_data is None:
            index = self.root_data_to_index[data]
            encoder.append_non_negative_binary_integer(index, self.root_number_of_bits)
        else:
            if data in self.root_data_to_index:
                encoder.append_bit(0)
                index = self.root_data_to_index[data]
                encoder.append_non_negative_binary_integer(
                    index, self.root_number_of_bits
                )
            else:
                encoder.append_bit(1)
                index = self.additions_data_to_index[data]
                encoder.append_normally_small_non_negative_whole_number(index)

    def decode(self, decoder):
        if self.additions_index_to_data is None:
            return self.decode_root(decoder)
        else:
            additions = decoder.read_bit()

            if additions == 0:
                return self.decode_root(decoder)
            else:
                index = decoder.read_normally_small_non_negative_whole_number()

                if index in self.additions_index_to_data:
                    return self.additions_index_to_data[index]
                else:
                    return None

    def decode_root(self, decoder):
        index = decoder.read_non_negative_binary_integer(self.root_number_of_bits)

        try:
            data = self.root_index_to_data[index]
        except KeyError:
            raise DecodeError(
                "Expected enumeration index {}, but got {}.".format(
                    self.format_root_indexes(), index
                )
            )

        return data


class Sequence(MembersType):
    def __init__(self, name, root_members, additions):
        super(Sequence, self).__init__(name, root_members, additions, "SEQUENCE")


class SequenceOf(ArrayType):
    def __init__(self, name, element_type, minimum, maximum, has_extension_marker):
        super(SequenceOf, self).__init__(
            name, element_type, minimum, maximum, has_extension_marker, "SEQUENCE OF"
        )


class Set(MembersType):
    def __init__(self, name, root_members, additions):
        super(Set, self).__init__(name, root_members, additions, "SET")


class SetOf(ArrayType):
    def __init__(self, name, element_type, minimum, maximum, has_extension_marker):
        super(SetOf, self).__init__(
            name, element_type, minimum, maximum, has_extension_marker, "SET OF"
        )


class Choice(Type):
    def __init__(self, name, root_members, additions):
        super(Choice, self).__init__(name, "CHOICE")

        # Root.
        index_to_member, name_to_index = self.create_maps(root_members)
        self.root_index_to_member = index_to_member
        self.root_name_to_index = name_to_index
        self.maximum = len(root_members) - 1
        self.root_number_of_bits = integer_as_number_of_bits(self.maximum)

        if self.maximum <= 65535:
            self.number_of_indefinite_bits = None
        else:
            number_of_bits = ((self.root_number_of_bits + 7) // 8 - 1).bit_length()
            self.number_of_indefinite_bits = number_of_bits

        # Optional additions.
        if additions is None:
            index_to_member = None
            name_to_index = None
        else:
            index_to_member, name_to_index = self.create_maps(additions)

        self.additions_index_to_member = index_to_member
        self.additions_name_to_index = name_to_index

    def create_maps(self, members):
        index_to_member = {index: member for index, member in enumerate(members)}
        name_to_index = {member.name: index for index, member in enumerate(members)}

        return index_to_member, name_to_index

    def format_root_indexes(self):
        return format_or(sorted(list(self.root_index_to_member)))

    def format_names(self):
        members = list(self.root_index_to_member.values())

        if self.additions_index_to_member is not None:
            members += list(self.additions_index_to_member.values())

        return format_or(sorted([member.name for member in members]))

    def encode(self, data, encoder):
        if self.additions_index_to_member is not None:
            if data[0] in self.root_name_to_index:
                encoder.append_bit(0)
                self.encode_root(data, encoder)
            else:
                encoder.append_bit(1)
                self.encode_additions(data, encoder)
        else:
            self.encode_root(data, encoder)

    def encode_root(self, data, encoder):
        try:
            index = self.root_name_to_index[data[0]]
        except KeyError:
            raise EncodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), data[0])
            )

        if len(self.root_index_to_member) > 1:
            self.encode_root_index(index, encoder)

        member = self.root_index_to_member[index]
        try:
            member.encode(data[1], encoder)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def encode_root_index(self, index, encoder):
        if self.number_of_indefinite_bits is None:
            number_of_bits = self.root_number_of_bits
        else:
            number_of_bytes = size_as_number_of_bytes(index)
            number_of_bits = 8 * number_of_bytes
            encoder.append_constrained_whole_number(
                number_of_bytes - 1,
                0,
                2**self.number_of_indefinite_bits,
                self.number_of_indefinite_bits,
            )
            encoder.align()

        encoder.append_constrained_whole_number(index, 0, self.maximum, number_of_bits)

    def encode_additions(self, data, encoder):
        try:
            index = self.additions_name_to_index[data[0]]
        except KeyError:
            raise EncodeError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), data[0])
            )

        addition_encoder = encoder.__class__()
        addition = self.additions_index_to_member[index]
        try:
            addition.encode(data[1], addition_encoder)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(addition)
            raise e

        # The extension addition in an open type.
        encoder.append_normally_small_non_negative_whole_number(index)
        put_open_type(addition_encoder, encoder)

    def decode(self, decoder):
        if self.additions_index_to_member is not None:
            if decoder.read_bit():
                return self.decode_additions(decoder)
            else:
                return self.decode_root(decoder)
        else:
            return self.decode_root(decoder)

    def decode_root(self, decoder):
        if len(self.root_index_to_member) > 1:
            index = self.decode_root_index(decoder)
        else:
            index = 0

        try:
            member = self.root_index_to_member[index]
        except KeyError:
            raise DecodeError(
                "Expected choice index {}, but got {}.".format(
                    self.format_root_indexes(), index
                )
            )
        try:
            return (member.name, member.decode(decoder))
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

    def decode_root_index(self, decoder):
        if self.number_of_indefinite_bits is None:
            number_of_bits = self.root_number_of_bits
        else:
            number_of_bytes = decoder.read_constrained_whole_number(
                0, 2**self.number_of_indefinite_bits, self.number_of_indefinite_bits
            )
            number_of_bytes += 1
            number_of_bits = 8 * number_of_bytes
            decoder.align()

        return decoder.read_constrained_whole_number(0, self.maximum, number_of_bits)

    def decode_additions(self, decoder):
        index = decoder.read_normally_small_non_negative_whole_number()

        if index in self.additions_index_to_member:
            addition = self.additions_index_to_member[index]
        else:
            addition = None

        if addition is None:
            # One of a later version.
            decode_open_type(decoder)

            return (None, None)

        try:
            decoded = get_open_type(decoder, addition.decode)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(addition)
            raise e

        return (addition.name, decoded)

    def __repr__(self):
        return "Choice({}, [{}])".format(
            self.name, ", ".join([repr(member) for member in self.root_name_to_index])
        )


class UTF8String(Type):
    def __init__(self, name):
        super(UTF8String, self).__init__(name, "UTF8String")

    def encode(self, data, encoder):
        encoded = data.encode("utf-8")
        encoder.align()

        for offset, length in encoder.append_length_determinant_chunks(len(encoded)):
            encoder.append_bytes(encoded[offset : offset + length])

    def decode(self, decoder):
        decoder.align()
        encoded = []

        for length in decoder.read_length_determinant_chunks():
            encoded.append(decoder.read_bytes(length))

        return b"".join(encoded).decode("utf-8")


class NumericString(KnownMultiplierStringType):
    CHARACTERS = Characters(map(ord, NUMERIC_STRING))


class PrintableString(KnownMultiplierStringType):
    CHARACTERS = Characters(map(ord, PRINTABLE_STRING))


class IA5String(KnownMultiplierStringType):
    CHARACTERS = Characters(map(ord, IA5_STRING))


class BMPString(KnownMultiplierStringType):
    CHARACTERS = WideCharacters(63488, 0xFFFF, range(0xD800, 0xE000))


class VisibleString(KnownMultiplierStringType):
    CHARACTERS = Characters(map(ord, VISIBLE_STRING))


class UniversalString(KnownMultiplierStringType):
    CHARACTERS = WideCharacters(2**32, 2**32 - 1)


class GeneralString(StringType):
    ENCODING = "latin-1"


class GraphicString(StringType):
    ENCODING = "latin-1"


class TeletexString(StringType):
    ENCODING = "iso-8859-1"


class VideotexString(StringType):
    ENCODING = "iso-8859-1"


class ObjectDescriptor(GraphicString):
    pass


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


class Date(Type):
    def __init__(self, name):
        super(Date, self).__init__(name, "DATE")
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

    def encode(self, data, encoder):
        if 2005 <= data.year <= 2020:
            choice = "immediate"
        elif 2021 <= data.year <= 2276:
            choice = "near_future"
        elif 1749 <= data.year <= 2004:
            choice = "near_past"
        else:
            choice = "reminder"

        data = {"year": (choice, data.year), "month": data.month, "day": data.day}

        return self._inner.encode(data, encoder)

    def decode(self, decoder):
        decoded = self._inner.decode(decoder)

        return datetime.date(decoded["year"][1], decoded["month"], decoded["day"])


class TimeOfDay(Type):
    def __init__(self, name):
        super(TimeOfDay, self).__init__(name, "TIME-OF-DAY")
        hours = Integer("hours")
        minutes = Integer("minutes")
        seconds = Integer("seconds")
        hours.set_restricted_to_range(0, 24, False)
        minutes.set_restricted_to_range(0, 59, False)
        seconds.set_restricted_to_range(0, 60, False)
        self._inner = Sequence("TIME-OF-DAY-ENCODING", [hours, minutes, seconds], None)

    def encode(self, data, encoder):
        data = {"hours": data.hour, "minutes": data.minute, "seconds": data.second}

        return self._inner.encode(data, encoder)

    def decode(self, decoder):
        decoded = self._inner.decode(decoder)

        return datetime.time(decoded["hours"], decoded["minutes"], decoded["seconds"])


class DurationIntegerMixin(Integer):
    """An INTEGER of DURATION-INTERVAL-ENCODING, which only has values
    above its root in the extension (X.691 32.6.1, 13.1).

    """

    def decode(self, decoder):
        if not decoder.read_bit():
            return self.decode_constrained(decoder)

        decoder.align()
        value = decoder.read_unconstrained_whole_number()

        if value <= self.maximum:
            raise DecodeError(
                "Expected a value above {} in the extension, but got {}.".format(
                    self.maximum, value
                )
            )

        return value


class Duration(Type):
    def __init__(self, name):
        super(Duration, self).__init__(name, "DURATION")
        self._inner = self._make_inner(Integer)

    @classmethod
    def _make_inner(cls, integer_cls):
        duration_integer_cls = type(
            "DurationInteger", (DurationIntegerMixin, integer_cls), {}
        )

        def make_int(name, min_val, max_val, opt=True):
            i = copy(duration_integer_cls(name))
            i.set_restricted_to_range(min_val, max_val, True)
            i.optional = opt
            return i

        years = make_int("years", 0, 31)
        months = make_int("months", 0, 15)
        weeks = make_int("weeks", 0, 63)
        days = make_int("days", 0, 31)
        hours = make_int("hours", 0, 31)
        minutes = make_int("minutes", 0, 63)
        seconds = make_int("seconds", 0, 63)

        nod = make_int("number-of-digits", 1, 3, False)
        fv = make_int("fractional-value", 0, 999, False)
        fp = copy(Sequence("fractional-part", [nod, fv], None))
        fp.optional = True

        return Sequence(
            "DURATION-INTERVAL-ENCODING",
            [years, months, weeks, days, hours, minutes, seconds, fp],
            None,
        )

    def encode(self, data, encoder):
        fields = duration_to_dict(data)
        return self._inner.encode(fields, encoder)

    def decode(self, decoder):
        fields = self._inner.decode(decoder)
        return dict_to_duration(fields)


class DateTime(Type):
    def __init__(self, name):
        super(DateTime, self).__init__(name, "DATE-TIME")
        self._inner = Sequence(
            "DATE-TIME-ENCODING", [Date("date"), TimeOfDay("time")], None
        )

    def encode(self, data, encoder):
        data = {"date": data, "time": data}

        return self._inner.encode(data, encoder)

    def decode(self, decoder):
        decoded = self._inner.decode(decoder)

        return datetime.datetime(
            decoded["date"].year,
            decoded["date"].month,
            decoded["date"].day,
            decoded["time"].hour,
            decoded["time"].minute,
            decoded["time"].second,
        )


def encode_open_type(data, encoder):
    """The encoding of the value of an open type, as an unconstrained
    OCTET STRING (X.691 11.2), in fragments if it is 16384 bytes or
    more.

    """

    encoder.align()

    for offset, length in encoder.append_length_determinant_chunks(len(data)):
        encoder.align()
        encoder.append_bytes(data[offset : offset + length])


def decode_open_type(decoder):
    decoder.align()
    decoded = []

    for length in decoder.read_length_determinant_chunks():
        decoder.align()
        decoded.append(decoder.read_bytes(length))

    return b"".join(decoded)


def put_open_type(inner, encoder):
    """An open type of what the encoder `inner` holds: its complete
    encoding, which is one byte 0 if it is empty (X.691 11.1).

    """

    inner.align_always()

    if inner.number_of_bits == 0:
        inner.append_non_negative_binary_integer(0, 8)

    encode_open_type(inner.as_bytearray(), encoder)


def get_open_type(decoder, decode, *args):
    """What `decode` reads from an open type, with a decoder of its own
    and `args`, see `put_open_type`. It has to fill the octets, but for
    the padding of the last.

    """

    offset = decoder.number_of_read_bits()
    data = decode_open_type(decoder)
    inner = decoder.__class__(bytearray(data))

    try:
        decoded = decode(inner, *args)
    except DecodeError as e:
        # Where in all of the data, or at the open type if it is in
        # fragments.
        if e.offset is not None:
            if len(data) < 16384:
                e.offset += decoder.number_of_read_bits() - 8 * len(data)
            else:
                e.offset = offset

        raise e

    used = inner.number_of_read_bits()

    if (used + 7) // 8 != len(data) and not (used == 0 and data == b"\x00"):
        raise DecodeError("The value does not fill its open type.", offset=offset)

    return decoded


class OpenType(Type):
    def __init__(self, name):
        super(OpenType, self).__init__(name, "OpenType")

    def encode(self, data, encoder):
        encode_open_type(data, encoder)

    def decode(self, decoder):
        return decode_open_type(decoder)


class Any(Type):
    """An ANY, encoded as the open type that took its place (X.691
    11.2), with the encoding of its value.

    """

    def __init__(self, name):
        super(Any, self).__init__(name, "ANY")

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
        super(AnyDefinedBy, self).__init__(name, "ANY DEFINED BY")
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

        inner = encoder.__class__()
        choice.encode(data, inner)
        put_open_type(inner, encoder)

    def decode(self, decoder, values=None):
        if not self.choices:
            return decode_open_type(decoder)

        choice = self.choice(values)

        if choice is None:
            raise DecodeError(
                self.bad_choice(values), offset=decoder.number_of_read_bits()
            )

        return get_open_type(decoder, choice.decode)


class Recursive(compiler.Recursive, Type):
    def __init__(self, name, type_name, module_name):
        super(Recursive, self).__init__(name, "RECURSIVE")
        self.type_name = type_name
        self.module_name = module_name
        self._inner: typing.Any = None

    def set_inner_type(self, inner):
        self._inner = inner

    def encode(self, data, encoder):
        self._inner.encode(data, encoder)

    def decode(self, decoder):
        return self._inner.decode(decoder)


class AdditionGroup(Sequence):
    pass


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
        elif type_name == "UniversalString":
            permitted_alphabet = self.get_permitted_alphabet(type_descriptor)
            compiled = UniversalString(
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

    def set_compiled_tag(self, compiled, type_descriptor):
        compiled = self.copy(compiled)
        tag = type_descriptor["tag"]
        class_prio = CLASS_PRIO[tag.get("class", "CONTEXT_SPECIFIC")]
        class_number = tag["number"]
        compiled.tag = (class_prio, class_number)

        return compiled

    def compile_members(
        self, members, module_name, sort_by_tag=False, flat_additions=False
    ):
        compiled_members: list[typing.Any] = []
        in_extension = False
        additions: list[typing.Any] | None = None

        for member in members:
            if member == EXTENSION_MARKER:
                in_extension = not in_extension

                if in_extension:
                    additions = []
            elif in_extension:
                self.compile_extension_member(
                    member, module_name, additions, flat_additions
                )
            else:
                self.compile_root_member(member, module_name, compiled_members)

        if sort_by_tag:
            compiled_members = self.sort_members_by_tag(
                compiled_members, members, module_name
            )

        return compiled_members, additions

    def compile_extension_member(self, member, module_name, additions, flat_additions):
        if isinstance(member, list):
            if flat_additions:
                for memb in member:
                    compiled_member = self.compile_member(memb, module_name)
                    additions.append(compiled_member)
            else:
                compiled_member, _ = self.compile_members(member, module_name)
                compiled_group = AdditionGroup(
                    "ExtensionAddition", compiled_member, None
                )
                additions.append(compiled_group)
        else:
            compiled_member = self.compile_member(member, module_name)
            additions.append(compiled_member)

    def get_permitted_alphabet(self, type_descriptor):
        def char_range(begin, end):
            return "".join([chr(char) for char in range(ord(begin), ord(end) + 1)])

        if "from" not in type_descriptor:
            return

        permitted_alphabet = type_descriptor["from"]

        # An extensible permitted alphabet is not PER-visible (X.691
        # 10.3.11), and an extensible constraint is not checked.
        if EXTENSION_MARKER in permitted_alphabet:
            return

        value = ""

        for item in permitted_alphabet:
            if isinstance(item, tuple):
                value += char_range(item[0], item[1])
            else:
                value += item

        return sorted(set(map(ord, value)))


class PERCodec(Codec):
    @staticmethod
    def compile_dict(specification, numeric_enums=False, lenient=False):
        return Compiler(specification, numeric_enums).process()

    @staticmethod
    def decode_full_length(data):
        return decode_full_length(data)


def decode_full_length(_data):
    raise DecodeError("Decode length is not supported for this codec.")
