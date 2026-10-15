// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

// The runtime for browsers, Deno and bundlers: the WebAssembly is
// fetched from next to this module the first time a specification is
// loaded.

import init, { Specification } from '../pkg/asnkitrt_wasm.js';

let initialized;

/** Loads the WebAssembly, once. `load()` does it too. */
export function ready() {
  initialized ??= init();

  return initialized.then(() => undefined);
}

/** Loads the contents of an `.asni` file for a codec. */
export async function load(asni, codec = 'ber', options = {}) {
  await ready();

  return new Specification(asni, codec, options);
}

export { Specification };
