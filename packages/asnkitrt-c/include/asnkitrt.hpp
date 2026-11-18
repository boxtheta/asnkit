/*
 * Copyright (c) 2025 BoxTheta
 *
 * SPDX-License-Identifier: MIT
 */

/*
 * asnkitrt.hpp: the runtime of asnkit for C++.
 *
 * It is asnkitrt.h for C++17 and later, and nothing more: handles that
 * free themselves, errors that are thrown, and text and bytes that are
 * views. It is only a header, links the same libasnkitrt_c, and builds
 * on the functions of the C header, which it includes.
 *
 *     auto foo = asnkit::spec::load_file("foo.asni", "ber");
 *     auto question = asnkit::value::map({{"id", 1}, {"question", "Is 1+1=3?"}});
 *     std::vector<std::uint8_t> data = foo.encode("Question", question);
 *     asnkit::value decoded = foo.decode("Question", data);
 *
 *     std::cout << decoded["question"].as_text() << "\n";
 *
 * Conventions:
 *
 * - Everything is in the namespace asnkit; what is in asnkit::detail is
 *   not for use.
 * - A function that fails throws an asnkit::error, of the class of its
 *   status (asnkit::decode_error for ASNKIT_ERROR_DECODE, and so on), with
 *   the message the runtime gave. Allocations throw std::bad_alloc.
 * - asnkit::spec and asnkit::value own what they hold, and free it when
 *   they are destroyed. A value is copied deeply; a specification is only
 *   moved. What is moved from is empty: it may be destroyed, assigned to
 *   and, for a value, copied, which gives an empty value too. Anything
 *   else throws asnkit::argument_error.
 * - What is borrowed from a temporary value is gone at the end of the
 *   statement: keep a decoded value in a variable while its text, its
 *   bytes or its members are used.
 * - What is given as a std::string_view, a bytes_view, a bits_view or a
 *   value_view is borrowed from the value it is in, and is valid as long
 *   as that is and is not changed, as in C.
 * - Names (of types, modules, codecs, members and alternatives) are
 *   taken as std::string_view, and may not have a NUL in them. Text and
 *   bytes may.
 * - A specification may be used from several threads at once. A value
 *   may be read from several threads, and changed from one only.
 *
 * tests/header.rs holds this file to the functions of src/, and
 * tests/c.rs compiles and runs examples/question.cpp and tests/api.cpp
 * with it.
 */

#ifndef ASNKITRT_HPP
#define ASNKITRT_HPP

#include <algorithm>
#include <cstddef>
#include <cstdint>
#include <filesystem>
#include <initializer_list>
#include <iterator>
#include <limits>
#include <memory>
#include <optional>
#include <stdexcept>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <vector>

#include "asnkitrt.h"

namespace asnkit {

/* Errors ---------------------------------------------------------------- */

/* What every function throws when it fails: the message of the runtime,
 * and its status, as ASNKIT_ERROR_DECODE. A status that this header does
 * not know (of a newer runtime) is thrown as an error, and the others as
 * the class of their own. */
class error : public std::runtime_error {
public:
    error(asnkit_status code, const std::string &message)
        : std::runtime_error(message), code_(code)
    {
    }

    asnkit_status code() const noexcept { return code_; }

private:
    asnkit_status code_;
};

/* The .asni file could not be read, or was not what the load options
 * expect. */
class load_error : public error {
public:
    explicit load_error(const std::string &message) : error(ASNKIT_ERROR_LOAD, message) {}
};

/* A value that can not be encoded as its type. */
class encode_error : public error {
public:
    explicit encode_error(const std::string &message) : error(ASNKIT_ERROR_ENCODE, message) {}
};

/* Data that can not be decoded, and where that was found. */
class decode_error : public error {
public:
    explicit decode_error(const std::string &message,
                          std::optional<std::size_t> offset = std::nullopt)
        : error(ASNKIT_ERROR_DECODE, message), offset_(offset)
    {
    }

