/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * The API of include/asnkitrt.hpp, from C++: what tests/api.c checks of
 * the header of C, and what is of C++ (ownership, the errors thrown, the
 * constructors and ranges). It is run by tests/c.rs with the path of
 * packages/asnkitrt-wasm/tests/fixtures/types.asni and of examples/corpus,
 * linked with tests/api_odr.cpp, and fails with the line of the first
 * check that does.
 */

#include <array>
#include <cstdint>
#include <cstdio>
#include <cstdlib>
#include <cstring>
#include <fstream>
#include <iterator>
#include <limits>
#include <string>
#include <string_view>
#include <type_traits>
#include <typeinfo>
#include <utility>
#include <vector>

#if __cplusplus >= 202002L && __has_include(<span>)
#include <span>
#endif

#include "asnkitrt.hpp"

/* Of tests/api_odr.cpp. */
asnkit::value odr_question();

namespace {

using asnkit::value;
using asnkit::value_view;
using bytes = std::vector<std::uint8_t>;
using namespace std::string_view_literals;

[[noreturn]] void fail(int line, const std::string &what)
{
    std::fprintf(stderr, "%s:%d: %s\n", __FILE__, line, what.c_str());
    std::exit(1);
}

#define CHECK(...)                                                                                 \
    do {                                                                                           \
        if (!(__VA_ARGS__)) {                                                                      \
            fail(__LINE__, #__VA_ARGS__);                                                          \
        }                                                                                          \
    } while (0)

/* Runs `body`, which has to throw exactly `Error`, of the status `code`. */
template <class Error, class Body>
void throws(int line, asnkit_status code, Body body)
{
    try {
        body();
    } catch (const Error &error) {
        if (typeid(error) != typeid(Error)) {
            fail(line, std::string("thrown as a subclass: ") + error.what());
        }

        if (error.code() != code) {
            fail(line, "thrown with the status " + std::to_string(error.code()));
        }

        return;
    } catch (const std::exception &error) {
        fail(line, std::string("thrown as another exception: ") + error.what());
    }

    fail(line, "nothing thrown");
}

#define THROWS(Error, code, ...) throws<asnkit::Error>(__LINE__, code, [&] { __VA_ARGS__; })

std::string corpus;

/* Compile time: what converts to what. */

static_assert(std::is_nothrow_move_constructible_v<value>);
static_assert(std::is_nothrow_move_assignable_v<value>);
static_assert(std::is_copy_constructible_v<value>);
static_assert(std::is_trivially_copyable_v<value_view>);
static_assert(std::is_convertible_v<value, value_view>);
static_assert(!std::is_convertible_v<value_view, value>);
static_assert(!std::is_copy_constructible_v<asnkit::spec>);
static_assert(std::is_nothrow_move_constructible_v<asnkit::spec>);
static_assert(std::is_nothrow_move_assignable_v<asnkit::spec>);

static_assert(std::is_convertible_v<const char *, value>);
static_assert(std::is_convertible_v<std::string, value>);
static_assert(std::is_convertible_v<std::string_view, value>);
static_assert(std::is_convertible_v<bool, value>);
static_assert(std::is_convertible_v<std::uint64_t, value>);
static_assert(std::is_convertible_v<std::uint8_t, value>);
static_assert(std::is_convertible_v<float, value>);
static_assert(!std::is_constructible_v<value, char>);
static_assert(!std::is_constructible_v<value, wchar_t>);
static_assert(!std::is_constructible_v<value, char32_t>);
static_assert(!std::is_constructible_v<value, void *>);

static_assert(std::is_convertible_v<bytes, asnkit::bytes_view>);
static_assert(std::is_convertible_v<std::vector<unsigned char>, asnkit::bytes_view>);
static_assert(std::is_convertible_v<std::array<std::uint8_t, 3>, asnkit::bytes_view>);
static_assert(std::is_convertible_v<const std::uint8_t (&)[3], asnkit::bytes_view>);
static_assert(!std::is_convertible_v<std::string, asnkit::bytes_view>);
static_assert(!std::is_convertible_v<std::string_view, asnkit::bytes_view>);
static_assert(!std::is_convertible_v<std::vector<char>, asnkit::bytes_view>);
static_assert(!std::is_convertible_v<const char *, asnkit::bytes_view>);
static_assert(!std::is_convertible_v<const char (&)[3], asnkit::bytes_view>);
#if __cplusplus >= 202002L && __has_include(<span>)
static_assert(std::is_convertible_v<std::span<const std::uint8_t>, asnkit::bytes_view>);
#endif

static_assert(std::is_same_v<decltype(std::declval<value &>().set("a", 1)), value &>);
/* A temporary that is chained is returned as a value, and not as a
 * reference to what is gone at the end of the statement. */
static_assert(std::is_same_v<decltype(std::declval<value>().set("a", 1)), value>);
static_assert(std::is_same_v<decltype(std::declval<value &>().push(1)), value &>);
static_assert(std::is_same_v<decltype(std::declval<value>().push(1)), value>);
static_assert(std::is_same_v<decltype(std::declval<const value &>()[0]), value_view>);
static_assert(std::is_same_v<decltype(std::declval<const value &>()["name"]), value_view>);
static_assert(std::is_same_v<decltype(std::declval<value_view>()[0]), value_view>);
static_assert(std::is_same_v<decltype(std::declval<value_view>()["name"]), value_view>);

/* What can be called with what, to hold what does not compile. */
template <class F, class... Arguments>
constexpr bool calls = std::is_invocable_v<F, Arguments...>;

constexpr auto make_boolean = [](auto &&x) -> decltype(value::boolean(std::forward<decltype(x)>(x))) {
    return value::boolean(std::forward<decltype(x)>(x));
};
constexpr auto make_integer = [](auto &&x) -> decltype(value::integer(std::forward<decltype(x)>(x))) {
    return value::integer(std::forward<decltype(x)>(x));
};
constexpr auto make_real = [](auto &&x) -> decltype(value::real(std::forward<decltype(x)>(x))) {
    return value::real(std::forward<decltype(x)>(x));
};
constexpr auto items_of = [](auto &&x) -> decltype(std::forward<decltype(x)>(x).items()) {
    return std::forward<decltype(x)>(x).items();
};
constexpr auto members_of = [](auto &&x) -> decltype(std::forward<decltype(x)>(x).members()) {
    return std::forward<decltype(x)>(x).members();
};

/* The functions named for the kinds take what the constructors do, and
 * nothing that converts to it. */
static_assert(calls<decltype(make_boolean), bool>);
static_assert(!calls<decltype(make_boolean), const char *>);
static_assert(!calls<decltype(make_boolean), int>);
static_assert(calls<decltype(make_integer), int>);
static_assert(calls<decltype(make_integer), std::uint64_t>);
static_assert(!calls<decltype(make_integer), double>);
static_assert(!calls<decltype(make_integer), bool>);
static_assert(!calls<decltype(make_integer), char>);
static_assert(calls<decltype(make_real), double>);
static_assert(calls<decltype(make_real), int>);
static_assert(!calls<decltype(make_real), bool>);

/* The items and the members of a value that is temporary are not given,
 * as it is freed before they are read; those of a view are. */
static_assert(calls<decltype(items_of), value &>);
static_assert(calls<decltype(items_of), const value &>);
static_assert(!calls<decltype(items_of), value>);
static_assert(!calls<decltype(members_of), value>);
static_assert(calls<decltype(items_of), value_view>);
static_assert(calls<decltype(members_of), value_view>);

/* Two integers are not a pointer and a length. */
static_assert(!std::is_constructible_v<asnkit::bytes_view, int, int>);
static_assert(std::is_constructible_v<asnkit::bytes_view, std::nullptr_t, int>);
static_assert(std::is_constructible_v<asnkit::bytes_view, const std::uint8_t *, std::size_t>);

static_assert(static_cast<int>(asnkit::kind::null) == ASNKIT_NULL);
static_assert(static_cast<int>(asnkit::kind::map) == ASNKIT_MAP);
static_assert(static_cast<int>(asnkit::kind::object_identifier) == ASNKIT_OBJECT_IDENTIFIER);

/* Encodes and decodes `value`, and returns what was decoded. */
value round_trip(const asnkit::spec &spec, std::string_view type, value_view value,
                 const asnkit::options &options = {})
{
    std::vector<std::uint8_t> data = spec.encode(type, value, options);

    return spec.decode(type, data, options);
}

bytes read_file(const std::string &path)
{
    std::ifstream file(path, std::ios::binary);

    CHECK(file);
    return bytes(std::istreambuf_iterator<char>(file), std::istreambuf_iterator<char>());
}

/* Round-trips a Question with `spec`. */
void ask(const asnkit::spec &spec)
{
    value decoded = round_trip(spec, "Question", odr_question());

    CHECK(decoded["question"].as_text() == "Is 1+1=3?");
}

void test_errors(const std::string &types)
{
    const std::uint8_t garbage[] = {1, 2, 3};

    CHECK(!asnkit::version().empty());
    CHECK(asnkit::version() == asnkit_version());

    try {
        asnkit::spec::load_file(types, "nope");
        fail(__LINE__, "nothing thrown");
    } catch (const asnkit::argument_error &error) {
        CHECK(std::strstr(error.what(), "Unsupported codec 'nope'") != nullptr);
        CHECK(error.code() == ASNKIT_ERROR_ARGUMENT);
    }

    THROWS(load_error, ASNKIT_ERROR_LOAD, asnkit::spec::load(garbage));
    THROWS(io_error, ASNKIT_ERROR_IO, asnkit::spec::load_file("/nonexistent/types.asni"));
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, asnkit::spec::load_file(types, "b\0er"sv));
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT,
           asnkit::spec::load_file(std::string("types\0.asni", 11)));

