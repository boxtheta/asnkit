// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigDecimal
import java.math.MathContext
import java.math.RoundingMode

// What the text codecs write numbers and bytes as: floats as Python writes
// them, and bytes in hex.

private const val HEX_UPPER = "0123456789ABCDEF"

/** The bytes in upper case hex. */
internal fun hexUpper(bytes: ByteArray): String {
    val out = StringBuilder(bytes.size * 2)

    for (byte in bytes) {
        out.append(HEX_UPPER[(byte.toInt() shr 4) and 0x0f]).append(HEX_UPPER[byte.toInt() and 0x0f])
    }

    return out.toString()
}

/** The bytes of hex of any case, with an even number of digits. */
internal fun hexDecode(text: String): ByteArray? {
    if (text.length % 2 == 1) return null

    val bytes = ByteArray(text.length / 2)

    for (index in bytes.indices) {
        val high = Character.digit(text[2 * index], 16)
        val low = Character.digit(text[2 * index + 1], 16)

        // `Character.digit` takes digits of other scripts too.
        if (high < 0 || low < 0 || text[2 * index] > 'f' || text[2 * index + 1] > 'f') return null

        bytes[index] = (high * 16 + low).toByte()
    }

    return bytes
}

/** The most digits of an integer that is read or written, as Python does: more is slow, and nothing sane has them. */
internal const val MAX_DIGITS: Int = 4300

/**
 * The fewest decimal digits that read back as `value`, which is finite and
 * not 0, and the exponent of the first of them: `value` is `0.d1d2d3... *
 * 10**(exponent + 1)`. Of two that are as few, the nearer to `value`.
 */
internal fun shortestDigits(value: Double): Pair<String, Int> {
    val exact = BigDecimal(Math.abs(value))

    for (precision in 1..17) {
        val candidates =
            listOf(RoundingMode.DOWN, RoundingMode.UP)
                .map { exact.round(MathContext(precision, it)) }
                .filter { it.toDouble() == Math.abs(value) }

        val best = candidates.minByOrNull { it.subtract(exact).abs() } ?: continue
        val stripped = best.stripTrailingZeros()
        val digits = stripped.unscaledValue().toString()

        return digits to (digits.length - 1 - stripped.scale())
    }

    // 17 digits always read back.
    error("No digits read back as $value.")
}

/** A float as `repr()` of Python writes it: the fewest digits that read back as the same float. */
internal fun floatRepr(value: Double): String {
    if (value.isNaN()) return "nan"

    if (value.isInfinite()) return if (value > 0) "inf" else "-inf"

    if (value == 0.0) return if (1.0 / value < 0) "-0.0" else "0.0"

    val (digits, exponent) = shortestDigits(value)
    val sign = if (value < 0) "-" else ""

    return if (exponent in -4 until 16) {
        if (exponent >= 0) {
            val whole = exponent + 1

            if (digits.length <= whole) {
                sign + digits + "0".repeat(whole - digits.length) + ".0"
            } else {
                sign + digits.substring(0, whole) + "." + digits.substring(whole)
            }
        } else {
            sign + "0." + "0".repeat(-exponent - 1) + digits
        }
    } else {
        val mantissa = if (digits.length > 1) digits.substring(0, 1) + "." + digits.substring(1) else digits
        val magnitude = Math.abs(exponent).toString().padStart(2, '0')

        sign + mantissa + "e" + (if (exponent < 0) "-" else "+") + magnitude
    }
}
