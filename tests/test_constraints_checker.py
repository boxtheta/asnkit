# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

import unittest
from datetime import datetime
from .utils import BaseTest
from tests import compiled as fixtures


class CheckConstraintsTest(BaseTest):
    maxDiff = None

    def assert_encode_decode_ok(self, foo, datas):
        for type_name, decoded in datas:
            encoded = foo.encode(type_name, decoded, check_constraints=True)
            foo.decode(type_name, encoded, check_constraints=True)

    def assert_encode_decode_bad(self, foo, datas, decode_check=True):
        for type_name, decoded, message in datas:
            # Encode check.
            with self.assertRaises(fixtures.ConstraintsError) as cm:
                foo.encode(type_name, decoded, check_constraints=True)

            self.assertEqual(str(cm.exception), message)

            # Decode check.
            if decode_check:
                encoded = foo.encode(type_name, decoded, check_constraints=False)

                with self.assertRaises(fixtures.ConstraintsError) as cm:
                    foo.decode(type_name, encoded, check_constraints=True)

                self.assertEqual(str(cm.exception), message)

    def test_all_codecs(self):
        codecs = ["ber", "der", "gser", "jer", "per", "uper", "xer"]

        for codec in codecs:
            foo = fixtures.compile_string(
                "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= INTEGER END", codec
            )

            encoded = foo.encode("A", 0, check_constraints=True)

            if codec != "gser":
                foo.decode("A", encoded, check_constraints=True)

    def test_boolean(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= BOOLEAN END"
        )

        # Ok.
        datas = [("A", True), ("A", False)]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = []

        self.assert_encode_decode_bad(foo, datas)

    def test_integer(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= INTEGER "
            "B ::= INTEGER (5..99) "
            "C ::= INTEGER (-10..10) "
            "D ::= INTEGER (5..99, ...) "
            "E ::= INTEGER (1000..1000) "
            "F ::= SEQUENCE { "
            "  a INTEGER (4..4), "
            "  b INTEGER (40..40), "
            "  c INTEGER (400..400) "
            "} "
            "G ::= B (6..7) "
            "H ::= INTEGER (MIN..10) "
            "I ::= INTEGER (10..MAX) "
            "J ::= INTEGER (MIN..MAX) "
            "K ::= INTEGER (1..2 | 4..5) "
            "END"
        )

        # Ok.
        datas = [
            ("A", 32768),
            ("A", 0),
            ("A", -32769),
            ("B", 5),
            ("B", 6),
            ("B", 99),
            ("C", -10),
            ("C", 10),
            ("D", 99),
            ("E", 1000),
            ("F", {"a": 4, "b": 40, "c": 400}),
            ("H", -1000),
            ("H", 10),
            ("I", 10),
            ("I", 1000),
            ("J", -1000),
            ("J", 1000),
            ("K", 1),
            ("K", 2),
            ("K", 4),
            ("K", 5),
        ]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("B", 4, "B: Expected an integer between 5 and 99, but got 4."),
            ("B", 100, "B: Expected an integer between 5 and 99, but got 100."),
            ("C", -11, "C: Expected an integer between -10 and 10, but got -11."),
            ("C", 11, "C: Expected an integer between -10 and 10, but got 11."),
            ("E", 0, "E: Expected an integer between 1000 and 1000, but got 0."),
            (
                "F",
                {"a": 4, "b": 41, "c": 400},
                "F.b: Expected an integer between 40 and 40, but got 41.",
            ),
            ("H", 11, "H: Expected an integer between MIN and 10, but got 11."),
            ("I", 9, "I: Expected an integer between 10 and MAX, but got 9."),
            # ToDo: 4..5 are allowed as well.
            ("K", 3, "K: Expected an integer in 1..2 | 4..5, but got 3."),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_integer_beyond_128_bits(self):
        # Such an integer is beyond any bound on its side and within any on
        # the other, which the runtime for Rust works out without
        # arithmetic on it.
        big = 2**200

        for codec in ["ber", "uper", "oer"]:
            foo = fixtures.compile_string(
                "Foo DEFINITIONS AUTOMATIC TAGS ::= "
                "BEGIN "
                "A ::= INTEGER (5..MAX) "
                "B ::= INTEGER (MIN..0) "
                "C ::= INTEGER (0..10) "
                "D ::= INTEGER (1 | 5..MAX) "
                "END",
                codec,
            )

            self.assert_encode_decode_ok(foo, [("A", big), ("B", -big), ("D", big)])

            for type_name, decoded in [
                ("A", -big),
                ("B", big),
                ("C", big),
                ("C", -big),
                ("D", -big),
            ]:
                with self.subTest(codec=codec, type_name=type_name):
                    with self.assertRaises(fixtures.ConstraintsError):
                        foo.encode(type_name, decoded, check_constraints=True)

    def test_real(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= REAL END"
        )

        # Ok.
        datas = [("A", 1.0)]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = []

        self.assert_encode_decode_bad(foo, datas)

    def test_null(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= NULL END"
        )

        # Ok.
        datas = [("A", None)]

        self.assert_encode_decode_ok(foo, datas)

    def test_bit_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= BIT STRING "
            "B ::= BIT STRING (SIZE (10)) "
            "END"
        )

        # Ok.
        datas = [("A", (b"", 0)), ("B", (b"\x01\x23", 10))]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("B", (b"\x01\x23", 9), "B: Expected between 10 and 10 bits, but got 9.")
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_octet_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= OCTET STRING "
            "B ::= OCTET STRING (SIZE (10)) "
            "C ::= OCTET STRING (SIZE (10, ...)) "
            "D ::= OCTET STRING (SIZE (10..MAX)) "
            "END"
        )

        # Ok.
        datas = [
            ("A", b""),
            ("B", 10 * b"\x23"),
            ("C", 11 * b"\x23"),
            ("D", 44 * b"\x23"),
        ]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("B", 11 * b"\x01", "B: Expected between 10 and 10 bytes, but got 11."),
            ("D", 9 * b"\x01", "D: Expected between 10 and MAX bytes, but got 9."),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_enumerated(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= ENUMERATED { a, b } END"
        )

        # Ok.
        datas = [("A", "a")]

        self.assert_encode_decode_ok(foo, datas)

    def test_sequence(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= SEQUENCE { "
            "  a A OPTIONAL "
            "} "
            "END"
        )

        # Ok.
        datas = [("A", {"a": {}})]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = []

        self.assert_encode_decode_bad(foo, datas)

    def test_sequence_of(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= SEQUENCE (SIZE (2)) OF INTEGER (3..5)"
            "B ::= SEQUENCE (SIZE (2, ...)) OF INTEGER (3..5)"
            "C ::= SEQUENCE (SIZE (MIN..2)) OF INTEGER (3..5)"
            "END"
        )

        # Ok.
        datas = [("A", [3, 4]), ("B", [3, 4, 5]), ("C", [3, 4])]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            (
                "A",
                [3, 4, 5],
                "A: Expected a list of between 2 and 2 elements, but got 3.",
            ),
            ("A", [3, 6], "A: Expected an integer between 3 and 5, but got 6."),
            (
                "C",
                [3, 4, 5],
                "C: Expected a list of between 0 and 2 elements, but got 3.",
            ),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_numeric_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= NumericString END"
        )

        # Ok.
        datas = [("A", "0123456789 ")]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("A", "a", "A: Expected a character in ' 0123456789', but got 'a' (0x61)."),
            ("A", "-", "A: Expected a character in ' 0123456789', but got '-' (0x2d)."),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_printable_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= PrintableString END"
        )

        # Ok.
        datas = [("A", "09azAZ")]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            (
                "A",
                "{",
                "A: Expected a character in '"
                "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
                " '()+,-./:=?', but got '{' (0x7b).",
            )
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_ia5_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= IA5String END"
        )

        # Ok.
        datas = [("A", "09azAZ")]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            (
                "A",
                b"\x81".decode("latin-1"),
                "A: Expected a character in '................................ !\"#$%"
                "&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcde"
                "fghijklmnopqrstuvwxyz{|}~.', but got '.' (0x81).",
            )
        ]

        self.assert_encode_decode_bad(foo, datas, decode_check=False)

    def test_visible_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= VisibleString "
            "B ::= VisibleString (SIZE (2..5)) "
            'C ::= VisibleString (FROM ("a".."j" | "u".."w")) '
            "D ::= VisibleString (SIZE (2..5, ...)) "
            "END"
        )

        # Ok.
        datas = [
            ("A", "123"),
            ("B", "12"),
            ("B", "12345"),
            ("C", "abijuvw"),
            ("D", "123456"),
        ]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("B", "1", "B: Expected between 2 and 5 characters, but got 1."),
            ("B", "123456", "B: Expected between 2 and 5 characters, but got 6."),
            (
                "C",
                "k",
                "C: Expected a character in 'abcdefghijuvw', but got 'k' (0x6b).",
            ),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_utf8_string(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= UTF8String "
            "B ::= UTF8String (SIZE (2..5)) "
            "END"
        )

        # Ok.
        datas = [("A", "Hello"), ("B", "12"), ("B", "12345")]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("B", "1", "B: Expected between 2 and 5 characters, but got 1."),
            ("B", "123456", "B: Expected between 2 and 5 characters, but got 6."),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_utc_time(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= BEGIN A ::= UTCTime END"
        )

        # Ok.
        datas = [("A", datetime(1982, 1, 2, 12, 0))]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = []

        self.assert_encode_decode_bad(foo, datas)

    def test_choice(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= CHOICE { a INTEGER (1..2) } "
            "END"
        )

        # Ok.
        datas = [("A", ("a", 1))]

        self.assert_encode_decode_ok(foo, datas)

        # Not ok.
        datas = [
            ("A", ("a", 3), "A.a: Expected an integer between 1 and 2, but got 3.")
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_effective_constraints(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "A ::= INTEGER (1..2 | 4..5) "
            "B ::= IA5String (SIZE (1..4)) "
            "C ::= B (SIZE (2)) "
            'D ::= IA5String (FROM ("AB")) '
            "E ::= D (SIZE (1)) "
            "H ::= OCTET STRING (SIZE (1..4, ...)) "
            "I ::= H (SIZE (1..2)) "
            "J ::= SEQUENCE (SIZE (0..1 | 3)) OF BOOLEAN "
            "END",
        )

        datas = [
            ("A", 4),
            ("C", "ab"),
            ("E", "A"),
            ("I", b"\x01\x02"),
            ("J", []),
            ("J", [True, False, True]),
        ]

        self.assert_encode_decode_ok(foo, datas)

        datas = [
            ("A", 3, "A: Expected an integer in 1..2 | 4..5, but got 3."),
            ("C", "abc", "C: Expected between 2 and 2 characters, but got 3."),
            ("E", "AB", "E: Expected between 1 and 1 characters, but got 2."),
            (
                "E",
                "C",
                "E: Expected a character in 'AB', but got 'C' (0x43).",
            ),
            ("I", b"\x01\x02\x03", "I: Expected between 1 and 2 bytes, but got 3."),
            (
                "J",
                [True, False],
                "J: Expected a list of 0..1 | 3 elements, but got 2.",
            ),
        ]

        self.assert_encode_decode_bad(foo, datas)

    def test_with_components(self):
        for codec in ["ber", "uper"]:
            foo = fixtures.compile_string(
                "Foo DEFINITIONS AUTOMATIC TAGS ::= "
                "BEGIN "
                "S ::= SEQUENCE { a INTEGER OPTIONAL, b IA5String OPTIONAL, c INTEGER } "
                "U ::= S (WITH COMPONENTS {..., a PRESENT} "
                "       | WITH COMPONENTS {..., b PRESENT}) "
                "F ::= S (WITH COMPONENTS {a (1..5), c}) "
                "I ::= U (WITH COMPONENTS {..., c (0..9)}) "
                "P ::= S (WITH COMPONENTS {..., a PRESENT}) "
                "        (WITH COMPONENTS {..., b ABSENT}) "
                "Q ::= S (U) "
                "E ::= S (WITH COMPONENTS {..., a ABSENT}, ...) "
                "N ::= SEQUENCE { s S, t S OPTIONAL } "
                "      (WITH COMPONENTS {..., s (WITH COMPONENTS {..., a ABSENT})}) "
                "C ::= CHOICE { x INTEGER, y BOOLEAN, z NULL } "
                "V ::= C (WITH COMPONENTS {x (0..1), y}) "
                "W ::= C (WITH COMPONENTS {..., x ABSENT}) "
                "M ::= SEQUENCE { u U, w W } "
                "END",
                codec,
            )

            datas = [
                ("U", {"a": 1, "c": 0}),
                ("U", {"b": "x", "c": 0}),
                ("U", {"a": 1, "b": "x", "c": 0}),
                ("F", {"a": 5, "c": 0}),
                ("I", {"b": "x", "c": 9}),
                ("P", {"a": 1, "c": 0}),
                ("Q", {"a": 1, "c": 0}),
                ("E", {"a": 1, "c": 0}),
                ("N", {"s": {"c": 0}, "t": {"a": 1, "c": 0}}),
                ("V", ("x", 1)),
                ("V", ("y", True)),
                ("W", ("z", None)),
                ("M", {"u": {"b": "x", "c": 0}, "w": ("y", False)}),
            ]

            self.assert_encode_decode_ok(foo, datas)

            datas = [
                (
                    "U",
                    {"c": 0},
                    "U: Expected a value that satisfies one of the alternatives "
                    "of WITH COMPONENTS.",
                ),
                ("F", {"c": 0}, "F: Expected 'a' to be present."),
                ("F", {"a": 1, "b": "x", "c": 0}, "F: Expected 'b' to be absent."),
                (
                    "F",
                    {"a": 6, "c": 0},
                    "F.a: Expected an integer between 1 and 5, but got 6.",
                ),
                (
                    "I",
                    {"a": 1, "c": 10},
                    "I: Expected a value that satisfies one of the alternatives "
                    "of WITH COMPONENTS.",
                ),
                ("P", {"a": 1, "b": "x", "c": 0}, "P: Expected 'b' to be absent."),
                (
                    "Q",
                    {"c": 0},
                    "Q: Expected a value that satisfies one of the alternatives "
                    "of WITH COMPONENTS.",
                ),
                ("N", {"s": {"a": 1, "c": 0}}, "N.s: Expected 'a' to be absent."),
                ("V", ("x", 2), "V.x: Expected an integer between 0 and 1, but got 2."),
                ("V", ("z", None), "V: Expected 'z' to be absent."),
                ("W", ("x", 1), "W: Expected 'x' to be absent."),
                (
                    "M",
                    {"u": {"b": "x", "c": 0}, "w": ("x", 0)},
                    "M.w: Expected 'x' to be absent.",
                ),
            ]

            self.assert_encode_decode_bad(foo, datas)

    def test_with_component(self):
        foo = fixtures.compile_string(
            "Foo DEFINITIONS AUTOMATIC TAGS ::= "
            "BEGIN "
            "S ::= SEQUENCE { a INTEGER OPTIONAL, b IA5String OPTIONAL } "
            "L ::= SEQUENCE OF S "
            "A ::= L (WITH COMPONENT (WITH COMPONENTS {..., b (SIZE (1..3))})) "
            "T ::= SEQUENCE OF IA5String "
            'B ::= T (SIZE (1..2)) (WITH COMPONENT (SIZE (2) ^ FROM ("ab"))) '
            "R ::= SEQUENCE OF INTEGER "
            "C ::= R (WITH COMPONENT (0..9, ...)) "
            "U ::= T (WITH COMPONENT (SIZE (1)) | WITH COMPONENT (SIZE (3))) "
            "I ::= T (WITH COMPONENT (SIZE (1..3))) "
            '        (WITH COMPONENT (FROM ("ab"))) '
            "V ::= R (WITH COMPONENT (1..2) | WITH COMPONENT (5..6)) "
            "        (WITH COMPONENT (2 | 5)) "
            "END",
        )

        datas = [
            ("A", []),
            ("A", [{"a": 1}, {"b": "xyz"}]),
            ("B", ["ab", "ba"]),
            ("C", [10]),
            ("U", []),
            ("U", ["a", "b"]),
            ("U", ["abc", "xyz"]),
            ("I", ["ab", "bab"]),
            ("V", [2, 2]),
            ("V", [5]),
        ]

        self.assert_encode_decode_ok(foo, datas)

        datas = [
            (
                "A",
                [{"b": "x"}, {"b": "wxyz"}],
                "A.b: Expected between 1 and 3 characters, but got 4.",
            ),
            ("B", ["ab", "a"], "B: Expected between 2 and 2 characters, but got 1."),
            (
                "B",
                ["ac"],
                "B: Expected a character in 'ab', but got 'c' (0x63).",
            ),
            # The elements all have the size of one alternative, or all
            # that of the other.
            (
                "U",
                ["a", "abc"],
                "U: Expected elements that all satisfy one of the alternatives "
                "of WITH COMPONENT.",
            ),
            ("I", ["abab"], "I: Expected between 1 and 3 characters, but got 4."),
            ("I", ["abc"], "I: Expected a character in 'ab', but got 'c' (0x63)."),
            (
                "V",
                [2, 5],
                "V: Expected elements that all satisfy one of the alternatives "
                "of WITH COMPONENT.",
            ),
            (
                "V",
                [1],
                "V: Expected elements that all satisfy one of the alternatives "
                "of WITH COMPONENT.",
            ),
        ]

        self.assert_encode_decode_bad(foo, datas)


if __name__ == "__main__":
    unittest.main()
