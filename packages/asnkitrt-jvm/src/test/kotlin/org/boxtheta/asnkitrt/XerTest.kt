// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertFalse
import org.junit.jupiter.api.Assertions.assertTrue
import org.junit.jupiter.api.Test
import org.junit.jupiter.api.assertThrows
import java.io.File

/** The XML XER reads and writes, past what the vectors have. */
class XerTest {
    /** A spec of the vectors in which `A` is a character string. */
    private val strings = Specification.load(File(repository, "tests/vectors/specs/7109b8113d927714.asni"), Options(Codec.XER))

    private fun read(xml: String): Value = strings.decode("A", xml.toByteArray())

    @Test
    fun readsWhatXmlAllows() {
        assertEquals(Value.Text("t&AB<c>"), read("<?xml version=\"1.0\"?>\n<!-- c --><A x='1' y=\"&lt;\">t&amp;&#65;&#x42;<![CDATA[<c>]]></A>\n"))
        // Line ends are line feeds, but for a reference.
        assertEquals(Value.Text("a\nb\nc\r"), read("<A>a\r\nb\rc&#13;</A>"))
        // A name is the local one, also with a prefix that is not declared.
        assertEquals(Value.Text("x"), read("<n:A xmlns:n='urn:x'>x</n:A>"))
        assertEquals(Value.Text("x"), read("<n:A>x</n:A>"))
        assertEquals(Value.Text("é😀"), read("<?pi?><A>é&#128512;</A>"))
        // The encoding of the declaration is not looked at: it is UTF-8.
        assertEquals(Value.Text("é"), read("<?xml version=\"1.0\" encoding=\"ISO-8859-1\"?><A>é</A>"))
    }

    @Test
    fun refusesWhatIsNotXmlOrMayGrow() {
        val laughs =
            "<?xml version=\"1.0\"?><!DOCTYPE A [<!ENTITY a \"aaaaaaaaaa\"><!ENTITY b \"&a;&a;&a;&a;&a;&a;&a;&a;&a;&a;\">" +
                "<!ENTITY c \"&b;&b;&b;&b;&b;&b;&b;&b;&b;&b;\">]><A>&c;</A>"

        for (bad in listOf(
            "",
            "<A>",
            "<A></B>",
            "<A/><B/>",
            "<A>&nope;</A>",
            "<A>&#0;</A>",
            "<A x=1/>",
            "<A x='1' x='2'/>",
            "<!DOCTYPE A [<!ENTITY e 'x'>]><A>&e;</A>",
            "<!DOCTYPE A SYSTEM \"file:///etc/passwd\"><A/>",
            "<!DOCTYPE A><A/>",
            laughs,
            "text<A/>",
            "<A/>text",
            "<A>\u0001</A>",
            "<?xml version=\"1.1\"?><A>&#1;</A>",
            "<A>&#xD800;</A>",
            "</A>",
            "<A>" + "<x>".repeat(200) + "</x>".repeat(200) + "</A>",
        )) {
            val error = assertThrows<DecodeException>(bad) { read(bad) }

            assertTrue(error.message.endsWith("Bad XML."), "$bad: ${error.message}")
        }

        assertEquals("A: Expected UTF-8.", assertThrows<DecodeException> { strings.decode("A", byteArrayOf(0x3c, 0x41, 0x3e, 0xff.toByte())) }.message)
    }

    @Test
    fun charactersXmlDoesNotHave() {
        // As an empty element of its name (X.680 12.15.5).
        assertEquals("<A>a<bel/>b&#13;&lt;&amp;&gt;&#233;&#128512;</A>", String(strings.encode("A", Value.Text("a\u0007b\r<&>é😀"))))
        assertEquals(Value.Text("a\u0007b\u0000"), read("<A>a<bel/>b<nul /></A>"))
        assertThrows<DecodeException> { read("<A>a<bell/></A>") }
        assertThrows<DecodeException> { read("<A>a<bel>x</bel></A>") }
        assertThrows<EncodeException> { strings.encode("A", Value.Text("\ud800")) }
    }

    @Test
    fun reals() {
        for ((value, text) in listOf(
            0.0 to "0.0E0",
            -0.0 to "-0.0E0",
            1.0 to "1.0E0",
            0.001 to "0.001E0",
            -2.25 to "-2.25E0",
            10.0 to "1.0E1",
            12345.0 to "1.2345E4",
            1e-5 to "1.0E-5",
            -1.5e-300 to "-1.5E-300",
            Double.MAX_VALUE to "1.7976931348623157E308",
            Double.MIN_VALUE to "5.0E-324",
        )) {
            assertEquals(text, xmlReal(value))
            assertTrue(isXmlReal(text), text)
            assertEquals(value, text.toDouble())
        }

        for (text in listOf("1", "-1", "1.", "1.5", "007.50", "1e5", "1E-5", "1.e+5", "-0")) assertTrue(isXmlReal(text), text)

        for (text in listOf("", "-", ".5", "+1", "1.5.", "1e", "1e+", "inf", "NaN", "INF", "1 ", "--1", "1d", "0x1p3")) assertFalse(isXmlReal(text), text)
    }
}
