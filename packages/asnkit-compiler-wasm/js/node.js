// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

// The compiler for Node.js: the WebAssembly is read from next to this
// module when it is imported.

import { readFileSync } from 'node:fs';
import { readFile } from 'node:fs/promises';

import * as wasm from '../pkg/asnkit_compiler_wasm.js';

wasm.initSync({
  module: readFileSync(new URL('../pkg/asnkit_compiler_wasm_bg.wasm', import.meta.url)),
});

/** The WebAssembly is loaded already. For the same API as in a browser. */
export function ready() {
  return Promise.resolve();
}

/** Compiles the text of a specification to the contents of an `.asni` file. */
export async function compile(specification, options = {}) {
  return wasm.compile(specification, options);
}

/** Compiles, and tells what the modules have. */
export async function compileWithSummary(specification, options = {}) {
  return wasm.compileWithSummary(specification, options);
}

/** Parses the text of a specification to the contents of an `.asnst` file. */
export async function parse(specification) {
  return wasm.parse(specification);
}

/** Compiles the ASN.1 file with this path, read as UTF-8. */
export async function compileFile(path, options = {}) {
  return wasm.compile(await readFile(path, 'utf8'), options);
}

/** The version of the compiler. */
export function version() {
  return wasm.version();
}
