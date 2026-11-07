// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import com.ctc.wstx.api.WstxInputProperties
import com.ctc.wstx.stax.WstxInputFactory
import java.io.StringReader
import java.math.BigInteger
import java.nio.ByteBuffer
import java.nio.charset.CharacterCodingException
import java.nio.charset.CodingErrorAction
import javax.xml.stream.XMLInputFactory
import javax.xml.stream.XMLStreamConstants
import javax.xml.stream.XMLStreamException
import javax.xml.stream.XMLStreamReader

// XML Encoding Rules (XER), X.693.
//
// A value is an element named after its type or its member. A SEQUENCE and
// a SET have an element for each member, a CHOICE one for the alternative, a
// SEQUENCE OF and a SET OF one for each item, named after the type of the
// items. A BOOLEAN is `<true />` or `<false />` and an ENUMERATED the element
// of its value, in an element. What is written is the text of
// `ElementTree.tostring` of the runtime for Python, in ASCII.

private const val MAX_DEPTH = 128

/** An element of XML, as far as XER needs it. */
internal class XmlElement(
    val name: String,
) {
    /** The text before the first child, `null` if there is none. */
    var text: String? = null

    val children: ArrayList<XmlElement> = ArrayList()

    /** The text after the element, before the next one, in the element it is in. */
    var tail: String? = null

    fun child(name: String): XmlElement? = children.firstOrNull { it.name == name }

    val isEmpty: Boolean get() = text == null && children.isEmpty()

    companion object {
        /** An element to write, of which a space in the name is `_`. */
        fun of(
            name: String,
            text: String? = null,
        ): XmlElement = XmlElement(name.replace(' ', '_')).also { if (!text.isNullOrEmpty()) it.text = text }
    }
}

/**
 * XML, read with Woodstox and written as `ElementTree.tostring` of Python
 * writes it, in ASCII.
 *
 * A document type declaration is refused, so there are no entities to grow
 * and none that are fetched, and how deep elements are and how many events
 * there are is limited. Attributes and namespaces are dropped: a name is the
 * local name of the element, what comes after its first `:`.
 */
internal object XmlText {
    /** How many events a document may have. */
    private const val MAX_EVENTS = 4_000_000

    /**
     * The names of the control characters in an `xmlcstring`, by their code
     * (X.680 12.15.5, Table 3). Tab, line feed and carriage return are written
     * as they are.
     */
    private val CONTROLS =
        arrayOf(
            "nul", "soh", "stx", "etx", "eot", "enq", "ack", "bel", "bs", "", "", "vt", "ff", "", "so", "si",
            "dle", "dc1", "dc2", "dc3", "dc4", "nak", "syn", "etb", "can", "em", "sub", "esc", "is4", "is3", "is2", "is1",
        )

    /** The control character an empty element in an `xmlcstring` stands for, as `<bel/>` (X.680 12.15.5). */
    fun control(name: String): Char? {
        val code = CONTROLS.indexOf(name)

        return if (name.isEmpty() || code < 0) null else code.toChar()
    }

    // A factory of Woodstox, and not whatever `XMLInputFactory.newInstance()`
    // finds, is thread-safe once it is set up.
    private val factory: XMLInputFactory =
        WstxInputFactory().apply {
            setProperty(XMLInputFactory.SUPPORT_DTD, false)
            setProperty(XMLInputFactory.IS_SUPPORTING_EXTERNAL_ENTITIES, false)
            setProperty(XMLInputFactory.IS_NAMESPACE_AWARE, false)
            setProperty(XMLInputFactory.IS_COALESCING, true)
            setProperty(XMLInputFactory.IS_REPLACING_ENTITY_REFERENCES, true)
            setProperty(WstxInputProperties.P_MAX_ELEMENT_DEPTH, MAX_DEPTH + 1)
        }

    private fun bad(): Nothing = decodeError("Bad XML.", 0)

    private fun isXmlChar(code: Int): Boolean =
        code == 0x9 || code == 0xa || code == 0xd || code in 0x20..0xd7ff || code in 0xe000..0xfffd || code in 0x10000..0x10ffff

