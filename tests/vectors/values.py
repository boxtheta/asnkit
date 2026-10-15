# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Values of the runtime for Python as they are written in the vectors.

A value is JSON, with what JSON can not tell written as an object of
one key:

=====================  ==============================================
``{"$bytes": hex}``    OCTET STRING.
``{"$bits": [hex, n]}``  BIT STRING of n bits.
``{"$choice": [name, value]}``  CHOICE. ``["", null]`` is an unknown
                       alternative of an extensible one, ``(None, None)``
                       in Python.
``{"$float": text}``   ``inf``, ``-inf``, ``nan`` or ``-0.0``.
``{"$datetime": s}``   ``datetime.isoformat()``.
``{"$date": s}``       ``date.isoformat()``.
``{"$time": s}``       ``time.isoformat()``.
=====================  ==============================================

Everything else is itself: integers (of any size), floats, strings,
booleans, ``null`` for NULL, lists, and maps for SEQUENCE and SET.

"""

import datetime
import math


class Skip(Exception):
    """A value that can not be written."""


def to_json(value):
    if value is None or isinstance(value, (bool, str)):
        return value
    elif isinstance(value, int):
        return value
    elif isinstance(value, float):
        if math.isnan(value):
            return {"$float": "nan"}
        elif math.isinf(value):
            return {"$float": "inf" if value > 0 else "-inf"}
        elif value == 0.0 and math.copysign(1.0, value) < 0:
            return {"$float": "-0.0"}

        return value
    elif isinstance(value, (bytes, bytearray)):
        return {"$bytes": bytes(value).hex()}
    elif value == (None, None):
        return {"$choice": ["", None]}
    elif isinstance(value, tuple):
        if len(value) == 2 and isinstance(value[0], (bytes, bytearray)):
            if not isinstance(value[1], int):
                raise Skip()

            return {"$bits": [bytes(value[0]).hex(), value[1]]}
        elif len(value) == 2 and isinstance(value[0], str):
            return {"$choice": [value[0], to_json(value[1])]}

        raise Skip()
    elif isinstance(value, list):
        return [to_json(item) for item in value]
    elif isinstance(value, dict):
        if not all(isinstance(key, str) for key in value):
            raise Skip()

        return {key: to_json(item) for key, item in value.items()}
    elif isinstance(value, datetime.datetime):
        return {"$datetime": value.isoformat()}
    elif isinstance(value, datetime.date):
        return {"$date": value.isoformat()}
    elif isinstance(value, datetime.time):
        return {"$time": value.isoformat()}

    raise Skip()


def from_json(value):
    """The inverse of `to_json()`."""

    if isinstance(value, list):
        return [from_json(item) for item in value]
    elif not isinstance(value, dict):
        return value

    if len(value) == 1:
        [(key, item)] = value.items()

        if key == "$bytes":
            return bytes.fromhex(item)
        elif key == "$bits":
            return (bytes.fromhex(item[0]), item[1])
        elif key == "$choice":
            if item == ["", None]:
                return (None, None)

            return (item[0], from_json(item[1]))
        elif key == "$float":
            return float(item)
        elif key == "$datetime":
            return datetime.datetime.fromisoformat(item)
        elif key == "$date":
            return datetime.date.fromisoformat(item)
        elif key == "$time":
            return datetime.time.fromisoformat(item)

    return {key: from_json(item) for key, item in value.items()}