    /* Where the error was found, in bytes, or in bits for PER and
     * UPER. */
    std::optional<std::size_t> offset() const noexcept { return offset_; }

private:
    std::optional<std::size_t> offset_;
};

/* A value that does not fulfil the constraints of its type. */
class constraints_error : public error {
public:
    explicit constraints_error(const std::string &message)
        : error(ASNKIT_ERROR_CONSTRAINTS, message)
    {
    }
};

/* Something the runtime does not do (yet), or a codec that does not do
 * it, as an indent for BER. */
class unsupported_error : public error {
public:
    explicit unsupported_error(const std::string &message)
        : error(ASNKIT_ERROR_UNSUPPORTED, message)
    {
    }
};

/* A type that is not in the specification, or that more than one module
 * has and that is asked for with no module. */
class no_such_type_error : public error {
public:
    explicit no_such_type_error(const std::string &message)
        : error(ASNKIT_ERROR_NO_SUCH_TYPE, message)
    {
    }
};

/* An empty value or specification, a name with a NUL in it, text that is
 * not UTF-8, an unknown codec and so on. */
class argument_error : public error {
public:
    explicit argument_error(const std::string &message) : error(ASNKIT_ERROR_ARGUMENT, message)
    {
    }
};

/* A value that is not of the kind asked for. */
class kind_error : public error {
public:
    explicit kind_error(const std::string &message) : error(ASNKIT_ERROR_KIND, message) {}
};

/* A number that does not fit, an index that is out of range, or a member
 * that is not there. It is not named range_error, so as not to be
 * std::range_error where both namespaces are used. */
class asn_range_error : public error {
public:
    explicit asn_range_error(const std::string &message) : error(ASNKIT_ERROR_RANGE, message) {}
};

/* A file that could not be read. */
class io_error : public error {
public:
    explicit io_error(const std::string &message) : error(ASNKIT_ERROR_IO, message) {}
};

/* A bug of the runtime, caught before it got to C++. */
class panic_error : public error {
public:
    explicit panic_error(const std::string &message) : error(ASNKIT_ERROR_PANIC, message) {}
};

namespace detail {

/* Throws the last error of this thread, which `status` is, with its
 * message and its offset; they are read before anything else can call the
 * runtime. */
[[noreturn]] inline void throw_error(asnkit_status status)
{
    std::size_t at = 0;
    std::optional<std::size_t> offset;

    if (asnkit_last_error_offset(&at)) {
        offset = at;
    }

    std::string message = asnkit_last_error();

    switch (status) {
    case ASNKIT_ERROR_LOAD:
        throw load_error(message);
    case ASNKIT_ERROR_ENCODE:
        throw encode_error(message);
    case ASNKIT_ERROR_DECODE:
        throw decode_error(message, offset);
    case ASNKIT_ERROR_CONSTRAINTS:
        throw constraints_error(message);
    case ASNKIT_ERROR_UNSUPPORTED:
        throw unsupported_error(message);
    case ASNKIT_ERROR_NO_SUCH_TYPE:
        throw no_such_type_error(message);
    case ASNKIT_ERROR_ARGUMENT:
        throw argument_error(message);
    case ASNKIT_ERROR_KIND:
        throw kind_error(message);
    case ASNKIT_ERROR_RANGE:
        throw asn_range_error(message);
    case ASNKIT_ERROR_IO:
        throw io_error(message);
    case ASNKIT_ERROR_PANIC:
        throw panic_error(message);
    default:
        throw error(status, message);
    }
}

/* Throws if `status` is an error. */
inline void check(asnkit_status status)
{
    if (status != ASNKIT_OK) {
        throw_error(status);
    }
}

/* `handle`, or what made it NULL thrown. */
template <class T>
T *check_handle(T *handle)
{
    if (handle == nullptr) {
        asnkit_status status = asnkit_last_error_status();

        if (status == ASNKIT_OK) {
            throw panic_error("The runtime gave no handle and no error, which is a bug.");
        }

        throw_error(status);
    }

    return handle;
}

[[noreturn]] inline void throw_empty_value()
{
    throw argument_error("The value is empty: it was moved from, or is a view of no value.");
}

/* `name` as a string of C, for a function that takes one; `what` it is
 * names it in the error. */
inline std::string c_name(std::string_view name, const char *what)
{
    if (name.find('\0') != std::string_view::npos) {
        throw argument_error(std::string("Expected ") + what + " with no NUL in it.");
    }

    return std::string(name);
}

} // namespace detail

/* The version of the runtime, as "1.2.3". */
inline std::string_view version() noexcept
{
    return asnkit_version();
}

/* Bytes ------------------------------------------------------------------ */

namespace detail {

template <class Container>
using data_of = decltype(std::data(std::declval<const Container &>()));

/* If `Container` is contiguous and of bytes: it has data() and size(),
 * and data() points to std::uint8_t or unsigned char (and not char, so
 * that a std::string is never taken for bytes). */
template <class Container, class = void>
struct is_bytes : std::false_type {
};

template <class Container>
struct is_bytes<Container,
                std::void_t<data_of<Container>,
                            decltype(std::size(std::declval<const Container &>()))>>
    : std::bool_constant<
          std::is_pointer_v<data_of<Container>>
          && (std::is_same_v<std::remove_cv_t<std::remove_pointer_t<data_of<Container>>>,
                             std::uint8_t>
              || std::is_same_v<std::remove_cv_t<std::remove_pointer_t<data_of<Container>>>,
                                unsigned char>)> {
};

} // namespace detail

/* Bytes that are borrowed: a pointer and a length, which may be NULL when
 * the length is 0. It is made from a pointer and a length, or from a
 * contiguous container of std::uint8_t or unsigned char (a std::vector, a
 * std::array, an array, a std::span), which has to outlive it. */
class bytes_view {
public:
    using value_type = std::uint8_t;
    using size_type = std::size_t;
    using const_iterator = const std::uint8_t *;
    using iterator = const_iterator;

    constexpr bytes_view() noexcept = default;

    constexpr bytes_view(const std::uint8_t *data, std::size_t size) noexcept
        : data_(data), size_(size)
    {
    }

    /* Not from two integers: {0, 0} would be NULL and no bytes, and not
     * two bytes that are 0. */
    template <class Integer, std::enable_if_t<std::is_integral_v<Integer>, int> = 0>
    bytes_view(Integer, std::size_t) = delete;

    template <class Container,
              std::enable_if_t<detail::is_bytes<Container>::value
                                   && !std::is_same_v<Container, bytes_view>,
                               int> = 0>
    bytes_view(const Container &container) noexcept
        : data_(reinterpret_cast<const std::uint8_t *>(std::data(container))),
          size_(std::size(container))
    {
    }

    constexpr const std::uint8_t *data() const noexcept { return data_; }
    constexpr std::size_t size() const noexcept { return size_; }
    constexpr bool empty() const noexcept { return size_ == 0; }
    constexpr const std::uint8_t *begin() const noexcept { return data_; }
    constexpr const std::uint8_t *end() const noexcept { return data_ + size_; }
    constexpr const std::uint8_t &operator[](std::size_t index) const noexcept
    {
        return data_[index];
    }

private:
    const std::uint8_t *data_ = nullptr;
    std::size_t size_ = 0;
};

/* The same bytes, wherever they are. */
inline bool operator==(bytes_view a, bytes_view b) noexcept
{
    return std::equal(a.begin(), a.end(), b.begin(), b.end());
}

inline bool operator!=(bytes_view a, bytes_view b) noexcept
{
    return !(a == b);
}

/* A BIT STRING, borrowed: its bytes, and how many bits of them there
 * are. */
struct bits_view {
    bytes_view bytes;
    std::size_t count = 0;
};

/* Values ----------------------------------------------------------------- */

/* The kind of a value, as asnkitrt.h has them: what ASN.1 types are of
 * each is there. */
enum class kind : int {
    null = ASNKIT_NULL,
    boolean = ASNKIT_BOOLEAN,
    integer = ASNKIT_INTEGER,
    real = ASNKIT_REAL,
    text = ASNKIT_TEXT,
    bytes = ASNKIT_BYTES,
    bits = ASNKIT_BITS,
    list = ASNKIT_LIST,
    map = ASNKIT_MAP,
    choice = ASNKIT_CHOICE,
    date_time = ASNKIT_DATE_TIME,
    date = ASNKIT_DATE,
    time = ASNKIT_TIME,
    object_identifier = ASNKIT_OBJECT_IDENTIFIER
};

/* A DATE. */
struct date {
    std::int32_t year = 0;
    /* From 1. */
    std::uint8_t month = 1;
    std::uint8_t day = 1;
};

/* A TIME-OF-DAY. It is not named time, so as not to be ::time of <ctime>
 * where the namespace is used. */
struct time_of_day {
    std::uint8_t hour = 0;
    std::uint8_t minute = 0;
    std::uint8_t second = 0;
    std::uint32_t microsecond = 0;
    /* In minutes east of UTC, or none for a local time. */
    std::optional<std::int32_t> offset = std::nullopt;
};

/* A UTCTime, GeneralizedTime or DATE-TIME. */
struct date_time {
    asnkit::date date;
    time_of_day time;
};

inline bool operator==(const date &a, const date &b) noexcept
{
    return a.year == b.year && a.month == b.month && a.day == b.day;
}

inline bool operator!=(const date &a, const date &b) noexcept
{
    return !(a == b);
}

inline bool operator==(const time_of_day &a, const time_of_day &b) noexcept
{
    return a.hour == b.hour && a.minute == b.minute && a.second == b.second
           && a.microsecond == b.microsecond && a.offset == b.offset;
}

inline bool operator!=(const time_of_day &a, const time_of_day &b) noexcept
{
    return !(a == b);
}

inline bool operator==(const date_time &a, const date_time &b) noexcept
{
    return a.date == b.date && a.time == b.time;
}

inline bool operator!=(const date_time &a, const date_time &b) noexcept
{
    return !(a == b);
}

class value;
class value_view;

namespace detail {

class item_range;
class member_range;
class list_item;
class map_member;

/* What value and value_view have to read a value, written once:
 * `Derived` has get(), the handle, which may be NULL for an empty value.
 * The members are defined after both. */
template <class Derived>
class reader {
public:
    /* What kind of value it is. */
    asnkit::kind kind() const;

