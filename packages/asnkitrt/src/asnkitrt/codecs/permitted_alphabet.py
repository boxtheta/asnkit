# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Permitted alphabet."""

import string

NUMERIC_STRING = " 0123456789"

PRINTABLE_STRING = (
    string.ascii_uppercase + string.ascii_lowercase + string.digits + " '()+,-./:=?"
)

IA5_STRING = "".join([chr(v) for v in range(128)])

# ud800 - udfff are reserved code points for utf-16 surrogates.
# at this point, do not support code points in supplementary planes.
BMP_STRING = "".join([chr(v) for v in range(65536) if v < 0xD800 or v > 0xDFFF])

VISIBLE_STRING = "".join([chr(v) for v in range(32, 127)])
