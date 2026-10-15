# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Various exceptions/errors."""


class Error(Exception):
    """Base exception of all exceptions of this package."""


class CompileError(Error):
    """General ASN.1 compile error."""


class EncodeError(Error):
    """General ASN.1 encode error."""


class DecodeError(Error):
    """General ASN.1 decode error."""


class ConstraintsError(Error):
    """General ASN.1 constraints error."""


class ParseError(Error):
    """General ASN.1 parse error."""
