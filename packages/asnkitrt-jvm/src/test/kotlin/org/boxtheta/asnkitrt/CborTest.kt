// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.math.BigInteger

/** The reader of CBOR, as the runtime for Rust tests its own. */
class CborTest {
    private fun parse(hex: String): Cbor = CborReader.parse(hex(hex))

    private fun int(value: Long) = Cbor.Integer(BigInteger.valueOf(value))

    private fun reason(hex: String): AsniException.Reason = assertThrows<AsniException> { parse(hex) }.reason

    @Test
    fun integers() {
        assertEquals(int(0), parse("00"))
        assertEquals(int(23), parse("17"))
        assertEquals(int(24), parse("1818"))
        assertEquals(int(1000), parse("1903e8"))
        assertEquals(int(1_000_000), parse("1a000f4240"))
        assertEquals(Cbor.Integer(BigInteger("18446744073709551615")), parse("1bffffffffffffffff"))
        assertEquals(int(-1), parse("20"))
        assertEquals(int(-1000), parse("3903e7"))
        assertEquals(Cbor.Integer(BigInteger("-18446744073709551616")), parse("3bffffffffffffffff"))
        // In more bytes than they take.
        assertEquals(int(1), parse("1801"))
        assertEquals(int(1), parse("1b0000000000000001"))
    }

    @Test
    fun bignums() {
        // 2**64 and -2**64 - 1, from RFC 8949 appendix A.
        assertEquals(Cbor.Integer(BigInteger.ONE.shiftLeft(64)), parse("c249010000000000000000"))
        assertEquals(Cbor.Integer(BigInteger.ONE.shiftLeft(64).negate().subtract(BigInteger.ONE)), parse("c349010000000000000000"))
        // All ones: -(2**72 - 1) - 1 = -2**72.
        assertEquals(Cbor.Integer(BigInteger.ONE.shiftLeft(72).negate()), parse("c349ffffffffffffffffff"))
        assertEquals(int(10), parse("c24900000000000000000a"))
        assertEquals(int(-1), parse("c340"))
        assertEquals(int(0), parse("c240"))
        // A tag in more bytes than it takes.
        assertEquals(Cbor.Integer(BigInteger.ONE.shiftLeft(64)), parse("d80249010000000000000000"))
    }

    @Test
    fun floats() {
        assertEquals(Cbor.Float(0.0), parse("f90000"))
        assertEquals(Cbor.Float(1.0), parse("f93c00"))
        assertEquals(Cbor.Float(1.5), parse("f93e00"))
        assertEquals(Cbor.Float(65504.0), parse("f97bff"))
        assertEquals(Cbor.Float(5.960464477539063e-8), parse("f90001"))
        assertEquals(Cbor.Float(-4.0), parse("f9c400"))
        assertEquals(Cbor.Float(Double.POSITIVE_INFINITY), parse("f97c00"))
        assertEquals(Cbor.Float(Double.NEGATIVE_INFINITY), parse("f9fc00"))
        assertEquals(Cbor.Float(100000.0), parse("fa47c35000"))
        assertEquals(Cbor.Float(1.1), parse("fb3ff199999999999a"))
        assertTrue((parse("f97e00") as Cbor.Float).value.isNaN())
    }

    @Test
    fun simpleAndText() {
        assertEquals(Cbor.Bool(false), parse("f4"))
        assertEquals(Cbor.Bool(true), parse("f5"))
        assertEquals(Cbor.Null, parse("f6"))
        assertEquals(Cbor.Text(""), parse("60"))
        assertEquals(Cbor.Text("IETF"), parse("6449455446"))
        assertEquals(Cbor.Text("ü"), parse("62c3bc"))
    }

    @Test
    fun containers() {
        assertEquals(Cbor.Array(emptyList()), parse("80"))
        assertEquals(Cbor.Array(listOf(int(1), int(2), int(3))), parse("83010203"))

        val map = parse("a2616101616202") as Cbor.Map

        assertEquals(int(1), map["a"])
        assertEquals(int(2), map["b"])
        assertEquals(null, map["c"])
        // The last of a key that is there twice.
        assertEquals(int(2), (parse("a2616101616102") as Cbor.Map)["a"])
    }

    @Test
    fun firstItemAndItsEnd() {
        val data = hex("ff" + "6449455446" + "820102")

        assertEquals(Cbor.Text("IETF") to 6, CborReader.parseFirst(data, 1))
        assertEquals(Cbor.Array(listOf(int(1), int(2))) to 9, CborReader.parseFirst(data, 6))
    }

    @Test
    fun rejected() {
        val outOfData = AsniException.Reason.OUT_OF_DATA
        val unsupported = AsniException.Reason.UNSUPPORTED_CBOR

        assertEquals(outOfData, reason(""))
        assertEquals(outOfData, reason("18"))
        assertEquals(AsniException.Reason.TRAILING_DATA, reason("6161ff"))
        assertEquals(outOfData, reason("62"))
        assertEquals(outOfData, reason("8201"))
        // A count that is more than the file is long, without memory being asked for.
        assertEquals(outOfData, reason("9bffffffffffffffff"))
        assertEquals(outOfData, reason("9b000000007fffffff"))
        assertEquals(outOfData, reason("7bffffffffffffffff"))
        assertEquals(outOfData, reason("a2616101"))
        // Indefinite lengths, byte strings, other tags, other simple values,
        // and a map with a key that is not text.
        assertEquals(unsupported, reason("9f01ff"))
        assertEquals(unsupported, reason("bf616101ff"))
        assertEquals(unsupported, reason("7f6161ff"))
        assertEquals(unsupported, reason("5f"))
        assertEquals(unsupported, reason("4101"))
        assertEquals(unsupported, reason("c101"))
        assertEquals(unsupported, reason("c26161"))
        assertEquals(unsupported, reason("c180"))
        assertEquals(unsupported, reason("f7"))
        assertEquals(unsupported, reason("f0"))
        assertEquals(unsupported, reason("f820"))
        assertEquals(unsupported, reason("a10101"))
        // Text that is not UTF-8.
        assertEquals(AsniException.Reason.BAD_TEXT, reason("61ff"))
    }

    @Test
    fun depth() {
        val nested = hex("81".repeat(10) + "80")

        CborReader.parse(nested, 11)
        assertEquals(AsniException.Reason.TOO_DEEP, assertThrows<AsniException> { CborReader.parse(nested, 10) }.reason)

        // Deep enough to overflow the stack, if there were no limit.
        val deep = ByteArray(1_000_000) { 0x81.toByte() }

        assertEquals(AsniException.Reason.TOO_DEEP, assertThrows<AsniException> { CborReader.parse(deep) }.reason)
    }
}
