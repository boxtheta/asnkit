# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Base Compiler class used by all codecs."""

import binascii
import sys
from copy import copy, deepcopy
from operator import attrgetter
from typing import Any


from ..errors import CompileError
from ..constants import EXTENSION_MARKER, CString, ValueRange


def flatten(dlist):
    flist = []

    for item in dlist:
        if isinstance(item, list):
            flist.extend(item)
        else:
            flist.append(item)

    return flist


def is_object_class_type_name(type_name):
    return "&" in type_name


def is_open_type(type_name):
    index = type_name.find("&")

    if index == -1:
        return False

    return type_name[index + 1].isupper()


CLASS_ORDER = {"UNIVERSAL": 0, "APPLICATION": 1, "CONTEXT_SPECIFIC": 2, "PRIVATE": 3}

# The universal tag of each type that has one.
UNIVERSAL_TAGS = {
    "BOOLEAN": 1,
    "INTEGER": 2,
    "BIT STRING": 3,
    "OCTET STRING": 4,
    "NULL": 5,
    "OBJECT IDENTIFIER": 6,
    "ObjectDescriptor": 7,
    "EXTERNAL": 8,
    "REAL": 9,
    "ENUMERATED": 10,
    "UTF8String": 12,
    "SEQUENCE": 16,
    "SEQUENCE OF": 16,
    "SET": 17,
    "SET OF": 17,
    "NumericString": 18,
    "PrintableString": 19,
    "TeletexString": 20,
    "T61String": 20,
    "IA5String": 22,
    "ISO646String": 22,
    "UTCTime": 23,
    "GeneralizedTime": 24,
    "GraphicString": 25,
    "VisibleString": 26,
    "GeneralString": 27,
    "UniversalString": 28,
    "BMPString": 30,
    "DATE": 31,
    "TIME-OF-DAY": 32,
    "DATE-TIME": 33,
    "DURATION": 34,
}

# Types with no tag of their own.
UNTAGGED_TYPES = ["CHOICE", "ANY", "ANY DEFINED BY", "OpenType"]


def is_type_name(type_name):
    """Does not handle keywords."""

    return type_name[0].isupper()


def root_members(members):
    """The members of the root, which are those before the first extension
    marker and after the second.

    """

    root = []
    in_extension = False

    for member in members:
        if member == EXTENSION_MARKER:
            in_extension = not in_extension
        elif not in_extension and not isinstance(member, list):
            root.append(member)

    return root


def addition_members(members):
    """The extension additions, with the members of groups among them."""

    additions = []
    in_extension = False

    for member in members:
        if member == EXTENSION_MARKER:
            in_extension = not in_extension
        elif in_extension:
            additions.extend(member if isinstance(member, list) else [member])

    return additions


def constraint_parts(items, convert_value):
    """The values and the ranges of the root of a constraint, which are its
    elements before an extension marker, as tuples of their bounds. A bound
    given by name is converted by given function, and an excluded bound of
    an integer is the next one in.

    """

    parts = []

    for item in items:
        if item == EXTENSION_MARKER:
            break

        if isinstance(item, tuple):
            minimum, maximum = item
        elif isinstance(item, dict):
            # A contained subtype, which the compiler has worked out.
            minimum, maximum = "MIN", "MAX"
        else:
            minimum, maximum = item, item

        if isinstance(minimum, str):
            minimum = convert_value(minimum)

        if isinstance(maximum, str):
            maximum = convert_value(maximum)

        if isinstance(item, ValueRange):
            if item.is_minimum_excluded and isinstance(minimum, int):
                minimum += 1

            if item.is_maximum_excluded and isinstance(maximum, int):
                maximum -= 1

        parts.append((minimum, maximum))

    return parts


def hull(parts):
    """The least and the greatest bounds of given parts of a constraint, with
    ``MIN`` and ``MAX`` for no bound.

    """

    lowers = [minimum for minimum, _ in parts]
    uppers = [maximum for _, maximum in parts]

    if not lowers or any(not isinstance(bound, (int, float)) for bound in lowers):
        minimum = "MIN"
    else:
        minimum = min(lowers)

    if not uppers or any(not isinstance(bound, (int, float)) for bound in uppers):
        maximum = "MAX"
    else:
        maximum = max(uppers)

    return minimum, maximum


def is_in_parts(value, parts):
    """If given value is in one of given parts of a constraint."""

    for minimum, maximum in parts:
        lower_ok = not isinstance(minimum, (int, float)) or value >= minimum
        upper_ok = not isinstance(maximum, (int, float)) or value <= maximum

        if lower_ok and upper_ok:
            return True

    return False


def lowest_set_bit(value):
    offset = (value & -value).bit_length() - 1

    if offset < 0:
        offset = 0

    return offset


def rstrip_bit_string_zeros(data):
    data = data.rstrip(b"\x00")

    if len(data) == 0:
        number_of_bits = 0
    else:
        number_of_bits = 8 * len(data) - lowest_set_bit(data[-1])

    return (data, number_of_bits)


def clean_bit_string_value(value, has_named_bits):
    data = bytearray(value[0])
    number_of_bits = value[1]
    number_of_bytes, number_of_rest_bits = divmod(number_of_bits, 8)

    if number_of_rest_bits == 0:
        data = data[:number_of_bytes]
    else:
        data = data[: number_of_bytes + 1]
        data[number_of_bytes] &= (0xFF >> number_of_rest_bits) ^ 0xFF

    if has_named_bits:
        return rstrip_bit_string_zeros(data)
    else:
        return (data, number_of_bits)


