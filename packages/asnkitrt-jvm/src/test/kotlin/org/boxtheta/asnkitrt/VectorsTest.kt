// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import com.fasterxml.jackson.databind.DeserializationFeature
import com.fasterxml.jackson.databind.JsonNode
import com.fasterxml.jackson.databind.ObjectMapper
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import java.io.File
import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime
import java.util.zip.GZIPInputStream

/** The root of the repository, of which the tests read the vectors and the examples. */
internal val repository: File = File(System.getProperty("asnkit.repository") ?: "../..")

internal fun hex(text: String): ByteArray = ByteArray(text.length / 2) { text.substring(2 * it, 2 * it + 2).toInt(16).toByte() }

/**
 * Holds the runtime to the vectors in `tests/vectors`, which are what the
 * runtime for Rust, the one of reference, encodes and decodes. Those of
 * the codecs this runtime has are all held to: `tests/vectors/README.rst`
 * tells how a vector is written.
 *
 * `VECTORS_CODEC=ber` runs the vectors of one codec, and `VECTORS_VERBOSE=1`
 * tells each one that fails.
 */
class VectorsTest {
    private val mapper = ObjectMapper().enable(DeserializationFeature.USE_BIG_INTEGER_FOR_INTS)

    private data class SpecKey(
        val spec: String,
        val codec: String,
        val numericEnums: Boolean,
        val lenient: Boolean,
    )

    private fun load(key: SpecKey): Result<Specification> =
        runCatching {
            val codec = Codec.valueOf(key.codec.uppercase())

            Specification.load(File(repository, "tests/vectors/specs/${key.spec}.asni"), Options(codec, key.numericEnums, key.lenient))
        }

    private fun parseDate(text: String): LocalDate = LocalDate.of(text.substring(0, 4).toInt(), text.substring(5, 7).toInt(), text.substring(8, 10).toInt())

    /** A time as `time.isoformat()` of Python writes it, and its offset in minutes. */
    private fun parseTime(text: String): Pair<LocalTime, Int?> {
        var rest = text.substring(8)
        var microsecond = 0

        if (rest.startsWith('.')) {
            val digits = rest.substring(1).takeWhile { it.isDigit() }

            microsecond = digits.toInt()
            rest = rest.substring(1 + digits.length)
        }

        val offset =
            if (rest.length >= 6) {
                (if (rest[0] == '-') -1 else 1) * (rest.substring(1, 3).toInt() * 60 + rest.substring(4, 6).toInt())
            } else {
                null
            }

        return LocalTime.of(text.substring(0, 2).toInt(), text.substring(3, 5).toInt(), text.substring(6, 8).toInt(), microsecond * 1000) to offset
    }

    /** A value as `tests/vectors/values.py` writes it. */
    private fun toValue(json: JsonNode): Value {
        when {
            json.isNull -> return Value.Null
            json.isBoolean -> return Value.Boolean(json.booleanValue())
            json.isFloatingPointNumber -> return Value.Real(json.doubleValue())
            json.isIntegralNumber -> return Value.Integer(json.bigIntegerValue())
            json.isTextual -> return Value.Text(json.textValue())
            json.isArray -> return Value.List(json.map(::toValue))
        }

        if (json.size() == 1) {
            val (key, item) = json.properties().first()

            when (key) {
                "\$bytes" -> return Value.Bytes(hex(item.textValue()))
                "\$bits" -> return Value.Bits(hex(item[0].textValue()), item[1].intValue())
                "\$choice" -> return Value.Choice(item[0].textValue(), toValue(item[1]))
                "\$float" ->
                    return Value.Real(
                        when (item.textValue()) {
                            "inf" -> Double.POSITIVE_INFINITY
                            "-inf" -> Double.NEGATIVE_INFINITY
                            "-0.0" -> -0.0
                            else -> Double.NaN
                        },
                    )
                "\$datetime" -> {
                    val text = item.textValue()
                    val (time, offset) = parseTime(text.substring(11))

                    return Value.DateTime(LocalDateTime.of(parseDate(text), time), offset)
                }
                "\$date" -> return Value.Date(parseDate(item.textValue()))
                "\$time" -> {
                    val (time, offset) = parseTime(item.textValue())

                    return Value.Time(time, offset)
                }
            }
        }

        return Value.Map(json.properties().associateTo(LinkedHashMap()) { (key, item) -> key to toValue(item) })
    }

    /** The value of `json` for the type `name`, in which the text of an OBJECT IDENTIFIER is one, as the JSON has it as text. */
    private fun valueOf(
        spec: Specification,
        name: String,
        json: JsonNode,
    ): Value {
        val value = toValue(json)
        val id = spec.types.findUnique(name) ?: return value

        return typeObjectIdentifiers(spec.types, id, value, 0)
    }

