# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Convert nodes to and from primitives: dictionaries with string
keys, lists, strings, integers, floats, booleans and ``None``. That
is what serialization formats such as CBOR and JSON can carry.

There are two ways of doing so:

- Readable, by :func:`to_primitive()`. A node is a dictionary with
  its class name as ``'kind'`` and its fields as the other keys.
  Fields that are ``None`` are left out. Tuples are lists.

- Compact, by :func:`to_compact()`. A node is a list of the number of
  its kind followed by its fields in a fixed order. A field that is
  what it is by default is ``None``, and left out if nothing but such
  fields follow. A tuple is a list of 0 followed by its items.

"""

import dataclasses
import json
from typing import Any, Dict, Iterator, List, Type

from .convert import ConvertError
from .nodes import Node


def _node_classes(cls: Type[Node]) -> Iterator[Type[Node]]:
    yield cls

    for subclass in cls.__subclasses__():
        yield from _node_classes(subclass)


NODE_CLASSES: Dict[str, Type[Node]] = {cls.__name__: cls for cls in _node_classes(Node)}


def to_primitive(node: Any) -> Any:
    """Convert given syntax tree node to primitives.

    >>> asnkitrt.syntax.to_primitive(asnkitrt.syntax.Range(lower=1, upper=5))
    {'kind': 'Range', 'lower': 1, 'upper': 5}

    """

    if isinstance(node, Node):
        primitive = {"kind": type(node).__name__}

        for field in dataclasses.fields(node):
            value = getattr(node, field.name)

            if value is not None:
                primitive[field.name] = to_primitive(value)

        return primitive
    elif isinstance(node, tuple):
        return [to_primitive(item) for item in node]
    else:
        return node


def _from_primitive(primitive: Any, is_strict: bool) -> Any:
    if isinstance(primitive, list):
        return tuple(_from_primitive(item, is_strict) for item in primitive)
    elif not isinstance(primitive, dict):
        return primitive

    try:
        cls = NODE_CLASSES[primitive["kind"]]
    except KeyError:
        raise ConvertError("Unsupported node {!r}.".format(primitive.get("kind")))

    fields = {field.name for field in dataclasses.fields(cls)}
    unknown = set(primitive) - fields - {"kind"}

    if unknown and is_strict:
        raise ConvertError(
            "Unsupported field(s) {} in node '{}'.".format(
                ", ".join(repr(name) for name in sorted(unknown)), cls.__name__
            )
        )

    # Fields without a default may be missing as they are None.
    values = {
        field.name: _from_primitive(primitive.get(field.name), is_strict)
        for field in dataclasses.fields(cls)
        if field.name in primitive or field.default is dataclasses.MISSING
    }

    return cls(**values)


def from_primitive(primitive: Any, is_strict: bool = True) -> Any:
    """Convert given primitives, as returned by :func:`to_primitive()`,
    to a node.

    A field that is not known is an error, unless `is_strict` is
    ``False``. It is then ignored, as it is that of a later minor
    version.

    """

    return _from_primitive(primitive, is_strict)


# The number of each kind of node in the compact form. A number is
# never given to another kind, as that is what tells them apart in a
# file. Those of what is found the most in a compiled specification
# are below 24, which are the ones taking a single byte.
KIND_NUMBERS = {
    "CompiledSpecification": 1,
    "CompiledModule": 2,
    "TypeAssignment": 3,
    "Member": 4,
    "Tag": 5,
    "Range": 6,
    "NamedNumber": 7,
    "ExtensionMarker": 8,
    "ExtensionAdditionGroup": 9,
    "BuiltinType": 10,
    "ReferencedType": 11,
    "IntegerType": 12,
    "BitStringType": 13,
    "EnumeratedType": 14,
    "SequenceType": 15,
    "SetType": 16,
    "ChoiceType": 17,
    "SequenceOfType": 18,
    "SetOfType": 19,
    "Min": 20,
    "Max": 21,
    "Identifier": 22,
    "HexString": 23,
    "BinaryString": 24,
    "Null": 25,
    "PlusInfinity": 26,
    "MinusInfinity": 27,
    "NotANumber": 28,
    "NamedValue": 29,
    "SequenceValue": 30,
    "UnsupportedValue": 31,
    "Excluded": 32,
    "TableConstraint": 33,
    "ComponentConstraint": 34,
    "ElementConstraint": 35,
    "AnyDefinedByType": 36,
    "AnyDefinedByChoice": 37,
    "ComponentsOf": 38,
    "ObjectClassAssignment": 39,
    "ObjectClassField": 40,
    "SyntaxTree": 41,
    "Module": 42,
    "Import": 43,
    "ValueAssignment": 44,
    "ObjectAssignment": 45,
    "ObjectSetAssignment": 46,
    "Object": 47,
    "ObjectField": 48,
    "UnresolvedObject": 49,
    "ObjectSet": 50,
    "OptionalGroup": 51,
    "Parameter": 52,
    "ElementSetSpecs": 53,
    "Union": 54,
    "Intersection": 55,
    "Except": 56,
    "All": 57,
    "SizeConstraint": 58,
    "PermittedAlphabet": 59,
    "ContainedSubtype": 60,
    "PatternConstraint": 61,
    "InnerTypeConstraint": 62,
    "InnerTypeConstraints": 63,
    "NamedConstraint": 64,
    "ContentsConstraint": 65,
    "UserDefinedConstraint": 66,
    "UnsupportedConstraint": 67,
}

KIND_NAMES = {number: name for name, number in KIND_NUMBERS.items()}

# First in the list of a tuple.
TUPLE = 0


# The fields of each kind of node in the order of the compact form.
# Those of what a node is a kind of, as a type, come after its own,
# as they are seldom given.
#
# A field is only ever added last in a list, as its position is what
# tells it apart in a file.
COMPACT_FIELD_NAMES = {
    "CompiledSpecification": ["modules"],
    "CompiledModule": ["name", "types", "object_classes"],
    "TypeAssignment": ["name", "type", "location"],
    "Member": ["name", "type", "optional", "default", "location"],
    "Tag": ["number", "tag_class", "mode"],
    "Range": ["lower", "upper"],
    "NamedNumber": ["name", "number"],
    "ExtensionMarker": [],
    "ExtensionAdditionGroup": ["members"],
    "BuiltinType": [
        "name",
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
        "constraints",
    ],
    "ReferencedType": [
        "name",
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
        "constraints",
    ],
    "IntegerType": [
        "named_numbers",
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
        "constraints",
    ],
    "BitStringType": [
        "named_bits",
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
        "constraints",
    ],
    "EnumeratedType": [
        "values",
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
        "constraints",
    ],
    "SequenceType": [
        "members",
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
        "constraints",
    ],
    "SetType": [
        "members",
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
        "constraints",
    ],
    "ChoiceType": [
        "members",
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
        "constraints",
    ],
    "SequenceOfType": [
        "element",
        "element_name",
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
        "constraints",
    ],
    "SetOfType": [
        "element",
        "element_name",
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
        "constraints",
    ],
    "Min": [],
    "Max": [],
    "Identifier": ["name"],
    "HexString": ["digits"],
    "BinaryString": ["bits"],
    "Null": [],
    "PlusInfinity": [],
    "MinusInfinity": [],
    "NotANumber": [],
    "NamedValue": ["name", "value"],
    "SequenceValue": ["members"],
    "UnsupportedValue": [],
    "Excluded": ["value"],
    "TableConstraint": ["object_set", "component_ids"],
    "ComponentConstraint": [
        "name",
        "presence",
        "size",
        "restricted_to",
        "permitted_alphabet",
        "with_components",
        "with_component",
        "containing",
    ],
    "ElementConstraint": [
        "size",
        "restricted_to",
        "permitted_alphabet",
        "with_components",
        "with_component",
        "containing",
    ],
    "AnyDefinedByType": [
        "value",
        "choices",
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
        "constraints",
    ],
    "AnyDefinedByChoice": ["value", "type"],
    "ComponentsOf": ["type_name"],
    "ObjectClassAssignment": ["name", "fields", "syntax", "location"],
    "ObjectClassField": ["name", "type", "location"],
    "SyntaxTree": ["modules"],
    "Module": [
        "name",
        "tags",
        "extensibility_implied",
        "imports",
        "types",
        "values",
        "objects",
        "object_classes",
        "object_sets",
        "location",
    ],
    "Import": ["module", "symbols"],
    "ValueAssignment": ["name", "type", "value", "location"],
    "ObjectAssignment": ["name", "class_name", "object", "location"],
    "ObjectSetAssignment": ["name", "class_name", "members", "location"],
    "Object": ["fields"],
    "ObjectField": ["name", "setting"],
    "UnresolvedObject": ["source"],
    "ObjectSet": ["members"],
    "OptionalGroup": ["tokens"],
    "Parameter": ["name", "governor"],
    "ElementSetSpecs": ["root", "extensible", "additions"],
    "Union": ["elements"],
    "Intersection": ["elements"],
    "Except": ["elements", "exclusions"],
    "All": ["exclusions"],
    "SizeConstraint": ["constraint"],
    "PermittedAlphabet": ["constraint"],
    "ContainedSubtype": ["type", "includes"],
    "PatternConstraint": ["value"],
    "InnerTypeConstraint": ["constraint"],
    "InnerTypeConstraints": ["partial", "components"],
    "NamedConstraint": ["name", "constraints", "presence"],
    "ContentsConstraint": ["type", "encoded_by"],
    "UserDefinedConstraint": [],
    "UnsupportedConstraint": [],
}


def _compact_fields(name: str) -> List[Any]:
    fields = {field.name: field for field in dataclasses.fields(NODE_CLASSES[name])}

    return [fields[field_name] for field_name in COMPACT_FIELD_NAMES[name]]


COMPACT_FIELDS = {name: _compact_fields(name) for name in KIND_NUMBERS}


def _is_default(field: Any, value: Any) -> bool:
    if value is None:
        return True

    if field.default is dataclasses.MISSING:
        return False

    return type(value) is type(field.default) and value == field.default


def to_compact(node: Any) -> Any:
    """Convert given node to primitives in the compact form.

    >>> asnkitrt.syntax.to_compact(asnkitrt.syntax.Range(lower=1, upper=5))
    [6, 1, 5]

    """

    if isinstance(node, Node):
        name = type(node).__name__

        try:
            compact: List[Any] = [KIND_NUMBERS[name]]
        except KeyError:
            raise ConvertError("Unsupported node '{}'.".format(name))

        for field in COMPACT_FIELDS[name]:
            value = getattr(node, field.name)

            if _is_default(field, value):
                compact.append(None)
            else:
                compact.append(to_compact(value))

        while compact[-1] is None:
            compact.pop()

        return compact
    elif isinstance(node, tuple):
        return [TUPLE] + [to_compact(item) for item in node]
    else:
        return node


def from_compact(compact: Any) -> Any:
    """Convert given primitives in the compact form, as returned by
    :func:`to_compact()`, to a node.

    Fields past those known are ignored, as they are those of a later
    minor version.

    """

    if not isinstance(compact, list):
        return compact

    if not compact or isinstance(compact[0], bool) or not isinstance(compact[0], int):
        raise ConvertError("Expected the number of a kind of node first in a list.")

    if compact[0] == TUPLE:
        return tuple(from_compact(item) for item in compact[1:])

    try:
        name = KIND_NAMES[compact[0]]
    except KeyError:
        raise ConvertError("Unsupported kind of node {}.".format(compact[0]))

    values = {}

    for field, value in zip(COMPACT_FIELDS[name], compact[1:]):
        if value is not None:
            values[field.name] = from_compact(value)

    for field in COMPACT_FIELDS[name]:
        if field.name not in values and field.default is dataclasses.MISSING:
            values[field.name] = None

    return NODE_CLASSES[name](**values)


def to_json(tree: Any, indent: int | None = 4) -> str:
    """Return given syntax tree or compiled specification, or any node
    of them, as JSON. This is meant for looking at one, not for
    storing it.

    >>> print(asnkitrt.syntax.to_json(tree))

    """

    return json.dumps(to_primitive(tree), indent=indent)
