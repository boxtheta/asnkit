// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

// Values cross between JavaScript and the WebAssembly as one buffer,
// and not a call for each of their parts, which costs much more than
// the codecs. A value to encode is written as the types of JavaScript
// it is made of, which the WebAssembly converts as its ASN.1 type says
// (src/convert.rs), and a decoded value is read from the buffer the
// WebAssembly writes. The tags and their layouts are in both, and have
// to agree. Numbers are little-endian, and lengths and counts are u32.

/** The tags of the values written here. */
const NULL = 0;
const FALSE = 1;
const TRUE = 2;
const NUMBER = 3;
const INT32 = 4;
const BIGINT = 5;
const STRING = 6;
const BYTES = 7;
const ARRAY = 8;
const OBJECT = 9;
const DATE = 10;
const UNSUPPORTED = 11;
const TOO_DEEP = 12;

/**
 * How deep a value is written. The WebAssembly refuses what is nested
 * deeper than 256, and this is a little more, so that it does that as it
 * did before, and a value that holds itself is not written forever.
 */
const MAX_DEPTH = 257;

const encoder = new TextEncoder();
const decoder = new TextDecoder();

class Writer {
  constructor() {
    this.bytes = new Uint8Array(1024);
    this.view = new DataView(this.bytes.buffer);
    this.at = 0;
  }

  reserve(length) {
    if (this.at + length > this.bytes.length) {
      const bytes = new Uint8Array(Math.max(this.bytes.length * 2, this.at + length));

      bytes.set(this.bytes.subarray(0, this.at));
      this.bytes = bytes;
      this.view = new DataView(bytes.buffer);
    }
  }

  tag(tag) {
    this.reserve(1);
    this.bytes[this.at++] = tag;
  }

  u32(value) {
    this.reserve(4);
    this.view.setUint32(this.at, value, true);
    this.at += 4;
  }

  /** A length and UTF-8. */
  text(text) {
    const length = text.length;

    // A UTF-16 code unit is at most 3 bytes of UTF-8.
    this.reserve(4 + length * 3);

    const start = this.at + 4;
    let index = 0;

    // Most text is ASCII, which is quicker to copy than to encode.
    for (; index < length; index += 1) {
      const unit = text.charCodeAt(index);

      if (unit >= 0x80) {
        break;
      }

      this.bytes[start + index] = unit;
    }

    const written =
      index < length ? encoder.encodeInto(text, this.bytes.subarray(start)).written : length;

    this.view.setUint32(this.at, written, true);
    this.at = start + written;
  }

  value(value, depth) {
    switch (typeof value) {
      case 'undefined':
        this.tag(NULL);
        return;
      case 'boolean':
        this.tag(value ? TRUE : FALSE);
        return;
      case 'number':
        // Not -0, which is a REAL.
        if ((value | 0) === value && !Object.is(value, -0)) {
          this.reserve(5);
          this.bytes[this.at] = INT32;
          this.view.setInt32(this.at + 1, value, true);
          this.at += 5;
        } else {
          this.reserve(9);
          this.bytes[this.at] = NUMBER;
          this.view.setFloat64(this.at + 1, value, true);
          this.at += 9;
        }
        return;
      case 'bigint':
        this.tag(BIGINT);
        this.text(value.toString(16));
        return;
      case 'string':
        this.tag(STRING);
        this.text(value);
        return;
      case 'object':
        break;
      default:
        // A function or a symbol.
        this.tag(UNSUPPORTED);
        return;
    }

    if (value === null) {
      this.tag(NULL);
    } else if (value instanceof Uint8Array) {
      this.tag(BYTES);
      this.u32(value.length);
      this.reserve(value.length);
      this.bytes.set(value, this.at);
      this.at += value.length;
    } else if (value instanceof Date) {
      this.reserve(57);
      this.bytes[this.at] = DATE;

      const fields = [
        value.getUTCFullYear(),
        value.getUTCMonth(),
        value.getUTCDate(),
        value.getUTCHours(),
        value.getUTCMinutes(),
        value.getUTCSeconds(),
        value.getUTCMilliseconds(),
      ];

      for (let index = 0; index < 7; index += 1) {
        this.view.setFloat64(this.at + 1 + index * 8, fields[index], true);
      }

      this.at += 57;
    } else if (depth >= MAX_DEPTH) {
      this.tag(TOO_DEEP);
    } else if (Array.isArray(value)) {
      this.tag(ARRAY);
      this.u32(value.length);

      for (const item of value) {
        this.value(item, depth + 1);
      }
    } else {
      // The properties that are not undefined, which are not there.
      this.tag(OBJECT);

      const count = this.at;
      let written = 0;

      this.u32(0);

      for (const key of Object.keys(value)) {
        const item = value[key];

        if (item !== undefined) {
          this.text(key);
          this.value(item, depth + 1);
          written += 1;
        }
      }

      this.view.setUint32(count, written, true);
    }
  }
}

// A writer is used again, unless a getter of the value encodes too.
let spare = new Writer();

/** Writes `value`, and gives what is written to `use`. */
function written(value, use) {
  const writer = spare ?? new Writer();

  spare = null;
  writer.at = 0;

  try {
    writer.value(value, 0);

    return use(writer.bytes.subarray(0, writer.at));
  } finally {
    spare = writer;
  }
}

