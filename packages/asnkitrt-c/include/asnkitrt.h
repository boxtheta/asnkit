/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * asnkitrt.h: the runtime of asnkit for C.
 *
 * It loads the .asni files that asnkitcc makes, and encodes and decodes
 * ASN.1 with them, in BER, DER, OER, PER, UPER, JER and XER. It is the
 * runtime for Rust, built as libasnkitrt_c (shared and static), and
 * does what that does.
 *
 *     asnkit_spec *foo;
 *
 *     if (asnkit_spec_load_file("foo.asni", "ber", NULL, &foo) != ASNKIT_OK) {
 *         fprintf(stderr, "%s\n", asnkit_last_error());
 *     }
 *
 * Conventions:
 *
 * - A function returns an asnkit_status, ASNKIT_OK or an error, and
 *   writes what it gives through its last arguments; or it returns a
 *   handle, which is NULL on an error. Either way the message of an
 *   error is then asnkit_last_error(), of the thread it happened on.
 * - Handles are opaque. What a function gives is owned by the caller and
 *   freed with the function named for it (asnkit_spec_free(),
 *   asnkit_value_free(), asnkit_bytes_free()), unless it says it is
 *   borrowed. The free functions do nothing for NULL.
 * - Text that is given is UTF-8 with a NUL at its end, unless it comes
 *   with a length. Text that is taken from a value comes with a length
 *   and has no NUL at its end, as it may have one in it: print it with
 *   "%.*s".
 * - A pointer to data may be NULL when its length is 0.
 * - Nothing panics or unwinds into C. A bug of the runtime returns
 *   ASNKIT_ERROR_PANIC.
 * - A specification may be used from several threads at once. A value
 *   may be read from several threads, and changed from one only.
 *
 * tests/header.rs holds this file to the functions of src/, and
 * tests/c.rs compiles and runs examples/question.c with it.
 */

#ifndef ASNKITRT_H
#define ASNKITRT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/* Statuses ------------------------------------------------------------ */

typedef int asnkit_status;

enum {
    ASNKIT_OK = 0,
    /* The .asni file could not be read, or was not what the load
     * options expect: signed, encrypted, bound or with a nonce. */
    ASNKIT_ERROR_LOAD = 1,
    /* A value that can not be encoded as its type. */
    ASNKIT_ERROR_ENCODE = 2,
    /* Data that can not be decoded; see asnkit_last_error_offset(). */
    ASNKIT_ERROR_DECODE = 3,
    /* A value that does not fulfil the constraints of its type. */
    ASNKIT_ERROR_CONSTRAINTS = 4,
    /* Something the runtime does not do (yet), or a codec that does not
     * do it, as an indent for BER. */
    ASNKIT_ERROR_UNSUPPORTED = 5,
    /* A type that is not in the specification, or that more than one
     * module has and that is asked for with no module. */
    ASNKIT_ERROR_NO_SUCH_TYPE = 6,
    /* A NULL pointer, text that is not UTF-8, an unknown codec and so
     * on. */
    ASNKIT_ERROR_ARGUMENT = 7,
    /* A value that is not of the kind asked for. */
    ASNKIT_ERROR_KIND = 8,
    /* A number that does not fit, an index that is out of range, or a
     * buffer that is too small. */
    ASNKIT_ERROR_RANGE = 9,
    /* A file that could not be read. */
    ASNKIT_ERROR_IO = 10,
    /* A bug of the runtime, caught before it got to C. */
    ASNKIT_ERROR_PANIC = 11
};

/* The message of the last error of this thread, or "" if there was
 * none. It is valid until the next error of the thread. */
const char *asnkit_last_error(void);

/* If the last error of this thread was ASNKIT_ERROR_DECODE, writes where
 * it was found to *offset, in bytes, or in bits for PER and UPER, and
 * returns 1. Returns 0 otherwise. */
int asnkit_last_error_offset(size_t *offset);

/* The version of the runtime, as "1.2.3". */
const char *asnkit_version(void);

/* Frees a buffer of `length` bytes that asnkit_encode() gave. */
void asnkit_bytes_free(uint8_t *data, size_t length);

/* Loading -------------------------------------------------------------- */

/* A specification, loaded for one codec. */
typedef struct asnkit_spec asnkit_spec;

/* What a file has to be to be loaded, and how it is. By default only a
 * file that is not signed, not encrypted, with no nonce and not bound is
 * loaded; see docs/file_formats.md. The setters return
 * ASNKIT_ERROR_ARGUMENT for a NULL argument. */
