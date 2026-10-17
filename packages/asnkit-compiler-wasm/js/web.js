// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

// The compiler for browsers, Deno and bundlers: the WebAssembly is
// fetched from next to this module the first time it is used.

import init, * as wasm from '../pkg/asnkit_compiler_wasm.js';

let initialized;

/** Loads the WebAssembly, once. The other functions do it too. */
export function ready() {
  initialized ??= init();

  return initialized.then(() => undefined);
}

/** Compiles the text of a specification to the contents of an `.asni` file. */
export async function compile(specification, options = {}) {
  await ready();

  return wasm.compile(specification, options);
}

/** Compiles, and tells what the modules have. */
export async function compileWithSummary(specification, options = {}) {
  await ready();

  return wasm.compileWithSummary(specification, options);
}

/** Parses the text of a specification to the contents of an `.asnst` file. */
export async function parse(specification) {
  await ready();

  return wasm.parse(specification);
}

/** The version of the compiler, once `ready()` is done. */
export function version() {
  return wasm.version();
}
