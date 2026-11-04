// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigInteger

// The types of a compiled specification, lowered once to what the codecs
// go through: every type has an id, a reference is an id, the tags that
// are on the wire are worked out, and the constraints that the codecs look
// at are numbers.

/** A value or a range of values of the root of a constraint. A bound that is `null` is MIN or MAX. */
internal data class Part(
    val lower: BigInteger?,
    val upper: BigInteger?,
)

/** A range of sizes or values. A bound that is `null` is MIN or MAX. */
internal data class Range(
    val min: BigInteger?,
    val max: BigInteger?,
    /** There is an extension marker. */
    val extensible: Boolean,
)

/**
 * The constraints of a type that the codecs look at: those of the type
 * itself, or else those of the type it refers to.
 */
internal data class Constraints(
    val size: Range? = null,
    val range: Range? = null,
    /** The permitted alphabet, as code points in order. */
    val alphabet: IntArray? = null,
    /** The values and ranges of the size and of the range, of which those are the least and the greatest. */
    val sizeParts: List<Part>? = null,
    val rangeParts: List<Part>? = null,
) {
    fun isEmpty(): Boolean = size == null && range == null && alphabet == null && sizeParts == null && rangeParts == null

    /**
     * Those of a type with constraints of its own, this, that refers to a
     * type with `inherited`: each of its own takes the place of that of the
     * type it refers to, as the compiler has worked them out of both.
     */
    fun over(
        inherited: Constraints,
        alphabetGiven: Boolean,
    ): Constraints =
        Constraints(
            size = size ?: inherited.size,
            range = range ?: inherited.range,
            alphabet = if (alphabetGiven) alphabet else inherited.alphabet,
            sizeParts = sizeParts ?: inherited.sizeParts,
            rangeParts = rangeParts ?: inherited.rangeParts,
        )

    override fun equals(other: Any?): Boolean =
        other is Constraints &&
            size == other.size &&
            range == other.range &&
            alphabet.contentEquals(other.alphabet) &&
            sizeParts == other.sizeParts &&
            rangeParts == other.rangeParts

    override fun hashCode(): Int = listOf(size, range, alphabet?.contentHashCode(), sizeParts, rangeParts).hashCode()

    companion object {
        val NONE = Constraints()
    }
}

/** The constraints a value is checked against, of a component in WITH COMPONENTS or of the elements in WITH COMPONENT. */
internal class Checked(
    val size: Range?,
    val sizeParts: List<Part>?,
    val range: Range?,
    val rangeParts: List<Part>?,
    val alphabet: IntArray?,
    val inner: Inner?,
)

/** An inner type constraint (X.680 51.8), as the compiler has worked it out. */
internal sealed class Inner {
    /** WITH COMPONENTS: the values that satisfy any of the alternatives. */
    class Components(
        val alternatives: List<List<ComponentCheck>>,
    ) : Inner()

    /** WITH COMPONENT: the values of which each element satisfies the constraints of any of the alternatives, the same for all. */
    class Element(
        val alternatives: List<Checked>,
    ) : Inner()

    /** An extensible one, which is not checked. */
    data object Unchecked : Inner()
}

/** The constraint of a component in an alternative of WITH COMPONENTS. */
internal class ComponentCheck(
    val name: String,
    /** `true` if it has to be present, `false` if absent. */
    val presence: Boolean?,
    val constraint: Checked,
)

/** A tag as it is on the wire. */
internal data class WireTag(
    val tagClass: TagClass,
    val number: Long,
    val constructed: Boolean,
)

/** A tag as the canonical order of tags has it: its class, then its number (X.680 8.6). */
internal data class TagKey(
    val tagClass: TagClass,
    val number: Long,
) : Comparable<TagKey> {
    override fun compareTo(other: TagKey): Int = compareValuesBy(this, other, { it.tagClass }, { it.number })
}

internal enum class StrKind {
    UTF8,
    NUMERIC,
    PRINTABLE,
    IA5,
    VISIBLE,
    GENERAL,
    BMP,
    GRAPHIC,

    /** ObjectDescriptor: a GraphicString with the tag 7. */
    OBJECT_DESCRIPTOR,
    UNIVERSAL,
    TELETEX,
    VIDEOTEX,
}

internal class MemberInfo(
    val name: String,
    val type: Int,
    val optional: Boolean,
    /** The default as the compiler gives it, until it is made a value. */
    var defaultConst: Const?,
) {
    var default: Value? = null

    val hasDefault: Boolean get() = default != null
}

internal sealed class Addition {
    class Of(
        val member: MemberInfo,
    ) : Addition()

    class Group(
        val members: List<MemberInfo>,
    ) : Addition()
}

/** The members of a SEQUENCE, a SET or a CHOICE. */
internal class Members(
    val root: List<MemberInfo>,
    /** Not `null` if there is an extension marker. */
    val additions: List<Addition>?,
    /** How many of the root members are after the additions, after the second extension marker. */
    val rootAfterAdditions: Int,
) {
    /**
     * The root members in the canonical order of their tags (X.680 8.6), in
     * which BER writes the members of a SET. Empty for a SEQUENCE.
     */
    var order: List<Int> = emptyList()

    /** All members, in the order they are encoded in: the root, then the additions. */
    val all: List<MemberInfo> by lazy {
        root +
            additions.orEmpty().flatMap {
                when (it) {
                    is Addition.Of -> listOf(it.member)
                    is Addition.Group -> it.members
                }
            }
    }

    private val byName: Map<String, MemberInfo> by lazy {
        // The first of a name, as a search in order finds it.
        val names = LinkedHashMap<String, MemberInfo>()

        all.forEach { names.putIfAbsent(it.name, it) }
        names
    }

    fun named(name: String): MemberInfo? = byName[name]

    /** The additions, in the order they are encoded in. */
    val additionMembers: List<MemberInfo> get() = all.subList(root.size, all.size)
}