    /* A handle that C gives as NULL is thrown as its status. */
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value::text("\xff"));
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value(static_cast<const char *>(nullptr)));
    THROWS(asn_range_error, ASNKIT_ERROR_RANGE, value::bits(garbage, 8));
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value::object_identifier("1"));

    /* A status this header does not know is an error, and no more. */
    THROWS(error, 99, asnkit::detail::throw_error(99));

    /* Changing a value of the wrong kind. */
    {
        value list = value::list();
        value integer = 1;

        THROWS(kind_error, ASNKIT_ERROR_KIND, list.set("a", value()));
        THROWS(kind_error, ASNKIT_ERROR_KIND, integer.push(2));
        CHECK(list.size() == 0);
        CHECK(integer.as_integer() == 1);
    }

    /* Names with a NUL in them are refused, and not cut at it. */
    {
        value map = value::map();
        asnkit::options in_module;

        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, map.set("a\0b"sv, 1));
        CHECK(map.size() == 0);
        map.set("a", 1);
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, map.at("a\0b"sv));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, map.find("a\0b"sv));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, map.contains("a\0b"sv));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value::choice("a\0b"sv, 1));

        asnkit::spec der = asnkit::spec::load_file(types, "der");

        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.encode("Big\0Real"sv, value(1)));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.decode("Big\0Real"sv, garbage));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT,
               der.check_constraints("Big\0Real"sv, value(1)));
        in_module.module = std::string("Types\0Other", 11);
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.encode("Big", value(1), in_module));
    }
}

