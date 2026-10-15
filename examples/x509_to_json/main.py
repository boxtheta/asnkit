#!/usr/bin/env python
# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0


"""Dump an X.509 certificate, in PEM or DER, as JSON."""

import argparse
import base64
import datetime
import json
import os
import re
import sys

import asnkitrt

SCRIPT_DIR = os.path.dirname(os.path.realpath(__file__))
RFC5280_ASNI_PATH = os.path.join(SCRIPT_DIR, "..", "compiled", "rfc5280.asni")

PEM_RE = re.compile(
    rb"-----BEGIN CERTIFICATE-----(.+?)-----END CERTIFICATE-----", re.DOTALL
)


def pem_to_der(data):
    """Returns the DER of the first certificate in given PEM, or given
    data as is if it is not PEM.

    """

    match = PEM_RE.search(data)

    if match is None:
        return data

    return base64.b64decode(b"".join(match.group(1).split()))


def to_json_value(value):
    """Make what the decoder returns fit in JSON: bytes as hex, bit
    strings, which are (bytes, number of bits), as a string of bits,
    and times as ISO 8601.

    """

    if isinstance(value, datetime.datetime):
        return value.isoformat()
    elif isinstance(value, (bytes, bytearray)):
        return value.hex()
    elif (
        isinstance(value, tuple)
        and len(value) == 2
        and isinstance(value[0], (bytes, bytearray))
    ):
        data, number_of_bits = value
        bits = "".join("{:08b}".format(byte) for byte in data)

        return bits[:number_of_bits]
    elif isinstance(value, (list, tuple)):
        return [to_json_value(item) for item in value]
    elif isinstance(value, dict):
        return {key: to_json_value(item) for key, item in value.items()}

    return value


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "certificate",
        nargs="?",
        default="-",
        help="PEM or DER file to read, or - for standard input (default: -).",
    )
    args = parser.parse_args()

    if args.certificate == "-":
        data = sys.stdin.buffer.read()
    else:
        with open(args.certificate, "rb") as fin:
            data = fin.read()

    rfc5280 = asnkitrt.load_asni_file(RFC5280_ASNI_PATH, "der")
    decoded = rfc5280.decode("Certificate", pem_to_der(data))

    print(json.dumps(to_json_value(decoded), indent=4))


if __name__ == "__main__":
    main()
