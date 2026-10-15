# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""Holds the runtime for Python to the vectors in tests/vectors, which
are what the runtime for Rust, the one of reference, does.

"""

import collections
import gzip
import json
import os
import unittest

import asnkitrt
from tests.vectors.values import Skip, from_json, to_json

FOLDER = os.path.join(os.path.dirname(__file__), "vectors")


def read_vectors():
    with gzip.open(os.path.join(FOLDER, "vectors.jsonl.gz"), "rt") as fin:
        return [json.loads(line) for line in fin]


def load(spec, codec, numeric_enums, lenient):
    with open(os.path.join(FOLDER, "specs", spec + ".asnc"), "rb") as fin:
        return asnkitrt.load_asnc(
            fin.read(), codec, numeric_enums=numeric_enums, lenient=lenient
        )


def perform(spec, vector):
    """The value or the bytes the operation of a vector gives."""

    name = vector["type"]
    options = vector["options"]

    if vector["operation"] == "encode":
        kwargs = {}

        if "indent" in options:
            kwargs["indent"] = options["indent"]

        return spec.encode(
            name,
            from_json(vector["value"]),
            check_types=options["check_types"],
            check_constraints=options["check_constraints"],
            **kwargs,
        ).hex()
    else:
        decoded = spec.decode(
            name,
            bytes.fromhex(vector["encoded"]),
            check_constraints=options["check_constraints"],
        )

        return to_json(decoded)


class VectorsTest(unittest.TestCase):
    def test_vectors(self):
        specs = {}
        passed = collections.Counter()
        failures = []

        for vector in read_vectors():
            key = (
                vector["spec"],
                vector["codec"],
                vector["numeric_enums"],
                vector["lenient"],
            )

            if key not in specs:
                specs[key] = load(*key)

            describe = "{} {} {} {}: {}".format(
                vector["codec"],
                vector["operation"],
                vector["spec"],
                vector["type"],
                json.dumps(vector["value"])[:100],
            )
            expected = (
                vector["encoded"]
                if vector["operation"] == "encode"
                else vector["value"]
            )

            try:
                got = perform(specs[key], vector)
            except Skip:
                failures.append("decoded what is not written <= " + describe)
                continue
            except Exception as error:
                if vector["error"] is None:
                    failures.append("failed: {!r} <= {}".format(error, describe))
                else:
                    passed[vector["codec"]] += 1

                continue

            if vector["error"] is not None:
                failures.append(
                    "gave {} but expected {} <= {}".format(
                        json.dumps(got)[:100], vector["error"], describe
                    )
                )
            elif got != expected:
                failures.append("gave {} <= {}".format(json.dumps(got)[:100], describe))
            else:
                passed[vector["codec"]] += 1

        self.assertEqual(failures[:20], [], "{} failed".format(len(failures)))
        self.assertGreater(sum(passed.values()), 5000)


if __name__ == "__main__":
    unittest.main()
