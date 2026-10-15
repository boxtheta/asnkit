// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

// The runtime for Node.js: the WebAssembly is read from next to this
// module when it is imported.

import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';

import { initSync, Specification } from '../pkg/asnkitrt_wasm.js';

initSync({
  module: readFileSync(new URL('../pkg/asnkitrt_wasm_bg.wasm', import.meta.url)),
});

/** The WebAssembly is loaded already. For the same API as in a browser. */
export function ready() {
  return Promise.resolve();
}

/** Loads the contents of an `.asni` file for a codec. */
export async function load(asni, codec = 'ber', options = {}) {
  return new Specification(asni, codec, options);
}

/** Loads the `.asni` file with this path for a codec. */
export async function loadFile(path, codec = 'ber', options = {}) {
  return new Specification(await readFile(path), codec, options);
}

export { Specification };