    /** Reads the document in `data`, which is UTF-8. */
    fun parse(data: ByteArray): XmlElement {
        val text =
            try {
                Charsets.UTF_8
                    .newDecoder()
                    .onMalformedInput(CodingErrorAction.REPORT)
                    .onUnmappableCharacter(CodingErrorAction.REPORT)
                    .decode(ByteBuffer.wrap(data))
                    .toString()
            } catch (_: CharacterCodingException) {
                decodeError("Expected UTF-8.", 0)
            }

        return try {
            val reader = factory.createXMLStreamReader(StringReader(text))

            try {
                read(reader)
            } finally {
                reader.close()
            }
        } catch (error: AsnkitException) {
            throw error
        } catch (_: XMLStreamException) {
            bad()
        } catch (_: RuntimeException) {
            // Woodstox throws some of its errors unchecked, as it reads text
            // only when it is asked for.
            bad()
        }
    }

    private fun read(reader: XMLStreamReader): XmlElement {
        val stack = ArrayList<XmlElement>()
        var root: XmlElement? = null
        var events = 0

        // Text goes to the element that is open, or after its last child.
        fun add(more: String) {
            var index = 0

            while (index < more.length) {
                val code = more.codePointAt(index)

                if (!isXmlChar(code)) bad()

                index += Character.charCount(code)
            }

            val open = stack.lastOrNull()

            if (open == null) {
                // Between the elements and around the root, only space.
                if (more.any { it != ' ' && it != '\t' && it != '\r' && it != '\n' }) bad()

                return
            }

            if (more.isEmpty()) return

            val last = open.children.lastOrNull()

            if (last != null) last.tail = (last.tail ?: "") + more else open.text = (open.text ?: "") + more
        }

        while (true) {
            events += 1

            if (events > MAX_EVENTS) bad()

            when (reader.next()) {
                XMLStreamConstants.START_ELEMENT -> {
                    if (root != null || stack.size >= MAX_DEPTH) bad()

                    stack.add(XmlElement(reader.localName.substringAfter(':')))
                }
                XMLStreamConstants.END_ELEMENT -> {
                    val done = stack.removeLastOrNull() ?: bad()
                    val parent = stack.lastOrNull()

                    if (parent != null) parent.children.add(done) else root = done
                }
                XMLStreamConstants.CHARACTERS, XMLStreamConstants.CDATA, XMLStreamConstants.SPACE -> add(reader.text)
                // A document type declaration, and so any entity but the five XML has.
                XMLStreamConstants.DTD, XMLStreamConstants.ENTITY_REFERENCE, XMLStreamConstants.ENTITY_DECLARATION -> bad()
                XMLStreamConstants.END_DOCUMENT -> break
                // An XML declaration, a comment and a processing instruction.
                else -> {}
            }
        }

        return if (stack.isEmpty()) root ?: bad() else bad()
    }

    private fun writeText(
        out: StringBuilder,
        text: String,
    ) {
        var index = 0

        while (index < text.length) {
            val code = text.codePointAt(index)

            index += Character.charCount(code)

            when {
                code == '&'.code -> out.append("&amp;")
                code == '<'.code -> out.append("&lt;")
                code == '>'.code -> out.append("&gt;")
                // Written as it is, a reader would make it a line feed.
                code == '\r'.code -> out.append("&#13;")
                // The control characters XML does not have (X.680 12.15.5).
                code < 0x20 && code != '\t'.code && code != '\n'.code -> out.append('<').append(CONTROLS[code]).append("/>")
                code < 0x80 -> out.append(code.toChar())
                code in 0xd800..0xdfff -> encodeError("A lone surrogate is no character.")
                else -> out.append("&#").append(code).append(';')
            }
        }
    }

    /** Writes `element` as `ElementTree.tostring` of Python does, in ASCII. */
    fun write(
        element: XmlElement,
        out: StringBuilder,
    ) {
        out.append('<').append(element.name)

        if (element.text.isNullOrEmpty() && element.children.isEmpty()) {
            out.append(" />")

            return
        }

        out.append('>')
        element.text?.let { writeText(out, it) }
        element.children.forEach { write(it, out) }
        out.append("</").append(element.name).append('>')
    }

    /**
     * Writes `element` as [write] does, with `indent` spaces for each level, as
     * the runtime for Python does: each child on a line of its own, and a line
     * feed after the root if it has children.
     */
    fun writeIndented(
        element: XmlElement,
        out: StringBuilder,
        indent: Int,
    ) {
        fun level(
            element: XmlElement,
            level: Int,
        ) {
            if (element.children.isEmpty()) {
                write(element, out)

                return
            }

            out.append('<').append(element.name).append('>')
            element.text?.takeIf { it.isNotBlank() }?.let { writeText(out, it) }

            for (child in element.children) {
                out.append('\n')
                repeat((level + 1) * indent) { out.append(' ') }
                level(child, level + 1)
            }

            out.append('\n')
            repeat(level * indent) { out.append(' ') }
            out.append("</").append(element.name).append('>')
        }

        level(element, 0)

        if (element.children.isNotEmpty()) out.append('\n')
    }
}

