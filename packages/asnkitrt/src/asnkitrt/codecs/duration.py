# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""ISO 8601 duration handling for ASN.1 DURATION (X.680 38.3.6, 38.4.4;
X.690 8.26.5, 11.9.1; X.691 32.6; X.696 29.4).

"""

import re
from typing import Any

from ..errors import DecodeError, EncodeError

# \Z and not $, which also matches before a final newline.
NUMBER_RE = re.compile(r"(0|[1-9][0-9]*)(?:([.,])([0-9]+))?\Z")

# The most digits of a fractional part that PER and OER take, as the
# runtime for Rust keeps the fractional value in an i128. X.691 and
# X.696 set no limit, but a decoder needs one: a few bytes can give a
# number of digits that does not fit in memory.
MAX_FRACTION_DIGITS = 38

ORDER = ["years", "months", "weeks", "days", "hours", "minutes", "seconds"]

DESIGNATOR_LETTERS = {
    "years": "Y",
    "months": "M",
    "weeks": "W",
    "days": "D",
    "hours": "H",
    "minutes": "M",
    "seconds": "S",
}


def parse_duration(text: str) -> list[tuple[str, int, str | None, str | None]]:
    """Parses an ISO 8601 duration string into a list of components:
    (name, integral_value, fraction_digits, separator).

    """
    if not isinstance(text, str) or not text.startswith("P"):
        raise ValueError("A DURATION must start with 'P'.")

    rest = text[1:]
    if not rest:
        raise ValueError("A DURATION must not be empty.")

    # Week designation: [P] followed by week designation (X.680 38.3.6.1 b).
    # Cannot be combined with other components (38.2.3 Note 2).
    if (
        rest.endswith("W")
        and "T" not in rest
        and not any(c in rest[:-1] for c in "YMD")
    ):
        val_str = rest[:-1]
        m = NUMBER_RE.match(val_str)
        if not m:
            raise ValueError(f"Invalid weeks designation: {val_str}")
        return [("weeks", int(m.group(1)), m.group(3), m.group(2))]

    if "W" in rest:
        raise ValueError(
            "A weeks designation cannot be combined with other designations."
        )

    date_part, sep, time_part = rest.partition("T")
    if sep and not time_part:
        raise ValueError("The 'T' separator cannot be followed by nothing.")

    components: list[tuple[str, int, str | None, str | None]] = []

    # Year-month-day designation (X.680 38.3.6.2).
    pos = 0
    for name, letter in [("years", "Y"), ("months", "M"), ("days", "D")]:
        idx = date_part.find(letter, pos)
        if idx != -1:
            val_str = date_part[pos:idx]
            m = NUMBER_RE.match(val_str)
            if not m:
                raise ValueError(f"Invalid {name} designation: {val_str}")
            components.append((name, int(m.group(1)), m.group(3), m.group(2)))
            pos = idx + 1

    if pos != len(date_part):
        raise ValueError(f"Unexpected characters in date part: {date_part[pos:]}")

    # Hours-mins-secs designation (X.680 38.3.6.3).
    if sep:
        pos = 0
        for name, letter in [("hours", "H"), ("minutes", "M"), ("seconds", "S")]:
            idx = time_part.find(letter, pos)
            if idx != -1:
                val_str = time_part[pos:idx]
                m = NUMBER_RE.match(val_str)
                if not m:
                    raise ValueError(f"Invalid {name} designation: {val_str}")
                components.append((name, int(m.group(1)), m.group(3), m.group(2)))
                pos = idx + 1

        if pos != len(time_part):
            raise ValueError(f"Unexpected characters in time part: {time_part[pos:]}")

    if not components:
        raise ValueError("A DURATION must have at least one component.")

    # A fractional part can only appear on the last component (X.680 38.3.6.13).
    for c in components[:-1]:
        if c[2] is not None:
            raise ValueError(
                "Only the last component of a DURATION can have a fractional part."
            )

    return components


def validate_duration(text: str) -> None:
    """Checks that given text is a valid ISO 8601 duration string."""
    try:
        parse_duration(text)
    except ValueError as e:
        raise EncodeError(str(e)) from None


def canonical_duration(text: str) -> str:
    """Converts a duration string to canonical form as specified in
    X.690 11.9.1 a) and d):
    - All commas converted to full stop.
    - Remove all zero time components except the least significant
      component that is present.

    """
    comps = parse_duration(text)
    last_idx = len(comps) - 1
    kept = []

    for idx, (name, val, frac, _sep) in enumerate(comps):
        if val != 0 or frac is not None or idx == last_idx:
            kept.append((name, val, frac))

    out = ["P"]
    has_time = False

    for name, val, frac in kept:
        if name in ("hours", "minutes", "seconds") and not has_time:
            out.append("T")
            has_time = True

        out.append(str(val))

        if frac is not None:
            out.append("." + frac)

        out.append(DESIGNATOR_LETTERS[name])

    return "".join(out)


def duration_to_equivalent(text: str) -> dict[str, Any]:
    """Converts an ISO 8601 duration into its DURATION-EQUIVALENT value
    (X.680 38.4.4.2), with no zero time element but the least
    significant one, as they are the same abstract value (X.680 38.2.3).
    This is also DURATION-INTERVAL-ENCODING (X.691 32.6, X.696 29.4).
    Raises ValueError if it is not a duration.

    """
    comps = parse_duration(text)
    last_idx = len(comps) - 1
    result: dict[str, Any] = {}

    for idx, (name, val, frac, _sep) in enumerate(comps):
        if val != 0 or frac is not None or idx == last_idx:
            result[name] = val

        if frac is not None:
            result["fractional-part"] = {
                "number-of-digits": len(frac),
                "fractional-value": int(frac),
            }

    return result


def duration_to_dict(text: str) -> dict[str, Any]:
    """Converts an ISO 8601 duration into a dictionary suitable for
    DURATION-INTERVAL-ENCODING (X.691 32.6, X.696 29.4).

    """
    try:
        result = duration_to_equivalent(text)
    except ValueError as e:
        raise EncodeError(str(e)) from None

    fp = result.get("fractional-part")

    if fp is not None and fp["number-of-digits"] > MAX_FRACTION_DIGITS:
        raise EncodeError(
            "A fractional part of a DURATION can have at most {} "
            "digits in PER and OER.".format(MAX_FRACTION_DIGITS)
        )

    return result


def dict_to_duration(d: dict[str, Any]) -> str:
    """Converts a decoded DURATION-INTERVAL-ENCODING dictionary into an
    ISO 8601 duration string.

    """
    if not d:
        raise DecodeError("Expected at least one component in DURATION.")

    check_fields(d)
    fp = d.get("fractional-part")

    if "weeks" in d:
        if len(d) > (2 if fp is not None else 1):
            raise DecodeError(
                "Weeks cannot be combined with other components in DURATION."
            )

        w = d["weeks"]

        if fp is not None:
            frac = f"{fp['fractional-value']:0{fp['number-of-digits']}d}"
            return f"P{w}.{frac}W"

        return f"P{w}W"

    order = ["years", "months", "days", "hours", "minutes", "seconds"]
    least = None

    for name in reversed(order):
        if name in d:
            least = name
            break

    if least is None:
        raise DecodeError("Expected at least one component in DURATION.")

    date_comps = []

    for name, letter in [("years", "Y"), ("months", "M"), ("days", "D")]:
        if name in d:
            val = d[name]

            if name == least and fp is not None:
                frac = f"{fp['fractional-value']:0{fp['number-of-digits']}d}"
                date_comps.append(f"{val}.{frac}{letter}")
            else:
                date_comps.append(f"{val}{letter}")

    time_comps = []

    for name, letter in [("hours", "H"), ("minutes", "M"), ("seconds", "S")]:
        if name in d:
            val = d[name]

            if name == least and fp is not None:
                frac = f"{fp['fractional-value']:0{fp['number-of-digits']}d}"
                time_comps.append(f"{val}.{frac}{letter}")
            else:
                time_comps.append(f"{val}{letter}")

    res = "P" + "".join(date_comps)

    if time_comps:
        res += "T" + "".join(time_comps)

    return res


def check_fields(d: dict[str, Any]) -> None:
    """Checks the fields of a decoded DURATION-INTERVAL-ENCODING as
    X.691 32.6 and X.696 29.4 have them, before they make a duration.

    """
    present = [name for name in ORDER if name in d]

    for name in present:
        if d[name] < 0:
            raise DecodeError(
                "Expected a {} of a DURATION of at least 0, but got {}.".format(
                    name, d[name]
                )
            )

    # A zero time element is absent unless it is the least significant
    # one (X.691 32.6.3, X.696 29.4.4).
    for name in present[:-1]:
        if d[name] == 0:
            raise DecodeError(
                "The {} of a DURATION is zero and not its least significant "
                "time element, so it must be absent.".format(name)
            )

    fp = d.get("fractional-part")

    if fp is not None:
        digits = fp["number-of-digits"]
        value = fp["fractional-value"]

        if not 1 <= digits <= MAX_FRACTION_DIGITS:
            raise DecodeError(
                "Expected a number of digits of the fractional part of a "
                "DURATION between 1 and {}, but got {}.".format(
                    MAX_FRACTION_DIGITS, digits
                )
            )

        if not 0 <= value < 10**digits:
            raise DecodeError(
                "Expected a fractional value of a DURATION with at most {} "
                "digits, but got {}.".format(digits, value)
            )
