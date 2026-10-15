# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Encode and decode ASN.1 types of compiled specifications, which
are found in `.asni` files.

Nothing in this package parses ASN.1. That is what the compiler
`asnkitcc` does, which makes the `.asni` files.

```python
>>> import asnkitrt
>>> foo = asnkitrt.load_asni_file('foo.asni', 'uper')
>>> foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
```

"""

from . import codecs, constants, errors, runtime, syntax
from .errors import (
    CompileError,
    ConstraintsError,
    DecodeError,
    EncodeError,
    Error,
    ParseError,
)
from .runtime import Specification, load, load_asni, load_asni_file
from .version import __version__

__author__ = "BoxTheta"

__all__ = [
    "CompileError",
    "ConstraintsError",
    "DecodeError",
    "EncodeError",
    "Error",
    "ParseError",
    "Specification",
    "codecs",
    "constants",
    "errors",
    "load",
    "load_asni",
    "load_asni_file",
    "runtime",
    "syntax",
    "__version__",
]
