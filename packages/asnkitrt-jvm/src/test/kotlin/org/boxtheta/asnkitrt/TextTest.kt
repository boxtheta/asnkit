// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.math.BigInteger

/** The numbers, hex and JSON of the text codecs. */
class TextTest {
    @Test
    fun floatsAsPythonWritesThem() {
        for ((value, text) in listOf(
            1.0 to "1.0",
            0.1 to "0.1",
            -2.5 to "-2.5",
            0.0001 to "0.0001",
            0.00001 to "1e-05",
            1e16 to "1e+16",
            1.5e16 to "1.5e+16",
            123456789012345.6 to "123456789012345.6",
            9999999999999998.0 to "9999999999999998.0",
            1e22 to "1e+22",
            1.7976931348623157e308 to "1.7976931348623157e+308",
            5e-324 to "5e-324",
            2.2250738585072014e-308 to "2.2250738585072014e-308",
            0.3 to "0.3",
            2.0 / 3.0 to "0.6666666666666666",
            // A power of two, of which the float below is nearer.
            Math.pow(2.0, -1022.0) to "2.2250738585072014e-308",
            Math.pow(2.0, 60.0) to "1.152921504606847e+18",
            -0.0 to "-0.0",
            Double.NaN to "nan",
            Double.NEGATIVE_INFINITY to "-inf",
        )) {
            assertEquals(text, floatRepr(value), "$value")
        }
    }

    @Test
    fun hex() {
        assertEquals("00FF7F", hexUpper(hex("00ff7f")))
        assertEquals("00ff7f", hexDecode("00Ff7F")?.toHex())

        for (bad in listOf("0", "0g", "\u0661\u0662")) assertEquals(null, hexDecode(bad), bad)
    }

    @Test
    fun readsJson() {
        fun parse(text: String) = JsonText.parse(text.toByteArray())

        assertEquals(Json.Null, parse(" null "))
        assertEquals(Json.Bool(true), parse("true"))
        assertEquals(Json.Integer(BigInteger.ZERO), parse("-0"))
        assertEquals(Json.Float(15.0), parse("1.5e1"))
        assertEquals(Json.Str("a\u00e9\ud83d\ude00\n"), parse("\"a\\u00e9\\ud83d\\ude00\\n\""))
        assertEquals(
            Json.Object(linkedMapOf("a" to Json.Integer(BigInteger.TWO), "c" to Json.Str(""))),
            parse("{\"a\":[1,{\"b\":null}],\"a\":2,\"c\":\"\"}"),
        )
        assertTrue((parse("NaN") as Json.Float).value.isNaN())

        for (bad in listOf(
            "", "{", "[1,]", "{\"a\":1,}", "01", "1.", ".5", "+1", "\"\\x\"", "\"a", "nul", "[] []",
            "\"\u0001\"", "\"\\ud800\"", "{1:2}", "'a'", "1 2", "[".repeat(130) + "]".repeat(130),
        )) {
            assertThrows<DecodeException>(bad) { parse(bad) }
        }

        assertThrows<DecodeException> { JsonText.parse(byteArrayOf(0x22, 0xff.toByte(), 0x22)) }
    }

    @Test
    fun writesStringsAsPython() {
        val out = StringBuilder()

        JsonText.writeString(out, "a\"\\/\n\t\u0001\u00e9\u007f\ud83d\ude00")

        assertEquals("\"a\\\"\\\\/\\n\\t\\u0001\\u00e9\\u007f\\ud83d\\ude00\"", out.toString())
        assertEquals("{\n  \"a\": [\n    1,\n    {}\n  ]\n}", JsonText.indent("{\"a\":[1,{}]}", 2))
    }
}