/** The tags of the values read here. */
const R_NULL = 0;
const R_FALSE = 1;
const R_TRUE = 2;
const R_NUMBER = 3;
const R_INT32 = 4;
const R_BIGINT = 5;
const R_STRING = 6;
const R_BYTES = 7;
const R_BITS = 8;
const R_LIST = 9;
const R_MAP = 10;
const R_CHOICE = 11;
const R_DATE_TIME = 12;
const R_DATE = 13;
const R_TIME = 14;

class Reader {
  constructor(bytes) {
    this.bytes = bytes;
    this.view = new DataView(bytes.buffer, bytes.byteOffset, bytes.byteLength);
    this.at = 0;
  }

  u32() {
    const value = this.view.getUint32(this.at, true);

    this.at += 4;

    return value;
  }

  text() {
    const length = this.u32();
    const start = this.at;
    const end = start + length;

    this.at = end;

    // Short ASCII is quicker to copy than to decode.
    if (length <= 32) {
      let text = '';

      for (let index = start; index < end; index += 1) {
        const byte = this.bytes[index];

        if (byte >= 0x80) {
          return decoder.decode(this.bytes.subarray(start, end));
        }

        text += String.fromCharCode(byte);
      }

      return text;
    }

    return decoder.decode(this.bytes.subarray(start, end));
  }

  /** A copy, so that it does not hold on to the whole buffer. */
  bytesOf() {
    const length = this.u32();
    const bytes = this.bytes.slice(this.at, this.at + length);

    this.at += length;

    return bytes;
  }

  /** The integer in two's complement. */
  bigint() {
    const bytes = this.bytesOf();
    let hex = '0x0';

    for (const byte of bytes) {
      hex += (byte < 16 ? '0' : '') + byte.toString(16);
    }

    const magnitude = BigInt(hex);

    return bytes.length > 0 && bytes[0] & 0x80 ? magnitude - (1n << BigInt(bytes.length * 8)) : magnitude;
  }

  date(into) {
    into.year = this.view.getInt32(this.at, true);
    into.month = this.bytes[this.at + 4];
    into.day = this.bytes[this.at + 5];
    this.at += 6;

    return into;
  }

  time(into) {
    into.hour = this.bytes[this.at];
    into.minute = this.bytes[this.at + 1];
    into.second = this.bytes[this.at + 2];
    into.microsecond = this.view.getUint32(this.at + 3, true);
    into.offset = this.bytes[this.at + 7] ? this.view.getInt32(this.at + 8, true) : null;
    this.at += 12;

    return into;
  }

  value() {
    const tag = this.bytes[this.at++];

    switch (tag) {
      case R_NULL:
        return null;
      case R_FALSE:
        return false;
      case R_TRUE:
        return true;
      case R_NUMBER: {
        const value = this.view.getFloat64(this.at, true);

        this.at += 8;

        return value;
      }
      case R_INT32: {
        const value = this.view.getInt32(this.at, true);

        this.at += 4;

        return value;
      }
      case R_BIGINT:
        return this.bigint();
      case R_STRING:
        return this.text();
      case R_BYTES:
        return this.bytesOf();
      case R_BITS: {
        const bytes = this.bytesOf();
        const bits = this.view.getFloat64(this.at, true);

        this.at += 8;

        return [bytes, bits];
      }
      case R_LIST: {
        const count = this.u32();
        const list = new Array(count);

        for (let index = 0; index < count; index += 1) {
          list[index] = this.value();
        }

        return list;
      }
      case R_MAP: {
        const count = this.u32();
        const map = {};

        for (let index = 0; index < count; index += 1) {
          const name = this.text();

          map[name] = this.value();
        }

        return map;
      }
      case R_CHOICE: {
        const name = this.text();

        return [name, this.value()];
      }
      case R_DATE_TIME:
        return this.time(this.date({}));
      case R_DATE:
        return this.date({});
      case R_TIME:
        return this.time({});
      default:
        throw new Error(`Unknown tag ${tag} of a decoded value.`);
    }
  }
}

function read(bytes) {
  return new Reader(bytes).value();
}

/**
 * Gives the class of the WebAssembly the methods that take or give
 * values, which it has with a `_` and with buffers.
 */
export function install(Specification) {
  Object.assign(Specification.prototype, {
    /**
     * Encodes `value` as the type `name`. The options are
     * `checkConstraints`, `module` and `indent`.
     */
    encode(name, value, options) {
      return written(value, (bytes) => this._encode(name, bytes, options));
    },

    /** Decodes `data`, a `Uint8Array` or, for JER and XER, a string. */
    decode(name, data, options) {
      return read(this._decode(name, data, options));
    },

    /** The same as `decode()`, but gives `[value, length]`. BER and DER only. */
    decodeWithLength(name, data, options) {
      const [bytes, length] = this._decodeWithLength(name, data, options);

      return [read(bytes), length];
    },

    /** Throws a `ConstraintsError` if the value does not fulfil its constraints. */
    checkConstraints(name, value, options) {
      written(value, (bytes) => this._checkConstraints(name, bytes, options));
    },
  });

  return Specification;
}
