// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

// What the compiler makes in WebAssembly is loaded by the runtime in
// WebAssembly, which is to be built first, in packages/asnkitrt-wasm.

import { strict as assert } from 'node:assert';
import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { describe, test } from 'node:test';

import {
  compile,
  compileFile,
  compileWithSummary,
  parse,
  ready,
  version,
} from '../js/node.js';
import { load } from '../../asnkitrt-wasm/js/node.js';

const FOO = new URL('../../../examples/compiled/foo.asn', import.meta.url);
const CORPUS = new URL('../../../examples/corpus/', import.meta.url);
const FOO_TEXT = readFileSync(FOO, 'utf8');
const QUESTION = { id: 1, question: 'Is 1+1=3?' };

function hex(bytes) {
  return Buffer.from(bytes).toString('hex');
}

function keyFile(name) {
  return Uint8Array.from(Buffer.from(readFileSync(new URL(name, CORPUS), 'utf8').trim(), 'hex'));
}

function keyId(key) {
  return Uint8Array.from(createHash('sha256').update(key).digest().subarray(0, 16));
}

async function rejectsWith(promise, name, pattern) {
  await assert.rejects(promise, (error) => {
    assert.equal(error.name, name);
    assert.match(error.message, pattern);

    return true;
  });
}

describe('compile', () => {
  test('makes what the runtime loads', async () => {
    await ready();

    const asni = await compile(FOO_TEXT);

    assert.ok(asni instanceof Uint8Array);
    assert.equal(Buffer.from(asni.subarray(0, 4)).toString(), 'ASNI');

    for (const codec of ['ber', 'der', 'uper', 'jer']) {
      const spec = await load(asni, codec);

      assert.deepEqual(spec.decode('Question', spec.encode('Question', QUESTION)), QUESTION);
    }

    const ber = await load(asni, 'ber');
    assert.equal(hex(ber.encode('Question', QUESTION)), '300e0201011609497320312b313d333f');
  });

  test('of a file, and with the summary of its modules', async () => {
    assert.deepEqual(await compileFile(FOO.pathname), await compile(FOO_TEXT));

    const { asni, modules } = await compileWithSummary(FOO_TEXT);

    assert.deepEqual(asni, await compile(FOO_TEXT));
    assert.deepEqual(modules, [{ name: 'Foo', types: 2, objectClasses: 0 }]);
  });

  test('tells the version of the compiler it writes', async () => {
    const asni = Buffer.from(await compile(FOO_TEXT));

    assert.match(version(), /^\d+\.\d+\.\d+/);
    assert.ok(asni.includes(`asnkit-compiler ${version()} (Rust)`));
  });

  test('signs, encrypts, binds and gives a nonce', async () => {
    const signingKey = keyFile('signing.key');
    const encryptionKey = keyFile('encryption.key');
    const publicKey = keyFile('signing.pub');
    const options = { nonce: 7, boundTo: 'demo', signingKey, encryptionKey };
    const asni = await compile(FOO_TEXT, options);
    const spec = await load(asni, 'der', {
      nonce: 7,
      boundTo: 'demo',
      verificationKeys: [[keyId(publicKey), publicKey]],
      decryptionKeys: [[keyId(encryptionKey), encryptionKey]],
    });

    assert.deepEqual(spec.decode('Question', spec.encode('Question', QUESTION)), QUESTION);

    // The initialization vector is random, from the crypto of JavaScript.
    const again = await compile(FOO_TEXT, options);
    assert.notDeepEqual(again, asni);

    await rejectsWith(load(asni, 'der'), 'LoadError', /signed/);
  });

  test('takes the identifier of a key', async () => {
    const encryptionKey = keyFile('encryption.key');
    const encryptionKeyId = new Uint8Array(16).fill(9);
    const asni = await compile(FOO_TEXT, { encryptionKey, encryptionKeyId });

    await load(asni, 'ber', { decryptionKeys: [[encryptionKeyId, encryptionKey]] });
    await rejectsWith(
      load(asni, 'ber', { decryptionKeys: [[keyId(encryptionKey), encryptionKey]] }),
      'LoadError',
      /not among those given/,
    );
  });

  test('throws errors that tell where', async () => {
    await rejectsWith(compile('A DEFINITIONS ::= BEGIN B ::= END'), 'ParseError', /line 1/);
    await rejectsWith(
      compile('A DEFINITIONS ::= BEGIN B ::= NOPE END'),
      'CompileError',
      /Type 'NOPE' not found in module 'A'/,
    );
  });

  test('checks its options', async () => {
    for (const [options, pattern] of [
      ['yes', /options to be an object/],
      [{ nonce: -1 }, /'nonce'/],
      [{ nonce: 1.5 }, /'nonce'/],
      [{ boundTo: 3 }, /'boundTo' to be a string/],
      [{ signingKey: new Uint8Array(31) }, /'signingKey' to be a Uint8Array of 32 bytes/],
      [{ encryptionKeyId: new Uint8Array(16) }, /'encryptionKey' with 'encryptionKeyId'/],
    ]) {
      await assert.rejects(compile(FOO_TEXT, options), (error) => {
        assert.ok(error instanceof TypeError, String(error));
        assert.match(error.message, pattern);

        return true;
      });
    }

    // What the format does not allow, the compiler tells.
    await rejectsWith(compile(FOO_TEXT, { nonce: 2 ** 24 }), 'Error', /nonce/);
  });
});

describe('parse', () => {
  test('makes the syntax tree', async () => {
    const asnst = Buffer.from(await parse(FOO_TEXT));

    assert.ok(asnst.includes('asnst'));
    assert.ok(asnst.includes('Question'));
    await rejectsWith(parse('A DEFINITIONS ::= BEGIN B ::= END'), 'ParseError', /line 1/);
  });
});
