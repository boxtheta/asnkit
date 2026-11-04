// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import com.fasterxml.jackson.core.JsonFactory
import com.fasterxml.jackson.core.JsonParser
import com.fasterxml.jackson.core.JsonToken
import com.fasterxml.jackson.core.StreamReadConstraints
import com.fasterxml.jackson.core.json.JsonReadFeature
import java.io.IOException
import java.math.BigInteger
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction

// JSON Encoding Rules (JER), X.697.
//
// A SEQUENCE and a SET are objects, a CHOICE an object with one name, a
// SEQUENCE OF and a SET OF arrays, an OCTET STRING and a BIT STRING hex in
// upper case, an INTEGER a number and the strings strings. What is written
// is compact and with ASCII only, as `json.dumps` of the runtime for Python
// writes it.

private const val MAX_DEPTH = 128

/** JSON, as JER needs it. */
internal sealed class Json {
    data object Null : Json()

    data class Bool(
        val value: Boolean,
    ) : Json()

    data class Integer(
        val value: BigInteger,
    ) : Json()

    data class Float(
        val value: Double,
    ) : Json()

    data class Str(
        val value: String,
    ) : Json()

    data class Array(
        val items: List<Json>,
    ) : Json()

    /** In the order of the first appearance of each name, with the last value of a name given twice. */
    data class Object(
        val members: LinkedHashMap<String, Json>,
    ) : Json()
}

internal object JsonText {
    private val factory: JsonFactory =
        JsonFactory
            .builder()
            // Python reads these too.
            .enable(JsonReadFeature.ALLOW_NON_NUMERIC_NUMBERS)
            .streamReadConstraints(
                StreamReadConstraints
                    .builder()
                    .maxNestingDepth(Int.MAX_VALUE)
                    .maxStringLength(Int.MAX_VALUE)
                    .maxNumberLength(Int.MAX_VALUE)
                    .build(),
            ).build()

    private fun error(what: String): Nothing = decodeError(what, 0)

    /** Reads the JSON in `data`, which is UTF-8, and nothing after it. */
    fun parse(data: ByteArray): Json {
        val text =
            try {
                Charsets.UTF_8
                    .newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(java.nio.ByteBuffer.wrap(data))
                    .toString()
            } catch (_: CharacterCodingException) {
                error("Expected UTF-8.")
            }

        try {
            factory.createParser(text).use { parser ->
                val value = value(parser, parser.nextToken() ?: error("Expected a JSON value."), 0)

                if (parser.nextToken() != null) error("Data after the JSON value.")

                return value
            }
        } catch (error: IOException) {
            error("Bad JSON: ${error.message?.lineSequence()?.firstOrNull()}")
        }
    }

    private fun string(text: String): String {
        // A lone surrogate is no character.
        var index = 0

        while (index < text.length) {
            val c = text[index]

            if (Character.isHighSurrogate(c) && index + 1 < text.length && Character.isLowSurrogate(text[index + 1])) {
                index += 2
                continue
            }

            if (Character.isSurrogate(c)) error("A lone surrogate in a JSON string.")

            index += 1
        }

        return text
    }

    private fun value(
        parser: JsonParser,
        token: JsonToken,
        depth: Int,
    ): Json {
        if (depth > MAX_DEPTH) error("The JSON is nested too deeply.")

        return when (token) {
            JsonToken.START_OBJECT -> {
                val members = LinkedHashMap<String, Json>()

                while (true) {
                    val next = parser.nextToken() ?: error("Expected , or } in a JSON object.")

                    if (next == JsonToken.END_OBJECT) break

                    val name = string(parser.currentName())

                    members[name] = value(parser, parser.nextToken() ?: error("Expected a JSON value."), depth + 1)
                }

                Json.Object(members)
            }
            JsonToken.START_ARRAY -> {
                val items = ArrayList<Json>()

                while (true) {
                    val next = parser.nextToken() ?: error("Expected , or ] in a JSON array.")

                    if (next == JsonToken.END_ARRAY) break

                    items.add(value(parser, next, depth + 1))
                }

                Json.Array(items)
            }
            JsonToken.VALUE_STRING -> Json.Str(string(parser.text))
            JsonToken.VALUE_TRUE -> Json.Bool(true)
            JsonToken.VALUE_FALSE -> Json.Bool(false)
            JsonToken.VALUE_NULL -> Json.Null
            JsonToken.VALUE_NUMBER_INT -> {
                val digits = parser.text.removePrefix("-")

                if (digits.length > MAX_DIGITS) error("A JSON integer with too many digits.")

                Json.Integer(parser.bigIntegerValue)
            }
            JsonToken.VALUE_NUMBER_FLOAT -> Json.Float(parser.doubleValue)
            else -> error("Expected a JSON value.")
        }
    }

