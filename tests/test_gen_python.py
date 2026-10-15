# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""The Python code asnkit-gen-python makes from compiled specifications."""

import os
import subprocess
import sys
import tempfile
import types
import unittest
from typing import Any

import asnkitrt
from asnkitrt import syntax
from asnkitrt.errors import CompileError
from asnkit_gen_python import GenerateError, generate

from tests.test_asni_asn import convert_node, golden_cases, without_defaults
from tests.test_openssl_x509 import (
    EXTENSIONS,
    OPENSSL,
    SUBJECTS,
    make_certificate,
)

FOO_ASNI = os.path.join("examples", "compiled", "foo.asni")
RFC5280_ASNI = os.path.join("examples", "compiled", "rfc5280.asni")
GENERATED_FOO = os.path.join("examples", "generated", "foo.py")

counter = 0


def read(path: str) -> bytes:
    with open(path, "rb") as fin:
        return fin.read()


def load_module(text: str) -> Any:
    """Returns the module the given text of Python is."""

    global counter
    counter += 1
    module = types.ModuleType("generated_{}".format(counter))
    sys.modules[module.__name__] = module
    exec(compile(text, module.__name__, "exec"), module.__dict__)

    return module


def asni_of(case: dict[str, Any]) -> bytes:
    return syntax.to_asni(syntax.from_compact(case["compile"]["nodes"]))


def runtime_can_load(asni: bytes) -> bool:
    try:
        asnkitrt.load_asni(asni, "ber")
    except CompileError:
        return False

    return True


class GeneratedFooTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.foo = load_module(generate(read(FOO_ASNI), ["foo.asn"], "uper"))
        cls.runtime = asnkitrt.load_asni_file(FOO_ASNI, "uper")

    def test_encode_and_decode_are_those_of_the_runtime(self) -> None:
        question = self.foo.Question(id=1, question="Is 1+1=3?")
        value = {"id": 1, "question": "Is 1+1=3?"}

        self.assertEqual(question.to_value(), value)
        self.assertEqual(question.encode(), self.runtime.encode("Question", value))
        self.assertEqual(
            self.foo.Question.decode(question.encode()),
            question,
        )

        for codec in ["ber", "der", "oer", "per", "uper", "xer", "jer"]:
            encoded = question.encode(codec)
            self.assertEqual(self.foo.Question.decode(encoded, codec), question)

    def test_the_default_codec_is_the_one_asked_for(self) -> None:
        self.assertEqual(self.foo.DEFAULT_CODEC, "uper")

    def test_a_wrong_value_is_an_error_of_the_runtime(self) -> None:
        with self.assertRaises(asnkitrt.EncodeError):
            self.foo.Question(id="x", question="a").encode()  # type: ignore[arg-type]

        with self.assertRaises(asnkitrt.DecodeError):
            self.foo.Question.decode(b"\xff")

    def test_embedded_specification_is_the_file(self) -> None:
        self.assertEqual(self.foo._ASNI, read(FOO_ASNI))

    def test_what_is_committed_is_what_is_generated(self) -> None:
        text = generate(read(FOO_ASNI), ["foo.asn"])

        with open(GENERATED_FOO, encoding="utf-8") as fin:
            self.assertEqual(
                fin.read(),
                text,
                "Make it again with: uv run asnkit-gen-python --asni {} "
                "--out examples/generated".format(FOO_ASNI),
            )


SHAPES = """
Shapes DEFINITIONS AUTOMATIC TAGS ::= BEGIN
Color ::= ENUMERATED { red, light-green, blue, ... }
Point ::= SEQUENCE { x INTEGER, y INTEGER }
Shape ::= CHOICE {
    circle  SEQUENCE { centre Point, radius INTEGER },
    square  Point,
    nothing NULL,
    colors  SEQUENCE OF Color
}
Drawing ::= SEQUENCE {
    class       UTF8String,
    my-color    Color OPTIONAL,
    flag        NULL OPTIONAL,
    always      NULL,
    shapes      SEQUENCE OF Shape,
    size        INTEGER DEFAULT 5,
    bits        BIT STRING,
    ...,
    note        UTF8String OPTIONAL,
    encode      BOOLEAN DEFAULT FALSE
}
Names ::= SEQUENCE OF UTF8String
Id ::= INTEGER (0..100)
Tree ::= SEQUENCE { value INTEGER, children SEQUENCE OF Tree }
END
"""


class GeneratedShapesTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        from tests import compiled as fixtures

        asni = fixtures.compile_string_to_asni(SHAPES)
        cls.shapes = load_module(generate(asni, ["shapes.asn"]))

    def drawing(self) -> Any:
        m = self.shapes

        return m.Drawing(
            class_="a",
            always=None,
            shapes=[
                m.ShapeCircle(m.ShapeCircleValue(centre=m.Point(x=1, y=2), radius=3)),
                m.ShapeSquare(m.Point(x=-1, y=0)),
                m.ShapeNothing(None),
                m.ShapeColors([m.Color.RED, m.Color.LIGHT_GREEN]),
            ],
            bits=(b"\xa0", 3),
        )

    def test_names_are_made_python(self) -> None:
        drawing = self.drawing()

        self.assertEqual(drawing.class_, "a")
        self.assertEqual(self.shapes.Color.LIGHT_GREEN.value, "light-green")
        # A member called encode does not hide the method.
        self.assertTrue(callable(drawing.encode))
        self.assertTrue(hasattr(drawing, "encode_"))

    def test_roundtrip_with_each_codec(self) -> None:
        drawing = self.drawing()

        for codec in ["ber", "der", "oer", "per", "uper"]:
            decoded = self.shapes.Drawing.decode(drawing.encode(codec), codec)

            # What is left out is what the specification gives it by default,
            # which BER and DER tell and the others do not.
            if codec in ("ber", "der"):
                self.assertEqual(decoded.size, 5, codec)
                self.assertEqual(decoded.encode_, False, codec)

            decoded.size = None
            decoded.encode_ = None
            self.assertEqual(decoded, drawing, codec)

    def test_optional_null_is_a_bool_and_a_null_is_none(self) -> None:
        drawing = self.drawing()

        self.assertNotIn("flag", drawing.to_value())
        self.assertIn("always", drawing.to_value())

        drawing.flag = True
        self.assertIn("flag", drawing.to_value())
        self.assertIs(
            self.shapes.Drawing.decode(drawing.encode("der"), "der").flag, True
        )

    def test_choice_alternatives_are_classes_of_the_choice(self) -> None:
        circle = self.drawing().shapes[0]

        self.assertIsInstance(circle, self.shapes.Shape)
        self.assertEqual(circle.to_value()[0], "circle")
        self.assertEqual(
            self.shapes.Shape.decode(circle.encode("uper"), "uper"), circle
        )

    def test_types_that_are_not_classes_have_functions(self) -> None:
        shapes = self.shapes
        encoded = shapes.encode_names(["a", "bc"], "uper")

        self.assertEqual(shapes.decode_names(encoded, "uper"), ["a", "bc"])
        self.assertEqual(shapes.decode_id(shapes.encode_id(7, "der"), "der"), 7)

        with self.assertRaises(asnkitrt.Error):
            shapes.encode_id(101, "der", check_constraints=True)

    def test_recursive_types(self) -> None:
        tree = self.shapes.Tree(
            value=1,
            children=[
                self.shapes.Tree(value=2, children=[]),
                self.shapes.Tree(
                    value=3, children=[self.shapes.Tree(value=4, children=[])]
                ),
            ],
        )

        self.assertEqual(self.shapes.Tree.decode(tree.encode("der"), "der"), tree)

    def test_extensible_enumeration_unknown_value_is_none(self) -> None:
        value = self.shapes.Color.from_value("blue")

        self.assertIs(value, self.shapes.Color.BLUE)