    bool is_null() const { return kind() == asnkit::kind::null; }
    bool is_boolean() const { return kind() == asnkit::kind::boolean; }
    bool is_integer() const { return kind() == asnkit::kind::integer; }
    bool is_real() const { return kind() == asnkit::kind::real; }
    bool is_text() const { return kind() == asnkit::kind::text; }
    bool is_bytes() const { return kind() == asnkit::kind::bytes; }
    bool is_bits() const { return kind() == asnkit::kind::bits; }
    bool is_list() const { return kind() == asnkit::kind::list; }
    bool is_map() const { return kind() == asnkit::kind::map; }
    bool is_choice() const { return kind() == asnkit::kind::choice; }
    bool is_date_time() const { return kind() == asnkit::kind::date_time; }
    bool is_date() const { return kind() == asnkit::kind::date; }
    bool is_time() const { return kind() == asnkit::kind::time; }
    bool is_object_identifier() const { return kind() == asnkit::kind::object_identifier; }

    /* The accessors throw kind_error for a value of another kind. */

    bool as_boolean() const;
    /* asn_range_error if the integer does not fit in 64 bits; then
     * as_integer_bytes() gives it. */
    std::int64_t as_integer() const;
    /* An integer that is not negative, as value(std::uint64_t) makes
     * above INT64_MAX; asn_range_error if it is negative, or does not fit
     * in 64 bits. */
    std::uint64_t as_unsigned() const;
    /* The two's complement of an integer of any size, most significant
     * byte first and as short as it can be. */
    std::vector<std::uint8_t> as_integer_bytes() const;
    double as_real() const;
    /* Borrowed. It may have a NUL in it. */
    std::string_view as_text() const;
    /* The arcs of an OBJECT IDENTIFIER, as "1.2.3". */
    std::string as_object_identifier() const;
    /* Borrowed: the contents octets of an OBJECT IDENTIFIER, as BER
     * encodes them. */
    bytes_view as_object_identifier_bytes() const;
    /* Borrowed. */
    bytes_view as_bytes() const;
    /* Borrowed: the bytes, and how many bits of them there are. */
    bits_view as_bits() const;
    /* The number of items of a list, or of members of a map. */
    std::size_t size() const;
    /* The item `index` of a list, borrowed; asn_range_error if there is
     * none. */
    value_view at(std::size_t index) const;
    /* The member `name` of a map, borrowed; asn_range_error if it is not
     * there. */
    value_view at(std::string_view name) const;
    value_view operator[](std::size_t index) const;
    value_view operator[](std::string_view name) const;
    /* The member `name` of a map, borrowed, or none if it is not there. */
    std::optional<value_view> find(std::string_view name) const;
    bool contains(std::string_view name) const;
    /* The name and the value of the member `index` of a map, borrowed, in
     * the order they are in; asn_range_error if there is none. */
    std::pair<std::string_view, value_view> member(std::size_t index) const;
    /* The name of the alternative of a choice, and its value, borrowed. */
    std::pair<std::string_view, value_view> as_choice() const;
    asnkit::date as_date() const;
    time_of_day as_time() const;
    asnkit::date_time as_date_time() const;
    /* The items of a list, borrowed, for a range-for. */
    item_range items() const;
    /* The members of a map, borrowed, in order, for a range-for: each a
     * std::pair of the name and the value. */
    member_range members() const;
    /* A copy, owned by the caller. */
    value clone() const;

protected:
    reader() = default;
    reader(const reader &) = default;
    reader &operator=(const reader &) = default;
    ~reader() = default;

private:
    /* The handle, which is not NULL. */
    const asnkit_value *checked() const;
};

} // namespace detail

/* A value that is borrowed: what the accessors give, and what an owned
 * value is seen as by what only reads it. It is valid as long as the value
 * it is in is, and is not changed. It is copied as a pointer. */
class value_view : public detail::reader<value_view> {
public:
    /* An empty view, of no value. */
    value_view() noexcept = default;

    /* A view of `handle`, which the caller keeps. */
    explicit value_view(const asnkit_value *handle) noexcept : handle_(handle) {}

    const asnkit_value *get() const noexcept { return handle_; }

private:
    const asnkit_value *handle_ = nullptr;
};

namespace detail {

/* An iterator over the items of a list. */
class item_iterator {
public:
    using iterator_category = std::input_iterator_tag;
    using value_type = value_view;
    using difference_type = std::ptrdiff_t;
    using pointer = void;
    using reference = value_view;

    item_iterator() noexcept = default;

    item_iterator(const asnkit_value *list, std::size_t index) noexcept
        : list_(list), index_(index)
    {
    }

    value_view operator*() const { return value_view(asnkit_list_get(list_, index_)); }

    item_iterator &operator++() noexcept
    {
        ++index_;
        return *this;
    }

    item_iterator operator++(int) noexcept
    {
        item_iterator before = *this;

        ++index_;
        return before;
    }

    friend bool operator==(const item_iterator &a, const item_iterator &b) noexcept
    {
        return a.list_ == b.list_ && a.index_ == b.index_;
    }

