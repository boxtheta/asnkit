# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""What is decoded may be anything: cut short, changed, or made up.
Whatever it is, decoding ends in a value or in an error of this package,
and not in another exception, an endless loop or more memory than the
data is worth.

Each case here was found by decoding changed data with the runtimes for
Python and for Rust and comparing what they said. They are vectors too
(see `tests/vectors/record.py`), so every runtime is held to them.

"""

import unittest
import zlib

import asnkitrt
from tests import compiled as fixtures

SPECIFICATION = """
Robust DEFINITIONS AUTOMATIC TAGS ::= BEGIN
S ::= SEQUENCE { a BOOLEAN }
SX ::= SEQUENCE { a BOOLEAN, ... }
SY ::= SEQUENCE { a BOOLEAN, ..., b INTEGER OPTIONAL }
X ::= [5] EXPLICIT INTEGER
OS ::= OCTET STRING
E ::= ENUMERATED { a, b }
I ::= INTEGER
R ::= REAL
L ::= SEQUENCE OF BOOLEAN
SL ::= SET OF BOOLEAN
LN ::= SEQUENCE OF NULL
O ::= OBJECT IDENTIFIER
U ::= UniversalString
BS ::= BIT STRING
C ::= CHOICE { a BOOLEAN, b INTEGER }
D ::= DATE
T ::= TIME-OF-DAY
END
"""

# What decoding gives for an error.
ERROR = object()

# The codec, the type, the data and what it is decoded to.
CASES = [
    # An element of another tag, which was decoded again without an end.
    ("der", "L", "3003020100", ERROR),
    ("der", "SL", "3103020100", ERROR),
    ("ber", "L", "3003020100", ERROR),
    # A tag number has one form (X.690 8.1.2).
    ("ber", "I", "1f020105", ERROR),
    ("der", "I", "1f020105", ERROR),
    ("ber", "C", "9f0001ff", ERROR),
    ("ber", "C", "9f808001ff", ERROR),
    ("ber", "I", "020105", 5),
    # The first subidentifier is of the first two arcs (X.690 8.19.4).
    ("ber", "O", "06017f", "2.47"),
    ("ber", "O", "0602a121", "2.4177"),
    ("ber", "O", "060188", ERROR),
    ("ber", "O", "0600", ERROR),
    ("ber", "O", "06028001", "0.1"),
    ("der", "O", "06028001", ERROR),
    ("uper", "O", "017f", "2.47"),
    ("oer", "O", "017f", "2.47"),
    # The unused bits are of the last byte (X.690 8.6.2).
    ("ber", "BS", "030101", ERROR),
    ("ber", "BS", "03020800", ERROR),
    ("ber", "BS", "0300", ERROR),
    ("ber", "BS", "03020780", (b"\x80", 1)),
    ("oer", "BS", "020800", ERROR),
    ("oer", "BS", "0101", ERROR),
    ("oer", "BS", "020780", (b"\x80", 1)),
    # DER has one encoding: nothing follows the members, but for the
    # additions of a later version.
    ("der", "S", "30068001ff020100", ERROR),
    ("der", "S", "30038001ff", {"a": True}),
    ("der", "SX", "30068001ff020100", {"a": True}),
    ("der", "SX", "30058001ff0580", ERROR),
    ("der", "SX", "30048001ff05", ERROR),
    # And so in BER, where an addition is all of a TLV of BER.
    ("ber", "S", "30068001ff020100", ERROR),
    ("ber", "S", "30038001ff", {"a": True}),
    ("ber", "S", "30808001ff0201000000", ERROR),
    ("ber", "SX", "30068001ff020100", {"a": True}),
    ("ber", "SX", "30078001ff02810100", {"a": True}),
    ("der", "SX", "30078001ff02810100", ERROR),
    ("ber", "SX", "300a8001ffa0800201000000", {"a": True}),
    ("ber", "SX", "30808001ffa08002010000000000", {"a": True}),
    ("ber", "SX", "30058001ff0580", ERROR),
    ("ber", "SX", "30048001ff05", ERROR),
    # The end of the contents is read once, also before additions that
    # are known.
    ("ber", "SY", "30808001ff0000", {"a": True}),
    ("ber", "SY", "30808001ff8101050000", {"a": True, "b": 5}),
    # Nothing in the contents ends after them.
    ("ber", "S", "30028001ff", ERROR),
    ("der", "S", "30028001ff", ERROR),
    ("ber", "L", "30020101ff", ERROR),
    ("der", "L", "30020101ff", ERROR),
    ("ber", "OS", "24040403616263", ERROR),
    ("ber", "OS", "2405040361626300", ERROR),
    ("ber", "OS", "24050403616263", b"abc"),
    # What is tagged is all of the contents of the tag.
    ("ber", "X", "a503020105", 5),
    ("ber", "X", "a5800201050000", 5),
    ("ber", "X", "a5050201050000", ERROR),
    ("der", "X", "a5050201050000", ERROR),
    ("ber", "X", "a502020105", ERROR),
    # A length and an INTEGER are of a byte or more.
    ("oer", "I", "80", ERROR),
    ("oer", "I", "00", ERROR),
    ("oer", "I", "0105", 5),
    ("oer", "E", "80", ERROR),
    ("oer", "E", "01", "b"),
    ("uper", "I", "00", ERROR),
    ("uper", "I", "0105", 5),
    # A tag number has one form (X.696 8.7.2).
    ("oer", "C", "bf01ff", ERROR),
    ("oer", "C", "80ff", ("a", True)),
    # Items of no bits are as many as the length says, and not more
    # than the data is worth.
    ("oer", "LN", "02ffff", ERROR),
    ("oer", "LN", "0105", [None] * 5),
    ("uper", "LN", "c400", ERROR),
    ("uper", "LN", "05", [None] * 5),
    ("per", "LN", "c400", ERROR),
    # A date is one of the calendar, and a time one of the day.
    ("oer", "D", "0207d50d01", ERROR),
    ("oer", "D", "0207d5021e", ERROR),
    ("oer", "T", "180000", ERROR),
    ("oer", "T", "003c00", ERROR),
    ("uper", "D", "00ff", ERROR),
    ("uper", "T", "ff0000", ERROR),
    # Half of a surrogate pair is not a character.
    ("uper", "U", "010000d800", ERROR),
    ("uper", "U", "0100000061", "a"),
    # A JSON value is of the kind of its type.
    ("jer", "I", b"1.5", ERROR),
    ("jer", "I", b'"1"', ERROR),
    ("jer", "I", b"true", ERROR),
    ("jer", "I", b"1e2", ERROR),
    ("jer", "I", b"12", 12),
    ("jer", "R", b"1", 1.0),
    ("jer", "R", b'"1"', ERROR),
    ("jer", "R", b"true", ERROR),
    ("jer", "S", b"{}", ERROR),
    ("jer", "S", b"[]", ERROR),
    ("jer", "S", b'{"a": 1}', ERROR),
    ("jer", "S", b'{"a": true}', {"a": True}),
    ("jer", "L", b"{}", ERROR),
    ("jer", "L", b"[[]]", ERROR),
    ("jer", "C", b"{}", ERROR),
    ("jer", "C", b"[]", ERROR),
    ("jer", "U", b"1", ERROR),
    ("jer", "BS", b'{"value": "00", "length": 20}', ERROR),
    ("jer", "BS", b'{"value": "0", "length": 4}', ERROR),
    ("jer", "BS", b'{"value": "80", "length": 1}', (b"\x80", 1)),
    # The first arc is 0, 1 or 2, and the second below 40 under 0 and 1.
    ("jer", "O", b'"3.7"', ERROR),
    ("jer", "O", b'"1.40"', ERROR),
    ("jer", "O", b'"1.2/3"', ERROR),
    ("jer", "O", b'"1..3"', ERROR),
    ("jer", "O", b'"+1.2"', ERROR),
    ("jer", "O", b'"1"', ERROR),
    ("jer", "O", b'"2.999.1"', "2.999.1"),
    ("xer", "O", b"<O>3.7</O>", ERROR),
    ("xer", "O", b"<O>1.40</O>", ERROR),
    ("xer", "O", b"<O>2.999.1</O>", "2.999.1"),
    # An INTEGER is digits.
    ("xer", "I", b"<I>1_000</I>", ERROR),
    ("xer", "I", b"<I>1 0</I>", ERROR),
    ("xer", "I", b"<I></I>", ERROR),
    ("xer", "I", b"<I> -12 </I>", -12),
    # A member that has to be there, and XML that is XML.
    ("xer", "S", b"<S></S>", ERROR),
    ("xer", "S", b"<S><a><true/></a></S>", {"a": True}),
    ("xer", "S", b"<S(/>", ERROR),
    ("xer", "L", b"< />", ERROR),
    ("xer", "S", b"<!DOCTYPE S []><S><a><true/></a></S>", ERROR),
]

# What a lenient reader decodes: anything after the members that are
# known, and after what is tagged, but nothing that ends after its
# contents.
LENIENT_CASES = [
    ("ber", "S", "30068001ff020100", {"a": True}),
    ("der", "S", "30068001ff020100", {"a": True}),
    ("ber", "SX", "30048001ff05", {"a": True}),
    ("der", "SX", "30058001ff0580", {"a": True}),
    ("ber", "X", "a5050201050000", 5),
    ("der", "X", "a5050201050000", 5),
    ("ber", "S", "30028001ff", ERROR),
    ("der", "S", "30028001ff", ERROR),
    ("ber", "X", "a502020105", ERROR),
    ("ber", "S", "30808001ff0201000000", ERROR),
]

# An OBJECT IDENTIFIER that is not one is not encoded as another.
NOT_OBJECT_IDENTIFIERS = ["3.7", "1.40", "0.40", "1", "", "1.2.", "1.-2", "+1.2", "a.b"]


def data_of(data):
    return data if isinstance(data, bytes) else bytes.fromhex(data)


class RobustTest(unittest.TestCase):
    maxDiff = None

    def setUp(self):
        self.specs = {}

    def spec(self, codec, lenient=False):
        if (codec, lenient) not in self.specs:
            self.specs[codec, lenient] = fixtures.compile_string(
                SPECIFICATION, codec, lenient=lenient
            )

        return self.specs[codec, lenient]

    def test_cases(self):
        self.check(CASES, False)

    def test_lenient_cases(self):
        self.check(LENIENT_CASES, True)

    def check(self, cases, lenient):
        failures = []

        for codec, name, data, expected in cases:
            describe = "{} {} {!r}".format(codec, name, data)

            try:
                decoded = self.spec(codec, lenient).decode(name, data_of(data))
            except asnkitrt.DecodeError:
                if expected is not ERROR:
                    failures.append("failed: " + describe)
            except Exception as error:
                failures.append("raised {!r}: {}".format(error, describe))
            else:
                if expected is ERROR:
                    failures.append("gave {!r}: {}".format(decoded, describe))
                elif decoded != expected:
                    failures.append("gave {!r}: {}".format(decoded, describe))

        self.assertEqual(failures, [])

    def test_object_identifiers_that_are_not(self):
        for codec in ["ber", "der", "oer", "per", "uper", "jer", "xer", "gser"]:
            for value in NOT_OBJECT_IDENTIFIERS:
                with self.assertRaises(asnkitrt.EncodeError, msg=(codec, value)):
                    self.spec(codec).encode("O", value, check_types=False)

            self.spec(codec).encode("O", "2.999.1")

    def test_a_tag_of_another_element_ends(self):
        # It was decoded again and again, until the memory was used up.
        with self.assertRaises(asnkitrt.DecodeError) as cm:
            self.spec("der").decode("L", b"\x30\x03\x02\x01\x00")

        self.assertEqual(
            str(cm.exception),
            "L: Expected BOOLEAN with tag '01', but got '02'. (At offset: 2)",
        )

    def test_a_file_that_is_not_one(self):
        """A file of which the CRC-32 matches and the nodes are not
        those of a specification is an error of this package.

        """

        plain = fixtures.compile_string_to_asni(SPECIFICATION)
        tried = 0

        for index in range(10, len(plain)):
            for change in (0x01, 0x20, 0x80):
                body = bytearray(plain[10:])
                body[index - 10] ^= change
                data = plain[:6] + zlib.crc32(body).to_bytes(4, "big") + body
                tried += 1

                try:
                    asnkitrt.load_asni(data, "uper")
                except asnkitrt.Error:
                    pass

        self.assertGreater(tried, 300)


if __name__ == "__main__":
    unittest.main()