    private fun typeObjectIdentifiers(
        types: Types,
        id: Int,
        value: Value,
        depth: Int,
    ): Value {
        // As deep as the codecs go.
        if (depth > 128) return value

        val kind = types.kind(id)

        return when {
            kind is Kind.ObjectIdentifier && value is Value.Text ->
                runCatching { Value.ObjectIdentifier.parse(value.value) }.getOrDefault(value)
            (kind is Kind.SequenceOf || kind is Kind.SetOf) && value is Value.List -> {
                val element = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element

                Value.List(value.items.map { typeObjectIdentifiers(types, element, it, depth + 1) })
            }
            kind is Kind.Choice && value is Value.Choice ->
                kind.members.named(value.name)?.let { Value.Choice(value.name, typeObjectIdentifiers(types, it.type, value.value, depth + 1)) } ?: value
            kind.mapMembers != null && value is Value.Map -> {
                val members = kind.mapMembers!!
                // The members an ANY DEFINED BY is defined by first, as the
                // choice is found by their value.
                val map =
                    value.members.mapValuesTo(LinkedHashMap()) { (name, member) ->
                        val type = members.named(name)?.type

                        if (type != null && types.kind(type) !is Kind.AnyDefinedBy) typeObjectIdentifiers(types, type, member, depth + 1) else member
                    }

                for ((name, member) in map.entries.toList()) {
                    val type = members.named(name)?.type ?: continue
                    val chosen = types.definedBy(type, map) ?: continue

                    if (chosen != Types.NO_CHOICE) map[name] = typeObjectIdentifiers(types, chosen, member, depth + 1)
                }

                Value.Map(map)
            }
            else -> value
        }
    }

    /** If two values are the same, whatever order the members of a map are in and if a number is not a number. */
    private fun same(
        a: Value,
        b: Value,
    ): Boolean =
        when {
            a is Value.Real && b is Value.Real -> (a.value.isNaN() && b.value.isNaN()) || a.value == b.value
            a is Value.Real && b is Value.Integer -> a.value == b.value.toDouble()
            a is Value.Integer && b is Value.Real -> b.value == a.value.toDouble()
            a is Value.List && b is Value.List -> a.items.size == b.items.size && a.items.zip(b.items).all { (x, y) -> same(x, y) }
            a is Value.Map && b is Value.Map ->
                a.members.size == b.members.size && a.members.all { (name, x) -> b.members[name]?.let { same(x, it) } ?: false }
            a is Value.Choice && b is Value.Choice -> a.name == b.name && same(a.value, b.value)
            else -> a == b
        }

    /** What an operation of a vector gives: the bytes it encoded, or the value it decoded. */
    private fun perform(
        spec: Specification,
        vector: JsonNode,
    ): Any {
        val name = vector["type"].textValue()
        val options = vector["options"]
        val check = options["check_constraints"]?.booleanValue() == true
        val indent = options["indent"]?.intValue()

        if (vector["operation"].textValue() == "encode") {
            val value = valueOf(spec, name, vector["value"])

            if (check) spec.checkConstraints(name, value)

            return if (indent != null) spec.encodeIndented(name, value, indent) else spec.encode(name, value)
        }

        val decoded = spec.decode(name, hex(vector["encoded"].textValue()))

        if (check) spec.checkConstraints(name, decoded)

        return decoded
    }

    @Test
    fun vectors() {
        val only = System.getenv("VECTORS_CODEC")
        val verbose = System.getenv("VECTORS_VERBOSE") != null
        val codecs = Codec.entries.map { it.name.lowercase() }
        val specs = HashMap<SpecKey, Result<Specification>>()
        val passed = sortedMapOf<String, Int>()
        val skipped = sortedMapOf<String, Int>()
        val failures = ArrayList<String>()
        val lines = GZIPInputStream(File(repository, "tests/vectors/vectors.jsonl.gz").inputStream()).bufferedReader().readLines()

        for (line in lines) {
            val vector = mapper.readTree(line)
            val key = SpecKey(vector["spec"].textValue(), vector["codec"].textValue(), vector["numeric_enums"].booleanValue(), vector["lenient"].booleanValue())

            if (key.codec !in codecs || (only != null && only != key.codec)) continue

            val describe = "${key.codec} ${vector["operation"]} ${key.spec} ${vector["type"]}: ${vector["value"]}"
            val expectedError = vector["error"].textValue()
            val spec =
                specs.getOrPut(key) { load(key) }.getOrElse {
                    failures.add("not loaded: ${it.message} <= $describe")
                    continue
                }
            val outcome = runCatching { perform(spec, vector) }
            val failure =
                when {
                    outcome.exceptionOrNull() is UnsupportedException && expectedError == null -> {
                        skipped.merge("${key.codec} unsupported", 1, Int::plus)
                        continue
                    }
                    expectedError != null ->
                        outcome.fold(
                            { "${if (it is ByteArray) "encoded ${it.toHex()}" else "decoded $it"} but expected $expectedError" },
                            { null },
                        )
                    outcome.isFailure -> "failed: ${outcome.exceptionOrNull()}"
                    else ->
                        when (val done = outcome.getOrThrow()) {
                            is ByteArray -> {
                                val encoded = vector["encoded"].textValue()

                                if (done.toHex() == encoded) null else "encoded ${done.toHex()} instead of $encoded"
                            }
                            else -> {
                                val expected = valueOf(spec, vector["type"].textValue(), vector["value"])

                                if (same(done as Value, expected)) null else "decoded $done"
                            }
                        }
                }

            if (failure == null) passed.merge(key.codec, 1, Int::plus) else failures.add("${failure.take(160)} <= $describe")
        }

        println("passed: $passed")
        println("skipped: $skipped")
        println("failed: ${failures.size}")

        if (verbose) failures.take(60).forEach { println(it.take(300)) }

        assertTrue(passed.values.sum() > 2000, "Expected the vectors of BER, DER and JER, found $passed.")
        assertTrue(failures.isEmpty(), "${failures.size} vectors failed, see VECTORS_VERBOSE")
    }
}
