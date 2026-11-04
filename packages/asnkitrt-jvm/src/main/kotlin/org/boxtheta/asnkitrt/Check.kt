// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigInteger

// Checks a value against the constraints of its type: the ranges of sizes
// and numbers, the permitted alphabets of strings, and WITH COMPONENT and
// WITH COMPONENTS (X.680 51.8). A constraint with an extension marker is
// not checked.

private const val MAX_DEPTH = 128

/** If a value is in one of the values or ranges of the root of a constraint. */
private fun inRange(
    range: Range?,
    parts: List<Part>?,
    value: BigInteger,
): Boolean {
    // The extension marker means it is not known what is in range.
    if (range == null || range.extensible) return true

    fun within(
        lower: BigInteger?,
        upper: BigInteger?,
    ) = (lower == null || value >= lower) && (upper == null || value <= upper)

    return parts?.any { within(it.lower, it.upper) } ?: within(range.min, range.max)
}

private fun inRange(
    range: Range?,
    parts: List<Part>?,
    value: Int,
): Boolean = inRange(range, parts, BigInteger.valueOf(value.toLong()))

private fun inDefaultAlphabet(
    kind: StrKind,
    code: Int,
): Boolean =
    when (kind) {
        StrKind.NUMERIC -> code == 32 || code in 48..57
        StrKind.PRINTABLE -> code in 'a'.code..'z'.code || code in 'A'.code..'Z'.code || code in '0'.code..'9'.code || code.toChar() in " '()+,-./:=?" && code < 128
        StrKind.IA5 -> code < 128
        StrKind.BMP -> code < 0x10000 && code !in 0xd800..0xdfff
        StrKind.VISIBLE -> code in 32 until 127
        else -> true
    }

internal object Check {
    /** Checks `value` as the type `id`. */
    fun check(
        types: Types,
        id: Int,
        value: Value,
    ) = checkAt(types, id, value, 0)

    private fun checkAt(
        types: Types,
        id: Int,
        value: Value,
        depth: Int,
    ) {
        if (depth > MAX_DEPTH) constraintsError("The value is nested too deeply.")

        checkKind(types, id, value, depth)

        val node = types.node(id)
        val inner = node.effectiveInner?.let { types.node(it).inner } ?: return

        if (!satisfiesInner(types, node.concrete, inner, value, depth + 1)) {
            constraintsError(
                if (inner is Inner.Element) "Expected elements that satisfy WITH COMPONENT." else "Expected a value that satisfies WITH COMPONENTS.",
            )
        }
    }

    /** Checks `value` as the type `id` but for its inner type constraint. */
    private fun checkKind(
        types: Types,
        id: Int,
        value: Value,
        depth: Int,
    ) {
        val constraints = types.constraints(id)

        when (val kind = types.kind(id)) {
            is Kind.Integer -> {
                val number =
                    when (value) {
                        is Value.Integer -> value.value
                        is Value.Text -> kind.number(value.value) ?: encodeError("Expected an integer or a named number.")
                        else -> encodeError("Expected data of type int or str.")
                    }

                if (!inRange(constraints.range, constraints.rangeParts, number)) constraintsError("Expected an integer in the range.")
            }
            is Kind.Str -> if (value is Value.Text) checkString(kind.kind, value.value, constraints, true)
            // Its text is made only for a SIZE, of which it is the length.
            is Kind.ObjectIdentifier ->
                if (value is Value.ObjectIdentifier && constraints.size != null) {
                    checkString(StrKind.UTF8, value.toString(), constraints, false)
                }
            is Kind.OctetString ->
                if (value is Value.Bytes && !inRange(constraints.size, constraints.sizeParts, value.value.size)) {
                    constraintsError("Expected a number of bytes in the range.")
                }
            is Kind.BitString ->
                if (value is Value.Bits && !inRange(constraints.size, constraints.sizeParts, value.length)) {
                    constraintsError("Expected a number of bits in the range.")
                }
            is Kind.SequenceOf, is Kind.SetOf -> {
                if (value !is Value.List) return

                val element = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element

                if (!inRange(constraints.size, constraints.sizeParts, value.items.size)) constraintsError("Expected a list of a length in the range.")

                value.items.forEachIndexed { index, item -> inElement(index) { checkAt(types, element, item, depth + 1) } }
            }
            is Kind.Choice -> {
                if (value !is Value.Choice) return

                val member = kind.members.named(value.name)

                when {
                    member != null -> inMember(member.name) { checkAt(types, member.type, value.value, depth + 1) }
                    // An alternative that is not known is one of a later version.
                    kind.members.additions != null -> {}
                    else -> constraintsError("Expected the name of an alternative of the CHOICE.")
                }
            }
            else -> {
                val members = kind.mapMembers

                if (members != null && value is Value.Map) {
                    for (member in members.all) {
                        val found = value.members[member.name] ?: continue

                        inMember(member.name) { checkAt(types, member.type, found, depth + 1) }
                    }
                }
                // A value that is of another kind is the business of the codec.
            }
        }
    }

