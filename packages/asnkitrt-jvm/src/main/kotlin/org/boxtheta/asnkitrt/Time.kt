// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

package org.boxtheta.asnkitrt

import java.time.LocalDate
import java.time.LocalDateTime
import java.time.LocalTime

// The strings of UTCTime, GeneralizedTime, DATE, TIME-OF-DAY and
// DATE-TIME, and the values they are, and those of DURATION.

/** A time string that is not one, to be made the error of what reads it. */
private class BadTime : RuntimeException(null, null, false, false)

private fun bad(): Nothing = throw BadTime()

private fun isLeap(year: Int): Boolean = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0)

private fun daysIn(
    year: Int,
    month: Int,
): Int =
    when (month) {
        1, 3, 5, 7, 8, 10, 12 -> 31
        4, 6, 9, 11 -> 30
        else -> if (isLeap(year)) 29 else 28
    }

private fun checkedDate(
    year: Int,
    month: Int,
    day: Int,
): LocalDate {
    if (year !in 1..9999 || month !in 1..12 || day < 1 || day > daysIn(year, month)) bad()

    return LocalDate.of(year, month, day)
}

private fun checkedTime(
    hour: Int,
    minute: Int,
    second: Int,
    microsecond: Int,
): LocalTime {
    if (hour > 23 || minute > 59 || second > 59) bad()

    return LocalTime.of(hour, minute, second, microsecond * 1000)
}

/** A cursor over the characters of a string. */
private class Cursor(
    val text: String,
) {
    var at = 0

    /** A number of exactly `digits` digits. */
    fun number(digits: Int): Int {
        if (at + digits > text.length) bad()

        var value = 0

        for (index in at until at + digits) {
            val c = text[index]

            if (c !in '0'..'9') bad()

            value = value * 10 + (c - '0')
        }

        at += digits

        return value
    }

    fun peek(): Char? = text.getOrNull(at)

    fun isDigit(): Boolean = peek()?.let { it in '0'..'9' } ?: false

    fun done(): Boolean = at == text.length

    /**
     * The digits of a fraction of a unit of `unit` microseconds, as
     * microseconds, rounded down. There may be any number of digits (X.680
     * 46.3), but those past the 20th are less than a microsecond, and are
     * not looked at.
     */
    fun fraction(unit: Long): Long {
        var digits = 0
        var value = java.math.BigInteger.ZERO
        var any = false

        while (isDigit()) {
            if (digits < 20) {
                value = value.multiply(java.math.BigInteger.TEN).add(java.math.BigInteger.valueOf((text[at] - '0').toLong()))
                digits += 1
            }

            any = true
            at += 1
        }

        if (!any) bad()

        return value.multiply(java.math.BigInteger.valueOf(unit)).divide(java.math.BigInteger.TEN.pow(digits)).toLong()
    }

    /** `+HHMM` or `-HHMM`, or with `hoursOnly` also `+HH` or `-HH`, as minutes east of UTC. */
    fun offset(hoursOnly: Boolean): Int {
        val sign =
            when (peek()) {
                '+' -> 1
                '-' -> -1
                else -> bad()
            }

        at += 1

        val hours = number(2)
        val minutes = if (hoursOnly && done()) 0 else number(2)

        if (minutes > 59 || hours > 23) bad()

        return sign * (hours * 60 + minutes)
    }
}

private fun StringBuilder.pad(
    value: Int,
    width: Int,
): StringBuilder {
    val digits = value.toString()

    repeat(width - digits.length) { append('0') }

    return append(digits)
}

private fun StringBuilder.offset(offset: Int): StringBuilder {
    append(if (offset < 0) '-' else '+')

    return pad(Math.abs(offset) / 60, 2).pad(Math.abs(offset) % 60, 2)
}

internal fun microsecond(time: LocalTime): Int = time.nano / 1000

/** The date and time in UTC, with no offset. */
internal fun toUtc(value: Value.DateTime): Value.DateTime {
    val offset = value.offsetMinutes ?: return value

    return Value.DateTime(value.dateTime.minusMinutes(offset.toLong()), null)
}

/** As `%y` is read: 69 to 99 are 1969 to 1999. */
private fun twoDigitYear(year: Int): Int = if (year >= 69) 1900 + year else 2000 + year

