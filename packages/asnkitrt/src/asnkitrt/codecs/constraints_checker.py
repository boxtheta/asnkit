# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""ASN.1 constraints checker."""

import string
from copy import copy
from typing import Any

from . import ConstraintsError, ErrorWithLocation, NamedNumbersMixin
from . import compiler
from ..errors import CompileError
from ..constants import EXTENSION_MARKER
from . import format_or
from .duration import duration_to_equivalent
from .permitted_alphabet import NUMERIC_STRING
from .permitted_alphabet import PRINTABLE_STRING
from .permitted_alphabet import IA5_STRING
from .permitted_alphabet import BMP_STRING
from .permitted_alphabet import VISIBLE_STRING

STRING_TYPES = [
    "OBJECT IDENTIFIER",
    "TeletexString",
    "VideotexString",
    "NumericString",
    "PrintableString",
    "IA5String",
    "VisibleString",
    "GeneralString",
    "UTF8String",
    "BMPString",
    "GraphicString",
    "UniversalString",
    "ObjectDescriptor",
]

TIME_TYPES = [
    "UTCTime",
    "GeneralizedTime",
    "DATE",
    "TIME-OF-DAY",
    "DATE-TIME",
]

# The members of DURATION-EQUIVALENT (X.680 38.4.4.2), which the inner
# constraints of a DURATION apply to (X.680 38.4.4.3).
DURATION_EQUIVALENT_MEMBERS = [
    {"name": name, "type": "INTEGER", "optional": True}
    for name in ["years", "months", "weeks", "days", "hours", "minutes", "seconds"]
] + [
    {
        "name": "fractional-part",
        "type": "SEQUENCE",
        "optional": True,
        "members": [
            {"name": "number-of-digits", "type": "INTEGER"},
            {"name": "fractional-value", "type": "INTEGER"},
        ],
    }
]

# The keys of the constraints of a type, of a component in WITH
# COMPONENTS and of the elements in WITH COMPONENT.
CONSTRAINT_KEYS = ["size", "restricted-to", "from", "with-components", "with-component"]


class WithComponents(object):
    """WITH COMPONENTS (X.680 51.8), as the compiler has worked it out: the
    alternatives of which a value has to satisfy any. Each is a list of
    the name of a component, its presence as ``'PRESENT'``, ``'ABSENT'``
    or ``None``, and the constraints of its value if it has any.

    """

    def __init__(self, alternatives):
        self.alternatives = alternatives

    def check(self, data):
        # The members of a SEQUENCE or a SET, or the alternative of a
        # CHOICE.
        if isinstance(data, dict):
            values = data
        elif isinstance(data, tuple):
            values = {data[0]: data[1]}
        else:
            return

        errors = []

        for alternative in self.alternatives:
            try:
                self.check_alternative(alternative, values)

                return
            except ConstraintsError as e:
                errors.append(e)

        if len(errors) == 1:
            raise errors[0]

        raise ConstraintsError(
            "Expected a value that satisfies one of the alternatives of WITH "
            "COMPONENTS."
        )

    def check_alternative(self, alternative, values):
        for name, presence, checker in alternative:
            if presence == "PRESENT" and name not in values:
                raise ConstraintsError("Expected '{}' to be present.".format(name))

            if presence == "ABSENT" and name in values:
                raise ConstraintsError("Expected '{}' to be absent.".format(name))

            if checker is not None and name in values:
                try:
                    checker.encode(values[name])
                except ErrorWithLocation as e:
                    e.add_location(checker)
                    raise e


class WithComponent(object):
    """WITH COMPONENT (X.680 51.8), as the compiler has worked it out: the
    alternatives, of which all the elements of a value have to satisfy
    the same one, each the element type with its constraints.

    """

    def __init__(self, alternatives):
        self.alternatives = alternatives

    def check(self, data):
        errors = []

        for element in self.alternatives:
            try:
                for entry in data:
                    element.encode(entry)

                return
            except ConstraintsError as e:
                errors.append(e)

        if len(errors) == 1:
            raise errors[0]

        raise ConstraintsError(
            "Expected elements that all satisfy one of the alternatives of WITH "
            "COMPONENT."
        )


