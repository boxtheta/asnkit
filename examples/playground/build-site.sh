#!/bin/sh
# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

# Copies the playground into a folder of its own, with the packages it
# loads, as built in packages/asnkit-compiler-wasm and
# packages/asnkitrt-wasm, next to it. The folder is served as it is, as
# the playground of the documentation on GitHub Pages.
#
#   examples/playground/build-site.sh site/playground

set -e

out=${1:?usage: build-site.sh FOLDER}
here=$(cd "$(dirname "$0")" && pwd)
packages=$here/../../packages

mkdir -p "$out/packages"
cp "$here/index.html" "$here/main.js" "$here/style.css" "$out/"

for package in asnkit-compiler-wasm asnkitrt-wasm; do
    if [ ! -d "$packages/$package/pkg" ]; then
        echo "error: build $package first: cd packages/$package && npm run build" >&2
        exit 1
    fi

    mkdir -p "$out/packages/$package/pkg"
    cp -R "$packages/$package/js" "$out/packages/$package/"
    cp "$packages/$package/pkg/"*.js "$packages/$package/pkg/"*.wasm "$out/packages/$package/pkg/"
done

# The import map points to the copies.
sed 's|"\.\./\.\./packages/|"./packages/|' "$here/index.html" > "$out/index.html"

if grep -q '\.\./\.\./packages/' "$out/index.html"; then
    echo "error: the import map of index.html was not rewritten" >&2
    exit 1
fi
