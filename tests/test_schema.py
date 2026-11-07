# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""What ``asnkitcc schema`` writes is held to the vectors: each value that
JER and XER encode in ``tests/vectors`` is valid in the JSON Schema and
the DTD of the types of its specification.

Give the path of the binary in ASNKIT_RUST_BINARY, as built with
``cargo build --bin asnkitcc``, and have the group ``schema`` of
pyproject.toml, with ``uv run --group schema``. Without either the tests
are skipped.

"""

import gzip
import json
import os
import subprocess
import tempfile
import unittest
from collections import defaultdict
from typing import Any

BINARY = os.environ.get("ASNKIT_RUST_BINARY")

HERE = os.path.dirname(os.path.abspath(__file__))
VECTORS = os.path.join(HERE, "vectors")

try:
    import jsonschema  # type: ignore[import-untyped]
    from lxml import etree  # type: ignore[import-untyped]
except ImportError:  # pragma: no cover
    jsonschema = None  # type: ignore[assignment]
    etree = None  # type: ignore[assignment]


def encodings(codec: str) -> dict[str, list[tuple[str, bytes]]]:
    """The type and the encoding of each value encoded with the codec,
    by specification."""

    found: dict[str, list[tuple[str, bytes]]] = defaultdict(list)

    with gzip.open(os.path.join(VECTORS, "vectors.jsonl.gz"), "rt") as fin:
        for line in fin:
            vector = json.loads(line)

            if (
                vector["codec"] == codec
                and vector["operation"] == "encode"
                and vector["error"] is None
            ):
                found[vector["spec"]].append(
                    (vector["type"], bytes.fromhex(vector["encoded"]))
                )

    return found


@unittest.skipIf(BINARY is None, "ASNKIT_RUST_BINARY is not set.")
@unittest.skipIf(etree is None, "jsonschema and lxml are not installed.")
class SchemaTest(unittest.TestCase):
    def setUp(self) -> None:
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)

    def schema(self, spec: str, form: str, *arguments: str) -> tuple[str | None, str]:
        """The schema of the specification, or None if it can not be made,
        and what is told on standard error."""

        assert BINARY is not None

        output = os.path.join(self.directory.name, "schema")
        done = subprocess.run(
            [
                BINARY,
                "schema",
                "--format",
                form,
                "-o",
                output,
                *arguments,
                os.path.join(VECTORS, "specs", f"{spec}.asni"),
            ],
            capture_output=True,
            text=True,
        )

        if done.returncode != 0:
            return None, done.stderr

        with open(output) as fin:
            return fin.read(), done.stderr

    def test_jer(self) -> None:
        checked = 0

        for spec, values in sorted(encodings("jer").items()):
            text, stderr = self.schema(spec, "json-schema", "--skip-unsupported")

            self.assertIsNotNone(text, stderr)
            assert text is not None

            document = json.loads(text)
            jsonschema.Draft202012Validator.check_schema(document)

            for name, encoded in values:
                if name not in document["$defs"]:
                    continue

                schema: dict[str, Any] = dict(document, **{"$ref": f"#/$defs/{name}"})
                errors = list(
                    jsonschema.Draft202012Validator(schema).iter_errors(
                        json.loads(encoded)
                    )
                )

                self.assertEqual(
                    errors, [], f"{spec} {name}: {encoded.decode()} {errors[:1]}"
                )
                checked += 1

        # All but those of open types.
        self.assertGreater(checked, 180)

    def test_dtd(self) -> None:
        checked = 0

        # A DTD of each type alone, as the types of the vectors have
        # members of the same names with other types.
        for spec, values in sorted(encodings("xer").items()):
            for name in sorted({name for name, _ in values}):
                text, stderr = self.schema(spec, "dtd", "-t", name)

                if text is None:
                    self.assertTrue(
                        "which a DTD can not tell apart" in stderr
                        or "the codec needs the type" in stderr,
                        f"{spec} {name}: {stderr}",
                    )
                    continue

                dtd = etree.DTD(file=os.path.join(self.directory.name, "schema"))

                for other, encoded in values:
                    if other != name:
                        continue

                    document = etree.fromstring(encoded)

                    self.assertTrue(
                        dtd.validate(document),
                        f"{spec} {name}: {encoded.decode()} "
                        f"{dtd.error_log.filter_from_errors()}",
                    )
                    checked += 1

        self.assertGreater(checked, 180)

    def test_xsd(self) -> None:
        checked = 0

        for spec, values in sorted(encodings("xer").items()):
            text, stderr = self.schema(spec, "xsd", "--skip-unsupported")

            self.assertIsNotNone(text, stderr)
            assert text is not None

            document = etree.fromstring(text.encode())
            # It is an XML Schema, with no element of two types in one
            # content and no particle that is unsure.
            schema = etree.XMLSchema(document)
            roots = {
                element.get("name")
                for element in document
                if element.tag == "{http://www.w3.org/2001/XMLSchema}element"
            }

            for name, encoded in values:
                if name not in roots:
                    continue

                self.assertTrue(
                    schema.validate(etree.fromstring(encoded)),
                    f"{spec} {name}: {encoded.decode()} {schema.error_log.last_error}",
                )
                checked += 1

        # All but those of open types, and roots of one name in two modules.
        self.assertGreater(checked, 200)

    def test_wrong_documents_are_not_valid(self) -> None:
        assert BINARY is not None

        foo = os.path.join(HERE, "..", "examples", "compiled", "foo.asni")
        output = os.path.join(self.directory.name, "foo")

        subprocess.run(
            [
                BINARY,
                "schema",
                "--format",
                "json-schema",
                "-t",
                "Question",
                "-o",
                output,
                foo,
            ],
            check=True,
        )

        with open(output) as fin:
            validator = jsonschema.Draft202012Validator(json.load(fin))

        self.assertTrue(validator.is_valid({"id": 1, "question": "Is 1+1=3?"}))

        for wrong in [
            {"id": "1", "question": "Is 1+1=3?"},
            {"id": 1},
            {"id": 1, "question": "Is 1+1=3?", "more": 2},
            {"id": 1, "question": "\u00e9"},
            [1],
        ]:
            self.assertFalse(validator.is_valid(wrong), wrong)

        subprocess.run(
            [BINARY, "schema", "--format", "dtd", "-o", output, foo], check=True
        )
        dtd = etree.DTD(file=output)

        self.assertTrue(
            dtd.validate(
                etree.fromstring(
                    b"<Answer><id>1</id><answer><true /></answer></Answer>"
                )
            )
        )

        subprocess.run(
            [BINARY, "schema", "--format", "xsd", "-o", output, foo], check=True
        )
        xsd = etree.XMLSchema(file=output)

        for wrong in [
            b"<Answer><answer><true /></answer></Answer>",
            b"<Answer><answer><true /></answer><id>1</id></Answer>",
            b"<Answer><id>1</id><answer>true</answer></Answer>",
            b"<Answer><id>x</id><answer><true /></answer></Answer>",
            b"<Answer><id>1</id><answer><true>x</true></answer></Answer>",
            b"<Question><id>1</id><question>a<bell/></question></Question>",
        ]:
            self.assertFalse(xsd.validate(etree.fromstring(wrong)), wrong)

        self.assertTrue(
            xsd.validate(
                etree.fromstring(
                    b"<Question><id>1</id><question>a<bel/></question></Question>"
                )
            )
        )

        for wrong in [
            b"<Answer><answer><true /></answer></Answer>",
            b"<Answer><answer><true /></answer><id>1</id></Answer>",
            b"<Answer><id>1</id><answer>true</answer></Answer>",
            b"<Answer><id><x /></id><answer><true /></answer></Answer>",
        ]:
            self.assertFalse(dtd.validate(etree.fromstring(wrong)), wrong)


if __name__ == "__main__":
    unittest.main()
