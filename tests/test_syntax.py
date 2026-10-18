# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

import glob
import importlib.util
import json
import unittest

from asnkitrt.constants import ValueRange
from asnkitrt import syntax
from asnkitrt.syntax.convert import _tree_from_dict, _tree_to_dict


def load_expected(path):
    spec = importlib.util.spec_from_file_location("expected", path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)

    return module.EXPECTED


def parsed_specification_paths():
    """All files with a parsed specification dictionary. Those ending
    with _pp.py are pre-processed, which is another format.

    """

    paths = glob.glob("tests/files/**/*.py", recursive=True)

    return sorted(path for path in paths if not path.endswith("_pp.py"))


class SyntaxTest(unittest.TestCase):
    maxDiff = None

    def test_dict_round_trip(self):
        paths = parsed_specification_paths()
        self.assertGreater(len(paths), 40)

        for path in paths:
            with self.subTest(path=path):
                expected = load_expected(path)
                tree = _tree_from_dict(expected)
                converted = _tree_to_dict(tree)
                self.assertEqual(converted, expected)
                self.assertEqual(_tree_from_dict(converted), tree)

    def assert_same_types(self, first, second, path="specification"):
        """Equal is not enough, as a string in quotes is equal to the
        same word without, and True is equal to 1.

        """

        if isinstance(first, dict):
            for key, value in first.items():
                self.assert_same_types(value, second[key], "{}.{}".format(path, key))
        elif isinstance(first, (list, tuple)):
            self.assertIs(type(first), type(second), path)

            for index, (first_item, second_item) in enumerate(zip(first, second)):
                self.assert_same_types(
                    first_item, second_item, "{}[{}]".format(path, index)
                )
        else:
            self.assertIs(type(first), type(second), "{}: {!r}".format(path, first))

        if isinstance(first, ValueRange):
            self.assertEqual(
                (first.is_minimum_excluded, first.is_maximum_excluded),
                (second.is_minimum_excluded, second.is_maximum_excluded),
                path,
            )

    def assert_primitive(self, primitive, path="tree"):
        if isinstance(primitive, dict):
            for key, value in primitive.items():
                self.assertIsInstance(key, str, path)
                self.assert_primitive(value, "{}.{}".format(path, key))
        elif isinstance(primitive, list):
            for index, item in enumerate(primitive):
                self.assert_primitive(item, "{}[{}]".format(path, index))
        elif primitive is not None:
            self.assertIn(type(primitive), [str, int, float, bool], path)

    def test_primitive_round_trip(self):
        for path in parsed_specification_paths():
            with self.subTest(path=path):
                tree = _tree_from_dict(load_expected(path))
                primitive = syntax.to_primitive(tree)
                self.assert_primitive(primitive)
                self.assertEqual(syntax.from_primitive(primitive), tree)
                # Must survive a serialization format.
                primitive = json.loads(json.dumps(primitive))
                self.assertEqual(syntax.from_primitive(primitive), tree)

    def test_from_primitive_errors(self):
        datas = [
            ({"kind": "Foo"}, "Unsupported node 'Foo'."),
            ({"a": 1}, "Unsupported node None."),
            (
                {"kind": "Range", "lower": 1, "upper": 2, "step": 1},
                "Unsupported field(s) 'step' in node 'Range'.",
            ),
        ]

        for primitive, message in datas:
            with self.assertRaises(syntax.ConvertError) as cm:
                syntax.from_primitive(primitive)

            self.assertEqual(str(cm.exception), message)

    def test_from_dict_unsupported_key(self):
        with self.assertRaises(syntax.ConvertError) as cm:
            _tree_from_dict({"A": {"types": {"B": {"type": "NULL", "foo": 1}}}})

        self.assertEqual(str(cm.exception), "Unsupported key(s) 'foo' in type 'NULL'.")

    def test_dict_functions_are_removed(self):
        import asnkitrt

        for name in ["from_dict", "to_dict", "compiled_from_dict", "compiled_to_dict"]:
            with self.subTest(name=name):
                self.assertFalse(hasattr(syntax, name))

        self.assertFalse(hasattr(asnkitrt.runtime, "load_dict"))


if __name__ == "__main__":
    unittest.main()
