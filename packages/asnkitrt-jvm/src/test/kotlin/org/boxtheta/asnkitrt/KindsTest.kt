// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Test
import java.io.File

class KindsTest {
    /** The table of the specification is the table here. */
    @Test
    fun sameAsTheSpecification() {
        val text = File(repository, "docs/file_formats.md").readText()
        val table = text.substringAfter("<!-- kinds-begin -->").substringBefore("<!-- kinds-end -->")
        val found =
            table.lines().mapNotNull { line ->
                val cells = line.trim().trim('|').split('|').map { it.trim() }
                val number = cells.firstOrNull()?.toIntOrNull()

                if (!line.trimStart().startsWith('|') || cells.size < 3 || number == null) {
                    null
                } else {
                    KindOfNode(number, cells[1], if (cells[2] == "--") "" else cells[2])
                }
            }

        assertEquals(KINDS, found)
    }
}
