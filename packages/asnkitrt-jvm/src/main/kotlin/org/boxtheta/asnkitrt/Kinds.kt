// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

/** A kind of node: its number, its name and its fields, as `docs/file_formats.md` writes them. */
internal data class KindOfNode(val number: Int, val name: String, val fields: String)

/**
 * The kinds of nodes of `docs/file_formats.md`, with their fields in the
 * order of the compact form. `*type fields*` and `*constraint fields*`
 * stand for the fields all types and all constraints have. A test
 * compares this with the table of the specification, so that they do not
 * drift apart.
 */
internal val KINDS: List<KindOfNode> =
    listOf(
        KindOfNode(1, "CompiledSpecification", "modules"),
        KindOfNode(2, "CompiledModule", "name, types = [], object_classes = []"),
        KindOfNode(3, "TypeAssignment", "name, type, location = null"),
        KindOfNode(4, "Member", "name, type, optional = false, default = null, location = null"),
        KindOfNode(5, "Tag", "number, tag_class = null, mode = null"),
        KindOfNode(6, "Range", "lower, upper"),
        KindOfNode(7, "NamedNumber", "name, number"),
        KindOfNode(8, "ExtensionMarker", ""),
        KindOfNode(9, "ExtensionAdditionGroup", "members"),
        KindOfNode(10, "BuiltinType", "name, *type fields*"),
        KindOfNode(11, "ReferencedType", "name, *type fields*"),
        KindOfNode(12, "IntegerType", "named_numbers = null, *type fields*"),
        KindOfNode(13, "BitStringType", "named_bits = null, *type fields*"),
        KindOfNode(14, "EnumeratedType", "values, *type fields*"),
        KindOfNode(15, "SequenceType", "members, *type fields*"),
        KindOfNode(16, "SetType", "members, *type fields*"),
        KindOfNode(17, "ChoiceType", "members, *type fields*"),
        KindOfNode(18, "SequenceOfType", "element, element_name = null, *type fields*"),
        KindOfNode(19, "SetOfType", "element, element_name = null, *type fields*"),
        KindOfNode(20, "Min", ""),
        KindOfNode(21, "Max", ""),
        KindOfNode(22, "Identifier", "name"),
        KindOfNode(23, "HexString", "digits"),
        KindOfNode(24, "BinaryString", "bits"),
        KindOfNode(25, "Null", ""),
        KindOfNode(26, "PlusInfinity", ""),
        KindOfNode(27, "MinusInfinity", ""),
        KindOfNode(28, "NotANumber", ""),
        KindOfNode(29, "NamedValue", "name, value"),
        KindOfNode(30, "SequenceValue", "members"),
        KindOfNode(31, "UnsupportedValue", ""),
        KindOfNode(32, "Excluded", "value"),
        KindOfNode(33, "TableConstraint", "object_set, component_ids = null"),
        KindOfNode(34, "ComponentConstraint", "name, presence = null, *constraint fields*"),
        KindOfNode(35, "ElementConstraint", "*constraint fields*"),
        KindOfNode(36, "AnyDefinedByType", "value, choices = [], *type fields*"),
        KindOfNode(37, "AnyDefinedByChoice", "value, type"),
        KindOfNode(38, "ComponentsOf", "type_name"),
        KindOfNode(39, "ObjectClassAssignment", "name, fields, syntax = null, location = null"),
        KindOfNode(40, "ObjectClassField", "name, type, location = null"),
        KindOfNode(41, "SyntaxTree", "modules"),
        KindOfNode(
            42,
            "Module",
            "name, tags = null, extensibility_implied = false, imports = [], types = [], values = [], " +
                "objects = [], object_classes = [], object_sets = [], location = null",
        ),
        KindOfNode(43, "Import", "module, symbols"),
        KindOfNode(44, "ValueAssignment", "name, type, value, location = null"),
        KindOfNode(45, "ObjectAssignment", "name, class_name, object, location = null"),
        KindOfNode(46, "ObjectSetAssignment", "name, class_name, members, location = null"),
        KindOfNode(47, "Object", "fields"),
        KindOfNode(48, "ObjectField", "name, setting"),
        KindOfNode(49, "UnresolvedObject", "source"),
        KindOfNode(50, "ObjectSet", "members"),
        KindOfNode(51, "OptionalGroup", "tokens"),
        KindOfNode(52, "Parameter", "name, governor = null"),
        KindOfNode(53, "ElementSetSpecs", "root, extensible = false, additions = null"),
        KindOfNode(54, "Union", "elements"),
        KindOfNode(55, "Intersection", "elements"),
        KindOfNode(56, "Except", "elements, exclusions"),
        KindOfNode(57, "All", "exclusions"),
        KindOfNode(58, "SizeConstraint", "constraint"),
        KindOfNode(59, "PermittedAlphabet", "constraint"),
        KindOfNode(60, "ContainedSubtype", "type, includes = false"),
        KindOfNode(61, "PatternConstraint", "value"),
        KindOfNode(62, "InnerTypeConstraint", "constraint"),
        KindOfNode(63, "InnerTypeConstraints", "partial = false, components"),
        KindOfNode(64, "NamedConstraint", "name, constraints = null, presence = null"),
        KindOfNode(65, "ContentsConstraint", "type = null, encoded_by = null"),
        KindOfNode(66, "UserDefinedConstraint", ""),
        KindOfNode(67, "UnsupportedConstraint", ""),
    )

/** The number of the kind with this name. */
internal fun kindNumber(name: String): Int = KINDS.first { it.name == name }.number
