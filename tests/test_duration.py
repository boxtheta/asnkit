# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""What the codecs make of DURATION at its edges: values that are
valid but large, and encodings that X.691 32.6 and X.696 29.4 do not
allow, which a decoder has to reject rather than turn into a wrong or
huge duration.

"""

import unittest
from typing import Any

from tests import compiled as fixtures

SPECIFICATION = """
D DEFINITIONS AUTOMATIC TAGS ::= BEGIN
Duration ::= DURATION
END
"""


def bits(text: str) -> bytes:
    """Bits written out as 0 and 1, with zeros up to a whole octet."""

    text = text.replace(" ", "")
    text += "0" * (-len(text) % 8)

    return int(text, 2).to_bytes(len(text) // 8, "big")


class DurationTest(unittest.TestCase):
    specs: dict[str, Any]

    @classmethod
    def setUpClass(cls) -> None:
        cls.specs = {
            codec: fixtures.compile_string(SPECIFICATION, codec)
            for codec in ("ber", "der", "oer", "per", "uper", "jer", "xer")
        }

    def assert_decode_error(self, codec: str, encoded: bytes) -> None:
        with self.assertRaises(fixtures.DecodeError):
            self.specs[codec].decode("Duration", encoded)

    def test_large_values(self) -> None:
        # No time element has an upper bound (X.680 38.4.4.2).
        for value in ["P{}Y".format(2**130), "PT{}.5S".format(10**40)]:
            for codec, spec in self.specs.items():
                with self.subTest(codec=codec, value=value):
                    encoded = spec.encode("Duration", value)
                    self.assertEqual(spec.decode("Duration", encoded), value)

    def test_fraction_digits(self) -> None:
        within = "PT1." + "5" * 38 + "S"
        beyond = "PT1." + "5" * 39 + "S"

        for codec, spec in self.specs.items():
            with self.subTest(codec=codec):
                encoded = spec.encode("Duration", within)
                self.assertEqual(spec.decode("Duration", encoded), within)

        # Only PER and OER have a number of digits to limit.
        for codec in ("oer", "per", "uper"):
            with self.subTest(codec=codec):
                with self.assertRaises(fixtures.EncodeError):
                    self.specs[codec].encode("Duration", beyond)

        for codec in ("ber", "der", "jer", "xer"):
            with self.subTest(codec=codec):
                encoded = self.specs[codec].encode("Duration", beyond)
                self.assertEqual(self.specs[codec].decode("Duration", encoded), beyond)

    def test_final_newline(self) -> None:
        for codec, spec in self.specs.items():
            with self.subTest(codec=codec):
                with self.assertRaises(fixtures.EncodeError):
                    spec.encode("Duration", "P5\nY")

    def test_not_ascii(self) -> None:
        for codec in ("ber", "der"):
            with self.subTest(codec=codec):
                self.assert_decode_error(codec, b"\x1f\x22\x02" + "é".encode())

    def test_uper(self) -> None:
        for name, encoded in [
            # Years in the extension, as -5.
            ("negative", "10000000 1 00000001 11111011"),
            # Years 3 in the extension, which only has 32 and above.
            ("in the root", "10000000 1 00000001 00000011"),
            # Seconds 1, number-of-digits -1, fractional-value 5.
            ("negative digits", "00000011 0 000001 1 00000001 11111111 0 0000000101"),
            # Seconds 1, number-of-digits 2^33, fractional-value 5.
            (
                "too many digits",
                "00000011 0 000001 1 00000101 "
                + format(2**33, "040b")
                + " 0 0000000101",
            ),
            # Seconds 1, number-of-digits 1, fractional-value 15.
            ("too wide", "00000011 0 000001 0 00 0 0000001111"),
            # Years 0 and months 5, where years must be absent.
            ("zero", "11000000 0 00000 0 0101"),
            # Weeks 2 and days 1.
            ("weeks", "00110000 0 000010 0 00001"),
        ]:
            with self.subTest(name):
                self.assert_decode_error("uper", bits(encoded))

    def test_oer(self) -> None:
        for name, encoded in [
            # Seconds 1, number-of-digits 0, fractional-value 5.
            ("no digits", "03 0101 0100 0105"),
            # Seconds 1, number-of-digits 2^33, fractional-value 5.
            ("too many digits", "03 0101 05 0200000000 0105"),
            # Seconds 1, number-of-digits 1, fractional-value 15.
            ("too wide", "03 0101 0101 010f"),
            # Years 0 and months 5, where years must be absent.
            ("zero", "c0 0100 0105"),
            # Nothing at all.
            ("empty", "00"),
        ]:
            with self.subTest(name):
                self.assert_decode_error("oer", bytes.fromhex(encoded.replace(" ", "")))

    def test_oer_large_years(self) -> None:
        encoded = bytes.fromhex("80 10" + "ff" * 16)

        self.assertEqual(
            self.specs["oer"].decode("Duration", encoded), "P{}Y".format(2**128 - 1)
        )


CONSTRAINED = """
C DEFINITIONS AUTOMATIC TAGS ::= BEGIN
NoWeeks ::= DURATION (WITH COMPONENTS {..., weeks ABSENT})
ShortYears ::= DURATION (WITH COMPONENTS {..., years (0..5)})
Millis ::= DURATION (WITH COMPONENTS {
    ..., fractional-part (WITH COMPONENTS {number-of-digits (3)})})
Extensible ::= DURATION (WITH COMPONENTS {..., weeks ABSENT}, ...)
END
"""


class DurationConstraintsTest(unittest.TestCase):
    """The inner constraints of a DURATION, which are those of its
    DURATION-EQUIVALENT value (X.680 38.4.4).

    """

    def test_inner_constraints(self) -> None:
        for codec in ("ber", "uper", "jer"):
            spec = fixtures.compile_string(CONSTRAINED, codec)

            for name, value, satisfied in [
                ("NoWeeks", "P2W", False),
                ("NoWeeks", "P2D", True),
                ("ShortYears", "P9Y", False),
                ("ShortYears", "P3Y", True),
                # The zero years are not a time element of the value.
                ("ShortYears", "P0Y9M", True),
                ("ShortYears", "P{}Y".format(10**50), False),
                ("Millis", "PT1.5S", False),
                ("Millis", "PT1.500S", True),
                ("Millis", "PT1S", True),
                # An extensible constraint is not checked.
                ("Extensible", "P2W", True),
            ]:
                with self.subTest(codec=codec, name=name, value=value):
                    if satisfied:
                        spec.encode(name, value, check_constraints=True)
                    else:
                        with self.assertRaises(fixtures.ConstraintsError):
                            spec.encode(name, value, check_constraints=True)


if __name__ == "__main__":
    unittest.main()
