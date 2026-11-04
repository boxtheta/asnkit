// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import com.fasterxml.jackson.core.JsonParseException
import com.fasterxml.jackson.core.JsonToken
import com.fasterxml.jackson.core.StreamReadConstraints
import com.fasterxml.jackson.core.io.JsonEOFException
import com.fasterxml.jackson.dataformat.cbor.CBORFactory
import com.fasterxml.jackson.dataformat.cbor.CBORParser
import java.io.IOException
import java.math.BigInteger

/**
 * A CBOR item of the part of CBOR (RFC 8949) the files use: integers,
 * floating point numbers, text strings, booleans, null, arrays and maps
 * with text strings as keys, all of a known length. Bignums are the only
 * byte strings and the only tags.
 */
internal sealed class Cbor {
    data object Null : Cbor()

    data class Bool(val value: Boolean) : Cbor()

    /** An integer of any size, also a bignum. */
    data class Integer(val value: BigInteger) : Cbor()

    data class Float(val value: Double) : Cbor()

    data class Text(val value: String) : Cbor()

    data class Array(val items: List<Cbor>) : Cbor()

    /** The members in the order of the data, a key there more than once as often as it is. */
    data class Map(val members: List<Pair<String, Cbor>>) : Cbor() {
        /** The value of `key`: the last if it is there more than once, as the reader of Python does. */
        operator fun get(key: String): Cbor? = members.lastOrNull { it.first == key }?.second
    }

    /** The item as plain Kotlin values: maps, lists, strings, `BigInteger`s, doubles, booleans and null. */
    fun toPlain(): Any? =
        when (this) {
            Null -> null
            is Bool -> value
            is Integer -> value
            is Float -> value
            is Text -> value
            is Array -> items.map { it.toPlain() }
            is Map -> members.associate { (key, value) -> key to value.toPlain() }
        }
}

/**
 * Reads the CBOR of the files with Jackson, which takes more than the
 * files may have: every item is checked against its initial byte (RFC
 * 8949 3), so that nothing of indefinite length, no byte string but that
 * of a bignum, no other tag, no simple value but `false`, `true` and
 * `null`, and no key of a map that is not text is read.
 */
internal object CborReader {
    /** How deep a reader goes into arrays and maps by default. The nodes of the files of this repository go to 15. */
    const val DEFAULT_MAX_DEPTH: Int = 64

    private const val MAJOR_UNSIGNED = 0
    private const val MAJOR_NEGATIVE = 1
    private const val MAJOR_BYTES = 2
    private const val MAJOR_TEXT = 3
    private const val MAJOR_ARRAY = 4
    private const val MAJOR_MAP = 5
    private const val MAJOR_TAG = 6
    private const val MAJOR_SIMPLE = 7

    private val factory: CBORFactory =
        CBORFactory
            .builder()
            .streamReadConstraints(
                // The limits are those of the reader here, and what a file
                // may ask for is bounded by its size.
                StreamReadConstraints
                    .builder()
                    .maxNestingDepth(Int.MAX_VALUE)
                    .maxStringLength(Int.MAX_VALUE)
                    .maxNumberLength(Int.MAX_VALUE)
                    .build(),
            ).build()

    private fun fail(reason: AsniException.Reason): Nothing = throw AsniException(reason)

    /** Reads the item at `offset` of `data`, and returns it with the offset of what follows. */
    fun parseFirst(
        data: ByteArray,
        offset: Int,
        maxDepth: Int = DEFAULT_MAX_DEPTH,
    ): Pair<Cbor, Int> {
        if (offset >= data.size) fail(AsniException.Reason.OUT_OF_DATA)

        try {
            (factory.createParser(data, offset, data.size - offset) as CBORParser).use { parser ->
                val reader = Reader(data, parser)
                val item = reader.item(reader.next(), maxDepth)

                return item to parser.currentLocation().byteOffset.toInt()
            }
        } catch (error: JsonEOFException) {
            fail(AsniException.Reason.OUT_OF_DATA)
        } catch (error: JsonParseException) {
            fail(if ("UTF-8" in (error.originalMessage ?: "")) AsniException.Reason.BAD_TEXT else AsniException.Reason.UNSUPPORTED_CBOR)
        } catch (error: IOException) {
            fail(AsniException.Reason.UNSUPPORTED_CBOR)
        }
    }

    /** Reads `data` as one item and nothing else. */
    fun parse(
        data: ByteArray,
        maxDepth: Int = DEFAULT_MAX_DEPTH,
    ): Cbor {
        val (item, end) = parseFirst(data, 0, maxDepth)

        if (end != data.size) fail(AsniException.Reason.TRAILING_DATA)

        return item
    }

    /** The head of an item: its major type, its additional information, its argument and where what follows it is. */
    private class Head(val major: Int, val info: Int, val argument: Long, val end: Int)

    private class Reader(val data: ByteArray, val parser: CBORParser) {
        /** The head at `at`, of which the argument is not of an indefinite length nor reserved. */
        fun head(at: Int): Head {
            if (at >= data.size) fail(AsniException.Reason.OUT_OF_DATA)

            val initial = data[at].toInt() and 0xff
            val info = initial and 0x1f
            var argument = info.toLong()
            var end = at + 1

            when (info) {
                in 0..23 -> {}
                in 24..27 -> {
                    val size = 1 shl (info - 24)

                    if (size > data.size - end) fail(AsniException.Reason.OUT_OF_DATA)

                    argument = 0

                    repeat(size) { argument = (argument shl 8) or (data[end + it].toLong() and 0xff) }
                    end += size
                }
                // Reserved, and the indefinite lengths.
                else -> fail(AsniException.Reason.UNSUPPORTED_CBOR)
            }

            return Head(initial ushr 5, info, argument, end)
        }

