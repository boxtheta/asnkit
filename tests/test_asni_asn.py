# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""examples/asni.asn describes the nodes of a compiled specification.
It is compiled, and the compiled specifications of the golden cases of
the compiler, which are the nodes of real specifications, are encoded
and decoded with it. That is a test of the compilers and the runtime
with a specification of about 50 types that refer to each other, and
of values that go deep.

"""

import glob
import gzip
import json
import os
import re
import unittest
from typing import Any

import asnkitrt
from asnkitrt import syntax
from asnkitrt.runtime import CodecType

GOLDEN = os.path.join("packages", "asnkit-compiler", "tests", "golden", "cases")

# The kinds of nodes that are types, by the choice of Type they are.
TYPES = {
    "BuiltinType": "builtin",
    "ReferencedType": "referenced",
    "IntegerType": "integer",
    "BitStringType": "bit-string",
    "EnumeratedType": "enumerated",
    "SequenceType": "sequence",
    "SetType": "set",
    "ChoiceType": "choice",
    "SequenceOfType": "sequence-of",
    "SetOfType": "set-of",
    "AnyDefinedByType": "any-defined-by",
}

VALUES = {
    "Identifier": "identifier",
    "HexString": "hex-string",
    "BinaryString": "binary-string",
    "Null": "null",
    "PlusInfinity": "plus-infinity",
    "MinusInfinity": "minus-infinity",
    "NotANumber": "not-a-number",
    "NamedValue": "named-value",
    "SequenceValue": "sequence-value",
    "UnsupportedValue": "unsupported",
}

BOUNDS = {"Min": "min", "Max": "max", "Excluded": "excluded"}

MEMBER_LIST_ITEMS = {
    "Member": "member",
    "ExtensionMarker": "extension-marker",
    "ExtensionAdditionGroup": "extension-addition-group",
}

# What a field is, when it is not a plain number, text or boolean: T is
# a type, V a value, B a bound of a range, E an element of a constraint,
# M an item of a list of members, C an alternative of WITH COMPONENTS,
# W one of WITH COMPONENT, N an item of an enumeration, S an item of the
# syntax of an object class, a kind of node is that node, and a list of
# any is [it].
TYPE_FIELDS: dict[str, Any] = {
    "tag": "Tag",
    "size": ["E"],
    "restricted_to": ["E"],
    "permitted_alphabet": ["E"],
    "with_components": ["C"],
    "with_component": ["W"],
    "table": "TableConstraint",
}

CONSTRAINT_FIELDS: dict[str, Any] = {
    "size": ["E"],
    "restricted_to": ["E"],
    "permitted_alphabet": ["E"],
    "with_components": ["C"],
    "with_component": ["W"],
    "containing": "T",
}

FIELDS: dict[str, Any] = {
    "CompiledSpecification": {"modules": ["CompiledModule"]},
    "CompiledModule": {
        "types": ["TypeAssignment"],
        "object_classes": ["ObjectClassAssignment"],
    },
    "TypeAssignment": {"type": "T"},
    "Member": {"type": "T", "default": "V"},
    "Range": {"lower": "B", "upper": "B"},
    "ExtensionAdditionGroup": {"members": ["M"]},
    "IntegerType": {"named_numbers": ["NamedNumber"]},
    "BitStringType": {"named_bits": ["NamedNumber"]},
    "EnumeratedType": {"values": ["N"]},
    "SequenceType": {"members": ["M"]},
    "SetType": {"members": ["M"]},
    "ChoiceType": {"members": ["M"]},
    "SequenceOfType": {"element": "T"},
    "SetOfType": {"element": "T"},
    "AnyDefinedByType": {"choices": ["AnyDefinedByChoice"]},
    "AnyDefinedByChoice": {"value": "V", "type": "T"},
    "NamedValue": {"value": "V"},
    "SequenceValue": {"members": ["NamedValue"]},
    "Excluded": {"value": "V"},
    "ElementConstraint": CONSTRAINT_FIELDS,
    "ComponentConstraint": CONSTRAINT_FIELDS,
    "ObjectClassAssignment": {"fields": ["ObjectClassField"], "syntax": ["S"]},
    "ObjectClassField": {"type": "T"},
    "OptionalGroup": {"tokens": ["S"]},
}

# What a field is by default, which an encoder leaves out.
DEFAULTS = {
    "optional": False,
    "types": [],
    "object_classes": [],
    "choices": [],
}


def read_kinds() -> dict[int, tuple[str, list[str]]]:
    """The kinds of nodes with the names of their fields, from the
    table of docs/file_formats.md.

    """

    with open(os.path.join("docs", "file_formats.md")) as fin:
        text = fin.read()

    table = text.split("<!-- kinds-begin -->")[1].split("<!-- kinds-end -->")[0]
    kinds = {}

    for line in table.splitlines():
        match = re.match(r"\|\s*(\d+)\s*\|\s*(\w+)\s*\|\s*(.*?)\s*\|$", line)

        if match is None:
            continue

        fields = []

        for field in match.group(3).split(", "):
            field = field.strip()

            if field == "--":
                continue

            if field == "*type fields*":
                fields += [
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
                ]
            elif field == "*constraint fields*":
                fields += list(CONSTRAINT_FIELDS)
            else:
                fields.append(field.split(" = ")[0])

        kinds[int(match.group(1))] = (match.group(2), fields)

    return kinds


KINDS = read_kinds()


def is_node(item: Any) -> bool:
    return isinstance(item, list) and item[0] != 0


def convert(shape: Any, item: Any) -> Any:
    """Returns given compact form as a value of the ASN.1 type that
    given kind of field has.

    """

    if isinstance(shape, list):
        assert item[0] == 0, item

        return [convert(shape[0], i) for i in item[1:]]

    if shape == "V":
        return convert_value(item)

    if shape == "T":
        kind = KINDS[item[0]][0]

        return (TYPES[kind], convert_node(item))

    if shape == "B":
        if is_node(item) and KINDS[item[0]][0] in BOUNDS:
            return (BOUNDS[KINDS[item[0]][0]], convert_node(item))

        return ("value", convert_value(item))

    if shape == "E":
        if is_node(item):
            kind = KINDS[item[0]][0]

            if kind in ("Range", "ExtensionMarker"):
                return (
                    "range" if kind == "Range" else "extension-marker",
                    convert_node(item),
                )

            if kind in TYPES:
                return ("type", convert(shape="T", item=item))

        return ("value", convert_value(item))

    if shape == "M":
        return (MEMBER_LIST_ITEMS[KINDS[item[0]][0]], convert_node(item))

    if shape == "C":
        if is_node(item):
            return ("extension-marker", convert_node(item))

        return ("alternative", [convert_node(component) for component in item[1:]])

    if shape == "W":
        kind = KINDS[item[0]][0]

        return (
            "alternative" if kind == "ElementConstraint" else "extension-marker",
            convert_node(item),
        )

    if shape == "N":
        kind = KINDS[item[0]][0]

        return (
            "named-number" if kind == "NamedNumber" else "extension-marker",
            convert_node(item),
        )

    if shape == "S":
        if isinstance(item, str):
            return ("token", item)

        return ("optional-group", convert_node(item))

    assert KINDS[item[0]][0] == shape, (item, shape)

    return convert_node(item)


def convert_value(item: Any) -> Any:
    if isinstance(item, bool):
        return ("boolean", item)

    if isinstance(item, int):
        return ("integer", item)

    if isinstance(item, float):
        return ("real", item)

    if isinstance(item, str):
        return ("text", item)

    if is_node(item):
        return (VALUES[KINDS[item[0]][0]], convert_node(item))

    assert item[0] == 0, item

    return ("list", [convert_value(i) for i in item[1:]])


def convert_node(node: list[Any]) -> dict[str, Any]:
    kind, fields = KINDS[node[0]]
    shapes = {**TYPE_FIELDS, **FIELDS.get(kind, {})}
    values = {}

    for name, item in zip(fields, node[1:]):
        if name in ("parameters", "actual_parameters"):
            assert item is None

            continue

        if item is None:
            continue

        # A list of text strings is plain.
        if name == "component_ids":
            item = item[1:]
        elif name in shapes:
            item = convert(shapes[name], item)

        values[name.replace("_", "-")] = item

    return values


def without_defaults(value: Any) -> Any:
    """Returns given value without the fields that are what they are by
    default, as a decoder gives them and an encoder leaves them out.

    """

    if isinstance(value, dict):
        return {
            key: without_defaults(item)
            for key, item in value.items()
            if key.replace("-", "_") not in DEFAULTS
            or DEFAULTS[key.replace("-", "_")] != item
            or isinstance(item, bool)
            != isinstance(DEFAULTS[key.replace("-", "_")], bool)
        }

    if isinstance(value, tuple):
        return tuple(without_defaults(item) for item in value)

    if isinstance(value, list):
        return [without_defaults(item) for item in value]

    return value


def golden_cases() -> list[tuple[str, Any]]:
    cases = []

    for filename in sorted(glob.glob(os.path.join(GOLDEN, "*"))):
        opener = gzip.open if filename.endswith(".gz") else open

        with opener(filename, "rb") as fin:
            case = json.load(fin)

        if case["compile"]["ok"]:
            cases.append((case["id"], case))

    return cases


def load_asni_asn(codec: CodecType) -> asnkitrt.Specification:
    case = [c for _, c in golden_cases() if c.get("files") == ["examples/asni.asn"]][0]

    return asnkitrt.load(syntax.from_compact(case["compile"]["nodes"]), codec)


class AsniAsnTest(unittest.TestCase):
    def check(self, codec: CodecType) -> None:
        asni = load_asni_asn(codec)
        cases = golden_cases()

        self.assertGreater(len(cases), 60)

        for case_id, case in cases:
            nodes = case["compile"]["nodes"]
            value = (  # The top node is a CompiledSpecification.
                convert_node(nodes)
            )

            with self.subTest(case=case_id):
                encoded = asni.encode("CompiledSpecification", value)
                decoded = asni.decode("CompiledSpecification", encoded)

                self.assertEqual(without_defaults(decoded), without_defaults(value))

    def test_der(self) -> None:
        self.check("der")

    def test_uper(self) -> None:
        self.check("uper")


if __name__ == "__main__":
    unittest.main()