/** A UTCTime string. `restricted` is the form DER has. */
internal fun utcTimeToString(
    value: Value.DateTime,
    restricted: Boolean,
): String {
    val out = StringBuilder()

    if (restricted) {
        val utc = toUtc(value).dateTime

        return out
            .pad(utc.year % 100, 2)
            .pad(utc.monthValue, 2)
            .pad(utc.dayOfMonth, 2)
            .pad(utc.hour, 2)
            .pad(utc.minute, 2)
            .pad(utc.second, 2)
            .append('Z')
            .toString()
    }

    val time = value.dateTime

    out.pad(time.year % 100, 2).pad(time.monthValue, 2).pad(time.dayOfMonth, 2).pad(time.hour, 2).pad(time.minute, 2)

    if (time.second > 0) out.pad(time.second, 2)

    val offset = value.offsetMinutes

    if (offset == null) out.append('Z') else out.offset(offset)

    return out.toString()
}

internal fun utcTimeFromString(
    text: String,
    restricted: Boolean,
): Value.DateTime =
    try {
        val cursor = Cursor(text)
        val year = twoDigitYear(cursor.number(2))
        val month = cursor.number(2)
        val day = cursor.number(2)
        val hour = cursor.number(2)
        val minute = cursor.number(2)
        val second = if (restricted || cursor.isDigit()) cursor.number(2) else 0
        val offset =
            when {
                cursor.peek() == 'Z' -> {
                    cursor.at += 1
                    null
                }
                restricted -> bad()
                else -> cursor.offset(false)
            }

        if (!cursor.done()) bad()

        Value.DateTime(LocalDateTime.of(checkedDate(year, month, day), checkedTime(hour, minute, second, 0)), offset)
    } catch (_: BadTime) {
        decodeError("Expected a UTC time string.", 0)
    }

/** The fraction of a second, with no zero digits at its end. */
private fun StringBuilder.fraction(microsecond: Int): StringBuilder {
    if (microsecond == 0) return this

    var digits = microsecond
    var count = 6

    while (digits % 10 == 0) {
        digits /= 10
        count -= 1
    }

    return append('.').pad(digits, count)
}

/** A GeneralizedTime string. `restricted` is the form DER has. */
internal fun generalizedTimeToString(
    value: Value.DateTime,
    restricted: Boolean,
): String {
    val out = StringBuilder()
    val microsecond = microsecond(value.dateTime.toLocalTime())

    if (restricted) {
        val utc = toUtc(value).dateTime

        return out
            .pad(utc.year, 4)
            .pad(utc.monthValue, 2)
            .pad(utc.dayOfMonth, 2)
            .pad(utc.hour, 2)
            .pad(utc.minute, 2)
            .pad(utc.second, 2)
            .fraction(microsecond)
            .append('Z')
            .toString()
    }

    val time = value.dateTime

    out.pad(time.year, 4).pad(time.monthValue, 2).pad(time.dayOfMonth, 2).pad(time.hour, 2).pad(time.minute, 2)

    // A fraction after the minute would be one of a minute (X.680 46.3).
    if (time.second != 0 || microsecond != 0) out.pad(time.second, 2)

    out.fraction(microsecond)

    when (val offset = value.offsetMinutes) {
        null -> {}
        0 -> out.append('Z')
        else -> out.offset(offset)
    }

    return out.toString()
}

internal fun generalizedTimeFromString(
    text: String,
    restricted: Boolean,
): Value.DateTime =
    try {
        if (restricted) {
            if (!text.endsWith('Z')) bad()

            if ('.' in text && (text.length < 2 || text[text.length - 2] == '0' || text.getOrNull(14) != '.')) bad()

            if ('.' !in text && text.length != 15) bad()
        }

        val cursor = Cursor(text)
        val year = cursor.number(4)
        val month = cursor.number(2)
        val day = cursor.number(2)
        val hour = cursor.number(2)
        // The minute and the second may be left out, and a fraction is one
        // of the last of the hour, the minute and the second there is
        // (X.680 46.3).
        val minute = if (cursor.isDigit()) cursor.number(2) else null
        val second = if (minute != null && cursor.isDigit()) cursor.number(2) else null
        val unit =
            when {
                minute == null -> 3_600_000_000L
                second == null -> 60_000_000L
                else -> 1_000_000L
            }
        val fraction =
            if (cursor.peek() == '.' || cursor.peek() == ',') {
                cursor.at += 1
                cursor.fraction(unit)
            } else {
                0L
            }
        val offset =
            when (cursor.peek()) {
                'Z' -> {
                    cursor.at += 1
                    0
                }
                '+', '-' -> cursor.offset(true)
                else -> null
            }

        if (!cursor.done()) bad()

        val minutes = (minute ?: 0) + fraction / 60_000_000
        val rest = fraction % 60_000_000
        val seconds = (second ?: 0) + rest / 1_000_000

        if (minutes > 255 || seconds > 255) bad()

        Value.DateTime(
            LocalDateTime.of(
                checkedDate(year, month, day),
                checkedTime(hour, minutes.toInt(), seconds.toInt(), (rest % 1_000_000).toInt()),
            ),
            offset,
        )
    } catch (_: BadTime) {
        decodeError("Expected a generalized time string.", 0)
    }

