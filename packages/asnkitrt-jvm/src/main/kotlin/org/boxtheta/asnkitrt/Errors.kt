// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

/**
 * A step from a value to one in it: the name of a type, of a member of a
 * SEQUENCE or a SET or of an alternative of a CHOICE, or the index of an
 * element of a SEQUENCE OF or a SET OF.
 */
public sealed class Step {
    public data class Name(public val name: String) : Step()

    public data class Index(public val index: Int) : Step()
}

/** What every error of this package is. */
public sealed class AsnkitException(message: String) : RuntimeException(message)

/**
 * What can go wrong reading an `.asni` file. It is the same for every
 * failed check, and tells nothing of the contents of the file.
 */
public class AsniException internal constructor(
    public val reason: Reason,
    detail: String? = null,
) : AsnkitException(detail ?: reason.message) {
    public enum class Reason(internal val message: String) {
        /** The CBOR ends before its item does. */
        OUT_OF_DATA("Out of CBOR data."),

        /** The CBOR is nested deeper than the limit. */
        TOO_DEEP("The CBOR is nested too deeply."),

        /** The CBOR has something this package does not read. */
        UNSUPPORTED_CBOR("Unsupported CBOR."),

        /** A CBOR text string is not UTF-8. */
        BAD_TEXT("Bad CBOR text string."),

        /** There is data after the nodes. */
        TRAILING_DATA("Unexpected data after the nodes."),

        /** Not an `.asni` file. */
        NOT_ASNI("Expected an .asni file."),

        /** A major version that is not known. */
        UNSUPPORTED_VERSION("Unsupported major version."),

        /** The CRC-32 in the header is not that of the rest of the file, which is so corrupt. */
        BAD_CHECKSUM("The CRC-32 of the file does not match: the file is corrupt."),

        /** The metadata is not what it has to be. */
        BAD_METADATA("Bad metadata."),

        /** The nodes are not what a compiled specification has. */
        BAD_NODES("Bad nodes."),
    }
}

/** A type that is not in the specification, or not in it once only. */
public class NoSuchTypeException internal constructor(name: String) :
    AsnkitException("The type $name is not found in the specification.")

/**
 * An error with a value, encoding, decoding or checking it, which tells
 * where in the value it is: its [location], as
 * `Message.tags[1]: Expected data of type str.`.
 */
public sealed class ValueException(internal val what: String) : AsnkitException(what) {
    private val steps = ArrayDeque<Step>()

    /** Where in the value the error is, from the outside in. Empty if it is not known. */
    public val location: List<Step> get() = steps.toList()

    /** The location as text, as `Message.payload.reading.flags` or `Message.tags[1]`. */
    public val path: String
        get() = buildString {
            steps.forEachIndexed { index, step ->
                when (step) {
                    is Step.Name -> if (index == 0) append(step.name) else append('.').append(step.name)
                    is Step.Index -> append('[').append(step.index).append(']')
                }
            }
        }

    override val message: String
        get() = if (steps.isEmpty()) what else "$path: $what"

    /** The same error, found in the value at `step` of the one around it. */
    internal fun within(step: Step): ValueException {
        steps.addFirst(step)

        return this
    }
}

/** A value that can not be encoded. */
public class EncodeException internal constructor(what: String) : ValueException(what)

/**
 * Data that can not be decoded, and the [offset] it was found at: in bytes
 * for BER and DER, and 0 for JER, of which the text is read whole first.
 */
public class DecodeException internal constructor(what: String, public val offset: Int) : ValueException(what)

/** A value that does not fulfil the constraints of its type. */
public class ConstraintsException internal constructor(what: String) : ValueException(what)

/**
 * Something this runtime does not do (yet), as a codec it does not have
 * or an ANY in JER, and where in the value, if it is about one.
 */
public class UnsupportedException internal constructor(what: String) : ValueException(what)

internal fun badNodes(what: String): Nothing = throw AsniException(AsniException.Reason.BAD_NODES, what)

internal fun encodeError(what: String): Nothing = throw EncodeException(what)

internal fun decodeError(what: String, offset: Int): Nothing = throw DecodeException(what, offset)

internal fun constraintsError(what: String): Nothing = throw ConstraintsException(what)

internal fun unsupported(what: String): Nothing = throw UnsupportedException(what)

/** Runs `block`, and locates an error in its value at `step`. */
internal inline fun <T> within(step: Step, block: () -> T): T =
    try {
        block()
    } catch (error: ValueException) {
        throw error.within(step)
    }

internal inline fun <T> inMember(name: String, block: () -> T): T = within(Step.Name(name), block)

internal inline fun <T> inElement(index: Int, block: () -> T): T = within(Step.Index(index), block)
