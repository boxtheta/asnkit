# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

import contextlib
import dataclasses
import io
import json
import os
import unittest
import zlib

from tests import compiled as fixtures
from asnkitrt import runtime, syntax
from asnkitrt.syntax import cbor, wire
from asnkitrt.syntax.__main__ import main


CORRUPT = "The CRC-32 of the file does not match: the file is corrupt."


def asni_of(rest: bytes, version: bytes = b"\x02\x00") -> bytes:
    """An .asni file of the version with given metadata and nodes, and
    their CRC-32 in its header.

    """

    return b"ASNI" + version + zlib.crc32(rest).to_bytes(4, "big") + rest


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

    def test_pragmas(self):
        # The comments --@ key = value before a module, an assignment
        # or a member are pairs of the node in the syntax tree, and are
        # not in the compiled specification.
        specification = (
            "--@ rpc.package = greeter.v1\n"
            "Greeter DEFINITIONS AUTOMATIC TAGS ::= BEGIN\n"
            "--@ rpc.message\n"
            "--@ go-name = HelloRequest\n"
            "HelloRequest ::= SEQUENCE {\n"
            "    --@ json.name = the_name\n"
            "    name UTF8String,\n"
            "    age INTEGER OPTIONAL\n"
            "}\n"
            "END\n"
        )
        tree = syntax.from_asnst(fixtures.parse_string_to_asnst(specification))
        module = tree.modules[0]
        request = module.types[0]

        self.assertEqual(module.pragmas, (("rpc.package", "greeter.v1"),))
        self.assertEqual(
            dict(request.pragmas), {"rpc.message": "", "go-name": "HelloRequest"}
        )
        self.assertEqual(request.type.members[0].pragmas, (("json.name", "the_name"),))
        self.assertIsNone(request.type.members[1].pragmas)
        self.assertEqual(syntax.from_asnst(syntax.to_asnst(tree)), tree)

        compiled = syntax.from_asni(fixtures.compile_string_to_asni(specification))
        self.assertIsNone(compiled.modules[0].types[0].pragmas)

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

        # The magic, the version, the CRC-32 of the rest, and no metadata.
        self.assertEqual(data[:6], b"ASNI\x02\x00")
        self.assertEqual(data[6:10], zlib.crc32(data[10:]).to_bytes(4, "big"))
        self.assertEqual(data[10:11], b"\xa0")
        self.assertEqual(
            cbor.loads(data[11:]),
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
        self.assertEqual(len(data), 61)

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
        later = asni_of(asni[10:header_size] + cbor.dumps(compact), b"\x02\x01")
        self.assertEqual(syntax.from_asni(later), syntax.from_asni(asni))
        self.assertEqual(syntax.read_header(later).version, (2, 1))

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
            (syntax.from_asni, asni[:9], "Expected an .asni or .asnst file."),
            # Version 1, which had a nonce, a signature and encryption.
            (
                syntax.from_asni,
                asni[:4] + b"\x01" + asni[5:],
                "Unsupported version 1.0, expected 2.x.",
            ),
            (
                syntax.from_asni,
                asni[:4] + b"\x03" + asni[5:],
                "Unsupported version 3.0, expected 2.x.",
            ),
            # Anything changed after the header, or cut short, or added
            # to, and the CRC-32 itself changed, before any CBOR is read.
            (syntax.from_asni, asni[:-1] + bytes([asni[-1] ^ 1]), CORRUPT),
            (syntax.from_asni, asni[:6] + bytes([asni[6] ^ 0x80]) + asni[7:], CORRUPT),
            (syntax.from_asni, asni[:10], CORRUPT),
            (syntax.from_asni, asni[:-1], CORRUPT),
            (syntax.from_asni, asni + b"\x00", CORRUPT),
            # The same with the CRC-32 made to match, which the CBOR catches.
            (syntax.from_asni, asni_of(b""), "Out of CBOR data."),
            (syntax.from_asni, asni_of(asni[10:-1]), "Out of CBOR data."),
            (
                syntax.from_asni,
                asni_of(asni[10:] + b"\x00"),
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
                asni_of(b"\x00" + cbor.dumps([20])),
                "Expected the metadata as a map.",
            ),
            (
                syntax.from_asni,
                asni_of(b"\xa0" + cbor.dumps([20])),
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

        # The metadata follows the CRC-32.
        self.assertEqual(asni[:6], b"ASNI\x02\x00")
        self.assertEqual(asni_of(asni[10:]), asni)
        self.assertIsInstance(cbor.loads_first(asni, 10)[0], dict)

        # An .asnst file is CBOR and nothing else.
        contents = cbor.loads(asnst)
        self.assertEqual(list(contents), ["format", "version", "metadata", "tree"])
        self.assertEqual(contents["format"], "asnst")
        self.assertEqual(contents["version"], [1, 1])
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

    def test_crc32(self):
        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        data = fixtures.compile_string_to_asni(string)
        compiled = syntax.from_asni(data)

        # That of zlib, of the bytes after the header, most significant
        # byte first.
        self.assertEqual(zlib.crc32(b"123456789"), 0xCBF43926)
        self.assertEqual(int.from_bytes(data[6:10], "big"), zlib.crc32(data[10:]))

        # Every bit flipped after the magic and the version is caught,
        # in the CRC-32 itself too, also by read_header().
        for index in range(6, len(data)):
            for bit in range(8):
                corrupt = bytearray(data)
                corrupt[index] ^= 1 << bit

                for function in [
                    syntax.from_asni,
                    syntax.read_header,
                    runtime.load_asni,
                    runtime.load,
                ]:
                    with self.assertRaises(syntax.ConvertError) as cm:
                        function(bytes(corrupt))

                    self.assertEqual(str(cm.exception), CORRUPT)

        # The minor version is not covered: a later one is read.
        self.assertEqual(syntax.from_asni(data[:5] + b"\x09" + data[6:]), compiled)

        # What a file of 2.0.0-rc4 told it was bound to is metadata that
        # is not known, and is ignored.
        metadata, offset = cbor.loads_first(data, 10)
        bound = asni_of(cbor.dumps(dict(metadata, bound_to="Thing 2")) + data[offset:])
        self.assertEqual(syntax.from_asni(bound), compiled)
        self.assertEqual(syntax.read_header(bound), syntax.read_header(data))

    def test_removed_options(self):
        """The options of what an .asni file no longer has, a nonce, a
        signature and encryption, are not taken, so that whoever still
        gives them is not given a file that is not what they asked for.

        """

        string = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"
        data = fixtures.compile_string_to_asni(string)
        compiled = syntax.from_asni(data)

        for name in [
            "nonce",
            "accept_any_nonce",
            "verification_keys",
            "accept_any_signature",
            "decryption_keys",
            "bound_to",
            "accept_any_binding",
        ]:
            with self.subTest(name=name):
                for function in [syntax.from_asni, runtime.load_asni, runtime.load]:
                    with self.assertRaises(TypeError):
                        function(data, **{name: None})

        for name in ["nonce", "signing_key", "encryption_key"]:
            with self.subTest(name=name):
                with self.assertRaises(TypeError):
                    syntax.to_asni(compiled, **{name: None})

        # Nor given in their place, as the options after the data are
        # given by name.
        with self.assertRaises(TypeError):
            syntax.from_asni(data, 5)

        with self.assertRaises(TypeError):
            syntax.to_asni(compiled, None, 5)

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
        decoded = {"id": 1, "question": "Is 1+1=3?"}

        with open("examples/corpus/plain.asni", "rb") as fin:
            header = syntax.read_header(fin.read())

        self.assertEqual(header.version, (2, 0))
        foo = runtime.load_asni_file("examples/corpus/plain.asni", "ber")
        self.assertEqual(
            foo.decode("Question", foo.encode("Question", decoded)), decoded
        )

        with self.assertRaises(syntax.ConvertError) as cm:
            runtime.load_asni_file("examples/corpus/corrupt.asni", "ber")

        self.assertEqual(str(cm.exception), CORRUPT)

        self.assertEqual(
            sorted(
                os.path.splitext(name)[0]
                for name in os.listdir("examples/corpus")
                if name.endswith(".asni")
            ),
            ["corrupt", "plain"],
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
        self.assertEqual(header["version"], [2, 0])
        self.assertEqual(header["metadata"]["sources"][0]["name"], "foo.asn")
        self.assertEqual(list(header), ["kind", "version", "metadata"])


if __name__ == "__main__":
    unittest.main()
