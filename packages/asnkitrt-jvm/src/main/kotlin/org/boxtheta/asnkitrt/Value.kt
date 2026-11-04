// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigInteger
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.time.OffsetDateTime
import java.time.ZoneOffset

/**
 * A value of an ASN.1 type, as the runtimes for Python and Rust have it.
 *
 * | ASN.1 | Value |
 * |---|---|
 * | BOOLEAN | [Boolean] |
 * | INTEGER | [Integer], or the name of a named number as [Text] when encoding |
 * | REAL | [Real] |
 * | NULL | [Null] |
 * | BIT STRING | [Bits] |
 * | OCTET STRING | [Bytes] |
 * | OBJECT IDENTIFIER | [ObjectIdentifier] |
 * | ENUMERATED | [Text] with the name, or [Integer] with `numericEnums` |
 * | SEQUENCE, SET and EXTERNAL | [Map] of the members that are there |
 * | SEQUENCE OF and SET OF | [List] |
 * | CHOICE | [Choice] |
 * | character strings | [Text] |
 * | UTCTime, GeneralizedTime and DATE-TIME | [DateTime] |
 * | DATE | [Date] |
 * | TIME-OF-DAY | [Time] |
 * | DURATION | [Text], as an ISO 8601 duration |
 * | ANY and open types | [Bytes] of the encoding |
 */
public sealed class Value {
    /** NULL. Also what an unknown value of an extensible ENUMERATED decodes to. */
    public data object Null : Value()

    public data class Boolean(
        public val value: kotlin.Boolean,
    ) : Value()

    /** An INTEGER of any size. */
    public data class Integer(
        public val value: BigInteger,
    ) : Value() {
        public constructor(value: Long) : this(BigInteger.valueOf(value))

        override fun toString(): String = "Integer($value)"
    }

    /** A REAL, as a float of 64 bits. Two are equal as `Double.equals` has them: `NaN` is `NaN`, and `-0.0` is not `0.0`. */
    public data class Real(
        public val value: Double,
    ) : Value()

    /** A character string, a DURATION and an ENUMERATED by name. */
    public data class Text(
        public val value: String,
    ) : Value()

    /** An OCTET STRING, or the encoding of the value of an ANY or an open type. */
    public class Bytes(
        public val value: ByteArray,
    ) : Value() {
        override fun equals(other: Any?): kotlin.Boolean = other is Bytes && value.contentEquals(other.value)

        override fun hashCode(): Int = value.contentHashCode()

        override fun toString(): String = "Bytes(${value.toHex()})"
    }

    /** A BIT STRING: its bytes, the first bit the most significant of the first byte, and how many bits of them there are. */
    public class Bits(
        public val bytes: ByteArray,
        public val length: Int,
    ) : Value() {
        override fun equals(other: Any?): kotlin.Boolean = other is Bits && length == other.length && bytes.contentEquals(other.bytes)

        override fun hashCode(): Int = 31 * bytes.contentHashCode() + length

        override fun toString(): String = "Bits(${bytes.toHex()}, $length)"
    }

    /** A SEQUENCE OF or a SET OF. */
    public data class List(
        public val items: kotlin.collections.List<Value>,
    ) : Value() {
        public constructor(vararg items: Value) : this(items.toList())
    }

    /** A SEQUENCE, a SET or an EXTERNAL: its members that are there, by name. A decoder gives them in the order they are declared in. */
    public data class Map(
        public val members: kotlin.collections.Map<String, Value>,
    ) : Value() {
        public constructor(vararg members: Pair<String, Value>) : this(linkedMapOf(*members))

        public operator fun get(name: String): Value? = members[name]
    }

    /**
     * A CHOICE: the name of the alternative and its value. An alternative
     * of a later version of an extensible CHOICE decodes to the name `""`
     * and [Null].
     */
    public data class Choice(
        public val name: String,
        public val value: Value,
    ) : Value()

    /**
     * A date and a time, held to the microsecond, and its offset in
     * minutes east of UTC, or `null` for a local time.
     */
    public data class DateTime(
        public val dateTime: LocalDateTime,
        public val offsetMinutes: Int? = null,
    ) : Value() {
        public constructor(dateTime: OffsetDateTime) :
            this(dateTime.toLocalDateTime(), dateTime.offset.totalSeconds / 60)

        /** The same instant with its offset, or `null` for a local time. */
        public fun toOffsetDateTime(): OffsetDateTime? =
            offsetMinutes?.let { dateTime.atOffset(ZoneOffset.ofTotalSeconds(it * 60)) }
    }