internal sealed class Kind {
    /** The type is another one, with a tag or constraints of its own. */
    class Ref(
        val target: Int,
    ) : Kind()

    data object Boolean : Kind()

    class Integer(
        val named: List<Pair<String, BigInteger>>,
    ) : Kind() {
        fun number(name: String): BigInteger? = named.firstOrNull { it.first == name }?.second
    }

    data object Real : Kind()

    data object Null : Kind()

    /** The named bits, of which only if there are any is looked at. */
    class BitString(
        val named: List<Pair<String, Long>>,
    ) : Kind()

    data object OctetString : Kind()

    data object ObjectIdentifier : Kind()

    class Enumerated(
        val root: List<Pair<String, Long>>,
        /** Not `null` if there is an extension marker. */
        val additions: List<Pair<String, Long>>?,
    ) : Kind() {
        val items: List<Pair<String, Long>> = root + additions.orEmpty()
    }

    class Sequence(
        val members: Members,
    ) : Kind()

    class Set(
        val members: Members,
    ) : Kind()

    class Choice(
        val members: Members,
    ) : Kind()

    /** EXTERNAL, a SEQUENCE with the tag 8. */
    class External(
        val members: Members,
    ) : Kind()

    class SequenceOf(
        val element: Int,
    ) : Kind()

    class SetOf(
        val element: Int,
    ) : Kind()

    class Str(
        val kind: StrKind,
    ) : Kind()

    data object Utc : Kind()

    data object Generalized : Kind()

    data object Date : Kind()

    data object TimeOfDay : Kind()

    data object DateTime : Kind()

    data object Duration : Kind()

    data object Any : Kind()

    class AnyDefinedBy(
        val value: String,
        var choices: List<Pair<Const, Int>>,
    ) : Kind()

    data object OpenType : Kind()

    /** The members of a SEQUENCE, a SET, an EXTERNAL or a CHOICE. */
    val membersOrNull: Members?
        get() =
            when (this) {
                is Sequence -> members
                is Set -> members
                is External -> members
                is Choice -> members
                else -> null
            }

    /** The members of a SEQUENCE, a SET or an EXTERNAL, of which a value is a map. */
    val mapMembers: Members?
        get() =
            when (this) {
                is Sequence -> members
                is Set -> members
                is External -> members
                else -> null
            }
}

internal class TypeNode {
    var kind: Kind = Kind.Null

    /** The tag of the type itself, if it has one. */
    var tag: Tag? = null

    /**
     * The constraints of the type or of the one it refers to. While the
     * types are lowered, until they are worked out with those of the type
     * it refers to, those of the type itself.
     */
    var constraints: Constraints = Constraints.NONE

    /**
     * If the type has a permitted alphabet of its own, also one that is no
     * permitted alphabet as it is extensible, which is then not taken from
     * the type it refers to.
     */
    var alphabetGiven: Boolean = false

    /** The type it ends in, past the references. */
    var concrete: Int = 0

    /** All of its tags on the wire, the outermost first. */
    var tags: List<WireTag> = emptyList()

    /** The inner type constraint of the type itself. */
    var inner: Inner? = null

    /** The type of which the inner type constraint is that of this one: itself, or one it refers to. */
    var effectiveInner: Int? = null

    /** For a type with no tag of its own (a CHOICE): the tags it can begin with, and if it can begin with anything (an ANY). */
    var beginsWith: List<TagKey> = emptyList()
    var beginsWithAny: Boolean = false

    /**
     * The tag the type is in the place of in the canonical order of tags:
     * its first tag or, for a CHOICE with no tag of its own, the least of
     * those of the alternatives of its root. `null` for a type that may
     * begin with anything, as an ANY, which is last.
     */
    var canonical: TagKey? = null
}

/** The codecs this runtime has. */
public enum class Codec {
    /** Basic Encoding Rules, X.690. */
    BER,

    /** Distinguished Encoding Rules, X.690. */
    DER,

    /** JSON Encoding Rules, X.697. */
    JER,
}

/** The types of a specification. */
internal class Types(
    val nodes: List<TypeNode>,
    /** Module name, and the names of its types with their ids. */
    val modules: List<Pair<String, List<Pair<String, Int>>>>,
    val numericEnums: Boolean,
    val codec: Codec,
    /** DER that decodes what BER allows. */
    val lenient: Boolean,
) {
    private val index = HashMap<String, HashMap<String, Int>>()

    /** The types with a name that is found once only, in all the modules. */
    private val unique = HashMap<String, Int?>()

    init {
        for ((module, types) in modules) {
            val names = index.getOrPut(module) { HashMap() }

            for ((name, id) in types) {
                names.putIfAbsent(name, id)
                // A name in two modules, or twice in one, is not unique.
                unique[name] = if (name in unique) null else id
            }
        }
    }

    /** The type with `name` in the module `module`. */
    fun find(
        module: String,
        name: String,
    ): Int? = index[module]?.get(name)

    /** The type with `name`, if there is one of it in all of the modules. */
    fun findUnique(name: String): Int? = unique[name]

    fun node(id: Int): TypeNode = nodes[id]

    fun constraints(id: Int): Constraints = nodes[id].constraints

    /** The kind of a type, past the references. */
    fun kind(id: Int): Kind = nodes[nodes[id].concrete].kind

    /** If a type begins with a tag of this class and number. A type that has none, as ANY, accepts any. */
    fun accepts(
        id: Int,
        tagClass: TagClass,
        number: Long,
    ): Boolean {
        val node = nodes[id]
        val first = node.tags.firstOrNull()

        return if (first != null) {
            first.tagClass == tagClass && first.number == number
        } else {
            node.beginsWithAny || TagKey(tagClass, number) in node.beginsWith
        }
    }

    /**
     * The type of the value of a member, `id`, that is an ANY DEFINED BY
     * with choices: that of the choice of the value of the member it names,
     * among `siblings`. `null` if it is not one, and [NO_CHOICE] if no
     * choice has that value.
     */
    fun definedBy(
        id: Int,
        siblings: Map<String, Value>?,
    ): Int? {
        val kind = kind(id) as? Kind.AnyDefinedBy ?: return null

        if (kind.choices.isEmpty()) return null

        val key = siblings?.get(kind.value) ?: return NO_CHOICE

        return kind.choices.firstOrNull { constMatches(it.first, key) }?.second ?: NO_CHOICE
    }

    companion object {
        const val NO_CHOICE: Int = -1
    }
}