    friend bool operator!=(const item_iterator &a, const item_iterator &b) noexcept
    {
        return !(a == b);
    }

private:
    const asnkit_value *list_ = nullptr;
    std::size_t index_ = 0;
};

/* The items of a list, as reader::items() gives them. */
class item_range {
public:
    item_range(const asnkit_value *list, std::size_t size) noexcept : list_(list), size_(size) {}

    item_iterator begin() const noexcept { return item_iterator(list_, 0); }
    item_iterator end() const noexcept { return item_iterator(list_, size_); }
    std::size_t size() const noexcept { return size_; }
    bool empty() const noexcept { return size_ == 0; }

private:
    const asnkit_value *list_;
    std::size_t size_;
};

/* An iterator over the members of a map. */
class member_iterator {
public:
    using iterator_category = std::input_iterator_tag;
    using value_type = std::pair<std::string_view, value_view>;
    using difference_type = std::ptrdiff_t;
    using pointer = void;
    using reference = value_type;

    member_iterator() noexcept = default;

    member_iterator(const asnkit_value *map, std::size_t index) noexcept
        : map_(map), index_(index)
    {
    }

    value_type operator*() const
    {
        const char *name = nullptr;
        std::size_t length = 0;
        const asnkit_value *member = nullptr;

        check(asnkit_map_member(map_, index_, &name, &length, &member));
        return value_type(std::string_view(name, length), value_view(member));
    }

    member_iterator &operator++() noexcept
    {
        ++index_;
        return *this;
    }

    member_iterator operator++(int) noexcept
    {
        member_iterator before = *this;

        ++index_;
        return before;
    }

    friend bool operator==(const member_iterator &a, const member_iterator &b) noexcept
    {
        return a.map_ == b.map_ && a.index_ == b.index_;
    }

    friend bool operator!=(const member_iterator &a, const member_iterator &b) noexcept
    {
        return !(a == b);
    }

private:
    const asnkit_value *map_ = nullptr;
    std::size_t index_ = 0;
};

/* The members of a map, as reader::members() gives them. */
class member_range {
public:
    member_range(const asnkit_value *map, std::size_t size) noexcept : map_(map), size_(size) {}

    member_iterator begin() const noexcept { return member_iterator(map_, 0); }
    member_iterator end() const noexcept { return member_iterator(map_, size_); }
    std::size_t size() const noexcept { return size_; }
    bool empty() const noexcept { return size_ == 0; }

private:
    const asnkit_value *map_;
    std::size_t size_;
};

/* The types that are taken as characters, and not as integers, by the
 * constructors of value. */
template <class T>
inline constexpr bool is_character_v = std::is_same_v<T, char> || std::is_same_v<T, wchar_t>
#ifdef __cpp_char8_t
                                || std::is_same_v<T, char8_t>
#endif
                                || std::is_same_v<T, char16_t> || std::is_same_v<T, char32_t>;

/* The types that value takes as an integer: integral, of 64 bits or
 * fewer, and neither bool nor a character. */
template <class T>
inline constexpr bool is_integer_v = std::is_integral_v<T> && !std::is_same_v<T, bool>
                              && !is_character_v<T> && sizeof(T) <= sizeof(std::uint64_t);

inline asnkit_date to_c(const asnkit::date &date) noexcept
{
    asnkit_date c{};

    c.year = date.year;
    c.month = date.month;
    c.day = date.day;
    return c;
}

inline asnkit_time to_c(const time_of_day &time) noexcept
{
    asnkit_time c{};

    c.hour = time.hour;
    c.minute = time.minute;
    c.second = time.second;
    c.microsecond = time.microsecond;
    c.has_offset = time.offset.has_value() ? 1 : 0;
    c.offset = time.offset.value_or(0);
    return c;
}

inline asnkit::date from_c(const asnkit_date &c) noexcept
{
    asnkit::date date;

    date.year = c.year;
    date.month = c.month;
    date.day = c.day;
    return date;
}

inline time_of_day from_c(const asnkit_time &c) noexcept
{
    time_of_day time;

    time.hour = c.hour;
    time.minute = c.minute;
    time.second = c.second;
    time.microsecond = c.microsecond;

    if (c.has_offset) {
        time.offset = c.offset;
    }

    return time;
}

} // namespace detail

/* A value that is owned: made, decoded or copied. It is freed when it is
 * destroyed, with all it holds. A copy is deep, and independent of what
 * it is copied from; a move leaves what it is moved from empty.
 *
 * It is made from what C++ has, which picks its kind:
 *
 *     asnkit::value nothing;                 // NULL
 *     asnkit::value yes = true;              // a boolean
 *     asnkit::value one = 1;                 // an integer, of any integral type
 *     asnkit::value half = 0.5;              // a real
 *     asnkit::value hello = "hello";         // text, never a boolean
 *
 * or with the functions named for the kinds, as value::list() and
 * value::map(). */
class value : public detail::reader<value> {
public:
    /* NULL. */
    value() : handle_(detail::check_handle(asnkit_value_new_null())) {}

    /* A boolean, from a bool and nothing that converts to one. */
    template <class T, std::enable_if_t<std::is_same_v<T, bool>, int> = 0>
    value(T boolean) : handle_(detail::check_handle(asnkit_value_new_boolean(boolean ? 1 : 0)))
    {
    }

    /* An integer, from any integral type but bool and the characters; one
     * above INT64_MAX is right too. */
    template <class T, std::enable_if_t<detail::is_integer_v<T>, int> = 0>
    value(T integer) : handle_(new_integer(integer))
    {
    }

    /* A real. */
    template <class T, std::enable_if_t<std::is_floating_point_v<T>, int> = 0>
    value(T real) : handle_(detail::check_handle(asnkit_value_new_real(static_cast<double>(real))))
    {
    }

    /* Text, of UTF-8; argument_error for NULL. */
    value(const char *text);
    value(std::string_view text);
    value(const std::string &text) : value(std::string_view(text)) {}

    /* A deep copy; of an empty value, an empty value. */
    value(const value &other)
        : detail::reader<value>(),
          handle_(other.handle_ == nullptr
                      ? nullptr
                      : detail::check_handle(asnkit_value_clone(other.handle_)))
    {
    }

    value(value &&other) noexcept : handle_(std::exchange(other.handle_, nullptr)) {}

    value &operator=(const value &other)
    {
        if (this != &other) {
            *this = value(other);
        }

        return *this;
    }

    value &operator=(value &&other) noexcept
    {
        if (this != &other) {
            asnkit_value_free(handle_);
            handle_ = std::exchange(other.handle_, nullptr);
        }

        return *this;
    }

