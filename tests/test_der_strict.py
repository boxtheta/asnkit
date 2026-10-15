# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

"""What DER decodes, and what it does not unless told to be lenient.

Each bad encoding here is one that BER allows and X.690 clause 10 and
11 do not. It is rejected by default and decoded with lenient=True.

"""

import datetime
import unittest
from typing import Any

import asnkitrt
from tests import compiled as fixtures

SPECIFICATION = """
A DEFINITIONS ::= BEGIN
Bool ::= BOOLEAN
Int ::= INTEGER
Enum ::= ENUMERATED { a(0), b(1) }
Oid ::= OBJECT IDENTIFIER
Plain ::= BIT STRING
Named ::= BIT STRING { a(0), b(1), c(2) }
Nothing ::= NULL
Numbers ::= SET OF INTEGER
WithDefault ::= SEQUENCE { a INTEGER DEFAULT 5, b BOOLEAN OPTIONAL }
InOrder ::= SEQUENCE { a [0] INTEGER OPTIONAL, b [1] INTEGER OPTIONAL }
Unordered ::= SET { a INTEGER, b BOOLEAN }
Item ::= SEQUENCE { n INTEGER }
Nested ::= [APPLICATION 1] EXPLICIT SEQUENCE OF Item
Deep ::= SEQUENCE { nested [0] EXPLICIT SET OF Nested }
Real ::= REAL
Utc ::= UTCTime
Generalized ::= GeneralizedTime
Duration ::= DURATION
END
"""


