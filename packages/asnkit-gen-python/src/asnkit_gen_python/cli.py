# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""The plugin ``asnkit-gen-python``, as ``asnkitcc generate`` runs it:

.. code-block:: text

   asnkit-gen-python --asnc FILE --asnst FILE --out FOLDER [--opt KEY=VALUE ...]

It writes one Python module to the folder. The options are ``name``,
which is the name of the module without ``.py`` (default: that of
the first file the specification is from), and ``codec``, which is
what the module encodes and decodes with if not told otherwise
(default: ``ber``).

"""

import argparse
import os
import sys

from asnkitrt import syntax
from asnkitrt.errors import Error

from .generate import DEFAULT_CODEC, GenerateError, generate
from .version import __version__

OPTIONS = {"name", "codec"}


def read_options(items: list[str]) -> dict[str, str]:
    options = {}

    for item in items:
        key, separator, value = item.partition("=")

        if not separator or key not in OPTIONS:
            raise GenerateError(
                "Expected the option {} as KEY=VALUE with a KEY of {}.".format(
                    item, ", ".join(sorted(OPTIONS))
                )
            )

        options[key] = value

    return options


def run(args: argparse.Namespace) -> str:
    options = read_options(args.opt)

    with open(args.asnc, "rb") as fin:
        asnc = fin.read()

    # The names of the files it was made from are in the .asnst file.
    sources: list[str] = []

    if args.asnst is not None:
        with open(args.asnst, "rb") as fin:
            header = syntax.read_header(fin.read())

        if header.metadata is not None:
            sources = [source.name for source in header.metadata.sources if source.name]

    name = options.get("name")

    if name is None:
        name = os.path.splitext(sources[0])[0] if sources else "specification"

    if not name.isidentifier():
        raise GenerateError("Expected a name of a module, but got '{}'.".format(name))

    text = generate(asnc, sources, options.get("codec", DEFAULT_CODEC))
    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, name + ".py")

    with open(path, "w", encoding="utf-8") as fout:
        fout.write(text)

    return path


def main() -> None:
    parser = argparse.ArgumentParser(
        prog="asnkit-gen-python",
        description="Makes Python code with data classes from a compiled ASN.1 "
        "specification. Run by `asnkitcc generate`.",
    )
    parser.add_argument("--version", action="version", version=__version__)
    parser.add_argument("--asnc", required=True, help="The compiled specification.")
    parser.add_argument("--asnst", help="The syntax tree of the specification.")
    parser.add_argument("--out", required=True, help="The folder to write to.")
    parser.add_argument(
        "--opt", action="append", default=[], help="An option, as KEY=VALUE."
    )
    args = parser.parse_args()

    try:
        run(args)
    except (GenerateError, Error, OSError) as error:
        sys.exit("error: {}".format(error))
