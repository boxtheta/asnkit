// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: MIT

// What the vectors do not show: the values of JavaScript the runtime
// takes and gives, its errors, and the checks of signed, encrypted and
// bound files.

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { describe, test } from 'node:test';

import { load, loadFile, ready, Specification } from '../js/node.js';

const FOO = new URL('../../../examples/compiled/foo.asni', import.meta.url);
const CORPUS = new URL('../../../examples/corpus/', import.meta.url);
const TYPES = readFileSync(new URL('fixtures/types.asni', import.meta.url));

function hex(bytes) {
  return Buffer.from(bytes).toString('hex');
}

function bytes(text) {
  return Uint8Array.from(Buffer.from(text, 'hex'));
}

function roundTrip(spec, name, value) {
  return spec.decode(name, spec.encode(name, value));
}

describe('the example of the README', () => {
  test('BER and PER', async () => {
    const question = { id: 1, question: 'Is 1+1=3?' };
    const ber = await loadFile(FOO.pathname);

    assert.equal(ber.codec, 'ber');
    assert.equal(hex(ber.encode('Question', question)), '300e0201011609497320312b313d333f');
    assert.deepEqual(roundTrip(ber, 'Question', question), question);

    const per = await load(readFileSync(FOO), 'per');

    assert.equal(hex(per.encode('Question', question)), '010109497320312b313d333f');
  });
});

describe('values', () => {
  const spec = new Specification(TYPES, 'ber');

  test('an INTEGER is a number, or a bigint if it is not safe', () => {
    assert.equal(hex(spec.encode('Big', 5)), '020105');
    assert.equal(spec.decode('Big', bytes('020105')), 5);
    assert.equal(hex(spec.encode('Big', 5n)), '020105');
    assert.equal(hex(spec.encode('Big', 2n ** 70n)), '0209400000000000000000');
    assert.equal(spec.decode('Big', bytes('0209400000000000000000')), 2n ** 70n);
    assert.equal(roundTrip(spec, 'Big', -(2n ** 70n)), -(2n ** 70n));
    assert.equal(roundTrip(spec, 'Big', 2n ** 100n + 1n), 2n ** 100n + 1n);
    assert.equal(roundTrip(spec, 'Big', Number.MAX_SAFE_INTEGER), Number.MAX_SAFE_INTEGER);
    assert.equal(roundTrip(spec, 'Big', 2n ** 53n), 2n ** 53n);
  });

  test('a REAL of an integer is a REAL', () => {
    assert.equal(roundTrip(spec, 'Real', 1), 1);
    assert.equal(roundTrip(spec, 'Real', -2.5), -2.5);
    assert.equal(roundTrip(spec, 'Real', Infinity), Infinity);
  });

  test('an OBJECT IDENTIFIER is a string', () => {
    assert.equal(hex(spec.encode('Oid', '1.2.840.113549')), '06062a864886f70d');
    assert.equal(spec.decode('Oid', bytes('06062a864886f70d')), '1.2.840.113549');
    assert.throws(() => spec.encode('Oid', '1'), { name: 'EncodeError' });
  });

  test('a BIT STRING is [bytes, number of bits]', () => {
    assert.equal(hex(spec.encode('Bits', [bytes('a0'), 3])), '030205a0');
    assert.deepEqual(spec.decode('Bits', bytes('030205a0')), [bytes('a0'), 3]);
  });

  test('a CHOICE is [name, value], and a SEQUENCE OF of two is an array', () => {
    assert.deepEqual(roundTrip(spec, 'Choice', ['a', 7]), ['a', 7]);
    assert.deepEqual(roundTrip(spec, 'Choice', ['b', [true, false]]), ['b', [true, false]]);
    assert.deepEqual(roundTrip(spec, 'Pair', ['a', 'b']), ['a', 'b']);
  });

  test('a member that is undefined is not there', () => {
    const encoded = hex(spec.encode('Record', { id: 1 }));

    assert.equal(hex(spec.encode('Record', { id: 1, name: undefined })), encoded);
    assert.deepEqual(spec.decode('Record', bytes(encoded)), { id: 1 });
    assert.deepEqual(roundTrip(spec, 'Record', { id: 1, name: 'x' }), { id: 1, name: 'x' });
  });

  test('dates and times are objects, and a Date is UTC', () => {
    const when = { year: 2024, month: 1, day: 2, hour: 3, minute: 4, second: 5, microsecond: 0, offset: 0 };

    assert.deepEqual(roundTrip(spec, 'When', when), when);
    assert.equal(
      hex(spec.encode('When', new Date(Date.UTC(2024, 0, 2, 3, 4, 5)))),
      hex(spec.encode('When', when)),
    );
    assert.deepEqual(roundTrip(spec, 'Day', { year: 2024, month: 2, day: 29 }), {
      year: 2024,
      month: 2,
      day: 29,
    });
  });

  test('an ENUMERATED is a name, or a number with numericEnums', () => {
    assert.equal(roundTrip(spec, 'Color', 'green'), 'green');

    const numeric = new Specification(TYPES, 'ber', { numericEnums: true });

    assert.equal(numeric.decode('Color', spec.encode('Color', 'green')), 1);
  });

  test('a type of more than one module is found with module', () => {
    assert.throws(() => spec.encode('Twice', 1), { name: 'NoSuchTypeError' });
    assert.equal(hex(spec.encode('Twice', 1, { module: 'Types' })), '020101');
    assert.equal(hex(spec.encode('Twice', true, { module: 'Other' })), '0101ff');
    assert.equal(spec.decode('Twice', bytes('0101ff'), { module: 'Other' }), true);
  });
});