/** If `c` is white-space in Unicode (`White_Space`), as `str::trim` of Rust has it. */
private fun isUnicodeSpace(c: Char): Boolean =
    c in '\u0009'..'\u000d' || c == ' ' || c == '\u0085' || c == ' ' || c == ' ' || c in ' '..' ' ||
        c == ' ' || c == ' ' || c == ' ' || c == ' ' || c == '　'

/** Whether `c` is white-space in XML. */
private fun isXmlSpace(c: Char): Boolean = c == ' ' || c == '\t' || c == '\n' || c == '\r'

/** Whether `text` is nothing but white-space in XML. */
private fun isBlank(text: String?): Boolean = text.orEmpty().all(::isXmlSpace)

/** If the text is an XER real number: a `realnumber` (X.680 12.9), after a `-` or not (X.680 21.9). */
internal fun isXmlReal(text: String): Boolean {
    var at = if (text.startsWith('-')) 1 else 0

    fun digits(): Int {
        val from = at

        while (at < text.length && text[at] in '0'..'9') at += 1

        return at - from
    }

    if (digits() == 0) return false

    if (at < text.length && text[at] == '.') {
        at += 1
        digits()
    }

    if (at < text.length && (text[at] == 'e' || text[at] == 'E')) {
        at += 1

        if (at < text.length && (text[at] == '-' || text[at] == '+')) at += 1

        return digits() > 0 && at == text.length
    }

    return at == text.length
}

/**
 * A finite REAL as XER writes it: the digits of JER then `E0` if it is 0, or
 * less than 10 away from 0 and JER writes it with no `e`, and else the fewest
 * digits that read back, with one before the `.` and one after it at least,
 * then `E` and the exponent.
 */
internal fun xmlReal(value: Double): String {
    val repr = floatRepr(value)

    if (value == 0.0 || (Math.abs(value) < 10.0 && 'e' !in repr)) return repr + "E0"

    val (digits, exponent) = shortestDigits(value)
    val sign = if (value < 0) "-" else ""
    val mantissa = digits.substring(0, 1) + "." + (if (digits.length > 1) digits.substring(1) else "0")

    return "${sign}${mantissa}E$exponent"
}

