# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""The files the Rust compiler writes are read by the runtime of Python,
which is the other side of what the Rust compiler writes.

Give the path of the binary in ASNKIT_RUST_BINARY, as built with
``cargo build --release --bin asnkitcc``. Without it the tests are skipped.

"""

import os
import stat
import subprocess
import sys
import tempfile
import unittest
import zlib
from typing import Any

import asnkitrt
from asnkitrt.runtime import CodecType

BINARY = os.environ.get("ASNKIT_RUST_BINARY")

SPECIFICATION = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"


@unittest.skipIf(BINARY is None, "ASNKIT_RUST_BINARY is not set.")
class RustCompilerTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.write("a.asn", SPECIFICATION.encode())

    def path(self, name: str) -> str:
        return os.path.join(self.directory.name, name)

    def write(self, name: str, data: bytes) -> None:
        with open(self.path(name), "wb") as fout:
            fout.write(data)

    def compile(self, *arguments: str) -> subprocess.CompletedProcess[bytes]:
        assert BINARY is not None

        return subprocess.run(
            [
                BINARY,
                "compile",
                "-o",
                self.path("a.asni"),
                *arguments,
                self.path("a.asn"),
            ],
            capture_output=True,
        )

    def read(self, **kwargs) -> bytes:  # type: ignore[no-untyped-def]
        return asnkitrt.load_asni_file(  # type: ignore[no-any-return]
            self.path("a.asni"), "der", **kwargs
        ).encode("B", 5)

    def test_plain(self) -> None:
        result = self.compile()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(self.read(), b"\x02\x01\x05")

        with open(self.path("a.asni"), "rb") as fin:
            data = fin.read()

        self.assertEqual(data[:6], b"ASNI\x02\x00")
        self.assertEqual(data[6:10], zlib.crc32(data[10:]).to_bytes(4, "big"))

    def test_the_options_of_a_nonce_and_keys_are_refused(self) -> None:
        self.write("key", bytes(32))

        for option in [
            ["--nonce", "1"],
            ["--signing-key-file", self.path("key")],
            ["--signing-key-id", "00" * 16],
            ["--encryption-key-file", self.path("key")],
            ["--encryption-key-id", "00" * 16],
            ["--bound-to", "corpus"],
        ]:
            with self.subTest(option=option[0]):
                result = self.compile(*option)

                self.assertNotEqual(result.returncode, 0)
                self.assertIn(
                    option[0].encode() + b" is no longer supported", result.stderr
                )
                self.assertFalse(os.path.exists(self.path("a.asni")))

    def test_verbose_tells_on_stderr_and_not_otherwise(self) -> None:
        quiet = self.compile()
        self.assertEqual(quiet.returncode, 0)
        self.assertEqual(quiet.stderr, b"")

        with open(self.path("a.asni"), "rb") as fin:
            quiet_file = fin.read()

        result = self.compile("-v")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, b"")
        text = result.stderr.decode()

        self.assertIn("Reading ", text)
        self.assertIn("Module A: 1 types, 0 object classes.", text)
        self.assertIn("Wrote ", text)

        # What is written is not changed by telling of it.
        result = self.compile("-v")
        with open(self.path("a.asni"), "rb") as fin:
            self.assertEqual(fin.read(), quiet_file)

    def generate(self, *arguments: str) -> Any:
        assert BINARY is not None
        # The plugins installed next to this Python are found by name.
        environment = dict(os.environ)
        environment["PATH"] = (
            os.path.dirname(sys.executable) + os.pathsep + environment.get("PATH", "")
        )

        return subprocess.run(
            [BINARY, "generate", *arguments, self.path("a.asn")],
            capture_output=True,
            env=environment,
        )

    def make_plugin(self, body: str) -> str:
        path = self.path("plugin.py")
        self.write(path, ("#!" + sys.executable + "\n" + body).encode())
        os.chmod(path, os.stat(path).st_mode | stat.S_IXUSR)

        return path

    def test_generate_runs_a_plugin_with_the_files_and_the_options(self) -> None:
        plugin = self.make_plugin(
            "import json, os, sys\n"
            "a = sys.argv[1:]\n"
            "files = {a[i]: open(a[i + 1], 'rb').read() for i in (0, 2)}\n"
            "out = a[a.index('--out') + 1]\n"
            "info = {'args': [x for x in a if not x.startswith('/')],\n"
            "        'asni': files['--asni'][:4].decode(),\n"
            "        'asnst': len(files['--asnst']) > 10,\n"
            "        'paths': [a[1], a[3]]}\n"
            "json.dump(info, open(os.path.join(out, 'info.json'), 'w'))\n"
        )
        result = self.generate(
            "--plugin",
            plugin,
            "--plugin-opt",
            "one=1",
            "--plugin-opt",
            "two=2",
            "-o",
            self.path("out"),
        )
        self.assertEqual(result.returncode, 0, result.stderr)

        import json

        with open(self.path("out/info.json")) as fin:
            info = json.load(fin)

        self.assertEqual(
            info["args"],
            ["--asni", "--asnst", "--out", "--opt", "one=1", "--opt", "two=2"],
        )
        self.assertEqual(info["asni"], "ASNI")
        self.assertTrue(info["asnst"])
        # The files were removed once the plugin was done.
        for path in info["paths"]:
            self.assertFalse(os.path.exists(path))

    def test_generate_tells_when_the_plugin_fails_or_is_missing(self) -> None:
        plugin = self.make_plugin("import sys\nsys.exit('it broke')\n")
        result = self.generate("--plugin", plugin, "-o", self.path("out"))

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"it broke", result.stderr)
        self.assertIn(b"failed", result.stderr)

        result = self.generate("--plugin", "no-such-plugin-anywhere")
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"is not found", result.stderr)

    def test_generate_refuses_what_it_does_not_take(self) -> None:
        result = self.generate("--plugin", "python", "--bound-to", "x")

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"--bound-to is no longer supported", result.stderr)

        result = self.generate()
        self.assertIn(b"needs --plugin", result.stderr)

    def test_generate_with_the_python_plugin(self) -> None:
        result = self.generate("--plugin", "python", "-o", self.path("out"))
        self.assertEqual(result.returncode, 0, result.stderr)

        sys.path.insert(0, self.path("out"))
        self.addCleanup(sys.path.remove, self.path("out"))
        self.addCleanup(sys.modules.pop, "a", None)

        import importlib

        module = importlib.import_module("a")

        self.assertEqual(module.decode_b(module.encode_b(5, "der"), "der"), 5)


CONVERT_SPECIFICATION = """
C DEFINITIONS AUTOMATIC TAGS ::= BEGIN
    M ::= SEQUENCE {
        id      INTEGER (0..1000),
        name    UTF8String (SIZE (1..20)),
        color   ENUMERATED { red, green, blue },
        flags   BIT STRING (SIZE (4)),
        data    OCTET STRING OPTIONAL,
        items   SEQUENCE OF CHOICE { n INTEGER, b BOOLEAN }
    }
