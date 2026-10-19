# Copyright (c) 2025 BoxTheta
# SPDX-License-Identifier: Apache-2.0

"""Compiler mappings consumed by the Python runtime and generated models."""

import os
from pathlib import Path
import subprocess
import tempfile
import unittest

import asnkitrt
from asnkit_gen_python import generate

from tests.compiled import _binary
from tests.test_gen_python import load_module


@unittest.skipUnless(_binary(), "asnkitcc is needed for compiler configuration tests")
class AnyDefinedByCompilerTest(unittest.TestCase):
    def test_runtime_and_generated_models(self):
        with tempfile.TemporaryDirectory() as folder:
            root = Path(folder)
            (root / "m.asn").write_text("""
M DEFINITIONS ::= BEGIN
K ::= ENUMERATED { text(1), number(2) }
T ::= SEQUENCE { kind INTEGER, payload ANY DEFINED BY kind }
E ::= SEQUENCE { kind K, payload ANY DEFINED BY kind }
O ::= SEQUENCE { kind OBJECT IDENTIFIER, payload ANY DEFINED BY kind }
END
""")
            (root / "choices.toml").write_text("""
[[any_defined_by]]
type = "M.T"
member = "payload"
choices = { "1" = "UTF8String", "2" = "INTEGER" }
[[any_defined_by]]
type = "M.E"
member = "payload"
choices = { text = "UTF8String", number = "INTEGER" }
[[any_defined_by]]
type = "M.O"
member = "payload"
choices = { "1.2.3" = "UTF8String" }
""")
            subprocess.run(
                [
                    os.path.abspath(_binary()),
                    "compile",
                    "--any-defined-by",
                    "choices.toml",
                    "-o",
                    "m.asni",
                    "m.asn",
                ],
                cwd=root,
                check=True,
                capture_output=True,
            )
            asni = (root / "m.asni").read_bytes()
            generated = load_module(generate(asni))
            for codec in ["ber", "der", "per", "uper", "oer", "jer", "xer"]:
                for numeric in [False, True]:
                    spec = asnkitrt.load_asni(asni, codec, numeric_enums=numeric)
                    for name, value in [
                        ("T", {"kind": 1, "payload": "hello"}),
                        ("T", {"kind": 2, "payload": 42}),
                        ("E", {"kind": 1 if numeric else "text", "payload": "hello"}),
                        ("O", {"kind": "1.2.3", "payload": "hello"}),
                    ]:
                        with self.subTest(codec=codec, numeric=numeric, type=name):
                            self.assertEqual(
                                spec.decode(name, spec.encode(name, value)), value
                            )
                value = generated.T(kind=1, payload="hello")
                self.assertEqual(generated.T.decode(value.encode(codec), codec), value)
