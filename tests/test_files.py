# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

import contextlib
import dataclasses
import hashlib
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest

from tests import compiled as fixtures
from asnkitrt import runtime, syntax
from asnkitrt.syntax import cbor, wire
from asnkitrt.syntax.__main__ import main


class FilesTest(unittest.TestCase):
    maxDiff = None

    def test_cbor(self):
        # Most are examples of RFC 8949, appendix A.
        datas = [
            (0, "00"),
            (1, "01"),
            (10, "0a"),
            (23, "17"),
            (24, "1818"),
            (25, "1819"),
            (100, "1864"),
            (1000, "1903e8"),
            (1000000, "1a000f4240"),
            (1000000000000, "1b000000e8d4a51000"),
            (18446744073709551615, "1bffffffffffffffff"),
            (18446744073709551616, "c249010000000000000000"),
            (-18446744073709551616, "3bffffffffffffffff"),
            (-18446744073709551617, "c349010000000000000000"),
            (-1, "20"),
            (-10, "29"),
            (-100, "3863"),
            (-1000, "3903e7"),
            (1.1, "fb3ff199999999999a"),
            (-4.1, "fbc010666666666666"),
            (False, "f4"),
            (True, "f5"),
            (None, "f6"),
            ("", "60"),
            ("a", "6161"),
            ("IETF", "6449455446"),
            ("ü", "62c3bc"),
            ("水", "63e6b0b4"),
            ([], "80"),
            ([1, 2, 3], "83010203"),
            ([1, [2, 3], [4, 5]], "8301820203820405"),
            ({}, "a0"),
            ({"a": 1, "b": [2, 3]}, "a26161016162820203"),
            ({"a": "A", "b": "B"}, "a26161614161626142"),
        ]

        for decoded, encoded in datas:
            with self.subTest(decoded=decoded):
                self.assertEqual(cbor.dumps(decoded).hex(), encoded)
                self.assertEqual(cbor.loads(bytes.fromhex(encoded)), decoded)
                self.assertIs(type(cbor.loads(bytes.fromhex(encoded))), type(decoded))

        self.assertEqual(cbor.dumps((1, (2,))).hex(), "82018102")

    def test_cbor_decode_floats(self):
        datas = [
            ("f93e00", 1.5),
            ("f97c00", float("inf")),
            ("fa47c35000", 100000.0),
            ("fb7e37e43c8800759c", 1.0e300),
        ]

        for encoded, decoded in datas:
            self.assertEqual(cbor.loads(bytes.fromhex(encoded)), decoded)

    def test_cbor_errors(self):
        datas = [
            ("", "Out of CBOR data."),
            ("18", "Out of CBOR data."),
            ("616161", "Unexpected data after the CBOR data at offset 2."),
            ("8201", "Out of CBOR data."),
            ("6261", "Out of CBOR data."),
            ("61ff", "Bad CBOR text string."),
            ("a10102", "Unsupported CBOR map key 1."),
            ("4161", "Unsupported CBOR major type 2 with additional information 1."),
            ("c101", "Unsupported CBOR major type 6 with additional information 1."),
            ("c201", "Expected a CBOR byte string as bignum."),
            ("9f01ff", "Unsupported CBOR additional information 31 at offset 0."),
        ]

        for encoded, message in datas:
            with self.subTest(encoded=encoded):
                with self.assertRaises(syntax.ConvertError) as cm:
                    cbor.loads(bytes.fromhex(encoded))

                self.assertEqual(str(cm.exception), message)

        with self.assertRaises(syntax.ConvertError) as cm:
            cbor.dumps({1: 2})

        self.assertEqual(str(cm.exception), "Unsupported map key 1.")

        with self.assertRaises(syntax.ConvertError) as cm:
            cbor.dumps(b"1")

        self.assertEqual(str(cm.exception), "Unsupported value b'1'.")

    def test_to_asnst_from_asnst(self):
        for name in ["all_types", "information_object", "x680", "parameterization"]:
            with self.subTest(name=name):
                tree = syntax.from_asnst(
                    fixtures.parse_files_to_asnst("tests/files/{}.asn".format(name))
                )
                self.assertEqual(syntax.from_asnst(syntax.to_asnst(tree)), tree)

    def test_to_asni_from_asni(self):
        for name in ["all_types", "parameterization", "3gpp/rrc_8_6_0", "ietf/rfc5280"]:
            with self.subTest(name=name):
                compiled = syntax.from_asni(
                    fixtures.compile_files_to_asni("tests/files/{}.asn".format(name))
                )
                self.assertEqual(syntax.from_asni(syntax.to_asni(compiled)), compiled)

    def test_asni_contents(self):
        data = syntax.to_asni(
            syntax.from_asni(
                fixtures.compile_string_to_asni(
                    "Foo DEFINITIONS ::= BEGIN "
                    "A ::= SEQUENCE { a INTEGER (1..max), b BOOLEAN OPTIONAL } "
                    "max INTEGER ::= 5 "
                    "END"
                )
            ),
            syntax.Metadata(generator=None),
        )

        # The magic, the version, no flags, a nonce not in use and no
        # metadata.
        self.assertEqual(data[:12], b"ASNI\x01\x00\x00\x00\x00\x00\x00\xa0")
        self.assertEqual(
            cbor.loads(data[12:]),
            # CompiledSpecification, modules.
            [
                1,
                [
                    0,
                    # CompiledModule, name, types.
                    [
                        2,
                        "Foo",
                        [
                            0,
                            # TypeAssignment, name, type.
                            [
                                3,
                                "A",
                                # SequenceType, members.
                                [
                                    15,
                                    [
                                        0,
                                        # Member, name, type.
                                        [
                                            4,
                                            "a",
                                            # IntegerType, named_numbers,
                                            # tag, size, restricted_to.
                                            [12, None, None, None, [0, [6, 1, 5]]],
                                        ],
                                        # Member, name, type, optional.
                                        [4, "b", [10, "BOOLEAN"], True],
                                    ],
                                ],
                            ],
                        ],
                    ],
                ],
            ],
        )
        self.assertEqual(len(data), 62)

    def test_compact(self):
        datas = [
            (syntax.Range(lower=1, upper=5), [6, 1, 5]),
            (syntax.ExtensionMarker(), [8]),
            ((1, "a", syntax.Min()), [0, 1, "a", [20]]),
            ((), [0]),
            # A field that is what it is by default takes no room
            # last, and is None elsewhere.
            (syntax.Member(name="a", type=syntax.IntegerType()), [4, "a", [12]]),
            (
                syntax.Member(name="a", type=syntax.IntegerType(), default=0),
                [4, "a", [12], None, 0],
            ),
            (
                syntax.ReferencedType(name="B", module_name="C"),
                [11, "B", None, None, None, None, None, None, None, None, None, "C"],
            ),
        ]

        for node, compact in datas:
            with self.subTest(node=node):
                self.assertEqual(syntax.to_compact(node), compact)
                self.assertEqual(syntax.from_compact(compact), node)

        # Fields of a later minor version are ignored.
        self.assertEqual(
            syntax.from_compact([6, 1, 5, "later"]), syntax.Range(lower=1, upper=5)
        )

    def test_compact_numbers_of_kinds(self):
        # Every kind of node has a number of its own.
        kinds = set(wire.NODE_CLASSES) - {"Node", "Type"}
        self.assertEqual(set(wire.KIND_NUMBERS), kinds)
        self.assertEqual(len(set(wire.KIND_NUMBERS.values())), len(kinds))
        self.assertNotIn(0, wire.KIND_NUMBERS.values())

    def test_compact_fields(self):
        # Every field of every kind of node has a position of its
        # own.
        for name, field_names in wire.COMPACT_FIELD_NAMES.items():
            with self.subTest(name=name):
                fields = dataclasses.fields(wire.NODE_CLASSES[name])
                self.assertEqual(len(field_names), len(fields))
                self.assertEqual(set(field_names), set(field.name for field in fields))

        self.assertEqual(set(wire.COMPACT_FIELD_NAMES), set(wire.KIND_NUMBERS))

    def test_later_minor_version(self):
        # Fields added to nodes by a later minor version are ignored.
        asni = fixtures.compile_files_to_asni("examples/compiled/foo.asn")
        header_size = len(asni) - len(
            cbor.dumps(syntax.to_compact(syntax.from_asni(asni)))
        )
        compact = cbor.loads(asni[header_size:])
        # CompiledModule and the first Member of the first type.
        compact[1][1].extend([None, None, "later"])
        compact[1][1][2][1][2][1][1].extend([None, None, [0, "later"]])
        later = asni[:5] + b"\x01" + asni[6:header_size] + cbor.dumps(compact)
        self.assertEqual(syntax.from_asni(later), syntax.from_asni(asni))
        self.assertEqual(syntax.read_header(later).version, (1, 1))

        asnst = fixtures.parse_files_to_asnst("examples/compiled/foo.asn")
        contents = cbor.loads(asnst)
        primitive = contents["tree"]
        primitive["modules"][0]["later"] = [{"kind": "Min"}]
        primitive["modules"][0]["types"][0]["type"]["later"] = 1
        contents["version"] = [1, 99]
        contents["later"] = True
        contents["metadata"]["later"] = True
        later = cbor.dumps(contents)
        self.assertEqual(syntax.from_asnst(later), syntax.from_asnst(asnst))
        self.assertEqual(syntax.read_header(later).version, (1, 99))
        self.assertEqual(
            syntax.read_header(later).metadata, syntax.read_header(asnst).metadata
        )

        # A kind of node that is not known is not ignored.
        primitive["modules"][0]["types"][0]["type"] = {"kind": "Later"}
        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.from_asnst(cbor.dumps(contents))

        self.assertEqual(str(cm.exception), "Unsupported node 'Later'.")

    def test_compact_errors(self):
        datas = [
            ([99], "Unsupported kind of node 99."),
            ([], "Expected the number of a kind of node first in a list."),
            (["a"], "Expected the number of a kind of node first in a list."),
        ]

        for compact, message in datas:
            with self.assertRaises(syntax.ConvertError) as cm:
                syntax.from_compact(compact)

            self.assertEqual(str(cm.exception), message)

    def test_file_errors(self):
        asni = fixtures.compile_files_to_asni("examples/compiled/foo.asn")
        asnst = fixtures.parse_files_to_asnst("examples/compiled/foo.asn")
        datas = [
            (syntax.from_asni, b"", "Expected an .asni or .asnst file."),
            (syntax.from_asni, b"ASN1" + asni[4:], "Expected an .asni or .asnst file."),
            (syntax.from_asni, asni[:7], "Expected an .asni or .asnst file."),
            (
                syntax.from_asni,
                asni[:4] + b"\x02" + asni[5:],
                "Unsupported version 2.0, expected 1.x.",
            ),
            # A flag that is not known.
            (
                syntax.from_asni,
                asni[:7] + b"\x10" + asni[8:],
                "Unsupported flags 0x0010.",
            ),
            # Bound without telling to what.
            (
                syntax.from_asni,
                asni[:7] + b"\x08" + asni[8:],
                "Expected a bound file to tell what it is bound to.",
            ),
            (syntax.from_asni, asni[:10], "Expected an .asni or .asnst file."),
            (syntax.from_asni, asni[:11], "Out of CBOR data."),
            (syntax.from_asni, asni[:-1], "Out of CBOR data."),
            (
                syntax.from_asni,
                asni + b"\x00",
                "Unexpected data after the nodes at offset {}.".format(len(asni)),
            ),
            (
                syntax.from_asni,
                asnst,
                "Expected an .asni file, but got an .asnst file.",
            ),
            (
                syntax.from_asnst,
                asni,
                "Expected an .asnst file, but got an .asni file.",
            ),
            (
                syntax.from_asni,
                asni[:11] + b"\x00" + cbor.dumps([20]),
                "Expected the metadata as a map.",
            ),
            (
                syntax.from_asni,
                asni[:11] + b"\xa0" + cbor.dumps([20]),
                "Expected a compiled specification.",
            ),
            (
                syntax.from_asnst,
                cbor.dumps(dict(cbor.loads(asnst), tree={"kind": "Min"})),
                "Expected a syntax tree.",
            ),
        ]

        for function, data, message in datas:
            with self.subTest(message=message):
                with self.assertRaises(syntax.ConvertError) as cm:
                    function(data)

                self.assertEqual(str(cm.exception), message)

        # The nonce is not looked at without its flag.
        self.assertEqual(asni[8:11], b"\x00\x00\x00")
        self.assertEqual(
            syntax.from_asni(asni[:8] + b"\x12\x34\x56" + asni[11:]),
            syntax.from_asni(asni),
        )

        # An .asnst file is CBOR and nothing else.
        contents = cbor.loads(asnst)
        self.assertEqual(list(contents), ["format", "version", "metadata", "tree"])
        self.assertEqual(contents["format"], "asnst")
        self.assertEqual(contents["version"], [1, 0])
        self.assertEqual(contents["tree"]["kind"], "SyntaxTree")

        for changes, message in [
            ({"format": "asni"}, "Expected an .asni or .asnst file."),
            ({"version": [2, 0]}, "Unsupported version 2.0, expected 1.x."),
            ({"version": 1}, "Expected the version of an .asnst file as two numbers."),
            ({"metadata": []}, "Expected the metadata as a map."),
        ]:
            with self.assertRaises(syntax.ConvertError) as cm:
                syntax.from_asnst(cbor.dumps(dict(contents, **changes)))

            self.assertEqual(str(cm.exception), message)

        for data in [b"", b"\x00", cbor.dumps([1]), asnst[:-1], asnst + b"\x00"]:
            with self.assertRaises(syntax.ConvertError) as cm:
                syntax.from_asnst(data)

            self.assertEqual(str(cm.exception), "Expected an .asni or .asnst file.")

        # Any minor version of the major version is read.
        self.assertEqual(
            syntax.from_asni(asni[:5] + b"\x07" + asni[6:]), syntax.from_asni(asni)
        )

    def test_documented_kinds(self):
        """The kinds of nodes and their fields in the specification of
        the files are those of the nodes.

        """

        type_fields = [
            "tag",
            "size",
            "restricted_to",
            "permitted_alphabet",
            "with_components",
            "with_component",
            "table",
            "parameters",
            "actual_parameters",
            "module_name",
            "constraints",
        ]
        constraint_fields = [
            "size",
            "restricted_to",
            "permitted_alphabet",
            "with_components",
            "with_component",
            "containing",
        ]
        defaults = {None: "null", False: "false", (): "[]"}
        rows = []

        for name, number in wire.KIND_NUMBERS.items():
            fields = wire.COMPACT_FIELDS[name]
            names = [field.name for field in fields]
            shorthand = None

            for shorthand_fields, candidate in [
                (type_fields, "*type fields*"),
                (constraint_fields, "*constraint fields*"),
            ]:
                if names[-len(shorthand_fields) :] == shorthand_fields:
                    for field in fields[-len(shorthand_fields) :]:
                        self.assertIsNone(field.default, (name, field.name))

                    fields = fields[: -len(shorthand_fields)]
                    shorthand = candidate
                    break
            else:
                shorthand = None

            documented = []

            for field in fields:
                if field.default is dataclasses.MISSING:
                    documented.append(field.name)
                else:
                    documented.append(
                        "{} = {}".format(field.name, defaults[field.default])
                    )

            if shorthand is not None:
                documented.append(shorthand)

            rows.append((str(number), name, ", ".join(documented) or "--"))

        with open("docs/file_formats.md", "r") as fin:
            text = fin.read()

        table = text.split("<!-- kinds-begin -->")[1].split("<!-- kinds-end -->")[0]
        lines = [
            [cell.strip() for cell in line.strip().strip("|").split("|")]
            for line in table.strip().split("\n")
            if line[:3] == "| 1" or (line[:1] == "|" and line[1:].strip()[:1].isdigit())
        ]
        self.assertEqual([tuple(cells) for cells in lines], rows)

        # A list is told from a node by a number no kind has.
        self.assertNotIn(wire.TUPLE, wire.KIND_NUMBERS.values())

    def test_nonce(self):
        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        plain = fixtures.compile_string_to_asni(string)
        data = fixtures.compile_string_to_asni(string, nonce=0x123456)
        compiled = syntax.from_asni(plain)

        # The flag and the number, and nothing else.
        self.assertEqual(data[6:11], b"\x00\x04\x12\x34\x56")
        self.assertEqual(data[:6] + b"\x00\x00\x00\x00\x00" + data[11:], plain)
        self.assertEqual(syntax.read_header(data).nonce, 0x123456)
        self.assertIsNone(syntax.read_header(plain).nonce)

        # Read if it is the number expected, or one a function says
        # will do.
        seen = []

        def is_greater_than_last(nonce):
            seen.append(nonce)

            return nonce > 0x123455

        for nonce in [0x123456, is_greater_than_last]:
            self.assertEqual(syntax.from_asni(data, nonce), compiled)
            self.assertEqual(
                repr(runtime.load_asni(data, nonce=nonce).types),
                repr(runtime.load(compiled).types),
            )
            self.assertEqual(
                repr(runtime.load(data, nonce=nonce).types),
                repr(runtime.load(compiled).types),
            )

        self.assertEqual(seen, [0x123456] * 3)

        # Zero is a nonce as any other.
        zero = fixtures.compile_string_to_asni(string, nonce=0)
        self.assertEqual(zero[6:11], b"\x00\x04\x00\x00\x00")
        self.assertEqual(syntax.read_header(zero).nonce, 0)
        self.assertEqual(syntax.from_asni(zero, 0), compiled)

        # Not read otherwise, unless any nonce will do, which is not
        # so by default.
        datas = [
            (
                data,
                {},
                "The file has the nonce 1193046 to check, but no nonce is expected.",
            ),
            (data, {"nonce": 0x123457}, "Expected the nonce 1193047, but got 1193046."),
            (
                data,
                {"nonce": lambda nonce: nonce > 0x123456},
                "The nonce 1193046 of the file is not accepted.",
            ),
            (zero, {}, "The file has the nonce 0 to check, but no nonce is expected."),
            (plain, {"nonce": 0}, "Expected a file with a nonce to check."),
            (
                plain,
                {"nonce": lambda nonce: True},
                "Expected a file with a nonce to check.",
            ),
        ]

        for encoded, arguments, message in datas:
            with self.subTest(message=message):
                for function in [syntax.from_asni, runtime.load_asni, runtime.load]:
                    with self.assertRaises(syntax.ConvertError) as cm:
                        function(encoded, **arguments)

                    self.assertEqual(str(cm.exception), message)

                self.assertEqual(
                    syntax.from_asni(encoded, accept_any_nonce=True, **arguments),
                    compiled,
                )

        # From files, too.
        encoded = fixtures.compile_files_to_asni(
            "examples/compiled/foo.asn", nonce=0xFFFFFF
        )
        self.assertEqual(syntax.read_header(encoded).nonce, 0xFFFFFF)
        self.assertEqual(
            syntax.from_asni(encoded, 0xFFFFFF),
            syntax.from_asni_file("examples/compiled/foo.asni"),
        )

        with tempfile.TemporaryDirectory() as directory:
            filename = os.path.join(directory, "a.asni")

            with open(filename, "wb") as fout:
                fout.write(data)

            self.assertEqual(syntax.from_asni_file(filename, 0x123456), compiled)
            runtime.load_asni_file(filename, nonce=0x123456)

            with self.assertRaises(syntax.ConvertError):
                runtime.load_asni_file(filename)

        # A nonce is a number of 24 bits.
        for nonce, message in [
            (-1, "Expected a nonce of 0 to 16777215, but got -1."),
            (0x1000000, "Expected a nonce of 0 to 16777215, but got 16777216."),
            ("1", "Expected a nonce as an integer, but got '1'."),
            (True, "Expected a nonce as an integer, but got True."),
        ]:
            with self.assertRaises(syntax.ConvertError) as cm:
                fixtures.compile_string_to_asni(string, nonce=nonce)

            self.assertEqual(str(cm.exception), message)

    def test_bound_to(self):
        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        metadata = syntax.Metadata(generator=None)
        plain = fixtures.compile_string_to_asni(string, metadata=metadata)
        data = fixtures.compile_string_to_asni(
            string, metadata=metadata, bound_to="Thing 2"
        )
        compiled = syntax.from_asni(plain)

        # The flag, and what it is bound to in the metadata.
        self.assertEqual(data[6:8], b"\x00\x08")
        self.assertEqual(cbor.loads_first(data, 11)[0], {"bound_to": "Thing 2"})
        self.assertEqual(syntax.read_header(data).metadata.bound_to, "Thing 2")
        self.assertIsNone(syntax.read_header(plain).metadata.bound_to)
        self.assertEqual(
            syntax.to_asni(
                compiled, syntax.Metadata(generator=None, bound_to="Thing 2")
            ),
            data,
        )

        # Read if bound to what is expected.
        self.assertEqual(syntax.from_asni(data, bound_to="Thing 2"), compiled)
        self.assertEqual(
            repr(runtime.load_asni(data, bound_to="Thing 2").types),
            repr(runtime.load(compiled).types),
        )
        self.assertEqual(
            repr(runtime.load(data, bound_to="Thing 2").types),
            repr(runtime.load(compiled).types),
        )

        # Not read otherwise, unless any binding will do, which is
        # not so by default.
        datas = [
            (data, {}, "The file is bound to 'Thing 2', but no binding is expected."),
            (
                data,
                {"bound_to": "Thing 1"},
                "Expected a file bound to 'Thing 1', but it is bound to 'Thing 2'.",
            ),
            (
                data,
                {"bound_to": "Thing 2 "},
                "Expected a file bound to 'Thing 2 ', but it is bound to 'Thing 2'.",
            ),
            (
                data,
                {"bound_to": ""},
                "Expected a file bound to '', but it is bound to 'Thing 2'.",
            ),
            (plain, {"bound_to": "Thing 2"}, "Expected a file bound to 'Thing 2'."),
        ]

        for encoded, arguments, message in datas:
            with self.subTest(message=message):
                for function in [syntax.from_asni, runtime.load_asni, runtime.load]:
                    with self.assertRaises(syntax.ConvertError) as cm:
                        function(encoded, **arguments)

                    self.assertEqual(str(cm.exception), message)

                self.assertEqual(
                    syntax.from_asni(encoded, accept_any_binding=True, **arguments),
                    compiled,
                )

        # What a file that is not bound says it is bound to is not
        # looked at.
        unbound = data[:6] + b"\x00\x00" + data[8:]
        self.assertIsNone(syntax.read_header(unbound).metadata.bound_to)
        self.assertEqual(syntax.from_asni(unbound), compiled)

        # At most 255 bytes, which is not as many characters.
        longest = "\u00e9" * 127 + "e"
        encoded = fixtures.compile_string_to_asni(string, bound_to=longest)
        self.assertEqual(syntax.from_asni(encoded, bound_to=longest), compiled)
        message = "Expected what the file is bound to as at most 255 bytes as UTF-8."

        with self.assertRaises(syntax.ConvertError) as cm:
            fixtures.compile_string_to_asni(string, bound_to="\u00e9" * 128)

        self.assertEqual(str(cm.exception), message)

        # Also when found in a file, whatever is expected.
        for arguments in [{}, {"bound_to": "a" * 256}, {"accept_any_binding": True}]:
            with self.assertRaises(syntax.ConvertError) as cm:
                syntax.from_asni(
                    data[:11]
                    + cbor.dumps({"bound_to": "a" * 256})
                    + data[11 + len(cbor.dumps({"bound_to": "Thing 2"})) :],
                    **arguments,
                )

            self.assertEqual(str(cm.exception), message)

        # With a nonce as well, both are checked.
        both = fixtures.compile_string_to_asni(string, nonce=5, bound_to="Thing 2")
        self.assertEqual(both[6:11], b"\x00\x0c\x00\x00\x05")
        self.assertEqual(syntax.from_asni(both, nonce=5, bound_to="Thing 2"), compiled)
        self.assertEqual(
            syntax.from_asni(both, accept_any_nonce=True, accept_any_binding=True),
            compiled,
        )

        for arguments in [
            {"nonce": 5},
            {"bound_to": "Thing 2"},
            {"nonce": 5, "accept_any_nonce": True},
            {"bound_to": "Thing 2", "accept_any_binding": True},
        ]:
            with self.assertRaises(syntax.ConvertError):
                syntax.from_asni(both, **arguments)

        # A syntax tree is for plugins, and has neither.
        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.to_asnst(
                syntax.from_asnst(fixtures.parse_string_to_asnst(string)),
                syntax.Metadata(bound_to="Thing 2"),
            )

        self.assertEqual(
            str(cm.exception), "Only an .asni file can be bound to something."
        )

        with self.assertRaises(syntax.ConvertError) as cm:
            fixtures.compile_string_to_asni(string, bound_to=5)

        self.assertEqual(
            str(cm.exception),
            "Expected what the file is bound to as a string, but got 5.",
        )

    def make_keys(self):
        """Returns the keys of a signer and a key to encrypt with, each
        with its identifier.

        """

        from cryptography.hazmat.primitives import serialization
        from cryptography.hazmat.primitives.asymmetric.ed25519 import (
            Ed25519PrivateKey,
        )

        private_key = bytes(range(32))
        public_key = (
            Ed25519PrivateKey.from_private_bytes(private_key)
            .public_key()
            .public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        )

        return (
            (b"signing key 0001", private_key),
            {b"signing key 0001": public_key},
            (b"encrypt key 0001", bytes(range(32, 64))),
        )

    def test_signed(self):
        from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        signing_key, verification_keys, _ = self.make_keys()
        plain = fixtures.compile_string_to_asni(string)
        data = fixtures.compile_string_to_asni(string, signing_key=signing_key)
        compiled = syntax.from_asni(plain)

        # The flag, the key and the signature, and nothing else.
        self.assertEqual(data[6:8], b"\x00\x01")
        self.assertEqual(data[11:27], b"signing key 0001")
        self.assertEqual(len(data), len(plain) + 80)
        self.assertEqual(data[:6] + b"\x00\x00" + data[8:11] + data[91:], plain)

        # The signature is that of the SHA-256 of all but itself.
        Ed25519PublicKey.from_public_bytes(verification_keys[signing_key[0]]).verify(
            data[27:91], hashlib.sha256(data[:27] + data[91:]).digest()
        )

        header = syntax.read_header(data)
        self.assertEqual(header.signed_by, b"signing key 0001")
        self.assertIsNone(header.encrypted_with)
        self.assertIsNone(syntax.read_header(plain).signed_by)

        # Read if signed by a key that is trusted.
        self.assertEqual(
            syntax.from_asni(data, verification_keys=verification_keys), compiled
        )

        for function in [runtime.load_asni, runtime.load]:
            self.assertEqual(
                repr(function(data, verification_keys=verification_keys).types),
                repr(runtime.load(compiled).types),
            )

        # Not read otherwise.
        other_keys = {b"signing key 0002": verification_keys[b"signing key 0001"]}
        wrong_keys = {b"signing key 0001": bytes(32)}
        datas = [
            (data, {}, "The file is signed, but no keys to verify it with are given."),
            (
                data,
                {"verification_keys": {}},
                "The file is signed with the key 7369676e696e67206b65792030303031, "
                "which is not among those given.",
            ),
            (
                data,
                {"verification_keys": other_keys},
                "The file is signed with the key 7369676e696e67206b65792030303031, "
                "which is not among those given.",
            ),
            (
                data,
                {"verification_keys": wrong_keys},
                "The signature of the file is wrong.",
            ),
            (
                data,
                {"verification_keys": {b"signing key 0001": b"short"}},
                "Expected the key 7369676e696e67206b65792030303031 as 32 bytes.",
            ),
            (
                plain,
                {"verification_keys": verification_keys},
                "Expected a signed file.",
            ),
            (plain, {"verification_keys": {}}, "Expected a signed file."),
        ]

        for encoded, arguments, message in datas:
            with self.subTest(message=message):
                for function in [syntax.from_asni, runtime.load_asni, runtime.load]:
                    with self.assertRaises(syntax.ConvertError) as cm:
                        function(encoded, **arguments)

                    self.assertEqual(str(cm.exception), message)

        # Any change to the file shows, wherever it is.
        for offset in [0, 5, 7, 8, 10, 11, 26, 27, 90, 91, len(data) - 1]:
            changed = bytearray(data)
            changed[offset] ^= 0x10

            with self.assertRaises(syntax.ConvertError):
                syntax.from_asni(bytes(changed), verification_keys=verification_keys)

        for changed in [data[:-1], data + b"\x00", data[:60]]:
            with self.assertRaises(syntax.ConvertError):
                syntax.from_asni(changed, verification_keys=verification_keys)

        # Without the flag it is no .asni file at all, as the
        # signature is where the metadata is expected.
        with self.assertRaises(syntax.ConvertError):
            syntax.from_asni(data[:6] + b"\x00\x00" + data[8:])

        # Only looked at, the signature is not checked.
        self.assertEqual(syntax.from_asni(data, accept_any_signature=True), compiled)
        self.assertEqual(syntax.from_asni(plain, accept_any_signature=True), compiled)
        self.assertEqual(
            syntax.from_asni(
                data, verification_keys=wrong_keys, accept_any_signature=True
            ),
            compiled,
        )

        # The nonce and what the file is bound to are signed.
        data = fixtures.compile_string_to_asni(
            string, nonce=7, bound_to="Thing 2", signing_key=signing_key
        )
        self.assertEqual(data[6:11], b"\x00\x0d\x00\x00\x07")
        arguments = {
            "nonce": 7,
            "bound_to": "Thing 2",
            "verification_keys": verification_keys,
        }
        self.assertEqual(syntax.from_asni(data, **arguments), compiled)
        changed = data[:10] + b"\x08" + data[11:]

        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.from_asni(changed, **dict(arguments, nonce=8))

        self.assertEqual(str(cm.exception), "The signature of the file is wrong.")

        # A key is its identifier of 16 bytes and 32 bytes of key.
        for key in [
            (b"short", bytes(32)),
            (bytes(16), bytes(31)),
            (bytes(16), bytes(32), b""),
            [bytes(16), bytes(32)],
            ("signing key 0001", bytes(32)),
            bytes(48),
        ]:
            with self.assertRaises(syntax.ConvertError) as cm:
                fixtures.compile_string_to_asni(string, signing_key=key)

            self.assertEqual(
                str(cm.exception),
                "Expected the key to sign with as its identifier of 16 bytes and "
                "the key of 32 bytes.",
            )

    def test_encrypted(self):
        from cryptography.hazmat.primitives.ciphers.aead import AESGCM

        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        signing_key, verification_keys, encryption_key = self.make_keys()
        decryption_keys = {encryption_key[0]: encryption_key[1]}
        plain = fixtures.compile_string_to_asni(string)
        data = fixtures.compile_string_to_asni(string, encryption_key=encryption_key)
        compiled = syntax.from_asni(plain)

        # The flag, the key, 12 bytes that are never the same twice
        # and the tag, before the metadata and the nodes encrypted.
        self.assertEqual(data[6:8], b"\x00\x02")
        self.assertEqual(data[11:27], b"encrypt key 0001")
        self.assertEqual(len(data), len(plain) + 44)
        self.assertNotIn(b"generator", data)
        self.assertNotEqual(
            data[27:39],
            fixtures.compile_string_to_asni(string, encryption_key=encryption_key)[
                27:39
            ],
        )

        # AES-256 in Galois/Counter Mode, with all before the tag as
        # what is not encrypted but can not be changed either.
        self.assertEqual(
            AESGCM(encryption_key[1]).decrypt(
                data[27:39], data[55:] + data[39:55], data[:39]
            ),
            plain[11:],
        )

        header = syntax.read_header(data)
        self.assertEqual(header.encrypted_with, b"encrypt key 0001")
        self.assertIsNone(header.signed_by)
        self.assertIsNone(header.metadata)
        self.assertIsNone(syntax.read_header(plain).encrypted_with)

        # Read with the key.
        self.assertEqual(
            syntax.from_asni(data, decryption_keys=decryption_keys), compiled
        )

        for function in [runtime.load_asni, runtime.load]:
            self.assertEqual(
                repr(function(data, decryption_keys=decryption_keys).types),
                repr(runtime.load(compiled).types),
            )

        # Not read otherwise.
        failed = (
            "The file is not what was encrypted, or the key is not the one it "
            "was encrypted with."
        )
        datas = [
            (
                data,
                {},
                "The file is encrypted, but no keys to decrypt it with are given.",
            ),
            (
                data,
                {"decryption_keys": {b"encrypt key 0002": encryption_key[1]}},
                "The file is encrypted with the key "
                "656e6372797074206b65792030303031, which is not among those given.",
            ),
            (data, {"decryption_keys": {encryption_key[0]: bytes(32)}}, failed),
            (
                plain,
                {"decryption_keys": decryption_keys},
                "Expected an encrypted file.",
            ),
            (data, {"accept_any_signature": True}, None),
            (data, {"accept_any_nonce": True, "accept_any_binding": True}, None),
        ]

        for encoded, arguments, message in datas:
            with self.subTest(arguments=arguments):
                for function in [syntax.from_asni, runtime.load_asni, runtime.load]:
                    with self.assertRaises(syntax.ConvertError) as cm:
                        function(encoded, **arguments)

                    if message is not None:
                        self.assertEqual(str(cm.exception), message)

        # Any change to the file shows, wherever it is.
        for offset in [5, 7, 8, 10, 27, 38, 39, 54, 55, len(data) - 1]:
            changed = bytearray(data)
            changed[offset] ^= 0x10

            with self.assertRaises(syntax.ConvertError):
                syntax.from_asni(bytes(changed), decryption_keys=decryption_keys)

        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.from_asni(data[:-1], decryption_keys=decryption_keys)

        self.assertEqual(str(cm.exception), failed)

        # Signed as well, with a nonce and bound to something: what
        # it is bound to is not seen without the key.
        data = fixtures.compile_string_to_asni(
            string,
            nonce=7,
            bound_to="Thing 2",
            signing_key=signing_key,
            encryption_key=encryption_key,
        )
        self.assertEqual(data[6:11], b"\x00\x0f\x00\x00\x07")
        self.assertEqual(data[11:27], b"signing key 0001")
        self.assertEqual(data[91:107], b"encrypt key 0001")
        self.assertNotIn(b"Thing 2", data)
        header = syntax.read_header(data)
        self.assertEqual(header.signed_by, b"signing key 0001")
        self.assertEqual(header.encrypted_with, b"encrypt key 0001")
        self.assertEqual(header.nonce, 7)
        arguments = {
            "nonce": 7,
            "bound_to": "Thing 2",
            "verification_keys": verification_keys,
            "decryption_keys": decryption_keys,
        }
        self.assertEqual(syntax.from_asni(data, **arguments), compiled)

        for name in arguments:
            with self.assertRaises(syntax.ConvertError):
                syntax.from_asni(
                    data, **{key: arguments[key] for key in arguments if key != name}
                )

        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.from_asni(data, **dict(arguments, bound_to="Thing 1"))

        self.assertEqual(
            str(cm.exception),
            "Expected a file bound to 'Thing 1', but it is bound to 'Thing 2'.",
        )

        # The signature is checked first, without the key to decrypt
        # with.
        changed = data[:-1] + bytes([data[-1] ^ 1])

        with self.assertRaises(syntax.ConvertError) as cm:
            syntax.from_asni(changed, verification_keys=verification_keys)

        self.assertEqual(str(cm.exception), "The signature of the file is wrong.")

        with self.assertRaises(syntax.ConvertError) as cm:
            fixtures.compile_string_to_asni(string, encryption_key=(b"k", bytes(32)))

        self.assertEqual(
            str(cm.exception),
            "Expected the key to encrypt with as its identifier of 16 bytes and "
            "the key of 32 bytes.",
        )

    def test_signed_and_encrypted_without_cryptography(self):
        """Nothing but a signed or encrypted file needs the
        cryptography package, and one that does tells so.

        """

        signing_key, verification_keys, encryption_key = self.make_keys()
        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        script = """
import sys

sys.modules["cryptography"] = None

from tests import compiled as fixtures
from asnkitrt import runtime, syntax

string = sys.argv[1]
plain = fixtures.compile_string_to_asni(string, nonce=1, bound_to="a")
runtime.load_asni(plain, nonce=1, bound_to="a")
files = dict(zip(["signed", "encrypted"], sys.stdin.buffer.read().split(b"\\n--\\n")))
key = (bytes(16), bytes(32))
keys = {b"signing key 0001": bytes(32), b"encrypt key 0001": bytes(32)}
messages = []

for function in [
    lambda: fixtures.compile_string_to_asni(string, signing_key=key),
    lambda: fixtures.compile_string_to_asni(string, encryption_key=key),
    lambda: runtime.load_asni(files["signed"], verification_keys=keys),
    lambda: runtime.load_asni(files["encrypted"], decryption_keys=keys),
]:
    try:
        function()
    except syntax.ConvertError as e:
        messages.append(str(e))

print("\\n".join(messages))
print(syntax.read_header(files["signed"]).signed_by.decode())
print(syntax.read_header(files["encrypted"]).encrypted_with.decode())
"""
        signed = fixtures.compile_string_to_asni(string, signing_key=signing_key)
        encrypted = fixtures.compile_string_to_asni(
            string, encryption_key=encryption_key
        )
        self.assertNotIn(b"\n--\n", signed + encrypted)
        result = subprocess.run(
            [sys.executable, "-c", script, string],
            input=signed + b"\n--\n" + encrypted,
            capture_output=True,
            env=dict(os.environ, PYTHONPATH=os.pathsep.join(sys.path)),
        )
        self.assertEqual(result.returncode, 0, result.stderr.decode())
        signed_message = (
            "A signed file needs the cryptography package, which is installed "
            "with the extra 'signing' of this package."
        )
        encrypted_message = (
            "An encrypted file needs the cryptography package, which is "
            "installed with the extra 'encryption' of this package."
        )
        self.assertEqual(
            result.stdout.decode().splitlines(),
            [
                signed_message,
                encrypted_message,
                signed_message,
                encrypted_message,
                "signing key 0001",
                "encrypt key 0001",
            ],
        )

    def test_examples(self):
        """The example files are those compiled from the specifications
        next to them. Compile them again if the nodes or the version
        of the package change.

        """

        for name in ["foo", "information_object", "rfc5280"]:
            path = "examples/compiled/{}".format(name)

            with self.subTest(name=name, kind="asnst"):
                with open(path + ".asnst", "rb") as fin:
                    encoded = fin.read()

                self.assertEqual(fixtures.parse_files_to_asnst(path + ".asn"), encoded)

            with self.subTest(name=name, kind="asni"):
                with open(path + ".asni", "rb") as fin:
                    encoded = fin.read()

                self.assertEqual(fixtures.compile_files_to_asni(path + ".asn"), encoded)

    def test_load_examples(self):
        decoded = {"id": 1, "question": "Is 1+1=3?"}
        encoded = b"\x01\x01\x09\x93\xcd\x03\x15\x6c\x5e\xb3\x7e"

        with open("examples/compiled/foo.asni", "rb") as fin:
            asni = fin.read()

        for foo in [
            runtime.load_asni_file("examples/compiled/foo.asni", "uper"),
            runtime.load_asni(asni, "uper"),
            runtime.load_asni(bytearray(asni), "uper"),
            runtime.load(asni, "uper"),
            runtime.load(syntax.from_asni_file("examples/compiled/foo.asni"), "uper"),
            fixtures.compile_files("examples/compiled/foo.asn", "uper"),
        ]:
            self.assertEqual(foo.encode("Question", decoded), encoded)
            self.assertEqual(foo.decode("Question", encoded), decoded)

    def test_corpus(self):
        def read(name):
            with open("examples/corpus/" + name, "r") as fin:
                return bytes.fromhex(fin.read().strip())

        verification_keys = {
            bytes.fromhex("d543cae4811fc8b033704dc6e7896ff7"): read("signing.pub")
        }
        decryption_keys = {
            bytes.fromhex("16cde2f77598fc5472df7570a510f54d"): read("encryption.key")
        }
        decoded = {"id": 1, "question": "Is 1+1=3?"}
        names = []

        for signed in [False, True]:
            for encrypted in [False, True]:
                for bound in [False, True]:
                    name = "-".join(
                        [
                            word
                            for word, is_set in [
                                ("signed", signed),
                                ("encrypted", encrypted),
                                ("bound", bound),
                            ]
                            if is_set
                        ]
                        or ["plain"]
                    )
                    names.append(name)

                    with self.subTest(name=name):
                        path = "examples/corpus/{}.asni".format(name)

                        with open(path, "rb") as fin:
                            header = syntax.read_header(fin.read())

                        self.assertEqual(header.signed_by is not None, signed)
                        self.assertEqual(header.encrypted_with is not None, encrypted)

                        foo = runtime.load_asni_file(
                            path,
                            "ber",
                            bound_to="corpus" if bound else None,
                            accept_any_binding=not bound,
                            verification_keys=verification_keys if signed else None,
                            accept_any_signature=not signed,
                            decryption_keys=decryption_keys if encrypted else None,
                        )
                        self.assertEqual(
                            foo.decode("Question", foo.encode("Question", decoded)),
                            decoded,
                        )

        self.assertEqual(
            sorted(names),
            sorted(
                os.path.splitext(name)[0]
                for name in os.listdir("examples/corpus")
                if name.endswith(".asni")
            ),
        )

    def test_load_real_specification_example(self):
        compiled = fixtures.compile_files("examples/compiled/rfc5280.asn", "der")
        decoded = {
            "algorithm": "1.2.840.113549.1.1.11",
            "parameters": b"\x05\x00",
        }

        for loaded in [
            runtime.load_asni_file("examples/compiled/rfc5280.asni", "der"),
        ]:
            self.assertEqual(repr(loaded.types), repr(compiled.types))
            encoded = loaded.encode("AlgorithmIdentifier", decoded)
            self.assertEqual(encoded, compiled.encode("AlgorithmIdentifier", decoded))
            self.assertEqual(loaded.decode("AlgorithmIdentifier", encoded), decoded)

    def test_print_as_json(self):
        def run(arguments):
            stdout = io.StringIO()

            with contextlib.redirect_stdout(stdout):
                main(arguments)

            return stdout.getvalue()

        tree = syntax.from_asnst_file("examples/compiled/foo.asnst")
        compiled = syntax.from_asni_file("examples/compiled/foo.asni")

        path = "examples/compiled/foo.asnst"
        self.assertEqual(run([path]), syntax.to_json(tree) + "\n")
        self.assertEqual(syntax.from_primitive(json.loads(run([path]))), tree)

        self.assertEqual(
            run(["examples/compiled/foo.asni"]), syntax.to_json(compiled) + "\n"
        )
        self.assertEqual(
            run(["--indent", "1", "examples/compiled/foo.asni"]),
            syntax.to_json(compiled, 1) + "\n",
        )

        header = json.loads(run(["--header", "examples/compiled/foo.asni"]))
        self.assertEqual(header["kind"], "asni")
        self.assertEqual(header["version"], [1, 0])
        self.assertEqual(header["metadata"]["sources"][0]["name"], "foo.asn")


if __name__ == "__main__":
    unittest.main()
