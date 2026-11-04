// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.math.BigDecimal
import java.math.BigInteger
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction

// Basic Encoding Rules (BER) and Distinguished Encoding Rules (DER), X.690.
//
// A type is encoded as a tag, a length and the contents. The tags a type
// has on the wire are worked out when the types are lowered: an explicit
// tag is a tag around the tag of the type it tags, and an implicit one
// takes the place of that tag.

private const val MAX_DEPTH = 128

/** A tag as it is read: its class, if it is constructed, its number, and the offset after it. */
internal class ReadTag(
    val tagClass: TagClass,
    val constructed: Boolean,
    val number: ULong,
    val end: Int,
)

/** A length as it is read: `null` if it is indefinite, and the offset after it. */
internal class ReadLength(
    val length: Int?,
    val end: Int,
)

/** A value that is decoded, and the offset after it. */
internal class Decoded(
    val value: Value,
    val end: Int,
)

internal object Ber {
    fun encode(
        types: Types,
        id: Int,
        value: Value,
    ): ByteArray {
        val out = ByteBuffer()

        Coder(types).encodeType(id, value, out, null, 0)

        return out.toByteArray()
    }

    /** Decodes the type `id` from the start of `data`, and returns it with the length of what was decoded. */
    fun decodeWithLength(
        types: Types,
        id: Int,
        data: ByteArray,
    ): Decoded = Coder(types).decodeType(id, data, 0, null, 0) ?: decodeError("Expected a tag that the type has.", 0)

    /** Decodes the type `id` from `data`, which is all of it. */
    fun decode(
        types: Types,
        id: Int,
        data: ByteArray,
    ): Value {
        val decoded = decodeWithLength(types, id, data)

        if (decoded.end != data.size) decodeError("Expected the data to be all of the value.", decoded.end)

        return decoded.value
    }

    /** The total length of the first value in `data`, if it is known. */
    fun decodeLength(data: ByteArray): Int? {
        val start =
            try {
                skipTag(data, 0)
            } catch (_: DecodeException) {
                return null
            }

        return try {
            val length = readLength(data, start, true, false)

            length.length?.let { length.end + it }
        } catch (_: DecodeException) {
            // Not all there: the length it says, if it can be read.
            readLengthOnly(data, start)
        }
    }

    private fun readLengthOnly(
        data: ByteArray,
        offset: Int,
    ): Int? {
        val first = data.getOrNull(offset)?.toInt()?.and(0xff) ?: return null

        if (first and 0x80 == 0) return offset + 1 + first

        val count = first and 0x7f

        if (count == 0 || count > 8 || offset + 1 + count > data.size) return null

        var length = 0L

        for (index in offset + 1..offset + count) {
            length = (length shl 8) or (data[index].toLong() and 0xff)
        }

        val total = length + offset + 1 + count

        return if (length < 0 || total > Int.MAX_VALUE) null else total.toInt()
    }

    /** The bytes of a tag. */
    fun tagBytes(
        tagClass: TagClass,
        constructed: Boolean,
        number: ULong,
        out: ByteBuffer,
    ) {
        val flags =
            when (tagClass) {
                TagClass.UNIVERSAL -> 0x00
                TagClass.APPLICATION -> 0x40
                TagClass.CONTEXT_SPECIFIC -> 0x80
                TagClass.PRIVATE -> 0xc0
            } or if (constructed) 0x20 else 0

        if (number < 31uL) {
            out.add(flags or number.toInt())

            return
        }

        out.add(flags or 0x1f)

        val groups = (64 - number.countLeadingZeroBits() + 6) / 7

        for (group in 0 until groups) {
            val bits = (number shr (7 * (groups - 1 - group))).toInt() and 0x7f

            out.add(if (group + 1 < groups) 0x80 or bits else bits)
        }
    }

    /** The tag at `offset`. */
    fun readTag(
        data: ByteArray,
        offset: Int,
    ): ReadTag {
        val first = data.getOrNull(offset)?.toInt()?.and(0xff) ?: decodeError("Ran out of data when reading tag", offset)
        val tagClass =
            when (first shr 6) {
                0 -> TagClass.UNIVERSAL
                1 -> TagClass.APPLICATION
                2 -> TagClass.CONTEXT_SPECIFIC
                else -> TagClass.PRIVATE
            }
        var at = offset + 1
        var number = (first and 0x1f).toULong()

        if (number == 0x1fuL) {
            number = 0uL

            while (true) {
                val byte = data.getOrNull(at)?.toInt()?.and(0xff) ?: decodeError("Ran out of data when reading tag", at)

                at += 1

                if (number shr 57 != 0uL) decodeError("A tag number is too large.", offset)

                number = (number shl 7) or (byte and 0x7f).toULong()

                if (byte and 0x80 == 0) break
            }
        }

        return ReadTag(tagClass, first and 0x20 != 0, number, at)
    }

    /** The offset after the tag at `offset`. As BER does, it needs a byte after the tag. */
    fun skipTag(
        data: ByteArray,
        offset: Int,
    ): Int {
        val at = readTag(data, offset).end

        if (at >= data.size) decodeError("Ran out of data when reading tag", at)

        return at
    }

    /** Reads a length, and checks that there is as much data after it. */
    fun readLength(
        data: ByteArray,
        offset: Int,
        enforceDefinite: Boolean,
        enforceMinimal: Boolean,
    ): ReadLength {
        val first = data.getOrNull(offset)?.toInt()?.and(0xff) ?: decodeError("Ran out of data when trying to read length", offset)
        var at = offset + 1
        val length: Long

        if (first and 0x80 == 0) {
            length = first.toLong()
        } else if (first == 0x80) {
            if (enforceDefinite) decodeError("Expected definite length, but got indefinite.", offset)

            return ReadLength(null, at)
        } else {
            val count = first and 0x7f

            if (count > data.size - at) decodeError("Ran out of data when trying to read length", at)

            if (count > 8 && (at until at + count - 8).any { data[it].toInt() != 0 }) decodeError("A length is too large.", at)

            var value = 0L

            for (index in at until at + count) value = (value shl 8) or (data[index].toLong() and 0xff)

            length = value

            if (enforceMinimal && ((length in 0..127) || data[at].toInt() == 0)) {
                decodeError("Expected the length in as few bytes as possible.", offset)
            }

            at += count
        }

        if (length < 0 || length > data.size - at) decodeError("Expected more contents bytes than there are.", at)

        return ReadLength(length.toInt(), at)
    }