void test_ownership(const std::string &types)
{
    /* A copy is independent of what it is copied from. */
    {
        value original = value::map({{"id", 1}});
        value copy = original;

        copy.set("id", 2);
        CHECK(original["id"].as_integer() == 1);
        CHECK(copy["id"].as_integer() == 2);
        CHECK(copy.get() != original.get());

        value assigned;

        assigned = original;
        original.set("id", 3);
        CHECK(assigned["id"].as_integer() == 1);

        value &same = assigned;

        assigned = same;
        CHECK(assigned["id"].as_integer() == 1);

        value cloned = value_view(original).clone();

        CHECK(cloned["id"].as_integer() == 3);
    }

    /* What is moved from is empty, may be assigned to and copied, and
     * throws for anything else. */
    {
        value first = 7;
        const asnkit_value *handle = first.get();
        value second = std::move(first);
        value list = value::list();
        asnkit::spec der = asnkit::spec::load_file(types, "der");

        CHECK(second.get() == handle);
        CHECK(first.get() == nullptr);
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.kind());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.is_null());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.as_integer());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.as_text());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.size());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.at(0));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.find("a"));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.items());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.clone());
        {
            value copy = first;
            value assigned = 1;

            CHECK(copy.get() == nullptr);
            assigned = first;
            CHECK(assigned.get() == nullptr);
        }
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.push(1));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, first.set("a", 1));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, list.push(std::move(first)));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value::choice("a", std::move(first)));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.encode("Big", first));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.check_constraints("Big", first));
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value_view().kind());
        CHECK(list.size() == 0);

        first = 5;
        CHECK(first.as_integer() == 5);
        second = std::move(first);
        CHECK(second.as_integer() == 5);
        first = value::text("again");
        CHECK(first.as_text() == "again");
    }

    /* A specification moves, and is empty once moved from. */
    {
        asnkit::spec der = asnkit::spec::load_file(types, "der");
        const asnkit_spec *handle = der.get();
        asnkit::spec moved = std::move(der);
        asnkit::spec jer = asnkit::spec::load_file(types, "jer");

        CHECK(moved.get() == handle);
        CHECK(der.get() == nullptr);
        CHECK(moved.codec() == "der");
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.codec());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.encode("Big", value(1)));
        der = std::move(jer);
        CHECK(der.codec() == "jer");
        moved = std::move(der);
        CHECK(moved.codec() == "jer");
    }

    /* Handles to and from C. */
    {
        asnkit_value *raw = asnkit_value_new_integer(42);
        value adopted = value::adopt(raw);

        CHECK(adopted.get() == raw);
        CHECK(adopted.as_integer() == 42);
        CHECK(asnkit_value_kind(adopted.get()) == ASNKIT_INTEGER);

        asnkit_value *released = adopted.release();

        CHECK(released == raw);
        CHECK(adopted.get() == nullptr);
        CHECK(value_view(released).as_integer() == 42);
        asnkit_value_free(released);
        CHECK(value::adopt(nullptr).get() == nullptr);
    }
}

