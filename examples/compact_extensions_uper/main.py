#!/usr/bin/env python
# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

import os
import binascii
import subprocess
import tempfile

import asnkitrt


def compile_files(paths, codec="ber"):
    """Compiles the files with asnkitcc, which has to be found where
    programs are, and loads the result."""

    with tempfile.TemporaryDirectory() as directory:
        output = os.path.join(directory, "out.asnc")
        subprocess.run(
            ["asnkitcc", "compile", "-o", output]
            + ([paths] if isinstance(paths, str) else list(paths)),
            check=True,
        )

        return asnkitrt.load_asnc_file(output, codec)


SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
FOO_V1_ASN_PATH = os.path.realpath(os.path.join(SCRIPT_DIR, "v1.asn"))
FOO_V2_ASN_PATH = os.path.realpath(os.path.join(SCRIPT_DIR, "v2.asn"))
FOO_V2_EM_ASN_PATH = os.path.realpath(
    os.path.join(SCRIPT_DIR, "v2_extension_markers.asn")
)


def encode(foo, name, decoded):
    print("  Input: {}".format(decoded))
    encoded = foo.encode(name, decoded)
    print(
        "  Output: {} ({} bytes)".format(
            binascii.hexlify(encoded).decode("ascii"), len(encoded)
        )
    )


def decode(foo, name, encoded):
    print("  Input: {}".format(binascii.hexlify(encoded).decode("ascii")))
    decoded = foo.decode(name, encoded)
    print("  Output: {}".format(decoded))


def main():
    foo_v1 = compile_files(FOO_V1_ASN_PATH, "uper")
    foo_v2 = compile_files(FOO_V2_ASN_PATH, "uper")
    foo_v2_em = compile_files(FOO_V2_EM_ASN_PATH, "uper")

    decoded_v1 = ("foo", {"a": True, "b": 55, "c": 3, "d": False, "e": "on"})

    decoded_v2 = (
        "foo",
        {"a": True, "b": 55, "c": 3, "d": False, "e": "on", "v2": {"a": -1, "b": True}},
    )

    decoded_v2_bar = ("bar", 3)

    decoded_v2_em = (
        "foo",
        {"a": True, "b": 55, "c": 3, "d": False, "e": "on", "f": -1, "g": True},
    )

    print("Encode V1:")
    print()

    encode(foo_v1, "Message", decoded_v1)

    print()
    print("Encode V2:")
    print()

    encode(foo_v2, "Message", decoded_v1)
    encode(foo_v2, "Message", decoded_v2)
    encode(foo_v2, "Message", decoded_v2_bar)

    print()
    print("Encode V2 with extension markers:")
    print()

    encode(foo_v2_em, "Message", decoded_v1)
    encode(foo_v2_em, "Message", decoded_v2_em)
    encode(foo_v2_em, "Message", decoded_v2_bar)

    print()
    print("Decode V1 with V1:")
    print()

    decode(foo_v1, "Message", b"\x16\xec")

    print()
    print("Decode V1 with V2:")
    print()

    decode(foo_v2, "Message", b"\x16\xec")

    print()
    print("Decode V2 with V1:")
    print()

    decode(foo_v1, "Message", b"\x16\xec")
    decode(foo_v1, "Message", b"\x36\xec\x3f\xc0")
    decode(foo_v1, "Message", b"\x46")

    print()
    print("Decode V2 with V2:")
    print()

    decode(foo_v2, "Message", b"\x16\xec")
    decode(foo_v2, "Message", b"\x36\xec\x3f\xc0")
    decode(foo_v2, "Message", b"\x46")


if __name__ == "__main__":
    main()