    fun stringBytes(
        kind: StrKind,
        text: String,
    ): ByteArray =
        when (kind) {
            StrKind.UTF8 -> text.toByteArray(Charsets.UTF_8)
            StrKind.NUMERIC, StrKind.PRINTABLE, StrKind.IA5, StrKind.VISIBLE -> {
                if (text.any { it.code > 0x7f }) encodeError("A character is not ASCII.")

                text.toByteArray(Charsets.US_ASCII)
            }
            StrKind.GENERAL, StrKind.GRAPHIC, StrKind.OBJECT_DESCRIPTOR, StrKind.TELETEX, StrKind.VIDEOTEX -> {
                if (text.any { it.code > 0xff }) encodeError("A character is not Latin-1.")

                text.toByteArray(Charsets.ISO_8859_1)
            }
            StrKind.BMP -> bmpBytes(text)
            StrKind.UNIVERSAL -> {
                val out = ByteBuffer(text.length * 4)

                text.codePoints().forEach { code ->
                    out.add(code ushr 24)
                    out.add(code ushr 16)
                    out.add(code ushr 8)
                    out.add(code)
                }

                out.toByteArray()
            }
        }

    /** A BMPString in the 2-octet form, of which each character is in the BMP (X.690 8.23.8). */
    private fun bmpBytes(text: String): ByteArray {
        val out = ByteBuffer(text.length * 2)

        text.codePoints().forEach { code ->
            if (code > 0xffff) encodeError("A character is not in the BMP.")

            out.add(code ushr 8)
            out.add(code)
        }

        return out.toByteArray()
    }

    private fun isCharacter(code: Int): Boolean = code in 0..0x10ffff && code !in 0xd800..0xdfff

    /** The text of a string from its bytes. */
    fun stringText(
        kind: StrKind,
        bytes: ByteArray,
        offset: Int,
    ): String =
        when (kind) {
            StrKind.UTF8 ->
                try {
                    Charsets.UTF_8
                        .newDecoder()
                        .onMalformedInput(CodingErrorAction.REPORT)
                        .onUnmappableCharacter(CodingErrorAction.REPORT)
                        .decode(java.nio.ByteBuffer.wrap(bytes))
                        .toString()
                } catch (_: CharacterCodingException) {
                    decodeError("The string is not UTF-8.", offset)
                }
            StrKind.NUMERIC, StrKind.PRINTABLE, StrKind.IA5, StrKind.VISIBLE -> ascii(bytes, offset)
            StrKind.GENERAL, StrKind.GRAPHIC, StrKind.OBJECT_DESCRIPTOR, StrKind.TELETEX, StrKind.VIDEOTEX ->
                String(bytes, Charsets.ISO_8859_1)
            StrKind.BMP -> {
                if (bytes.size % 2 != 0) decodeError("The string is not a BMPString.", offset)

                val out = StringBuilder(bytes.size / 2)

                for (index in bytes.indices step 2) {
                    val code = ((bytes[index].toInt() and 0xff) shl 8) or (bytes[index + 1].toInt() and 0xff)

                    // A surrogate is not a character.
                    if (!isCharacter(code)) decodeError("The string is not a BMPString.", offset)

                    out.append(code.toChar())
                }

                out.toString()
            }
            StrKind.UNIVERSAL -> {
                if (bytes.size % 4 != 0) decodeError("The string is not UTF-32.", offset)

                val out = StringBuilder(bytes.size / 4)

                for (index in bytes.indices step 4) {
                    val code = (0 until 4).fold(0) { code, at -> (code shl 8) or (bytes[index + at].toInt() and 0xff) }

                    if (!isCharacter(code)) decodeError("The string is not UTF-32.", offset)

                    out.appendCodePoint(code)
                }

                out.toString()
            }
        }

    fun ascii(
        bytes: ByteArray,
        offset: Int,
    ): String {
        if (bytes.any { it < 0 }) decodeError("The string is not ASCII.", offset)

        return String(bytes, Charsets.US_ASCII)
    }

    /**
     * The REAL of X.690 8.5 in the form of DER (X.690 11.3.1): base 2, no
     * scaling factor, and an odd mantissa and its exponent each in the
     * fewest octets.
     */
    fun encodeReal(value: Double): ByteArray {
        when {
            value == Double.POSITIVE_INFINITY -> return byteArrayOf(0x40)
            value == Double.NEGATIVE_INFINITY -> return byteArrayOf(0x41)
            value.isNaN() -> return byteArrayOf(0x42)
            // Minus zero is a special value (X.690 8.5.3).
            value == 0.0 -> return if (1.0 / value < 0) byteArrayOf(0x43) else ByteArray(0)
        }

        val bits = java.lang.Double.doubleToRawLongBits(Math.abs(value))
        val biased = ((bits ushr 52) and 0x7ff).toInt()
        val fraction = bits and ((1L shl 52) - 1)
        // The number is mantissa * 2**exponent.
        var mantissa = if (biased == 0) fraction else fraction or (1L shl 52)
        var exponent = if (biased == 0) -1074 else biased - 1075
        val shift = java.lang.Long.numberOfTrailingZeros(mantissa)

        mantissa = mantissa ushr shift
        exponent += shift

        val sign = if (value < 0) 0x40 else 0
        val out = ByteBuffer(12)

        if (exponent in -128..127) {
            out.add(0x80 or sign)
            out.add(exponent)
        } else {
            // Every exponent of a double fits in two octets.
            out.add(0x81 or sign)
            out.add(exponent shr 8)
            out.add(exponent)
        }

        val length = (64 - java.lang.Long.numberOfLeadingZeros(mantissa) + 7) / 8

        for (index in length - 1 downTo 0) out.add((mantissa ushr (8 * index)).toInt())

        return out.toByteArray()
    }

