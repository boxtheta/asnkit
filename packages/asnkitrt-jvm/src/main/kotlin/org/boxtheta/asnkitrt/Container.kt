// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.util.zip.CRC32

/** An ASN.1 file a compiled specification was made from. */
public data class Source(
    public val name: String? = null,
    /** The SHA-256 digest of its text, as 64 lowercase hexadecimal digits. */
    public val sha256: String? = null,
    /** Its text, which an `.asni` file never has. */
    public val text: String? = null,
)

/** What a compiled specification tells of itself. */
public data class Metadata(
    /** What wrote the file, and its version. */
    public val generator: String? = null,
    public val sources: List<Source> = emptyList(),
    /**
     * Anything else those making the file wanted it to tell, as maps,
     * lists, strings, `BigInteger`s, doubles, booleans and null.
     */
    public val other: Any? = null,
)

/** The header of an `.asni` file: its version, as the major and the minor, and its metadata. */
public data class Header(
    public val majorVersion: Int,
    public val minorVersion: Int,
    public val metadata: Metadata,
)

/** What a file has to be to be read. */
public data class Checks(
    /** How deep its CBOR may be nested, the outermost array or map counted as 1. */
    public val maxDepth: Int = CborReader.DEFAULT_MAX_DEPTH,
)

/**
 * Reads `.asni` files: the header, the CRC-32 of the rest of the file,
 * the metadata and the nodes, in the order of `docs/file_formats.md`.
 */
public object Asni {
    /** The magic, the version and the CRC-32 of the rest of the file. */
    public const val HEADER_SIZE: Int = 10

    /** The major version of the files this runtime reads. */
    public const val MAJOR_VERSION: Int = 2

    private val MAGIC = "ASNI".toByteArray(Charsets.US_ASCII)

    /** The CRC-32 of `data`, as zlib, gzip and PNG compute it: that of `123456789` is `0xcbf43926`. */
    public fun crc32(
        data: ByteArray,
        offset: Int = 0,
        length: Int = data.size - offset,
    ): Long = CRC32().apply { update(data, offset, length) }.value

    /** Checks the header of `data` and the CRC-32 of the rest, and returns the version. */
    private fun readPrefix(data: ByteArray): Pair<Int, Int> {
        if (data.size < HEADER_SIZE || !data.copyOfRange(0, 4).contentEquals(MAGIC)) {
            throw AsniException(AsniException.Reason.NOT_ASNI)
        }

        if (data[4].toInt() != MAJOR_VERSION) throw AsniException(AsniException.Reason.UNSUPPORTED_VERSION)

        val expected = (6 until 10).fold(0L) { value, index -> (value shl 8) or (data[index].toLong() and 0xff) }

        if (crc32(data, HEADER_SIZE) != expected) throw AsniException(AsniException.Reason.BAD_CHECKSUM)

        return (data[4].toInt() and 0xff) to (data[5].toInt() and 0xff)
    }

    private fun badMetadata(): Nothing = throw AsniException(AsniException.Reason.BAD_METADATA)

    private fun readText(
        map: Cbor.Map,
        key: String,
    ): String? =
        when (val value = map[key]) {
            null -> null
            is Cbor.Text -> value.value
            else -> badMetadata()
        }

    private fun readMetadata(item: Cbor): Metadata {
        if (item !is Cbor.Map) badMetadata()

        val sources =
            when (val sources = item["sources"]) {
                null -> emptyList()
                is Cbor.Array ->
                    sources.items.map { source ->
                        if (source !is Cbor.Map) badMetadata()

                        Source(readText(source, "name"), readText(source, "sha256"), readText(source, "text"))
                    }
                else -> badMetadata()
            }

        // Anything not known is that of a later minor version.
        return Metadata(readText(item, "generator"), sources, item["other"]?.toPlain())
    }

    /** The header of `data`, after its version and the CRC-32 of the file are checked. The nodes are not read. */
    public fun readHeader(
        data: ByteArray,
        checks: Checks = Checks(),
    ): Header {
        val (major, minor) = readPrefix(data)
        val (metadata, _) = CborReader.parseFirst(data, HEADER_SIZE, checks.maxDepth)

        return Header(major, minor, readMetadata(metadata))
    }

    /** Reads the file in `data` after the checks, and returns its header and its nodes. */
    internal fun read(
        data: ByteArray,
        checks: Checks,
    ): Pair<Header, Cbor> {
        val (major, minor) = readPrefix(data)
        val (item, offset) = CborReader.parseFirst(data, HEADER_SIZE, checks.maxDepth)
        val metadata = readMetadata(item)
        val (nodes, end) = CborReader.parseFirst(data, offset, checks.maxDepth)

        if (end != data.size) throw AsniException(AsniException.Reason.TRAILING_DATA)

        return Header(major, minor, metadata) to nodes
    }
}