    ~value() { asnkit_value_free(handle_); }

    /* The functions named for the kinds take what the constructors take,
     * and nothing that converts to it: value::integer(0.5) and
     * value::boolean("text") do not compile. */
    static value null() { return value(); }
    template <class T, std::enable_if_t<std::is_same_v<T, bool>, int> = 0>
    static value boolean(T boolean)
    {
        return value(boolean);
    }
    template <class T, std::enable_if_t<detail::is_integer_v<T>, int> = 0>
    static value integer(T integer)
    {
        return value(integer);
    }
    /* An integer of any size, from its two's complement, most significant
     * byte first. No bytes are 0. */
    static value integer_bytes(bytes_view bytes)
    {
        return adopt(
            detail::check_handle(asnkit_value_new_integer_bytes(bytes.data(), bytes.size())));
    }
    /* A real, of an integer too: value::real(1). */
    template <class T,
              std::enable_if_t<std::is_floating_point_v<T> || detail::is_integer_v<T>, int> = 0>
    static value real(T real)
    {
        return adopt(detail::check_handle(asnkit_value_new_real(static_cast<double>(real))));
    }
    /* Text, of UTF-8, which is copied; it may have a NUL in it. */
    static value text(std::string_view text) { return value(text); }
    static value text(const char *text) { return value(text); }
    /* The arcs of an OBJECT IDENTIFIER, as "1.2.3". */
    static value object_identifier(std::string_view text)
    {
        return adopt(
            detail::check_handle(asnkit_value_new_object_identifier(text.data(), text.size())));
    }
    static value bytes(bytes_view bytes)
    {
        return adopt(detail::check_handle(asnkit_value_new_bytes(bytes.data(), bytes.size())));
    }
    /* The first `count` bits of `bytes`, which have to be as many as the
     * bits take; asn_range_error if they are not. */
    static value bits(bytes_view bytes, std::size_t count)
    {
        return adopt(
            detail::check_handle(asnkit_value_new_bits(bytes.data(), bytes.size(), count)));
    }
    static value bits(bits_view bits) { return value::bits(bits.bytes, bits.count); }
    /* An empty list, for push(). */
    static value list() { return adopt(detail::check_handle(asnkit_value_new_list())); }
    /* A list of `items`, which are moved into it and not copied:
     * value::list({true, false}). */
    static value list(std::initializer_list<detail::list_item> items);
    /* An empty map, for set(). */
    static value map() { return adopt(detail::check_handle(asnkit_value_new_map())); }
    /* A map of `members`, in order, which are moved into it and not
     * copied: value::map({{"id", 1}, {"question", "Is 1+1=3?"}}). A name
     * that is given twice is set twice. */
    static value map(std::initializer_list<detail::map_member> members);
    /* The alternative `name` with `inner`. */
    static value choice(std::string_view name, value inner);
    static value date(asnkit::date date)
    {
        return adopt(detail::check_handle(asnkit_value_new_date(detail::to_c(date))));
    }
    static value time(time_of_day time)
    {
        return adopt(detail::check_handle(asnkit_value_new_time(detail::to_c(time))));
    }
    static value date_time(asnkit::date_time date_time)
    {
        return adopt(detail::check_handle(asnkit_value_new_date_time(
            detail::to_c(date_time.date), detail::to_c(date_time.time))));
    }

    /* Adds `item` to the end of a list; kind_error if this is not one. It
     * returns the list, to chain: value::list().push(1).push(2). A list
     * that is temporary is returned as a value, which is moved and not
     * copied. */
    value &push(value item) &;
    value push(value item) &&;
    /* Sets the member `name` of a map to `member`, in place of the member
     * of that name if there is one; kind_error if this is not a map. It
     * returns the map, to chain: value::map().set("id", 1). */
    value &set(std::string_view name, value member) &;
    value set(std::string_view name, value member) &&;

    /* As of a view, but not of a value that is temporary, which would be
     * freed before the first item is read:
     * for (auto item : decode().items()). */
    detail::item_range items() const & { return detail::reader<value>::items(); }
    detail::item_range items() const && = delete;
    detail::member_range members() const & { return detail::reader<value>::members(); }
    detail::member_range members() const && = delete;

    /* The handle, still owned by the value, for C. */
    const asnkit_value *get() const noexcept { return handle_; }

    /* A value that owns `handle`, which a function of C gave, and frees
     * it. NULL makes an empty value. */
    static value adopt(asnkit_value *handle) noexcept { return value(handle, adopted{}); }

    /* The handle, which the caller then owns, to free with
     * asnkit_value_free() or to give to C. The value is left empty. */
    asnkit_value *release() noexcept { return std::exchange(handle_, nullptr); }

    operator value_view() const noexcept { return value_view(handle_); }

private:
    struct adopted {
    };

    value(asnkit_value *handle, adopted) noexcept : handle_(handle) {}

    template <class T>
    static asnkit_value *new_integer(T integer)
    {
        if constexpr (std::is_signed_v<T>) {
            return detail::check_handle(asnkit_value_new_integer(static_cast<std::int64_t>(integer)));
        } else {
            // In parentheses, for it not to be the macro of <windows.h>.
            constexpr std::uint64_t max = (std::numeric_limits<std::int64_t>::max)();
            std::uint64_t unsigned_integer = static_cast<std::uint64_t>(integer);

            if (unsigned_integer <= max) {
                return detail::check_handle(
                    asnkit_value_new_integer(static_cast<std::int64_t>(unsigned_integer)));
            }

            // Above INT64_MAX: a 0 and then the 8 bytes, as two's
            // complement.
            std::uint8_t twos_complement[9] = {};

            for (int i = 8; i >= 1; --i) {
                twos_complement[i] = static_cast<std::uint8_t>(unsigned_integer & 0xff);
                unsigned_integer >>= 8;
            }

            return detail::check_handle(
                asnkit_value_new_integer_bytes(twos_complement, sizeof(twos_complement)));
        }
    }

    /* The handle, to change, which is not NULL. */
    asnkit_value *mutable_handle() const
    {
        if (handle_ == nullptr) {
            detail::throw_empty_value();
        }

        return handle_;
    }

    asnkit_value *handle_ = nullptr;
};

namespace detail {

/* An item of value::list({...}). The list moves the value out of it,
 * which an element of a std::initializer_list, being const, would not
 * let it do otherwise. */
class list_item {
public:
    template <class T, std::enable_if_t<std::is_constructible_v<value, T &&>, int> = 0>
    list_item(T &&item) : item_(std::forward<T>(item))
    {
    }

private:
    friend class asnkit::value;