    /**
     * An integer as a REAL: the nearest double to one of up to 128 bits, as
     * the runtime for Rust has it, and one of more as it adds up its bytes.
     */
    fun realOfInteger(value: BigInteger): Double {
        val real =
            if (value.bitLength() < 128) {
                value.toDouble()
            } else {
                val bytes = value.toByteArray()
                val negative = value.signum() < 0
                var sum = 0.0

                for (byte in bytes) {
                    val b = if (negative) byte.toInt().inv() and 0xff else byte.toInt() and 0xff

                    sum = sum * 256.0 + b
                }

                if (negative) -(sum + 1.0) else sum
            }

        if (real.isInfinite()) encodeError("The integer is too large for a REAL.")

        return real
    }

    /** If the two's complement number has an octet more than it needs. */
    private fun redundant(
        bytes: ByteArray,
        from: Int,
        to: Int,
    ): Boolean =
        to - from > 1 &&
            (
                (bytes[from].toInt() == 0x00 && bytes[from + 1] >= 0) ||
                    (bytes[from].toInt() == -1 && bytes[from + 1] < 0)
            )

    private val LIMIT = BigInteger.ONE.shiftLeft(100)

    /** A two's complement number, as one beyond 2**100 is that to the float it makes. */
    private fun exponentValue(
        bytes: ByteArray,
        from: Int,
        to: Int,
    ): BigInteger {
        var start = from

        while (redundant(bytes, start, to)) start += 1

        if (to - start > 12) return if (bytes[start] < 0) LIMIT.negate() else LIMIT

        return integerOf(bytes.copyOfRange(start, to))
    }

    /** The float nearest to `mantissa` times two to the power of `exponent`, with ties to even, as Python reads an integer as a float. */
    private fun nearestFloat(
        mantissa: BigInteger,
        exponent: BigInteger,
    ): Double {
        if (mantissa.signum() == 0) return 0.0

        // Of the highest bit.
        val top = exponent.add(BigInteger.valueOf((mantissa.bitLength() - 1).toLong()))

        if (top > BigInteger.valueOf(1023)) throw RealError("The REAL is too large for a float.")

        if (top < BigInteger.valueOf(-1075)) return 0.0

        val shift = exponent.toInt()
        val value =
            if (shift >= 0) {
                mantissa.shiftLeft(shift).toDouble()
            } else {
                BigDecimal(mantissa).divide(BigDecimal(BigInteger.ONE.shiftLeft(-shift))).toDouble()
            }

        if (value.isInfinite()) throw RealError("The REAL is too large for a float.")

        return value
    }

    /** What is wrong with the contents of a REAL. */
    private class RealError(
        val what: String,
    ) : RuntimeException(what, null, false, false)

    /** A REAL in binary (X.690 8.5.7), of DER only if `strict` (X.690 11.3.1). */
    private fun binaryReal(
        contents: ByteArray,
        strict: Boolean,
    ): Double {
        val out = "Out of data in a REAL."
        val control = contents[0].toInt() and 0xff
        val baseBits =
            when ((control shr 4) and 0x03) {
                0 -> 1L
                1 -> 3L
                2 -> 4L
                else -> throw RealError("A REAL has a reserved base.")
            }
        val exponentFrom: Int
        val exponentTo: Int

        if (control and 0x03 == 0x03) {
            if (contents.size < 2) throw RealError(out)

            val count = contents[1].toInt() and 0xff

            if (count == 0) throw RealError("A REAL has an exponent of no octets.")

            exponentFrom = 2
            exponentTo = 2 + count
        } else {
            exponentFrom = 1
            exponentTo = 1 + (control and 0x03) + 1
        }

        if (exponentTo > contents.size) throw RealError(out)

        // The long form has no first nine bits all the same (X.690 8.5.7.4 d).
        if (control and 0x03 == 0x03 && redundant(contents, exponentFrom, exponentTo)) {
            throw RealError("The exponent of a REAL is not in the fewest octets.")
        }

        if (strict) {
            if (control and 0x3c != 0) throw RealError("A REAL of DER has the base 2 and no scaling factor.")

            if (redundant(contents, exponentFrom, exponentTo) || control and 0x03 == 0x03 && exponentTo - exponentFrom <= 3) {
                throw RealError("The exponent of a REAL is not in the fewest octets.")
            }

            if (exponentTo == contents.size || contents[exponentTo].toInt() == 0 || contents.last().toInt() and 1 == 0) {
                throw RealError("The mantissa of a REAL is not odd and in the fewest octets.")
            }
        }

        val scaling = ((control shr 2) and 0x03).toLong()
        val exponent = exponentValue(contents, exponentFrom, exponentTo) * BigInteger.valueOf(baseBits) + BigInteger.valueOf(scaling)
        val value = nearestFloat(BigInteger(1, contents.copyOfRange(exponentTo, contents.size)), exponent)

        return if (control and 0x40 != 0) -value else value
    }

    private fun digits(
        text: String,
        from: Int,
    ): Int {
        var at = from

        while (at < text.length && text[at] in '0'..'9') at += 1

        return at - from
    }

    /** A REAL in decimal (X.690 8.5.8): ISO 6093 NR1, NR2 or NR3, after spaces, or only NR3 in the form of DER if `strict` (X.690 11.3.2). */
    private fun decimalReal(
        control: Int,
        text: String,
        strict: Boolean,
    ): Double {
        val bad = "Bad decimal REAL."
        val form = control and 0x3f

        if (strict) {
            // -?[1-9]([0-9]*[1-9])?.E(+0|-?[1-9][0-9]*)
            val rest = text.removePrefix("-")
            val mantissa = digits(rest, 0)

            if (!rest.startsWith(".E", mantissa)) throw RealError(bad)

            val exponent = rest.substring(mantissa + 2)
            val goodExponent =
                exponent == "+0" ||
                    exponent.removePrefix("-").let { it.isNotEmpty() && it[0] in '1'..'9' && digits(it, 0) == it.length }
            val good = form == 0x03 && mantissa > 0 && rest[0] != '0' && rest[mantissa - 1] != '0' && goodExponent

            if (!good) throw RealError("A decimal REAL of DER is not in the form of X.690 11.3.2.")
        } else {
            var at = 0

            while (at < text.length && text[at] == ' ') at += 1

            if (at < text.length && (text[at] == '-' || text[at] == '+')) at += 1

            val whole = digits(text, at)
            var point = false
            var fraction = 0

            at += whole

            if (at < text.length && (text[at] == '.' || text[at] == ',')) {
                point = true
                fraction = digits(text, at + 1)
                at += 1 + fraction
            }

            var exponent = false

            if (at < text.length && (text[at] == 'E' || text[at] == 'e')) {
                at += 1

                if (at < text.length && (text[at] == '-' || text[at] == '+')) at += 1

                val count = digits(text, at)

                if (count == 0 || at + count != text.length) throw RealError(bad)

                exponent = true
                at = text.length
            }

            val good =
                at == text.length &&
                    whole + fraction > 0 &&
                    when (form) {
                        0x01 -> !point && !exponent
                        0x02 -> point && !exponent
                        0x03 -> point && exponent
                        else -> throw RealError("A decimal REAL has a reserved form.")
                    }

            if (!good) throw RealError(bad)
        }

        val value =
            try {
                text.filter { it != ' ' }.replace(',', '.').toDouble()
            } catch (_: NumberFormatException) {
                throw RealError(bad)
            }

        if (value.isInfinite()) throw RealError("The REAL is too large for a float.")

        return value
    }