void test_construction()
{
    /* The constructors take what C++ has, and pick the kind. */
    CHECK(value().is_null());
    CHECK(value::null().kind() == asnkit::kind::null);
    CHECK(value("text").is_text());
    CHECK(value("text").as_text() == "text");
    CHECK(value(std::string("string")).as_text() == "string");
    CHECK(value("view"sv).as_text() == "view");
    CHECK(value(true).is_boolean());
    CHECK(value(true).as_boolean());
    CHECK(!value(false).as_boolean());
    CHECK(value::boolean(true).as_boolean());
    CHECK(value(1).as_integer() == 1);
    CHECK(value(-5LL).as_integer() == -5);
    CHECK(value(static_cast<short>(-3)).as_integer() == -3);
    CHECK(value(static_cast<std::uint8_t>(200)).as_integer() == 200);
    CHECK(value(std::numeric_limits<std::int64_t>::min()).as_integer()
          == std::numeric_limits<std::int64_t>::min());
    CHECK(value(static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max())).as_integer()
          == std::numeric_limits<std::int64_t>::max());
    CHECK(value::integer(-129).as_integer() == -129);
    CHECK(value::integer(std::uint64_t(-1)).as_unsigned() == std::uint64_t(-1));
    CHECK(value::real(1).as_real() == 1.0);
    CHECK(value::text("text").as_text() == "text");
    THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, value::text(static_cast<const char *>(nullptr)));

    /* Unsigned integers, read as such. */
    {
        static const std::uint8_t two_to_the_64[] = {0x01, 0, 0, 0, 0, 0, 0, 0, 0};

        CHECK(value(0).as_unsigned() == 0);
        CHECK(value(200).as_unsigned() == 200);
        CHECK(value(std::numeric_limits<std::int64_t>::max()).as_unsigned()
              == static_cast<std::uint64_t>(std::numeric_limits<std::int64_t>::max()));
        CHECK(value(std::uint64_t(1) << 63).as_unsigned() == std::uint64_t(1) << 63);
        CHECK(value(std::uint64_t(-1)).as_unsigned() == std::uint64_t(-1));
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, value(std::uint64_t(-1)).as_integer());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, value(-1).as_unsigned());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE,
               value(std::numeric_limits<std::int64_t>::min()).as_unsigned());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, value::integer_bytes(two_to_the_64).as_unsigned());
        THROWS(kind_error, ASNKIT_ERROR_KIND, value("1").as_unsigned());
    }

    /* A chain on a temporary is a value of its own, which a reference
     * keeps. */
    {
        const value &map = value::map().set("a", 1).set("b", 2);
        auto &&list = value::list().push(1).push(2);

        CHECK(map.size() == 2);
        CHECK(list.size() == 2);
    }

    /* Structs that are given in part. */
    {
        asnkit::time_of_day noon{12, 0, 0};
        asnkit::date_time leap{{2024, 2, 29}, {12, 30}};
        asnkit::options in_other{"Other"};

        CHECK(!noon.offset && noon.microsecond == 0);
        CHECK(leap.time.minute == 30 && !leap.time.offset);
        CHECK(in_other.module == "Other" && !in_other.check_constraints && !in_other.indent);
    }

    /* Bytes are compared as what they are. */
    {
        const bytes expected = {1, 2, 3};
        const std::uint8_t other[] = {1, 2, 4};
        value made = value::bytes(expected);

        CHECK(made.as_bytes() == expected);
        CHECK(expected == made.as_bytes());
        CHECK(made.as_bytes() != asnkit::bytes_view(other));
        CHECK(made.as_bytes() != asnkit::bytes_view(expected.data(), 2));
        CHECK(asnkit::bytes_view() == asnkit::bytes_view(nullptr, 0));
    }
    CHECK(value(0.5).is_real());
    CHECK(value(0.5).as_real() == 0.5);
    CHECK(value(0.25f).as_real() == 0.25);
    CHECK(value::real(1.5).as_real() == 1.5);

    /* An unsigned integer above INT64_MAX. */
    {
        value big = std::uint64_t(-1);
        const bytes expected = {0x00, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff, 0xff};

        CHECK(big.is_integer());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, big.as_integer());
        CHECK(big.as_integer_bytes() == expected);
        CHECK(value(std::uint64_t(1) << 63).as_integer_bytes()
              == bytes({0x00, 0x80, 0, 0, 0, 0, 0, 0, 0}));
    }

    /* Lists and maps, of initializer lists, which move what they are
     * given. */
    {
        value inner = value::list({1, 2});
        value kept = "kept";
        value list = value::list({true, false, std::move(inner), kept, "text", 0.5});

        CHECK(inner.get() == nullptr);
        CHECK(kept.as_text() == "kept");
        CHECK(list.size() == 6);
        CHECK(list[0].as_boolean() && !list[1].as_boolean());
        CHECK(list[2].size() == 2 && list[2][1].as_integer() == 2);
        CHECK(list[3].as_text() == "kept" && list[4].as_text() == "text");
        CHECK(list[5].as_real() == 0.5);
        CHECK(value::list({}).size() == 0);
        CHECK(value::map({}).size() == 0);

        value map = value::map({{"id", 1},
                                {"nested", value::map({{"list", value::list({"a", "b"})}})},
                                {"null", {}},
                                {"id", 2}});

        CHECK(map.size() == 3);
        CHECK(map["id"].as_integer() == 2);
        CHECK(map["nested"]["list"][1].as_text() == "b");
        CHECK(map["null"].is_null());
    }

    /* Builders, which chain on what can be changed and move what is
     * temporary. */
    {
        value map = value::map();
        // Not bound to a reference, which GCC 13 would take for one to the
        // temporary argument (-Wdangling-reference).
        value *same = &map.set("id", 1).set("name", "one");
        value built = value::map().set("id", 1).set("name", "one");
        value list = value::list().push(1).push("two").push(value::list().push(3));

        CHECK(same == &map);
        CHECK(map.size() == 2 && built.size() == 2);
        CHECK(built["name"].as_text() == "one");
        CHECK(list.size() == 3 && list[2][0].as_integer() == 3);
        list.push(4).push(5);
        CHECK(list.size() == 5);
    }

    /* Choices, dates and times. */
    {
        value choice = value::choice("a", 5);
        auto [name, inner] = choice.as_choice();
        asnkit::date day{2024, 2, 29};
        asnkit::time_of_day noon{12, 0, 0, 500, -60};
        asnkit::date_time when{day, noon};

        CHECK(name == "a" && inner.as_integer() == 5);
        CHECK(value::date(day).as_date() == day);
        CHECK(value::time(noon).as_time() == noon);
        CHECK(value::time(noon).as_time().offset == -60);
        CHECK(value::date_time(when).as_date_time() == when);
        CHECK(value::time(asnkit::time_of_day{}).as_time().offset == std::nullopt);
        CHECK(day != asnkit::date{2024, 3, 1});
        CHECK(noon != asnkit::time_of_day{12, 0, 0, 500, std::nullopt});
        CHECK(when != asnkit::date_time{day, asnkit::time_of_day{}});
    }

    /* Empty bytes and text, which may have no pointer. */
    {
        CHECK(value::bytes(asnkit::bytes_view()).as_bytes().empty());
        CHECK(value::bytes(bytes()).as_bytes().size() == 0);
        CHECK(value::text(std::string_view()).as_text().empty());
        CHECK(value("").as_text().empty());
        CHECK(value::bits(asnkit::bytes_view(nullptr, 0), 0).as_bits().count == 0);

        const std::uint8_t data[] = {1, 2, 3};
        asnkit::bytes_view view = value::bytes(data).as_bytes();

        CHECK(view.size() == 3);
    }

    /* Ranges over items and members. */
    {
        value list = value::list({1, 2, 3});
        value map = value::map({{"a", 1}, {"b", 2}});
        std::int64_t sum = 0;
        std::string names;

        for (value_view item : list.items()) {
            sum += item.as_integer();
        }

        for (auto [name, member] : map.members()) {
            names += std::string(name);
            sum += member.as_integer();
        }

        for (const auto &[name, member] : value_view(map).members()) {
            names += std::string(name);
            (void) member;
        }

        CHECK(sum == 9);
        CHECK(names == "abab");
        CHECK(list.items().size() == 3);
        {
            value empty = value::list();

            CHECK(empty.items().empty());
        }
        THROWS(kind_error, ASNKIT_ERROR_KIND, map.items());
        THROWS(kind_error, ASNKIT_ERROR_KIND, list.members());
        THROWS(kind_error, ASNKIT_ERROR_KIND, list.at("a"));
        THROWS(kind_error, ASNKIT_ERROR_KIND, list.find("a"));
        THROWS(kind_error, ASNKIT_ERROR_KIND, map.at(0));
        THROWS(kind_error, ASNKIT_ERROR_KIND, value(1).size());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, list.at(3));
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, list[3]);
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, map["c"]);
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, map.member(2));
    }
}

