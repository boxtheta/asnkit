# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""The files the Rust compiler signs and encrypts are read by the runtime
of Python, which is the other side of what the Rust compiler writes.

Give the path of the binary in ASNKIT_RUST_BINARY, as built with
``cargo build --release --bin asnkitcc``. Without it the tests are skipped.

"""

import hashlib
import os
import stat
import subprocess
import sys
import tempfile
import unittest
from typing import Any

import asnkitrt

BINARY = os.environ.get("ASNKIT_RUST_BINARY")

SPECIFICATION = "A DEFINITIONS ::= BEGIN B ::= INTEGER END"


@unittest.skipIf(BINARY is None, "ASNKIT_RUST_BINARY is not set.")
class RustCompilerProtectionTest(unittest.TestCase):
    def setUp(self) -> None:
        from cryptography.hazmat.primitives import serialization
        from cryptography.hazmat.primitives.asymmetric.ed25519 import (
            Ed25519PrivateKey,
        )

        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)

        self.signing_key = bytes(range(32))
        self.encryption_key = bytes(range(100, 132))
        self.public_key = (
            Ed25519PrivateKey.from_private_bytes(self.signing_key)
            .public_key()
            .public_bytes(serialization.Encoding.Raw, serialization.PublicFormat.Raw)
        )
        self.signing_id = hashlib.sha256(self.public_key).digest()[:16]
        self.encryption_id = hashlib.sha256(self.encryption_key).digest()[:16]

        self.write("a.asn", SPECIFICATION.encode())
        # A key is given as bytes or as hexadecimal digits.
        self.write("signing.key", self.signing_key)
        self.write("encryption.key", self.encryption_key.hex().encode() + b"\n")

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
                self.path("a.asnc"),
                *arguments,
                self.path("a.asn"),
            ],
            capture_output=True,
        )

    def read(self, **kwargs) -> bytes:  # type: ignore[no-untyped-def]
        return asnkitrt.load_asnc_file(  # type: ignore[no-any-return]
            self.path("a.asnc"), "der", **kwargs
        ).encode("B", 5)

    def test_signed(self) -> None:
        result = self.compile("--signing-key-file", self.path("signing.key"))
        self.assertEqual(result.returncode, 0, result.stderr)

        self.assertEqual(
            self.read(verification_keys={self.signing_id: self.public_key}),
            b"\x02\x01\x05",
        )

        with self.assertRaises(asnkitrt.Error):
            self.read()

        with self.assertRaises(asnkitrt.Error):
            self.read(verification_keys={self.signing_id: bytes(32)})

    def test_encrypted(self) -> None:
        result = self.compile("--encryption-key-file", self.path("encryption.key"))
        self.assertEqual(result.returncode, 0, result.stderr)

        self.assertEqual(
            self.read(decryption_keys={self.encryption_id: self.encryption_key}),
            b"\x02\x01\x05",
        )

        with self.assertRaises(asnkitrt.Error):
            self.read(decryption_keys={self.encryption_id: bytes(32)})

    def test_signed_encrypted_with_nonce_and_binding(self) -> None:
        result = self.compile(
            "--signing-key-file",
            self.path("signing.key"),
            "--signing-key-id",
            "00" * 16,
            "--encryption-key-file",
            self.path("encryption.key"),
            "--nonce",
            "7",
            "--bound-to",
            "corpus",
        )
        self.assertEqual(result.returncode, 0, result.stderr)

        arguments = dict(
            verification_keys={bytes(16): self.public_key},
            decryption_keys={self.encryption_id: self.encryption_key},
            nonce=7,
            bound_to="corpus",
        )

        self.assertEqual(self.read(**arguments), b"\x02\x01\x05")

        arguments["nonce"] = 8

        with self.assertRaises(asnkitrt.Error):
            self.read(**arguments)

    def test_a_new_iv_each_time(self) -> None:
        arguments = ["--encryption-key-file", self.path("encryption.key")]
        self.compile(*arguments)

        with open(self.path("a.asnc"), "rb") as fin:
            first = fin.read()

        self.compile(*arguments)

        with open(self.path("a.asnc"), "rb") as fin:
            second = fin.read()

        # The initialization vector follows the header and the key id.
        self.assertNotEqual(first[27:39], second[27:39])

    def test_verbose_tells_on_stderr_and_not_otherwise(self) -> None:
        quiet = self.compile()
        self.assertEqual(quiet.returncode, 0)
        self.assertEqual(quiet.stderr, b"")

        with open(self.path("a.asnc"), "rb") as fin:
            quiet_file = fin.read()

        result = self.compile(
            "-v", "--signing-key-file", self.path("signing.key"), "--nonce", "3"
        )
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, b"")
        text = result.stderr.decode()

        self.assertIn("Reading ", text)
        self.assertIn("Module A: 1 types, 0 object classes.", text)
        self.assertIn("Signed with the key " + self.signing_id.hex(), text)
        self.assertIn("Nonce 3.", text)
        self.assertIn("Wrote ", text)

        # What is written is not changed by telling of it.
        result = self.compile("-v")
        with open(self.path("a.asnc"), "rb") as fin:
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
            "        'asnc': files['--asnc'][:4].decode(),\n"
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
            ["--asnc", "--asnst", "--out", "--opt", "one=1", "--opt", "two=2"],
        )
        self.assertEqual(info["asnc"], "ASNC")
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

    def test_generate_refuses_what_a_file_that_is_embedded_can_not_have(self) -> None:
        for option in [
            ["--nonce", "1"],
            ["--bound-to", "x"],
            ["--signing-key-file", self.path("signing.key")],
            ["--encryption-key-file", self.path("encryption.key")],
        ]:
            result = self.generate("--plugin", "python", *option)

            self.assertNotEqual(result.returncode, 0, option)
            self.assertIn(b"does not sign", result.stderr)

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

    def test_bad_key_options(self) -> None:
        result = self.compile("--signing-key-id", "00" * 16)
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"--signing-key-file", result.stderr)

        result = self.compile("--signing-key-file", self.path("a.asn"))
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"32 bytes or 64 hexadecimal digits", result.stderr)

        result = self.compile(
            "--encryption-key-file",
            self.path("encryption.key"),
            "--encryption-key-id",
            "zz",
        )
        self.assertNotEqual(result.returncode, 0)
        self.assertIn(b"32 hexadecimal digits", result.stderr)


if __name__ == "__main__":
    unittest.main()
