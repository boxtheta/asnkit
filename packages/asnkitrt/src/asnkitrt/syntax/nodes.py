# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Nodes of the syntax tree of a parsed ASN.1 specification.

Every node is a frozen dataclass whose fields only hold other nodes,
tuples, strings, integers, floats, booleans and ``None``, so that a
tree can be serialized without knowing anything about Python.

"""

from dataclasses import dataclass, field
from typing import Any, Tuple, Union

# Version of the layout of the nodes, as (major, minor). It is found
# in the header of a file holding them.
#
# Increment minor when adding a field to a node, which goes last in
# its fields in the compact form, as that is all a reader of an
# earlier minor can ignore. Increment major on any other change, such
# as a new kind of node.
SCHEMA_VERSION = (1, 0)

# Where something is found in the ASN.1 files a syntax tree was made
# from: the index of the file among the files parsed, and the line
# and the column it starts at in that file, both counted from 1. A
# column is a number of characters, not bytes.
#
# The sources in the metadata of the file a tree is found in are the
# files parsed, in the same order, unless other metadata was given
# when making it.
#
# It is None when not known, as in a compiled specification, and is
# not part of what makes two nodes equal.
Location = Tuple[int, int, int]


@dataclass(frozen=True, kw_only=True)
class Node:
    """Base class of all syntax tree nodes."""


@dataclass(frozen=True, kw_only=True)
class ExtensionMarker(Node):
    """The ``...`` in a member list, an enumeration or a constraint."""


@dataclass(frozen=True, kw_only=True)
class Identifier(Node):
    """A value given by name: a reference to a value, or an identifier
    of an enumeration, a named number or a named bit. Which one
    depends on the type it is a value of.

    """

    name: str


@dataclass(frozen=True, kw_only=True)
class Min(Node):
    """``MIN`` as the lower bound of a range."""


@dataclass(frozen=True, kw_only=True)
class Max(Node):
    """``MAX`` as the upper bound of a range."""


@dataclass(frozen=True, kw_only=True)
class BinaryString(Node):
    """``'0110'B``, with `bits` as ``'0110'``."""

    bits: str


@dataclass(frozen=True, kw_only=True)
class HexString(Node):
    """``'1f'H``, with `digits` as ``'1f'``."""

    digits: str


@dataclass(frozen=True, kw_only=True)
class UnsupportedValue(Node):
    """A value the parser does not support, such as a ``SEQUENCE``
    value.

    """


@dataclass(frozen=True, kw_only=True)
class Null(Node):
    """``NULL``."""


@dataclass(frozen=True, kw_only=True)
class PlusInfinity(Node):
    """``PLUS-INFINITY``."""


@dataclass(frozen=True, kw_only=True)
class MinusInfinity(Node):
    """``MINUS-INFINITY``."""


@dataclass(frozen=True, kw_only=True)
class NotANumber(Node):
    """``NOT-A-NUMBER``."""


@dataclass(frozen=True, kw_only=True)
class NamedValue(Node):
    """A value with a name. What the name is depends on the type it
    is a value of: the alternative of a choice in ``name : value``, a
    member of a sequence or a set in ``{name value, ...}``, or a
    component of an object identifier in ``{name(value) ...}``.

    """

    name: str
    value: Any


@dataclass(frozen=True, kw_only=True)
class SequenceValue(Node):
    """A value in braces with every item named, as of a sequence or a
    set: ``{name value, ...}``.

    """

    members: Tuple[NamedValue, ...]


@dataclass(frozen=True, kw_only=True)
class NamedNumber(Node):
    """A named number of an ``INTEGER``, a named bit of a ``BIT
    STRING`` or an item of an enumeration.

    """

    name: str
    number: Union[int, Identifier]


# A value:
#
# - A number, a boolean or a character string as an integer, a float,
#   a boolean or a string. A string is never a reference.
# - Identifier, BinaryString, HexString, Null, PlusInfinity,
#   MinusInfinity or NotANumber.
# - NamedValue as the value of a choice.
# - SequenceValue for a value in braces with every item named.
# - A tuple of values for any other value in braces, such as a list
#   of values, a list of named bits or an object identifier, of which
#   a component with both a name and a number is a NamedValue.
# - UnsupportedValue.
Value = Any


@dataclass(frozen=True, kw_only=True)
class Excluded(Node):
    """A bound that is not part of its range, as the lower one in
    ``1<..5``.

    """

    value: Any


@dataclass(frozen=True, kw_only=True)
class Range(Node):
    """``lower..upper``, with values, Min, Max or any of them as
    Excluded as bounds.

    """

    lower: Any
    upper: Any


@dataclass(frozen=True, kw_only=True)
class Tag(Node):
    """``[tag_class number] mode``, for example ``[APPLICATION 1]
    IMPLICIT``.

    """

    number: Union[int, Identifier]
    tag_class: str | None = None
    mode: str | None = None


@dataclass(frozen=True, kw_only=True)
class ObjectField(Node):
    """A field of an object and what it is set to: a type for a type
    field such as ``&Type``, and a value for a value field such as
    ``&id``.

    """

    name: str
    setting: Union["Type", Value]


@dataclass(frozen=True, kw_only=True)
class Object(Node):
    """An object defined in place, as in ``{&id 1, &Type INTEGER}``, or
    the same given with the syntax defined by its class.

    """

    fields: Tuple[ObjectField, ...]


@dataclass(frozen=True, kw_only=True)
class UnresolvedObject(Node):
    """An object given with the syntax defined by its class, which
    could not be found or which syntax it does not match. `source` is
    its text without the braces.

    """

    source: str


# A member of an object set: the name of an object or object set, an
# extension marker, or an object defined in place.
ObjectSetMember = Union[str, ExtensionMarker, Object, UnresolvedObject]


@dataclass(frozen=True, kw_only=True)
class ObjectSet(Node):
    """An object set given in place, as in the actual parameter
    ``{Objects | object, ...}``.

    """

    members: Tuple[ObjectSetMember, ...]


@dataclass(frozen=True, kw_only=True)
class ElementConstraint(Node):
    """The constraint of the elements of a ``SEQUENCE OF`` or a ``SET
    OF`` in ``WITH COMPONENT``: any of the constraints of a type.

    """

    size: Tuple["ConstraintElement", ...] | None = None
    restricted_to: Tuple["ConstraintElement", ...] | None = None
    permitted_alphabet: Tuple["ConstraintElement", ...] | None = None
    with_components: Tuple["WithComponentsItem", ...] | None = None
    with_component: Tuple["WithComponentItem", ...] | None = None
    containing: Union["Type", None] = None


@dataclass(frozen=True, kw_only=True)
class ComponentConstraint(ElementConstraint):
    """The constraint of a component in ``WITH COMPONENTS``: any of the
    constraints of a type, and `presence` as ``'PRESENT'`` or
    ``'ABSENT'``.

    """

    name: str
    presence: str | None = None


# An item of a ``WITH COMPONENTS`` as the compiler has worked it out: an
# alternative, with the constraints of components of different names, or
# an extension marker if it is extensible. A value satisfies it if it
# satisfies all the constraints of any of the alternatives.
WithComponentsItem = Union[Tuple[ComponentConstraint, ...], ExtensionMarker]

# An item of a ``WITH COMPONENT`` as the compiler has worked it out: an
# alternative, with the constraints of the elements, or an extension
# marker if it is extensible. A value satisfies it if all its elements
# satisfy the constraints of the same alternative.
WithComponentItem = Union[ElementConstraint, ExtensionMarker]


# A value, size or permitted alphabet constraint element: a single
# value, a range, a type for a contained subtype, or an extension
# marker.
ConstraintElement = Union[Range, ExtensionMarker, "Type", Value]


@dataclass(frozen=True, kw_only=True)
class TableConstraint(Node):
    """``({ObjectSet})`` or ``({ObjectSet}{@component})`` on a field of
    an object class: the objects it is restricted to, and the
    components selecting one of them.

    """

    object_set: str
    component_ids: Tuple[str, ...] | None = None


@dataclass(frozen=True, kw_only=True)
class ElementSetSpecs(Node):
    """A constraint of sets of values, as written in parentheses: the root,
    and if it is extensible, the additions.

    """

    root: "ElementSet"
    extensible: bool = False
    additions: Union["ElementSet", None] = None


@dataclass(frozen=True, kw_only=True)
class SetUnion(Node):
    """The values of any of the sets: ``|`` or ``UNION``."""

    elements: Tuple["ElementSet", ...]


# The kind is ``Union``, which is also the name of ``typing.Union``.
SetUnion.__name__ = SetUnion.__qualname__ = "Union"


@dataclass(frozen=True, kw_only=True)
class Intersection(Node):
    """The values of all of the sets: ``^`` or ``INTERSECTION``."""

    elements: Tuple["ElementSet", ...]


@dataclass(frozen=True, kw_only=True)
class Except(Node):
    """The values of `elements` but those of `exclusions`."""

    elements: "ElementSet"
    exclusions: "ElementSet"


@dataclass(frozen=True, kw_only=True)
class All(Node):
    """``ALL EXCEPT`` the values of `exclusions`."""

    exclusions: "ElementSet"


@dataclass(frozen=True, kw_only=True)
class SizeConstraint(Node):
    """``SIZE`` and the constraint of the sizes."""

    constraint: "Constraint"


@dataclass(frozen=True, kw_only=True)
class PermittedAlphabet(Node):
    """``FROM`` and the constraint of the characters."""

    constraint: "Constraint"


@dataclass(frozen=True, kw_only=True)
class ContainedSubtype(Node):
    """The values of a type, with ``INCLUDES`` or not."""

    type: "Type"
    includes: bool = False


@dataclass(frozen=True, kw_only=True)
class PatternConstraint(Node):
    """``PATTERN`` and its value."""

    value: Value


@dataclass(frozen=True, kw_only=True)
class InnerTypeConstraint(Node):
    """``WITH COMPONENT`` and the constraint of the elements."""

    constraint: "Constraint"


@dataclass(frozen=True, kw_only=True)
class NamedConstraint(Node):
    """A component in ``WITH COMPONENTS``: its constraints, each applied with
    those before it, and its presence as ``'PRESENT'``, ``'ABSENT'`` or
    ``'OPTIONAL'``.

    """

    name: str
    constraints: Tuple["Constraint", ...] | None = None
    presence: str | None = None


@dataclass(frozen=True, kw_only=True)
class InnerTypeConstraints(Node):
    """``WITH COMPONENTS``, of which a partial one starts with ``...``."""

    partial: bool = False
    components: Tuple[NamedConstraint, ...]


@dataclass(frozen=True, kw_only=True)
class ContentsConstraint(Node):
    """``CONTAINING`` a type, and ``ENCODED BY`` a value."""

    type: Union["Type", None] = None
    encoded_by: Value = None


@dataclass(frozen=True, kw_only=True)
class UserDefinedConstraint(Node):
    """``CONSTRAINED BY { ... }``."""


@dataclass(frozen=True, kw_only=True)
class UnsupportedConstraint(Node):
    """A table constraint of a set of objects that is not given by name."""


# An element of a set of values: a single value, a range, or one of the
# constraints above.
ElementSet = Any

# A constraint as written in parentheses after a type.
Constraint = Union[
    ElementSetSpecs,
    TableConstraint,
    ContentsConstraint,
    UserDefinedConstraint,
    UnsupportedConstraint,
]


@dataclass(frozen=True, kw_only=True)
class Type(Node):
    """Base class of all types, holding what applies to any of them.

    The constraints are ``None`` when not present.

    """

    tag: Tag | None = None
    size: Tuple[ConstraintElement, ...] | None = None
    restricted_to: Tuple[ConstraintElement, ...] | None = None
    permitted_alphabet: Tuple[ConstraintElement, ...] | None = None
    with_components: Tuple[WithComponentsItem, ...] | None = None
    with_component: Tuple[WithComponentItem, ...] | None = None
    table: TableConstraint | None = None
    parameters: Tuple["Parameter", ...] | None = None
    actual_parameters: Tuple["ActualParameter", ...] | None = None
    # The module its references to types and object classes are found
    # in, if not the one it is found in itself. Only in a compiled
    # specification.
    module_name: str | None = None
    # The constraints as they are written, each applied with those before
    # it. Only in a syntax tree: a compiled specification has what the
    # compiler made of them in the fields above.
    constraints: Tuple[Constraint, ...] | None = None


@dataclass(frozen=True, kw_only=True)
class Parameter(Node):
    """A parameter of a parameterized type. Its governor is the type
    its value is of, or a reference to the class its object or object
    set is of, and ``None`` for a type.

    """

    name: str
    governor: Type | None = None


# A type, an object set, an object or a value.
ActualParameter = Union[Type, ObjectSet, Object, UnresolvedObject, Value]


@dataclass(frozen=True, kw_only=True)
class BuiltinType(Type):
    """A built-in type without any syntax of its own, for example
    ``BOOLEAN``, ``OCTET STRING`` and ``UTF8String``.

    """

    name: str


@dataclass(frozen=True, kw_only=True)
class ReferencedType(Type):
    """A reference to a type defined elsewhere, or to an object class
    field such as ``ALGORITHM.&id``.

    """

    name: str


@dataclass(frozen=True, kw_only=True)
class IntegerType(Type):
    named_numbers: Tuple[NamedNumber, ...] | None = None


@dataclass(frozen=True, kw_only=True)
class BitStringType(Type):
    named_bits: Tuple[NamedNumber, ...] | None = None


@dataclass(frozen=True, kw_only=True)
class EnumeratedType(Type):
    values: Tuple[Union[NamedNumber, ExtensionMarker], ...]


@dataclass(frozen=True, kw_only=True)
class Member(Node):
    """A named component of a ``SEQUENCE``, ``SET`` or ``CHOICE``."""

    name: str
    type: Type
    optional: bool = False
    # None if it has no default.
    default: Value = None
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class ComponentsOf(Node):
    """``COMPONENTS OF type_name``."""

    type_name: str


@dataclass(frozen=True, kw_only=True)
class ExtensionAdditionGroup(Node):
    """``[[ ... ]]``."""

    members: Tuple["MemberListItem", ...]


MemberListItem = Union[Member, ComponentsOf, ExtensionMarker, ExtensionAdditionGroup]


@dataclass(frozen=True, kw_only=True)
class SequenceType(Type):
    members: Tuple[MemberListItem, ...]


@dataclass(frozen=True, kw_only=True)
class SetType(Type):
    members: Tuple[MemberListItem, ...]


@dataclass(frozen=True, kw_only=True)
class ChoiceType(Type):
    members: Tuple[MemberListItem, ...]


@dataclass(frozen=True, kw_only=True)
class SequenceOfType(Type):
    element: Type
    element_name: str | None = None


@dataclass(frozen=True, kw_only=True)
class SetOfType(Type):
    element: Type
    element_name: str | None = None


@dataclass(frozen=True, kw_only=True)
class AnyDefinedByChoice(Node):
    """The type to use when the member defining it has given value."""

    value: Any
    type: Type


@dataclass(frozen=True, kw_only=True)
class AnyDefinedByType(Type):
    """``ANY DEFINED BY value``, with `value` as the name of a member.
    The choices are not part of a specification; they are added when
    compiling.

    """

    value: str
    choices: Tuple[AnyDefinedByChoice, ...] = ()


@dataclass(frozen=True, kw_only=True)
class TypeAssignment(Node):
    name: str
    type: Type
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class ValueAssignment(Node):
    """A value assignment. `type` is the name of its type."""

    name: str
    type: str
    value: Value
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class ObjectAssignment(Node):
    """`object` is an object defined in place, an Identifier
    referencing another object, or an UnsupportedValue.

    """

    name: str
    class_name: str
    object: Union[Object, UnresolvedObject, Identifier, UnsupportedValue]
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class ObjectClassField(Node):
    name: str
    type: Type
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class OptionalGroup(Node):
    """``[ ... ]`` in the syntax of an object class."""

    tokens: Tuple[Union[str, "OptionalGroup"], ...]


@dataclass(frozen=True, kw_only=True)
class ObjectClassAssignment(Node):
    """`syntax` is what ``WITH SYNTAX`` gives: words, ``','`` and
    field names such as ``'&id'``, and optional groups of those. It
    is ``None`` when the class has no syntax of its own.

    """

    name: str
    fields: Tuple[ObjectClassField, ...]
    syntax: Tuple[Union[str, OptionalGroup], ...] | None = None
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class ObjectSetAssignment(Node):
    name: str
    class_name: str
    members: Tuple[ObjectSetMember, ...]
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class Import(Node):
    """``symbols FROM module``."""

    module: str
    symbols: Tuple[str, ...]


@dataclass(frozen=True, kw_only=True)
class Module(Node):
    name: str
    tags: str | None = None
    extensibility_implied: bool = False
    imports: Tuple[Import, ...] = ()
    types: Tuple[TypeAssignment, ...] = ()
    values: Tuple[ValueAssignment, ...] = ()
    objects: Tuple[ObjectAssignment, ...] = ()
    object_classes: Tuple[ObjectClassAssignment, ...] = ()
    object_sets: Tuple[ObjectSetAssignment, ...] = ()
    location: Location | None = field(default=None, compare=False)


@dataclass(frozen=True, kw_only=True)
class SyntaxTree(Node):
    """Root of the tree: all modules of a specification."""

    modules: Tuple[Module, ...]


@dataclass(frozen=True, kw_only=True)
class CompiledModule(Node):
    name: str
    types: Tuple[TypeAssignment, ...] = ()
    object_classes: Tuple[ObjectClassAssignment, ...] = ()


@dataclass(frozen=True, kw_only=True)
class CompiledSpecification(Node):
    """A specification resolved to what encoding and decoding its
    types needs, which is what an ``.asnc`` file holds.

    Unlike in a syntax tree:

    - Every type is what it is with ``COMPONENTS OF``,
      ``EXTENSIBILITY IMPLIED``, the tagging of its module and any
      actual parameters applied. No type has parameters.

    - Sizes, ranges, tags, named numbers, named bits and enumeration
      numbers are numbers, never an :class:`Identifier`.

    - A reference to a type or object class of another module tells
      which in `module_name`.

    - There are only types and object classes.

    """

    modules: Tuple[CompiledModule, ...]
