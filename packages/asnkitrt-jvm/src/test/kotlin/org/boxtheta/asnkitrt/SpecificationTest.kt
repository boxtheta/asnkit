// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertArrayEquals
import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.io.File

/** The public API, with the compiled examples of the repository. */
class SpecificationTest {
    private val foo = File(repository, "examples/compiled/foo.asni")
    private val question = Value.Map("id" to Value.Integer(1), "question" to Value.Text("Is 1+1=3?"))

    @Test
    fun theExampleOfTheReadme() {
        for (codec in listOf(Codec.BER, Codec.DER)) {
            val spec = Specification.load(foo, Options(codec))
            val encoded = spec.encode("Question", question)

            assertArrayEquals(byteArrayOf(0x30, 0x0e, 0x02, 0x01, 0x01, 0x16, 0x09) + "Is 1+1=3?".toByteArray(), encoded)
            assertEquals(question, spec.decode("Question", encoded))
            assertEquals(question, spec.decode("Foo", "Question", encoded))
            assertEquals(question to encoded.size, spec.decodeWithLength("Question", encoded + byteArrayOf(0x30, 0x00)))
            assertEquals(encoded.size, spec.decodeLength(encoded.copyOf(3)))
        }
    }

    @Test
    fun jer() {
        val spec = Specification.load(foo, Options(Codec.JER))

        assertEquals("{\"id\":1,\"question\":\"Is 1+1=3?\"}", String(spec.encode("Question", question)))
        assertEquals("{\n  \"id\": 1,\n  \"question\": \"Is 1+1=3?\"\n}", String(spec.encodeIndented("Question", question, 2)))
        assertEquals(question, spec.decode("Question", "{\"question\":\"Is 1+1=3?\",\"id\":1}".toByteArray()))
        assertThrows<UnsupportedException> { Specification.load(foo).encodeIndented("Question", question, 2) }
    }

    @Test
    fun errorsTellWhereInTheValueTheyAre() {
        val spec = Specification.load(foo, Options(Codec.DER))
        val error = assertThrows<EncodeException> { spec.encode("Question", Value.Map("id" to Value.Integer(1), "question" to Value.Integer(2))) }

        assertEquals(listOf(Step.Name("Question"), Step.Name("question")), error.location)
        assertEquals("Question.question: Expected data of type str.", error.message)

        val decode = assertThrows<DecodeException> { spec.decode("Question", hex("300302018001")) }

        assertEquals("Question", decode.path)
        assertThrows<NoSuchTypeException> { spec.encode("Nothing", question) }
        // A member is missing.
        assertThrows<EncodeException> { spec.encode("Question", Value.Map("id" to Value.Integer(1))) }
    }

    @Test
    fun aCertificateOfRfc5280() {
        // The bytes of one of the certificates the Rust runtime checks against OpenSSL.
        val fixtures = File(repository, "packages/asnkitrt-rs/tests/fixtures/x509")
        val spec = Specification.load(File(repository, "examples/compiled/rfc5280.asni"), Options(Codec.DER))

        for (file in fixtures.listFiles { _, name -> name.endsWith(".der") }!!.sorted()) {
            val data = file.readBytes()
            val certificate = spec.decode("Certificate", data)

            spec.checkConstraints("Certificate", certificate)
            assertArrayEquals(data, spec.encode("Certificate", certificate), file.name)
        }
    }
}
