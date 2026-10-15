# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""Makes Python code with data classes from a compiled ASN.1
specification, for `asnkit`.

"""

from .generate import GenerateError, generate
from .version import __version__

__all__ = ["GenerateError", "generate", "__version__"]
