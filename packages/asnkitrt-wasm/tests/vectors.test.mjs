// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

// The vectors of tests/vectors, which the runtime for Rust recorded,
// through the JavaScript of this runtime: the values are converted to
// those of JavaScript, encoded and decoded, and have to come out the
// same, as they do in the runtime for Rust (tests/vectors.rs).

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { gunzipSync } from 'node:zlib';

import { Specification } from '../js/node.js';

const VECTORS = new URL('../../../tests/vectors/', import.meta.url);

function hex(text) {
  return Uint8Array.from(text.match(/../g) ?? [], (byte) => parseInt(byte, 16));
}

function hexOf(bytes) {
  return Buffer.from(bytes).toString('hex');
}

/** An integer as the runtime gives it: a number if it is safe, or a bigint. */
function integer(text) {
  const value = BigInt(text);

  return value >= BigInt(Number.MIN_SAFE_INTEGER) && value <= BigInt(Number.MAX_SAFE_INTEGER)
    ? Number(value)
    : value;
}

/** Parses JSON with integers of any size, and REALs as numbers. */
function parse(line) {
  return JSON.parse(line, (key, value, context) => {
    if (typeof value === 'number' && !/[.eE]/.test(context.source)) {
      return integer(context.source);
    }

    return value;
  });
}

function time(text) {
  const match = /^(\d\d):(\d\d):(\d\d)(?:\.(\d+))?(?:([+-])(\d\d):(\d\d))?/.exec(text);
  const [, hour, minute, second, fraction, sign, hours, minutes] = match;

  return {
    hour: Number(hour),
    minute: Number(minute),
    second: Number(second),
    microsecond: fraction ? Number(fraction) : 0,
    offset: sign ? (sign === '-' ? -1 : 1) * (Number(hours) * 60 + Number(minutes)) : null,
  };
}

function date(text) {
  return { year: Number(text.slice(0, 4)), month: Number(text.slice(5, 7)), day: Number(text.slice(8, 10)) };
}

/** The value of a vector, as a value of JavaScript. */
function toJs(json) {
  if (Array.isArray(json)) {
    return json.map(toJs);
  }

  if (json === null || typeof json !== 'object') {
    return json;
  }

  const keys = Object.keys(json);

  if (keys.length === 1) {
    const value = json[keys[0]];

    switch (keys[0]) {
      case '$bytes':
        return hex(value);
      case '$bits':
        return [hex(value[0]), Number(value[1])];
      case '$choice':
        return [value[0], toJs(value[1])];
      case '$float':
        return { inf: Infinity, '-inf': -Infinity, '-0.0': -0 }[value] ?? NaN;
      case '$datetime':
        return { ...date(value), ...time(value.slice(11)) };
      case '$date':
        return date(value);
      case '$time':
        return time(value);
    }
  }

  return Object.fromEntries(keys.map((key) => [key, toJs(json[key])]));
}

/** If two values are the same, whatever order the members are in. */
function same(a, b) {
  if (typeof a === 'number' && typeof b === 'number') {
    return (Number.isNaN(a) && Number.isNaN(b)) || a === b;
  }

  if (a instanceof Uint8Array && b instanceof Uint8Array) {
    return hexOf(a) === hexOf(b);
  }

  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((item, index) => same(item, b[index]));
  }

  if (a !== null && b !== null && typeof a === 'object' && typeof b === 'object') {
    const keys = Object.keys(a);

    return (
      keys.length === Object.keys(b).length &&
      keys.every((key) => Object.hasOwn(b, key) && same(a[key], b[key]))
    );
  }

  return a === b;
}

function describe(value) {
  return JSON.stringify(value, (key, item) => {
    if (typeof item === 'bigint') {
      return `${item}n`;
    }

    if (item instanceof Uint8Array) {
      return hexOf(item);
    }

    return item;
  });
}

test('the vectors', () => {
  const lines = gunzipSync(readFileSync(new URL('vectors.jsonl.gz', VECTORS)))
    .toString('utf-8')
    .split('\n')
    .filter((line) => line.length > 0);
  const specs = new Map();
  const passed = {};
  const skipped = {};
  const failures = [];

  for (const line of lines) {
    const vector = parse(line);
    const key = [vector.spec, vector.codec, vector.numeric_enums, vector.lenient].join(' ');

    if (!specs.has(key)) {
      try {
        specs.set(
          key,
          new Specification(readFileSync(new URL(`specs/${vector.spec}.asni`, VECTORS)), vector.codec, {
            numericEnums: vector.numeric_enums,
            lenient: vector.lenient,
          }),
        );
      } catch (error) {
        specs.set(key, error);
      }
    }

    const spec = specs.get(key);
    const about = `${vector.codec} ${vector.operation} ${vector.spec} ${vector.type}: ${describe(vector.value)}`;

    if (spec instanceof Error) {
      failures.push(`not loaded: ${spec.message} <= ${about}`);
      continue;
    }

    const options = {
      checkConstraints: vector.options.check_constraints === true,
      indent: vector.options.indent,
    };
    const value = toJs(vector.value);
    let outcome;

    try {
      if (vector.operation === 'encode') {
        const encoded = hexOf(spec.encode(vector.type, value, options));

        outcome =
          vector.error !== null
            ? `encoded ${encoded} but expected ${vector.error}`
            : encoded === vector.encoded
              ? null
              : `encoded ${encoded} instead of ${vector.encoded}`;
      } else {
        const decoded = spec.decode(vector.type, hex(vector.encoded), options);

        outcome =
          vector.error !== null
            ? `decoded ${describe(decoded)} but expected ${vector.error}`
            : same(decoded, value)
              ? null
              : `decoded ${describe(decoded)}`;
      }
    } catch (error) {
      if (vector.error !== null) {
        outcome = null;
      } else if (error.name === 'UnsupportedError') {
        skipped[vector.codec] = (skipped[vector.codec] ?? 0) + 1;
        continue;
      } else {
        outcome = `failed: ${error.name}: ${error.message}`;
      }
    }

    if (outcome === null) {
      passed[vector.codec] = (passed[vector.codec] ?? 0) + 1;
    } else {
      failures.push(`${outcome.slice(0, 160)} <= ${about.slice(0, 200)}`);
    }
  }

  for (const spec of specs.values()) {
    if (!(spec instanceof Error)) {
      spec.free();
    }
  }

  console.log('passed:', passed, 'skipped:', skipped, 'failed:', failures.length);
  assert.deepEqual(failures.slice(0, 40), []);
});
