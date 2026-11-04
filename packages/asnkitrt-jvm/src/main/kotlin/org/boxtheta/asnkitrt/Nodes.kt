// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigInteger

// The nodes of a compiled specification (kinds 1 to 40 of
// `docs/file_formats.md`), read from the compact form.

private val COMPILED_SPECIFICATION = kindNumber("CompiledSpecification")
private val COMPILED_MODULE = kindNumber("CompiledModule")
private val TYPE_ASSIGNMENT = kindNumber("TypeAssignment")
private val MEMBER = kindNumber("Member")
private val RANGE = kindNumber("Range")
private val NAMED_NUMBER = kindNumber("NamedNumber")
private val EXTENSION_MARKER = kindNumber("ExtensionMarker")
private val EXTENSION_ADDITION_GROUP = kindNumber("ExtensionAdditionGroup")
private val BUILTIN_TYPE = kindNumber("BuiltinType")
private val REFERENCED_TYPE = kindNumber("ReferencedType")
private val INTEGER_TYPE = kindNumber("IntegerType")
private val BIT_STRING_TYPE = kindNumber("BitStringType")
private val ENUMERATED_TYPE = kindNumber("EnumeratedType")
private val SEQUENCE_TYPE = kindNumber("SequenceType")
private val SET_TYPE = kindNumber("SetType")
private val CHOICE_TYPE = kindNumber("ChoiceType")
private val SEQUENCE_OF_TYPE = kindNumber("SequenceOfType")
private val SET_OF_TYPE = kindNumber("SetOfType")
private val MIN = kindNumber("Min")
private val MAX = kindNumber("Max")
private val IDENTIFIER = kindNumber("Identifier")
private val HEX_STRING = kindNumber("HexString")
private val BINARY_STRING = kindNumber("BinaryString")
private val NULL = kindNumber("Null")
private val PLUS_INFINITY = kindNumber("PlusInfinity")
private val MINUS_INFINITY = kindNumber("MinusInfinity")
private val NOT_A_NUMBER = kindNumber("NotANumber")
private val NAMED_VALUE = kindNumber("NamedValue")
private val SEQUENCE_VALUE = kindNumber("SequenceValue")
private val UNSUPPORTED_VALUE = kindNumber("UnsupportedValue")
private val EXCLUDED = kindNumber("Excluded")
private val TABLE_CONSTRAINT = kindNumber("TableConstraint")
private val COMPONENT_CONSTRAINT = kindNumber("ComponentConstraint")
private val ELEMENT_CONSTRAINT = kindNumber("ElementConstraint")
private val ANY_DEFINED_BY_TYPE = kindNumber("AnyDefinedByType")
private val ANY_DEFINED_BY_CHOICE = kindNumber("AnyDefinedByChoice")
private val OBJECT_CLASS_ASSIGNMENT = kindNumber("ObjectClassAssignment")
private val OBJECT_CLASS_FIELD = kindNumber("ObjectClassField")
private val OPTIONAL_GROUP = kindNumber("OptionalGroup")

private val TYPE_KINDS =
    setOf(
        BUILTIN_TYPE,
        REFERENCED_TYPE,
        INTEGER_TYPE,
        BIT_STRING_TYPE,
        ENUMERATED_TYPE,
        SEQUENCE_TYPE,
        SET_TYPE,
        CHOICE_TYPE,
        SEQUENCE_OF_TYPE,
        SET_OF_TYPE,
        ANY_DEFINED_BY_TYPE,
    )

/** The number of a kind of node, if `item` begins with one: an array of it and its fields. */
private fun kindOf(item: Cbor): BigInteger? = ((item as? Cbor.Array)?.items?.firstOrNull() as? Cbor.Integer)?.value