/** If the value of the member an ANY DEFINED BY is defined by is that of a choice of its types. */
internal fun constMatches(
    key: Const,
    value: Value,
): Boolean =
    when {
        key is Const.Integer && value is Value.Integer -> key.value == value.value
        key is Const.Text && value is Value.Text -> key.value == value.value
        key is Const.Identifier && value is Value.Text -> key.name == value.value
        key is Const.ObjectIdentifier && value is Value.ObjectIdentifier -> key.value == value
        key is Const.Bool && value is Value.Boolean -> key.value == value.value
        else -> false
    }

/**
 * How deep types are lowered, each in the one before it. The types of a
 * file are as deep as its nodes, which the reader of CBOR limits, but a
 * field of an object class is lowered where it is used, and may hold
 * itself or another field, without end, as the compiler never makes it.
 */
private const val MAX_LOWERING_DEPTH = 128

/** Lowers the types of `spec`. */
internal fun lower(
    spec: Spec,
    codec: Codec,
    numericEnums: Boolean,
    lenient: Boolean,
): Types {
    val lowering = Lowering(spec, codec)
    val nodes = lowering.nodes

    // Every type of a module has its id before any is lowered, as types
    // refer to each other.
    for (module in spec.modules) {
        val ids = module.types.map {
            nodes.add(TypeNode())
            nodes.size - 1
        }
        val names = HashMap<String, Int>()

        // The first of a name is the one that is found.
        module.types.zip(ids).forEach { (assignment, id) -> names.putIfAbsent(assignment.name, id) }

        lowering.typeIndex.add(names)
        lowering.ids.add(ids)
    }

    spec.modules.forEachIndexed { moduleIndex, module ->
        module.types.forEachIndexed { typeIndex, assignment ->
            nodes[lowering.ids[moduleIndex][typeIndex]] = lowering.lowerNode(assignment.type, moduleIndex)
        }
    }

    val modules =
        spec.modules.zip(lowering.ids).map { (module, ids) ->
            module.name to module.types.zip(ids).map { (assignment, id) -> assignment.name to id }
        }
    val types = Types(nodes, modules, numericEnums, codec, lenient)

    finish(types)

    return types
}

private fun hasOwn(type: Type): Boolean = type.tag != null || type.size != null || type.restrictedTo != null || type.permittedAlphabet != null

/** If a type is the type it refers to, and nothing else. */
private fun isPlainReference(type: Type): Boolean = type.body is Body.Referenced && !hasOwn(type) && '&' !in type.body.name

private fun bound(
    bound: Bound,
    isLower: Boolean,
): BigInteger? =
    when (val value = bound.value) {
        is Const.Min, is Const.Max -> null
        is Const.Integer ->
            // An excluded bound of the range of an integer, or of a size, is
            // the next integer in.
            when {
                bound is Bound.Included -> value.value
                isLower -> value.value + BigInteger.ONE
                else -> value.value - BigInteger.ONE
            }
        is Const.Identifier -> badNodes("A value in a constraint is not resolved.")
        // A number that is not an integer, and anything else, is no bound.
        else -> null
    }

/** The values and the ranges of the root of a constraint, which are those before an extension marker. */
private fun partsOf(elements: List<Element>): List<Part> {
    val parts = ArrayList<Part>()

    for (element in elements) {
        parts.add(
            when (element) {
                is Element.ExtensionMarker -> break
                is Element.Range -> Part(bound(element.lower, true), bound(element.upper, false))
                is Element.Value -> bound(Bound.Included(element.value), true).let { Part(it, it) }
                is Element.Type -> Part(null, null)
            },
        )
    }

    return parts
}

/** The least and the greatest values of the root of a constraint, and if there is an extension marker (X.691 10.3.10). */
private fun rangeOf(elements: List<Element>): Range {
    val parts = partsOf(elements)
    val min = if (parts.any { it.lower == null }) null else parts.mapNotNull { it.lower }.minOrNull()
    val max = if (parts.any { it.upper == null }) null else parts.mapNotNull { it.upper }.maxOrNull()

    return Range(min, max, elements.any { it is Element.ExtensionMarker })
}

private fun isCharacter(code: Int): Boolean = code in 0..0x10ffff && code !in 0xd800..0xdfff

private fun alphabetOf(elements: List<Element>): IntArray? {
    val characters = ArrayList<Int>()

    for (element in elements) {
        val lower = ((element as? Element.Range)?.lower as? Bound.Included)?.value as? Const.Text
        val upper = ((element as? Element.Range)?.upper as? Bound.Included)?.value as? Const.Text

        when {
            element is Element.Value && element.value is Const.Text -> element.value.value.codePoints().forEach(characters::add)
            lower != null && upper != null -> {
                if (lower.value.isEmpty() || upper.value.isEmpty()) badNodes("Bad range of a permitted alphabet.")

                (lower.value.codePointAt(0)..upper.value.codePointAt(0)).filter(::isCharacter).forEach(characters::add)
            }
            // An extensible permitted alphabet is not PER-visible (X.691
            // 10.3.11), and an extensible constraint is not checked.
            else -> return null
        }
    }

    return characters.toIntArray().apply { sort() }
}

