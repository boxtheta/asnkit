/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * The API of include/asnkitrt.h, from C, as tests/api.test.mjs of the
 * runtime for WebAssembly has it from JavaScript. It is run by tests/c.rs
 * with the path of packages/asnkitrt-wasm/tests/fixtures/types.asni and
 * of examples/corpus, and fails with the line of the first check that
 * does.
 */

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "asnkitrt.h"

#define CHECK(condition)                                                              \
    do {                                                                              \
        if (!(condition)) {                                                           \
            fprintf(stderr, "%s:%d: %s (last error: %s)\n", __FILE__, __LINE__,       \
                    #condition, asnkit_last_error());                                 \
            exit(1);                                                                  \
        }                                                                             \
    } while (0)

#define OK(call) CHECK((call) == ASNKIT_OK)

static const char *corpus;

/* Encodes and decodes `value`, which it frees, and returns what was
 * decoded. */
static asnkit_value *round_trip(const asnkit_spec *spec,
                                const char *name,
                                asnkit_value *value,
                                const asnkit_options *options)
{
    uint8_t *data;
    size_t length;
    asnkit_value *decoded;

    CHECK(value != NULL);
    OK(asnkit_encode(spec, name, value, options, &data, &length));
    CHECK(data[length] == '\0');
    OK(asnkit_decode(spec, name, data, length, options, &decoded));
    asnkit_bytes_free(data, length);
    asnkit_value_free(value);

    return decoded;
}

static int text_is(const asnkit_value *value, const char *expected)
{
    const char *text;
    size_t length;

    return asnkit_value_text(value, &text, &length) == ASNKIT_OK && length == strlen(expected)
           && memcmp(text, expected, length) == 0;
}

static void hex(const char *text, uint8_t *bytes, size_t length)
{
    size_t i;
    unsigned int byte;

    CHECK(strlen(text) >= 2 * length);

    for (i = 0; i < length; i++) {
        CHECK(sscanf(text + 2 * i, "%2x", &byte) == 1);
        bytes[i] = (uint8_t) byte;
    }
}

/* The key in hexadecimal in the file `name` of the corpus. */
static void read_key(const char *name, uint8_t key[ASNKIT_KEY_SIZE])
{
    char path[4096];
    char text[2 * ASNKIT_KEY_SIZE + 1];
    FILE *file;

    snprintf(path, sizeof(path), "%s/%s", corpus, name);
    file = fopen(path, "r");
    CHECK(file != NULL);
    CHECK(fread(text, 1, 2 * ASNKIT_KEY_SIZE, file) == 2 * ASNKIT_KEY_SIZE);
    fclose(file);
    text[2 * ASNKIT_KEY_SIZE] = '\0';
    hex(text, key, ASNKIT_KEY_SIZE);
}

/* Round-trips a Question with `spec`, which it frees. */
static void ask(asnkit_spec *spec)
{
    asnkit_value *question;
    asnkit_value *decoded;

    question = asnkit_value_new_map();
    OK(asnkit_map_set(question, "id", asnkit_value_new_integer(1)));
    OK(asnkit_map_set(question, "question", asnkit_value_new_text("Is 1+1=3?", 9)));
    decoded = round_trip(spec, "Question", question, NULL);
    CHECK(text_is(asnkit_map_get(decoded, "question"), "Is 1+1=3?"));
    asnkit_value_free(decoded);
    asnkit_spec_free(spec);
}

static asnkit_status load_corpus(const char *name, const asnkit_load_options *options)
{
    char path[4096];
    asnkit_spec *spec = NULL;
    asnkit_status status;

    snprintf(path, sizeof(path), "%s/%s", corpus, name);
    status = asnkit_spec_load_file(path, "uper", options, &spec);

    if (status == ASNKIT_OK) {
        ask(spec);
    }

    return status;
}

static asnkit_status load_bytes(const uint8_t *asni, size_t length, const asnkit_load_options *options)
{
    asnkit_spec *spec = NULL;
    asnkit_status status = asnkit_spec_load(asni, length, "uper", options, &spec);

    if (status == ASNKIT_OK) {
        ask(spec);
    }

    return status;
}

/* The contents of the file `name` of the corpus, to free. */
static uint8_t *read_corpus(const char *name, size_t *length)
{
    char path[4096];
    uint8_t *data;
    long size;
    FILE *file;

    snprintf(path, sizeof(path), "%s/%s", corpus, name);
    file = fopen(path, "rb");
    CHECK(file != NULL);
    CHECK(fseek(file, 0, SEEK_END) == 0);
    size = ftell(file);
    CHECK(size > 0);
    rewind(file);
    data = malloc((size_t) size);
    CHECK(data != NULL);
    CHECK(fread(data, 1, (size_t) size, file) == (size_t) size);
    fclose(file);
    *length = (size_t) size;

    return data;
}

static void test_errors(const char *types)
{
    static const uint8_t garbage[] = { 1, 2, 3 };
    asnkit_spec *spec = NULL;
    asnkit_value *list;
    size_t offset;

    CHECK(asnkit_spec_load_file(types, "nope", NULL, &spec) == ASNKIT_ERROR_ARGUMENT);
    CHECK(strstr(asnkit_last_error(), "Unsupported codec 'nope'") != NULL);
    CHECK(asnkit_spec_load(garbage, sizeof(garbage), NULL, NULL, &spec) == ASNKIT_ERROR_LOAD);
    CHECK(asnkit_spec_load_file("/nonexistent/types.asni", NULL, NULL, &spec) == ASNKIT_ERROR_IO);
    CHECK(asnkit_spec_load_file(types, NULL, NULL, NULL) == ASNKIT_ERROR_ARGUMENT);
    CHECK(asnkit_last_error_offset(&offset) == 0);
    CHECK(spec == NULL);

    /* NULL is never a value, nor a place to write one. */
    CHECK(asnkit_value_kind(NULL) == -1);
    CHECK(asnkit_value_new_text(NULL, 1) == NULL);
    CHECK(asnkit_value_new_text("\xff", 1) == NULL);
    CHECK(asnkit_value_new_bits(garbage, 3, 8) == NULL);
    CHECK(asnkit_list_push(NULL, asnkit_value_new_null()) == ASNKIT_ERROR_ARGUMENT);
    list = asnkit_value_new_list();
    CHECK(asnkit_map_set(list, "a", NULL) == ASNKIT_ERROR_ARGUMENT);
    CHECK(asnkit_map_set(list, "a", asnkit_value_new_null()) == ASNKIT_ERROR_KIND);
    asnkit_value_free(list);
    asnkit_value_free(NULL);
    asnkit_spec_free(NULL);
    asnkit_bytes_free(NULL, 0);
    asnkit_load_options_free(NULL);
}

static void test_values(const char *types)
{
    asnkit_spec *der;
    asnkit_spec *jer;
    asnkit_value *value;
    asnkit_value *list;
    asnkit_value *decoded;
    const asnkit_value *inner;
    const char *name;
    size_t name_length;
    size_t length;
    size_t bits;
    const uint8_t *data;
    int boolean;
    int64_t integer;
    double real;
    uint8_t big[32];
    /* 2 ** 100. */
    static const uint8_t two_to_the_100[] = { 0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0 };
    static const uint8_t bit_string[] = { 0xa0 };
    asnkit_date date = { 2024, 2, 29 };
    asnkit_time time = { 12, 30, 15, 0, 1, 0 };
    asnkit_date date_out;
    asnkit_time time_out;

    OK(asnkit_spec_load_file(types, "der", NULL, &der));
    OK(asnkit_spec_load_file(types, "jer", NULL, &jer));
    CHECK(strcmp(asnkit_spec_codec(der), "der") == 0);

    /* CHOICE: ["b", [true, false]]. */
    list = asnkit_value_new_list();
    OK(asnkit_list_push(list, asnkit_value_new_boolean(1)));
    OK(asnkit_list_push(list, asnkit_value_new_boolean(0)));
    decoded = round_trip(der, "Choice", asnkit_value_new_choice("b", list), NULL);
    CHECK(asnkit_value_kind(decoded) == ASNKIT_CHOICE);
    OK(asnkit_choice_get(decoded, &name, &name_length, &inner));
    CHECK(name_length == 1 && name[0] == 'b');
    OK(asnkit_value_length(inner, &length));
    CHECK(length == 2);
    OK(asnkit_value_boolean(asnkit_list_get(inner, 0), &boolean));
    CHECK(boolean == 1);
    OK(asnkit_value_boolean(asnkit_list_get(inner, 1), &boolean));
    CHECK(boolean == 0);
    CHECK(asnkit_list_get(inner, 2) == NULL);
    CHECK(asnkit_value_integer(inner, &integer) == ASNKIT_ERROR_KIND);
    asnkit_value_free(decoded);

    /* An INTEGER of more than 64 bits. */
    decoded = round_trip(der, "Big",
                         asnkit_value_new_integer_bytes(two_to_the_100, sizeof(two_to_the_100)),
                         NULL);
    CHECK(asnkit_value_kind(decoded) == ASNKIT_INTEGER);
    CHECK(asnkit_value_integer(decoded, &integer) == ASNKIT_ERROR_RANGE);
    CHECK(asnkit_value_integer_bytes(decoded, NULL, 0, &length) == ASNKIT_ERROR_RANGE);
    CHECK(length == sizeof(two_to_the_100));
    OK(asnkit_value_integer_bytes(decoded, big, sizeof(big), &length));
    CHECK(memcmp(big, two_to_the_100, length) == 0);
    asnkit_value_free(decoded);

    decoded = round_trip(der, "Big", asnkit_value_new_integer(-129), NULL);
    OK(asnkit_value_integer(decoded, &integer));
    CHECK(integer == -129);
    OK(asnkit_value_integer_bytes(decoded, big, sizeof(big), &length));
    CHECK(length == 2 && big[0] == 0xff && big[1] == 0x7f);
    asnkit_value_free(decoded);

    decoded = round_trip(der, "Real", asnkit_value_new_real(0.5), NULL);
    OK(asnkit_value_real(decoded, &real));
    CHECK(real == 0.5);
    asnkit_value_free(decoded);

    decoded = round_trip(der, "Bits", asnkit_value_new_bits(bit_string, 1, 3), NULL);
    OK(asnkit_value_bits(decoded, &data, &length, &bits));
    CHECK(length == 1 && bits == 3 && data[0] == 0xa0);
    asnkit_value_free(decoded);

    decoded = round_trip(der, "Day", asnkit_value_new_date(date), NULL);
    OK(asnkit_value_date(decoded, &date_out));
    CHECK(date_out.year == 2024 && date_out.month == 2 && date_out.day == 29);
    asnkit_value_free(decoded);

    decoded = round_trip(der, "When", asnkit_value_new_date_time(date, time), NULL);
    OK(asnkit_value_date_time(decoded, &date_out, &time_out));
    CHECK(date_out.day == 29 && time_out.hour == 12 && time_out.minute == 30);
    /* DER has times in UTC, which are read as naive, as in Python. */
    CHECK(time_out.second == 15 && !time_out.has_offset);
    CHECK(asnkit_value_time(decoded, &time_out) == ASNKIT_ERROR_KIND);
    asnkit_value_free(decoded);

    /* An OBJECT IDENTIFIER, as text and as its contents octets. */
    {
        static const uint8_t rsa[] = { 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d };
        char text[32];

        CHECK(asnkit_value_new_object_identifier("1", 1) == NULL);
        decoded = round_trip(der, "Oid", asnkit_value_new_object_identifier("1.2.840.113549", 14),
                             NULL);
        CHECK(asnkit_value_kind(decoded) == ASNKIT_OBJECT_IDENTIFIER);
        OK(asnkit_value_object_identifier_bytes(decoded, &data, &length));
        CHECK(length == sizeof(rsa) && memcmp(data, rsa, length) == 0);
        CHECK(asnkit_value_object_identifier(decoded, NULL, 0, &length) == ASNKIT_ERROR_RANGE);
        CHECK(length == 14);
        OK(asnkit_value_object_identifier(decoded, text, sizeof(text), &length));
        CHECK(length == 14 && memcmp(text, "1.2.840.113549", 14) == 0);
        CHECK(asnkit_value_text(decoded, &name, &name_length) == ASNKIT_ERROR_KIND);
        asnkit_value_free(decoded);
    }

    decoded = round_trip(der, "Color", asnkit_value_new_text("green", 5), NULL);
    CHECK(text_is(decoded, "green"));
    asnkit_value_free(decoded);

    /* A SEQUENCE, its members in order, and a member set twice. */
    value = asnkit_value_new_map();
    OK(asnkit_map_set(value, "id", asnkit_value_new_integer(1)));
    OK(asnkit_map_set(value, "name", asnkit_value_new_text("one", 3)));
    OK(asnkit_map_set(value, "id", asnkit_value_new_integer(2)));
    decoded = round_trip(der, "Record", asnkit_value_clone(value), NULL);
    OK(asnkit_value_length(decoded, &length));
    CHECK(length == 2);
    OK(asnkit_map_member(decoded, 0, &name, &name_length, &inner));
    CHECK(name_length == 2 && memcmp(name, "id", 2) == 0);
    OK(asnkit_value_integer(inner, &integer));
    CHECK(integer == 2);
    CHECK(asnkit_map_member(decoded, 2, &name, &name_length, &inner) == ASNKIT_ERROR_RANGE);
    CHECK(asnkit_map_get(decoded, "other") == NULL);
    asnkit_value_free(decoded);

    /* BER and DER have the length of a value in it. */
    {
        uint8_t *encoded;
        size_t encoded_length;
        size_t consumed;
        int known;

        OK(asnkit_encode(der, "Record", value, NULL, &encoded, &encoded_length));
        OK(asnkit_decode_length(der, encoded, encoded_length, &length, &known));
        CHECK(known && length == encoded_length);
        OK(asnkit_decode_length(der, encoded, 1, &length, &known));
        CHECK(!known);
        OK(asnkit_decode_with_length(der, "Record", encoded, encoded_length, NULL, &decoded,
                                     &consumed));
        CHECK(consumed == encoded_length);
        asnkit_value_free(decoded);
        CHECK(asnkit_decode_length(jer, encoded, encoded_length, &length, &known)
              == ASNKIT_ERROR_UNSUPPORTED);
        asnkit_bytes_free(encoded, encoded_length);
    }

    /* Indented JER, and no indent for DER. */
    {
        asnkit_options indented = ASNKIT_OPTIONS_INIT;
        uint8_t *encoded;
        size_t encoded_length;

        indented.indent = 4;
        OK(asnkit_encode(jer, "Record", value, &indented, &encoded, &encoded_length));
        CHECK(strstr((const char *) encoded, "\n    \"id\": 2") != NULL);
        asnkit_bytes_free(encoded, encoded_length);
        CHECK(asnkit_encode(der, "Record", value, &indented, &encoded, &encoded_length)
              == ASNKIT_ERROR_UNSUPPORTED);
    }

    asnkit_value_free(value);

    /* Constraints, checked when asked to. */
    {
        asnkit_options checked = ASNKIT_OPTIONS_INIT;
        uint8_t *encoded;
        size_t encoded_length;

        checked.check_constraints = 1;
        value = asnkit_value_new_integer(8);
        CHECK(asnkit_check_constraints(der, "Small", value, NULL) == ASNKIT_ERROR_CONSTRAINTS);
        CHECK(asnkit_encode(der, "Small", value, &checked, &encoded, &encoded_length)
              == ASNKIT_ERROR_CONSTRAINTS);
        OK(asnkit_encode(der, "Small", value, NULL, &encoded, &encoded_length));
        CHECK(asnkit_decode(der, "Small", encoded, encoded_length, &checked, &decoded)
              == ASNKIT_ERROR_CONSTRAINTS);
        asnkit_bytes_free(encoded, encoded_length);
        asnkit_value_free(value);
    }

    /* A type of two modules, and one of none. */
    {
        asnkit_options in_other = ASNKIT_OPTIONS_INIT;

        in_other.module = "Other";
        value = asnkit_value_new_boolean(1);
        CHECK(asnkit_check_constraints(der, "Twice", value, NULL) == ASNKIT_ERROR_NO_SUCH_TYPE);
        CHECK(asnkit_check_constraints(der, "Nope", value, NULL) == ASNKIT_ERROR_NO_SUCH_TYPE);
        decoded = round_trip(der, "Twice", value, &in_other);
        CHECK(asnkit_value_kind(decoded) == ASNKIT_BOOLEAN);
        asnkit_value_free(decoded);
    }

    /* A decoding error, with its offset. */
    {
        static const uint8_t truncated[] = { 0x30, 0x05, 0x02, 0x01 };
        size_t offset = 0;

        CHECK(asnkit_decode(der, "Record", truncated, sizeof(truncated), NULL, &decoded)
              == ASNKIT_ERROR_DECODE);
        CHECK(asnkit_last_error_offset(&offset) == 1);
        CHECK(asnkit_last_error()[0] != '\0');
    }

    asnkit_spec_free(jer);
    asnkit_spec_free(der);
}

static void test_numeric_enums(const char *types)
{
    asnkit_load_options *options = asnkit_load_options_new();
    asnkit_spec *spec;
    asnkit_value *decoded;
    int64_t integer;

    OK(asnkit_load_options_set_numeric_enums(options, 1));
    OK(asnkit_spec_load_file(types, "uper", options, &spec));
    asnkit_load_options_free(options);
    decoded = round_trip(spec, "Color", asnkit_value_new_integer(1), NULL);
    OK(asnkit_value_integer(decoded, &integer));
    CHECK(integer == 1);
    asnkit_value_free(decoded);
    asnkit_spec_free(spec);
}

static void test_protected_files(void)
{
    asnkit_load_options *options;
    uint8_t signing_id[ASNKIT_KEY_ID_SIZE];
    uint8_t encryption_id[ASNKIT_KEY_ID_SIZE];
    uint8_t signing_key[ASNKIT_KEY_SIZE];
    uint8_t encryption_key[ASNKIT_KEY_SIZE];
    uint8_t other_key[ASNKIT_KEY_SIZE];

    hex("d543cae4811fc8b033704dc6e7896ff7", signing_id, ASNKIT_KEY_ID_SIZE);
    hex("16cde2f77598fc5472df7570a510f54d", encryption_id, ASNKIT_KEY_ID_SIZE);
    read_key("signing.pub", signing_key);
    read_key("encryption.key", encryption_key);
    memset(other_key, 1, sizeof(other_key));

    OK(load_corpus("plain.asni", NULL));
    CHECK(load_corpus("signed.asni", NULL) == ASNKIT_ERROR_LOAD);
    CHECK(load_corpus("encrypted.asni", NULL) == ASNKIT_ERROR_LOAD);
    CHECK(load_corpus("bound.asni", NULL) == ASNKIT_ERROR_LOAD);

    options = asnkit_load_options_new();
    OK(asnkit_load_options_add_verification_key(options, signing_id, signing_key));
    OK(load_corpus("signed.asni", options));
    CHECK(load_corpus("plain.asni", options) == ASNKIT_ERROR_LOAD);
    OK(asnkit_load_options_add_decryption_key(options, encryption_id, encryption_key));
    OK(asnkit_load_options_set_bound_to(options, "corpus"));
    OK(load_corpus("signed-encrypted-bound.asni", options));
    OK(asnkit_load_options_set_bound_to(options, "elsewhere"));
    CHECK(load_corpus("signed-encrypted-bound.asni", options) == ASNKIT_ERROR_LOAD);
    asnkit_load_options_free(options);

    options = asnkit_load_options_new();
    OK(asnkit_load_options_add_verification_key(options, signing_id, other_key));
    CHECK(load_corpus("signed.asni", options) == ASNKIT_ERROR_LOAD);
    asnkit_load_options_free(options);

    options = asnkit_load_options_new();
    OK(asnkit_load_options_set_accept_any_signature(options));
    OK(asnkit_load_options_set_accept_any_binding(options));
    OK(load_corpus("signed.asni", options));
    OK(load_corpus("signed-bound.asni", options));
    CHECK(asnkit_load_options_add_verification_key(options, NULL, signing_key)
          == ASNKIT_ERROR_ARGUMENT);
    asnkit_load_options_free(options);
}

/* What the nonce callback was called with. */
typedef struct nonces {
    uint32_t minimum;
    uint32_t seen;
    int calls;
} nonces;

static int accept_from_minimum(uint32_t nonce, void *context)
{
    nonces *state = context;

    state->seen = nonce;
    state->calls++;

    return nonce >= state->minimum;
}

/* A file with a nonce, made as tests/corpus.rs of the runtime for Rust
 * makes it: plain.asni with the flag of a nonce and the nonce, of 24
 * bits, in its header. It is not signed, so nothing else has to change. */
static void test_nonces(void)
{
    size_t length;
    uint8_t *plain = read_corpus("plain.asni", &length);
    uint8_t *with_nonce = malloc(length);
    asnkit_load_options *options;
    nonces state = { 0, 0, 0 };

    CHECK(with_nonce != NULL && length > 11);
    memcpy(with_nonce, plain, length);
    with_nonce[7] |= 0x04;
    with_nonce[8] = 0x01;
    with_nonce[9] = 0x02;
    with_nonce[10] = 0x03;

    /* Rejected unless a nonce is expected. */
    CHECK(load_bytes(with_nonce, length, NULL) == ASNKIT_ERROR_LOAD);
    CHECK(strstr(asnkit_last_error(), "nonce") != NULL);

    options = asnkit_load_options_new();
    OK(asnkit_load_options_set_nonce(options, 0x010203));
    OK(load_bytes(with_nonce, length, options));
    OK(asnkit_load_options_set_nonce(options, 0x010204));
    CHECK(load_bytes(with_nonce, length, options) == ASNKIT_ERROR_LOAD);
    /* A file with no nonce, when one is expected. */
    OK(asnkit_load_options_set_nonce(options, 1));
    CHECK(load_bytes(plain, length, options) == ASNKIT_ERROR_LOAD);
    asnkit_load_options_free(options);

    /* Accepted as the callback says, which is given the nonce and the
     * context. */
    options = asnkit_load_options_new();
    state.minimum = 0x010203;
    OK(asnkit_load_options_set_nonce_callback(options, accept_from_minimum, &state));
    OK(load_bytes(with_nonce, length, options));
    CHECK(state.calls == 1 && state.seen == 0x010203);
    state.minimum = 0x010204;
    CHECK(load_bytes(with_nonce, length, options) == ASNKIT_ERROR_LOAD);
    CHECK(state.calls == 2);
    CHECK(asnkit_load_options_set_nonce_callback(options, NULL, &state) == ASNKIT_ERROR_ARGUMENT);
    asnkit_load_options_free(options);

    /* Any nonce, and also none. */
    options = asnkit_load_options_new();
    OK(asnkit_load_options_set_accept_any_nonce(options));
    OK(load_bytes(with_nonce, length, options));
    OK(load_bytes(plain, length, options));
    /* The last setter of the nonce is the one. */
    OK(asnkit_load_options_set_nonce(options, 0x010204));
    CHECK(load_bytes(with_nonce, length, options) == ASNKIT_ERROR_LOAD);
    asnkit_load_options_free(options);

    free(with_nonce);
    free(plain);
}

int main(int argc, const char *argv[])
{
    if (argc != 3) {
        fprintf(stderr, "usage: %s TYPES.ASNI CORPUS\n", argv[0]);
        return 2;
    }

    corpus = argv[2];

    test_errors(argv[1]);
    test_values(argv[1]);
    test_numeric_enums(argv[1]);
    test_protected_files();
    test_nonces();

    printf("ok\n");

    return 0;
}
