// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

/** The codecs a specification can be loaded for. */
export type Codec = 'ber' | 'der' | 'oer' | 'per' | 'uper' | 'jer' | 'xer';

/** A date: DATE. */
export interface AsnDate {
  year: number;
  month: number;
  day: number;
}

/**
 * A time of day: TIME-OF-DAY. `offset` is minutes east of UTC, or
 * `null` for local time.
 */
export interface AsnTime {
  hour: number;
  minute: number;
  second: number;
  microsecond: number;
  offset: number | null;
}

/** A date and a time: UTCTime, GeneralizedTime and DATE-TIME. */
export type AsnDateTime = AsnDate & AsnTime;

/**
 * A value of an ASN.1 type, as the runtime for Python has it:
 *
 * - BOOLEAN: `boolean`. NULL: `null`.
 * - INTEGER: a `number`, or a `bigint` outside the safe integers, or
 *   the name of a named number when encoding.
 * - REAL: a `number`.
 * - ENUMERATED: the name, or a `number` with `numericEnums`.
 * - OCTET STRING, ANY and open types: a `Uint8Array` of the bytes.
 * - BIT STRING: `[bytes, number of bits]`.
 * - OBJECT IDENTIFIER: a string, as `'1.2.3'`. Character strings: a
 *   string.
 * - SEQUENCE, SET and EXTERNAL: an object of the members that are there.
 * - SEQUENCE OF and SET OF: an array.
 * - CHOICE: `[name of the alternative, value]`.
 * - Dates and times: the objects above. A `Date` is accepted when
 *   encoding, in UTC.
 */
export type Value =
  | null
  | boolean
  | number
  | bigint
  | string
  | Uint8Array
  | Date
  | AsnDate
  | AsnTime
  | AsnDateTime
  | Value[]
  | { [member: string]: Value };

/** A key and its identifier, of 16 and 32 bytes. */
export type KeyPair = [id: Uint8Array, key: Uint8Array];

/** What an `.asnc` file has to be to be loaded, and how. */
export interface LoadOptions {
  /** ENUMERATED values are numbers and not names. */
  numericEnums?: boolean;
  /** DER decodes what BER allows. */
  lenient?: boolean;
  /** The nonce the file has to have, or a function that tells if one is accepted. */
  nonce?: number | ((nonce: number) => boolean);
  acceptAnyNonce?: boolean;
  /** What the file has to be bound to. */
  boundTo?: string;
  acceptAnyBinding?: boolean;
  /** The Ed25519 public keys of which one has to have signed the file. */
  verificationKeys?: Iterable<KeyPair>;
  /** Loads a signed file without checking the signature. */
  acceptAnySignature?: boolean;
  /** The AES-256 keys to decrypt the file with. */
  decryptionKeys?: Iterable<KeyPair>;
}

export interface CodecOptions {
  /** Checks the value against the constraints of its type. */
  checkConstraints?: boolean;
  /** The module of the type, if more than one has a type with the name. */
  module?: string;
  /** encode() of JER and XER only: the number of spaces for each level. */
  indent?: number;
}

/** A compiled specification, loaded for a codec. */
export class Specification {
  /** Loads a specification. The WebAssembly has to be loaded first: see `ready()`. */
  constructor(asnc: Uint8Array, codec?: Codec, options?: LoadOptions);
  readonly codec: Codec;
  encode(name: string, value: Value, options?: CodecOptions): Uint8Array;
  /** Decodes bytes, or the text of JER and XER. */
  decode(name: string, data: Uint8Array | string, options?: CodecOptions): Value;
  /** BER and DER only. Also gives the number of bytes the value took. */
  decodeWithLength(
    name: string,
    data: Uint8Array,
    options?: Omit<CodecOptions, 'module'>,
  ): [Value, number];
  /** BER and DER only. `null` if there is not enough data to tell. */
  decodeLength(data: Uint8Array): number | null;
  /** Throws a `ConstraintsError` if the value does not fulfil its constraints. */
  checkConstraints(name: string, value: Value, options?: Pick<CodecOptions, 'module'>): void;
  free(): void;
}

/** Loads the WebAssembly, which `load()` does too. */
export function ready(): Promise<void>;

/** Loads the contents of an `.asnc` file. */
export function load(asnc: Uint8Array, codec?: Codec, options?: LoadOptions): Promise<Specification>;

/** Node.js only: loads the `.asnc` file with this path. */
export function loadFile(path: string, codec?: Codec, options?: LoadOptions): Promise<Specification>;
