/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * Encodes and decodes a Question of examples/compiled/foo.asni:
 *
 *     Question ::= SEQUENCE {
 *         id        INTEGER,
 *         question  IA5String
 *     }
 *
 * in BER, and shows it as JER. Run it with the path of foo.asni:
 *
 *     ./question examples/compiled/foo.asni
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "asnkitrt.h"

/* Prints the last error and fails, if `status` is one. */
static void check(asnkit_status status, const char *what)
{
    size_t offset;

    if (status == ASNKIT_OK) {
        return;
    }

    if (asnkit_last_error_offset(&offset)) {
        fprintf(stderr, "%s: %s (at %zu)\n", what, asnkit_last_error(), offset);
    } else {
        fprintf(stderr, "%s: %s\n", what, asnkit_last_error());
    }

    exit(1);
}

static void print_hex(const uint8_t *data, size_t length)
{
    size_t i;

    for (i = 0; i < length; i++) {
        printf("%02x", data[i]);
    }

    printf("\n");
}

int main(int argc, const char *argv[])
{
    static const uint8_t expected[] = {
        0x30, 0x0e, 0x02, 0x01, 0x01, 0x16, 0x09, 'I', 's', ' ', '1', '+', '1', '=', '3', '?'
    };
    static const uint8_t garbage[] = { 0x30, 0x0e, 0x02, 0x01 };
    asnkit_spec *ber;
    asnkit_spec *jer;
    asnkit_value *question;
    asnkit_value *decoded;
    const asnkit_value *member;
    const char *text;
    size_t text_length;
    int64_t id;
    uint8_t *data;
    size_t length;
    asnkit_options indented = ASNKIT_OPTIONS_INIT;
    asnkit_status status;

    if (argc != 2) {
        fprintf(stderr, "usage: %s FOO.ASNI\n", argv[0]);
        return 2;
    }

    printf("asnkitrt %s\n", asnkit_version());

    check(asnkit_spec_load_file(argv[1], "ber", NULL, &ber), "Loading for BER");
    check(asnkit_spec_load_file(argv[1], "jer", NULL, &jer), "Loading for JER");

    /* {id: 1, question: "Is 1+1=3?"}. The map takes the members. */
    question = asnkit_value_new_map();
    check(asnkit_map_set(question, "id", asnkit_value_new_integer(1)), "Setting id");
    check(asnkit_map_set(question, "question", asnkit_value_new_text("Is 1+1=3?", 9)),
          "Setting question");

    check(asnkit_encode(ber, "Question", question, NULL, &data, &length), "Encoding");
    printf("BER: ");
    print_hex(data, length);

    if (length != sizeof(expected) || memcmp(data, expected, length) != 0) {
        fprintf(stderr, "Expected 300e0201011609497320312b313d333f.\n");
        return 1;
    }

    check(asnkit_decode(ber, "Question", data, length, NULL, &decoded), "Decoding");
    asnkit_bytes_free(data, length);

    /* What is borrowed from `decoded` is valid as long as it is. */
    member = asnkit_map_get(decoded, "id");
    check(asnkit_value_integer(member, &id), "Reading id");
    member = asnkit_map_get(decoded, "question");
    check(asnkit_value_text(member, &text, &text_length), "Reading question");
    printf("Decoded: id %lld, question \"%.*s\"\n", (long long) id, (int) text_length, text);

    if (id != 1 || text_length != 9 || memcmp(text, "Is 1+1=3?", 9) != 0) {
        fprintf(stderr, "Expected what was encoded.\n");
        return 1;
    }

    /* The same value, as JER, which is a string of C too. */
    indented.indent = 2;
    check(asnkit_encode(jer, "Question", decoded, &indented, &data, &length), "Encoding JER");
    printf("JER:\n%s\n", (const char *) data);
    asnkit_bytes_free(data, length);

    /* An error, with where it is. */
    asnkit_value_free(decoded);
    status = asnkit_decode(ber, "Question", garbage, sizeof(garbage), NULL, &decoded);

    if (status != ASNKIT_ERROR_DECODE) {
        fprintf(stderr, "Expected a decoding error.\n");
        return 1;
    }

    printf("Truncated: %s\n", asnkit_last_error());

    asnkit_value_free(question);
    asnkit_spec_free(jer);
    asnkit_spec_free(ber);

    return 0;
}
