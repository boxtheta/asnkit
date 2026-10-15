# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Record the test vectors that every runtime is held to.

Run it from the root of the repository:

    uv run python tests/vectors/record.py

It needs cargo, as the runtime for Rust is the one of reference: what it
encodes and decodes is what the vectors say.

This finds the cases, the specifications, types and values to try, in
the tests of the codecs of the runtime for Python. It runs them with
`load_dict()` and the encode and decode methods of a specification
wrapped. Each time a test loads a specification, it is resolved and
kept as an ``.asnc`` file in ``specs``, named by the SHA-256 of its
contents. Each time a test encodes or decodes, the type and what is
given (the value to encode, or the bytes to decode) are noted.

The runtime for Rust then does each of these, with
``packages/asnkitrt-rs/examples/record_vectors.rs``, and what it gives
is written in ``vectors.jsonl.gz``, one JSON object per line. What it
does not support is left out. ``tests/test_vectors.py`` holds the
runtime for Python to them.

Only the codecs a runtime in another language has are kept: ``ber``,
``der``, ``oer``, ``per``, ``uper``, ``jer`` and ``xer``. The encoded
bytes of ``jer`` and ``xer`` are UTF-8 text. ``values.py`` tells how a
value is written.

"""

import copy
import gzip
import hashlib
import json
import os
import subprocess
import sys
import unittest

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), "..", ".."))
os.chdir(ROOT)
sys.path.insert(0, ROOT)
for folder in ["", "3gpp", "cen", "etsi", "ieee", "ietf", "oma"]:
    sys.path.append(os.path.join(ROOT, "tests", "files", folder))

from asnkitrt import runtime, syntax  # noqa: E402
from tests.vectors.values import Skip, to_json  # noqa: E402

TEST_MODULES = [
    "tests.test_ber",
    "tests.test_der",
    "tests.test_der_strict",
    "tests.test_oer",
    "tests.test_per",
    "tests.test_uper",
    "tests.test_jer",
    "tests.test_xer",
    "tests.test_codecs",
    "tests.test_codecs_consistency",
    "tests.test_duration",
    "tests.test_big_integers",
    "tests.test_files",
    "tests.test_runtime",
    "tests.test_type_checker",
    "tests.test_constraints_checker",
]

CODECS = ("ber", "der", "oer", "per", "uper", "jer", "xer")

HERE = os.path.dirname(os.path.abspath(__file__))


class Recorder:
    def __init__(self):
        self.specs = {}
        self.vectors = {}
        self.by_object = {}
        self.skipped = 0

    def note_load(self, specification, codec, numeric_enums, lenient, loaded):
        if codec not in CODECS:
            return

        try:
            asnc = syntax.to_asnc(
                syntax.compiled_from_dict(specification),
                syntax.Metadata(generator=None),
            )
        except Exception:
            # Not a specification that can be compiled to a file.
            return

        digest = hashlib.sha256(asnc).hexdigest()[:16]
        self.specs[digest] = asnc
        self.by_object[id(loaded)] = (loaded, digest, codec, numeric_enums, lenient)

    def note(self, loaded, operation, name, given, options):
        """Note a case: what is given, the value to encode or the bytes to
        decode. What is done with it is left to the runtime for Rust.

        """

        entry = self.by_object.get(id(loaded))

        if entry is None or entry[0] is not loaded:
            return

        _, digest, codec, numeric_enums, lenient = entry

        try:
            vector = {
                "spec": digest,
                "codec": codec,
                "numeric_enums": numeric_enums,
                "lenient": lenient,
                "operation": operation,
                "type": name,
                "value": to_json(given) if operation == "encode" else None,
                "encoded": bytes(given).hex() if operation == "decode" else "",
                "error": None,
                "options": options,
            }
        except Skip:
            self.skipped += 1
            return

        key = json.dumps(vector, sort_keys=True)
        self.vectors[key] = vector


RECORDER = Recorder()


def install():
    original_load_dict = runtime.load_dict
    original_encode = runtime.Specification.encode
    original_decode = runtime.Specification.decode

    def load_dict(specification, codec="ber", numeric_enums=False, lenient=False):
        snapshot = copy.deepcopy(specification)
        loaded = original_load_dict(specification, codec, numeric_enums, lenient)
        RECORDER.note_load(snapshot, codec, numeric_enums, lenient, loaded)

        return loaded

    def encode(self, name, data, check_types=True, check_constraints=False, **kwargs):
        options = {"check_types": check_types, "check_constraints": check_constraints}

        if kwargs.get("indent") is not None:
            options["indent"] = kwargs["indent"]

        # Only JER and XER take an indent, the one keyword argument kept.
        if set(kwargs) <= {"indent"}:
            try:
                value = copy.deepcopy(data)
            except Exception:
                value = data

            RECORDER.note(self, "encode", name, value, options)

        return original_encode(
            self, name, data, check_types, check_constraints, **kwargs
        )

    def decode(self, name, data, check_constraints=False):
        options = {"check_constraints": check_constraints}
        RECORDER.note(self, "decode", name, data, options)

        return original_decode(self, name, data, check_constraints)

    runtime.load_dict = load_dict
    runtime.Specification.encode = encode
    runtime.Specification.decode = decode


def run_rust(cases):
    """Each case done by the runtime for Rust: a vector, or None."""

    command = [
        "cargo",
        "run",
        "--quiet",
        "--release",
        "--package",
        "asnkitrt",
        "--example",
        "record_vectors",
    ]
    lines = "".join(json.dumps(case) + "\n" for case in cases)
    result = subprocess.run(
        command, input=lines, capture_output=True, text=True, check=True
    )

    return [json.loads(line) for line in result.stdout.splitlines()]


def main():
    install()
    suite = unittest.defaultTestLoader.loadTestsFromNames(TEST_MODULES)

    with open(os.devnull, "w") as devnull:
        result = unittest.TextTestRunner(stream=devnull, verbosity=0).run(suite)

    print(
        "Ran {} tests, {} failed, {} errors.".format(
            result.testsRun, len(result.failures), len(result.errors)
        )
    )

    specs_folder = os.path.join(HERE, "specs")
    os.makedirs(specs_folder, exist_ok=True)

    for name in os.listdir(specs_folder):
        os.remove(os.path.join(specs_folder, name))

    for digest, asnc in RECORDER.specs.items():
        with open(os.path.join(specs_folder, digest + ".asnc"), "wb") as fout:
            fout.write(asnc)

    cases = list(RECORDER.vectors.values())
    done = run_rust(cases)
    unsupported = done.count(None)

    # The same inputs always make the same file.
    vectors = sorted(
        json.dumps(vector, sort_keys=True) for vector in done if vector is not None
    )

    with gzip.GzipFile(os.path.join(HERE, "vectors.jsonl.gz"), "wb", mtime=0) as fout:
        for line in vectors:
            fout.write(line.encode("utf-8") + b"\n")

    print(
        "{} specifications, {} vectors, {} not written, {} not supported "
        "by the runtime for Rust.".format(
            len(RECORDER.specs), len(vectors), RECORDER.skipped, unsupported
        )
    )


if __name__ == "__main__":
    main()
