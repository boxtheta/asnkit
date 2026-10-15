#!/usr/bin/env python
# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

"""Example execution:

$ ./main.py
Encoding and decoding a question found in my protocol.

Encoded: 300e8001018109497320312b313d333f
Decoded: {'id': 1, 'question': 'Is 1+1=3?'}

Comparing OER encoding to struct pack, as they are similar for defined programming types.

OER encoded:
fcfffdfffffffeffffffffffffffff01000200000003000000000000000440a00000c025000000000000ff084120737472696e67021122
Struct packed:
fcfffdfffffffeffffffffffffffff01000200000003000000000000000440a00000c025000000000000ff084120737472696e67021122
OER encoded and struct packed are equal.
$

"""

import os
import binascii
import struct
import subprocess
import tempfile

import asnkitrt


def compile_files(paths, codec="ber"):
    """Compiles the files with asnkitcc, which has to be found where
    programs are, and loads the result."""

    with tempfile.TemporaryDirectory() as directory:
        output = os.path.join(directory, "out.asni")
        subprocess.run(
            ["asnkitcc", "compile", "-o", output]
            + ([paths] if isinstance(paths, str) else list(paths)),
            check=True,
        )

        return asnkitrt.load_asni_file(output, codec)


SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
PROGRAMMING_TYPES_ASN_PATH = os.path.realpath(
    os.path.join(SCRIPT_DIR, "programming_types.asn")
)

print("Encoding and decoding a question found in my protocol.")
print()

MY_PROTOCOL_ASN_PATH = os.path.realpath(os.path.join(SCRIPT_DIR, "my_protocol.asn"))

my_protocol = compile_files([MY_PROTOCOL_ASN_PATH, PROGRAMMING_TYPES_ASN_PATH])

encoded = my_protocol.encode("Question", {"id": 1, "question": "Is 1+1=3?"})
print("Encoded:", binascii.hexlify(encoded).decode("ascii"))

decoded = my_protocol.decode("Question", encoded)
print("Decoded:", decoded)
print()

print(
    "Comparing OER encoding to struct pack, as they are similar "
    "for defined programming types."
)
print()

ALL_TYPES_ASN_PATH = os.path.realpath(os.path.join(SCRIPT_DIR, "all_types.asn"))

all_types = compile_files([ALL_TYPES_ASN_PATH, PROGRAMMING_TYPES_ASN_PATH], "oer")

data = {
    "int8": -4,
    "int16": -3,
    "int32": -2,
    "int64": -1,
    "uint8": 1,
    "uint16": 2,
    "uint32": 3,
    "uint64": 4,
    "float": 5.0,
    "double": -10.5,
    "bool": True,
    "string": "A string",
    "bytes": b"\x11\x22",
}

encoded = all_types.encode("Struct", data)
print("OER encoded:")
print(binascii.hexlify(encoded).decode("ascii"))

string_length = len(data["string"].encode("ascii"))
bytes_length = len(data["bytes"])
fmt = ">bhiqBHIQfdBB{}sB{}s".format(string_length, bytes_length)
packed = struct.pack(
    fmt,
    data["int8"],
    data["int16"],
    data["int32"],
    data["int64"],
    data["uint8"],
    data["uint16"],
    data["uint32"],
    data["uint64"],
    data["float"],
    data["double"],
    255 if data["bool"] else 0,
    string_length,
    data["string"].encode("ascii"),
    bytes_length,
    data["bytes"],
)
print("Struct packed:")
print(binascii.hexlify(packed).decode("ascii"))

if encoded == packed:
    print("OER encoded and struct packed are equal.")
