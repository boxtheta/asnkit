/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * A second translation unit with include/asnkitrt.hpp, linked with
 * tests/api.cpp: a function of the header that is not inline is then
 * defined twice, and the link fails.
 */

#include "asnkitrt.hpp"

/* A Question, made here and read in tests/api.cpp. */
asnkit::value odr_question()
{
    return asnkit::value::map({{"id", 1}, {"question", "Is 1+1=3?"}});
}

/* With std and asnkit both used, no name of asnkit is one of std: the
 * error of a range is asn_range_error, and not std::range_error. */
namespace both {

using namespace std;
using namespace asnkit;

static_assert(is_base_of_v<error, asn_range_error>);
static_assert(is_base_of_v<runtime_error, range_error>);
static_assert(!is_base_of_v<error, range_error>);
static_assert(is_base_of_v<error, load_error> && is_base_of_v<error, encode_error>
              && is_base_of_v<error, decode_error> && is_base_of_v<error, constraints_error>
              && is_base_of_v<error, unsupported_error> && is_base_of_v<error, no_such_type_error>
              && is_base_of_v<error, argument_error> && is_base_of_v<error, kind_error>
              && is_base_of_v<error, io_error> && is_base_of_v<error, panic_error>);
static_assert(is_convertible_v<value, value_view> && is_move_constructible_v<spec>);
static_assert(sizeof(bytes_view) > 0 && sizeof(bits_view) > 0 && sizeof(date) > 0
              && sizeof(time_of_day) > 0 && sizeof(date_time) > 0 && sizeof(options) > 0
              && sizeof(load_options) > 0 && sizeof(kind) > 0);

} // namespace both
