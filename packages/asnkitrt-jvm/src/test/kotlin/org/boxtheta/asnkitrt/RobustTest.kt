// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.fail
import org.junit.jupiter.api.Test
import java.io.File
import kotlin.random.Random

/**
 * Data made to break the runtime, from what it reads made wrong at
 * random: it is to fail with an [AsnkitException] and nothing else, as
 * an exception of the JVM or an error of a stack too deep.
 */
class RobustTest {
    private val random = Random(20251104)

    private fun mutate(data: ByteArray): ByteArray {
        val out = data.toMutableList()

        repeat(1 + random.nextInt(4)) {
            when (random.nextInt(4)) {
                0 -> if (out.isNotEmpty()) out[random.nextInt(out.size)] = random.nextInt(256).toByte()
                1 -> if (out.isNotEmpty()) out.removeAt(random.nextInt(out.size))
                2 -> out.add(random.nextInt(out.size + 1), random.nextInt(256).toByte())
                // A length, or a count, as large as can be.
                else -> if (out.isNotEmpty()) out[random.nextInt(out.size)] = listOf(0x80, 0x84, 0x9b, 0xbb, 0xff, 0x00).random(random).toByte()
            }
        }

        return out.toByteArray()
    }

    private fun survives(
        what: String,
        data: ByteArray,
        block: (ByteArray) -> Unit,
    ) {
        try {
            block(data)
        } catch (_: AsnkitException) {
        } catch (error: Throwable) {
            fail<Unit>("$what ${data.toHex()}: $error", error)
        }
    }

    @Test
    fun certificates() {
        val fixtures = File(repository, "packages/asnkitrt-rs/tests/fixtures/x509").listFiles { _, name -> name.endsWith(".der") }!!.sorted()
        val asni = File(repository, "examples/compiled/rfc5280.asni")

        for (codec in listOf(Codec.BER, Codec.DER)) {
            val spec = Specification.load(asni, Options(codec))

            for (file in fixtures) {
                val data = file.readBytes()

                repeat(300) {
                    survives("$codec ${file.name}", mutate(data)) { mutated ->
                        val decoded = spec.decode("Certificate", mutated)

                        // What DER decodes is not always the one encoding of
                        // its value: as the runtime for Rust does, and the
                        // vectors have, the members of a SEQUENCE may be
                        // followed by data that is not read.
                        spec.checkConstraints("Certificate", decoded)
                        spec.encode("Certificate", decoded)
                    }
                }
            }
        }
    }

    /** What a text codec encodes in the vectors, of every type there, made wrong. */
    private fun text(codec: Codec) {
        val mapper = com.fasterxml.jackson.databind.ObjectMapper()
        val specs = HashMap<String, Specification>()
        val name = codec.name.lowercase()
        val seeds =
            java.util.zip
                .GZIPInputStream(File(repository, "tests/vectors/vectors.jsonl.gz").inputStream())
                .bufferedReader()
                .lineSequence()
                .map { mapper.readTree(it) }
                .filter { it["codec"].textValue() == name && it["error"].isNull && !it["numeric_enums"].booleanValue() }
                .toList()

        for (seed in seeds) {
            val spec =
                specs.getOrPut(seed["spec"].textValue()) {
                    Specification.load(File(repository, "tests/vectors/specs/${seed["spec"].textValue()}.asni"), Options(codec))
                }
            val text = hex(seed["encoded"].textValue())

            repeat(20) { survives("$codec ${seed["type"]}", mutate(text)) { spec.decode(seed["type"].textValue(), it) } }
        }
    }

    @Test
    fun jer() = text(Codec.JER)

    @Test
    fun xer() = text(Codec.XER)

    @Test
    fun specifications() {
        for (file in File(repository, "examples/compiled").listFiles { _, name -> name.endsWith(".asni") }!!.sorted()) {
            val data = file.readBytes()

            repeat(500) {
                val mutated = mutate(data.copyOfRange(Asni.HEADER_SIZE, data.size))
                val crc = Asni.crc32(mutated)
                // With the CRC-32 made to match, so that the CBOR and the
                // nodes are read.
                val sealed = data.copyOf(Asni.HEADER_SIZE).also { for (index in 0 until 4) it[6 + index] = (crc ushr (24 - 8 * index)).toByte() } + mutated

                survives(file.name, sealed) { asni ->
                    for (codec in Codec.entries) Specification.fromAsni(asni, Options(codec))
                }
            }
        }
    }
}
