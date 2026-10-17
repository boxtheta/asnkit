// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

/** How an `.asni` file is made, as the options of `asnkitcc compile`. */
export interface CompileOptions {
  /** A number from 0 to 2^24 - 1 the runtime is to check. */
  nonce?: number;
  /** What the file is bound to, which the runtime is to check. */
  boundTo?: string;
  /** The Ed25519 private key to sign with, of 32 bytes. */
  signingKey?: Uint8Array;
  /** Its identifier, of 16 bytes, derived from the key if not given. */
  signingKeyId?: Uint8Array;
  /** The AES-256 key to encrypt with, of 32 bytes. */
  encryptionKey?: Uint8Array;
  /** Its identifier, of 16 bytes, derived from the key if not given. */
  encryptionKeyId?: Uint8Array;
}

/** What a compiled module has. */
export interface ModuleSummary {
  name: string;
  types: number;
  objectClasses: number;
}

/** Loads the WebAssembly, which the other functions do too. */
export function ready(): Promise<void>;

/**
 * Compiles the text of a specification, of one module or more, to the
 * contents of an `.asni` file. Throws an `Error` named `ParseError` or
 * `CompileError` that tells where, or a `TypeError` for wrong options.
 */
export function compile(specification: string, options?: CompileOptions): Promise<Uint8Array>;

/** The same as `compile()`, and tells what the modules have. */
export function compileWithSummary(
  specification: string,
  options?: CompileOptions,
): Promise<{ asni: Uint8Array; modules: ModuleSummary[] }>;

/** Parses the text of a specification to the contents of an `.asnst` file. */
export function parse(specification: string): Promise<Uint8Array>;

/** Compiles the ASN.1 file with this path, read as UTF-8. Node.js only. */
export function compileFile(path: string, options?: CompileOptions): Promise<Uint8Array>;

/** The version of the compiler. In a browser, once `ready()` is done. */
export function version(): string;
