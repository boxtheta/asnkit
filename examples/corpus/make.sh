#!/bin/sh
# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

# Makes the .asni files, bound and not. Run it in this folder.

set -e

asnkitcc compile -o plain.asni foo.asn
asnkitcc compile -o bound.asni --bound-to corpus foo.asn
