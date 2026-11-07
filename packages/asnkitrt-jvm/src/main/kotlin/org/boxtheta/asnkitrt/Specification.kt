// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.io.File
import java.nio.file.Files
import java.nio.file.Path

/** How a specification is loaded. */
public data class Options(
    public val codec: Codec = Codec.BER,
    /** ENUMERATED values are numbers and not names. */
    public val numericEnums: Boolean = false,
    /** DER decodes what BER allows. Only DER looks at it. */
    public val lenient: Boolean = false,
)

/**
 * A compiled specification, an `.asni` file, loaded for a codec.
 *
 * ```kotlin
 * val foo = Specification.fromAsni(File("foo.asni").readBytes(), Options(Codec.DER))
 * val question = Value.Map("id" to Value.Integer(1), "question" to Value.Text("Is 1+1=3?"))
 * val encoded = foo.encode("Question", question)
 * val decoded = foo.decode("Question", encoded)
 * ```
 *
 * A type is named by its name alone if it is found in one module only, or
 * else with its module too. It is thread-safe: nothing of it changes after
 * it is loaded.
 */
public class Specification private constructor(
    internal val types: Types,
    /** The header of the file it was loaded from. */
    public val header: Header,
) {
    /** The codec it was loaded for. */
    public val codec: Codec get() = types.codec

    /** The names of the modules, each with the names of its types, in the order of the file. */
    public val modules: Map<String, List<String>>
        get() = types.modules.associate { (module, names) -> module to names.map { it.first } }

    private fun find(name: String): Int = types.findUnique(name) ?: throw NoSuchTypeException(name)

    private fun find(
        module: String,
        name: String,
    ): Int = types.find(module, name) ?: throw NoSuchTypeException("$module.$name")

    /** Checks `value` against the constraints of the type `type`, and throws a [ConstraintsException] if it does not fulfil them. */
    public fun checkConstraints(
        type: String,
        value: Value,
    ): Unit = checkConstraints(find(type), type, value)

    /** Checks `value` against the constraints of the type `type` of the module `module`. */
    public fun checkConstraints(
        module: String,
        type: String,
        value: Value,
    ): Unit = checkConstraints(find(module, type), type, value)

    private fun checkConstraints(
        id: Int,
        name: String,
        value: Value,
    ) = within(Step.Name(name)) { Check.check(types, id, value) }

    /** Encodes `value` as the type `type`. */
    public fun encode(
        type: String,
        value: Value,
    ): ByteArray = encode(find(type), type, value, null)

    /** Encodes `value` as the type `type` of the module `module`. */
    public fun encode(
        module: String,
        type: String,
        value: Value,
    ): ByteArray = encode(find(module, type), type, value, null)

    /**
     * Encodes `value` as the type `type` in JER or XER, with `indent` spaces
     * for each level, as the runtime for Python does. The other codecs are
     * not indented, and throw an [UnsupportedException].
     */
    public fun encodeIndented(
        type: String,
        value: Value,
        indent: Int,
    ): ByteArray = encode(find(type), type, value, indent)

    private fun encode(
        id: Int,
        name: String,
        value: Value,
        indent: Int?,
    ): ByteArray {
        if (indent != null && codec != Codec.JER && codec != Codec.XER) unsupported("Only JER and XER are indented.")

        return within(Step.Name(name)) {
            when (codec) {
                Codec.BER, Codec.DER -> Ber.encode(types, id, value)
                Codec.JER -> Jer(types).encode(id, value, indent)
                Codec.XER -> Xer(types).encode(id, value, indent)
            }
        }
    }

    /** Decodes `data` as the type `type`, which is all of it. */
    public fun decode(
        type: String,
        data: ByteArray,
    ): Value = decode(find(type), type, data)

    /** Decodes `data` as the type `type` of the module `module`. */
    public fun decode(
        module: String,
        type: String,
        data: ByteArray,
    ): Value = decode(find(module, type), type, data)

    private fun decode(
        id: Int,
        name: String,
        data: ByteArray,
    ): Value =
        within(Step.Name(name)) {
            when (codec) {
                Codec.BER, Codec.DER -> Ber.decode(types, id, data)
                Codec.JER -> Jer(types).decode(id, data)
                Codec.XER -> Xer(types).decode(id, data)
            }
        }

    /**
     * Decodes the first value of `data` as the type `type`, and returns it
     * with the number of bytes it was. Only BER and DER have values one after
     * another.
     */
    public fun decodeWithLength(
        type: String,
        data: ByteArray,
    ): Pair<Value, Int> {
        val id = find(type)

        if (codec != Codec.BER && codec != Codec.DER) unsupported("Only BER and DER have values one after another.")

        val decoded = within(Step.Name(type)) { Ber.decodeWithLength(types, id, data) }

        return decoded.value to decoded.end
    }

    /**
     * The length of the first value of `data`, if its tag and length are
     * there, also if the rest is not yet, as when it is read from a stream.
     * `null` if it is not known, as of one of an indefinite length. Only BER
     * and DER have it.
     */
    public fun decodeLength(data: ByteArray): Int? {
        if (codec != Codec.BER && codec != Codec.DER) unsupported("This codec has no length in the data.")

        return Ber.decodeLength(data)
    }

    public companion object {
        /** Loads the contents of an `.asni` file, after `checks`. */
        @JvmStatic
        @JvmOverloads
        public fun fromAsni(
            data: ByteArray,
            options: Options = Options(),
            checks: Checks = Checks(),
        ): Specification {
            val (header, nodes) = Asni.read(data, checks)

            return Specification(lower(Spec.fromNodes(nodes), options.codec, options.numericEnums, options.lenient), header)
        }

        /** Loads an `.asni` file. */
        @JvmStatic
        @JvmOverloads
        public fun load(
            path: Path,
            options: Options = Options(),
            checks: Checks = Checks(),
        ): Specification = fromAsni(Files.readAllBytes(path), options, checks)

        /** Loads an `.asni` file. */
        @JvmStatic
        @JvmOverloads
        public fun load(
            file: File,
            options: Options = Options(),
            checks: Checks = Checks(),
        ): Specification = load(file.toPath(), options, checks)
    }
}