    private fun checkString(
        kind: StrKind,
        text: String,
        constraints: Constraints,
        hasAlphabet: Boolean,
    ) {
        if (!inRange(constraints.size, constraints.sizeParts, text.codePointCount(0, text.length))) {
            constraintsError("Expected a number of characters in the range.")
        }

        if (!hasAlphabet) return

        val alphabet = constraints.alphabet
        val allowed =
            if (alphabet != null) {
                text.codePoints().allMatch { java.util.Arrays.binarySearch(alphabet, it) >= 0 }
            } else {
                text.codePoints().allMatch { inDefaultAlphabet(kind, it) }
            }

        if (!allowed) constraintsError("Expected a character in the permitted alphabet.")
    }

    /** If `value` of the type `id` satisfies the constraints of a component or of the elements. */
    private fun satisfies(
        types: Types,
        id: Int,
        checked: Checked,
        value: Value,
        depth: Int,
    ): Boolean {
        if (depth > MAX_DEPTH) constraintsError("The value is nested too deeply.")

        val concrete = types.node(id).concrete
        val kind = types.node(concrete).kind

        fun size(size: Int) = inRange(checked.size, checked.sizeParts, size)

        val isIn =
            when {
                kind is Kind.Integer && value is Value.Integer -> inRange(checked.range, checked.rangeParts, value.value)
                kind is Kind.Integer && value is Value.Text -> kind.number(value.value)?.let { inRange(checked.range, checked.rangeParts, it) } ?: true
                kind is Kind.Str && value is Value.Text -> {
                    val alphabet = checked.alphabet

                    size(value.value.codePointCount(0, value.value.length)) &&
                        (alphabet == null || value.value.codePoints().allMatch { java.util.Arrays.binarySearch(alphabet, it) >= 0 })
                }
                kind is Kind.OctetString && value is Value.Bytes -> size(value.value.size)
                kind is Kind.BitString && value is Value.Bits -> size(value.length)
                (kind is Kind.SequenceOf || kind is Kind.SetOf) && value is Value.List -> size(value.items.size)
                // A value that is of another kind is the business of the codec.
                else -> true
            }

        if (!isIn) return false

        return checked.inner?.let { satisfiesInner(types, concrete, it, value, depth + 1) } ?: true
    }

    /**
     * If `value` of the type `concrete` satisfies an inner type constraint:
     * any of the alternatives of WITH COMPONENTS, or of WITH COMPONENT with
     * every element.
     */
    private fun satisfiesInner(
        types: Types,
        concrete: Int,
        inner: Inner,
        value: Value,
        depth: Int,
    ): Boolean {
        val kind = types.node(concrete).kind
        val element = (kind as? Kind.SequenceOf)?.element ?: (kind as? Kind.SetOf)?.element
        val members = kind.membersOrNull

        return when {
            inner is Inner.Element && element != null && value is Value.List ->
                inner.alternatives.any { checked -> value.items.all { satisfies(types, element, checked, it, depth) } }
            inner is Inner.Components && members != null && (value is Value.Map || value is Value.Choice) -> {
                fun component(name: String): Value? =
                    if (value is Value.Map) {
                        value.members[name]
                    } else {
                        (value as Value.Choice).takeIf { it.name == name }?.value
                    }

                inner.alternatives.any { alternative ->
                    alternative.all { constraint ->
                        val found = component(constraint.name)
                        val member = members.named(constraint.name)

                        (constraint.presence == null || constraint.presence == (found != null)) &&
                            (found == null || member == null || satisfies(types, member.type, constraint.constraint, found, depth))
                    }
                }
            }
            kind is Kind.Duration && value is Value.Text -> satisfiesDuration(inner, value.value)
            else -> true
        }
    }

    /** If a DURATION satisfies an inner type constraint, which is one of its DURATION-EQUIVALENT value (X.680 38.4.4.3). */
    private fun satisfiesDuration(
        inner: Inner,
        text: String,
    ): Boolean {
        // Not a duration, which is the business of the codec.
        val elements =
            try {
                durationEquivalent(text)
            } catch (_: EncodeException) {
                return true
            }
        val fields = elements.map { it.first to it.second }
        val fraction = elements.firstNotNullOfOrNull { it.third }?.let { listOf("number-of-digits" to it.length.toString(), "fractional-value" to it) }

        return satisfiesFields(inner, fields, fraction)
    }

    /** If an integer that is not negative, given by its digits, is in the range of a constraint. */
    private fun digitsInRange(
        checked: Checked,
        digits: String,
    ): Boolean = inRange(checked.range, checked.rangeParts, BigInteger(digits))

    /** If fields satisfy WITH COMPONENTS, with those of a fractional part that is a component of its own. */
    private fun satisfiesFields(
        inner: Inner,
        fields: List<Pair<String, String>>,
        fraction: List<Pair<String, String>>?,
    ): Boolean {
        if (inner !is Inner.Components) return true

        return inner.alternatives.any { alternative ->
            alternative.all { constraint ->
                val field = fields.firstOrNull { it.first == constraint.name }
                val isFraction = constraint.name == "fractional-part"
                val present = if (isFraction) fraction != null else field != null

                when {
                    constraint.presence != null && constraint.presence != present -> false
                    isFraction && fraction != null -> constraint.constraint.inner?.let { satisfiesFields(it, fraction, null) } ?: true
                    !isFraction && field != null -> digitsInRange(constraint.constraint, field.second)
                    else -> true
                }
            }
        }
    }
}