    public data class Date(
        public val date: LocalDate,
    ) : Value()

    /** A time of day, held to the microsecond, and its offset in minutes east of UTC, or `null` for a local time. */
    public data class Time(
        public val time: LocalTime,
        public val offsetMinutes: Int? = null,
    ) : Value()

    /**
     * An OBJECT IDENTIFIER, as the contents octets that BER and DER encode
     * it as (X.690 8.19): its subidentifiers, each in as few bytes as it can
     * be, of which the first is that of the first two arcs. Its text, as
     * `1.2.840.113549`, is made when it is asked for, by [toString], and
     * read by [parse]. Two are equal if and only if their bytes are.
     */
    public class ObjectIdentifier private constructor(
        private val octets: ByteArray,
    ) : Value() {
        /** The contents octets, a copy. */
        public val bytes: ByteArray get() = octets.copyOf()

        internal val bytesUnsafe: ByteArray get() = octets

        /** The arcs, of which there are at least two. */
        public val arcs: kotlin.collections.List<BigInteger>
            get() {
                val arcs = ArrayList<BigInteger>()
                var arc = BigInteger.ZERO

                for (byte in octets) {
                    arc = arc.shiftLeft(7).or(BigInteger.valueOf((byte.toLong() and 0x7f)))

                    if (byte.toInt() and 0x80 != 0) continue

                    if (arcs.isEmpty()) {
                        // The first subidentifier is of the first two arcs (X.690 8.19.4).
                        val first = if (arc < FORTY) 0L else if (arc < EIGHTY) 1L else 2L

                        arcs.add(BigInteger.valueOf(first))
                        arcs.add(arc.subtract(BigInteger.valueOf(40 * first)))
                    } else {
                        arcs.add(arc)
                    }

                    arc = BigInteger.ZERO
                }

                return arcs
            }

        override fun equals(other: Any?): kotlin.Boolean = other is ObjectIdentifier && octets.contentEquals(other.octets)

        override fun hashCode(): Int = octets.contentHashCode()

        /** The arcs, as `1.2.840.113549`. */
        override fun toString(): String = arcs.joinToString(".")

        public companion object {
            private val FORTY = BigInteger.valueOf(40)
            private val EIGHTY = BigInteger.valueOf(80)

            /** A subidentifier is read back if it is of at most 127 bits. */
            private const val MAX_BITS = 127

            /** The OBJECT IDENTIFIER of its arcs, as `1.2.840.113549`. */
            @JvmStatic
            public fun parse(text: String): ObjectIdentifier {
                val parts = text.split('.')

                if (parts.size < 2) encodeError("Expected an OBJECT IDENTIFIER of at least two arcs.")

                val arcs =
                    parts.map { part ->
                        if (part.isEmpty() || !part.all { it in '0'..'9' }) encodeError("Expected an OBJECT IDENTIFIER.")

                        BigInteger(part)
                    }
                val out = ByteBuffer()

                fun push(subidentifier: BigInteger) {
                    if (subidentifier.bitLength() > MAX_BITS) encodeError("An arc of an OBJECT IDENTIFIER is too large.")

                    pushSubidentifier(subidentifier, out)
                }

                push(arcs[0].multiply(FORTY).add(arcs[1]))
                arcs.drop(2).forEach(::push)

                return ObjectIdentifier(out.toByteArray())
            }

            /**
             * The OBJECT IDENTIFIER of these contents octets, if they are those
             * of one: subidentifiers in as few bytes as they can be, of at most
             * 127 bits.
             */
            @JvmStatic
            public fun fromBytes(bytes: ByteArray): ObjectIdentifier {
                check(bytes, true)?.let { (what, index) -> decodeError(what, index) }

                return ObjectIdentifier(bytes.copyOf())
            }

            /**
             * The OBJECT IDENTIFIER of the contents octets of a decoder, found
             * at `offset`. BER allows a subidentifier in more bytes than it
             * needs, but for `strict`, and it is made the bytes it needs.
             */
            internal fun decode(
                bytes: ByteArray,
                offset: Int,
                strict: kotlin.Boolean,
            ): ObjectIdentifier {
                val problem = check(bytes, true) ?: return ObjectIdentifier(bytes)
                val (what, index) = problem

                if (strict || bytes.getOrNull(index) != 0x80.toByte()) decodeError(what, offset + index)

                check(bytes, false)?.let { (what, index) -> decodeError(what, offset + index) }

                // The bytes are made again, as few as they can be.
                val out = ByteBuffer()
                var arc = BigInteger.ZERO

                for (byte in bytes) {
                    arc = arc.shiftLeft(7).or(BigInteger.valueOf(byte.toLong() and 0x7f))

                    if (byte.toInt() and 0x80 == 0) {
                        pushSubidentifier(arc, out)
                        arc = BigInteger.ZERO
                    }
                }

                return ObjectIdentifier(out.toByteArray())
            }

            /** What is wrong with the contents octets of an OBJECT IDENTIFIER, and at which of them. */
            private fun check(
                bytes: ByteArray,
                minimal: kotlin.Boolean,
            ): Pair<String, Int>? {
                // The bits of the arc so far, as many as it would have.
                var bits = 0
                var start = true

                for ((index, byte) in bytes.withIndex()) {
                    val value = byte.toInt() and 0xff

                    // A subidentifier is in as few bytes as possible.
                    if (minimal && start && value == 0x80) {
                        return "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible." to index
                    }

                    if (bits > 120) return "An arc of an OBJECT IDENTIFIER is too large." to index

                    start = value and 0x80 == 0
                    bits =
                        when {
                            start -> 0
                            bits == 0 && value and 0x7f == 0 -> 0
                            bits == 0 -> 32 - java.lang.Integer.numberOfLeadingZeros(value and 0x7f)
                            else -> bits + 7
                        }
                }

                if (bytes.isEmpty() || !start) return "Expected an OBJECT IDENTIFIER." to 0

                return null
            }

            /** A subidentifier: groups of 7 bits, the most significant first, all but the last with the high bit set. */
            private fun pushSubidentifier(
                arc: BigInteger,
                out: ByteBuffer,
            ) {
                val groups = maxOf(1, (arc.bitLength() + 6) / 7)

                for (group in groups - 1 downTo 0) {
                    val bits = arc.shiftRight(7 * group).toInt() and 0x7f

                    out.add(if (group == 0) bits else 0x80 or bits)
                }
            }
        }
    }
}