private class OwnRange(
    val range: Range?,
    val parts: List<Part>?,
)

private fun integerRange(elements: List<Element>?): OwnRange =
    if (elements == null) OwnRange(null, null) else OwnRange(rangeOf(elements), partsOf(elements))

private class Lowering(
    val spec: Spec,
    val codec: Codec,
) {
    val nodes = ArrayList<TypeNode>()

    /** Ids of the types of each module. */
    val ids = ArrayList<List<Int>>()

    /** The types of each module, by name. */
    val typeIndex = ArrayList<Map<String, Int>>()

    /** The index of each module by name, the first of a name being the one that is found. */
    private val moduleIndex = HashMap<String, Int>()

    /** How many types are being lowered, each in the one before it. */
    private var depth = 0

    init {
        spec.modules.forEachIndexed { index, module -> moduleIndex.putIfAbsent(module.name, index) }
    }

    /** The module a type is found in, if it says, else the one it is used in. */
    fun moduleOf(
        type: Type,
        module: Int,
    ): Int = type.moduleName?.let { moduleIndex[it] } ?: module

    fun lookup(
        name: String,
        module: Int,
        moduleName: String?,
    ): Int {
        val found = moduleName?.let { moduleIndex[it] } ?: module

        return typeIndex[found][name] ?: badNodes("A type is not found.")
    }

    fun lowerMembers(
        items: List<MemberItem>,
        module: Int,
    ): Members {
        val root = ArrayList<MemberInfo>()
        var additions: ArrayList<Addition>? = null
        var inExtension = false
        var rootAfterAdditions = 0

        for (item in items) {
            when (item) {
                is MemberItem.ExtensionMarker -> {
                    inExtension = !inExtension

                    if (inExtension && additions == null) additions = ArrayList()
                }
                is MemberItem.Of ->
                    if (inExtension) {
                        additions!!.add(Addition.Of(lowerMember(item.member, module)))
                    } else {
                        root.add(lowerMember(item.member, module))

                        if (additions != null) rootAfterAdditions += 1
                    }
                is MemberItem.Group -> {
                    val group =
                        item.items.map {
                            (it as? MemberItem.Of)?.let { member -> lowerMember(member.member, module) }
                                ?: badNodes("Expected a member in a group.")
                        }

                    (additions ?: badNodes("A group of additions is not after a marker.")).add(Addition.Group(group))
                }
            }
        }

        return Members(root, additions, rootAfterAdditions)
    }

    fun lowerMember(
        member: Member,
        module: Int,
    ): MemberInfo = MemberInfo(member.name, lowerType(member.type, module), member.optional, member.default)

    /** The id of a type that is found where it is used. A reference with nothing of its own is the type it refers to. */
    fun lowerType(
        type: Type,
        module: Int,
    ): Int {
        val found = moduleOf(type, module)

        if (isPlainReference(type)) return lookup((type.body as Body.Referenced).name, found, type.moduleName)

        val node = lowerNode(type, found)

        nodes.add(node)

        return nodes.size - 1
    }

    /** The type of a field of an object class, as `ALGORITHM.&id`, and the module of its class. */
    fun fieldType(
        name: String,
        module: Int,
        moduleName: String?,
    ): Pair<Type, Int> {
        val dot = name.indexOf('.')

        if (dot < 0) badNodes("Bad field.")

        val className = name.substring(0, dot)
        val fieldName = name.substring(dot + 1)
        val start = moduleName?.let { moduleIndex[it] } ?: module

        // The class is in the module the type is found in, or in one the
        // specification has.
        for (candidate in listOf(start) + spec.modules.indices) {
            val field =
                spec.modules[candidate]
                    .objectClasses
                    .firstOrNull { it.name == className }
                    ?.fields
                    ?.firstOrNull { it.name == fieldName }

            if (field != null) return field.type to candidate
        }

        badNodes("A field of an object class is not found.")
    }

    /** Lowers `type`, in the types being lowered, of which there may be [MAX_LOWERING_DEPTH]. */
    fun lowerNode(
        type: Type,
        module: Int,
    ): TypeNode {
        if (depth >= MAX_LOWERING_DEPTH) {
            badNodes("Types are nested too deeply, or fields of object classes hold each other.")
        }

        depth += 1

        try {
            return lowerNodeIn(type, module)
        } finally {
            depth -= 1
        }
    }

    private fun lowerNodeIn(
        type: Type,
        outer: Int,
    ): TypeNode {
        val module = moduleOf(type, outer)
        val node = TypeNode()
        val range = integerRange(type.restrictedTo)

        node.tag = type.tag
        // Its own until `settle` works them out with those of the type it
        // refers to.
        node.constraints =
            Constraints(
                size = type.size?.let(::rangeOf),
                range = range.range,
                alphabet = type.permittedAlphabet?.let(::alphabetOf),
                sizeParts = type.size?.let(::partsOf),
                rangeParts = range.parts,
            )
        node.alphabetGiven = type.permittedAlphabet != null
        node.inner = innerOf(type.withComponents, type.withComponent)
        node.kind =
            when (val body = type.body) {
                is Body.Builtin -> lowerBuiltin(body.builtin)
                is Body.Referenced ->
                    if ('&' in body.name) {
                        // A field of an object class is the type it is, of
                        // which the constraints are taken as those of any
                        // type it refers to. The types in the field are
                        // lowered first, and take the ids before its own.
                        val (field, classModule) = fieldType(body.name, module, type.moduleName)
                        val lowered = lowerNode(field.copy(tag = null), classModule)

                        nodes.add(lowered)
                        Kind.Ref(nodes.size - 1)
                    } else {
                        Kind.Ref(lookup(body.name, module, type.moduleName))
                    }
                is Body.Integer ->
                    Kind.Integer(body.namedNumbers.mapNotNull { (it.number as? Const.Integer)?.let { number -> it.name to number.value } })
                is Body.BitString ->
                    Kind.BitString(
                        body.namedBits.mapNotNull {
                            val number = (it.number as? Const.Integer)?.value

                            if (number == null || number.signum() < 0 || number.bitLength() > 63) null else it.name to number.toLong()
                        },
                    )
                is Body.Enumerated -> {
                    val root = ArrayList<Pair<String, Long>>()
                    var additions: ArrayList<Pair<String, Long>>? = null

                    for (item in body.items) {
                        when (item) {
                            is EnumItem.Of -> {
                                val number = (item.value.number as? Const.Integer)?.value

                                if (number == null || number.bitLength() > 63) badNodes("Bad number of an enumerated value.")

                                (additions ?: root).add(item.value.name to number.toLong())
                            }
                            is EnumItem.ExtensionMarker -> if (additions == null) additions = ArrayList()
                        }
                    }

                    Kind.Enumerated(root, additions)
                }
                is Body.Sequence -> Kind.Sequence(lowerMembers(body.items, module))
                is Body.Set -> Kind.Set(lowerMembers(body.items, module))
                is Body.Choice -> Kind.Choice(lowerMembers(body.items, module))
                is Body.SequenceOf -> Kind.SequenceOf(lowerType(body.element, module))
                is Body.SetOf -> Kind.SetOf(lowerType(body.element, module))
                is Body.AnyDefinedBy -> Kind.AnyDefinedBy(body.value, body.choices.map { it.value to lowerType(it.type, module) })
            }

        return node
    }

    /**
     * An open type, as the value of a field of an object class that is a
     * type: in BER and DER the encoding of the value as it is, as an ANY.
     * JER needs the type to encode it with (X.697 41), and so does not
     * support it.
     */
    private fun openType(): Kind =
        when (codec) {
            Codec.BER, Codec.DER -> Kind.Any
            else -> Kind.OpenType
        }

    private fun add(
        kind: Kind,
        tag: Pair<Long, TagMode>?,
    ): Int {
        val node = TypeNode()

        node.kind = kind
        node.tag = tag?.let { (number, mode) -> Tag(number, TagClass.CONTEXT_SPECIFIC, mode) }
        nodes.add(node)

        return nodes.size - 1
    }

    /** The members of EXTERNAL, as X.690 8.18.1 says them, in an environment of EXPLICIT TAGS. */
    private fun external(): Members {
        val direct = add(Kind.ObjectIdentifier, null)
        val indirect = add(Kind.Integer(emptyList()), null)
        val descriptor = add(Kind.Str(StrKind.OBJECT_DESCRIPTOR), null)
        // [0] ABSTRACT-SYNTAX.&Type, an open type.
        val single = add(openType(), 0L to TagMode.EXPLICIT)
        val octets = add(Kind.OctetString, 1L to TagMode.IMPLICIT)
        val arbitrary = add(Kind.BitString(emptyList()), 2L to TagMode.IMPLICIT)
        val encoding =
            add(
                Kind.Choice(
                    Members(
                        listOf(
                            MemberInfo("single-ASN1-type", single, false, null),
                            MemberInfo("octet-aligned", octets, false, null),
                            MemberInfo("arbitrary", arbitrary, false, null),
                        ),
                        null,
                        0,
                    ),
                ),
                null,
            )

        return Members(
            listOf(
                MemberInfo("direct-reference", direct, true, null),
                MemberInfo("indirect-reference", indirect, true, null),
                MemberInfo("data-value-descriptor", descriptor, true, null),
                MemberInfo("encoding", encoding, false, null),
            ),
            null,
            0,
        )
    }

    private fun lowerBuiltin(builtin: Builtin): Kind =
        when (builtin) {
            Builtin.ANY -> Kind.Any
            Builtin.BIT_STRING -> Kind.BitString(emptyList())
            Builtin.BMP -> Kind.Str(StrKind.BMP)
            Builtin.BOOLEAN -> Kind.Boolean
            Builtin.DATE -> Kind.Date
            Builtin.DATE_TIME -> Kind.DateTime
            Builtin.GENERAL -> Kind.Str(StrKind.GENERAL)
            Builtin.GENERALIZED -> Kind.Generalized
            Builtin.GRAPHIC -> Kind.Str(StrKind.GRAPHIC)
            Builtin.OBJECT_DESCRIPTOR -> Kind.Str(StrKind.OBJECT_DESCRIPTOR)
            Builtin.IA5, Builtin.ISO646 -> Kind.Str(StrKind.IA5)
            Builtin.INTEGER -> Kind.Integer(emptyList())
            Builtin.NULL -> Kind.Null
            Builtin.NUMERIC -> Kind.Str(StrKind.NUMERIC)
            Builtin.OBJECT_IDENTIFIER -> Kind.ObjectIdentifier
            Builtin.OCTET_STRING -> Kind.OctetString
            Builtin.OPEN_TYPE -> openType()
            Builtin.PRINTABLE -> Kind.Str(StrKind.PRINTABLE)
            Builtin.REAL -> Kind.Real
            Builtin.T61, Builtin.TELETEX -> Kind.Str(StrKind.TELETEX)
            Builtin.VIDEOTEX -> Kind.Str(StrKind.VIDEOTEX)
            Builtin.TIME_OF_DAY -> Kind.TimeOfDay
            Builtin.DURATION -> Kind.Duration
            Builtin.UTC -> Kind.Utc
            Builtin.UTF8 -> Kind.Str(StrKind.UTF8)
            Builtin.UNIVERSAL -> Kind.Str(StrKind.UNIVERSAL)
            Builtin.VISIBLE -> Kind.Str(StrKind.VISIBLE)
            Builtin.EXTERNAL -> Kind.External(external())
            Builtin.CHARACTER_STRING, Builtin.EMBEDDED_PDV, Builtin.UNSUPPORTED -> badNodes("A type is not supported.")
        }
}