/** A node: its kind and its fields, as found in the file. */
private class Node(
    val kind: Int,
    val fields: List<Cbor>,
) {
    /** The field at `index`, or `null` if it is null or left out. */
    fun get(index: Int): Cbor? = fields.getOrNull(index)?.takeUnless { it == Cbor.Null }

    fun required(index: Int): Cbor = get(index) ?: badNodes("A field is missing.")

    fun text(index: Int): String = (required(index) as? Cbor.Text)?.value ?: badNodes("Expected a text string.")

    fun optionalText(index: Int): String? =
        when (val item = get(index)) {
            null -> null
            is Cbor.Text -> item.value
            else -> badNodes("Expected a text string.")
        }

    fun boolean(
        index: Int,
        default: Boolean,
    ): Boolean =
        when (val item = get(index)) {
            null -> default
            is Cbor.Bool -> item.value
            else -> badNodes("Expected a boolean.")
        }

    companion object {
        fun of(item: Cbor): Node {
            val kind = kindOf(item)

            if (kind == null || kind < BigInteger.ONE || kind > BigInteger.valueOf(255)) badNodes("Expected a node.")

            val items = (item as Cbor.Array).items

            return Node(kind.toInt(), items.subList(1, items.size))
        }
    }
}

/** The items of a list, which is an array of 0 and the items. */
private fun list(item: Cbor): List<Cbor> {
    if (kindOf(item) != BigInteger.ZERO) badNodes("Expected a list.")

    val items = (item as Cbor.Array).items

    return items.subList(1, items.size)
}

private fun <T> mapList(
    item: Cbor?,
    read: (Cbor) -> T,
): List<T> = if (item == null) emptyList() else list(item).map(read)

/** A value, as the default of a member or a bound of a range. */
internal sealed class Const {
    data class Integer(
        val value: BigInteger,
    ) : Const()

    data class Real(
        val value: Double,
    ) : Const()

    data class Bool(
        val value: Boolean,
    ) : Const()

    data class Text(
        val value: String,
    ) : Const()

    /**
     * The `Text` of a choice of an ANY DEFINED BY that is defined by an
     * OBJECT IDENTIFIER, which the lowering makes, so that the value of the
     * member is compared with it as bytes.
     */
    data class ObjectIdentifier(
        val value: Value.ObjectIdentifier,
    ) : Const()

    data class Identifier(
        val name: String,
    ) : Const()

    /** Digits of a hexadecimal string. */
    data class Hex(
        val digits: String,
    ) : Const()

    /** Digits of a binary string. */
    data class Binary(
        val bits: String,
    ) : Const()

    data object Null : Const()

    data object PlusInfinity : Const()

    data object MinusInfinity : Const()

    data object NotANumber : Const()

    data object Min : Const()

    data object Max : Const()

    data class Named(
        val name: String,
        val value: Const,
    ) : Const()

    data class Sequence(
        val members: List<Pair<String, Const>>,
    ) : Const()

    data class Items(
        val items: List<Const>,
    ) : Const()

    data object Unsupported : Const()
}

private fun readConst(item: Cbor): Const =
    when (item) {
        is Cbor.Integer -> Const.Integer(item.value)
        is Cbor.Float -> Const.Real(item.value)
        is Cbor.Bool -> Const.Bool(item.value)
        is Cbor.Text -> Const.Text(item.value)
        is Cbor.Array ->
            if (kindOf(item) == BigInteger.ZERO) {
                Const.Items(list(item).map(::readConst))
            } else {
                val node = Node.of(item)

                when (node.kind) {
                    MIN -> Const.Min
                    MAX -> Const.Max
                    IDENTIFIER -> Const.Identifier(node.text(0))
                    HEX_STRING -> Const.Hex(node.text(0))
                    BINARY_STRING -> Const.Binary(node.text(0))
                    NULL -> Const.Null
                    PLUS_INFINITY -> Const.PlusInfinity
                    MINUS_INFINITY -> Const.MinusInfinity
                    NOT_A_NUMBER -> Const.NotANumber
                    NAMED_VALUE -> Const.Named(node.text(0), readConst(node.required(1)))
                    SEQUENCE_VALUE ->
                        Const.Sequence(
                            mapList(node.get(0)) { member ->
                                val inner = Node.of(member)

                                inner.text(0) to readConst(inner.required(1))
                            },
                        )
                    UNSUPPORTED_VALUE -> Const.Unsupported
                    else -> badNodes("Expected a value.")
                }
            }
        else -> badNodes("Expected a value.")
    }