/** An integer from its two's complement, most significant byte first. Nothing at all is 0. */
internal fun integerOf(bytes: ByteArray): BigInteger = if (bytes.isEmpty()) BigInteger.ZERO else BigInteger(bytes)

/** The bytes as lowercase hexadecimal digits. */
internal fun ByteArray.toHex(): String = joinToString("") { "%02x".format(it.toInt() and 0xff) }

/** Bytes to write to, which can also be put before what is written. */
internal class ByteBuffer(
    capacity: Int = 64,
) {
    private var bytes = ByteArray(capacity)

    var size: Int = 0
        private set

    private fun ensure(more: Int) {
        if (size + more > bytes.size) bytes = bytes.copyOf(maxOf(bytes.size * 2, size + more))
    }

    fun add(byte: Int) {
        ensure(1)
        bytes[size++] = byte.toByte()
    }

    fun addAll(
        data: ByteArray,
        from: Int = 0,
        to: Int = data.size,
    ) {
        ensure(to - from)
        System.arraycopy(data, from, bytes, size, to - from)
        size += to - from
    }

    /** Puts `data` at `at`, before what is there. */
    fun insert(
        at: Int,
        data: ByteArray,
    ) {
        ensure(data.size)
        System.arraycopy(bytes, at, bytes, at + data.size, size - at)
        System.arraycopy(data, 0, bytes, at, data.size)
        size += data.size
    }

    operator fun set(
        at: Int,
        byte: Int,
    ) {
        bytes[at] = byte.toByte()
    }

    fun truncate(length: Int) {
        size = length
    }

    fun copyRange(
        from: Int,
        to: Int,
    ): ByteArray = bytes.copyOfRange(from, to)

    fun toByteArray(): ByteArray = bytes.copyOf(size)
}