class CompiledType(object):
    def __init__(self, type_):
        self.constraints_checker: Any = None
        self.type_checker: Any = None
        self._type = type_

    @property
    def type(self):
        return self._type

    @property
    def name(self):
        return self._type.name

    def check_types(self, data):
        return self.type_checker.encode(data)

    def check_constraints(self, data):
        return self.constraints_checker.encode(data)

    def encode(self, data):
        raise NotImplementedError("This codec does not support encode().")

    def decode(self, data):
        raise NotImplementedError("This codec does not support decode().")

    def decode_with_length(self, data):
        raise NotImplementedError("This codec does not support decode_with_length().")

    def __repr__(self):
        return repr(self._type)


class Recursive(object):
    """
    Mixin used to identify Recursive types
    """

    type_name: str

    def __repr__(self):
        return "{}({})".format(type(self).__name__, self.type_name)


class OpenType(object):
    def __init__(self, name, table):
        self._name = name
        self._object_set_name = table[0]
        self._table = []

        for item in table[1]:
            offset = item.count(".")
            offset *= -1
            self._table.append((offset, item.lstrip(".")))

    def __repr__(self):
        return "OpenType({}, {}, {})".format(
            self._name, self._object_set_name, self._table
        )


class OpenTypeSequence(object):
    def __init__(self, name, members):
        self._name = name
        self._members = members

    def __repr__(self):
        return "OpenTypeSequence({}, {})".format(self._name, self._members)


class OpenTypeSequenceOf(object):
    def __init__(self, name, element_type):
        self._name = name
        self._element_type = element_type

    def __repr__(self):
        return "OpenTypeSequenceOf({}, {})".format(self._name, self._element_type)


class CompiledOpenTypes(CompiledType):
    def __init__(self, compiled_open_types, compiled_type):
        super(CompiledOpenTypes, self).__init__(compiled_type)
        self._compiled_open_types = compiled_open_types

    @property
    def type(self):
        return self._type.type

    def encode(self, data, **kwargs):
        # print()
        # print('data:', data)
        # print(self._compiled_open_types)

        return self._type.encode(data, **kwargs)

    def decode(self, data):
        return self._type.decode(data)