void test_values(const std::string &types)
{
    asnkit::spec der = asnkit::spec::load_file(types, "der");
    asnkit::spec jer = asnkit::spec::load(read_file(types), "jer");

    CHECK(der.codec() == "der");
    CHECK(jer.codec() == "jer");

    /* CHOICE: ["b", [true, false]]. */
    {
        value decoded = round_trip(der, "Choice", value::choice("b", value::list({true, false})));

        CHECK(decoded.kind() == asnkit::kind::choice);

        // Not a structured binding, which a lambda of C++17 can not capture.
        std::pair<std::string_view, value_view> choice = decoded.as_choice();
        value_view inner = choice.second;

        CHECK(choice.first == "b");
        CHECK(inner.size() == 2);
        CHECK(inner[0].as_boolean());
        CHECK(!inner[1].as_boolean());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, inner.at(2));
        THROWS(kind_error, ASNKIT_ERROR_KIND, inner.as_integer());
    }

    /* An INTEGER of more than 64 bits. */
    {
        /* 2 ** 100. */
        const bytes two_to_the_100 = {0x10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0};
        value decoded = round_trip(der, "Big", value::integer_bytes(two_to_the_100));

        CHECK(decoded.is_integer());
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, decoded.as_integer());
        CHECK(decoded.as_integer_bytes() == two_to_the_100);

        decoded = round_trip(der, "Big", value(-129));
        CHECK(decoded.as_integer() == -129);
        CHECK(decoded.as_integer_bytes() == bytes({0xff, 0x7f}));
        THROWS(kind_error, ASNKIT_ERROR_KIND, value("1").as_integer_bytes());
    }

    CHECK(round_trip(der, "Real", value(0.5)).as_real() == 0.5);

    {
        const std::uint8_t bit_string[] = {0xa0};
        value decoded = round_trip(der, "Bits", value::bits(bit_string, 3));
        asnkit::bits_view bits = decoded.as_bits();

        CHECK(bits.bytes.size() == 1 && bits.count == 3 && bits.bytes[0] == 0xa0);

        /* What was read makes the same again. */
        value again = value::bits(bits);

        CHECK(again.as_bits().count == 3 && again.as_bits().bytes == bits.bytes);
    }

    /* Dates and times. */
    {
        asnkit::date day{2024, 2, 29};
        asnkit::time_of_day time{12, 30, 15, 0, 0};

        CHECK(round_trip(der, "Day", value::date(day)).as_date() == day);

        value decoded = round_trip(der, "When", value::date_time({day, time}));
        asnkit::date_time when = decoded.as_date_time();

        CHECK(when.date.day == 29 && when.time.hour == 12 && when.time.minute == 30);
        /* DER has times in UTC, which are read as such, as in Python. */
        CHECK(when.time.second == 15 && when.time.offset == 0);
        CHECK(when == (asnkit::date_time{day, time}));
        THROWS(kind_error, ASNKIT_ERROR_KIND, decoded.as_time());
    }

    /* An OBJECT IDENTIFIER, as text and as its contents octets. */
    {
        const bytes rsa = {0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d};
        value decoded = round_trip(der, "Oid", value::object_identifier("1.2.840.113549"));
        asnkit::bytes_view contents = decoded.as_object_identifier_bytes();

        CHECK(decoded.is_object_identifier());
        CHECK(bytes(contents.begin(), contents.end()) == rsa);
        CHECK(decoded.as_object_identifier() == "1.2.840.113549");
        THROWS(kind_error, ASNKIT_ERROR_KIND, decoded.as_text());
    }

    CHECK(round_trip(der, "Color", value("green")).as_text() == "green");

    /* Text with a NUL in it, and empty text. */
    {
        value decoded = round_trip(der, "Pair", value::list({"a\0b"sv, ""}));

        CHECK(decoded[0].as_text() == "a\0b"sv);
        CHECK(decoded[0].as_text().size() == 3);
        CHECK(decoded[1].as_text().empty());
    }

    /* A SEQUENCE, its members in order, and a member set twice. */
    value record = value::map().set("id", 1).set("name", "one").set("id", 2);

    {
        value decoded = round_trip(der, "Record", record);

        CHECK(decoded.size() == 2);

        auto [name, member] = decoded.member(0);

        CHECK(name == "id" && member.as_integer() == 2);
        CHECK(decoded.member(1).first == "name");
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, decoded.member(2));
        CHECK(!decoded.find("other"));
        CHECK(!decoded.contains("other"));
        CHECK(decoded.contains("name"));
        CHECK(decoded.find("name")->as_text() == "one");
        THROWS(asn_range_error, ASNKIT_ERROR_RANGE, decoded.at("other"));
    }

    /* BER and DER have the length of a value in it. */
    {
        bytes encoded = der.encode("Record", record);

        CHECK(der.decode_length(encoded) == encoded.size());
        CHECK(der.decode_length(asnkit::bytes_view(encoded.data(), 1)) == std::nullopt);

        auto [decoded, consumed] = der.decode_with_length("Record", encoded);

        CHECK(consumed == encoded.size());
        CHECK(decoded["id"].as_integer() == 2);

        /* With more after it, which is not of the value. */
        {
            bytes followed = encoded;

            followed.push_back(0x05);
            followed.push_back(0x00);

            auto [first, length] = der.decode_with_length("Record", followed);

            CHECK(length == encoded.size());
            CHECK(first["id"].as_integer() == 2);
            THROWS(decode_error, ASNKIT_ERROR_DECODE, der.decode("Record", followed));
        }
        THROWS(unsupported_error, ASNKIT_ERROR_UNSUPPORTED, jer.decode_length(encoded));
        THROWS(decode_error, ASNKIT_ERROR_DECODE, der.decode("Record", asnkit::bytes_view()));
    }

    /* Indented JER, and no indent for DER. */
    {
        asnkit::options indented;

        indented.indent = 4;
        CHECK(jer.encode_text("Record", record, indented).find("\n    \"id\": 2")
              != std::string::npos);
        CHECK(jer.decode_text("Record", R"({"id": 3})")["id"].as_integer() == 3);
        THROWS(unsupported_error, ASNKIT_ERROR_UNSUPPORTED, der.encode("Record", record, indented));
    }

    /* Constraints, checked when asked to. */
    {
        asnkit::options checked;
        value eight = 8;

        checked.check_constraints = true;
        THROWS(constraints_error, ASNKIT_ERROR_CONSTRAINTS, der.check_constraints("Small", eight));
        THROWS(constraints_error, ASNKIT_ERROR_CONSTRAINTS, der.encode("Small", eight, checked));

        bytes encoded = der.encode("Small", eight);

        THROWS(constraints_error, ASNKIT_ERROR_CONSTRAINTS, der.decode("Small", encoded, checked));
        der.check_constraints("Small", value(7));
    }

    /* A value that is not of its type. */
    THROWS(encode_error, ASNKIT_ERROR_ENCODE, der.encode("Big", value("one")));

    /* A type of two modules, and one of none. */
    {
        asnkit::options in_other;

        in_other.module = "Other";
        THROWS(no_such_type_error, ASNKIT_ERROR_NO_SUCH_TYPE, der.check_constraints("Twice", value(true)));
        THROWS(no_such_type_error, ASNKIT_ERROR_NO_SUCH_TYPE, der.check_constraints("Nope", value(true)));
        CHECK(round_trip(der, "Twice", value(true), in_other).is_boolean());
        THROWS(argument_error, ASNKIT_ERROR_ARGUMENT, der.decode_with_length("Twice", {}, in_other));
    }

    /* A decoding error, with its offset. */
    {
        const std::uint8_t truncated[] = {0x30, 0x05, 0x02, 0x01};

        try {
            der.decode("Record", truncated);
            fail(__LINE__, "nothing thrown");
        } catch (const asnkit::decode_error &error) {
            CHECK(error.code() == ASNKIT_ERROR_DECODE);
            // Where the contents of the SEQUENCE start, which the length
            // promises more of.
            CHECK(error.offset() == std::size_t(2));
            CHECK(error.what()[0] != '\0');
        }

        try {
            der.decode("Record", truncated);
        } catch (const asnkit::error &error) {
            CHECK(dynamic_cast<const asnkit::decode_error *>(&error) != nullptr);
        }
    }
}