/** A bound of a range. */
internal sealed class Bound {
    abstract val value: Const

    data class Included(
        override val value: Const,
    ) : Bound()

    /** The value is not part of the range. */
    data class Excluded(
        override val value: Const,
    ) : Bound()
}

private fun readBound(item: Cbor): Bound =
    if (kindOf(item) == BigInteger.valueOf(EXCLUDED.toLong())) {
        Bound.Excluded(readConst(Node.of(item).required(0)))
    } else {
        Bound.Included(readConst(item))
    }

/** An element of a constraint. */
internal sealed class Element {
    data class Range(
        val lower: Bound,
        val upper: Bound,
    ) : Element()

    data object ExtensionMarker : Element()

    data class Type(
        val type: org.boxtheta.asnkitrt.Type,
    ) : Element()

    data class Value(
        val value: Const,
    ) : Element()
}

private fun readElement(item: Cbor): Element {
    val kind = kindOf(item)

    if (kind != null) {
        when {
            kind == BigInteger.valueOf(RANGE.toLong()) -> {
                val node = Node.of(item)

                return Element.Range(readBound(node.required(0)), readBound(node.required(1)))
            }
            kind == BigInteger.valueOf(EXTENSION_MARKER.toLong()) -> return Element.ExtensionMarker
            kind.bitLength() < 32 && kind.toInt() in TYPE_KINDS -> return Element.Type(readType(item))
        }
    }

    return Element.Value(readConst(item))
}

private fun readElements(item: Cbor?): List<Element>? = item?.let { list(it).map(::readElement) }

internal enum class TagClass {
    UNIVERSAL,
    APPLICATION,
    CONTEXT_SPECIFIC,
    PRIVATE,
}

internal enum class TagMode {
    IMPLICIT,
    EXPLICIT,
}

/** A tag. Its `mode` is `null` if the compiler did not say, which it always does for a tag that is applied. */
internal data class Tag(
    val number: ULong,
    val tagClass: TagClass,
    val mode: TagMode?,
)

private fun readTag(item: Cbor): Tag {
    val node = Node.of(item)
    val number = (node.required(0) as? Cbor.Integer)?.value ?: badNodes("Expected the number of a tag.")
    val tagClass =
        when (node.optionalText(1)) {
            null -> TagClass.CONTEXT_SPECIFIC
            "UNIVERSAL" -> TagClass.UNIVERSAL
            "APPLICATION" -> TagClass.APPLICATION
            "PRIVATE" -> TagClass.PRIVATE
            else -> badNodes("Unknown class of a tag.")
        }
    val mode =
        when (node.optionalText(2)) {
            null -> null
            "IMPLICIT" -> TagMode.IMPLICIT
            "EXPLICIT" -> TagMode.EXPLICIT
            else -> badNodes("Unknown mode of a tag.")
        }

    // From 0 to 2^64 - 1, as the runtime in Rust reads it.
    if (number.signum() < 0 || number.bitLength() > 64) badNodes("Bad number of a tag.")

    return Tag(number.toLong().toULong(), tagClass, mode)
}

internal data class NamedNumber(
    val name: String,
    val number: Const,
)

private fun readNamedNumber(item: Cbor): NamedNumber {
    val node = Node.of(item)

    return NamedNumber(node.text(0), readConst(node.required(1)))
}

