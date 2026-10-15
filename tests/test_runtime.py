# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

import json
import os
import subprocess
import sys
import unittest

from tests import compiled as fixtures
from asnkitrt import runtime, syntax

QUESTION = {"id": 1, "question": "Is 1+1=3?"}
QUESTION_BER = b"0\x0e\x02\x01\x01\x16\x09Is 1+1=3?"

# Fails to import what only compiling needs, as an installation
# without it would.
WITHOUT_COMPILER = """
import sys

sys.modules["pyparsing"] = None
sys.modules["cryptography"] = None

import asnkitrt


def find_codecs():
    return sorted(
        name.split(".")[-1]
        for name in sys.modules
        if name.startswith("asnkitrt.codecs.")
        and name.split(".")[-1]
        in ["ber", "der", "gser", "jer", "oer", "per", "uper", "xer"]
    )


# A codec is not imported until used.
assert find_codecs() == [], find_codecs()

foo = asnkitrt.load_asni(sys.stdin.buffer.read())
encoded = foo.encode("Question", {"id": 1, "question": "Is 1+1=3?"})
assert foo.decode("Question", encoded) == {"id": 1, "question": "Is 1+1=3?"}
assert find_codecs() == ["ber"], find_codecs()
assert asnkitrt.codecs.jer.JERCodec
assert find_codecs() == ["ber", "jer"], find_codecs()

# Nothing of the compilers, which are other programs.
assert not any(name.startswith("fixtures") for name in sys.modules)

sys.stdout.write(encoded.hex())
"""


class RuntimeTest(unittest.TestCase):
    def compile(self, path):
        return syntax.from_asni(fixtures.compile_files_to_asni(path))

    def test_load_compiled(self):
        compiled = self.compile("tests/files/foo.asn")
        foo = runtime.load(compiled)
        self.assertEqual(foo.encode("Question", QUESTION), QUESTION_BER)
        self.assertEqual(foo.decode("Question", QUESTION_BER), QUESTION)

    def test_load_primitive(self):
        compiled = self.compile("tests/files/foo.asn")
        primitive = json.loads(json.dumps(syntax.to_primitive(compiled)))
        foo = runtime.load(syntax.from_primitive(primitive), "uper")
        self.assertEqual(
            foo.encode("Question", QUESTION),
            b"\x01\x01\x09\x93\xcd\x03\x15\x6c\x5e\xb3\x7e",
        )

    def test_load_numeric_enums(self):
        compiled = syntax.from_asni(
            fixtures.compile_string_to_asni(
                "A DEFINITIONS ::= BEGIN B ::= ENUMERATED { c, d } END"
            )
        )

        self.assertEqual(runtime.load(compiled).decode("B", b"\x0a\x01\x01"), "d")
        self.assertEqual(
            runtime.load(compiled, numeric_enums=True).decode("B", b"\x0a\x01\x01"), 1
        )

    def test_load_any_defined_by_choices(self):
        string = (
            "A DEFINITIONS ::= BEGIN "
            "B ::= SEQUENCE { a INTEGER, b ANY DEFINED BY a } "
            "END"
        )
        any_defined_by_choices = {("A", "B", "b"): {1: "BOOLEAN", 2: "NULL"}}
        expected = fixtures.compile_string(
            string, any_defined_by_choices=any_defined_by_choices
        )

        # The choices are part of the compiled specification.
        compiled = fixtures.compiled_with_choices(string, any_defined_by_choices)
        primitive = json.loads(json.dumps(syntax.to_primitive(compiled)))
        loaded = runtime.load(syntax.from_primitive(primitive))

        decoded = {"a": 1, "b": True}
        encoded = expected.encode("B", decoded)
        self.assertEqual(loaded.encode("B", decoded), encoded)
        self.assertEqual(loaded.decode("B", encoded), decoded)

    def test_load_errors(self):
        compiled = self.compile("tests/files/foo.asn")

        with self.assertRaises(fixtures.CompileError) as cm:
            runtime.load(compiled, "foo")

        self.assertEqual(str(cm.exception), "Unsupported codec 'foo'.")

        # Nothing but a compiled specification is loaded. A syntax
        # tree has to be compiled first.
        asnst = fixtures.parse_files_to_asnst("tests/files/foo.asn")

        for tree in [syntax.Range(lower=1, upper=2), syntax.from_asnst(asnst)]:
            with self.assertRaises(fixtures.CompileError) as cm:
                runtime.load(tree)

            self.assertEqual(str(cm.exception), "Expected a compiled specification.")

        for function in [runtime.load, runtime.load_asni]:
            with self.assertRaises(syntax.ConvertError) as cm:
                function(asnst)

            self.assertEqual(
                str(cm.exception), "Expected an .asni file, but got an .asnst file."
            )

    def test_load_asni_without_compiler(self):
        environment = dict(os.environ, PYTHONPATH=os.pathsep.join(sys.path))
        result = subprocess.run(
            [sys.executable, "-c", WITHOUT_COMPILER],
            input=fixtures.compile_files_to_asni("tests/files/foo.asn"),
            capture_output=True,
            env=environment,
        )

        self.assertEqual(result.returncode, 0, result.stderr.decode())
        self.assertEqual(result.stdout.decode(), QUESTION_BER.hex())


if __name__ == "__main__":
    unittest.main()