class GeneratedFromGoldenCasesTest(unittest.TestCase):
    def test_every_specification_generates_and_loads(self) -> None:
        count = 0
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)

        for case_id, case in golden_cases():
            asni = asni_of(case)

            if not runtime_can_load(asni):
                continue

            with self.subTest(case=case_id):
                text = generate(asni, [case_id])
                load_module(text).spec()
                name = "case_" + case_id.replace("-", "_")

                with open(os.path.join(directory.name, name + ".py"), "w") as fout:
                    fout.write(text)

                count += 1

        self.assertGreater(count, 60)
        self.mypy_accepts(directory.name)

    def mypy_accepts(self, directory: str) -> None:
        try:
            import mypy  # noqa: F401
        except ImportError:
            self.skipTest("mypy is not installed")

        result = subprocess.run(
            [sys.executable, "-m", "mypy", "--no-incremental", directory],
            capture_output=True,
            text=True,
        )

        self.assertEqual(result.returncode, 0, result.stdout[-3000:])

    def test_asni_asn_values_go_through_the_classes(self) -> None:
        """The compiled specification of every golden case is a value of
        examples/asni.asn, whose classes make it and give it back.

        """

        asni_asn = [
            c for _, c in golden_cases() if c.get("files") == ["examples/asni.asn"]
        ]
        module = load_module(generate(asni_of(asni_asn[0]), ["asni.asn"], "der"))
        runtime = asnkitrt.load_asni(asni_of(asni_asn[0]), "der")
        count = 0

        for case_id, case in golden_cases():
            value = convert_node(case["compile"]["nodes"])

            with self.subTest(case=case_id):
                obj = module.CompiledSpecification.from_value(value)

                self.assertEqual(obj.to_value(), value)
                self.assertEqual(
                    obj.encode("der"), runtime.encode("CompiledSpecification", value)
                )
                self.assertEqual(
                    without_defaults(
                        module.CompiledSpecification.decode(
                            obj.encode("der")
                        ).to_value()
                    ),
                    without_defaults(value),
                )
                count += 1

        self.assertGreater(count, 60)


@unittest.skipIf(OPENSSL is None, "openssl is not installed")
class GeneratedX509Test(unittest.TestCase):
    def test_certificates_of_openssl_go_through_the_classes_unchanged(self) -> None:
        rfc5280 = load_module(generate(read(RFC5280_ASNI), ["rfc5280.asn"], "der"))
        directory = tempfile.TemporaryDirectory()
        self.addCleanup(directory.cleanup)
        keys = {
            "rsa": ["-newkey", "rsa:2048"],
            "p256": ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1"],
            "ed25519": ["-newkey", "ed25519"],
        }

        for name, key_args in keys.items():
            for subject_name, subject in SUBJECTS.items():
                der = make_certificate(
                    directory.name,
                    name + "_" + subject_name,
                    key_args,
                    subject,
                    EXTENSIONS if subject_name == "plain" else (),
                )

                with self.subTest(key=name, subject=subject_name):
                    certificate = rfc5280.Certificate.decode(der, "der")

                    self.assertIsInstance(certificate, rfc5280.Certificate)
                    self.assertEqual(certificate.encode("der"), der)


class GenerateErrorsTest(unittest.TestCase):
    def test_a_file_that_needs_keys_is_not_embedded(self) -> None:
        from tests import compiled as fixtures

        with self.assertRaises(GenerateError):
            generate(fixtures.compile_string_to_asni(SHAPES, nonce=1))

    def test_two_things_with_one_name_in_python_is_an_error(self) -> None:
        from tests import compiled as fixtures

        asni = fixtures.compile_string_to_asni(
            "A DEFINITIONS ::= BEGIN Foo-Bar ::= INTEGER FooBar ::= INTEGER END"
        )

        with self.assertRaisesRegex(GenerateError, "FooBar"):
            generate(asni)

    def test_the_same_type_name_in_two_modules(self) -> None:
        from tests import compiled as fixtures

        asni = fixtures.compile_string_to_asni(
            "A DEFINITIONS ::= BEGIN T ::= SEQUENCE { a INTEGER } END "
            "B DEFINITIONS ::= BEGIN T ::= SEQUENCE { b BOOLEAN } END"
        )
        module = load_module(generate(asni))
        a = module.AT(a=1)
        b = module.BT(b=True)

        self.assertEqual(module.AT.decode(a.encode("der"), "der"), a)
        self.assertEqual(module.BT.decode(b.encode("der"), "der"), b)


if __name__ == "__main__":
    unittest.main()