typedef struct asnkit_load_options asnkit_load_options;

/* The sizes of the ids of keys and of keys, in bytes. */
#define ASNKIT_KEY_ID_SIZE 16
#define ASNKIT_KEY_SIZE 32

/* Tells if `nonce` is accepted, with nonzero. */
typedef int (*asnkit_nonce_callback)(uint32_t nonce, void *context);

asnkit_load_options *asnkit_load_options_new(void);
void asnkit_load_options_free(asnkit_load_options *options);

/* ENUMERATED values are integers, and not names. */
asnkit_status asnkit_load_options_set_numeric_enums(asnkit_load_options *options, int numeric_enums);
/* DER decodes what BER allows. */
asnkit_status asnkit_load_options_set_lenient(asnkit_load_options *options, int lenient);

/* Loads a file with a nonce, only if it is this one. */
asnkit_status asnkit_load_options_set_nonce(asnkit_load_options *options, uint32_t nonce);
/* Loads a file with a nonce, only if `accept` accepts it. It is called
 * while asnkit_spec_load() runs, with `context`. */
asnkit_status asnkit_load_options_set_nonce_callback(asnkit_load_options *options,
                                                     asnkit_nonce_callback accept,
                                                     void *context);
/* Loads a file whatever its nonce is. */
asnkit_status asnkit_load_options_set_accept_any_nonce(asnkit_load_options *options);

/* Loads a file only if it is bound to `bound_to`, which is copied. */
asnkit_status asnkit_load_options_set_bound_to(asnkit_load_options *options, const char *bound_to);
/* Loads a file whatever it is bound to. */
asnkit_status asnkit_load_options_set_accept_any_binding(asnkit_load_options *options);

/* Loads a file only if it is signed by one of the Ed25519 public keys
 * added, of ASNKIT_KEY_ID_SIZE and ASNKIT_KEY_SIZE bytes, which are
 * copied. */
asnkit_status asnkit_load_options_add_verification_key(asnkit_load_options *options,
                                                       const uint8_t *id,
                                                       const uint8_t *public_key);
/* Loads a file whether it is signed or not, and does not check the
 * signature. For looking at a file. */
asnkit_status asnkit_load_options_set_accept_any_signature(asnkit_load_options *options);

/* Loads a file only if it is encrypted with one of the AES-256 keys
 * added, which are copied. */
asnkit_status asnkit_load_options_add_decryption_key(asnkit_load_options *options,
                                                     const uint8_t *id,
                                                     const uint8_t *secret_key);

/* Loads the `length` bytes of an .asni file for `codec`: "ber", "der",
 * "oer", "per", "uper", "jer", "xer" or "gser", or "ber"
 * if NULL. `options` may be NULL for the default, and is not kept. */
asnkit_status asnkit_spec_load(const uint8_t *asni,
                               size_t length,
                               const char *codec,
                               const asnkit_load_options *options,
                               asnkit_spec **spec);

/* The same as asnkit_spec_load(), with the .asni file at `path`. */
asnkit_status asnkit_spec_load_file(const char *path,
                                    const char *codec,
                                    const asnkit_load_options *options,
                                    asnkit_spec **spec);

void asnkit_spec_free(asnkit_spec *spec);

/* The codec the specification is loaded for, as "ber", borrowed from
 * it. */
const char *asnkit_spec_codec(const asnkit_spec *spec);

/* Values --------------------------------------------------------------- */

/* A value of an ASN.1 type, as those of the runtime for Python:
 *
 *   ASNKIT_NULL       NULL, and an unknown value of an extensible
 *                     ENUMERATED
 *   ASNKIT_BOOLEAN    BOOLEAN
 *   ASNKIT_INTEGER    INTEGER, of any size, and ENUMERATED with
 *                     numeric enums
 *   ASNKIT_REAL       REAL
 *   ASNKIT_TEXT       character strings, OBJECT IDENTIFIER as "1.2.3",
 *                     ENUMERATED by name, DURATION
 *   ASNKIT_BYTES      OCTET STRING, ANY and open types, encoded
 *   ASNKIT_BITS       BIT STRING
 *   ASNKIT_LIST       SEQUENCE OF, SET OF
 *   ASNKIT_MAP        SEQUENCE, SET, EXTERNAL: the members that are
 *                     there, in order
 *   ASNKIT_CHOICE     CHOICE: the name of the alternative and its value
 *   ASNKIT_DATE_TIME  UTCTime, GeneralizedTime, DATE-TIME
 *   ASNKIT_DATE       DATE
 *   ASNKIT_TIME       TIME-OF-DAY
 *
 * A value that is made (asnkit_value_new_*()) or decoded is owned by the
 * caller, until it is given to a list, a map or a choice, which then own
 * it. The functions that take a value take it even when they fail, so it
 * is never freed after. What the accessors give is borrowed from the
 * value it is in, valid as long as that is and is not changed.
 *
 * A constructor returns NULL if it fails, which is for an argument that
 * is wrong (or for running out of memory, which aborts). */