void test_numeric_enums(const std::string &types)
{
    asnkit::load_options numeric;

    numeric.numeric_enums = true;

    asnkit::spec spec = asnkit::spec::load_file(types, "uper", numeric);

    CHECK(round_trip(spec, "Color", value(1)).as_integer() == 1);
}

/* What is after the members of a SEQUENCE is left out when lenient, and
 * is an error when not. */
void test_lenient(const std::string &types)
{
    static const std::uint8_t followed[] = {0x30, 0x06, 0x80, 0x01, 0x02, 0x82, 0x01, 0x00};
    asnkit::load_options lenient;

    lenient.lenient = true;

    asnkit::spec strict = asnkit::spec::load_file(types, "der");
    asnkit::spec loose = asnkit::spec::load_file(types, "der", lenient);

    THROWS(decode_error, ASNKIT_ERROR_DECODE, strict.decode("Record", followed));
    CHECK(loose.decode("Record", followed)["id"].as_integer() == 2);
}

void test_corrupt_files()
{
    ask(asnkit::spec::load_file(corpus + "/plain.asni", "uper"));

    try {
        asnkit::spec::load_file(corpus + "/corrupt.asni", "uper");
        fail(__LINE__, "nothing thrown");
    } catch (const asnkit::load_error &error) {
        CHECK(std::strstr(error.what(), "CRC-32") != nullptr);
    }
}

} // namespace

int main(int argc, const char *argv[])
{
    if (argc != 3) {
        std::fprintf(stderr, "usage: %s TYPES.ASNI CORPUS\n", argv[0]);
        return 2;
    }

    corpus = argv[2];

    try {
        test_errors(argv[1]);
        test_ownership(argv[1]);
        test_construction();
        test_values(argv[1]);
        test_numeric_enums(argv[1]);
        test_lenient(argv[1]);
        test_corrupt_files();
    } catch (const std::exception &error) {
        std::fprintf(stderr, "%s: uncaught: %s\n", __FILE__, error.what());
        return 1;
    }

    std::printf("ok\n");

    return 0;
}