/** A type that is built in, but for those that have a body of their own. */
internal enum class Builtin(
    val asn1Name: String,
) {
    ANY("ANY"),
    BIT_STRING("BIT STRING"),
    BMP("BMPString"),
    BOOLEAN("BOOLEAN"),
    CHARACTER_STRING("CHARACTER STRING"),
    DATE("DATE"),
    DATE_TIME("DATE-TIME"),
    EMBEDDED_PDV("EMBEDDED PDV"),
    EXTERNAL("EXTERNAL"),
    GENERAL("GeneralString"),
    GENERALIZED("GeneralizedTime"),
    GRAPHIC("GraphicString"),
    IA5("IA5String"),
    INTEGER("INTEGER"),
    ISO646("ISO646String"),
    NULL("NULL"),
    NUMERIC("NumericString"),
    OBJECT_IDENTIFIER("OBJECT IDENTIFIER"),
    OCTET_STRING("OCTET STRING"),
    OBJECT_DESCRIPTOR("ObjectDescriptor"),
    OPEN_TYPE("OpenType"),
    PRINTABLE("PrintableString"),
    REAL("REAL"),
    T61("T61String"),
    TELETEX("TeletexString"),
    TIME_OF_DAY("TIME-OF-DAY"),
    DURATION("DURATION"),
    UTC("UTCTime"),
    UTF8("UTF8String"),
    UNIVERSAL("UniversalString"),
    VIDEOTEX("VideotexString"),
    VISIBLE("VisibleString"),

    /** Any other name. */
    UNSUPPORTED(""),
    ;

    companion object {
        private val byName = entries.associateBy { it.asn1Name }

        fun fromName(name: String): Builtin = byName[name]?.takeIf { it != UNSUPPORTED } ?: UNSUPPORTED
    }
}

internal data class Member(
    val name: String,
    val type: Type,
    val optional: Boolean,
    val default: Const?,
)

/** A member of a SEQUENCE, a SET or a CHOICE. */
internal sealed class MemberItem {
    data class Of(
        val member: Member,
    ) : MemberItem()

    data object ExtensionMarker : MemberItem()

    /** Members added in a later version that are there all or not at all, as `[[ ... ]]`. */
    data class Group(
        val items: List<MemberItem>,
    ) : MemberItem()
}

private fun readMemberItem(item: Cbor): MemberItem {
    val node = Node.of(item)

    return when (node.kind) {
        MEMBER ->
            MemberItem.Of(
                Member(
                    node.text(0),
                    readType(node.required(1)),
                    node.boolean(2, false),
                    node.get(3)?.let(::readConst),
                ),
            )
        EXTENSION_MARKER -> MemberItem.ExtensionMarker
        EXTENSION_ADDITION_GROUP -> MemberItem.Group(mapList(node.required(0), ::readMemberItem))
        else -> badNodes("Expected a member.")
    }
}

internal sealed class EnumItem {
    data class Of(
        val value: NamedNumber,
    ) : EnumItem()

    data object ExtensionMarker : EnumItem()
}

/** The constraint of a component in WITH COMPONENTS, or of the elements in WITH COMPONENT. */
internal data class Constraint(
    val size: List<Element>? = null,
    val restrictedTo: List<Element>? = null,
    val permittedAlphabet: List<Element>? = null,
    val withComponents: List<ComponentItem>? = null,
    val withComponent: List<ElementItem>? = null,
    val containing: Type? = null,
)

internal data class ComponentConstraint(
    val name: String,
    /** `PRESENT` or `ABSENT`. */
    val presence: String?,
    val constraint: Constraint,
)

/**
 * An item of an effective WITH COMPONENTS: an alternative, with the
 * constraints of components of different names, or the extension marker of
 * one that is extensible.
 */
internal sealed class ComponentItem {
    data class Alternative(
        val components: List<ComponentConstraint>,
    ) : ComponentItem()

    data object ExtensionMarker : ComponentItem()
}

/**
 * An item of an effective WITH COMPONENT: an alternative, with the
 * constraints of every element, or the extension marker of one that is
 * extensible.
 */
internal sealed class ElementItem {
    data class Alternative(
        val constraint: Constraint,
    ) : ElementItem()

    data object ExtensionMarker : ElementItem()
}

private fun readConstraint(
    node: Node,
    first: Int,
): Constraint =
    Constraint(
        size = readElements(node.get(first)),
        restrictedTo = readElements(node.get(first + 1)),
        permittedAlphabet = readElements(node.get(first + 2)),
        withComponents = node.get(first + 3)?.let(::readComponents),
        withComponent = node.get(first + 4)?.let(::readElementsOf),
        containing = node.get(first + 5)?.let(::readType),
    )

private fun readElementsOf(item: Cbor): List<ElementItem> =
    list(item).map {
        val node = Node.of(it)

        when (node.kind) {
            ELEMENT_CONSTRAINT -> ElementItem.Alternative(readConstraint(node, 0))
            EXTENSION_MARKER -> ElementItem.ExtensionMarker
            else -> badNodes("Expected an alternative of WITH COMPONENT.")
        }
    }