typedef struct asnkit_value asnkit_value;

enum {
    ASNKIT_NULL = 0,
    ASNKIT_BOOLEAN = 1,
    ASNKIT_INTEGER = 2,
    ASNKIT_REAL = 3,
    ASNKIT_TEXT = 4,
    ASNKIT_BYTES = 5,
    ASNKIT_BITS = 6,
    ASNKIT_LIST = 7,
    ASNKIT_MAP = 8,
    ASNKIT_CHOICE = 9,
    ASNKIT_DATE_TIME = 10,
    ASNKIT_DATE = 11,
    ASNKIT_TIME = 12
};

typedef struct asnkit_date {
    int32_t year;
    /* From 1. */
    uint8_t month;
    uint8_t day;
} asnkit_date;

typedef struct asnkit_time {
    uint8_t hour;
    uint8_t minute;
    uint8_t second;
    uint32_t microsecond;
    /* Nonzero if the time has an offset, and zero for a local time. */
    int has_offset;
    /* In minutes east of UTC. */
    int32_t offset;
} asnkit_time;

asnkit_value *asnkit_value_new_null(void);
asnkit_value *asnkit_value_new_boolean(int boolean);
asnkit_value *asnkit_value_new_integer(int64_t integer);
/* An integer of any size, from its two's complement, most significant
 * byte first. No bytes are 0. */
asnkit_value *asnkit_value_new_integer_bytes(const uint8_t *data, size_t length);
asnkit_value *asnkit_value_new_real(double real);
/* `length` bytes of UTF-8, which are copied. */
asnkit_value *asnkit_value_new_text(const char *text, size_t length);
asnkit_value *asnkit_value_new_bytes(const uint8_t *data, size_t length);
/* The first `bits` bits of the `length` bytes at `data`, which have to be
 * as many as the bits take. */
asnkit_value *asnkit_value_new_bits(const uint8_t *data, size_t length, size_t bits);
/* An empty list, for asnkit_list_push(). */
asnkit_value *asnkit_value_new_list(void);
/* An empty map, for asnkit_map_set(). */
asnkit_value *asnkit_value_new_map(void);
/* The alternative `name` with `inner`, which it takes. */
asnkit_value *asnkit_value_new_choice(const char *name, asnkit_value *inner);
asnkit_value *asnkit_value_new_date(asnkit_date date);
asnkit_value *asnkit_value_new_time(asnkit_time time);
asnkit_value *asnkit_value_new_date_time(asnkit_date date, asnkit_time time);

/* A copy of `value`, owned by the caller. */
asnkit_value *asnkit_value_clone(const asnkit_value *value);
/* Frees a value the caller owns, with all it holds. */
void asnkit_value_free(asnkit_value *value);

/* Adds `item`, which it takes, to the end of `list`. */
asnkit_status asnkit_list_push(asnkit_value *list, asnkit_value *item);
/* Sets the member `name` of `map` to `member`, which it takes, in place
 * of the member of that name if there is one. */
asnkit_status asnkit_map_set(asnkit_value *map, const char *name, asnkit_value *member);

/* The kind of `value`, as ASNKIT_MAP, or -1 if it is NULL. */
int asnkit_value_kind(const asnkit_value *value);

/* The accessors return ASNKIT_ERROR_KIND for a value of another kind. */

asnkit_status asnkit_value_boolean(const asnkit_value *value, int *boolean);
/* ASNKIT_ERROR_RANGE if the integer does not fit in 64 bits; then
 * asnkit_value_integer_bytes() gives it. */
asnkit_status asnkit_value_integer(const asnkit_value *value, int64_t *integer);
/* Writes the two's complement of an integer, most significant byte first
 * and as short as it can be, to the `capacity` bytes at `data`, and its
 * length to *length. If `data` is NULL or too small, writes only the
 * length and returns ASNKIT_ERROR_RANGE. */
