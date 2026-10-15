#!/bin/sh
# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

# Makes the .asni files of every combination of signed, encrypted and
# bound. Run it in this folder. The initialization vector of an
# encrypted file is random, so those come out different every time.

set -e

for signed in "" signed; do
    for encrypted in "" encrypted; do
        for bound in "" bound; do
            name=$(printf '%s\n' plain $signed $encrypted $bound | paste -sd- -)
            name=${name#plain-}
            options=
            [ -z "$signed" ] || options="$options --signing-key-file signing.key"
            [ -z "$encrypted" ] || options="$options --encryption-key-file encryption.key"
            [ -z "$bound" ] || options="$options --bound-to corpus"
            asnkitcc compile -o "$name.asni" $options foo.asn
        done
    done
done