class Type(object):
    def __init__(self, name):
        self.name = name
        self.minimum = "MIN"
        self.maximum = "MAX"
        # The values and ranges of the root of the constraint, of which
        # minimum and maximum are the least and the greatest.
        self.parts = None
        self.extensible = False
        # WITH COMPONENTS or WITH COMPONENT, if the type has one that is
        # checked.
        self.inner_constraint: Any = None

    def check_inner_constraint(self, data):
        if self.inner_constraint is not None:
            self.inner_constraint.check(data)

    def set_range(self, minimum, maximum, has_extension_marker):
        self.extensible = bool(has_extension_marker)

        if has_extension_marker:
            # An extensible constraint is not checked.
            self.minimum = "MIN"
            self.maximum = "MAX"

            return

        if minimum is None:
            minimum = "MIN"

        if maximum is None:
            maximum = "MAX"

        self.minimum = minimum
        self.maximum = maximum

    def set_size_range(self, minimum, maximum, has_extension_marker):
        self.set_range(minimum, maximum, has_extension_marker)

    def set_restricted_to_range(self, minimum, maximum, has_extension_marker):
        self.set_range(minimum, maximum, has_extension_marker)

    def is_in_range(self, value):
        if self.parts is not None and not self.extensible:
            return compiler.is_in_parts(value, self.parts)

        minimum_ok = (not self.has_lower_bound()) or (value >= self.minimum)
        maximum_ok = (not self.has_upper_bound()) or (value <= self.maximum)

        return minimum_ok and maximum_ok

    def range_text(self, preposition=""):
        """The range a value is to be in, as a message tells it: between its
        least and greatest values, or each of its parts if it has several,
        after given preposition.

        """

        if self.parts is not None and len(self.parts) > 1:
            parts = [
                str(minimum)
                if minimum == maximum
                else "{}..{}".format(minimum, maximum)
                for minimum, maximum in self.parts
            ]

            return " ".join(
                ([preposition] if preposition else []) + [" | ".join(parts)]
            )

        return "between {} and {}".format(self.minimum, self.maximum)

    def set_default(self, value):
        pass

    def has_lower_bound(self):
        return self.minimum != "MIN"

    def has_upper_bound(self):
        return self.maximum != "MAX"

    def is_bound(self):
        return self.has_lower_bound() and self.has_upper_bound()

    def encode(self, data):
        raise NotImplementedError("To be implemented by subclasses.")


class String(Type):
    PERMITTED_ALPHABET: str | None = None

    def __init__(
        self, name, permitted_alphabet, minimum, maximum, has_extension_marker
    ):
        super(String, self).__init__(name)
        self.set_size_range(minimum, maximum, has_extension_marker)

        self.set_permitted_alphabet(permitted_alphabet)

    def set_permitted_alphabet(self, permitted_alphabet):
        if permitted_alphabet is None:
            permitted_alphabet = self.PERMITTED_ALPHABET

        self.permitted_alphabet = permitted_alphabet

    def encode(self, data):
        length = len(data)

        if not self.is_in_range(length):
            raise ConstraintsError(
                "Expected {} characters, but got {}.".format(self.range_text(), length)
            )

        if self.permitted_alphabet is None:
            return

        for character in data:
            if character not in self.permitted_alphabet:
                raise ConstraintsError(
                    "Expected a character in '{}', but got '{}' (0x{:02x}).".format(
                        "".join(
                            [
                                c if c in string.printable[:-5] else "."
                                for c in self.permitted_alphabet
                            ]
                        ),
                        character if character in string.printable else ".",
                        ord(character),
                    )
                )


class Boolean(Type):
    def encode(self, data):
        pass


class Integer(NamedNumbersMixin, Type):
    def __init__(self, name):
        super(Integer, self).__init__(name)

    def encode(self, data):
        data = self.to_number(data)

        if not self.is_in_range(data):
            raise ConstraintsError(
                "Expected an integer {}, but got {}.".format(
                    self.range_text("in"), data
                )
            )


class Float(Type):
    def encode(self, data):
        pass


class Null(Type):
    def encode(self, data):
        pass


class BitString(Type):
    def __init__(self, name, minimum, maximum, has_extension_marker):
        super(BitString, self).__init__(name)
        self.set_size_range(minimum, maximum, has_extension_marker)

    def encode(self, data):
        number_of_bits = data[1]

        if not self.is_in_range(number_of_bits):
            raise ConstraintsError(
                "Expected {} bits, but got {}.".format(
                    self.range_text(), number_of_bits
                )
            )


class Enumerated(Type):
    def encode(self, data):
        pass


class Bytes(Type):
    def __init__(self, name, minimum, maximum, has_extension_marker):
        super(Bytes, self).__init__(name)
        self.set_size_range(minimum, maximum, has_extension_marker)

    def encode(self, data):
        length = len(data)

        if not self.is_in_range(length):
            raise ConstraintsError(
                "Expected {} bytes, but got {}.".format(self.range_text(), length)
            )


class Dict(Type):
    def __init__(self, name, members):
        super(Dict, self).__init__(name)
        self.members = members

    def encode(self, data):
        for member in self.members:
            name = member.name

            if name in data:
                try:
                    member.encode(data[name])
                except ErrorWithLocation as e:
                    # Add member location
                    e.add_location(member)
                    raise e

        self.check_inner_constraint(data)


