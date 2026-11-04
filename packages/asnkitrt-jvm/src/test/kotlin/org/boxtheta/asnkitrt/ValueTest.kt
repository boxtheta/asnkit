// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertNotEquals
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.math.BigInteger

class ValueTest {
    private fun oid(text: String) = Value.ObjectIdentifier.parse(text)

    private val largest = BigInteger.ONE.shiftLeft(127).subtract(BigInteger.ONE)

    @Test
    fun objectIdentifiersRoundTrip() {
        for (text in listOf("0.0", "0.39", "1.2", "2.999.3", "1.2.840.113549.1.1.11", "1.3.$largest", "2.${largest - BigInteger.valueOf(80)}")) {
            val parsed = oid(text)

            assertEquals(text, parsed.toString())
            assertEquals(parsed, Value.ObjectIdentifier.fromBytes(parsed.bytes))
            assertEquals(text.split('.').size, parsed.arcs.size)
        }

        assertArrayEquals(hex("2a864886f70d"), oid("1.2.840.113549").bytes)

        // The first subidentifier is of the first two arcs.
        for ((bytes, text) in listOf("00" to "0.0", "28" to "1.0", "50" to "2.0", "8100" to "2.48")) {
            assertEquals(text, Value.ObjectIdentifier.fromBytes(hex(bytes)).toString())
        }
    }

    @Test
    fun objectIdentifiersAreRejected() {
        for ((bytes, what) in listOf(
            "" to "Expected an OBJECT IDENTIFIER.",
            "2a86" to "Expected an OBJECT IDENTIFIER.",
            "8001" to "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible.",
            "2a8001" to "Expected the subidentifiers of an OBJECT IDENTIFIER in as few bytes as possible.",
            "82" + "ff".repeat(17) + "7f" to "An arc of an OBJECT IDENTIFIER is too large.",
        )) {
            assertEquals(what, assertThrows<DecodeException> { Value.ObjectIdentifier.fromBytes(hex(bytes)) }.message)
        }

        Value.ObjectIdentifier.fromBytes(hex("81" + "ff".repeat(17) + "7f"))

        for (text in listOf("", "1", "1.", "1..2", "a.b", "1.2.-3", "1.2.+3", "1.2.${largest + BigInteger.ONE}", "2.${largest - BigInteger.valueOf(79)}")) {
            assertThrows<EncodeException>(text) { oid(text) }
        }
    }

    @Test
    fun berObjectIdentifiersAreMadeMinimal() {
        val bytes = hex("2a808648")

        assertThrows<DecodeException> { Value.ObjectIdentifier.decode(bytes, 0, true) }
        assertEquals(oid("1.2.840"), Value.ObjectIdentifier.decode(bytes, 0, false))
        // Its offset is that of the data.
        assertEquals(10, assertThrows<DecodeException> { Value.ObjectIdentifier.decode(hex("2a86"), 10, false) }.offset)
    }

    @Test
    fun equality() {
        assertEquals(Value.Bytes(byteArrayOf(1, 2)), Value.Bytes(byteArrayOf(1, 2)))
        assertNotEquals(Value.Bits(byteArrayOf(-128), 1), Value.Bits(byteArrayOf(-128), 2))
        assertEquals(Value.Map("a" to Value.Integer(1), "b" to Value.Null), Value.Map("b" to Value.Null, "a" to Value.Integer(1)))
        assertEquals(Value.Integer(5), Value.Integer(BigInteger.valueOf(5)))
        assertEquals(BigInteger.ZERO, integerOf(ByteArray(0)))
        assertEquals(BigInteger.valueOf(-5), integerOf(hex("fffffb")))
    }

    @Test
    fun cleanBits() {
        assertEquals("80" to 1, cleanBits(hex("ff"), 1, false).let { it.first.toHex() to it.second })
        assertEquals("a0" to 3, cleanBits(hex("a000"), 16, true).let { it.first.toHex() to it.second })
        assertEquals("" to 0, cleanBits(hex("0000"), 16, true).let { it.first.toHex() to it.second })
    }
}
