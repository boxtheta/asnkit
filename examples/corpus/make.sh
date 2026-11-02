#!/bin/sh
# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

# Makes the .asni files, whole and corrupt. Run it in this folder.

set -e

asnkitcc compile -o plain.asni foo.asn

# The same file, with the lowest bit of its last byte flipped, and the
# CRC-32 of the header left as it was.
size=$(wc -c < plain.asni)
last=$(tail -c 1 plain.asni | od -An -tu1 | tr -d ' ')
head -c $((size - 1)) plain.asni > corrupt.asni
printf "$(printf '\\%03o' $((last ^ 1)))" >> corrupt.asni