class List(Type):
    def __init__(self, name, element_type, minimum, maximum, has_extension_marker):
        super(List, self).__init__(name)
        self.element_type = element_type
        self.set_size_range(minimum, maximum, has_extension_marker)

    def encode(self, data):
        length = len(data)

        if not self.is_in_range(length):
            raise ConstraintsError(
                "Expected a list of {} elements, but got {}.".format(
                    self.range_text(), length
                )
            )

        for entry in data:
            self.element_type.encode(entry)

        self.check_inner_constraint(data)


class Choice(Type):
    def __init__(self, name, members, has_extension_marker):
        super(Choice, self).__init__(name)
        self.members = members
        self.name_to_member = {member.name: member for member in self.members}
        self.has_extension_marker = has_extension_marker

    def format_names(self):
        return format_or(sorted(self.name_to_member))

    def encode(self, data):
        value = data[0]

        if value in self.name_to_member:
            member = self.name_to_member[value]
        elif self.has_extension_marker:
            return
        else:
            raise ConstraintsError(
                "Expected choice {}, but got '{}'.".format(self.format_names(), value)
            )
        try:
            member.encode(data[1])
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(member)
            raise e

        self.check_inner_constraint(data)


class NumericString(String):
    PERMITTED_ALPHABET = NUMERIC_STRING


class PrintableString(String):
    PERMITTED_ALPHABET = PRINTABLE_STRING


class IA5String(String):
    PERMITTED_ALPHABET = IA5_STRING


class BMPString(String):
    PERMITTED_ALPHABET = BMP_STRING


class VisibleString(String):
    PERMITTED_ALPHABET = VISIBLE_STRING


class Time(Type):
    def encode(self, data):
        pass


class Duration(Type):
    """A DURATION, of which an inner constraint is checked on its
    DURATION-EQUIVALENT value (X.680 38.4.4).

    """

    def encode(self, data):
        if self.inner_constraint is None:
            return

        try:
            equivalent = duration_to_equivalent(data)
        except (ValueError, TypeError):
            # Not a duration, which the codec says.
            return

        self.check_inner_constraint(equivalent)


class Skip(Type):
    def encode(self, data):
        pass


class Recursive(compiler.Recursive, Type):
    def __init__(self, name, type_name, module_name):
        super(Recursive, self).__init__(name)
        self.type_name = type_name
        self.module_name = module_name
        self.inner: Any = None

    def set_inner_type(self, inner):
        self.inner = copy(inner)

    def encode(self, data):
        self.inner.encode(data)
        self.check_inner_constraint(data)


class CompiledType(compiler.CompiledType):
    def encode(self, data):
        try:
            self._type.encode(data)
        except ErrorWithLocation as e:
            # Add member location
            e.add_location(self._type)
            raise e


