# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""Compiled specifications for the tests.

The tests of the codecs need specifications to encode and decode with.
They are compiled by the compiler in Rust, `asnkitcc`, once, and kept
as `.asni` files in `tests/fixtures`, named after the digest of what
they were made from. So the tests need no compiler: they load the
files with `asnkitrt`, as an application does.

A file that is nonce-checked, bound, signed or encrypted is made from the
plain one with the writer of `asnkitrt.syntax`, as the keys of the tests
are known and nothing of the compiler is tried by it.

A fixture that is not there is made when the binary of `asnkitcc` is
found, in `ASNKITCC` or in `target/release` or `target/debug`:

    cargo build --bin asnkitcc
    uv run python -m unittest

The same goes for a specification that did not compile, which is kept as
a `.error` file with what the compiler said. Commit what is made. The
CI of the Rust crates runs the tests with the binary and fails if that
makes a change, so that the fixtures are those of the compiler as it is.

"""

import dataclasses
import hashlib
import json
import os
import subprocess
import tempfile
from typing import Any

import asnkitrt
import asnkitrt.runtime
import asnkitrt.syntax
from asnkitrt.syntax.convert import _compiled_from_dict, _compiled_to_dict
from asnkitrt.errors import (
    CompileError,
    ConstraintsError,
    DecodeError,
    EncodeError,
    Error,
    ParseError,
)

__all__ = [
    "CompileError",
    "ConstraintsError",
    "DecodeError",
    "EncodeError",
    "Error",
    "ParseError",
    "compile_files",
    "compile_files_to_asni",
    "compile_string",
    "compile_string_to_asni",
    "parse_files_to_asnst",
    "parse_string_to_asnst",
]

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
FIXTURES = os.environ.get("ASNKIT_FIXTURES") or os.path.join(ROOT, "tests", "fixtures")


def _binary() -> str | None:
    candidates = [os.environ.get("ASNKITCC", "")]

    for profile in ["release", "debug"]:
        candidates.append(os.path.join(ROOT, "target", profile, "asnkitcc"))

    for candidate in candidates:
        if candidate and os.path.isfile(candidate):
            return candidate

    return None


def _read(path: str) -> str:
    with open(path, encoding="utf-8", newline="") as fin:
        return fin.read()


def _make(sources: list[tuple[str, str]], path: str, command: str) -> None:
    """Runs `asnkitcc` on the sources, and keeps what comes of it in
    `path`, or in `path` with `.error` if it fails.

    """

    binary = _binary()

    if binary is None:
        raise FileNotFoundError(
            "{} is not there, and asnkitcc is not either to make it: "
            "run 'cargo build --bin asnkitcc' or set ASNKITCC.".format(path)
        )

    with tempfile.TemporaryDirectory() as directory:
        arguments = [binary, command, "-o", os.path.join(directory, "out")]

        for name, text in sources:
            source_path = os.path.join(directory, name)

            with open(source_path, "w", encoding="utf-8", newline="") as fout:
                fout.write(text)

            arguments.append(source_path)

        result = subprocess.run(arguments, capture_output=True, cwd=directory)
        os.makedirs(FIXTURES, exist_ok=True)

        if result.returncode == 0:
            with open(os.path.join(directory, "out"), "rb") as fin:
                data = fin.read()

            with open(path, "wb") as fout:
                fout.write(data)
        else:
            message = result.stderr.decode("utf-8", "replace").strip()

            with open(path + ".error", "w", encoding="utf-8") as fout:
                fout.write(message)


def _plain(sources: list[tuple[str, str]], command: str = "compile") -> bytes:
    digest = hashlib.sha256(json.dumps(sources).encode("utf-8")).hexdigest()[:24]
    path = os.path.join(
        FIXTURES, digest + (".asni" if command == "compile" else ".asnst")
    )

    if not os.path.exists(path) and not os.path.exists(path + ".error"):
        _make(sources, path, command)

    if os.path.exists(path + ".error"):
        message = _read(path + ".error")

        raise (
            ParseError(message) if "parse" in message.lower() else CompileError(message)
        )

    with open(path, "rb") as fin:
        return fin.read()


def _sources(files: str | list[str]) -> list[tuple[str, str]]:
    if isinstance(files, str):
        files = [files]

    return [(os.path.basename(path), _read(path)) for path in files]


def _protected(
    plain: bytes,
    metadata: Any = None,
    nonce: int | None = None,
    bound_to: str | None = None,
    signing_key: tuple[bytes, bytes] | None = None,
    encryption_key: tuple[bytes, bytes] | None = None,
) -> bytes:
    if (
        metadata is None
        and nonce is None
        and bound_to is None
        and signing_key is None
        and encryption_key is None
    ):
        return plain

    if metadata is None:
        metadata = asnkitrt.syntax.read_header(plain).metadata

    if bound_to is not None:
        metadata = dataclasses.replace(metadata, bound_to=bound_to)

    return asnkitrt.syntax.to_asni(
        asnkitrt.syntax.from_asni(plain), metadata, nonce, signing_key, encryption_key
    )


def compile_string_to_asni(string: str, **options: Any) -> bytes:
    """The contents of the `.asni` file of the specification."""

    return _protected(_plain([("string.asn", string)]), **options)


def compile_files_to_asni(files: str | list[str], **options: Any) -> bytes:
    return _protected(_plain(_sources(files)), **options)


def parse_string_to_asnst(string: str) -> bytes:
    """The contents of the `.asnst` file of the specification."""

    return _plain([("string.asn", string)], "parse")


def parse_files_to_asnst(files: str | list[str]) -> bytes:
    return _plain(_sources(files), "parse")


def _dict_with_choices(
    data: bytes, any_defined_by_choices: dict[tuple[str, ...], dict[Any, str]]
) -> Any:
    # What an ANY DEFINED BY can be is not something the compiler is told:
    # the choices are put in the dictionary of the compiled specification.
    specification = _compiled_to_dict(asnkitrt.syntax.from_asni(data))

    for location, choices in any_defined_by_choices.items():
        types = specification[location[0]]["types"]

        if len(location) == 2:
            target = types[location[1]]
        else:
            for name in location[1:-1]:
                types = types[name]

            target = next(m for m in types["members"] if m["name"] == location[-1])

        # Only the name of a type, found where the member is.
        target["choices"] = {key: {"type": value} for key, value in choices.items()}

    return specification


def _with_choices(
    data: bytes,
    codec: str,
    numeric_enums: bool,
    lenient: bool,
    any_defined_by_choices: dict[tuple[str, ...], dict[Any, str]] | None,
) -> asnkitrt.Specification:
    if any_defined_by_choices is None:
        return asnkitrt.load_asni(
            data, codec, numeric_enums=numeric_enums, lenient=lenient
        )

    specification = _dict_with_choices(data, any_defined_by_choices)

    return asnkitrt.load(
        _compiled_from_dict(specification),
        codec,
        numeric_enums=numeric_enums,
        lenient=lenient,
    )


def compiled_with_choices(
    string: str, any_defined_by_choices: dict[tuple[str, ...], dict[Any, str]]
) -> Any:
    """The compiled specification, with the choices of ANY DEFINED BY."""

    return _compiled_from_dict(
        _dict_with_choices(compile_string_to_asni(string), any_defined_by_choices)
    )


def compile_string(
    string: str,
    codec: str = "ber",
    numeric_enums: bool = False,
    lenient: bool = False,
    any_defined_by_choices: dict[tuple[str, ...], dict[Any, str]] | None = None,
) -> asnkitrt.Specification:
    """The specification to encode and decode with."""

    return _with_choices(
        compile_string_to_asni(string),
        codec,
        numeric_enums,
        lenient,
        any_defined_by_choices,
    )


def compile_files(
    files: str | list[str],
    codec: str = "ber",
    numeric_enums: bool = False,
    lenient: bool = False,
    any_defined_by_choices: dict[tuple[str, ...], dict[Any, str]] | None = None,
) -> asnkitrt.Specification:
    return _with_choices(
        compile_files_to_asni(files),
        codec,
        numeric_enums,
        lenient,
        any_defined_by_choices,
    )


def dict_of_string(string: str) -> Any:
    """The resolved dictionary of the specification, which the checkers
    of types and of constraints compile."""

    return _compiled_to_dict(asnkitrt.syntax.from_asni(compile_string_to_asni(string)))


def dict_of_files(files: str | list[str]) -> Any:
    return _compiled_to_dict(asnkitrt.syntax.from_asni(compile_files_to_asni(files)))
