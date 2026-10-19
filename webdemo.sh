#!/usr/bin/env sh

set -eu

if [ "$#" -gt 1 ]; then
    echo "usage: $0 [PORT]" >&2
    exit 1
fi

case "${1:-}" in
    -h|--help)
        echo "usage: $0 [PORT] (default: 8000)"
        exit 0
        ;;
esac

port=${1-8000}

case "$port" in
    ''|*[!0-9]*)
        echo "error: PORT must be an integer from 1 to 65535" >&2
        exit 1
        ;;
esac

if [ "${#port}" -gt 5 ] || [ "$port" -lt 1 ] || [ "$port" -gt 65535 ]; then
    echo "error: PORT must be an integer from 1 to 65535" >&2
    exit 1
fi

root=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$root"

for tool in python3 npm cargo wasm-bindgen; do
    if ! command -v "$tool" >/dev/null 2>&1; then
        echo "error: $tool is required; see examples/playground/README.rst" >&2
        exit 1
    fi
done

npm run build --prefix packages/asnkitrt-wasm
npm run build --prefix packages/asnkit-compiler-wasm

printf '\nPlayground: http://127.0.0.1:%s/examples/playground/\nPress Ctrl-C to stop.\n\n' "$port"
exec python3 -m http.server "$port" --bind 127.0.0.1
