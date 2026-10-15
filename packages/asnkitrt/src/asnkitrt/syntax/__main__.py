# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Print a syntax tree or a compiled specification as JSON.

Give an ``.asnst`` or ``.asni`` file::

    python -m asnkitrt.syntax foo.asni
    python -m asnkitrt.syntax foo.asnst

"""

import argparse
import dataclasses
import json
from typing import Any, Sequence

from .container import from_asni, from_asnst, read_header
from .wire import to_json


def main(arguments: Sequence[str] | None = None) -> None:
    parser = argparse.ArgumentParser(
        prog="python -m asnkitrt.syntax",
        description="Print a syntax tree or a compiled specification as JSON.",
    )
    parser.add_argument(
        "--indent",
        type=int,
        default=4,
        help="Number of spaces to indent by (default: %(default)s).",
    )
    parser.add_argument(
        "--header",
        action="store_true",
        help="Print the header of the file, with its metadata, instead.",
    )
    parser.add_argument(
        "files",
        nargs="+",
        metavar="FILE",
        help="An .asni file or an .asnst file.",
    )
    args = parser.parse_args(arguments)
    tree: Any

    if not args.files[0].endswith((".asni", ".asnst")):
        parser.error(
            "Expected an .asni or an .asnst file. ASN.1 is parsed by asnkitcc."
        )

    if len(args.files) > 1:
        parser.error("Only one .asni or .asnst file can be given.")

    with open(args.files[0], "rb") as fin:
        data = fin.read()

    header = read_header(data)

    if args.header:
        # The identifier of a key as hexadecimal digits.
        print(
            json.dumps(
                dataclasses.asdict(header), indent=args.indent, default=bytes.hex
            )
        )

        return

    # Whatever its nonce and what it is bound to, as it is only
    # looked at.
    if header.kind == "asni":
        # Who signed it does not matter either.
        tree = from_asni(
            data,
            accept_any_nonce=True,
            accept_any_binding=True,
            accept_any_signature=True,
        )
    else:
        tree = from_asnst(data)

    print(to_json(tree, args.indent))


if __name__ == "__main__":
    main()