internal class Xer(
    private val types: Types,
) {
    fun encode(
        id: Int,
        value: Value,
        indent: Int? = null,
    ): ByteArray {
        val element = encode(id, nameOf(id), value, 0)
        val out = StringBuilder()

        if (indent == null) XmlText.write(element, out) else XmlText.writeIndented(element, out, indent)

        return out.toString().toByteArray(Charsets.US_ASCII)
    }

    fun decode(
        id: Int,
        data: ByteArray,
    ): Value = decode(id, XmlText.parse(data), 0)

    /** The name a type is given at the top, that of its definition. */
    private fun nameOf(id: Int): String =
        types.modules
            .asSequence()
            .flatMap { it.second }
            .firstOrNull { it.second == id }
            ?.first ?: ""

    private fun enumerationName(
        kind: Kind.Enumerated,
        value: Value,
    ): String =
        when {
            value is Value.Text && !types.numericEnums -> kind.items.firstOrNull { it.first == value.value }?.first
            value is Value.Integer && types.numericEnums -> kind.items.firstOrNull { BigInteger.valueOf(it.second) == value.value }?.first
            else -> null
        } ?: encodeError("Expected an enumeration value.")

    private fun integerText(value: BigInteger): String {
        val text = value.toString()

        if (text.length > MAX_DIGITS + 1) encodeError("The integer has too many digits.")

        return text
    }

    private fun alternative(
        kind: Kind.Choice,
        value: Value,
    ): Pair<MemberInfo, Value> {
        if (value !is Value.Choice) encodeError("Expected data of type tuple(str, object).")

        val member = kind.members.named(value.name) ?: encodeError("Expected a name of an alternative of the CHOICE.")

        return member to value.value
    }

    /**
     * The element of a value that is an item of a SEQUENCE OF or a SET OF: a
     * BOOLEAN, an ENUMERATED and a CHOICE are not in an element of their own
     * then (X.693 9.3.6).
     */
    private fun encodeOf(
        id: Int,
        name: String,
        value: Value,
        depth: Int,
    ): XmlElement =
        when (val kind = types.kind(id)) {
            is Kind.Boolean -> XmlElement.of(if ((value as? Value.Boolean ?: encodeError("Expected data of type bool.")).value) "true" else "false")
            is Kind.Enumerated -> XmlElement.of(enumerationName(kind, value))
            is Kind.Choice -> {
                val (member, inner) = alternative(kind, value)

                inMember(member.name) { encode(member.type, member.name, inner, depth + 1) }
            }
            else -> encode(id, name, value, depth)
        }

    private fun encode(
        id: Int,
        name: String,
        value: Value,
        depth: Int,
    ): XmlElement {
        if (depth > MAX_DEPTH) encodeError("The value is nested too deeply.")

        val element = XmlElement.of(name)

        when (val kind = types.kind(id)) {
            is Kind.Ref -> encodeError("The type has no contents of its own.")
            is Kind.Boolean ->
                element.children.add(XmlElement.of(if ((value as? Value.Boolean ?: encodeError("Expected data of type bool.")).value) "true" else "false"))
            is Kind.Integer ->
                element.text =
                    when (value) {
                        is Value.Text -> (kind.number(value.value) ?: encodeError("Expected an integer or a named number.")).toString()
                        is Value.Integer -> integerText(value.value)
                        else -> encodeError("Expected data of type int or str.")
                    }
            is Kind.Real -> {
                val number =
                    when (value) {
                        is Value.Real -> value.value
                        is Value.Integer -> Ber.realOfInteger(value.value)
                        else -> encodeError("Expected data of type float or int.")
                    }

                // The special values are empty elements (X.680 21.9).
                when {
                    number.isNaN() -> element.children.add(XmlElement.of("NOT-A-NUMBER"))
                    number == Double.POSITIVE_INFINITY -> element.children.add(XmlElement.of("PLUS-INFINITY"))
                    number == Double.NEGATIVE_INFINITY -> element.children.add(XmlElement.of("MINUS-INFINITY"))
                    else -> element.text = xmlReal(number)
                }
            }
            is Kind.Null -> if (value != Value.Null) encodeError("Expected None.")
            is Kind.BitString -> {
                if (value !is Value.Bits) encodeError("Expected data of type tuple(bytes, int).")

                if (value.length < 0 || value.bytes.size.toLong() * 8 < value.length) encodeError("Expected more bits of data.")

                if (value.length > 0) {
                    val bits = StringBuilder(value.length)

                    for (bit in 0 until value.length) {
                        bits.append(if (value.bytes[bit / 8].toInt() and (0x80 ushr (bit % 8)) != 0) '1' else '0')
                    }

                    element.text = bits.toString()
                }
            }
            is Kind.OctetString -> {
                val bytes = (value as? Value.Bytes ?: encodeError("Expected data of type bytes or bytearray.")).value

                if (bytes.isNotEmpty()) element.text = hexUpper(bytes)
            }
            is Kind.ObjectIdentifier ->
                return XmlElement.of(name, (value as? Value.ObjectIdentifier ?: encodeError("Expected an OBJECT IDENTIFIER.")).toString())
            is Kind.Str -> return XmlElement.of(name, (value as? Value.Text ?: encodeError("Expected data of type str.")).value)
            is Kind.Enumerated -> element.children.add(XmlElement.of(enumerationName(kind, value)))
            is Kind.Sequence, is Kind.Set, is Kind.External -> {
                if (value !is Value.Map) encodeError("Expected data of type dict.")

                for (member in kind.mapMembers!!.all) {
                    val found = value.members[member.name]

                    if (found == null) {
                        if (member.optional || member.hasDefault) continue

                        encodeError("A member is not found in the SEQUENCE or SET.")
                    }

                    val type = memberType(member, value.members) ?: encodeError("Bad AnyDefinedBy choice.")

                    element.children.add(inMember(member.name) { encode(type, member.name, found, depth + 1) })
                }
            }
            is Kind.SequenceOf, is Kind.SetOf -> {
                if (value !is Value.List) encodeError("Expected data of type list.")

                val (item, itemName) =
                    when (kind) {
                        is Kind.SequenceOf -> kind.element to kind.elementName
                        else -> (kind as Kind.SetOf).element to kind.elementName
                    }

                value.items.forEachIndexed { index, entry ->
                    element.children.add(inElement(index) { encodeOf(item, itemName, entry, depth + 1) })
                }
            }
            is Kind.Utc -> element.text = utcTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), false)
            is Kind.Generalized ->
                element.text = generalizedTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."), false)
            is Kind.Date -> element.text = Iso.dateToString((value as? Value.Date ?: encodeError("Expected data of type datetime.date.")).date)
            is Kind.TimeOfDay -> {
                val time = value as? Value.Time ?: encodeError("Expected data of type datetime.time.")

                element.text = Iso.timeToString(time.time, time.offsetMinutes)
            }
            is Kind.DateTime -> element.text = Iso.dateTimeToString(value as? Value.DateTime ?: encodeError("Expected data of type datetime.datetime."))
            is Kind.Duration -> {
                val text = (value as? Value.Text ?: encodeError("Expected data of type str.")).value

                validateDuration(text)
                element.text = durationToCanonical(text)
            }
            is Kind.Choice -> {
                val (member, inner) = alternative(kind, value)

                element.children.add(inMember(member.name) { encode(member.type, member.name, inner, depth + 1) })
            }
            // The value of the type it is (X.693 8.5), which only an ANY
            // DEFINED BY with choices tells, as a member.
            is Kind.Any, is Kind.AnyDefinedBy, is Kind.OpenType -> unsupported("XER needs the type of the value of an ANY or an open type.")
        }

        return element
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

    private inline fun <T> time(block: () -> T): T =
        try {
            block()
        } catch (_: ValueException) {
            error("Expected a date or a time.")
        }

    private fun text(element: XmlElement): String = element.text ?: error("Expected text in the XML element.")

    /** A BOOLEAN, `<true/>` or `<false/>` (X.680 18.3): an empty element and nothing else. */
    private fun boolean(element: XmlElement): Value.Boolean =
        if ((element.name == "true" || element.name == "false") && element.isEmpty) {
            Value.Boolean(element.name == "true")
        } else {
            error("Expected <true/> or <false/>.")
        }

    private fun enumerationValue(item: Pair<String, Long>): Value = if (types.numericEnums) Value.Integer(item.second) else Value.Text(item.first)

    /** The text of a character string, in which a control character is an empty element, as `<bel/>` (X.680 12.15.5). */
    private fun string(element: XmlElement): String {
        val text = StringBuilder(element.text.orEmpty())

        for (child in element.children) {
            val control = XmlText.control(child.name)?.takeIf { child.isEmpty } ?: error("Expected text in the XML element.")

            text.append(control).append(child.tail.orEmpty())
        }

        return text.toString()
    }

    /** Decodes an item of a SEQUENCE OF or a SET OF. */
    private fun decodeOf(
        id: Int,
        element: XmlElement,
        depth: Int,
    ): Value =
        when (val kind = types.kind(id)) {
            is Kind.Boolean -> boolean(element)
            is Kind.Enumerated -> enumerationValue(kind.items.firstOrNull { it.first == element.name } ?: error("Expected an enumeration value."))
            is Kind.Choice -> {
                val member = kind.members.named(element.name) ?: error("Expected a name of an alternative of the CHOICE.")

                Value.Choice(member.name, inMember(member.name) { decode(member.type, element, depth + 1) })
            }
            else -> decode(id, element, depth)
        }

    private fun decode(
        id: Int,
        element: XmlElement,
        depth: Int,
    ): Value {
        if (depth > MAX_DEPTH) error("The data is nested too deeply.")

        return when (val kind = types.kind(id)) {
            is Kind.Ref -> error("The type has no contents of its own.")
            is Kind.Boolean -> {
                val value = element.children.singleOrNull()

                if (value == null || !isBlank(element.text) || !isBlank(value.tail)) error("Expected <true/> or <false/>.")

                boolean(value)
            }
            is Kind.Integer -> {
                val text = text(element).trim(::isUnicodeSpace)
                val digits = text.removePrefix("-").let { if (it.length == text.length) it.removePrefix("+") else it }

                if (digits.isEmpty() || digits.length > MAX_DIGITS || !digits.all { it in '0'..'9' }) error("Expected an integer.")

                Value.Integer(BigInteger(text))
            }
            is Kind.Real -> {
                val special = element.children.singleOrNull()

                when {
                    element.children.isEmpty() -> {
                        val text = text(element).trim(::isUnicodeSpace)

                        if (!isXmlReal(text)) error("Expected a real.")

                        Value.Real(text.toDouble())
                    }
                    special != null && element.text.orEmpty().trim(::isUnicodeSpace).isEmpty() && special.isEmpty ->
                        Value.Real(
                            when (special.name) {
                                "PLUS-INFINITY" -> Double.POSITIVE_INFINITY
                                "MINUS-INFINITY" -> Double.NEGATIVE_INFINITY
                                "NOT-A-NUMBER" -> Double.NaN
                                else -> error("Expected a real.")
                            },
                        )
                    else -> error("Expected a real.")
                }
            }
            is Kind.Null -> Value.Null
            // An `xmlbstring` may have space in it (X.680 12.10).
            is Kind.BitString -> {
                val text = element.text.orEmpty().filterNot(::isXmlSpace)
                val bytes = ByteArray((text.length + 7) / 8)

                text.forEachIndexed { bit, c ->
                    when (c) {
                        '0' -> {}
                        '1' -> bytes[bit / 8] = (bytes[bit / 8].toInt() or (0x80 ushr (bit % 8))).toByte()
                        else -> error("Expected 0 and 1 in a BIT STRING.")
                    }
                }

                Value.Bits(bytes, text.length)
            }
            // An `xmlhstring` may have space in it (X.680 12.12), and an odd
            // number of digits is ended by a zero (X.680 22.3).
            is Kind.OctetString -> {
                val text = element.text.orEmpty().filterNot(::isXmlSpace)

                Value.Bytes(hexDecode(if (text.length % 2 == 1) text + "0" else text) ?: error("Expected hex."))
            }
            is Kind.ObjectIdentifier ->
                try {
                    Value.ObjectIdentifier.parse(text(element))
                } catch (_: EncodeException) {
                    error("Expected an OBJECT IDENTIFIER.")
                }
            is Kind.Str -> Value.Text(string(element))
            is Kind.Enumerated -> {
                val inner = element.children.firstOrNull() ?: error("Expected an enumeration value.")
                val found = kind.items.firstOrNull { it.first == inner.name }

                when {
                    found != null -> enumerationValue(found)
                    kind.additions != null -> Value.Null
                    else -> error("Expected an enumeration value.")
                }
            }
            is Kind.Sequence, is Kind.Set, is Kind.External -> {
                val map = LinkedHashMap<String, Value>()

                for (member in kind.mapMembers!!.all) {
                    val inner = element.child(member.name)

                    if (inner != null) {
                        val type = memberType(member, map) ?: error("Bad AnyDefinedBy choice.")

                        map[member.name] = inMember(member.name) { decode(type, inner, depth + 1) }
                    } else if (!member.optional) {
                        map[member.name] = member.default ?: error("A member is not found in the element.")
                    }
                }

                Value.Map(map)
            }
            is Kind.SequenceOf, is Kind.SetOf -> {
                val item = (kind as? Kind.SequenceOf)?.element ?: (kind as Kind.SetOf).element

                Value.List(element.children.mapIndexed { index, child -> inElement(index) { decodeOf(item, child, depth + 1) } })
            }
            is Kind.Utc -> time { utcTimeFromString(text(element), false) }
            is Kind.Generalized -> time { generalizedTimeFromString(text(element), false) }
            is Kind.Date -> Value.Date(time { Iso.dateFromString(text(element)) })
            is Kind.TimeOfDay -> Value.Time(time { Iso.timeFromString(text(element)) })
            is Kind.DateTime -> Value.DateTime(time { Iso.dateTimeFromString(text(element)) })
            is Kind.Duration -> {
                val text = text(element)

                try {
                    validateDuration(text)
                } catch (_: EncodeException) {
                    error("Invalid DURATION.")
                }

                Value.Text(text)
            }
            is Kind.Choice -> {
                val inner = element.children.firstOrNull() ?: error("Expected an alternative of the CHOICE.")
                val member = kind.members.named(inner.name)

                when {
                    member != null -> Value.Choice(member.name, inMember(member.name) { decode(member.type, inner, depth + 1) })
                    kind.members.additions != null -> Value.Choice("", Value.Null)
                    else -> error("Expected a name of an alternative of the CHOICE.")
                }
            }
            is Kind.Any, is Kind.AnyDefinedBy, is Kind.OpenType -> unsupported("XER needs the type of the value of an ANY or an open type.")
        }
    }
}