    mutable value item_;
};

/* A member of value::map({...}): {"name", value}. The map moves the value
 * out of it, as of a list_item. */
class map_member {
public:
    map_member(std::string_view name, value member) : name_(name), member_(std::move(member)) {}

private:
    friend class asnkit::value;

    std::string_view name_;
    mutable value member_;
};

} // namespace detail

inline value::value(const char *text)
{
    if (text == nullptr) {
        throw argument_error("Expected text, and not NULL.");
    }

    std::string_view view(text);

    handle_ = detail::check_handle(asnkit_value_new_text(view.data(), view.size()));
}

inline value::value(std::string_view text)
    : handle_(detail::check_handle(asnkit_value_new_text(text.data(), text.size())))
{
}

inline value value::list(std::initializer_list<detail::list_item> items)
{
    value list = value::list();

    for (const detail::list_item &item : items) {
        list.push(std::move(item.item_));
    }

    return list;
}

inline value value::map(std::initializer_list<detail::map_member> members)
{
    value map = value::map();

    for (const detail::map_member &member : members) {
        map.set(member.name_, std::move(member.member_));
    }

    return map;
}

// The functions of C that take a value take it even when they fail: it is
// released before the call, and after all that may throw in C++.

inline value value::choice(std::string_view name, value inner)
{
    std::string c_name = detail::c_name(name, "the name of the alternative");

    if (inner.handle_ == nullptr) {
        detail::throw_empty_value();
    }

    return adopt(detail::check_handle(asnkit_value_new_choice(c_name.c_str(), inner.release())));
}

inline value &value::push(value item) &
{
    asnkit_value *list = mutable_handle();

    if (item.handle_ == nullptr) {
        detail::throw_empty_value();
    }

    detail::check(asnkit_list_push(list, item.release()));
    return *this;
}

inline value value::push(value item) &&
{
    push(std::move(item));
    return std::move(*this);
}

inline value &value::set(std::string_view name, value member) &
{
    std::string c_name = detail::c_name(name, "the name of the member");
    asnkit_value *map = mutable_handle();

    if (member.handle_ == nullptr) {
        detail::throw_empty_value();
    }

    detail::check(asnkit_map_set(map, c_name.c_str(), member.release()));
    return *this;
}

inline value value::set(std::string_view name, value member) &&
{
    set(name, std::move(member));
    return std::move(*this);
}

namespace detail {

template <class Derived>
const asnkit_value *reader<Derived>::checked() const
{
    const asnkit_value *handle = static_cast<const Derived &>(*this).get();

    if (handle == nullptr) {
        throw_empty_value();
    }

    return handle;
}

template <class Derived>
asnkit::kind reader<Derived>::kind() const
{
    return static_cast<asnkit::kind>(asnkit_value_kind(checked()));
}

template <class Derived>
bool reader<Derived>::as_boolean() const
{
    int boolean = 0;

    check(asnkit_value_boolean(checked(), &boolean));
    return boolean != 0;
}

template <class Derived>
std::int64_t reader<Derived>::as_integer() const
{
    std::int64_t integer = 0;

    check(asnkit_value_integer(checked(), &integer));
    return integer;
}

template <class Derived>
std::uint64_t reader<Derived>::as_unsigned() const
{
    // The two's complement, as short as it can be: of 8 bytes or fewer,
    // or of 9 of which the first is 0.
    std::vector<std::uint8_t> bytes = as_integer_bytes();
    std::uint64_t integer = 0;

    if (!bytes.empty() && (bytes[0] & 0x80) != 0) {
        throw asn_range_error("The integer is negative.");
    }

    if (bytes.size() > 9 || (bytes.size() == 9 && bytes[0] != 0)) {
        throw asn_range_error("The integer does not fit in 64 bits.");
    }

    for (std::uint8_t byte : bytes) {
        integer = (integer << 8) | byte;
    }

    return integer;
}

template <class Derived>
std::vector<std::uint8_t> reader<Derived>::as_integer_bytes() const
{
    const asnkit_value *handle = checked();
    std::size_t length = 0;
    // Its length first, which is ASNKIT_ERROR_RANGE with no buffer.
    asnkit_status status = asnkit_value_integer_bytes(handle, nullptr, 0, &length);

    if (status != ASNKIT_ERROR_RANGE) {
        check(status);
    }

    std::vector<std::uint8_t> bytes(length);

    if (length != 0) {
        check(asnkit_value_integer_bytes(handle, bytes.data(), bytes.size(), &length));
        bytes.resize(length);
    }

    return bytes;
}

template <class Derived>
double reader<Derived>::as_real() const
{
    double real = 0;

    check(asnkit_value_real(checked(), &real));
    return real;
}

template <class Derived>
std::string_view reader<Derived>::as_text() const
{
    const char *text = nullptr;
    std::size_t length = 0;

    check(asnkit_value_text(checked(), &text, &length));
    return std::string_view(text, length);
}

template <class Derived>
std::string reader<Derived>::as_object_identifier() const
{
    const asnkit_value *handle = checked();
    std::size_t length = 0;
    // Its length first, which is ASNKIT_ERROR_RANGE with no buffer.
    asnkit_status status = asnkit_value_object_identifier(handle, nullptr, 0, &length);

    if (status != ASNKIT_ERROR_RANGE) {
        check(status);
    }

    std::string text(length, '\0');

    if (length != 0) {
        check(asnkit_value_object_identifier(handle, text.data(), text.size(), &length));
        text.resize(length);
    }

    return text;
}

template <class Derived>
bytes_view reader<Derived>::as_object_identifier_bytes() const
{
    const std::uint8_t *data = nullptr;
    std::size_t length = 0;

    check(asnkit_value_object_identifier_bytes(checked(), &data, &length));
    return bytes_view(data, length);
}

template <class Derived>
bytes_view reader<Derived>::as_bytes() const
{
    const std::uint8_t *data = nullptr;
    std::size_t length = 0;

    check(asnkit_value_bytes(checked(), &data, &length));
    return bytes_view(data, length);
}

template <class Derived>
bits_view reader<Derived>::as_bits() const
{
    const std::uint8_t *data = nullptr;
    std::size_t length = 0;
    std::size_t count = 0;

    check(asnkit_value_bits(checked(), &data, &length, &count));

    bits_view bits;

    bits.bytes = bytes_view(data, length);
    bits.count = count;
    return bits;
}

template <class Derived>
std::size_t reader<Derived>::size() const
{
    std::size_t length = 0;

    check(asnkit_value_length(checked(), &length));
    return length;
}

// asnkit_list_get() and asnkit_map_get() return NULL for a value of another
// kind as for an item or a member that is not there, and leave no error:
// the kind is checked here.

template <class Derived>
value_view reader<Derived>::at(std::size_t index) const
{
    const asnkit_value *list = checked();

    if (asnkit_value_kind(list) != ASNKIT_LIST) {
        throw kind_error("The value is not a list.");
    }

    const asnkit_value *item = asnkit_list_get(list, index);

    if (item == nullptr) {
        throw asn_range_error("The list has no item at index " + std::to_string(index) + ".");
    }

    return value_view(item);
}

template <class Derived>
std::optional<value_view> reader<Derived>::find(std::string_view name) const
{
    const asnkit_value *map = checked();
    std::string c_name = detail::c_name(name, "the name of the member");

    if (asnkit_value_kind(map) != ASNKIT_MAP) {
        throw kind_error("The value is not a map.");
    }

    const asnkit_value *member = asnkit_map_get(map, c_name.c_str());

    if (member == nullptr) {
        return std::nullopt;
    }

    return value_view(member);
}

template <class Derived>
value_view reader<Derived>::at(std::string_view name) const
{
    std::optional<value_view> member = find(name);

    if (!member) {
        throw asn_range_error("The map has no member '" + std::string(name) + "'.");
    }

    return *member;
}

template <class Derived>
value_view reader<Derived>::operator[](std::size_t index) const
{
    return at(index);
}

template <class Derived>
value_view reader<Derived>::operator[](std::string_view name) const
{
    return at(name);
}

template <class Derived>
bool reader<Derived>::contains(std::string_view name) const
{
    return find(name).has_value();
}

template <class Derived>
std::pair<std::string_view, value_view> reader<Derived>::member(std::size_t index) const
{
    const char *name = nullptr;
    std::size_t length = 0;
    const asnkit_value *member = nullptr;

    check(asnkit_map_member(checked(), index, &name, &length, &member));
    return {std::string_view(name, length), value_view(member)};
}

template <class Derived>
std::pair<std::string_view, value_view> reader<Derived>::as_choice() const
{
    const char *name = nullptr;
    std::size_t length = 0;
    const asnkit_value *inner = nullptr;

    check(asnkit_choice_get(checked(), &name, &length, &inner));
    return {std::string_view(name, length), value_view(inner)};
}

template <class Derived>
asnkit::date reader<Derived>::as_date() const
{
    asnkit_date date{};

    check(asnkit_value_date(checked(), &date));
    return from_c(date);
}

template <class Derived>
time_of_day reader<Derived>::as_time() const
{
    asnkit_time time{};

    check(asnkit_value_time(checked(), &time));
    return from_c(time);
}

template <class Derived>
asnkit::date_time reader<Derived>::as_date_time() const
{
    asnkit_date date{};
    asnkit_time time{};

    check(asnkit_value_date_time(checked(), &date, &time));

    asnkit::date_time both;

    both.date = from_c(date);
    both.time = from_c(time);
    return both;
}

template <class Derived>
item_range reader<Derived>::items() const
{
    const asnkit_value *list = checked();

    if (asnkit_value_kind(list) != ASNKIT_LIST) {
        throw kind_error("The value is not a list.");
    }

    return item_range(list, size());
}

template <class Derived>
member_range reader<Derived>::members() const
{
    const asnkit_value *map = checked();

    if (asnkit_value_kind(map) != ASNKIT_MAP) {
        throw kind_error("The value is not a map.");
    }

    return member_range(map, size());
}

template <class Derived>
value reader<Derived>::clone() const
{
    return value::adopt(check_handle(asnkit_value_clone(checked())));
}

} // namespace detail

/* Encoding and decoding ------------------------------------------------- */

/* How a file is loaded. A file whose CRC-32 does not match is never
 * loaded. */
struct load_options {
    /* ENUMERATED values are integers, and not names. */
    bool numeric_enums = false;
    /* DER decodes what BER allows, and BER and DER leave out what is after
     * the members of a SEQUENCE or a SET, and after what is tagged. */
    bool lenient = false;
};

/* How a value is encoded or decoded. */
struct options {
    /* The module of the type, or none if only one module has a type of
     * that name. */
    std::optional<std::string> module = std::nullopt;
    /* To check the value against the constraints of its type, before
     * encoding it or after decoding it. */
    bool check_constraints = false;
    /* The number of spaces for each level of JER, XER and GSER, or none
     * (as for a negative number). Other codecs throw unsupported_error for
     * one. */
    std::optional<int> indent = std::nullopt;
};

namespace detail {

/* An options as asnkit_options, which borrows the module from it. */
class codec_options {
public:
    explicit codec_options(const options &how)
    {
        if (how.module) {
            module_ = c_name(*how.module, "the name of the module");
        }

        raw_.module = how.module ? module_.c_str() : nullptr;
        raw_.check_constraints = how.check_constraints ? 1 : 0;
        raw_.indent = how.indent ? *how.indent : -1;
    }