class DerStrictTest(unittest.TestCase):
    strict: Any
    lenient: Any

    @classmethod
    def setUpClass(cls) -> None:
        cls.strict = fixtures.compile_string(SPECIFICATION, "der")
        cls.lenient = fixtures.compile_string(SPECIFICATION, "der", lenient=True)

    def test_nested_integer_not_minimal(self) -> None:
        # [APPLICATION 1] EXPLICIT, SEQUENCE OF, SEQUENCE, INTEGER 0x0005.
        data = bytes.fromhex("61083006300402020005")

        with self.assertRaises(asnkitrt.DecodeError):
            self.strict.decode("Nested", data)

        self.assertEqual(self.lenient.decode("Nested", data), [{"n": 5}])

    def test_deeper_nesting(self) -> None:
        inner = bytes.fromhex("61083006300402020005")
        # [0] EXPLICIT SET OF Nested, in a SEQUENCE.
        set_of = b"\x31" + bytes([len(inner)]) + inner
        explicit = b"\xa0" + bytes([len(set_of)]) + set_of
        data = b"\x30" + bytes([len(explicit)]) + explicit

        with self.assertRaises(asnkitrt.DecodeError):
            self.strict.decode("Deep", data)

        self.assertEqual(self.lenient.decode("Deep", data), {"nested": [[{"n": 5}]]})

    def check(self, type_name: str, encoded: str, lenient_value: Any) -> None:
        data = bytes.fromhex(encoded)

        with self.assertRaises(asnkitrt.DecodeError):
            self.strict.decode(type_name, data)

        self.assertEqual(self.lenient.decode(type_name, data), lenient_value)

    def test_boolean_other_than_0xff(self) -> None:
        self.check("Bool", "010101", True)

    def test_boolean_length_not_minimal(self) -> None:
        self.check("Bool", "018101ff", True)

    def test_integer_not_minimal(self) -> None:
        self.check("Int", "02020001", 1)
        self.check("Int", "0202ff80", -128)

    def test_integer_without_bytes(self) -> None:
        self.check("Int", "0200", 0)

    def test_enumerated_not_minimal(self) -> None:
        self.check("Enum", "0a020001", "b")

    def test_object_identifier_with_a_leading_0x80(self) -> None:
        self.check("Oid", "06032a8001", "1.2.1")

    def test_bit_string_with_unused_bits_not_zero(self) -> None:
        self.check("Plain", "030204f1", (b"\xf1", 4))

    def test_named_bits_with_a_trailing_zero(self) -> None:
        self.check("Named", "030205c0", (b"\xc0", 3))

    def test_bit_string_with_too_many_unused_bits_is_never_decoded(self) -> None:
        for codec in (self.strict, self.lenient):
            with self.assertRaises(asnkitrt.DecodeError):
                codec.decode("Plain", bytes.fromhex("03020800"))

            with self.assertRaises(asnkitrt.DecodeError):
                codec.decode("Plain", bytes.fromhex("0300"))

    def test_null_with_contents_is_never_decoded(self) -> None:
        for codec in (self.strict, self.lenient):
            with self.assertRaises(asnkitrt.DecodeError):
                codec.decode("Nothing", bytes.fromhex("050100"))

    def test_set_of_not_in_order(self) -> None:
        self.check("Numbers", "3106020102020101", [2, 1])
        self.check("Numbers", "3106020105020104", [5, 4])
        # 02 02 00 ff comes after 02 01 05, as 02 01 05 is padded with 00.
        self.check("Numbers", "310a020200ff020104020105", [255, 4, 5])

    def test_member_with_its_default(self) -> None:
        self.check("WithDefault", "3003020105", {"a": 5})

    def test_sequence_members_not_in_order(self) -> None:
        self.check("InOrder", "3008a103020102a003020101", {"a": 1, "b": 2})

    def test_set_members_not_in_order(self) -> None:
        self.check("Unordered", "31060201010101ff", {"a": 1, "b": True})

    def test_length_not_minimal(self) -> None:
        self.check("Item", "308103020105", {"n": 5})

    def test_indefinite_length(self) -> None:
        self.check("Item", "30800201050000", {"n": 5})

    def test_what_is_left_after_the_data_is_never_accepted(self) -> None:
        for codec in (self.strict, self.lenient):
            with self.assertRaises(asnkitrt.DecodeError):
                codec.decode("Int", bytes.fromhex("02010100"))

    def test_real_not_in_base_2(self) -> None:
        self.check("Real", "0903900101", 8.0)

    def test_real_with_a_scaling_factor(self) -> None:
        self.check("Real", "0903840003", 6.0)

    def test_real_with_an_even_mantissa(self) -> None:
        self.check("Real", "0903800002", 2.0)

    def test_real_mantissa_not_minimal(self) -> None:
        self.check("Real", "090480000001", 1.0)

    def test_real_exponent_not_minimal(self) -> None:
        self.check("Real", "090481000001", 1.0)
        self.check("Real", "090483010001", 1.0)

    def test_real_decimal_not_nr3(self) -> None:
        self.check("Real", "09020131", 1.0)

    def test_real_decimal_not_canonical(self) -> None:
        # 10.E-1, 1.E0, 1.E+1, 1.5E+0 and 01.E+0 (X.690 11.3.2).
        self.check("Real", "09070331302e452d31", 1.0)
        self.check("Real", "090503312e4530", 1.0)
        self.check("Real", "090603312e452b31", 10.0)
        self.check("Real", "090703312e35452b30", 1.5)
        self.check("Real", "09070330312e452b30", 1.0)

    def test_utc_time_without_seconds(self) -> None:
        self.check(
            "Utc", "170b" + b"9207221321Z".hex(), datetime.datetime(1992, 7, 22, 13, 21)
        )

    def test_utc_time_with_an_offset(self) -> None:
        # The same time in UTC.
        self.check(
            "Utc",
            "1711" + b"920722132100+0100".hex(),
            datetime.datetime(1992, 7, 22, 12, 21),
        )

    def test_generalized_time_with_trailing_zeros(self) -> None:
        self.check(
            "Generalized",
            "1812" + b"19920722132100.30Z".hex(),
            datetime.datetime(1992, 7, 22, 13, 21, 0, 300000),
        )

    def test_generalized_time_with_a_comma(self) -> None:
        self.check(
            "Generalized",
            "1811" + b"19920722132100,3Z".hex(),
            datetime.datetime(1992, 7, 22, 13, 21, 0, 300000),
        )

    def test_generalized_time_without_seconds(self) -> None:
        self.check(
            "Generalized",
            "180d" + b"199207221321Z".hex(),
            datetime.datetime(1992, 7, 22, 13, 21),
        )

    def test_generalized_time_with_an_offset(self) -> None:
        self.check(
            "Generalized",
            "1813" + b"19920722132100-0130".hex(),
            datetime.datetime(1992, 7, 22, 14, 51),
        )

    def test_duration_with_a_comma(self) -> None:
        self.check(
            "Duration",
            "1f2204" + b"1,5Y".hex(),
            "P1,5Y",
        )

    def test_duration_with_unnecessary_zero(self) -> None:
        self.check(
            "Duration",
            "1f2205" + b"0Y29M".hex(),
            "P0Y29M",
        )

    def test_good_data_is_decoded_either_way(self) -> None:
        for codec in (self.strict, self.lenient):
            self.assertEqual(codec.decode("Bool", bytes.fromhex("0101ff")), True)
            self.assertEqual(codec.decode("Int", bytes.fromhex("020100")), 0)
            self.assertEqual(codec.decode("Int", bytes.fromhex("020200ff")), 255)
            self.assertEqual(codec.decode("Int", bytes.fromhex("0201ff")), -1)
            self.assertEqual(
                codec.decode("Named", bytes.fromhex("030206c0")), (b"\xc0", 2)
            )
            self.assertEqual(codec.decode("Named", bytes.fromhex("030100")), (b"", 0))
            self.assertEqual(
                codec.decode("WithDefault", bytes.fromhex("3000")), {"a": 5}
            )
            self.assertEqual(
                codec.decode("Numbers", bytes.fromhex("3106020101020102")), [1, 2]
            )
            # 255.0, 1.E+0 and -15.E-1.
            self.assertEqual(codec.decode("Real", bytes.fromhex("09038000ff")), 255.0)
            self.assertEqual(
                codec.decode("Real", bytes.fromhex("090603312e452b30")), 1.0
            )
            self.assertEqual(
                codec.decode("Real", bytes.fromhex("0908032d31352e452d31")), -1.5
            )
            self.assertEqual(
                codec.decode("Utc", b"\x17\x0d920722132100Z"),
                datetime.datetime(1992, 7, 22, 13, 21),
            )
            self.assertEqual(
                codec.decode("Generalized", b"\x18\x1119920722132100.3Z"),
                datetime.datetime(1992, 7, 22, 13, 21, 0, 300000),
            )
            self.assertEqual(
                codec.decode("Duration", bytes.fromhex("1f2204" + b"1.5Y".hex())),
                "P1.5Y",
            )
            self.assertEqual(
                codec.decode("Duration", bytes.fromhex("1f2203" + b"29M".hex())),
                "P29M",
            )

    def test_what_is_encoded_is_der_whatever_lenient_is(self) -> None:
        for codec in (self.strict, self.lenient):
            self.assertEqual(
                codec.encode("Numbers", [2, 1]), bytes.fromhex("3106020101020102")
            )
            self.assertEqual(
                codec.encode("Numbers", [5, 4, 255]),
                bytes.fromhex("310a020104020105020200ff"),
            )
            self.assertEqual(
                codec.encode("WithDefault", {"a": 5}), bytes.fromhex("3000")
            )
            self.assertEqual(
                codec.encode("Unordered", {"a": 1, "b": True}),
                bytes.fromhex("31060101ff020101"),
            )
            # Trailing zero bits of named bits are left out.
            self.assertEqual(
                codec.encode("Named", (b"\xc0", 3)), bytes.fromhex("030206c0")
            )
            self.assertEqual(
                codec.encode("Duration", "P1,5Y"),
                bytes.fromhex("1f2204" + b"1.5Y".hex()),
            )
            self.assertEqual(
                codec.encode("Duration", "P0Y29M"),
                bytes.fromhex("1f2203" + b"29M".hex()),
            )
            self.assertEqual(
                codec.encode("Named", (b"\x00", 3)), bytes.fromhex("030100")
            )
            self.assertEqual(
                codec.encode("Plain", (b"\xc0", 3)), bytes.fromhex("030205c0")
            )

    def test_only_the_der_codec_looks_at_lenient(self) -> None:
        ber = fixtures.compile_string(SPECIFICATION, "ber", lenient=True)

        self.assertEqual(ber.decode("Bool", bytes.fromhex("010101")), True)


if __name__ == "__main__":
    unittest.main()