/** The constraints of a component or of the elements, to check values against. */
private fun checkedOf(constraint: Constraint): Checked {
    val range = integerRange(constraint.restrictedTo)

    return Checked(
        size = constraint.size?.let(::rangeOf),
        sizeParts = constraint.size?.let(::partsOf),
        range = range.range,
        rangeParts = range.parts,
        alphabet = constraint.permittedAlphabet?.let(::alphabetOf),
        inner = innerOf(constraint.withComponents, constraint.withComponent),
    )
}

/** The inner type constraint of a type, a component or the elements. */
private fun innerOf(
    components: List<ComponentItem>?,
    element: List<ElementItem>?,
): Inner? {
    if (element != null) {
        val alternatives = ArrayList<Checked>()

        for (item in element) {
            when (item) {
                is ElementItem.Alternative -> alternatives.add(checkedOf(item.constraint))
                is ElementItem.ExtensionMarker -> return Inner.Unchecked
            }
        }

        return Inner.Element(alternatives)
    }

    if (components == null) return null

    val alternatives = ArrayList<List<ComponentCheck>>()

    for (item in components) {
        when (item) {
            is ComponentItem.Alternative ->
                alternatives.add(
                    item.components.map {
                        val presence =
                            when (it.presence) {
                                "PRESENT" -> true
                                "ABSENT" -> false
                                else -> null
                            }

                        ComponentCheck(it.name, presence, checkedOf(it.constraint))
                    },
                )
            is ComponentItem.ExtensionMarker -> return Inner.Unchecked
        }
    }

    return Inner.Components(alternatives)
}