describe('constraints', () => {
  const spec = new Specification(TYPES, 'ber');

  test('are checked when asked to', () => {
    spec.checkConstraints('Small', 7);
    assert.throws(() => spec.checkConstraints('Small', 8), { name: 'ConstraintsError' });
    assert.throws(() => spec.encode('Small', 8, { checkConstraints: true }), {
      name: 'ConstraintsError',
    });
    assert.equal(hex(spec.encode('Small', 8)), '020108');
    assert.throws(() => spec.decode('Small', bytes('020108'), { checkConstraints: true }), {
      name: 'ConstraintsError',
    });
  });
});

describe('errors', () => {
  const spec = new Specification(TYPES, 'ber');

  test('are named after what went wrong', () => {
    assert.throws(() => spec.encode('Record', { id: 'one' }), { name: 'EncodeError' });
    assert.throws(() => spec.encode('Bits', [bytes('a0'), -1]), { name: 'EncodeError' });
    assert.throws(() => spec.encode('Nothing', 1), { name: 'NoSuchTypeError' });
    assert.throws(() => new Specification(TYPES, 'json'), TypeError);
    assert.throws(() => new Specification(bytes('00010203'), 'ber'), { name: 'LoadError' });
  });

  test('a decode error tells where', () => {
    assert.throws(() => spec.decode('Record', bytes('3005020101')), (error) => {
      assert.equal(error.name, 'DecodeError');
      assert.equal(typeof error.offset, 'number');

      return true;
    });
  });

  test('an error with a value tells where in it', () => {
    for (const [name, value, path, message] of [
      ['Record', { id: 'one' }, ['Record', 'id'], 'Record.id: Expected an integer or a named number.'],
      ['Choice', ['b', [true, 'x']], ['Choice', 'b', 1], 'Choice.b[1]: Expected data of type bool.'],
      ['Pair', ['a', 5], ['Pair', 1], 'Pair[1]: Expected data of type str.'],
    ]) {
      assert.throws(() => spec.encode(name, value), { name: 'EncodeError', path, message });
    }

    assert.throws(() => spec.checkConstraints('Small', 8), {
      name: 'ConstraintsError',
      path: ['Small'],
      message: 'Small: Expected an integer in the range.',
    });
    assert.throws(() => spec.decode('Record', bytes('3005020101')), {
      name: 'DecodeError',
      path: ['Record'],
    });
    assert.throws(() => spec.encode('Nothing', 1), (error) => error.path === undefined);
  });

  test('a value nested too deep is refused', () => {
    let value = 1;

    for (let index = 0; index < 1000; index += 1) {
      value = [value];
    }

    assert.throws(() => spec.encode('Pair', value), { name: 'EncodeError' });
  });

  test('a value that holds itself is refused', () => {
    const value = { id: 1 };

    value.name = value;
    assert.throws(() => spec.encode('Record', value), { name: 'EncodeError' });
  });

  test('a function is not a value', () => {
    assert.throws(() => spec.encode('Record', { id: 1, name: () => 'x' }), { name: 'EncodeError' });
  });
});