class Compiler(compiler.Compiler):
    def process_type(self, type_name, type_descriptor, module_name):
        compiled_type = self.compile_type(type_name, type_descriptor, module_name)

        return CompiledType(compiled_type)

    def compile_type(self, name, type_descriptor, module_name):
        module_name = type_descriptor.get("module-name", module_name)
        type_name = type_descriptor["type"]

        compiled: Type
        if type_name in ["SEQUENCE", "SET"]:
            members, _ = self.compile_members(type_descriptor["members"], module_name)
            compiled = Dict(name, members)
        elif type_name in ["SEQUENCE OF", "SET OF"]:
            element_type = self.compile_type(
                "", type_descriptor["element"], module_name
            )
            compiled = List(
                name, element_type, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "CHOICE":
            compiled = Choice(
                name, *self.compile_members(type_descriptor["members"], module_name)
            )
        elif type_name == "INTEGER":
            compiled = Integer(name)
            compiled.named_numbers = self.get_named_numbers(
                type_descriptor, module_name
            )
        elif type_name == "REAL":
            compiled = Float(name)
        elif type_name == "BOOLEAN":
            compiled = Boolean(name)
        elif type_name == "OCTET STRING":
            compiled = Bytes(name, *self.get_size_range(type_descriptor, module_name))
        elif type_name == "ENUMERATED":
            compiled = Enumerated(name)
        elif type_name == "DURATION":
            compiled = Duration(name)
        elif type_name in TIME_TYPES:
            compiled = Time(name)
        elif type_name == "BIT STRING":
            compiled = BitString(
                name, *self.get_size_range(type_descriptor, module_name)
            )
        elif type_name == "NumericString":
            compiled = NumericString(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "PrintableString":
            compiled = PrintableString(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "IA5String":
            compiled = IA5String(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "BMPString":
            compiled = BMPString(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name == "VisibleString":
            compiled = VisibleString(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name in STRING_TYPES:
            compiled = String(
                name,
                self.get_permitted_alphabet(type_descriptor),
                *self.get_size_range(type_descriptor, module_name),
            )
        elif type_name in ["ANY", "ANY DEFINED BY", "OpenType"]:
            compiled = Skip(name)
        elif type_name == "NULL":
            compiled = Null(name)
        elif type_name == "EXTERNAL":
            members, _ = self.compile_members(
                self.external_type_descriptor()["members"], module_name
            )
            compiled = Dict(name, members)
        else:
            if type_name in self.types_backtrace:
                compiled = Recursive(name, type_name, module_name)
                self.recursive_types.append(compiled)
            else:
                compiled = self.compile_user_type(name, type_name, module_name)
                compiled = self.with_own_constraints(
                    compiled, type_descriptor, module_name
                )

        if "restricted-to" in type_descriptor:
            compiled = self.set_compiled_restricted_to(
                compiled, type_descriptor, module_name
            )
            compiled.parts = self.get_restricted_to_parts(type_descriptor, module_name)
        elif "size" in type_descriptor:
            compiled = self.copy(compiled)
            compiled.parts = self.get_size_parts(type_descriptor, module_name)

        # Not of a type that refers to itself, which is not copied, as it
        # is only made what it refers to when that is compiled.
        if (
            "with-components" in type_descriptor or "with-component" in type_descriptor
        ) and not isinstance(compiled, Recursive):
            compiled = self.copy(compiled)
            compiled.inner_constraint = self.compile_inner_constraint(
                type_descriptor, module_name
            )

        return compiled

    def resolve_with_module(self, type_descriptor, module_name):
        """Returns the descriptor given type ends in, following references,
        and the module it is found in.

        """

        module_name = type_descriptor.get("module-name", module_name)

        try:
            while True:
                type_name = type_descriptor["type"]

                if compiler.is_object_class_type_name(type_name):
                    type_descriptor, module_name = self.lookup_object_class_type(
                        type_name, module_name
                    )
                else:
                    type_descriptor, module_name = self.lookup_type_descriptor(
                        type_name, module_name
                    )

                module_name = type_descriptor.get("module-name", module_name)
        except (CompileError, TypeError):
            pass

        return type_descriptor, module_name

    def compile_inner_constraint(self, type_descriptor, module_name):
        """Returns the WITH COMPONENTS or the WITH COMPONENT of given type, or
        ``None`` if it is extensible, as it is then not checked.

        """

        resolved, resolved_module_name = self.resolve_with_module(
            type_descriptor, module_name
        )

        if "with-component" in type_descriptor:
            alternatives = type_descriptor["with-component"]

            if "element" not in resolved or EXTENSION_MARKER in alternatives:
                return None

            return WithComponent(
                [
                    self.compile_constrained(
                        resolved["element"], alternative, resolved_module_name
                    )
                    for alternative in alternatives
                ]
            )

        alternatives = type_descriptor["with-components"]

        if EXTENSION_MARKER in alternatives:
            return None

        members = {}

        def add(items):
            for item in items:
                if isinstance(item, list):
                    add(item)
                elif item is not EXTENSION_MARKER:
                    members[item["name"]] = item

        if resolved["type"] == "DURATION":
            add(DURATION_EQUIVALENT_MEMBERS)
        else:
            add(resolved.get("members", []))
        compiled_alternatives = []

        for alternative in alternatives:
            compiled_alternative = []

            for component in alternative:
                name = component["name"]
                checker = None

                if name in members and any(key in component for key in CONSTRAINT_KEYS):
                    checker = self.compile_constrained(
                        members[name], component, resolved_module_name
                    )

                compiled_alternative.append((name, component.get("presence"), checker))

            compiled_alternatives.append(compiled_alternative)

        return WithComponents(compiled_alternatives)

    def compile_constrained(self, type_descriptor, constraint, module_name):
        """Returns given type with the constraints of a component or of the
        elements in place of its own, which are checked with the type.

        """

        constrained = {
            key: value
            for key, value in type_descriptor.items()
            if key not in CONSTRAINT_KEYS + ["optional", "default"]
        }

        for key in CONSTRAINT_KEYS:
            if key in constraint:
                constrained[key] = constraint[key]

        return self.compile_type(
            type_descriptor.get("name", ""), constrained, module_name
        )

    def get_permitted_alphabet(self, type_descriptor):
        def char_range(begin, end):
            return "".join([chr(char) for char in range(ord(begin), ord(end) + 1)])

        if "from" not in type_descriptor:
            return

        permitted_alphabet = type_descriptor["from"]

        # An extensible permitted alphabet is not PER-visible (X.691
        # 10.3.11), and an extensible constraint is not checked.
        if EXTENSION_MARKER in permitted_alphabet:
            return

        value = ""

        for item in permitted_alphabet:
            if isinstance(item, tuple):
                value += char_range(item[0], item[1])
            else:
                value += item

        return "".join(sorted(value))


def compile_dict(specification, numeric_enums=False):
    return Compiler(specification, numeric_enums).process()