private fun readComponents(item: Cbor): List<ComponentItem> =
    list(item).map { alternative ->
        if (kindOf(alternative) == BigInteger.ZERO) {
            ComponentItem.Alternative(
                list(alternative).map {
                    val node = Node.of(it)

                    if (node.kind != COMPONENT_CONSTRAINT) badNodes("Expected a component constraint.")

                    ComponentConstraint(node.text(0), node.optionalText(1), readConstraint(node, 2))
                },
            )
        } else {
            when (Node.of(alternative).kind) {
                EXTENSION_MARKER -> ComponentItem.ExtensionMarker
                else -> badNodes("Expected an alternative of WITH COMPONENTS.")
            }
        }
    }

/** A table constraint: `{ObjectSet}{@a, @b}`. */
internal data class Table(
    val objectSet: String,
    val componentIds: List<String>?,
)

private fun readTable(item: Cbor): Table {
    val node = Node.of(item)

    if (node.kind != TABLE_CONSTRAINT) badNodes("Expected a table constraint.")

    return Table(
        node.text(0),
        node.get(1)?.let { ids -> list(ids).map { (it as? Cbor.Text)?.value ?: badNodes("Expected a text string.") } },
    )
}

internal data class AnyDefinedByChoice(
    val value: Const,
    val type: Type,
)

/** What is special about a type. */
internal sealed class Body {
    data class Builtin(
        val builtin: org.boxtheta.asnkitrt.Builtin,
    ) : Body()

    /** A reference to a type by name. */
    data class Referenced(
        val name: String,
    ) : Body()

    data class Integer(
        val namedNumbers: List<NamedNumber>,
    ) : Body()

    data class BitString(
        val namedBits: List<NamedNumber>,
    ) : Body()

    data class Enumerated(
        val items: List<EnumItem>,
    ) : Body()

    data class Sequence(
        val items: List<MemberItem>,
    ) : Body()

    data class Set(
        val items: List<MemberItem>,
    ) : Body()

    data class Choice(
        val items: List<MemberItem>,
    ) : Body()

    data class SequenceOf(
        val element: Type,
        val elementName: String?,
    ) : Body()

    data class SetOf(
        val element: Type,
        val elementName: String?,
    ) : Body()

    data class AnyDefinedBy(
        val value: String,
        val choices: List<AnyDefinedByChoice>,
    ) : Body()
}

internal data class Type(
    val body: Body,
    val tag: Tag?,
    val size: List<Element>?,
    val restrictedTo: List<Element>?,
    val permittedAlphabet: List<Element>?,
    val withComponents: List<ComponentItem>?,
    val withComponent: List<ElementItem>?,
    val table: Table?,
    /** The module the type is found in, if it is not the one it is used in. */
    val moduleName: String?,
)

