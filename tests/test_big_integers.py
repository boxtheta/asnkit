# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""INTEGERs with bounds of more than 128 bits, which the runtime for
Rust takes with the feature bigint, encoded as X.691 13 and X.696 10
have them.

"""

import unittest

from tests import compiled as fixtures

BIG = 2**130

SPECIFICATION = """
B DEFINITIONS AUTOMATIC TAGS ::= BEGIN
Range ::= INTEGER (0..{big})
Low ::= INTEGER ({big}..MAX)
Both ::= INTEGER (-{big}..{big})
Excluded ::= INTEGER (0..<{big})
Small ::= INTEGER ({big}..{big_and_5})
Values ::= INTEGER (1 | {big})
Extensible ::= INTEGER (0..{big}, ...)
END
""".format(big=BIG, big_and_5=BIG + 5)


class BigIntegersTest(unittest.TestCase):
    def check(self, codec: str, datas: list[tuple[str, int, str]]) -> None:
        spec = fixtures.compile_string(SPECIFICATION, codec)

        for name, value, encoded in datas:
            with self.subTest(codec=codec, name=name, value=value):
                self.assertEqual(
                    spec.encode(name, value, check_constraints=True).hex(), encoded
                )
                self.assertEqual(
                    spec.decode(name, bytes.fromhex(encoded), check_constraints=True),
                    value,
                )

    def test_per(self) -> None:
        self.check(
            "per",
            [
                # A range of more than 64K values: the number of octets in
                # 5 bits, then the octets (X.691 11.5.7.4).
                ("Range", BIG, "80" + "04" + "00" * 16),
                ("Range", 5, "00" + "05"),
                # A lower bound only: what is above it, with a length.
                ("Low", BIG + 7, "0107"),
                ("Both", -BIG, "00" + "00"),
                ("Excluded", BIG - 1, "80" + "03" + "ff" * 16),
                # A range of 6 values, in 3 bits.
                ("Small", BIG + 5, "a0"),
                ("Values", BIG, "80" + "03" + "ff" * 16),
                # Outside the root, as if there was no constraint.
                ("Extensible", -1, "80" + "01ff"),
            ],
        )

    def test_uper(self) -> None:
        self.check(
            "uper",
            [
                # 131 bits.
                ("Range", BIG, "80" + "00" * 16),
                ("Range", 5, "00" * 16 + "a0"),
                ("Small", BIG + 5, "a0"),
                ("Extensible", -1, "80ff80"),
            ],
        )

    def test_oer(self) -> None:
        self.check(
            "oer",
            [
                # Unsigned, as the lower bound is 0, with a length.
                ("Range", BIG, "11" + "04" + "00" * 16),
                ("Low", BIG + 7, "11" + "04" + "00" * 15 + "07"),
                # Signed.
                ("Both", -BIG, "11" + "fc" + "00" * 16),
                ("Small", BIG + 5, "11" + "04" + "00" * 15 + "05"),
                # No bounds, as the constraint is extensible.
                ("Extensible", -1, "01ff"),
            ],
        )

    def test_constraints(self) -> None:
        spec = fixtures.compile_string(SPECIFICATION, "ber")

        for name, value in [
            ("Range", BIG + 1),
            ("Range", -1),
            ("Low", BIG - 1),
            ("Excluded", BIG),
            ("Values", 2),
        ]:
            with self.subTest(name=name, value=value):
                with self.assertRaises(fixtures.ConstraintsError):
                    spec.encode(name, value, check_constraints=True)

        # An extensible constraint is not checked.
        spec.encode("Extensible", -1, check_constraints=True)


if __name__ == "__main__":
    unittest.main()