    /** Writes `text` as a JSON string, with all but ASCII as `\u` escapes, as `json.dumps` of Python does. */
    fun writeString(
        out: StringBuilder,
        text: String,
    ) {
        out.append('"')

        for (c in text) {
            when (c) {
                '"' -> out.append("\\\"")
                '\\' -> out.append("\\\\")
                '\n' -> out.append("\\n")
                '\r' -> out.append("\\r")
                '\t' -> out.append("\\t")
                '\b' -> out.append("\\b")
                '\u000c' -> out.append("\\f")
                in ' '..'~' -> out.append(c)
                // Each UTF-16 code unit, as a character is in Java already.
                else -> out.append("\\u").append(Integer.toHexString(c.code).padStart(4, '0'))
            }
        }

        out.append('"')
    }

    /**
     * JSON written with no spaces, as JER writes it, with `indent` spaces for
     * each level, as `json.dumps` of Python does with `indent`: an item and a
     * member on a line of its own, `": "` after a name, and an empty `{}`
     * and `[]` as they are.
     */
    fun indent(
        compact: String,
        indent: Int,
    ): String {
        val out = StringBuilder(compact.length * 2)
        var level = 0
        var inString = false
        var escaped = false
        var index = 0

        fun newLine() {
            out.append('\n')
            repeat(level * indent) { out.append(' ') }
        }

        while (index < compact.length) {
            val c = compact[index++]

            if (inString) {
                out.append(c)

                when {
                    escaped -> escaped = false
                    c == '\\' -> escaped = true
                    c == '"' -> inString = false
                }

                continue
            }

            when (c) {
                '"' -> {
                    inString = true
                    out.append(c)
                }
                '{', '[' -> {
                    out.append(c)

                    if (index < compact.length && (compact[index] == '}' || compact[index] == ']')) {
                        out.append(compact[index++])
                    } else {
                        level += 1
                        newLine()
                    }
                }
                '}', ']' -> {
                    level -= 1
                    newLine()
                    out.append(c)
                }
                ',' -> {
                    out.append(c)
                    newLine()
                }
                ':' -> out.append(": ")
                else -> out.append(c)
            }
        }

        return out.toString()
    }
}