private fun timeError(): Nothing = encodeError("Expected a time string.")

/** Runs `block`, of which a bad time is an error of encoding, as the runtime for Rust has it. */
private inline fun <T> reading(block: () -> T): T =
    try {
        block()
    } catch (_: BadTime) {
        timeError()
    }

internal fun dateToString(value: LocalDate): String =
    StringBuilder()
        .pad(value.year, 4)
        .pad(value.monthValue, 2)
        .pad(value.dayOfMonth, 2)
        .toString()

internal fun dateFromString(text: String): LocalDate =
    reading {
        val cursor = Cursor(text)
        val year = cursor.number(4)
        val month = cursor.number(2)
        val day = cursor.number(2)

        if (!cursor.done()) bad()

        checkedDate(year, month, day)
    }

/** Checks that a time is one of a TIME-OF-DAY or a DATE-TIME, which are a local time to the second (X.680 38.4.2, 38.4.3). */
internal fun checkLocalTime(
    time: LocalTime,
    offsetMinutes: Int?,
) {
    if (microsecond(time) != 0 || offsetMinutes != null) {
        encodeError("A TIME-OF-DAY or a DATE-TIME has no fraction of a second and no offset.")
    }
}

/** A TIME-OF-DAY as BER writes it (X.690 8.26.3). */
internal fun timeOfDayToString(value: Value.Time): String {
    checkLocalTime(value.time, value.offsetMinutes)

    return StringBuilder()
        .pad(value.time.hour, 2)
        .pad(value.time.minute, 2)
        .pad(value.time.second, 2)
        .toString()
}

internal fun timeOfDayFromString(text: String): LocalTime =
    reading {
        val cursor = Cursor(text)
        val hour = cursor.number(2)
        val minute = cursor.number(2)
        val second = cursor.number(2)

        if (!cursor.done()) bad()

        checkedTime(hour, minute, second, 0)
    }

/** A DATE-TIME as BER writes it (X.690 8.26.4). */
internal fun dateTimeToString(value: Value.DateTime): String {
    checkLocalTime(value.dateTime.toLocalTime(), value.offsetMinutes)

    val time = value.dateTime

    return StringBuilder()
        .pad(time.year, 4)
        .pad(time.monthValue, 2)
        .pad(time.dayOfMonth, 2)
        .pad(time.hour, 2)
        .pad(time.minute, 2)
        .pad(time.second, 2)
        .toString()
}

internal fun dateTimeFromString(text: String): LocalDateTime =
    reading {
        val cursor = Cursor(text)
        val year = cursor.number(4)
        val month = cursor.number(2)
        val day = cursor.number(2)
        val hour = cursor.number(2)
        val minute = cursor.number(2)
        val second = cursor.number(2)

        if (!cursor.done()) bad()

        LocalDateTime.of(checkedDate(year, month, day), checkedTime(hour, minute, second, 0))
    }

/** The forms of JER: as `str()` of Python writes dates and times, read with `%Y-%m-%d`, `%H:%M:%S` and `%Y-%m-%dT%H:%M:%S`. */
internal object Iso {
    fun dateToString(value: LocalDate): String =
        StringBuilder()
            .pad(value.year, 4)
            .append('-')
            .pad(value.monthValue, 2)
            .append('-')
            .pad(value.dayOfMonth, 2)
            .toString()

    fun timeToString(
        time: LocalTime,
        offsetMinutes: Int?,
    ): String {
        checkLocalTime(time, offsetMinutes)

        return StringBuilder()
            .pad(time.hour, 2)
            .append(':')
            .pad(time.minute, 2)
            .append(':')
            .pad(time.second, 2)
            .toString()
    }

    fun dateTimeToString(value: Value.DateTime): String =
        dateToString(value.dateTime.toLocalDate()) + "T" + timeToString(value.dateTime.toLocalTime(), value.offsetMinutes)

    /** One or two digits, as many as there are. */
    private fun number(cursor: Cursor): Int {
        var value = 0
        var digits = 0

        while (digits < 2 && cursor.isDigit()) {
            value = value * 10 + (cursor.text[cursor.at] - '0')
            cursor.at += 1
            digits += 1
        }

        if (digits == 0) bad()

        return value
    }

    private fun expect(
        cursor: Cursor,
        c: Char,
    ) {
        if (cursor.peek() != c) bad()

        cursor.at += 1
    }

    private fun readDate(cursor: Cursor): LocalDate {
        val year = cursor.number(4)

        expect(cursor, '-')

        val month = number(cursor)

        expect(cursor, '-')

        return checkedDate(year, month, number(cursor))
    }