/** The universal tag number of a kind, and if it is constructed, if it has one. */
private fun universalTag(kind: Kind): Pair<Long, Boolean>? =
    when (kind) {
        is Kind.Boolean -> 1L to false
        is Kind.Integer -> 2L to false
        is Kind.BitString -> 3L to false
        is Kind.OctetString -> 4L to false
        is Kind.Null -> 5L to false
        is Kind.ObjectIdentifier -> 6L to false
        is Kind.Real -> 9L to false
        is Kind.Enumerated -> 10L to false
        is Kind.Sequence, is Kind.SequenceOf -> 16L to true
        is Kind.External -> 8L to true
        is Kind.Set, is Kind.SetOf -> 17L to true
        is Kind.Str ->
            when (kind.kind) {
                StrKind.OBJECT_DESCRIPTOR -> 7L
                StrKind.UTF8 -> 12L
                StrKind.NUMERIC -> 18L
                StrKind.PRINTABLE -> 19L
                StrKind.TELETEX -> 20L
                StrKind.VIDEOTEX -> 21L
                StrKind.IA5 -> 22L
                StrKind.GRAPHIC -> 25L
                StrKind.VISIBLE -> 26L
                StrKind.GENERAL -> 27L
                StrKind.UNIVERSAL -> 28L
                StrKind.BMP -> 30L
            } to false
        is Kind.Utc -> 23L to false
        is Kind.Generalized -> 24L to false
        is Kind.Date -> 31L to false
        is Kind.TimeOfDay -> 32L to false
        is Kind.DateTime -> 33L to false
        is Kind.Duration -> 34L to false
        // A CHOICE and the types of any kind have no tag of their own.
        is Kind.Choice, is Kind.Any, is Kind.AnyDefinedBy, is Kind.OpenType, is Kind.Ref -> null
    }

/** Works out what is left to do with the lowered nodes: the tags on the wire, the constraints of references and the defaults. */
private fun finish(types: Types) {
    settleAll(types)
    computeBeginnings(types)
    computeCanonical(types)
    lowerObjectIdentifierChoices(types)

    // Defaults go by the kind of their type.
    for (node in types.nodes) {
        node.kind.mapMembers?.let { fillDefaults(types, it) }
    }

    // The canonical order of the members of a SET.
    for (node in types.nodes) {
        val kind = node.kind

        if (kind is Kind.Set) kind.members.order = canonicalOrder(types, kind.members.root)
    }
}

/**
 * The choices of an ANY DEFINED BY that is defined by an OBJECT IDENTIFIER
 * have the bytes of the OBJECT IDENTIFIER they are, to be compared with
 * those of the value of the member.
 */
private fun lowerObjectIdentifierChoices(types: Types) {
    for (node in types.nodes) {
        val members = node.kind.mapMembers ?: continue

        for (member in members.all) {
            val kind = types.kind(member.type) as? Kind.AnyDefinedBy ?: continue
            val definedByOid = members.all.any { it.name == kind.value && types.kind(it.type) is Kind.ObjectIdentifier }

            if (definedByOid) {
                kind.choices =
                    kind.choices.map { (key, type) ->
                        val oid =
                            (key as? Const.Text)?.let {
                                try {
                                    Value.ObjectIdentifier.parse(it.value)
                                } catch (_: EncodeException) {
                                    null
                                }
                            }

                        (if (oid != null) Const.ObjectIdentifier(oid) else key) to type
                    }
            }
        }
    }
}

/**
 * The indexes of `members` in the canonical order of their tags. A member
 * with no tag, as ANY, is after those with one. The order of members with
 * the same tag, which X.680 forbids, is kept.
 */