describe('values cross to the WebAssembly and back', () => {
  const spec = new Specification(TYPES, 'ber');

  test('with text that is not ASCII', () => {
    for (const name of ['é', 'ünïcödé', '日本語', '😀 and more', 'a'.repeat(40) + 'é']) {
      assert.deepEqual(roundTrip(spec, 'Record', { id: 1, name }), { id: 1, name });
    }
  });

  test('with the numbers of every size', () => {
    for (const number of [0, -1, 2 ** 31 - 1, -(2 ** 31), 2 ** 31, -(2 ** 31) - 1, 2 ** 40]) {
      assert.equal(roundTrip(spec, 'Big', number), number);
    }

    assert.ok(Object.is(roundTrip(spec, 'Real', -0), -0));
    assert.ok(Number.isNaN(roundTrip(spec, 'Real', NaN)));
  });

  test('with bytes that are their own', () => {
    const data = bytes('030205a0');
    const [decoded] = spec.decode('Bits', data);

    assert.equal(decoded.byteLength, 1);
    assert.equal(decoded.buffer.byteLength, 1);
  });

  test('when a getter of the value encodes too', () => {
    const value = {
      id: 1,
      get name() {
        return hex(spec.encode('Big', 5));
      },
    };

    assert.deepEqual(spec.decode('Record', spec.encode('Record', value)), { id: 1, name: '020105' });
  });
});

describe('codecs', () => {
  test('JER is decoded from text too', () => {
    const spec = new Specification(TYPES, 'jer');
    const encoded = spec.encode('Record', { id: 1, name: 'x' });

    assert.equal(new TextDecoder().decode(encoded), '{"id":1,"name":"x"}');
    assert.deepEqual(spec.decode('Record', '{"id":1,"name":"x"}'), { id: 1, name: 'x' });
  });

  test('the length of BER', () => {
    const spec = new Specification(TYPES, 'ber');
    const data = bytes('3003800101ffff');

    assert.equal(spec.decodeLength(data), 5);
    assert.equal(spec.decodeLength(bytes('30')), null);
    assert.deepEqual(spec.decodeWithLength('Record', data), [{ id: 1 }, 5]);
  });
});

describe('files that are signed, encrypted or bound', () => {
  const read = (name) => readFileSync(new URL(name, CORPUS));
  const key = (name) => bytes(readFileSync(new URL(name, CORPUS), 'utf-8').trim());
  const verificationKeys = [[bytes('d543cae4811fc8b033704dc6e7896ff7'), key('signing.pub')]];
  const decryptionKeys = new Map([[bytes('16cde2f77598fc5472df7570a510f54d'), key('encryption.key')]]);
  const question = { id: 1, question: 'Is 1+1=3?' };

  test('are only loaded when told what to expect', () => {
    for (const name of ['signed.asni', 'encrypted.asni', 'bound.asni']) {
      assert.throws(() => new Specification(read(name)), { name: 'LoadError' }, name);
    }

    assert.throws(() => new Specification(read('plain.asni'), 'ber', { verificationKeys }), {
      name: 'LoadError',
    });
  });

  test('are loaded with the keys and the binding', () => {
    const files = {
      'signed.asni': { verificationKeys },
      'encrypted.asni': { decryptionKeys },
      'bound.asni': { boundTo: 'corpus' },
      'signed-encrypted-bound.asni': { verificationKeys, decryptionKeys, boundTo: 'corpus' },
    };

    for (const [name, options] of Object.entries(files)) {
      const spec = new Specification(read(name), 'uper', options);

      assert.deepEqual(roundTrip(spec, 'Question', question), question, name);
    }
  });

  test('are refused with other keys or another binding', () => {
    const other = [[bytes('d543cae4811fc8b033704dc6e7896ff7'), new Uint8Array(32).fill(1)]];

    assert.throws(() => new Specification(read('signed.asni'), 'ber', { verificationKeys: other }), {
      name: 'LoadError',
    });
    assert.throws(() => new Specification(read('bound.asni'), 'ber', { boundTo: 'elsewhere' }), {
      name: 'LoadError',
    });
    new Specification(read('signed.asni'), 'ber', { acceptAnySignature: true });
    new Specification(read('bound.asni'), 'ber', { acceptAnyBinding: true });
  });

  test('take keys of the right sizes only', () => {
    assert.throws(
      () => new Specification(read('signed.asni'), 'ber', { verificationKeys: [[bytes('00'), bytes('00')]] }),
      TypeError,
    );
  });
});

describe('the entry for browsers', () => {
  test('fetches the WebAssembly from next to it', async () => {
    const original = globalThis.fetch;

    globalThis.fetch = async (url) =>
      new Response(readFileSync(new URL(url)), { headers: { 'Content-Type': 'application/wasm' } });

    try {
      const web = await import('../js/web.js');
      const foo = await web.load(readFileSync(FOO), 'uper');

      await web.ready();
      assert.deepEqual(roundTrip(foo, 'Question', { id: 2, question: '?' }), { id: 2, question: '?' });
    } finally {
      globalThis.fetch = original;
    }
  });

  test('the entry for Node.js is ready when it is imported', async () => {
    await ready();
  });
});