internal class Jer(
    private val types: Types,
) {
    fun encode(
        id: Int,
        value: Value,
        indent: Int? = null,
    ): ByteArray {
        val out = StringBuilder()

        encode(id, value, out, 0)

        val text = if (indent == null) out.toString() else JsonText.indent(out.toString(), indent)

        return text.toByteArray(Charsets.US_ASCII)
    }

    fun decode(
        id: Int,
        data: ByteArray,
    ): Value = decode(id, JsonText.parse(data), 0)

    /** The size of a BIT STRING that has one size only. */
    private fun fixedSize(id: Int): Int? {
        val size = types.constraints(id).size ?: return null
        val min = size.min ?: return null

        return if (min == size.max && min.bitLength() < 31) min.toInt() else null
    }

    private fun encode(
        id: Int,
        value: Value,
        out: StringBuilder,
        depth: Int,
    ) {
        if (depth > MAX_DEPTH) encodeError("The value is nested too deeply.")

        when (val kind = types.kind(id)) {
            is Kind.Boolean -> out.append((value as? Value.Boolean ?: encodeError("Expected data of type bool.")).value)
            is Kind.Integer ->
                when (value) {
                    is Value.Integer -> writeInteger(value.value, out)
                    is Value.Text -> writeInteger(kind.number(value.value) ?: encodeError("Expected an integer or a named number."), out)
                    else -> encodeError("Expected data of type int or str.")
                }
            is Kind.Real ->
                when (value) {
                    // The special values are strings (X.697 23.2).
                    is Value.Real ->
                        out.append(
                            when {
                                value.value.isNaN() -> "\"NaN\""
                                value.value == Double.POSITIVE_INFINITY -> "\"INF\""
                                value.value == Double.NEGATIVE_INFINITY -> "\"-INF\""
                                value.value == 0.0 && 1.0 / value.value < 0 -> "\"-0\""
                                else -> floatRepr(value.value)
                            },
                        )
                    is Value.Integer -> writeInteger(value.value, out)
                    else -> encodeError("Expected data of type float or int.")
                }
            is Kind.Null -> if (value == Value.Null) out.append("null") else encodeError("Expected None.")
            is Kind.BitString -> {
                if (value !is Value.Bits) encodeError("Expected data of type tuple(bytes, int).")

                if (value.length < 0 || value.bytes.size.toLong() * 8 < value.length) encodeError("Expected more bits of data.")

                // The bytes of the bits, with those past them 0 (X.697 24.2.1).
                var (bytes, bits) = cleanBits(value.bytes, value.length, false)
                val size = fixedSize(id)

                if (size != null) {
                    // With named bits, trailing 0 bits are added or removed to
                    // make the size (X.697 24.2.2).
                    if (kind.named.isNotEmpty()) {
                        val (trimmed, length) = cleanBits(bytes, bits, true)

                        if (length <= size) {
                            bytes = trimmed.copyOf((size + 7) / 8)
                            bits = size
                        }
                    }

                    if (bits != size) encodeError("The BIT STRING is not of its size.")

                    JsonText.writeString(out, hexUpper(bytes))
                } else {
                    out.append("{\"value\":")
                    JsonText.writeString(out, hexUpper(bytes))
                    out.append(",\"length\":").append(bits).append('}')
                }
            }
            is Kind.OctetString -> JsonText.writeString(out, hexUpper((value as? Value.Bytes ?: encodeError("Expected data of type bytes or bytearray.")).value))
            is Kind.ObjectIdentifier -> JsonText.writeString(out, (value as? Value.ObjectIdentifier ?: encodeError("Expected an OBJECT IDENTIFIER.")).toString())
            is Kind.Str -> JsonText.writeString(out, (value as? Value.Text ?: encodeError("Expected data of type str.")).value)
            is Kind.Enumerated -> {
                // The name, also of a value given as its number (X.697 22).
                val name =
                    when {
                        value is Value.Text && !types.numericEnums -> kind.items.firstOrNull { it.first == value.value }?.first
                        value is Value.Integer && types.numericEnums -> kind.items.firstOrNull { BigInteger.valueOf(it.second) == value.value }?.first
                        else -> null
                    } ?: encodeError("Expected an enumeration value.")

                JsonText.writeString(out, name)
            }
            is Kind.SequenceOf, is Kind.SetOf -> {
                if (value !is Value.List) encodeError("Expected data of type list.")

                val element = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element

                out.append('[')

                value.items.forEachIndexed { index, item ->
                    if (index > 0) out.append(',')

                    inElement(index) { encode(element, item, out, depth + 1) }
                }

                out.append(']')
            }
            is Kind.Utc -> JsonText.writeString(out, utcTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), false))
            is Kind.Generalized ->
                JsonText.writeString(out, generalizedTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), false))
            is Kind.Date -> JsonText.writeString(out, Iso.dateToString((value as? Value.Date ?: encodeError("Expected data of type datetime.date.")).date))
            is Kind.TimeOfDay -> {
                val time = value as? Value.Time ?: encodeError("Expected data of type datetime.time.")

                JsonText.writeString(out, Iso.timeToString(time.time, time.offsetMinutes))
            }
            is Kind.DateTime -> JsonText.writeString(out, Iso.dateTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime.")))
            is Kind.Duration -> {
                val text = (value as? Value.Text ?: encodeError("Expected data of type str.")).value

                validateDuration(text)
                JsonText.writeString(out, text)
            }
            is Kind.Choice -> {
                if (value !is Value.Choice) encodeError("Expected data of type tuple(str, object).")

                val member = kind.members.named(value.name) ?: encodeError("Expected a name of an alternative of the CHOICE.")

                out.append('{')
                JsonText.writeString(out, value.name)
                out.append(':')
                inMember(value.name) { encode(member.type, value.value, out, depth + 1) }
                out.append('}')
            }
            // The value of the type it is (X.697 41), which only an ANY
            // DEFINED BY with choices tells, as a member.
            is Kind.Any, is Kind.AnyDefinedBy, is Kind.OpenType -> unsupported("JER needs the type of the value of an ANY or an open type.")
            is Kind.Ref -> encodeError("The type has no contents of its own.")
            is Kind.Sequence, is Kind.Set, is Kind.External -> encodeMembers(kind.mapMembers!!, value, out, depth)
        }
    }

    private fun writeInteger(
        value: BigInteger,
        out: StringBuilder,
    ) {
        val text = value.toString()

        if (text.length > MAX_DIGITS + 1) encodeError("The integer has too many digits.")

        out.append(text)
    }

    private fun encodeMembers(
        members: Members,
        value: Value,
        out: StringBuilder,
        depth: Int,
    ) {
        if (value !is Value.Map) encodeError("Expected data of type dict.")

        var first = true

        out.append('{')

        for (member in members.all) {
            val found = value.members[member.name]

            if (found == null) {
                if (member.optional || member.hasDefault) continue

                encodeError("A member is not found in the SEQUENCE or SET.")
            }

            if (!first) out.append(',')

            first = false
            JsonText.writeString(out, member.name)
            out.append(':')

            val type = memberType(member, value.members) ?: encodeError("Bad AnyDefinedBy choice.")

            inMember(member.name) { encode(type, found, out, depth + 1) }
        }

        out.append('}')
    }

    /**
     * The type of a member, which for an ANY DEFINED BY with choices is that
     * of the choice of the value of the member it names, if there is one.
     */
    private fun memberType(
        member: MemberInfo,
        siblings: Map<String, Value>,
    ): Int? =
        when (val chosen = types.definedBy(member.type, siblings)) {
            null -> member.type
            Types.NO_CHOICE -> null
            else -> chosen
        }

    private fun error(what: String): Nothing = decodeError(what, 0)

    private fun timeError(): Nothing = error("Expected a date or a time.")

    private inline fun <T> time(block: () -> T): T =
        try {
            block()
        } catch (_: ValueException) {
            timeError()
        }

    private fun string(json: Json): String = (json as? Json.Str)?.value ?: error("Expected a JSON string.")

    private fun decode(
        id: Int,
        json: Json,
        depth: Int,
    ): Value {
        if (depth > MAX_DEPTH) error("The data is nested too deeply.")

        return when (val kind = types.kind(id)) {
            is Kind.Boolean -> Value.Boolean((json as? Json.Bool ?: error("Expected a JSON boolean.")).value)
            is Kind.Integer -> Value.Integer((json as? Json.Integer ?: error("Expected a JSON integer.")).value)
            is Kind.Real ->
                when (json) {
                    is Json.Float -> Value.Real(json.value)
                    is Json.Integer -> Value.Real(Ber.realOfInteger(json.value))
                    is Json.Str ->
                        Value.Real(
                            when (json.value) {
                                "INF" -> Double.POSITIVE_INFINITY
                                "-INF" -> Double.NEGATIVE_INFINITY
                                "NaN" -> Double.NaN
                                "-0" -> -0.0
                                else -> error("Expected a JSON number or INF, -INF, NaN or -0.")
                            },
                        )
                    else -> error("Expected a JSON number.")
                }
            is Kind.Null -> if (json == Json.Null) Value.Null else error("Expected JSON null.")
            is Kind.BitString -> {
                val size = fixedSize(id)
                val (hex, bits) =
                    when {
                        size != null && json is Json.Str -> json.value to size
                        size == null && json is Json.Object -> {
                            val hex = json.members["value"] as? Json.Str
                            val length = json.members["length"] as? Json.Integer

                            if (hex == null || length == null) error("Expected a value and a length of a BIT STRING.")

                            if (length.value.signum() < 0 || length.value.bitLength() > 31) error("Bad length of a BIT STRING.")

                            hex.value to length.value.toInt()
                        }
                        else -> error("Expected a BIT STRING.")
                    }
                val bytes = hexDecode(hex) ?: error("Expected hex.")

                if (bytes.size.toLong() * 8 < bits || bytes.size > (bits + 7L) / 8) error("The length of a BIT STRING is not that of its value.")

                Value.Bits(bytes, bits)
            }
            is Kind.OctetString -> Value.Bytes(hexDecode(string(json)) ?: error("Expected hex."))
            is Kind.ObjectIdentifier ->
                try {
                    Value.ObjectIdentifier.parse(string(json))
                } catch (_: EncodeException) {
                    error("Expected an OBJECT IDENTIFIER.")
                }
            is Kind.Str -> Value.Text(string(json))
            is Kind.Enumerated -> {
                val found = (json as? Json.Str)?.let { name -> kind.items.firstOrNull { it.first == name.value } }

                when {
                    found != null && types.numericEnums -> Value.Integer(found.second)
                    found != null -> Value.Text(found.first)
                    kind.additions != null -> Value.Null
                    else -> error("Expected an enumeration value.")
                }
            }
            is Kind.SequenceOf, is Kind.SetOf -> {
                val element = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element
                val items = (json as? Json.Array ?: error("Expected a JSON array.")).items

                Value.List(items.mapIndexed { index, item -> inElement(index) { decode(element, item, depth + 1) } })
            }
            is Kind.Utc -> time { utcTimeFromString(string(json), false) }
            is Kind.Generalized -> time { generalizedTimeFromString(string(json), false) }
            is Kind.Date -> Value.Date(time { Iso.dateFromString(string(json)) })
            is Kind.TimeOfDay -> Value.Time(time { Iso.timeFromString(string(json)) })
            is Kind.DateTime -> Value.DateTime(time { Iso.dateTimeFromString(string(json)) })
            is Kind.Duration -> {
                val text = string(json)

                try {
                    validateDuration(text)
                } catch (_: EncodeException) {
                    error("Invalid DURATION.")
                }

                Value.Text(text)
            }
            is Kind.Choice -> {
                val (name, inner) = (json as? Json.Object ?: error("Expected a JSON object.")).members.entries.firstOrNull()
                    ?: error("Expected an alternative of the CHOICE.")
                val member = kind.members.named(name)

                when {
                    member != null -> Value.Choice(member.name, inMember(name) { decode(member.type, inner, depth + 1) })
                    kind.members.additions != null -> Value.Choice("", Value.Null)
                    else -> error("Expected a name of an alternative of the CHOICE.")
                }
            }
            is Kind.Any, is Kind.AnyDefinedBy, is Kind.OpenType -> unsupported("JER needs the type of the value of an ANY or an open type.")
            is Kind.Ref -> error("The type has no contents of its own.")
            is Kind.Sequence, is Kind.Set, is Kind.External -> decodeMembers(kind.mapMembers!!, json, depth)
        }
    }

    private fun decodeMembers(
        members: Members,
        json: Json,
        depth: Int,
    ): Value {
        if (json !is Json.Object) error("Expected a JSON object.")

        val map = LinkedHashMap<String, Value>()

        for (member in members.all) {
            val inner = json.members[member.name]

            if (inner != null) {
                val type = memberType(member, map) ?: error("Bad AnyDefinedBy choice.")

                map[member.name] = inMember(member.name) { decode(type, inner, depth + 1) }
            } else if (member.optional) {
                continue
            } else {
                map[member.name] = member.default ?: error("A member is not found in the object.")
            }
        }

        return Value.Map(map)
    }
}