    /** A REAL (X.690 8.5), as the float nearest to it. `strict` takes only the form of DER (X.690 11.3). */
    fun decodeReal(
        contents: ByteArray,
        offset: Int,
        strict: Boolean,
    ): Double {
        if (contents.isEmpty()) return 0.0

        val control = contents[0].toInt() and 0xff

        try {
            return when {
                control and 0x80 != 0 -> binaryReal(contents, strict)
                // One octet only (X.690 8.5.9).
                control and 0x40 != 0 ->
                    when {
                        contents.size > 1 -> throw RealError("A special REAL has more than one octet.")
                        control == 0x40 -> Double.POSITIVE_INFINITY
                        control == 0x41 -> Double.NEGATIVE_INFINITY
                        control == 0x42 -> Double.NaN
                        control == 0x43 -> -0.0
                        else -> throw RealError("Unsupported special REAL control word.")
                    }
                else -> decimalReal(control, String(contents, 1, contents.size - 1, Charsets.ISO_8859_1), strict)
            }
        } catch (error: RealError) {
            decodeError(error.what, offset)
        }
    }

    /** The order of two encodings as octet strings, the shorter padded with zero octets (X.690 11.6). */
    fun paddedCompare(
        a: ByteArray,
        b: ByteArray,
    ): Int {
        for (index in 0 until maxOf(a.size, b.size)) {
            val x = a.getOrElse(index) { 0 }.toInt() and 0xff
            val y = b.getOrElse(index) { 0 }.toInt() and 0xff

            if (x != y) return x.compareTo(y)
        }

        return 0
    }

    /** The contents of a BOOLEAN, which DER has as 0x00 or 0xff. */
    fun checkBoolean(
        contents: ByteArray,
        strict: Boolean,
        offset: Int,
    ): Boolean {
        if (contents.size != 1) decodeError("Expected BOOLEAN contents length 1.", offset)

        if (strict && contents[0].toInt() != 0 && contents[0].toInt() != -1) {
            decodeError("Expected BOOLEAN contents of 0x00 or 0xff.", offset)
        }

        return contents[0].toInt() != 0
    }

    /** Checks what DER requires of the contents of an INTEGER: at least a byte, and as few as there can be. */
    fun checkIntegerContents(
        contents: ByteArray,
        strict: Boolean,
        offset: Int,
    ) {
        if (!strict) return

        if (contents.isEmpty()) decodeError("Expected at least one byte of an INTEGER.", offset)

        if (redundant(contents, 0, contents.size)) decodeError("Expected an INTEGER in as few bytes as possible.", offset)
    }

    /** Checks the contents of a BIT STRING, and returns its number of bits. */
    fun checkBitString(
        contents: ByteArray,
        strict: Boolean,
        named: Boolean,
        offset: Int,
    ): Int {
        if (contents.isEmpty()) decodeError("Expected at least one byte of a BIT STRING.", offset)

        val unused = contents[0].toInt() and 0xff

        if (unused > 7 || (contents.size == 1 && unused != 0)) decodeError("Expected at most 7 unused bits of a BIT STRING.", offset)

        val bits = 8 * (contents.size - 1) - unused

        if (strict && contents.size > 1) {
            val last = contents.last().toInt() and 0xff

            if (unused != 0 && last and ((1 shl unused) - 1) != 0) {
                decodeError("Expected the unused bits of a BIT STRING to be zero.", offset)
            }

            if (named && bits > 0 && (last shr unused) and 1 == 0) {
                decodeError("Expected a BIT STRING with named bits without trailing zero bits.", offset)
            }
        }

        return bits
    }
}

private fun Kind.hasBaseTag(): Boolean = !(this is Kind.Choice || this is Kind.Any || this is Kind.AnyDefinedBy || this is Kind.OpenType || this is Kind.Ref)

/** Types whose contents can be in the tag with or without the constructed bit. */
private fun Kind.isPrimitiveOrConstructed(): Boolean = this is Kind.BitString || this is Kind.OctetString || this is Kind.Str

private fun Kind.allowsIndefinite(): Boolean =
    this is Kind.Sequence || this is Kind.Set || this is Kind.External || this is Kind.SequenceOf || this is Kind.SetOf