    private fun readTime(cursor: Cursor): LocalTime {
        val hour = number(cursor)

        expect(cursor, ':')

        val minute = number(cursor)

        expect(cursor, ':')

        return checkedTime(hour, minute, number(cursor), 0)
    }

    fun dateFromString(text: String): LocalDate =
        reading {
            val cursor = Cursor(text)
            val date = readDate(cursor)

            if (!cursor.done()) bad()

            date
        }

    fun timeFromString(text: String): LocalTime =
        reading {
            val cursor = Cursor(text)
            val time = readTime(cursor)

            if (!cursor.done()) bad()

            time
        }

    fun dateTimeFromString(text: String): LocalDateTime =
        reading {
            val cursor = Cursor(text)
            val date = readDate(cursor)

            expect(cursor, 'T')

            val time = readTime(cursor)

            if (!cursor.done()) bad()

            LocalDateTime.of(date, time)
        }
}

/** A time element of a DURATION: its designator, its digits, and those of its fraction. */
private class Component(
    val designator: Char,
    val name: String,
    val digits: String,
    val fraction: String?,
)

/** A number of a DURATION: its digits, and those of its fraction. */
private fun parseNumber(text: String): Pair<String, String?>? {
    if (text.isEmpty()) return null

    val separator = text.indexOfFirst { it == '.' || it == ',' }
    val whole = if (separator < 0) text else text.substring(0, separator)
    val fraction = if (separator < 0) null else text.substring(separator + 1)

    if (fraction != null && (fraction.isEmpty() || !fraction.all { it in '0'..'9' })) return null

    if (whole.isEmpty() || !whole.all { it in '0'..'9' }) return null

    if (whole.length > 1 && whole.startsWith('0')) return null

    return whole to fraction
}

private val DATE_DESIGNATORS = listOf('Y' to "years", 'M' to "months", 'D' to "days")
private val TIME_DESIGNATORS = listOf('H' to "hours", 'M' to "minutes", 'S' to "seconds")

private fun parseDuration(text: String): List<Component> {
    if (!text.startsWith('P')) timeError()

    val rest = text.substring(1)

    if (rest.isEmpty()) timeError()

    if (rest.endsWith('W') && 'T' !in rest && rest.dropLast(1).none { it == 'Y' || it == 'M' || it == 'D' }) {
        val (whole, fraction) = parseNumber(rest.dropLast(1)) ?: timeError()

        return listOf(Component('W', "weeks", whole, fraction))
    }

    if ('W' in rest) timeError()

    val t = rest.indexOf('T')
    val datePart = if (t < 0) rest else rest.substring(0, t)
    val timePart = if (t < 0) "" else rest.substring(t + 1)

    if (t >= 0 && timePart.isEmpty()) timeError()

    val components = ArrayList<Component>()

    fun read(
        part: String,
        designators: List<Pair<Char, String>>,
    ) {
        var position = 0

        for ((letter, name) in designators) {
            val index = part.indexOf(letter, position)

            if (index >= 0) {
                val (whole, fraction) = parseNumber(part.substring(position, index)) ?: timeError()

                components.add(Component(letter, name, whole, fraction))
                position = index + 1
            }
        }

        if (position != part.length) timeError()
    }

    read(datePart, DATE_DESIGNATORS)

    if (t >= 0) read(timePart, TIME_DESIGNATORS)

    if (components.isEmpty()) timeError()

    // Only the last element may have a fraction.
    if (components.dropLast(1).any { it.fraction != null }) timeError()

    return components
}

internal fun validateDuration(text: String) {
    parseDuration(text)
}

/** The elements that are not 0 but the last, as DER writes a DURATION. */
private fun significant(components: List<Component>): List<Component> =
    components.filterIndexed { index, it -> it.digits != "0" || it.fraction != null || index == components.size - 1 }

internal fun durationToCanonical(text: String): String {
    val out = StringBuilder("P")
    var hasTime = false

    for (component in significant(parseDuration(text))) {
        if (component.name in setOf("hours", "minutes", "seconds") && !hasTime) {
            out.append('T')
            hasTime = true
        }

        out.append(component.digits)

        component.fraction?.let { out.append('.').append(it) }

        out.append(component.designator)
    }

    return out.toString()
}

/**
 * The time elements of a duration as DURATION-EQUIVALENT has them (X.680
 * 38.4.4.2), with no zero one but the least significant, as they are the
 * same abstract value (X.680 38.2.3): the name of each, its digits, and
 * those of its fractional part.
 */
internal fun durationEquivalent(text: String): List<Triple<String, String, String?>> =
    significant(parseDuration(text)).map { Triple(it.name, it.digits, it.fraction) }
