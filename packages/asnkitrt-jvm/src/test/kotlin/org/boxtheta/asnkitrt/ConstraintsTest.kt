// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import org.junit.jupiter.api.Assertions.assertEquals
import org.junit.jupiter.api.Assertions.assertThrows
import org.junit.jupiter.api.Test
import java.math.BigInteger

/** What the constraints of the nodes of a compiled specification mean. */
class ConstraintsTest {
    private fun int(value: Long) = Cbor.Integer(BigInteger.valueOf(value))

    private fun text(value: String) = Cbor.Text(value)

    private fun node(
        kind: Long,
        vararg fields: Cbor,
    ) = Cbor.Array(listOf(int(kind)) + fields)

    private fun list(vararg items: Cbor) = node(0, *items)

    private fun assignment(
        name: String,
        type: Cbor,
    ) = node(3, text(name), type)

    private fun integer(restrictedTo: Cbor) = node(12, Cbor.Null, Cbor.Null, Cbor.Null, restrictedTo)

    /** A specification of one module, `M`. */
    private fun module(
        types: List<Cbor>,
        objectClasses: List<Cbor> = emptyList(),
    ) = node(1, list(node(2, text("M"), list(*types.toTypedArray()), list(*objectClasses.toTypedArray()))))

    private fun load(
        nodes: Cbor,
        codec: Codec,
    ) = lower(Spec.fromNodes(nodes), codec, numericEnums = false, lenient = false)

    @Test
    fun aConstraintOfNoElementIsRefused() {
        // A root with no value that is not extensible is written as no
        // element, which was taken for no constraint. So were
        // I ::= INTEGER (5..1), K ::= J (10..20) with J ::= INTEGER (0..5)
        // and S ::= IA5String (SIZE (3..1)) written by 2.0.0-rc5.
        val j = assignment("J", integer(list(node(6, int(0), int(5)))))
        val refused =
            listOf(
                assignment("I", integer(list())),
                assignment("K", node(11, text("J"), Cbor.Null, Cbor.Null, list())),
                assignment("S", node(10, text("IA5String"), Cbor.Null, list())),
                // In the elements of a member.
                assignment("T", node(15, list(node(4, text("b"), node(18, integer(list())))))),
            )

        for (assigned in refused) {
            for (codec in Codec.entries) {
                val error = assertThrows(AsniException::class.java) { load(module(listOf(j, assigned)), codec) }

                assertEquals(AsniException.Reason.BAD_NODES, error.reason)
                assertEquals("A type has a constraint that permits no value.", error.message)
            }
        }

        // The type of a field of an object class, where it is used.
        val nodes =
            module(
                listOf(assignment("X", node(15, list(node(4, text("id"), node(11, text("C.&id"))))))),
                listOf(node(39, text("C"), list(node(40, text("&id"), integer(list()))))),
            )

        assertThrows(AsniException::class.java) { load(nodes, Codec.BER) }

        // A root with no value that is extensible, as (5..1, ...), is an
        // extension marker alone, and is loaded.
        load(module(listOf(j, assignment("W", integer(list(node(8)))))), Codec.BER)
    }
}