private fun canonicalOrder(
    types: Types,
    members: List<MemberInfo>,
): List<Int> =
    members.indices.sortedWith(
        compareBy<Int> { types.nodes[members[it].type].canonical == null }
            .thenBy { types.nodes[members[it].type].canonical?.tagClass }
            .thenBy { types.nodes[members[it].type].canonical?.number },
    )

private const val NEW = 0
private const val ON_THE_WAY = 1
private const val DONE = 2

/**
 * Works out the canonical tag of every type. That of an untagged CHOICE
 * depends on those of its root alternatives, which are worked out first,
 * with a stack of its own and not by recursion, so that a specification
 * can not use up the stack.
 */
private fun computeCanonical(types: Types) {
    val nodes = types.nodes
    val state = IntArray(nodes.size)

    for (start in nodes.indices) {
        if (state[start] == DONE) continue

        // A type, and if its alternatives are pushed already.
        val stack = ArrayDeque<Pair<Int, Boolean>>()

        stack.addLast(start to false)

        while (stack.isNotEmpty()) {
            val (id, expanded) = stack.removeLast()

            if (state[id] == DONE) continue

            val node = nodes[id]
            val first = node.tags.firstOrNull()

            if (first != null) {
                node.canonical = TagKey(first.tagClass, first.number)
                state[id] = DONE
                continue
            }

            val choice = nodes[node.concrete].kind as? Kind.Choice

            if (choice == null) {
                // An ANY or an open type: in no place.
                state[id] = DONE
                continue
            }

            if (!expanded) {
                if (state[id] == ON_THE_WAY) {
                    // A CHOICE in itself, untagged, which X.680 forbids.
                    state[id] = DONE
                    continue
                }

                state[id] = ON_THE_WAY
                stack.addLast(id to true)

                for (member in choice.members.root) {
                    if (state[member.type] == NEW) stack.addLast(member.type to false)
                }

                continue
            }

            node.canonical = choice.members.root.mapNotNull { nodes[it.type].canonical }.minOrNull()
            state[id] = DONE
        }
    }
}

/**
 * Works out each type that is a reference. It follows a reference to the
 * end of the chain, and then works back, so that a chain of a hundred
 * thousand references is no deeper than one.
 */
private fun settleAll(types: Types) {
    val nodes = types.nodes
    val state = IntArray(nodes.size)

    for (start in nodes.indices) {
        val path = ArrayList<Int>()
        var current = start

        while (true) {
            when (state[current]) {
                DONE -> break
                ON_THE_WAY -> badNodes("A type is defined by itself.")
            }

            state[current] = ON_THE_WAY
            path.add(current)
            current = (nodes[current].kind as? Kind.Ref)?.target ?: break
        }

        for (id in path.asReversed()) {
            settle(types, id)
            state[id] = DONE
        }
    }
}

/** Works out one type, of which the type it refers to is done. */
private fun settle(
    types: Types,
    id: Int,
) {
    val node = types.nodes[id]
    val kind = node.kind
    val concrete: Int
    val tags: MutableList<WireTag>
    val inherited: Constraints
    val inheritedInner: Int?

    if (kind is Kind.Ref) {
        val target = types.nodes[kind.target]

        concrete = target.concrete
        tags = target.tags.toMutableList()
        inherited = target.constraints
        inheritedInner = target.effectiveInner
    } else {
        concrete = id
        tags = universalTag(kind)?.let { (number, constructed) -> mutableListOf(WireTag(TagClass.UNIVERSAL, number, constructed)) } ?: mutableListOf()
        inherited = Constraints.NONE
        inheritedInner = null
    }

    node.tag?.let { tag ->
        if (tag.mode == TagMode.EXPLICIT) {
            tags.add(0, WireTag(tag.tagClass, tag.number, true))
        } else if (tags.isNotEmpty()) {
            // Implicit: it takes the place of the outermost tag.
            tags[0] = WireTag(tag.tagClass, tag.number, tags[0].constructed)
        }
    }

    // A type with no constraints of its own has those of the type it
    // refers to. An extensible permitted alphabet is none of its own, but
    // still takes the place of that of the type it refers to.
    val own = node.constraints

    node.constraints =
        when {
            own.isEmpty() && !node.alphabetGiven -> inherited
            inherited.isEmpty() -> own
            else -> own.over(inherited, node.alphabetGiven)
        }
    node.concrete = concrete
    node.tags = tags
    node.effectiveInner = if (node.inner != null) id else inheritedInner
}

/**
 * What a type with no tag of its own can begin with: a CHOICE, the tags of
 * all that is in it. Every one is looked at once, and the work there is
 * for all of them is limited.
 */
private fun computeBeginnings(types: Types) {
    val steps = 20_000_000
    val nodes = types.nodes
    var taken = 0
    // Which choice a type was last looked at for, plus one.
    val seen = IntArray(nodes.size)

    for (start in nodes.indices) {
        if (nodes[start].tags.isNotEmpty()) continue

        if (nodes[nodes[start].concrete].kind !is Kind.Choice) {
            // An ANY, or something else that has no tag: anything.
            nodes[start].beginsWithAny = true
            continue
        }

        val begins = sortedSetOf<TagKey>()
        var any = false
        val stack = ArrayDeque<Int>()

        stack.addLast(start)
        seen[start] = start + 1

        while (stack.isNotEmpty()) {
            val id = stack.removeLast()

            taken += 1

            if (taken > steps) badNodes("The specification is too complex.")

            val node = nodes[id]
            val first = node.tags.firstOrNull()

            if (first != null) {
                begins.add(TagKey(first.tagClass, first.number))
                continue
            }

            when (val kind = nodes[node.concrete].kind) {
                is Kind.Choice ->
                    for (member in kind.members.all) {
                        if (seen[member.type] != start + 1) {
                            seen[member.type] = start + 1
                            stack.addLast(member.type)
                        }
                    }
                else -> any = true
            }
        }

        nodes[start].beginsWith = begins.toList()
        nodes[start].beginsWithAny = any
    }
}