        /**
         * The size of a string, or the count of items of an array or a map.
         * As no item is less than a byte, a count that is more than what is
         * left is out of data.
         */
        fun checkLength(head: Head, perItem: Int) {
            val left = (data.size - head.end).toLong()

            // An argument of 2**63 or more is negative here.
            if (head.argument < 0 || head.argument > left / perItem) fail(AsniException.Reason.OUT_OF_DATA)
        }

        fun tokenStart(): Int = parser.currentTokenLocation().byteOffset.toInt()

        /**
         * The token of the next item, of which the head is checked before
         * Jackson reads it, so that a length that is more than the data is out
         * of data however Jackson would have it.
         */
        fun next(): JsonToken {
            val head = head(parser.currentLocation().byteOffset.toInt())

            when (head.major) {
                MAJOR_BYTES, MAJOR_TEXT, MAJOR_ARRAY -> checkLength(head, 1)
                MAJOR_MAP -> checkLength(head, 2)
                MAJOR_TAG -> if (head.argument == 2L || head.argument == 3L) checkLength(head(head.end), 1)
            }

            return parser.nextToken() ?: fail(AsniException.Reason.OUT_OF_DATA)
        }

        /** The end of an array or a map, which Jackson gives after the count of its items. */
        fun end(expected: JsonToken) {
            if (parser.nextToken() != expected) fail(AsniException.Reason.UNSUPPORTED_CBOR)
        }

        fun item(
            token: JsonToken,
            depth: Int,
        ): Cbor {
            val head = head(tokenStart())

            return when (token) {
                JsonToken.START_ARRAY -> {
                    if (head.major != MAJOR_ARRAY) fail(AsniException.Reason.UNSUPPORTED_CBOR)
                    if (depth == 0) fail(AsniException.Reason.TOO_DEEP)

                    checkLength(head, 1)

                    val items = ArrayList<Cbor>(head.argument.toInt())

                    repeat(head.argument.toInt()) { items.add(item(next(), depth - 1)) }
                    end(JsonToken.END_ARRAY)

                    Cbor.Array(items)
                }
                JsonToken.START_OBJECT -> {
                    if (head.major != MAJOR_MAP) fail(AsniException.Reason.UNSUPPORTED_CBOR)
                    if (depth == 0) fail(AsniException.Reason.TOO_DEEP)

                    // A key and a value are two bytes at the least.
                    checkLength(head, 2)

                    val members = ArrayList<Pair<String, Cbor>>(head.argument.toInt())

                    repeat(head.argument.toInt()) {
                        // Jackson makes a name of a key of any kind.
                        if (next() != JsonToken.FIELD_NAME || head(tokenStart()).major != MAJOR_TEXT) {
                            fail(AsniException.Reason.UNSUPPORTED_CBOR)
                        }

                        val name = parser.currentName()

                        members.add(name to item(next(), depth - 1))
                    }
                    end(JsonToken.END_OBJECT)

                    Cbor.Map(members)
                }
                JsonToken.VALUE_STRING -> {
                    if (head.major != MAJOR_TEXT) fail(AsniException.Reason.UNSUPPORTED_CBOR)

                    checkLength(head, 1)

                    Cbor.Text(parser.text)
                }
                JsonToken.VALUE_NUMBER_INT ->
                    when (head.major) {
                        MAJOR_UNSIGNED, MAJOR_NEGATIVE -> Cbor.Integer(parser.bigIntegerValue)
                        MAJOR_TAG -> bignum(head)
                        else -> fail(AsniException.Reason.UNSUPPORTED_CBOR)
                    }
                JsonToken.VALUE_NUMBER_FLOAT -> {
                    if (head.major != MAJOR_SIMPLE || head.info !in 25..27) fail(AsniException.Reason.UNSUPPORTED_CBOR)

                    Cbor.Float(parser.doubleValue)
                }
                JsonToken.VALUE_FALSE, JsonToken.VALUE_TRUE, JsonToken.VALUE_NULL -> {
                    // Not `undefined`, which Jackson reads as null.
                    if (head.major != MAJOR_SIMPLE || head.info !in 20..22) fail(AsniException.Reason.UNSUPPORTED_CBOR)

                    when (head.info) {
                        20 -> Cbor.Bool(false)
                        21 -> Cbor.Bool(true)
                        else -> Cbor.Null
                    }
                }
                else -> fail(AsniException.Reason.UNSUPPORTED_CBOR)
            }
        }

        /** A bignum: the number of its bytes, or one less than minus it (RFC 8949 3.4.3). */
        fun bignum(tag: Head): Cbor {
            if (tag.argument != 2L && tag.argument != 3L) fail(AsniException.Reason.UNSUPPORTED_CBOR)

            val bytes = head(tag.end)

            if (bytes.major != MAJOR_BYTES) fail(AsniException.Reason.UNSUPPORTED_CBOR)

            checkLength(bytes, 1)

            // Jackson reads `c3 40` as 0, so the number is read from the bytes.
            val magnitude = BigInteger(1, data.copyOfRange(bytes.end, bytes.end + bytes.argument.toInt()))

            return Cbor.Integer(if (tag.argument == 3L) magnitude.negate().subtract(BigInteger.ONE) else magnitude)
        }
    }
}