class Compiler(object):
    def __init__(self, specification, numeric_enums=False):
        self._specification = specification
        self._numeric_enums = numeric_enums
        self._types_backtrace = []
        self.recursive_types = []
        self.compiled = {}
        self.current_type_descriptor: Any = None

    def types_backtrace_push(self, type_name):
        self._types_backtrace.append(type_name)

    def types_backtrace_pop(self):
        self._types_backtrace.pop()

    @property
    def types_backtrace(self):
        return self._types_backtrace

    def process(self):
        self.pre_process()

        compiled: dict[str, Any] = {}

        for module_name in self._specification:
            items = self._specification[module_name]["types"].items()

            for type_name, type_descriptor in items:
                self.types_backtrace_push(type_name)
                compiled_type = self.process_type(
                    type_name, type_descriptor, module_name
                )
                compiled_type.type_name = type_name
                compiled_type.module_name = module_name
                compiled_open_types = self.compile_open_types(
                    type_name, type_descriptor, module_name
                )

                if compiled_open_types:
                    compiled_type = CompiledOpenTypes(
                        compiled_open_types, compiled_type
                    )

                self.types_backtrace_pop()

                if module_name not in compiled:
                    compiled[module_name] = {}

                compiled[module_name][type_name] = compiled_type

        for recursive_type in self.recursive_types:
            compiled_module = compiled[recursive_type.module_name]
            inner_type = compiled_module[recursive_type.type_name].type
            recursive_type.set_inner_type(inner_type)

        return compiled

    def pre_process(self):
        for module_name, module in self._specification.items():
            types = module["types"]
            type_descriptors = types.values()

            self.pre_process_components_of(type_descriptors, module_name)
            self.pre_process_extensibility_implied(module, type_descriptors)
            self.pre_process_tags(module, module_name)
            self.pre_process_default_value(type_descriptors, module_name)

        for module_name, module in self._specification.items():
            self.pre_process_parameterization_step_1(module["types"], module_name)

        for module in self._specification.values():
            types = module["types"]
            types = self.pre_process_parameterization_step_2(types)
            module["types"] = types

        for module_name, module in self._specification.items():
            for type_descriptor in module["types"].values():
                self.pre_process_object_class_fields(type_descriptor, module_name)

        return self._specification

    def pre_process_object_class_fields(self, type_descriptor, module_name):
        """Replace each field of an object class, as ``CLS.&id``, by the
        type of the field, wherever it is used.

        """

        while is_object_class_type_name(type_descriptor["type"]):
            converted = self.convert_object_class_type_descriptor(
                type_descriptor, module_name
            )
            type_descriptor.clear()
            type_descriptor.update(converted)

        module_name = type_descriptor.get("module-name", module_name)

        for member in type_descriptor.get("members", []):
            if member == EXTENSION_MARKER:
                continue

            for inner in member if isinstance(member, list) else [member]:
                if inner != EXTENSION_MARKER:
                    self.pre_process_object_class_fields(inner, module_name)

        if "element" in type_descriptor:
            self.pre_process_object_class_fields(
                type_descriptor["element"], module_name
            )

    def pre_process_components_of(self, type_descriptors, module_name):
        """COMPONENTS OF expansion."""

        for type_descriptor in type_descriptors:
            self.pre_process_components_of_type(type_descriptor, module_name)

    def pre_process_components_of_type(self, type_descriptor, module_name):
        if "members" not in type_descriptor:
            return

        type_descriptor["members"] = self.pre_process_components_of_expand_members(
            type_descriptor["members"], module_name
        )

    def pre_process_components_of_expand_members(self, members, module_name):
        expanded_members = []

        for member in members:
            if member != EXTENSION_MARKER and "components-of" in member:
                type_descriptor, inner_module_name = self.lookup_type_descriptor(
                    member["components-of"], module_name
                )
                inner_members = self.pre_process_components_of_expand_members(
                    type_descriptor["members"], inner_module_name
                )

                for inner_member in inner_members:
                    if inner_member == EXTENSION_MARKER:
                        break

                    expanded_members.append(deepcopy(inner_member))
            else:
                expanded_members.append(member)

        return expanded_members

    def pre_process_extensibility_implied(self, module, type_descriptors):
        """Make all types extensible."""

        if not module["extensibility-implied"]:
            return

        for type_descriptor in type_descriptors:
            self.pre_process_extensibility_implied_type(type_descriptor)

    def pre_process_extensibility_implied_type(self, type_descriptor):
        if "members" not in type_descriptor:
            return

        members = type_descriptor["members"]

        for member in members:
            if member == EXTENSION_MARKER:
                continue

            if isinstance(member, list):
                for type_descriptor in member:
                    self.pre_process_extensibility_implied_type(type_descriptor)
            else:
                self.pre_process_extensibility_implied_type(member)

        if EXTENSION_MARKER not in members:
            members.append(EXTENSION_MARKER)

    def pre_process_tags(self, module, module_name):
        """Add tags where missing."""

        module_tags = module.get("tags", "EXPLICIT")

        for type_descriptor in module["types"].values():
            self.current_type_descriptor = type_descriptor
            self.pre_process_tags_type(type_descriptor, module_tags, module_name)

    def is_dummy_reference(self, type_name):
        if "parameters" not in self.current_type_descriptor:
            return False

        if type_name in self.current_type_descriptor["parameters"]:
            return True

        return False

    def pre_process_tags_type(self, type_descriptor, module_tags, module_name):
        type_name = type_descriptor["type"]

        if "tag" in type_descriptor:
            tag = type_descriptor["tag"]
            resolved_type_name = self.resolve_type_name(type_name, module_name)

            if isinstance(tag["number"], str):
                # A value by reference.
                value = self.lookup_value(tag["number"], module_name)[0]["value"]

                if not isinstance(value, int) or isinstance(value, bool):
                    raise CompileError(
                        "Tag number '{}' in module '{}' is not an integer.".format(
                            tag["number"], module_name
                        )
                    )

                tag["number"] = value

            if "kind" not in tag:
                if resolved_type_name == "CHOICE":
                    tag["kind"] = "EXPLICIT"
                elif self.is_dummy_reference(type_name):
                    tag["kind"] = "EXPLICIT"
                elif module_tags in ["IMPLICIT", "EXPLICIT"]:
                    tag["kind"] = module_tags
                else:
                    tag["kind"] = "IMPLICIT"

        # SEQUENCE, SET and CHOICE.
        if "members" in type_descriptor:
            self.pre_process_tags_type_members(
                type_descriptor, module_tags, module_name
            )

        # SEQUENCE OF and SET OF.
        if "element" in type_descriptor:
            self.pre_process_tags_type(
                type_descriptor["element"], module_tags, module_name
            )

    def pre_process_tags_type_members(self, type_descriptor, module_tags, module_name):
        def is_any_member_tagged(members):
            for member in members:
                if member == EXTENSION_MARKER:
                    continue

                if "tag" in member:
                    return True

            return False

        number = None
        members = flatten(type_descriptor["members"])

        # Add tag number to all members if AUTOMATIC TAGS are
        # selected and no member is tagged.
        if module_tags == "AUTOMATIC" and not is_any_member_tagged(members):
            number = 0

        for member in members:
            if member == EXTENSION_MARKER:
                continue

            if number is not None:
                if "tag" not in member:
                    member["tag"] = {}

                member["tag"]["number"] = number
                number += 1

            self.pre_process_tags_type(member, module_tags, module_name)

    def pre_process_default_value(self, type_descriptors, module_name):
        """SEQUENCE and SET default member value cleanup."""

        sequences_and_sets = self.get_type_descriptors(
            type_descriptors, ["SEQUENCE", "SET"]
        )

        for type_descriptor in sequences_and_sets:
            for member in type_descriptor["members"]:
                if member == EXTENSION_MARKER:
                    continue

                if "default" not in member:
                    continue

                resolved_member = self.resolve_type_descriptor(member, module_name)
                self.pre_process_default_value_reference(
                    member, resolved_member, module_name
                )

                if resolved_member["type"] == "BIT STRING":
                    self.pre_process_default_value_bit_string(member, resolved_member)

                if resolved_member["type"] == "OCTET STRING":
                    self.pre_process_default_value_octet_string(member)

                if self._numeric_enums:
                    member["default"] = self.numeric_enums_of_default(
                        member["default"], resolved_member, module_name
                    )

    def numeric_enums_of_default(self, default, resolved, module_name):
        """The items of ENUMERATED types in `default`, a value of the type
        `resolved`, as their numbers. A member, an alternative and an
        item is one of its own type.

        """

        type_name = resolved["type"]

        if type_name == "ENUMERATED":
            for key, value in resolved["values"]:
                if key == default:
                    return value
        elif type_name in ["SEQUENCE", "SET"] and isinstance(default, dict):
            members = {
                member["name"]: member
                for member in flatten(resolved["members"])
                if member != EXTENSION_MARKER
            }

            return {
                name: self.numeric_enums_of_default(
                    value,
                    self.resolve_type_descriptor(members[name], module_name),
                    module_name,
                )
                if name in members
                else value
                for name, value in default.items()
            }
        elif type_name == "CHOICE" and isinstance(default, tuple):
            for member in flatten(resolved["members"]):
                if member != EXTENSION_MARKER and member["name"] == default[0]:
                    return (
                        default[0],
                        self.numeric_enums_of_default(
                            default[1],
                            self.resolve_type_descriptor(member, module_name),
                            module_name,
                        ),
                    )
        elif type_name in ["SEQUENCE OF", "SET OF"] and isinstance(default, list):
            element = self.resolve_type_descriptor(resolved["element"], module_name)

            return [
                self.numeric_enums_of_default(item, element, module_name)
                for item in default
            ]

        return default

    def pre_process_default_value_reference(self, member, resolved_member, module_name):
        """Replace a default value given as a named number or as a
        reference to a number, a boolean or a string by what it is.

        """

        default = member["default"]

        if (
            not isinstance(default, str)
            or isinstance(default, CString)
            or not default[:1].islower()
        ):
            return

        # The name of an item of an enumeration is its value.
        if resolved_member["type"] == "ENUMERATED":
            return

        if default in resolved_member.get("named-numbers", {}):
            default = resolved_member["named-numbers"][default]

        if isinstance(default, str):
            try:
                default = self.lookup_value(default, module_name)[0]["value"]
            except CompileError:
                return

        # Any other value has to be converted to what it is when
        # encoded and decoded, which is not supported.
        if isinstance(default, (bool, int, float, CString)):
            member["default"] = default

    def pre_process_default_value_bit_string(self, member, resolved_member):
        default = member["default"]

        if isinstance(default, tuple):
            # Already pre-processed.
            return

        if isinstance(default, list):
            named_bits = dict(resolved_member["named-bits"])
            reversed_mask = 0

            for name in default:
                reversed_mask |= 1 << int(named_bits[name])

            mask = int(bin(reversed_mask)[2:][::-1], 2)
            number_of_bits = reversed_mask.bit_length()
        elif default.startswith("0x"):
            if len(default) % 2 == 1:
                default += "0"

            default = "01" + default[2:]
            mask = int(default, 16)
            mask >>= lowest_set_bit(mask)
            number_of_bits = mask.bit_length() - 1
            mask ^= 1 << number_of_bits
        elif default == "0b":
            number_of_bits = 0
        else:
            mask = int(default, 2)
            mask >>= lowest_set_bit(mask)
            number_of_bits = len(default) - 2

        if number_of_bits > 0:
            # The bits first in as many bytes as needed.
            encoded_mask = (mask << (-number_of_bits % 8)).to_bytes(
                (number_of_bits + 7) // 8, "big"
            )
        else:
            encoded_mask = b""

        member["default"] = (encoded_mask, number_of_bits)

    def pre_process_default_value_octet_string(self, member):
        default = member["default"]

        if sys.version_info[0] > 2 and isinstance(default, bytes):
            # Already pre-processed.
            return

        if default.startswith("0b"):
            default = default[2:]
            if len(default) % 8 != 0:
                default += "0" * (-len(default) % 8)
            member["default"] = binascii.unhexlify(
                hex(int("11111111" + default, 2))[4:]
            )
        elif default.startswith("0x"):
            default = default[2:]
            if len(default) % 2 == 1:
                default += "0"
            member["default"] = binascii.unhexlify(default)

    def pre_process_parameterization_step_1(self, types, module_name):
        """X.683 parameterization pre processing - step 1."""

        for type_name, type_descriptor in types.items():
            # Skip if the type is parameterized.
            if "parameters" in type_descriptor:
                continue

            self.pre_process_parameterization_step_1_type(
                type_descriptor, type_name, module_name
            )

    def pre_process_parameterization_step_1_type(
        self, type_descriptor, type_name, module_name
    ):
        """Recursively find and replace all parameterized types with their
        actual types.

        """

        # SEQUENCE, SET and CHOICE.
        if "members" in type_descriptor:
            for member in type_descriptor["members"]:
                if member == EXTENSION_MARKER:
                    continue

                self.pre_process_parameterization_step_1_type(
                    member, type_name, module_name
                )

        # SEQUENCE OF and SET OF.
        if "element" in type_descriptor:
            self.pre_process_parameterization_step_1_type(
                type_descriptor["element"], type_name, module_name
            )

        # Just return if the type is not using a parameterized type.
        if "actual-parameters" not in type_descriptor:
            return

        parameterized_type_descriptor, parameterized_module_name = (
            self.lookup_type_descriptor(type_descriptor["type"], module_name)
        )

        if "parameters" not in parameterized_type_descriptor:
            raise CompileError(
                "Type '{}' in module '{}' is not parameterized.".format(
                    type_descriptor["type"], parameterized_module_name
                )
            )

        dummy_parameters = parameterized_type_descriptor["parameters"]
        actual_parameters = type_descriptor["actual-parameters"]

        if len(dummy_parameters) != len(actual_parameters):
            raise CompileError(
                "Parameterized type '{}' in module '{}' takes {} "
                "parameters, but {} are given in type '{}' in "
                "module '{}'.".format(
                    type_descriptor["type"],
                    parameterized_module_name,
                    len(dummy_parameters),
                    len(actual_parameters),
                    type_name,
                    module_name,
                )
            )

        parameterized_type_descriptor = deepcopy(parameterized_type_descriptor)

        self.pre_process_parameterization_step_1_dummy_to_actual_type(
            parameterized_type_descriptor,
            dummy_parameters,
            actual_parameters,
            parameterized_module_name,
        )

        self.pre_process_parameterization_step_1_type(
            parameterized_type_descriptor, type_name, parameterized_module_name
        )

        type_descriptor.update(parameterized_type_descriptor)

        if "module-name" not in type_descriptor:
            if module_name != parameterized_module_name:
                type_descriptor["module-name"] = parameterized_module_name

        if "parameters" in type_descriptor:
            del type_descriptor["parameters"]

        type_descriptor.pop("parameter-governors", None)

        del type_descriptor["actual-parameters"]

    def pre_process_parameterization_step_1_dummy_to_actual_type(
        self, type_descriptor, dummy_parameters, actual_parameters, module_name
    ):
        if "members" in type_descriptor:
            for member in type_descriptor["members"]:
                if member == EXTENSION_MARKER:
                    continue

                self.pre_process_parameterization_step_1_dummy_to_actual_type(
                    member, dummy_parameters, actual_parameters, module_name
                )
        elif "element" in type_descriptor:
            self.pre_process_parameterization_step_1_dummy_to_actual_type(
                type_descriptor["element"],
                dummy_parameters,
                actual_parameters,
                module_name,
            )

        # Replace dummy with actual in current type descriptor.
        for dummy_parameter, actual_parameter in zip(
            dummy_parameters, actual_parameters
        ):
            if type_descriptor["type"] == dummy_parameter:
                type_descriptor.update(actual_parameter)

            if "actual-parameters" in type_descriptor:
                for i, parameter in enumerate(type_descriptor["actual-parameters"]):
                    type_descriptor["actual-parameters"][i] = (
                        self.pre_process_parameterization_step_1_dummy_to_actual_parameter(
                            parameter, dummy_parameter, actual_parameter
                        )
                    )

            if "size" in type_descriptor:
                actual_size = []

                for item in type_descriptor["size"]:
                    if isinstance(item, tuple):
                        minimum, maximum = item

                        if minimum == dummy_parameter:
                            minimum = actual_parameter

                        if maximum == dummy_parameter:
                            maximum = actual_parameter

                        item = (minimum, maximum)
                    elif item == dummy_parameter:
                        item = actual_parameter

                    actual_size.append(item)

                type_descriptor["size"] = actual_size

            if "restricted-to" in type_descriptor:
                actual_restricted_to: list = []

                for item in type_descriptor["restricted-to"]:
                    minimum, maximum = item

                    if minimum == dummy_parameter:
                        minimum = actual_parameter

                    if maximum == dummy_parameter:
                        maximum = actual_parameter

                    if isinstance(item, ValueRange):
                        actual_restricted_to.append(
                            ValueRange(
                                minimum,
                                maximum,
                                item.is_minimum_excluded,
                                item.is_maximum_excluded,
                            )
                        )
                    else:
                        actual_restricted_to.append((minimum, maximum))

                type_descriptor["restricted-to"] = actual_restricted_to

    def pre_process_parameterization_step_1_dummy_to_actual_parameter(
        self, parameter, dummy_parameter, actual_parameter
    ):
        """Returns given actual parameter of a nested parameterized type
        with the dummy parameter replaced by its actual parameter.

        """

        if not isinstance(parameter, dict):
            # A value.
            if parameter == dummy_parameter:
                return actual_parameter
        elif "object-set" in parameter:
            if dummy_parameter in parameter["object-set"]:
                members = []

                for member in parameter["object-set"]:
                    if member != dummy_parameter:
                        members.append(member)
                    elif isinstance(actual_parameter, dict):
                        members.extend(actual_parameter["object-set"])
                    else:
                        members.append(actual_parameter)

                return {"object-set": members}
        elif parameter["type"] == dummy_parameter:
            return actual_parameter

        return parameter

    def pre_process_parameterization_step_2(self, types):
        """X.683 parameterization pre processing - step 2."""

        # Remove parameterized types as they are no longer needed.
        return {
            type_name: type_descriptor
            for type_name, type_descriptor in types.items()
            if "parameters" not in type_descriptor
        }

    def resolve_type_name(self, type_name, module_name):
        """Returns the ASN.1 type name of given type."""

        try:
            while True:
                if is_object_class_type_name(type_name):
                    member_type, module_name = self.lookup_object_class_type(
                        type_name, module_name
                    )
                    type_name = member_type["type"]
                else:
                    type_descriptor, module_name = self.lookup_type_descriptor(
                        type_name, module_name
                    )
                    type_name = type_descriptor["type"]
        except CompileError:
            pass

        return type_name

    def resolve_type_descriptor(self, type_descriptor, module_name):
        type_name = type_descriptor["type"]

        try:
            while True:
                if is_object_class_type_name(type_name):
                    member_type, module_name = self.lookup_object_class_type(
                        type_name, module_name
                    )
                    type_name = member_type["type"]
                else:
                    type_descriptor, module_name = self.lookup_type_descriptor(
                        type_name, module_name
                    )
                    type_name = type_descriptor["type"]
        except CompileError:
            pass

        return type_descriptor

    def get_type_descriptors(self, type_descriptors, type_names):
        result = []

        for type_descriptor in type_descriptors:
            result += self.get_type_descriptors_type(type_descriptor, type_names)

        return result

    def get_type_descriptors_type(self, type_descriptor, type_names):
        type_descriptors = []
        type_name = type_descriptor["type"]

        if type_name in type_names:
            type_descriptors.append(type_descriptor)

        if "members" in type_descriptor:
            for member in type_descriptor["members"]:
                if member == EXTENSION_MARKER:
                    continue

                if isinstance(member, list):
                    type_descriptors.extend(
                        self.get_type_descriptors(member, type_names)
                    )
                else:
                    type_descriptors += self.get_type_descriptors_type(
                        member, type_names
                    )

        if "element" in type_descriptor:
            type_descriptors += self.get_type_descriptors_type(
                type_descriptor["element"], type_names
            )

        return type_descriptors

    def process_type(self, type_name, type_descriptor, module_name):
        return NotImplementedError("To be implemented by subclasses.")

    def compile_type(self, name, type_descriptor, module_name):
        return NotImplementedError("To be implemented by subclasses.")

    def compile_open_types(self, name, type_descriptor, module_name):
        """Compile the open types wrapper for given type. Returns ``None`` if
        given type does not have any open types.

        """

        compiled: OpenTypeSequence | OpenTypeSequenceOf | None = None
        type_name = type_descriptor["type"]

        if type_name in ["SEQUENCE", "SET"]:
            compiled_members = []

            for member in type_descriptor["members"]:
                if member == EXTENSION_MARKER:
                    continue

                if isinstance(member, list):
                    # ToDo: Handle groups.
                    continue

                if is_open_type(member["type"]):
                    if "table" in member:
                        table = member["table"]

                        if isinstance(table, list):
                            compiled_members.append(OpenType(member["name"], table))
                else:
                    compiled_member = self.compile_open_types(
                        member["name"], member, module_name
                    )

                    if compiled_member is not None:
                        compiled_members.append(compiled_member)

            if compiled_members:
                compiled = OpenTypeSequence(name, compiled_members)
        elif type_name in ["SEQUENCE OF", "SET OF"]:
            compiled_element = self.compile_open_types(
                "", type_descriptor["element"], module_name
            )

            if compiled_element:
                compiled = OpenTypeSequenceOf(name, compiled_element)
        elif type_name == "CHOICE":
            # ToDo: Handle CHOICE.
            pass
        else:
            pass

        return compiled

    def compile_user_type(self, name, type_name, module_name):
        compiled = self.get_compiled_type(name, type_name, module_name)

        if compiled is None:
            self.types_backtrace_push(type_name)
            compiled = self.compile_type(
                name, *self.lookup_type_descriptor(type_name, module_name)
            )
            compiled.type_name = type_name
            compiled.module_name = module_name
            self.types_backtrace_pop()
            self.set_compiled_type(name, type_name, module_name, compiled)

        return compiled

    def compile_members(self, members, module_name, sort_by_tag=False):
        compiled_members = []
        has_extension_marker = False

        for member in members:
            if member == EXTENSION_MARKER:
                has_extension_marker = True
                continue

            if isinstance(member, list):
                group_members, _ = self.compile_members(member, module_name)
                compiled_members.extend(group_members)
                continue

            compiled_member = self.compile_member(member, module_name)
            compiled_members.append(compiled_member)

        if sort_by_tag:
            compiled_members = sorted(compiled_members, key=attrgetter("tag"))

        return compiled_members, has_extension_marker

    def compile_root_member(self, member, module_name, compiled_members):
        compiled_member = self.compile_member(member, module_name)
        compiled_members.append(compiled_member)

    def compile_member(self, member, module_name):
        compiled_member = self.compile_type(member["name"], member, module_name)

        if "optional" in member:
            compiled_member = self.copy(compiled_member)
            compiled_member.optional = member["optional"]

        if "default" in member:
            compiled_member = self.copy(compiled_member)
            compiled_member.set_default(member["default"])

        if "size" in member:
            compiled_member = self.copy(compiled_member)
            compiled_member.set_size_range(*self.get_size_range(member, module_name))

        return compiled_member

    def sort_members_by_tag(self, compiled_members, members, module_name):
        """Returns given compiled root members of a SET or a CHOICE in the
        canonical order of their tags (X.680 8.6): the class and then the
        number. A member with no tag, as an ANY, is last.

        """

        return self.in_canonical_order(
            compiled_members, root_members(members), module_name
        )

    def sort_additions_by_tag(self, compiled_additions, members, module_name):
        """Returns given compiled additions of a CHOICE, with those of groups
        among them, in the canonical order of their tags.

        """

        return self.in_canonical_order(
            compiled_additions, addition_members(members), module_name
        )

    def in_canonical_order(self, compiled_members, members, module_name):
        keys = [self.first_tag(member, module_name) for member in members]
        order = sorted(
            range(len(members)),
            key=lambda index: (keys[index] is None, keys[index] or (0, 0)),
        )

        return [compiled_members[index] for index in order]

    def first_tag(self, type_descriptor, module_name, seen=None):
        """Returns the tag given type is in the place of in the canonical
        order of tags, as the class and the number, or ``None`` if it may
        begin with anything. That of a CHOICE with no tag of its own is the
        least of those of the alternatives of its root (X.691 21 and 23.3,
        X.696 22).

        """

        tag = self.wire_tag(type_descriptor, module_name)

        if tag is not None:
            return tag

        type_descriptor, module_name = self.concrete_type_descriptor(
            type_descriptor, module_name
        )

        if type_descriptor is None or type_descriptor["type"] != "CHOICE":
            return None

        if seen is None:
            seen = set()

        if id(type_descriptor) in seen:
            return None

        seen.add(id(type_descriptor))
        tags = []

        for member in root_members(type_descriptor["members"]):
            tag = self.first_tag(member, module_name, seen)

            if tag is not None:
                tags.append(tag)

        return min(tags, default=None)

    def wire_tag(self, type_descriptor, module_name, depth=0):
        """Returns the outermost tag given type has on the wire, or ``None``
        if it has none, as a CHOICE. An implicit tag of a type with no
        tag does nothing.

        """

        if depth > 1000:
            return None

        module_name = type_descriptor.get("module-name", module_name)
        tag = type_descriptor.get("tag")

        if tag is not None:
            key = (CLASS_ORDER[tag.get("class") or "CONTEXT_SPECIFIC"], tag["number"])

            if tag.get("kind") == "EXPLICIT":
                return key

            untagged = dict(type_descriptor)
            del untagged["tag"]

            if self.wire_tag(untagged, module_name, depth + 1) is None:
                return None

            return key

        type_name = type_descriptor["type"]

        if type_name in UNIVERSAL_TAGS:
            return (0, UNIVERSAL_TAGS[type_name])

        if type_name in UNTAGGED_TYPES:
            return None

        try:
            referenced, module_name = self.lookup_type_descriptor(
                type_name, module_name
            )
        except CompileError:
            return None

        return self.wire_tag(referenced, module_name, depth + 1)

    def concrete_type_descriptor(self, type_descriptor, module_name):
        """Returns the type given type ends in, past references, and its
        module.

        """

        for _ in range(1000):
            module_name = type_descriptor.get("module-name", module_name)
            type_name = type_descriptor["type"]

            if type_name in UNIVERSAL_TAGS or type_name in UNTAGGED_TYPES:
                return type_descriptor, module_name

            try:
                type_descriptor, module_name = self.lookup_type_descriptor(
                    type_name, module_name
                )
            except CompileError:
                return None, module_name

        return None, module_name

    def get_size_range(self, type_descriptor, module_name) -> tuple[Any, Any, Any]:
        """Returns the least and the greatest sizes of the root of the size
        constraint of given type, which is the union of its elements, with
        ``MIN`` and ``MAX`` for no bound, and if it is extensible. Returns
        ``(None, None, None)`` if the type does not have a size constraint.

        """

        size = type_descriptor.get("size", None)

        if size is None:
            return None, None, None

        parts = self.get_size_parts(type_descriptor, module_name)

        return hull(parts) + (EXTENSION_MARKER in size,)

    def get_size_parts(self, type_descriptor, module_name):
        """Returns the sizes and the ranges of sizes of the root of the size
        constraint of given type, or ``None`` if it has none.

        """

        size = type_descriptor.get("size", None)

        if size is None:
            return None

        def convert_value(value):
            if value in ["MIN", "MAX"]:
                return value

            return self.lookup_value(value, module_name)[0]["value"]

        return constraint_parts(size, convert_value)

    def get_enum_values(self, type_descriptor, module_name):
        """Converts the enum values to ints if they are value references."""

        enum_values = []

        for value in type_descriptor["values"]:
            if value != EXTENSION_MARKER and not isinstance(value[1], int):
                lookup = self.lookup_value(value[1], module_name)
                enum_values.append((value[0], lookup[0]["value"]))
            else:
                enum_values.append(value)

        return enum_values

    def get_named_numbers(self, type_descriptor, module_name):
        """Returns the number of each named number of given INTEGER. A
        number given as a reference to a value is what that value is.

        """

        named_numbers = {}

        for name, number in type_descriptor.get("named-numbers", {}).items():
            if isinstance(number, str):
                try:
                    number = self.lookup_value(number, module_name)[0]["value"]
                except CompileError:
                    continue

            if isinstance(number, int):
                named_numbers[name] = number

        return named_numbers

    def get_restricted_to_range(self, type_descriptor, module_name):
        """Returns the least and the greatest values of the root of the value
        constraint of given type, which is the union of its elements, with
        ``MIN`` and ``MAX`` for no bound, and if it is extensible.

        """

        parts = self.get_restricted_to_parts(type_descriptor, module_name)
        restricted_to = type_descriptor["restricted-to"]

        return hull(parts) + (EXTENSION_MARKER in restricted_to,)

    def get_restricted_to_parts(self, type_descriptor, module_name):
        """Returns the values and the ranges of values of the root of the value
        constraint of given type.

        """

        def convert_value(value):
            # A string given in quotes is not a reference to a value.
            if isinstance(value, CString):
                return value

            try:
                return float(value)
            except ValueError:
                if is_type_name(value):
                    return value

                try:
                    resolved_type_descriptor = self.resolve_type_descriptor(
                        type_descriptor, module_name
                    )

                    return resolved_type_descriptor["named-numbers"][value]
                except KeyError:
                    return self.lookup_value(value, module_name)[0]["value"]

        return constraint_parts(type_descriptor["restricted-to"], convert_value)

    def get_permitted_alphabet(self, type_descriptor):
        """The permitted alphabet of given type, in the form a codec takes it,
        for those that use one.

        """

        return None

    def with_own_constraints(self, compiled, type_descriptor, module_name):
        """Returns given compiled type of a reference with the size and the
        permitted alphabet of its own, which the compiler has worked out of
        those of the type it refers to and those of the reference.

        """

        if "size" in type_descriptor:
            compiled = self.copy(compiled)
            compiled.set_size_range(*self.get_size_range(type_descriptor, module_name))

        if "from" in type_descriptor and hasattr(compiled, "set_permitted_alphabet"):
            compiled = self.copy(compiled)
            compiled.set_permitted_alphabet(
                self.get_permitted_alphabet(type_descriptor)
            )

        return compiled

    def get_with_components(self, type_descriptor):
        return type_descriptor.get("with-components", None)

    def get_named_bits(self, type_descriptor, module_name):
        named_bits = type_descriptor.get("named-bits", None)
        if named_bits is not None:
            named_bit_values = []

            for value in named_bits:
                if value != EXTENSION_MARKER and not value[1].isdigit():
                    lookup = self.lookup_value(value[1], module_name)
                    named_bit_values.append((value[0], lookup[0]["value"]))
                else:
                    named_bit_values.append((value[0], int(value[1])))

            return named_bit_values

    def is_explicit_tag(self, type_descriptor):
        try:
            return type_descriptor["tag"]["kind"] == "EXPLICIT"
        except KeyError:
            pass

        return False

    def get_module_name(self, type_descriptor, module_name):
        module_name = type_descriptor.get("module-name", module_name)
        try:
            _, module_name = self.lookup_type_descriptor(
                type_descriptor["type"], module_name
            )
        except CompileError:
            pass
        return module_name

    def lookup_in_modules(self, section, debug_string, name, module_name):
        begin_debug_string = debug_string[:1].upper() + debug_string[1:]
        module = self._specification[module_name]

        if name in module[section]:
            return module[section][name], module_name
        else:
            for from_module_name, imports in module["imports"].items():
                if name not in imports:
                    continue

                if from_module_name not in self._specification:
                    raise CompileError(
                        "Module '{}' cannot import {} '{}' from missing "
                        "module '{}'.".format(
                            module_name, debug_string, name, from_module_name
                        )
                    )

                try:
                    return self.lookup_in_modules(
                        section, debug_string, name, from_module_name
                    )
                except CompileError:
                    raise CompileError(
                        "{} '{}' imported by module '{}' not found in "
                        "module '{}'.".format(
                            begin_debug_string, name, module_name, from_module_name
                        )
                    )

        raise CompileError(
            "{} '{}' not found in module '{}'.".format(
                begin_debug_string, name, module_name
            )
        )

    def lookup_type_descriptor(self, type_name, module_name):
        return self.lookup_in_modules("types", "type", type_name, module_name)

    def lookup_value(self, value_name, module_name):
        return self.lookup_in_modules("values", "value", value_name, module_name)

    def lookup_object_class_descriptor(self, object_class_name, module_name):
        return self.lookup_in_modules(
            "object-classes", "object class", object_class_name, module_name
        )

    def lookup_object_class_type(self, type_name, module_name):
        class_name, member_name = type_name.split(".")
        result = self.lookup_object_class_descriptor(class_name, module_name)
        object_class_descriptor, module_name = result

        for member in object_class_descriptor["members"]:
            if member["name"] == member_name:
                return member, module_name

    def get_compiled_type(self, name, type_name, module_name):
        try:
            return self.compiled[module_name][type_name][name]
        except KeyError:
            return None

    def set_compiled_type(self, name, type_name, module_name, compiled):
        if module_name not in self.compiled:
            self.compiled[module_name] = {}

        if type_name not in self.compiled[module_name]:
            self.compiled[module_name][type_name] = {}

        self.compiled[module_name][type_name][name] = compiled

    def convert_object_class_type_descriptor(self, type_descriptor, module_name):
        """Returns the type of the field of an object class that given type
        is, without the tag of the field. The name, the tag and the
        constraints of given type take the place of those of the field.
        Names in the type of the field are found in the module of the
        class.

        """

        found = self.lookup_object_class_type(
            type_descriptor["type"], type_descriptor.get("module-name", module_name)
        )

        if found is None:
            raise CompileError(
                "Field '{}' not found in module '{}'.".format(
                    type_descriptor["type"], module_name
                )
            )

        field, class_module_name = found
        converted = deepcopy(field)

        for key in ["name", "tag", "optional", "default"]:
            converted.pop(key, None)

        if "module-name" not in converted and class_module_name != module_name:
            converted["module-name"] = class_module_name

        for key, value in type_descriptor.items():
            if key not in ["type", "module-name"]:
                converted[key] = deepcopy(value)

        return converted

    def copy(self, compiled_type):
        if not isinstance(compiled_type, Recursive):
            compiled_type = copy(compiled_type)

        return compiled_type

    def set_compiled_restricted_to(self, compiled, type_descriptor, module_name):
        compiled = self.copy(compiled)
        compiled.set_restricted_to_range(
            *self.get_restricted_to_range(type_descriptor, module_name)
        )

        return compiled

    def external_type_descriptor(self):
        return {
            "type": "SEQUENCE",
            "tag": {"class": "UNIVERSAL", "number": 8, "kind": "IMPLICIT"},
            "members": [
                {
                    "name": "direct-reference",
                    "type": "OBJECT IDENTIFIER",
                    "optional": True,
                },
                {"name": "indirect-reference", "type": "INTEGER", "optional": True},
                {
                    "name": "data-value-descriptor",
                    "type": "ObjectDescriptor",
                    "optional": True,
                },
                {
                    "name": "encoding",
                    "type": "CHOICE",
                    "members": [
                        {
                            # [0] ABSTRACT-SYNTAX.&Type, an open type, in an
                            # environment of EXPLICIT TAGS (X.690 8.18.1).
                            "name": "single-ASN1-type",
                            "type": "OpenType",
                            "tag": {"number": 0, "kind": "EXPLICIT"},
                        },
                        {
                            "name": "octet-aligned",
                            "type": "OCTET STRING",
                            "tag": {"number": 1, "kind": "IMPLICIT"},
                        },
                        {
                            "name": "arbitrary",
                            "type": "BIT STRING",
                            "tag": {"number": 2, "kind": "IMPLICIT"},
                        },
                    ],
                },
            ],
        }


def enum_values_as_dict(values):
    return {value[1]: value[0] for value in values if value != EXTENSION_MARKER}


def enum_values_split(values):
    if EXTENSION_MARKER in values:
        index = values.index(EXTENSION_MARKER)

        return values[:index], values[index + 1 :]
    else:
        return values, None


def pre_process(specification):
    return Compiler(specification).pre_process()