asnkit_status asnkit_value_integer_bytes(const asnkit_value *value,
                                         uint8_t *data,
                                         size_t capacity,
                                         size_t *length);
asnkit_status asnkit_value_real(const asnkit_value *value, double *real);
/* Borrowed, with no NUL at its end. */
asnkit_status asnkit_value_text(const asnkit_value *value, const char **text, size_t *length);
/* Borrowed. */
asnkit_status asnkit_value_bytes(const asnkit_value *value, const uint8_t **data, size_t *length);
/* Borrowed: the bytes, and how many bits of them there are. */
asnkit_status asnkit_value_bits(const asnkit_value *value,
                                const uint8_t **data,
                                size_t *length,
                                size_t *bits);
/* The number of items of a list, or of members of a map. */
asnkit_status asnkit_value_length(const asnkit_value *value, size_t *length);
/* The item `index` of a list, borrowed, or NULL if there is none. */
const asnkit_value *asnkit_list_get(const asnkit_value *list, size_t index);
/* The member `name` of a map, borrowed, or NULL if it is not there. */
const asnkit_value *asnkit_map_get(const asnkit_value *map, const char *name);
/* The name and the value of the member `index` of a map, borrowed;
 * ASNKIT_ERROR_RANGE if there is none. */
asnkit_status asnkit_map_member(const asnkit_value *map,
                                size_t index,
                                const char **name,
                                size_t *name_length,
                                const asnkit_value **member);
/* The name of the alternative of a choice, and its value, borrowed. */
asnkit_status asnkit_choice_get(const asnkit_value *choice,
                                const char **name,
                                size_t *name_length,
                                const asnkit_value **inner);
asnkit_status asnkit_value_date(const asnkit_value *value, asnkit_date *date);
asnkit_status asnkit_value_time(const asnkit_value *value, asnkit_time *time);
asnkit_status asnkit_value_date_time(const asnkit_value *value, asnkit_date *date, asnkit_time *time);

/* Encoding and decoding ------------------------------------------------- */

/* How a value is encoded or decoded. NULL in place of these is
 * ASNKIT_OPTIONS_INIT. */
typedef struct asnkit_options {
    /* The module of the type, or NULL if only one module has a type of
     * that name. */
    const char *module;
    /* Nonzero to check the value against the constraints of its type,
     * before encoding it or after decoding it. */
    int check_constraints;
    /* The number of spaces for each level of JER and XER, or negative
     * for none. Other codecs return ASNKIT_ERROR_UNSUPPORTED for one. */
    int indent;
} asnkit_options;

#define ASNKIT_OPTIONS_INIT { NULL, 0, -1 }

/* Encodes `value` as the type `name`. *data is then owned by the
 * caller, to free with asnkit_bytes_free(*data, *length). It is followed
 * by a NUL that *length does not count, so that JER and XER are a string
 * of C too. */
asnkit_status asnkit_encode(const asnkit_spec *spec,
                            const char *name,
                            const asnkit_value *value,
                            const asnkit_options *options,
                            uint8_t **data,
                            size_t *length);

/* Decodes the `length` bytes at `data` as the type `name`. *value is
 * then owned by the caller. */
asnkit_status asnkit_decode(const asnkit_spec *spec,
                            const char *name,
                            const uint8_t *data,
                            size_t length,
                            const asnkit_options *options,
                            asnkit_value **value);

/* Decodes the first value of `data`, and writes the number of bytes it
 * was encoded in to *consumed. BER and DER only, and with no module. */
asnkit_status asnkit_decode_with_length(const asnkit_spec *spec,
                                        const char *name,
                                        const uint8_t *data,
                                        size_t length,
                                        const asnkit_options *options,
                                        asnkit_value **value,
                                        size_t *consumed);

/* Writes the length in bytes of the first value of `data` to
 * *value_length, and 1 to *known; or 0 to both if there is not enough
 * data yet to tell. BER and DER only. */
asnkit_status asnkit_decode_length(const asnkit_spec *spec,
                                   const uint8_t *data,
                                   size_t length,
                                   size_t *value_length,
                                   int *known);

/* Checks `value` against the constraints of the type `name`, and returns
 * ASNKIT_ERROR_CONSTRAINTS if it does not fulfil them. Only `module` of
 * the options is used. */
asnkit_status asnkit_check_constraints(const asnkit_spec *spec,
                                       const char *name,
                                       const asnkit_value *value,
                                       const asnkit_options *options);

#ifdef __cplusplus
}
#endif

#endif /* ASNKITRT_H */