private fun fillDefaults(
    types: Types,
    members: Members,
) {
    for (member in members.all) {
        member.defaultConst?.let {
            member.default = defaultValue(types, member.type, it)
            member.defaultConst = null
        }
    }
}

private fun hexDigits(digits: String): ByteArray? {
    val padded = if (digits.length % 2 == 1) digits + "0" else digits

    return hexDecode(padded)
}

private fun bitsOf(bits: String): Value? {
    val bytes = ByteArray((bits.length + 7) / 8)

    bits.forEachIndexed { index, bit ->
        when (bit) {
            '1' -> bytes[index / 8] = (bytes[index / 8].toInt() or (0x80 ushr (index % 8))).toByte()
            '0' -> {}
            else -> return null
        }
    }

    return Value.Bits(bytes, bits.length)
}

/** What a `DEFAULT` is as a value of a type. */
private fun defaultValue(
    types: Types,
    type: Int,
    constant: Const,
): Value? {
    val kind = types.kind(type)

    return when {
        constant is Const.Null -> null
        kind is Kind.Boolean && constant is Const.Bool -> Value.Boolean(constant.value)
        kind is Kind.Real && constant is Const.Real -> Value.Real(constant.value)
        kind is Kind.Real && constant is Const.Integer -> Value.Real(constant.value.toDouble())
        kind is Kind.Real && constant is Const.PlusInfinity -> Value.Real(Double.POSITIVE_INFINITY)
        kind is Kind.Real && constant is Const.MinusInfinity -> Value.Real(Double.NEGATIVE_INFINITY)
        constant is Const.Integer -> Value.Integer(constant.value)
        kind is Kind.Enumerated && (constant is Const.Identifier || constant is Const.Text) -> {
            val name = (constant as? Const.Identifier)?.name ?: (constant as Const.Text).value

            if (types.numericEnums) {
                kind.items.firstOrNull { it.first == name }?.let { Value.Integer(it.second) }
            } else {
                Value.Text(name)
            }
        }
        kind is Kind.OctetString && constant is Const.Hex -> hexDigits(constant.digits)?.let { Value.Bytes(it) }
        kind is Kind.BitString && constant is Const.Binary -> bitsOf(constant.bits)
        kind is Kind.BitString && constant is Const.Hex -> hexDigits(constant.digits)?.let { Value.Bits(it, constant.digits.length * 4) }
        constant is Const.Bool -> Value.Boolean(constant.value)
        constant is Const.Real -> Value.Real(constant.value)
        kind is Kind.ObjectIdentifier && constant is Const.Text ->
            try {
                Value.ObjectIdentifier.parse(constant.value)
            } catch (_: EncodeException) {
                null
            }
        constant is Const.Text -> Value.Text(constant.value)
        constant is Const.Identifier -> Value.Text(constant.name)
        // The value of an alternative, a member or an item is one of its own type.
        kind is Kind.Choice && constant is Const.Named ->
            kind.members.named(constant.name)?.let { member ->
                defaultValue(types, member.type, constant.value)?.let { Value.Choice(constant.name, it) }
            }
        kind.mapMembers != null && constant is Const.Sequence -> {
            val members = kind.mapMembers!!
            val converted = LinkedHashMap<String, Value>()

            for ((name, value) in constant.members) {
                val member = members.named(name) ?: return null

                converted[name] = defaultValue(types, member.type, value) ?: return null
            }

            Value.Map(converted)
        }
        (kind is Kind.SequenceOf || kind is Kind.SetOf) && constant is Const.Items -> {
            val element = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element

            Value.List(constant.items.map { defaultValue(types, element, it) ?: return null })
        }
        else -> null
    }
}

/** A bit string with the bits past the length cleared, and, with named bits, the trailing zero bits dropped. */
internal fun cleanBits(
    bytes: ByteArray,
    bits: Int,
    named: Boolean,
): Pair<ByteArray, Int> {
    val whole = bits / 8
    val rest = bits % 8
    val data = bytes.copyOf(minOf(bytes.size, whole + if (rest != 0) 1 else 0))

    if (rest != 0 && whole < data.size) data[whole] = (data[whole].toInt() and (0xff ushr rest).inv()).toByte()

    if (!named) return data to bits

    var length = data.size

    while (length > 0 && data[length - 1].toInt() == 0) length -= 1

    if (length == 0) return ByteArray(0) to 0

    return data.copyOf(length) to 8 * length - Integer.numberOfTrailingZeros(data[length - 1].toInt() and 0xff)
}

/** If a value is the default of a member. */
internal fun isDefault(
    types: Types,
    member: MemberInfo,
    value: Value,
): Boolean {
    val default = member.default ?: return false
    val kind = types.kind(member.type)

    return when {
        kind is Kind.Null -> false
        kind is Kind.Integer && value is Value.Text -> kind.number(value.value)?.let { Value.Integer(it) == default } ?: false
        kind is Kind.BitString && value is Value.Bits && default is Value.Bits -> {
            val named = kind.named.isNotEmpty()
            val (a, aBits) = cleanBits(value.bytes, value.length, named)
            val (b, bBits) = cleanBits(default.bytes, default.length, named)

            value.bytes.size * 8 >= value.length && default.bytes.size * 8 >= default.length && aBits == bBits && a.contentEquals(b)
        }
        kind is Kind.Real && value is Value.Real && default is Value.Real -> value.value == default.value
        kind is Kind.Real && value is Value.Integer && default is Value.Real -> value.value.toDouble() == default.value
        else -> value == default
    }
}