END
"""

CONVERT_VALUE = {
    "id": 42,
    "name": "héllo",
    "color": "blue",
    "flags": (b"\xa0", 4),
    "data": b"\x01\x02",
    "items": [("n", -7), ("b", True)],
}

CODECS: list[CodecType] = ["ber", "der", "oer", "per", "uper", "jer", "xer"]


@unittest.skipIf(BINARY is None, "ASNKIT_RUST_BINARY is not set.")
class RustConvertTest(unittest.TestCase):
    """What ``asnkitcc convert`` writes is what the runtime for Python
    encodes the same value as."""

    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.asn = self.path("c.asn")

        self.asni = self.path("c.asni")

        with open(self.asn, "w") as fout:
            fout.write(CONVERT_SPECIFICATION)

        compiled = subprocess.run(
            [BINARY or "", "compile", "-o", self.asni, self.asn], capture_output=True
        )
        self.assertEqual(compiled.returncode, 0, compiled.stderr)

        with open(self.asni, "rb") as fin:
            asni = fin.read()

        self.specifications = {codec: asnkitrt.load(asni, codec) for codec in CODECS}

    def path(self, name: str) -> str:
        return os.path.join(self.directory.name, name)

    def convert(
        self, *arguments: str, data: bytes = b""
    ) -> subprocess.CompletedProcess[bytes]:
        assert BINARY is not None

        return subprocess.run(
            [BINARY, "convert", *arguments], input=data, capture_output=True
        )

    def encoded(self, codec: CodecType) -> bytes:
        return self.specifications[codec].encode("M", CONVERT_VALUE)  # type: ignore[no-any-return]

    def test_every_codec_to_every_codec(self) -> None:
        for source in CODECS:
            for target in CODECS:
                with self.subTest(source=source, target=target):
                    result = self.convert(
                        "-t",
                        "M",
                        "--from",
                        source,
                        "--to",
                        target,
                        self.asn,
                        data=self.encoded(source),
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    output = result.stdout

                    if target in ("jer", "xer"):
                        self.assertTrue(output.endswith(b"\n"))
                        output = output[:-1]

                    self.assertEqual(
                        self.specifications[target].decode("M", output),
                        CONVERT_VALUE,
                    )

                    if target not in ("jer", "xer"):
                        self.assertEqual(output, self.encoded(target))

    def test_every_codec_to_gser(self) -> None:
        """GSER is written as the runtime for Python writes it, and not
        read."""

        gser = asnkitrt.load(self.asni_data(), "gser")

        for source in CODECS:
            for indent in [None, 2]:
                with self.subTest(source=source, indent=indent):
                    arguments = [] if indent is None else ["--indent", str(indent)]
                    result = self.convert(
                        "-t",
                        "M",
                        "--from",
                        source,
                        "--to",
                        "gser",
                        *arguments,
                        self.asn,
                        data=self.encoded(source),
                    )
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual(
                        result.stdout,
                        gser.encode("M", CONVERT_VALUE, indent=indent) + b"\n",
                    )

    def asni_data(self) -> bytes:
        with open(self.asni, "rb") as fin:
            return fin.read()

    def test_an_asni_file_codecs_by_extension_and_hex(self) -> None:
        asni = self.asni

        with open(self.path("m.json"), "wb") as fout:
            fout.write(self.encoded("jer") + b"\n")

        result = self.convert(
            "-t", "C.M", "-i", self.path("m.json"), "-o", self.path("m.der"), asni
        )
        self.assertEqual(result.returncode, 0, result.stderr)

        with open(self.path("m.der"), "rb") as fin:
            self.assertEqual(fin.read(), self.encoded("der"))

        result = self.convert(
            "-t",
            "M",
            "-i",
            self.path("m.der"),
            "--to",
            "uper",
            "--output-hex",
            asni,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, self.encoded("uper").hex().encode() + b"\n")

        result = self.convert(
            "-t",
            "M",
            "--from",
            "uper",
            "--input-hex",
            "--to",
            "jer",
            "--indent",
            "2",
            asni,
            data=result.stdout,
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertIn(b'\n  "id": 42', result.stdout)

    def test_errors(self) -> None:
        for arguments, message in [
            (["--from", "der", "--to", "jer"], b"needs --type"),
            (["-t", "M", "--to", "jer"], b"--from"),
            (["-t", "M", "--from", "cer", "--to", "jer"], b"unknown codec 'cer'"),
            (["-t", "N", "--from", "der", "--to", "jer"], b"no type 'N'"),
            (["-t", "M", "--from", "der", "--to", "jer", "--indent", "x"], b"indent"),
            (["-t", "M", "--from", "jer", "--to", "der", "--input-hex"], b"binary"),
            (["-t", "M", "--from", "der", "--to", "jer", "--nonce", "1"], b"nonce"),
            (["-t", "M", "--from", "gser", "--to", "der"], b"GSER punctuation"),
        ]:
            with self.subTest(arguments=arguments):
                result = self.convert(*arguments, self.asn, data=b"0\x00")
                self.assertNotEqual(result.returncode, 0)
                self.assertIn(message, result.stderr)

        result = self.convert("-t", "M", "--from", "der", "--to", "jer", self.asn)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"error: decoding:", result.stderr)

        result = subprocess.run(
            [BINARY or "", "compile", "--to", "der", self.asn], capture_output=True
        )
        self.assertIn(b"are for convert", result.stderr)


if __name__ == "__main__":
    unittest.main()