    codec_options(const codec_options &) = delete;
    codec_options &operator=(const codec_options &) = delete;

    const asnkit_options *get() const noexcept { return &raw_; }

private:
    std::string module_;
    asnkit_options raw_{};
};

/* A buffer that asnkit_encode() gave, freed with it on every path. */
struct encoded {
    encoded() = default;
    encoded(const encoded &) = delete;
    encoded &operator=(const encoded &) = delete;
    ~encoded() { asnkit_bytes_free(data, length); }

    std::uint8_t *data = nullptr;
    std::size_t length = 0;
};

/* A path as asnkit_spec_load_file() takes it: as it is where it is bytes
 * (POSIX), and as UTF-8 elsewhere; u8string() is a std::string in C++17
 * and a std::u8string after. It is a template for only the branch of the
 * system to be compiled. */
template <class Path>
std::string path_bytes(const Path &path)
{
    if constexpr (std::is_same_v<typename Path::value_type, char>) {
        return path.native();
    } else {
        auto utf8 = path.u8string();

        return std::string(utf8.begin(), utf8.end());
    }
}

struct load_options_free {
    void operator()(asnkit_load_options *options) const noexcept
    {
        asnkit_load_options_free(options);
    }
};

/* `value`'s handle, which is not NULL. */
inline const asnkit_value *handle_of(value_view value)
{
    if (value.get() == nullptr) {
        throw_empty_value();
    }

    return value.get();
}

} // namespace detail

/* A specification, loaded for one codec: what encodes and decodes. It is
 * moved and not copied, and freed when it is destroyed. */
class spec {
public:
    /* Loads the bytes of an .asni file for `codec`: "ber", "der", "oer",
     * "per", "uper", "jer", "xer" or "gser". */
    static spec load(bytes_view asni, std::string_view codec = "ber",
                     const load_options &how = {});

