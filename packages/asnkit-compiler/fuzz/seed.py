# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""Make the corpus the fuzz targets start from, from the specifications
of the repository and the types of the tests of the compiler.

    python3 packages/asnkit-compiler/fuzz/seed.py

corpus/compile/ has the specifications as they are, and
corpus/compile_types/ the assignments of their modules and of the tests,
which compile_types puts in a module of its own.
"""

import hashlib
import pathlib
import re

FUZZ = pathlib.Path(__file__).resolve().parent
ROOT = FUZZ.parents[2]
# Larger specifications are slow to run, and most of what is in them is
# in the smaller ones too.
LIMIT = 32 * 1024
SKIP = {"target", ".venv", ".git", "node_modules", "corpus", "artifacts"}


def write(directory, data):
    directory.mkdir(parents=True, exist_ok=True)
    name = hashlib.sha1(data).hexdigest()
    (directory / name).write_bytes(data)


def specifications():
    for path in sorted(ROOT.rglob("*.asn")):
        if SKIP.isdisjoint(path.relative_to(ROOT).parts):
            if path.stat().st_size <= LIMIT:
                yield path.read_bytes()


def bodies(text):
    """What is between BEGIN and END of each module."""
    pattern = r"\bDEFINITIONS\b.*?::=\s*BEGIN\b(.*?)\bEND\b\s*(?=$|\S+\s+(?:\{[^}]*\}\s*)?DEFINITIONS)"

    for match in re.finditer(pattern, text, re.S):
        yield match.group(1).strip()


def test_types():
    """The types that the tests of the compiler put in a module, which are
    those that it followed without end or that ran out of stack."""
    for path in sorted((ROOT / "packages/asnkit-compiler/tests").glob("*.rs")):
        text = path.read_text()

        for match in re.finditer(
            r'(?:error_of|warnings_of|kinds_of|module)\(\s*"((?:[^"\\]|\\.)*)"', text
        ):
            yield match.group(1).encode().decode("unicode_escape")


def main():
    compile_ = FUZZ / "corpus/compile"
    types = FUZZ / "corpus/compile_types"
    count = {compile_: 0, types: 0}

    for data in specifications():
        write(compile_, data)
        count[compile_] += 1

        for body in bodies(data.decode("utf-8", "replace")):
            write(types, body.encode())
            count[types] += 1

    for body in test_types():
        write(types, body.encode())
        count[types] += 1

    for directory, number in count.items():
        print(f"{number} inputs in {directory.relative_to(ROOT)}")


if __name__ == "__main__":
    main()