private fun readType(item: Cbor): Type {
    val node = Node.of(item)
    // The fields all types have come after their own.
    val (body, first) =
        when (node.kind) {
            BUILTIN_TYPE -> Body.Builtin(Builtin.fromName(node.text(0))) to 1
            REFERENCED_TYPE -> Body.Referenced(node.text(0)) to 1
            INTEGER_TYPE -> Body.Integer(mapList(node.get(0), ::readNamedNumber)) to 1
            BIT_STRING_TYPE -> Body.BitString(mapList(node.get(0), ::readNamedNumber)) to 1
            ENUMERATED_TYPE ->
                Body.Enumerated(
                    mapList(node.get(0)) {
                        when (Node.of(it).kind) {
                            NAMED_NUMBER -> EnumItem.Of(readNamedNumber(it))
                            EXTENSION_MARKER -> EnumItem.ExtensionMarker
                            else -> badNodes("Expected a value of an enumeration.")
                        }
                    },
                ) to 1
            SEQUENCE_TYPE -> Body.Sequence(mapList(node.get(0), ::readMemberItem)) to 1
            SET_TYPE -> Body.Set(mapList(node.get(0), ::readMemberItem)) to 1
            CHOICE_TYPE -> Body.Choice(mapList(node.get(0), ::readMemberItem)) to 1
            SEQUENCE_OF_TYPE, SET_OF_TYPE -> {
                val element = readType(node.required(0))
                val elementName = node.optionalText(1)

                (if (node.kind == SEQUENCE_OF_TYPE) Body.SequenceOf(element, elementName) else Body.SetOf(element, elementName)) to 2
            }
            ANY_DEFINED_BY_TYPE ->
                Body.AnyDefinedBy(
                    node.text(0),
                    mapList(node.get(1)) {
                        val choice = Node.of(it)

                        if (choice.kind != ANY_DEFINED_BY_CHOICE) badNodes("Expected a choice of an ANY DEFINED BY.")

                        AnyDefinedByChoice(readConst(choice.required(0)), readType(choice.required(1)))
                    },
                ) to 2
            else -> badNodes("Expected a type.")
        }

    return Type(
        body = body,
        tag = node.get(first)?.let(::readTag),
        size = readElements(node.get(first + 1)),
        restrictedTo = readElements(node.get(first + 2)),
        permittedAlphabet = readElements(node.get(first + 3)),
        withComponents = node.get(first + 4)?.let(::readComponents),
        withComponent = node.get(first + 5)?.let(::readElementsOf),
        table = node.get(first + 6)?.let(::readTable),
        // `parameters` and `actual_parameters` are never in a compiled
        // specification.
        moduleName = node.optionalText(first + 9),
    )
}

internal data class TypeAssignment(
    val name: String,
    val type: Type,
)

internal data class ObjectClassField(
    val name: String,
    val type: Type,
)

/** A token of the syntax of an object class. */
internal sealed class SyntaxToken {
    data class Token(
        val text: String,
    ) : SyntaxToken()

    data class Optional(
        val tokens: List<SyntaxToken>,
    ) : SyntaxToken()
}

private fun readSyntax(item: Cbor): SyntaxToken {
    if (item is Cbor.Text) return SyntaxToken.Token(item.value)

    val node = Node.of(item)

    if (node.kind != OPTIONAL_GROUP) badNodes("Expected a token of a syntax.")

    return SyntaxToken.Optional(mapList(node.required(0), ::readSyntax))
}

internal data class ObjectClass(
    val name: String,
    val fields: List<ObjectClassField>,
    val syntax: List<SyntaxToken>?,
)

internal data class Module(
    val name: String,
    val types: List<TypeAssignment>,
    val objectClasses: List<ObjectClass>,
)

/** A compiled specification. */
internal data class Spec(
    val modules: List<Module>,
) {
    companion object {
        /** Reads the nodes of an `.asni` file, as the container gives them. */
        fun fromNodes(nodes: Cbor): Spec {
            val top = Node.of(nodes)

            if (top.kind != COMPILED_SPECIFICATION) badNodes("Expected a compiled specification.")

            return Spec(
                mapList(top.required(0)) { item ->
                    val module = Node.of(item)

                    if (module.kind != COMPILED_MODULE) badNodes("Expected a compiled module.")

                    Module(
                        module.text(0),
                        mapList(module.get(1)) {
                            val assignment = Node.of(it)

                            if (assignment.kind != TYPE_ASSIGNMENT) badNodes("Expected a type assignment.")

                            TypeAssignment(assignment.text(0), readType(assignment.required(1)))
                        },
                        mapList(module.get(2)) {
                            val objectClass = Node.of(it)

                            if (objectClass.kind != OBJECT_CLASS_ASSIGNMENT) badNodes("Expected an object class.")

                            ObjectClass(
                                objectClass.text(0),
                                mapList(objectClass.get(1)) { field ->
                                    val fieldNode = Node.of(field)

                                    if (fieldNode.kind != OBJECT_CLASS_FIELD) badNodes("Expected a field of an object class.")

                                    ObjectClassField(fieldNode.text(0), readType(fieldNode.required(1)))
                                },
                                objectClass.get(2)?.let { tokens -> list(tokens).map(::readSyntax) },
                            )
                        },
                    )
                },
            )
        }
    }
}