    /* The same as load(), with the .asni file at `path`. */
    static spec load_file(const std::filesystem::path &path, std::string_view codec = "ber",
                          const load_options &how = {});

    spec(spec &&other) noexcept : handle_(std::exchange(other.handle_, nullptr)) {}

    spec &operator=(spec &&other) noexcept
    {
        if (this != &other) {
            asnkit_spec_free(handle_);
            handle_ = std::exchange(other.handle_, nullptr);
        }

        return *this;
    }

    spec(const spec &) = delete;
    spec &operator=(const spec &) = delete;

    ~spec() { asnkit_spec_free(handle_); }

    /* The codec it is loaded for, as "ber". */
    std::string_view codec() const { return asnkit_spec_codec(checked()); }

    /* Encodes `value` as the type `type`. */
    std::vector<std::uint8_t> encode(std::string_view type, value_view value,
                                     const options &how = {}) const
    {
        detail::encoded out;

        encode_to(type, value, how, out);
        return std::vector<std::uint8_t>(out.data, out.data + out.length);
    }

    /* Encodes `value` as the type `type`, as text: for JER, XER and
     * GSER. */
    std::string encode_text(std::string_view type, value_view value,
                            const options &how = {}) const
    {
        detail::encoded out;

        encode_to(type, value, how, out);
        return std::string(reinterpret_cast<const char *>(out.data), out.length);
    }

    /* Decodes `data` as the type `type`. */
    value decode(std::string_view type, bytes_view data, const options &how = {}) const
    {
        std::string name = detail::c_name(type, "the name of the type");
        detail::codec_options c_options(how);
        asnkit_value *decoded = nullptr;

        detail::check(asnkit_decode(checked(), name.c_str(), data.data(), data.size(),
                                    c_options.get(), &decoded));
        return value::adopt(decoded);
    }

    /* Decodes `text` as the type `type`: for JER, XER and GSER. */
    value decode_text(std::string_view type, std::string_view text,
                      const options &how = {}) const
    {
        return decode(type, bytes_view(reinterpret_cast<const std::uint8_t *>(text.data()),
                                       text.size()),
                      how);
    }

    /* Decodes the first value of `data`, and gives how many bytes it was
     * encoded in. BER and DER only, and with no module. */
    std::pair<value, std::size_t> decode_with_length(std::string_view type, bytes_view data,
                                                     const options &how = {}) const
    {
        std::string name = detail::c_name(type, "the name of the type");
        detail::codec_options c_options(how);
        asnkit_value *decoded = nullptr;
        std::size_t consumed = 0;

        detail::check(asnkit_decode_with_length(checked(), name.c_str(), data.data(),
                                                data.size(), c_options.get(), &decoded,
                                                &consumed));
        return {value::adopt(decoded), consumed};
    }

    /* The length in bytes of the first value of `data`, or none if there
     * is not enough data yet to tell. BER and DER only. */
    std::optional<std::size_t> decode_length(bytes_view data) const
    {
        std::size_t length = 0;
        int known = 0;

        detail::check(asnkit_decode_length(checked(), data.data(), data.size(), &length, &known));

        if (!known) {
            return std::nullopt;
        }

        return length;
    }

    /* Checks `value` against the constraints of the type `type`, and
     * throws constraints_error if it does not fulfil them. Only `module`
     * of the options is used. */
    void check_constraints(std::string_view type, value_view value,
                           const options &how = {}) const
    {
        std::string name = detail::c_name(type, "the name of the type");
        const asnkit_value *handle = detail::handle_of(value);
        detail::codec_options c_options(how);

        detail::check(asnkit_check_constraints(checked(), name.c_str(), handle, c_options.get()));
    }

    /* The handle, still owned by the specification, for C. */
    const asnkit_spec *get() const noexcept { return handle_; }

private:
    explicit spec(asnkit_spec *handle) noexcept : handle_(handle) {}

    /* The handle, which is not NULL. */
    const asnkit_spec *checked() const
    {
        if (handle_ == nullptr) {
            throw argument_error("The specification is empty: it was moved from.");
        }

        return handle_;
    }

    void encode_to(std::string_view type, value_view value, const options &how,
                   detail::encoded &out) const
    {
        std::string name = detail::c_name(type, "the name of the type");
        const asnkit_value *handle = detail::handle_of(value);
        detail::codec_options c_options(how);

        detail::check(asnkit_encode(checked(), name.c_str(), handle, c_options.get(), &out.data,
                                    &out.length));
    }

    static spec load_c(const std::uint8_t *asni, std::size_t length, const std::string *path,
                       std::string_view codec, const load_options &how);

    asnkit_spec *handle_;
};

inline spec spec::load_c(const std::uint8_t *asni, std::size_t length, const std::string *path,
                         std::string_view codec, const load_options &how)
{
    std::string c_codec = detail::c_name(codec, "the codec");
    std::unique_ptr<asnkit_load_options, detail::load_options_free> c_options(
        detail::check_handle(asnkit_load_options_new()));
    asnkit_spec *loaded = nullptr;

    detail::check(asnkit_load_options_set_numeric_enums(c_options.get(), how.numeric_enums));
    detail::check(asnkit_load_options_set_lenient(c_options.get(), how.lenient));

    if (path != nullptr) {
        detail::check(
            asnkit_spec_load_file(path->c_str(), c_codec.c_str(), c_options.get(), &loaded));
    } else {
        detail::check(asnkit_spec_load(asni, length, c_codec.c_str(), c_options.get(), &loaded));
    }

    return spec(loaded);
}

inline spec spec::load(bytes_view asni, std::string_view codec, const load_options &how)
{
    return load_c(asni.data(), asni.size(), nullptr, codec, how);
}

inline spec spec::load_file(const std::filesystem::path &path, std::string_view codec,
                            const load_options &how)
{
    std::string c_path = detail::c_name(detail::path_bytes(path), "the path");

    return load_c(nullptr, 0, &c_path, codec, how);
}

} // namespace asnkit

#endif /* ASNKITRT_HPP */
