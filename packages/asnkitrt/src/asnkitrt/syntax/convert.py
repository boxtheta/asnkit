# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Convert between the dictionaries returned by the parser and the
syntax tree.

The conversions are lossless: ``to_dict(from_dict(d)) == d`` for any
dictionary `d` returned by the parser.

"""

import contextvars
import re
from typing import Any, Dict, List

from ..constants import CString, Located, ValueRange
from ..errors import Error
from .nodes import (
    AnyDefinedByChoice,
    AnyDefinedByType,
    BinaryString,
    BitStringType,
    BuiltinType,
    ChoiceType,
    CompiledModule,
    CompiledSpecification,
    ComponentConstraint,
    ComponentsOf,
    ElementConstraint,
    EnumeratedType,
    Excluded,
    ExtensionAdditionGroup,
    ExtensionMarker,
    HexString,
    Identifier,
    Import,
    IntegerType,
    Max,
    Member,
    MemberListItem,
    Min,
    MinusInfinity,
    Module,
    NamedNumber,
    NamedValue,
    NotANumber,
    Null,
    Object,
    ObjectAssignment,
    ObjectClassAssignment,
    ObjectClassField,
    ObjectField,
    ObjectSet,
    ObjectSetAssignment,
    OptionalGroup,
    Parameter,
    PlusInfinity,
    Range,
    ReferencedType,
    SequenceOfType,
    SequenceType,
    SequenceValue,
    SetOfType,
    SetType,
    SyntaxTree,
    TableConstraint,
    Tag,
    Type,
    TypeAssignment,
    UnresolvedObject,
    UnsupportedValue,
    ValueAssignment,
)

BUILTIN_TYPE_NAMES = frozenset(
    [
        "ANY",
        "BMPString",
        "BOOLEAN",
        "CHARACTER STRING",
        "DATE",
        "DATE-TIME",
        "DURATION",
        "EMBEDDED PDV",
        "EXTERNAL",
        "GeneralString",
        "GeneralizedTime",
        "GraphicString",
        "IA5String",
        "ISO646String",
        "NULL",
        "NumericString",
        "OBJECT IDENTIFIER",
        "OCTET STRING",
        "ObjectDescriptor",
        # Type field of an object class.
        "OpenType",
        "PrintableString",
        "REAL",
        "T61String",
        "TIME-OF-DAY",
        "TeletexString",
        "UTCTime",
        "UTF8String",
        "UniversalString",
        "VideotexString",
        "VisibleString",
    ]
)

# Keys any type dictionary may have, with their Type field.
COMMON_TYPE_KEYS = {
    "tag": "tag",
    "size": "size",
    "restricted-to": "restricted_to",
    "from": "permitted_alphabet",
    "with-components": "with_components",
    "with-component": "with_component",
    "table": "table",
    "parameters": "parameters",
    "parameter-governors": "parameters",
    "actual-parameters": "actual_parameters",
    "module-name": "module_name",
}

MEMBER_KEYS = ["name", "optional", "default"]


class ConvertError(Error):
    pass


def find_assignment(
    specification: Dict[str, Any], module_name: Any, kind: str, name: str
) -> Any:
    """Returns the assignment of given kind and name as seen from
    given module, and the name of the module it is found in, or None
    and None if it cannot be found.

    """

    visited = set()

    while module_name in specification and module_name not in visited:
        visited.add(module_name)
        module = specification[module_name]

        if name in module.get(kind, {}):
            return module[kind][name], module_name

        for from_module_name, symbols in module.get("imports", {}).items():
            if name in symbols:
                module_name = from_module_name
                break

    return None, None


def is_object_class_name(name: str) -> bool:
    """Returns True if given name of a type or object class can be
    that of an object class. They have no lower case letters, just as
    some built-in types.

    """

    return (
        name == name.upper()
        and name not in BUILTIN_TYPE_NAMES
        and name not in ["INTEGER", "BIT STRING", "ENUMERATED", "SEQUENCE", "SET"]
        and name not in ["CHOICE", "SEQUENCE OF", "SET OF", "ANY DEFINED BY"]
    )


# How to tell what a plain string in a specification dictionary is,
# given where it is found. Strings given in quotes are CString.
def _location_from_data(data: Any) -> Any:
    """Returns where what given dictionary describes is found, if
    known. A compiled specification does not tell.

    """

    if IS_RESOLVED.get():
        return None

    return getattr(data, "location", None)


def _located(data: Dict[str, Any], node: Any) -> Dict[str, Any]:
    """Returns given dictionary as one that knows where it is found,
    if given node does.

    """

    if node.location is None:
        return data

    located = Located(data)
    located.location = tuple(node.location)

    return located


VALUE = "value"
CONSTRAINT = "constraint"
PERMITTED_ALPHABET = "permitted-alphabet"

RE_IDENTIFIER = re.compile(r"[a-z][a-zA-Z0-9]*(-[a-zA-Z0-9]+)*")
RE_BINARY_STRING = re.compile(r"0b[01]*")
RE_HEX_STRING = re.compile(r"0x[0-9a-fA-F]*")


# True when converting a resolved specification to a dictionary, in
# which the default value of a BIT STRING or an OCTET STRING is what
# it is when encoded and decoded.
IS_RESOLVED: contextvars.ContextVar[bool] = contextvars.ContextVar(
    "IS_RESOLVED", default=False
)

# Words that are values of their own.
WORD_VALUES = {
    "PLUS-INFINITY": PlusInfinity,
    "MINUS-INFINITY": MinusInfinity,
    "NOT-A-NUMBER": NotANumber,
}


def _value_from_data(
    data: Any, where: str = VALUE, type_name: Any = None, is_nested: bool = False
) -> Any:
    """Convert given value of a specification dictionary to a value of
    the syntax tree. `type_name` is the name of the type it is a value
    of, if known.

    """

    if isinstance(data, bytes):
        # The default value of an OCTET STRING in a resolved
        # specification.
        return HexString(digits=data.hex())
    elif (
        isinstance(data, tuple)
        and len(data) == 2
        and isinstance(data[0], bytes)
        and isinstance(data[1], int)
    ):
        # The default value of a BIT STRING in a resolved
        # specification: the bits as bytes and how many they are.
        bits = "".join("{:08b}".format(byte) for byte in data[0])

        return BinaryString(bits=bits[: data[1]])
    elif data is None:
        # Nothing but NULL is None within a value. On its own it is
        # also what the parser gives for a value it does not support.
        if is_nested or type_name == "NULL":
            return Null()
        else:
            return UnsupportedValue()
    elif isinstance(data, (bool, int, float)):
        return data
    elif isinstance(data, list):
        return tuple(_value_from_data(item, where, None, True) for item in data)
    elif isinstance(data, tuple) and len(data) == 2:
        return NamedValue(
            name=data[0], value=_value_from_data(data[1], where, None, True)
        )
    elif isinstance(data, dict):
        return SequenceValue(
            members=tuple(
                NamedValue(name=name, value=_value_from_data(value, where, None, True))
                for name, value in data.items()
            )
        )
    elif not isinstance(data, str):
        raise ConvertError("Unsupported value {!r}.".format(data))
    elif isinstance(data, CString) or where == PERMITTED_ALPHABET:
        return str(data)
    elif where == CONSTRAINT and data == "MIN":
        return Min()
    elif where == CONSTRAINT and data == "MAX":
        return Max()
    elif where == CONSTRAINT and data in ["TRUE", "FALSE"]:
        return data == "TRUE"
    elif data in WORD_VALUES:
        return WORD_VALUES[data]()
    elif RE_BINARY_STRING.fullmatch(data):
        return BinaryString(bits=data[2:])
    elif RE_HEX_STRING.fullmatch(data):
        return HexString(digits=data[2:])
    elif RE_IDENTIFIER.fullmatch(data):
        return Identifier(name=data)
    else:
        # A string not given in quotes that is none of the above is
        # taken as a character string, as it is one in a dictionary
        # not coming from the parser. From the parser it is a word
        # the parser does not know of.
        return str(data)


def _value_to_data(value: Any, where: str = VALUE) -> Any:
    if isinstance(value, (UnsupportedValue, Null)):
        return None
    elif isinstance(value, bool):
        if where == CONSTRAINT:
            return "TRUE" if value else "FALSE"
        else:
            return value
    elif isinstance(value, tuple):
        return [_value_to_data(item, where) for item in value]
    elif isinstance(value, NamedValue):
        return (value.name, _value_to_data(value.value, where))
    elif isinstance(value, SequenceValue):
        return {
            member.name: _value_to_data(member.value, where) for member in value.members
        }
    elif isinstance(value, str):
        return CString(value)
    elif isinstance(value, Identifier):
        return value.name
    elif isinstance(value, Min):
        return "MIN"
    elif isinstance(value, Max):
        return "MAX"
    elif isinstance(value, BinaryString):
        if IS_RESOLVED.get():
            bits = value.bits + "0" * (-len(value.bits) % 8)

            return (
                bytes(int(bits[i : i + 8], 2) for i in range(0, len(bits), 8)),
                len(value.bits),
            )
        else:
            return "0b" + value.bits
    elif isinstance(value, HexString):
        if IS_RESOLVED.get():
            return bytes.fromhex(value.digits)
        else:
            return "0x" + value.digits
    else:
        for word, cls in WORD_VALUES.items():
            if isinstance(value, cls):
                return word

        return value


def _number_from_data(number: Any) -> Any:
    """Convert given number of a tag or a named number, which is an
    identifier if given by reference.

    """

    if isinstance(number, str):
        if number.isdigit():
            return int(number)
        else:
            return Identifier(name=str(number))
    else:
        return number


def _number_to_data(number: Any) -> Any:
    if isinstance(number, Identifier):
        return number.name
    else:
        return number


def _named_number_from_pair(pair: Any) -> NamedNumber:
    return NamedNumber(name=pair[0], number=_number_from_data(pair[1]))


def _named_number_to_pair(named_number: NamedNumber) -> Any:
    return (named_number.name, _number_to_data(named_number.number))


def _tag_from_dict(tag: Dict[str, Any]) -> Tag:
    return Tag(
        number=_number_from_data(tag["number"]),
        tag_class=tag.get("class"),
        mode=tag.get("kind"),
    )


def _tag_to_dict(tag: Tag) -> Dict[str, Any]:
    converted: Dict[str, Any] = {"number": _number_to_data(tag.number)}

    if tag.tag_class is not None:
        converted["class"] = tag.tag_class

    if tag.mode is not None:
        converted["kind"] = tag.mode

    return converted


def _constraint_from_list(elements: List[Any], where: str) -> Any:
    converted: List[Any] = []

    for element in elements:
        if element is None:
            converted.append(ExtensionMarker())
        elif isinstance(element, tuple) and len(element) == 2:
            lower = _value_from_data(element[0], where)
            upper = _value_from_data(element[1], where)

            if getattr(element, "is_minimum_excluded", False):
                lower = Excluded(value=lower)

            if getattr(element, "is_maximum_excluded", False):
                upper = Excluded(value=upper)

            converted.append(Range(lower=lower, upper=upper))
        elif isinstance(element, dict):
            converted.append(_type_from_dict(element))
        elif isinstance(element, (int, float, str)):
            converted.append(_value_from_data(element, where))
        else:
            raise ConvertError("Unsupported constraint element {!r}.".format(element))

    return tuple(converted)


def _bound_to_data(bound: Any, where: str) -> Any:
    """Returns given bound of a range as a value of a specification
    dictionary, and if it is excluded.

    """

    if isinstance(bound, Excluded):
        return _value_to_data(bound.value, where), True
    else:
        return _value_to_data(bound, where), False


def _constraint_to_list(elements: Any, where: str) -> List[Any]:
    converted: List[Any] = []

    for element in elements:
        if isinstance(element, ExtensionMarker):
            converted.append(None)
        elif isinstance(element, Range):
            lower, is_lower_excluded = _bound_to_data(element.lower, where)
            upper, is_upper_excluded = _bound_to_data(element.upper, where)

            if is_lower_excluded or is_upper_excluded:
                converted.append(
                    ValueRange(lower, upper, is_lower_excluded, is_upper_excluded)
                )
            else:
                converted.append((lower, upper))
        elif isinstance(element, Type):
            converted.append(_type_to_dict(element))
        else:
            converted.append(_value_to_data(element, where))

    return converted


# Key to the node field, and how to tell what a string is.
CONSTRAINT_KEYS = {
    "size": ("size", CONSTRAINT),
    "restricted-to": ("restricted_to", CONSTRAINT),
    "from": ("permitted_alphabet", PERMITTED_ALPHABET),
}


def _constraints_from_dict(constrained: Dict[str, Any]) -> Dict[str, Any]:
    """Returns the constraints found in given type or inner constraint
    dictionary as node fields.

    """

    constraints: Dict[str, Any] = {}

    for key, (field, where) in CONSTRAINT_KEYS.items():
        if key in constrained:
            constraints[field] = _constraint_from_list(constrained[key], where)

    if "with-components" in constrained:
        constraints["with_components"] = tuple(
            (
                ExtensionMarker()
                if alternative is None
                else tuple(
                    _component_constraint_from_dict(component)
                    for component in alternative
                )
            )
            for alternative in constrained["with-components"]
        )

    if "with-component" in constrained:
        constraints["with_component"] = tuple(
            (
                ExtensionMarker()
                if alternative is None
                else ElementConstraint(
                    **_inner_constraint_from_dict(alternative, [], "element")
                )
            )
            for alternative in constrained["with-component"]
        )

    return constraints


def _constraints_to_dict(constrained: Any, converted: Dict[str, Any]) -> None:
    for key, (field, where) in CONSTRAINT_KEYS.items():
        if getattr(constrained, field) is not None:
            converted[key] = _constraint_to_list(getattr(constrained, field), where)

    if constrained.with_components is not None:
        converted["with-components"] = [
            (
                None
                if isinstance(alternative, ExtensionMarker)
                else [
                    _component_constraint_to_dict(component)
                    for component in alternative
                ]
            )
            for alternative in constrained.with_components
        ]

    if constrained.with_component is not None:
        converted["with-component"] = [
            (
                None
                if isinstance(alternative, ExtensionMarker)
                else _inner_constraint_to_dict(alternative, {})
            )
            for alternative in constrained.with_component
        ]


def _inner_constraint_from_dict(
    constraint: Dict[str, Any], other_keys: List[str], what: str
) -> Dict[str, Any]:
    """Returns the constraints found in given WITH COMPONENT or WITH
    COMPONENTS constraint dictionary as node fields.

    """

    unsupported = set(constraint) - set(CONSTRAINT_KEYS) - set(other_keys)
    unsupported -= {"with-components", "with-component", "containing"}

    if unsupported:
        raise ConvertError(
            "Unsupported key(s) {} in constraint of {}.".format(
                ", ".join(repr(key) for key in sorted(unsupported)), what
            )
        )

    constraints = _constraints_from_dict(constraint)

    if "containing" in constraint:
        constraints["containing"] = _type_from_dict(constraint["containing"])

    return constraints


def _inner_constraint_to_dict(
    constraint: ElementConstraint, converted: Dict[str, Any]
) -> Dict[str, Any]:
    _constraints_to_dict(constraint, converted)

    if constraint.containing is not None:
        converted["containing"] = _type_to_dict(constraint.containing)

    return converted


def _component_constraint_from_dict(component: Dict[str, Any]) -> ComponentConstraint:
    return ComponentConstraint(
        name=component["name"],
        presence=component.get("presence"),
        **_inner_constraint_from_dict(
            component,
            ["name", "presence"],
            "component '{}'".format(component["name"]),
        ),
    )


def _component_constraint_to_dict(component: ComponentConstraint) -> Dict[str, Any]:
    converted = _inner_constraint_to_dict(component, {"name": component.name})

    if component.presence is not None:
        converted["presence"] = component.presence

    return converted


def _members_from_list(members: List[Any]) -> Any:
    converted: List[MemberListItem] = []

    for member in members:
        if member is None:
            converted.append(ExtensionMarker())
        elif isinstance(member, list):
            converted.append(ExtensionAdditionGroup(members=_members_from_list(member)))
        elif "components-of" in member:
            converted.append(ComponentsOf(type_name=member["components-of"]))
        else:
            default = None

            if "default" in member:
                default = _value_from_data(member["default"], type_name=member["type"])

            converted.append(
                Member(
                    name=member["name"],
                    type=_type_from_dict(member, MEMBER_KEYS),
                    optional=member.get("optional", False),
                    default=default,
                    location=_location_from_data(member),
                )
            )

    return tuple(converted)


def _members_to_list(members: Any) -> List[Any]:
    converted: List[Any] = []

    for member in members:
        if isinstance(member, ExtensionMarker):
            converted.append(None)
        elif isinstance(member, ExtensionAdditionGroup):
            converted.append(_members_to_list(member.members))
        elif isinstance(member, ComponentsOf):
            converted.append({"components-of": member.type_name})
        else:
            converted_member = _type_to_dict(member.type)
            converted_member["name"] = member.name

            if member.optional:
                converted_member["optional"] = True

            if member.default is not None:
                converted_member["default"] = _value_to_data(member.default)

            converted.append(_located(converted_member, member))

    return converted


def _syntax_from_list(tokens: List[Any]) -> Any:
    return tuple(
        (
            OptionalGroup(tokens=_syntax_from_list(token))
            if isinstance(token, list)
            else token
        )
        for token in tokens
    )


def _syntax_to_list(tokens: Any) -> List[Any]:
    return [
        _syntax_to_list(token.tokens) if isinstance(token, OptionalGroup) else token
        for token in tokens
    ]


def _object_from_dict(object_: Dict[str, Any]) -> Any:
    if "defined-syntax" in object_:
        return UnresolvedObject(source=object_["defined-syntax"])

    return Object(
        fields=tuple(
            ObjectField(
                name=name,
                setting=(
                    _type_from_dict(setting)
                    if isinstance(setting, dict) and "type" in setting
                    else _value_from_data(setting)
                ),
            )
            for name, setting in object_.items()
        )
    )


def _object_to_dict(object_: Any) -> Dict[str, Any]:
    if isinstance(object_, UnresolvedObject):
        return {"defined-syntax": object_.source}

    return {
        field.name: (
            _type_to_dict(field.setting)
            if isinstance(field.setting, Type)
            else _value_to_data(field.setting)
        )
        for field in object_.fields
    }


def _object_set_members_from_list(members: List[Any]) -> Any:
    converted: List[Any] = []

    for member in members:
        if member is None:
            converted.append(ExtensionMarker())
        elif isinstance(member, dict):
            converted.append(_object_from_dict(member))
        else:
            converted.append(member)

    return tuple(converted)


def _object_set_members_to_list(members: Any) -> List[Any]:
    converted: List[Any] = []

    for member in members:
        if isinstance(member, ExtensionMarker):
            converted.append(None)
        elif isinstance(member, (Object, UnresolvedObject)):
            converted.append(_object_to_dict(member))
        else:
            converted.append(member)

    return converted


def _is_object(data: Any) -> bool:
    return isinstance(data, dict) and (
        "defined-syntax" in data or all(name.startswith("&") for name in data)
    )


def _is_object_assignment(
    specification: Dict[str, Any], module_name: str, assignment: Dict[str, Any]
) -> bool:
    """Returns True if given assignment found among the values is that
    of an object. One referencing another object looks just like a
    value assignment, and is only told apart if its class is found.

    """

    value = assignment["value"]

    if _is_object(value):
        return True

    # A value in quotes, a number or anything in braces is no
    # reference to an object.
    if value is not None and (
        not isinstance(value, str)
        or isinstance(value, CString)
        or RE_IDENTIFIER.fullmatch(value) is None
    ):
        return False

    object_class, _ = find_assignment(
        specification, module_name, "object-classes", assignment["type"]
    )

    return object_class is not None


def _assigned_object_from_data(data: Any) -> Any:
    if isinstance(data, dict):
        return _object_from_dict(data)
    elif data is None:
        return UnsupportedValue()
    else:
        return Identifier(name=str(data))


def _assigned_object_to_data(object_: Any) -> Any:
    if isinstance(object_, (Object, UnresolvedObject)):
        return _object_to_dict(object_)
    elif isinstance(object_, Identifier):
        return object_.name
    else:
        return None


def _actual_parameter_from_dict(parameter: Any) -> Any:
    if not isinstance(parameter, dict):
        return _value_from_data(parameter)
    elif "object-set" in parameter:
        return ObjectSet(members=_object_set_members_from_list(parameter["object-set"]))
    elif _is_object(parameter):
        return _object_from_dict(parameter)
    else:
        return _type_from_dict(parameter)


def _actual_parameter_to_dict(parameter: Any) -> Any:
    if isinstance(parameter, Type):
        return _type_to_dict(parameter)
    elif isinstance(parameter, ObjectSet):
        return {"object-set": _object_set_members_to_list(parameter.members)}
    elif isinstance(parameter, (Object, UnresolvedObject)):
        return _object_to_dict(parameter)
    else:
        return _value_to_data(parameter)


def _table_from_data(table: Any) -> TableConstraint:
    if isinstance(table, dict):
        return TableConstraint(object_set=table["type"])
    elif isinstance(table[0], str):
        return TableConstraint(object_set=table[0], component_ids=tuple(table[1]))
    else:
        raise ConvertError("Unsupported table constraint {!r}.".format(table))


def _type_from_dict(type_: Dict[str, Any], ignored_keys: Any = ()) -> Type:
    """Convert given type dictionary, not looking at `ignored_keys`,
    which are those the caller takes care of.

    """

    keys = set(type_) - set(ignored_keys) - {"type"}
    common: Dict[str, Any] = {}

    if "tag" in type_:
        common["tag"] = _tag_from_dict(type_["tag"])

    common.update(_constraints_from_dict(type_))

    if type_.get("table") is not None:
        common["table"] = _table_from_data(type_["table"])

    if "module-name" in type_:
        common["module_name"] = type_["module-name"]

    if "parameters" in type_:
        names = type_["parameters"]
        governors = type_.get("parameter-governors", [None] * len(names))
        common["parameters"] = tuple(
            Parameter(
                name=name,
                governor=None if governor is None else _type_from_dict(governor),
            )
            for name, governor in zip(names, governors)
        )

    if "actual-parameters" in type_:
        common["actual_parameters"] = tuple(
            _actual_parameter_from_dict(parameter)
            for parameter in type_["actual-parameters"]
        )

    keys -= set(COMMON_TYPE_KEYS)
    name = type_["type"]
    converted: Type

    def take(key: str) -> Any:
        keys.discard(key)

        return type_[key]

    if name == "INTEGER":
        named_numbers = None

        if "named-numbers" in type_:
            named_numbers = tuple(
                _named_number_from_pair(pair) for pair in take("named-numbers").items()
            )

        converted = IntegerType(named_numbers=named_numbers, **common)
    elif name == "BIT STRING":
        named_bits = None

        if "named-bits" in type_:
            named_bits = tuple(
                _named_number_from_pair(pair) for pair in take("named-bits")
            )

        converted = BitStringType(named_bits=named_bits, **common)
    elif name == "ENUMERATED":
        converted = EnumeratedType(
            values=tuple(
                (ExtensionMarker() if value is None else _named_number_from_pair(value))
                for value in take("values")
            ),
            **common,
        )
    elif name == "SEQUENCE":
        converted = SequenceType(members=_members_from_list(take("members")), **common)
    elif name == "SET":
        converted = SetType(members=_members_from_list(take("members")), **common)
    elif name == "CHOICE":
        converted = ChoiceType(members=_members_from_list(take("members")), **common)
    elif name in ["SEQUENCE OF", "SET OF"]:
        element = _type_from_dict(take("element"))
        element_name = take("element_name") if "element_name" in type_ else None

        if name == "SEQUENCE OF":
            converted = SequenceOfType(
                element=element, element_name=element_name, **common
            )
        else:
            converted = SetOfType(element=element, element_name=element_name, **common)
    elif name == "ANY DEFINED BY":
        converted = AnyDefinedByType(
            value=take("value"),
            choices=tuple(
                AnyDefinedByChoice(value=value, type=_type_from_dict(choice))
                for value, choice in take("choices").items()
            ),
            **common,
        )
    elif name in BUILTIN_TYPE_NAMES:
        converted = BuiltinType(name=name, **common)
    else:
        converted = ReferencedType(name=name, **common)

    if keys:
        raise ConvertError(
            "Unsupported key(s) {} in type '{}'.".format(
                ", ".join(repr(key) for key in sorted(keys)), name
            )
        )

    return converted


def _type_to_dict(type_: Type) -> Dict[str, Any]:
    converted: Dict[str, Any] = {}

    if isinstance(type_, (BuiltinType, ReferencedType)):
        converted["type"] = type_.name
    elif isinstance(type_, IntegerType):
        converted["type"] = "INTEGER"

        if type_.named_numbers is not None:
            converted["named-numbers"] = {
                item.name: _number_to_data(item.number) for item in type_.named_numbers
            }
    elif isinstance(type_, BitStringType):
        converted["type"] = "BIT STRING"

        if type_.named_bits is not None:
            converted["named-bits"] = [
                # The number of a bit is a string.
                (item.name, str(_number_to_data(item.number)))
                for item in type_.named_bits
            ]
    elif isinstance(type_, EnumeratedType):
        converted["type"] = "ENUMERATED"
        converted["values"] = [
            (
                None
                if isinstance(value, ExtensionMarker)
                else _named_number_to_pair(value)
            )
            for value in type_.values
        ]
    elif isinstance(type_, (SequenceType, SetType, ChoiceType)):
        if isinstance(type_, SequenceType):
            converted["type"] = "SEQUENCE"
        elif isinstance(type_, SetType):
            converted["type"] = "SET"
        else:
            converted["type"] = "CHOICE"

        converted["members"] = _members_to_list(type_.members)
    elif isinstance(type_, (SequenceOfType, SetOfType)):
        if isinstance(type_, SequenceOfType):
            converted["type"] = "SEQUENCE OF"
        else:
            converted["type"] = "SET OF"

        converted["element"] = _type_to_dict(type_.element)

        if type_.element_name is not None:
            converted["element_name"] = type_.element_name
    elif isinstance(type_, AnyDefinedByType):
        converted["type"] = "ANY DEFINED BY"
        converted["value"] = type_.value
        converted["choices"] = {
            choice.value: _type_to_dict(choice.type) for choice in type_.choices
        }
    else:
        raise ConvertError("Unsupported type node {!r}.".format(type_))

    if type_.tag is not None:
        converted["tag"] = _tag_to_dict(type_.tag)

    _constraints_to_dict(type_, converted)

    if type_.table is not None:
        if type_.table.component_ids is None:
            converted["table"] = {"type": type_.table.object_set}
        else:
            converted["table"] = [
                type_.table.object_set,
                list(type_.table.component_ids),
            ]
    elif "&" in converted["type"]:
        # A field of an object class always has it.
        converted["table"] = None

    if type_.parameters is not None:
        converted["parameters"] = [parameter.name for parameter in type_.parameters]

        if any(parameter.governor is not None for parameter in type_.parameters):
            converted["parameter-governors"] = [
                (
                    None
                    if parameter.governor is None
                    else _type_to_dict(parameter.governor)
                )
                for parameter in type_.parameters
            ]

    if type_.module_name is not None:
        converted["module-name"] = type_.module_name

    if type_.actual_parameters is not None:
        converted["actual-parameters"] = [
            _actual_parameter_to_dict(parameter)
            for parameter in type_.actual_parameters
        ]

    return converted


def _module_from_dict(
    specification: Dict[str, Any], name: str, module: Dict[str, Any]
) -> Module:
    object_names = {
        value_name
        for value_name, value in module.get("values", {}).items()
        if _is_object_assignment(specification, name, value)
    }

    return Module(
        name=name,
        location=_location_from_data(module),
        tags=module.get("tags"),
        extensibility_implied=module.get("extensibility-implied", False),
        imports=tuple(
            Import(module=from_name, symbols=tuple(symbols))
            for from_name, symbols in module.get("imports", {}).items()
        ),
        types=tuple(
            TypeAssignment(
                name=type_name,
                type=_type_from_dict(type_),
                location=_location_from_data(type_),
            )
            for type_name, type_ in module.get("types", {}).items()
        ),
        values=tuple(
            ValueAssignment(
                name=value_name,
                type=value["type"],
                value=_value_from_data(value["value"], type_name=value["type"]),
                location=_location_from_data(value),
            )
            for value_name, value in module.get("values", {}).items()
            if value_name not in object_names
        ),
        objects=tuple(
            ObjectAssignment(
                name=object_name,
                class_name=object_["type"],
                object=_assigned_object_from_data(object_["value"]),
                location=_location_from_data(object_),
            )
            for object_name, object_ in module.get("values", {}).items()
            if object_name in object_names
        ),
        object_classes=_object_classes_from_dict(module.get("object-classes", {})),
        object_sets=tuple(
            ObjectSetAssignment(
                name=set_name,
                class_name=object_set["class"],
                members=_object_set_members_from_list(object_set["members"]),
                location=_location_from_data(object_set),
            )
            for set_name, object_set in module.get("object-sets", {}).items()
        ),
    )


def _module_to_dict(module: Module) -> Dict[str, Any]:
    converted: Dict[str, Any] = {
        "extensibility-implied": module.extensibility_implied,
        "imports": {
            import_.module: list(import_.symbols) for import_ in module.imports
        },
        "types": {
            assignment.name: _located(_type_to_dict(assignment.type), assignment)
            for assignment in module.types
        },
        "values": {
            assignment.name: _located(
                {
                    "type": assignment.type,
                    "value": _value_to_data(assignment.value),
                },
                assignment,
            )
            for assignment in module.values
        },
        "object-classes": _object_classes_to_dict(module.object_classes),
        "object-sets": {
            assignment.name: _located(
                {
                    "class": assignment.class_name,
                    "members": _object_set_members_to_list(assignment.members),
                },
                assignment,
            )
            for assignment in module.object_sets
        },
    }

    # Objects are found among the values.
    for object_ in module.objects:
        converted["values"][object_.name] = _located(
            {
                "type": object_.class_name,
                "value": _assigned_object_to_data(object_.object),
            },
            object_,
        )

    if module.tags is not None:
        converted["tags"] = module.tags

    return _located(converted, module)


def from_dict(specification: Dict[str, Any]) -> SyntaxTree:
    """Convert given specification dictionary, in the form the Python
    parser returned (which is gone), to a syntax tree.

    """

    return SyntaxTree(
        modules=tuple(
            _module_from_dict(specification, name, module)
            for name, module in specification.items()
        )
    )


def to_dict(tree: SyntaxTree) -> Dict[str, Any]:
    """Convert given syntax tree to a specification dictionary, in the
    form the Python parser returned (which is gone).

    """

    return {module.name: _module_to_dict(module) for module in tree.modules}


def _object_classes_from_dict(object_classes: Dict[str, Any]) -> Any:
    return tuple(
        ObjectClassAssignment(
            name=class_name,
            fields=tuple(
                ObjectClassField(
                    name=member["name"],
                    type=_type_from_dict(member, ["name"]),
                    location=_location_from_data(member),
                )
                for member in object_class["members"]
            ),
            syntax=(
                _syntax_from_list(object_class["syntax"])
                if "syntax" in object_class
                else None
            ),
            location=_location_from_data(object_class),
        )
        for class_name, object_class in object_classes.items()
    )


def _object_classes_to_dict(object_classes: Any) -> Dict[str, Any]:
    converted: Dict[str, Any] = {}

    for object_class in object_classes:
        members = []

        for field in object_class.fields:
            member = _type_to_dict(field.type)
            member["name"] = field.name
            members.append(_located(member, field))

        converted[object_class.name] = _located({"members": members}, object_class)

        if object_class.syntax is not None:
            converted[object_class.name]["syntax"] = _syntax_to_list(
                object_class.syntax
            )

    return converted


def compiled_from_dict(resolved: Dict[str, Any]) -> CompiledSpecification:
    """Convert given resolved specification dictionary, as
    :func:`compiled_to_dict()` returns, to a compiled specification.

    >>> resolved = asnkitrt.syntax.compiled_to_dict(compiled)
    >>> compiled = asnkitrt.syntax.compiled_from_dict(resolved)

    """

    token = IS_RESOLVED.set(True)

    try:
        return CompiledSpecification(
            modules=tuple(
                CompiledModule(
                    name=name,
                    types=tuple(
                        TypeAssignment(name=type_name, type=_type_from_dict(type_))
                        for type_name, type_ in module["types"].items()
                    ),
                    object_classes=_object_classes_from_dict(
                        module.get("object-classes", {})
                    ),
                )
                for name, module in resolved.items()
            )
        )
    finally:
        IS_RESOLVED.reset(token)


def compiled_to_dict(compiled: CompiledSpecification) -> Dict[str, Any]:
    """Convert given compiled specification to a resolved
    specification dictionary.

    """

    token = IS_RESOLVED.set(True)

    try:
        return {
            module.name: {
                "extensibility-implied": False,
                "imports": {},
                "types": {
                    assignment.name: _type_to_dict(assignment.type)
                    for assignment in module.types
                },
                "values": {},
                "object-classes": _object_classes_to_dict(module.object_classes),
                "object-sets": {},
            }
            for module in compiled.modules
        }
    finally:
        IS_RESOLVED.reset(token)