private class Coder(
    val types: Types,
) {
    val der = types.codec == Codec.DER

    /** What DER requires of what it decodes. */
    val strict = der && !types.lenient

    // Encoding.

    fun encodeType(
        id: Int,
        value: Value,
        out: ByteBuffer,
        siblings: Map<String, Value>?,
        depth: Int,
    ) {
        if (depth > MAX_DEPTH) encodeError("The value is nested too deeply.")

        val node = types.node(id)

        encodeTags(node.tags, 0, node.concrete, value, out, siblings, depth)
    }

    /** A TLV of `tag`, of which `contents` writes the contents after the tag and the length, which is made longer after them if it has to be. */
    inline fun writeTlv(
        tag: WireTag,
        out: ByteBuffer,
        contents: () -> Unit,
    ) {
        Ber.tagBytes(tag.tagClass, tag.constructed, tag.number, out)

        val at = out.size

        // Most lengths are a byte.
        out.add(0)
        contents()

        val length = out.size - at - 1

        if (length <= 127) {
            out[at] = length
        } else {
            val bytes = BigInteger.valueOf(length.toLong()).toByteArray().dropWhile { it.toInt() == 0 }.toByteArray()

            out[at] = 0x80 or bytes.size
            out.insert(at + 1, bytes)
        }
    }

    fun encodeTags(
        tags: List<WireTag>,
        index: Int,
        concrete: Int,
        value: Value,
        out: ByteBuffer,
        siblings: Map<String, Value>?,
        depth: Int,
    ) {
        val kind = types.node(concrete).kind

        if (index == tags.size) return encodeUntagged(kind, value, out, siblings, depth)

        if (index == tags.size - 1 && kind.hasBaseTag()) {
            writeTlv(tags[index], out) { encodeContent(kind, value, out, depth) }
        } else {
            writeTlv(tags[index], out) { encodeTags(tags, index + 1, concrete, value, out, siblings, depth) }
        }
    }

    /** A type with no tag of its own: a CHOICE is its alternative. */
    fun encodeUntagged(
        kind: Kind,
        value: Value,
        out: ByteBuffer,
        siblings: Map<String, Value>?,
        depth: Int,
    ) {
        when (kind) {
            is Kind.Choice -> {
                if (value !is Value.Choice) encodeError("Expected data of type tuple(str, object).")

                val member = kind.members.named(value.name) ?: encodeError("Expected a name of an alternative of the CHOICE.")

                inMember(value.name) { encodeType(member.type, value.value, out, null, depth + 1) }
            }
            is Kind.Any -> out.addAll((value as? Value.Bytes)?.value ?: encodeError("Expected data of type bytes."))
            is Kind.AnyDefinedBy -> {
                if (kind.choices.isEmpty()) {
                    out.addAll((value as? Value.Bytes)?.value ?: encodeError("Expected data of type bytes."))

                    return
                }

                val key = siblings?.get(kind.value)
                val choice = key?.let { kind.choices.firstOrNull { (candidate, _) -> constMatches(candidate, key) } } ?: encodeError("Bad AnyDefinedBy choice.")

                encodeType(choice.second, value, out, null, depth + 1)
            }
            else -> encodeError("The type has no tag.")
        }
    }

    /** The contents of a value of a type with a tag of its own. */
    fun encodeContent(
        kind: Kind,
        value: Value,
        out: ByteBuffer,
        depth: Int,
    ) {
        when {
            kind.mapMembers != null -> encodeMembers(kind.mapMembers!!, kind is Kind.Set, value, out, depth)
            kind is Kind.SequenceOf -> encodeElements(kind.element, false, value, out, depth)
            kind is Kind.SetOf -> encodeElements(kind.element, true, value, out, depth)
            kind is Kind.OctetString && value is Value.Bytes -> out.addAll(value.value)
            kind is Kind.ObjectIdentifier && value is Value.ObjectIdentifier -> out.addAll(value.bytesUnsafe)
            else -> out.addAll(encodePrimitive(kind, value))
        }
    }

    /** The contents of a value of a type with no others in it. */
    fun encodePrimitive(
        kind: Kind,
        value: Value,
    ): ByteArray =
        when (kind) {
            is Kind.Boolean -> byteArrayOf(if ((value as? Value.Boolean ?: encodeError("Expected data of type bool.")).value) -1 else 0)
            is Kind.Integer ->
                when (value) {
                    is Value.Integer -> value.value.toByteArray()
                    is Value.Text -> (kind.number(value.value) ?: encodeError("Expected an integer or a named number.")).toByteArray()
                    else -> encodeError("Expected data of type int or str.")
                }
            is Kind.Real ->
                when (value) {
                    is Value.Real -> Ber.encodeReal(value.value)
                    is Value.Integer -> Ber.encodeReal(Ber.realOfInteger(value.value))
                    else -> encodeError("Expected data of type float or int.")
                }
            is Kind.Null -> if (value == Value.Null) ByteArray(0) else encodeError("Expected None.")
            is Kind.BitString -> {
                if (value !is Value.Bits) encodeError("Expected data of type tuple(bytes, int).")

                if (value.length < 0 || value.bytes.size.toLong() * 8 < value.length) encodeError("Expected more bits of data.")

                // X.690 11.2.2: trailing zero bits of a named bit list are left out.
                val (bytes, bits) = if (der && kind.named.isNotEmpty()) cleanBits(value.bytes, value.length, true) else value.bytes to value.length
                val whole = bits / 8
                val rest = bits % 8
                val out = ByteBuffer(whole + 2)

                if (rest == 0) {
                    out.add(0)
                    out.addAll(bytes, 0, whole)
                } else {
                    out.add(8 - rest)
                    out.addAll(bytes, 0, whole)
                    out.add(bytes[whole].toInt() and (0xff ushr rest).inv())
                }

                out.toByteArray()
            }
            is Kind.OctetString -> (value as? Value.Bytes)?.value?.copyOf() ?: encodeError("Expected data of type bytes or bytearray.")
            is Kind.ObjectIdentifier -> (value as? Value.ObjectIdentifier)?.bytes ?: encodeError("Expected an OBJECT IDENTIFIER.")
            is Kind.Enumerated -> {
                val number =
                    when {
                        value is Value.Text && !types.numericEnums -> kind.items.firstOrNull { it.first == value.value }?.second
                        value is Value.Integer && types.numericEnums -> kind.items.firstOrNull { BigInteger.valueOf(it.second) == value.value }?.second
                        else -> encodeError("Expected an enumeration value.")
                    } ?: encodeError("Expected an enumeration value.")

                BigInteger.valueOf(number).toByteArray()
            }
            is Kind.Str -> Ber.stringBytes(kind.kind, (value as? Value.Text ?: encodeError("Expected data of type str.")).value)
            is Kind.Utc -> utcTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), der).toByteArray()
            is Kind.Generalized ->
                generalizedTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), der).toByteArray()
            is Kind.Date -> dateToString((value as? Value.Date ?: encodeError("Expected data of type datetime.date.")).date).toByteArray()
            is Kind.TimeOfDay -> timeOfDayToString(value as? Value.Time ?: encodeError("Expected data of type datetime.time.")).toByteArray()
            is Kind.DateTime -> dateTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime.")).toByteArray()
            is Kind.Duration -> {
                val text = (value as? Value.Text ?: encodeError("Expected data of type str.")).value

                validateDuration(text)

                (if (der) durationToCanonical(text) else text).substring(1).toByteArray()
            }
            else -> encodeError("The type has no contents of its own.")
        }

    fun encodeElements(
        element: Int,
        isSet: Boolean,
        value: Value,
        out: ByteBuffer,
        depth: Int,
    ) {
        if (value !is Value.List) encodeError("Expected data of type list.")

        if (!(der && isSet)) {
            value.items.forEachIndexed { index, item -> inElement(index) { encodeType(element, item, out, null, depth + 1) } }

            return
        }

        val encoded =
            value.items.mapIndexed { index, item ->
                val one = ByteBuffer()

                inElement(index) { encodeType(element, item, one, null, depth + 1) }
                one.toByteArray()
            }

        // X.690 11.6: sorted as octet strings, the shorter padded with zero octets.
        encoded.sortedWith(Ber::paddedCompare).forEach { out.addAll(it) }
    }

    fun encodeMembers(
        members: Members,
        isSet: Boolean,
        value: Value,
        out: ByteBuffer,
        depth: Int,
    ) {
        if (value !is Value.Map) encodeError("Expected data of type dict.")

        val map = value.members
        // DER sorts the members of a SET: where each ends.
        val sorted = isSet && der
        val start = out.size
        val ends = ArrayList<Int>()

        if (isSet) {
            for (index in members.order) {
                encodeMember(members.root[index], map, out, depth)

                if (sorted) ends.add(out.size)
            }
        } else {
            for (member in members.root) encodeMember(member, map, out, depth)
        }

        // An addition that can not be encoded ends the additions.
        additions@ for (addition in members.additions.orEmpty()) {
            val group =
                when (addition) {
                    is Addition.Of -> listOf(addition.member)
                    is Addition.Group -> addition.members
                }
            val before = out.size

            for (member in group) {
                try {
                    encodeMember(member, map, out, depth)
                } catch (_: EncodeException) {
                    out.truncate(before)
                    ends.removeAll { it > before }

                    break@additions
                }

                if (sorted) ends.add(out.size)
            }
        }

        // DER writes the members of a SET in the order of the tags they are
        // encoded with, which for an untagged CHOICE is that of the
        // alternative there (X.690 10.3).
        if (sorted) {
            val encoded = out.copyRange(start, out.size)
            val parts = ArrayList<ByteArray>()
            var from = 0

            for (end in ends) {
                parts.add(encoded.copyOfRange(from, end - start))
                from = end - start
            }

            out.truncate(start)

            parts
                .filter { it.isNotEmpty() }
                .sortedBy {
                    try {
                        val tag = Ber.readTag(it, 0)

                        TagKey(tag.tagClass, tag.number)
                    } catch (_: DecodeException) {
                        TagKey(TagClass.PRIVATE, ULong.MAX_VALUE)
                    }
                }.forEach { out.addAll(it) }
        }
    }

    fun encodeMember(
        member: MemberInfo,
        map: Map<String, Value>,
        out: ByteBuffer,
        depth: Int,
    ) {
        val value = map[member.name]

        when {
            value != null -> if (!isDefault(types, member, value)) inMember(member.name) { encodeType(member.type, value, out, map, depth + 1) }
            member.optional || member.hasDefault -> {}
            else -> encodeError("A member is not found in the SEQUENCE or SET.")
        }
    }

    // Decoding.

    fun indefiniteOk(kind: Kind): Boolean = !strict && kind.allowsIndefinite()

    /** Decodes the type `id` at `offset`, or says its tag is not the one there with `null`. */
    fun decodeType(
        id: Int,
        data: ByteArray,
        offset: Int,
        siblings: Map<String, Value>?,
        depth: Int,
    ): Decoded? {
        if (depth > MAX_DEPTH) decodeError("The data is nested too deeply.", offset)

        val node = types.node(id)

        return decodeTags(node.tags, 0, node.concrete, data, offset, siblings, true, depth)
    }

    fun decodeTags(
        tags: List<WireTag>,
        index: Int,
        concrete: Int,
        data: ByteArray,
        offset: Int,
        siblings: Map<String, Value>?,
        outer: Boolean,
        depth: Int,
    ): Decoded? {
        val kind = types.node(concrete).kind

        if (index == tags.size) return decodeUntagged(kind, data, offset, siblings, depth)

        val tag = tags[index]
        val read = Ber.readTag(data, offset)
        val isBase = index == tags.size - 1 && kind.hasBaseTag()
        val constructedOk = (isBase && kind.isPrimitiveOrConstructed()) || read.constructed == tag.constructed

        if (read.tagClass != tag.tagClass || read.number != tag.number || !constructedOk) {
            if (outer) return null

            decodeError("Expected another tag.", offset)
        }

        if (isBase) {
            val alternatives = kind.isPrimitiveOrConstructed()
            val length = Ber.readLength(data, read.end, !(indefiniteOk(kind) || alternatives && !strict), strict)
            val start = length.end
            val end = length.length?.let { start + it }

            return when {
                alternatives && read.constructed -> decodeConstructed(kind, data, start, end, depth)
                kind.mapMembers != null -> decodeMembers(kind.mapMembers!!, kind is Kind.Set, data, start, end, depth)
                kind is Kind.SequenceOf -> decodeArray(kind.element, false, data, start, end, depth)
                kind is Kind.SetOf -> decodeArray(kind.element, true, data, start, end, depth)
                else -> {
                    end ?: decodeError("Expected definite length.", read.end)

                    Decoded(decodeContent(kind, data.copyOfRange(start, end), start), end)
                }
            }
        }

        // An explicit tag around the rest.
        val length = Ber.readLength(data, read.end, strict, strict)
        val inner = decodeTags(tags, index + 1, concrete, data, length.end, siblings, false, depth) ?: decodeError("Expected another tag.", length.end)
        var end = inner.end

        if (length.length == null) {
            if (end + 2 > data.size || data[end].toInt() != 0 || data[end + 1].toInt() != 0) decodeError("Expected end-of-contents tag.", end)

            end += 2
        }

        return Decoded(inner.value, end)
    }

    fun decodeUntagged(
        kind: Kind,
        data: ByteArray,
        offset: Int,
        siblings: Map<String, Value>?,
        depth: Int,
    ): Decoded? =
        when (kind) {
            is Kind.Choice -> {
                val read = Ber.readTag(data, offset)
                val member = kind.members.all.firstOrNull { types.accepts(it.type, read.tagClass, read.number) }

                if (member != null) {
                    inMember(member.name) { decodeType(member.type, data, offset, null, depth + 1) }?.let {
                        Decoded(Value.Choice(member.name, it.value), it.end)
                    }
                } else if (kind.members.additions != null) {
                    // An alternative of a later version: skipped.
                    val length = Ber.readLength(data, Ber.skipTag(data, offset), true, false)

                    Decoded(Value.Choice("", Value.Null), length.end + (length.length ?: 0))
                } else {
                    null
                }
            }
            is Kind.Any -> decodeAny(data, offset)
            is Kind.AnyDefinedBy -> {
                if (kind.choices.isEmpty()) {
                    decodeAny(data, offset)
                } else {
                    val key = siblings?.get(kind.value)
                    val choice = key?.let { kind.choices.firstOrNull { (candidate, _) -> constMatches(candidate, key) } } ?: decodeError("Bad AnyDefinedBy choice.", offset)

                    decodeType(choice.second, data, offset, null, depth + 1)
                }
            }
            else -> decodeError("The type has no tag.", offset)
        }

    fun decodeAny(
        data: ByteArray,
        offset: Int,
    ): Decoded {
        val length = Ber.readLength(data, Ber.skipTag(data, offset), true, false)
        val end = length.end + (length.length ?: 0)

        return Decoded(Value.Bytes(data.copyOfRange(offset, end)), end)
    }

    fun decodeContent(
        kind: Kind,
        contents: ByteArray,
        offset: Int,
    ): Value =
        when (kind) {
            is Kind.Boolean -> Value.Boolean(Ber.checkBoolean(contents, strict, offset))
            is Kind.Integer -> {
                Ber.checkIntegerContents(contents, strict, offset)

                Value.Integer(integerOf(contents))
            }
            is Kind.Real -> Value.Real(Ber.decodeReal(contents, offset, strict))
            is Kind.Null -> if (contents.isEmpty()) Value.Null else decodeError("Expected NULL contents length 0.", offset)
            is Kind.BitString -> decodeBitString(contents, kind.named.isNotEmpty(), offset)
            is Kind.OctetString -> Value.Bytes(contents)
            is Kind.ObjectIdentifier -> Value.ObjectIdentifier.decode(contents, offset, strict)
            is Kind.Enumerated -> {
                Ber.checkIntegerContents(contents, strict, offset)

                val number = integerOf(contents)
                val found = kind.items.firstOrNull { BigInteger.valueOf(it.second) == number }

                when {
                    found != null && types.numericEnums -> Value.Integer(number)
                    found != null -> Value.Text(found.first)
                    kind.additions != null -> Value.Null
                    else -> decodeError("Expected an enumeration value.", offset)
                }
            }
            is Kind.Str -> Value.Text(Ber.stringText(kind.kind, contents, offset))
            // DER has one form of each (X.690 11.7, 11.8). A lenient reader
            // also takes any other BER has, as the same time in UTC.
            is Kind.Utc -> {
                val text = Ber.ascii(contents, offset)

                try {
                    utcTimeFromString(text, der)
                } catch (error: DecodeException) {
                    if (!der || strict) throw error

                    toUtc(utcTimeFromString(text, false))
                }
            }
            is Kind.Generalized -> {
                val text = Ber.ascii(contents, offset)

                try {
                    generalizedTimeFromString(text, der)
                } catch (error: DecodeException) {
                    if (!der || strict) throw error

                    // A GeneralizedTime with no offset is a local time, so
                    // the one in UTC has one of zero.
                    Value.DateTime(toUtc(generalizedTimeFromString(text, false)).dateTime, 0)
                }
            }
            is Kind.Date -> Value.Date(dateFromString(Ber.ascii(contents, offset)))
            is Kind.TimeOfDay -> Value.Time(timeOfDayFromString(Ber.ascii(contents, offset)))
            is Kind.DateTime -> Value.DateTime(dateTimeFromString(Ber.ascii(contents, offset)))
            is Kind.Duration -> {
                val text = "P" + Ber.ascii(contents, offset)
                val canonical =
                    try {
                        validateDuration(text)

                        if (der && strict) durationToCanonical(text) else text
                    } catch (_: EncodeException) {
                        decodeError("Invalid DURATION.", offset)
                    }

                if (canonical != text) decodeError("The DURATION is not in canonical form.", offset)

                Value.Text(text)
            }
            else -> decodeError("The type has no contents of its own.", offset)
        }

    fun decodeBitString(
        contents: ByteArray,
        named: Boolean,
        offset: Int,
    ): Value.Bits {
        val bits = Ber.checkBitString(contents, strict, named, offset)

        return Value.Bits(contents.copyOfRange(1, contents.size), bits)
    }

    fun atEndOfContents(
        data: ByteArray,
        at: Int,
    ): Boolean {
        if (at + 2 > data.size) decodeError("Ran out of data when trying to find End of Contents tag.", at)

        return data[at].toInt() == 0 && data[at + 1].toInt() == 0
    }

    /** A string in parts, the way BER allows. */
    fun decodeConstructed(
        kind: Kind,
        data: ByteArray,
        start: Int,
        end: Int?,
        depth: Int,
    ): Decoded {
        if (strict) decodeError("Expected a string that is not constructed.", start)

        if (depth > MAX_DEPTH) decodeError("The data is nested too deeply.", start)

        // The parts have the universal tag of the type, also if the type is
        // tagged otherwise: those of a BIT STRING are BIT STRINGs, and the
        // others are OCTET STRINGs.
        val segmentNumber = if (kind is Kind.BitString) 3uL else 4uL
        val segmentKind = if (kind is Kind.Str) Kind.OctetString else kind
        var at = start
        val segments = ArrayList<Value>()

        while (true) {
            if (end != null) {
                if (at >= end) break
            } else if (atEndOfContents(data, at)) {
                at += 2
                break
            }

            val read = Ber.readTag(data, at)

            if (read.tagClass != TagClass.UNIVERSAL || read.number != segmentNumber) decodeError("Expected another tag.", at)

            val length = Ber.readLength(data, read.end, false, false)

            if (read.constructed) {
                val inner = decodeConstructed(segmentKind, data, length.end, length.length?.let { length.end + it }, depth + 1)

                segments.add(inner.value)
                at = inner.end
            } else {
                val size = length.length ?: decodeError("Expected definite length.", read.end)
                val contents = data.copyOfRange(length.end, length.end + size)

                segments.add(
                    if (segmentKind is Kind.BitString) decodeBitString(contents, segmentKind.named.isNotEmpty(), length.end) else Value.Bytes(contents),
                )
                at = length.end + size
            }
        }

        val value =
            when (kind) {
                is Kind.BitString -> {
                    val bytes = ByteBuffer()
                    var bits = 0

                    for (segment in segments) {
                        if (segment !is Value.Bits) continue

                        // Only the last part has unused bits (X.690 8.6.4).
                        if (bits % 8 != 0) decodeError("Expected unused bits only in the last part of a BIT STRING.", start)

                        bytes.addAll(segment.bytes)
                        bits += segment.length
                    }

                    Value.Bits(bytes.toByteArray(), bits)
                }
                else -> {
                    val bytes = ByteBuffer()

                    segments.filterIsInstance<Value.Bytes>().forEach { bytes.addAll(it.value) }

                    if (kind is Kind.Str) Value.Text(Ber.stringText(kind.kind, bytes.toByteArray(), start)) else Value.Bytes(bytes.toByteArray())
                }
            }

        return Decoded(value, at)
    }

    /** The elements of a SEQUENCE OF or a SET OF, up to `end` or the end-of-contents octets. */
    fun decodeArray(
        element: Int,
        isSet: Boolean,
        data: ByteArray,
        offset: Int,
        end: Int?,
        depth: Int,
    ): Decoded {
        val items = ArrayList<Value>()
        var at = offset
        var previous: ByteArray? = null

        while (true) {
            if (end != null) {
                if (at >= end) break
            } else if (atEndOfContents(data, at)) {
                at += 2
                break
            }

            val start = at
            val item = inElement(items.size) { decodeType(element, data, at, null, depth + 1) } ?: decodeError("Expected the tag of the element.", at)

            at = item.end

            if (strict && isSet) {
                val encoded = data.copyOfRange(start, at)

                if (previous != null && Ber.paddedCompare(previous, encoded) > 0) {
                    decodeError("Expected the elements of a SET OF in ascending order of their encodings.", start)
                }

                previous = encoded
            }

            items.add(item.value)
        }

        return Decoded(Value.List(items), at)
    }

    /**
     * The members of a SEQUENCE or a SET, from `offset` up to `end` or, if
     * there is none, the end-of-contents octets.
     */
    fun decodeMembers(
        members: Members,
        isSet: Boolean,
        data: ByteArray,
        offset: Int,
        end: Int?,
        depth: Int,
    ): Decoded {
        val values = LinkedHashMap<String, Value>()
        var at: Int
        var outOfData: Boolean

        if (isSet) {
            // The members of a SET are in any order, the additions among
            // them, and DER has them in the order of their tags.
            val all = members.order.map { members.root[it] } + members.additionMembers
            val result = decodeMemberList(all, members.root.size, true, data, offset, end, values, depth)

            at = result.first
            outOfData = result.second
        } else {
            var result = decodeMemberList(members.root, members.root.size, false, data, offset, end, values, depth)

            // The additions, even when out of data, so that defaults are added.
            if (members.additions != null) {
                result = decodeMemberList(members.additionMembers, 0, false, data, result.first, end, values, depth)
            }

            at = result.first
            outOfData = result.second
        }

        if (outOfData) return Decoded(Value.Map(values), at)

        // Extra data is allowed in cases of versioned additions.
        return Decoded(Value.Map(values), end ?: decodeError("Could not find end-of-contents tag for indefinite length field.", at))
    }

    /** If the members end at `offset`, and the offset after the end-of-contents octets if they are there. */
    fun isEnd(
        data: ByteArray,
        offset: Int,
        end: Int?,
    ): Pair<Boolean, Int> =
        when {
            end != null -> (offset >= end) to offset
            atEndOfContents(data, offset) -> true to offset + 2
            else -> false to offset
        }

    /**
     * Decodes `members`, of which those from `additionsFrom` on are
     * additions, which may be missing. With `byTags`, the members of a SET,
     * DER has them in the order of their tags, and else in the order they
     * are declared in. Returns where they end, and if the data is all read.
     */
    fun decodeMemberList(
        members: List<MemberInfo>,
        additionsFrom: Int,
        byTags: Boolean,
        data: ByteArray,
        offset: Int,
        end: Int?,
        values: LinkedHashMap<String, Value>,
        depth: Int,
    ): Pair<Int, Boolean> {
        val decoded = BooleanArray(members.size)
        var successEarlier = false
        var lastTag: TagKey? = null
        var (outOfData, at) = isEnd(data, offset, end)

        // More than one pass lets the members be in any order.
        while (true) {
            var success = false

            for ((index, member) in members.withIndex()) {
                if (outOfData || decoded[index]) continue

                val start = at
                val found = inMember(member.name) { decodeType(member.type, data, at, values, depth + 1) }

                if (found != null) {
                    if (strict && byTags) {
                        val read = Ber.readTag(data, start)
                        val tag = TagKey(read.tagClass, read.number)

                        if (lastTag != null && lastTag >= tag) decodeError("The members of a SET are not in the order of their tags.", start)

                        lastTag = tag
                    } else if (successEarlier && strict) {
                        decodeError("A member is not in the order it is declared in.", found.end)
                    }

                    // A member with its default is left out in DER.
                    if (strict && member.hasDefault && isDefault(types, member, found.value)) {
                        decodeError("A member is encoded with its default value, which DER leaves out.", found.end)
                    }

                    success = true
                    at = found.end
                    decoded[index] = true
                    values[member.name] = found.value
                }

                val next = isEnd(data, at, end)

                outOfData = next.first
                at = next.second
            }

            successEarlier = successEarlier || success

            if (outOfData || !success) break
        }

        for ((index, member) in members.withIndex()) {
            if (decoded[index] || member.optional) continue

            val default = member.default

            when {
                default != null -> values[member.name] = default
                index >= additionsFrom -> break
                outOfData -> decodeError("A member is missing and has no default value.", at)
                else -> decodeError("Expected the tag of a member.", at)
            }
        }

        return at to outOfData
    }
}
