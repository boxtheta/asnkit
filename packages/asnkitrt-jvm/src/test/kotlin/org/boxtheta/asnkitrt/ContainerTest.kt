// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.io.File

/** The container, as `docs/third_party_runtimes.md` tells a runtime to be tested. */
class ContainerTest {
    private val plain = File(repository, "examples/corpus/plain.asni").readBytes()

    /** The file with the CRC-32 of its header made that of the rest of it. */
    private fun sealed(data: ByteArray): ByteArray {
        val crc = Asni.crc32(data, Asni.HEADER_SIZE)

        return data.copyOf().also { for (index in 0 until 4) it[6 + index] = (crc ushr (24 - 8 * index)).toByte() }
    }

    private fun reason(data: ByteArray): AsniException.Reason = assertThrows<AsniException> { Specification.fromAsni(data) }.reason

    @Test
    fun crc32IsThatOfZlib() {
        assertEquals(0L, Asni.crc32(ByteArray(0)))
        assertEquals(0xcbf43926L, Asni.crc32("123456789".toByteArray()))
        assertEquals(0x414fa339L, Asni.crc32("The quick brown fox jumps over the lazy dog".toByteArray()))
    }

    @Test
    fun theCorpus() {
        val spec = Specification.fromAsni(plain)

        assertEquals(mapOf("Foo" to listOf("Question", "Answer")), spec.modules)
        assertEquals(2, spec.header.majorVersion)
        assertEquals(AsniException.Reason.BAD_CHECKSUM, reason(File(repository, "examples/corpus/corrupt.asni").readBytes()))
    }

    @Test
    fun theHeader() {
        val header = Asni.readHeader(plain)

        assertEquals(2 to 0, header.majorVersion to header.minorVersion)
        assertTrue(header.metadata.generator!!.startsWith("asnkit"), header.metadata.generator)
        assertEquals(listOf("foo.asn"), header.metadata.sources.map { it.name })
        assertEquals(null, header.metadata.sources[0].text)
    }

    @Test
    fun rejected() {
        assertEquals(AsniException.Reason.NOT_ASNI, reason(ByteArray(0)))
        assertEquals(AsniException.Reason.NOT_ASNI, reason(plain.copyOf(9)))
        assertEquals(AsniException.Reason.NOT_ASNI, reason("ASNX".toByteArray() + plain.copyOfRange(4, plain.size)))
        // Version 1, and one to come.
        assertEquals(AsniException.Reason.UNSUPPORTED_VERSION, reason(plain.copyOf().also { it[4] = 1 }))
        assertEquals(AsniException.Reason.UNSUPPORTED_VERSION, reason(plain.copyOf().also { it[4] = 3 }))
        // Cut short, or with a byte added, with the CRC-32 as it was.
        assertEquals(AsniException.Reason.BAD_CHECKSUM, reason(plain.copyOf(plain.size - 1)))
        assertEquals(AsniException.Reason.BAD_CHECKSUM, reason(plain + byteArrayOf(0)))
        // With the CRC-32 made to match: trailing bytes, truncated CBOR and
        // malformed metadata.
        assertEquals(AsniException.Reason.TRAILING_DATA, reason(sealed(plain + byteArrayOf(0))))
        assertEquals(AsniException.Reason.OUT_OF_DATA, reason(sealed(plain.copyOf(plain.size - 1))))
        assertEquals(AsniException.Reason.BAD_METADATA, reason(sealed(plain.copyOf(10) + hex("80") + plain.copyOfRange(10, plain.size))))
        assertEquals(AsniException.Reason.BAD_METADATA, reason(sealed(plain.copyOf(10) + hex("a1696765" + "6e657261746f7201"))))
        // Nodes that are not those of a compiled specification.
        assertEquals(AsniException.Reason.BAD_NODES, reason(sealed(plain.copyOf(10) + hex("a0" + "820500"))))
        // A minor version to come is read.
        Specification.fromAsni(plain.copyOf().also { it[5] = 9 })
    }

    @Test
    fun theDepthOfTheCbor() {
        assertEquals(AsniException.Reason.TOO_DEEP, assertThrows<AsniException> { Specification.fromAsni(plain, checks = Checks(maxDepth = 3)) }.reason)
    }
}
