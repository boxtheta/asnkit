// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

// What the compiler makes in WebAssembly is loaded by the runtime in
// WebAssembly, which is to be built first, in packages/asnkitrt-wasm.

import { strict as assert } from 'node:assert';
import { readFileSync } from 'node:fs';
import { describe, test } from 'node:test';

import {
  compile,
  compileFile,
  compileWithSummary,
  parse,
  ready,
  schema,
  version,
} from '../js/node.js';
import { load } from '../../asnkitrt-wasm/js/node.js';

const FOO = new URL('../../../examples/compiled/foo.asn', import.meta.url);
const FOO_TEXT = readFileSync(FOO, 'utf8');
const QUESTION = { id: 1, question: 'Is 1+1=3?' };

// The CRC-32 of zlib, gzip and PNG, a bit at a time.
function crc32(bytes) {
  let crc = 0xffffffff;

  for (const byte of bytes) {
    crc ^= byte;

    for (let bit = 0; bit < 8; bit++) {
      crc = crc & 1 ? (crc >>> 1) ^ 0xedb88320 : crc >>> 1;
    }
  }

  return (crc ^ 0xffffffff) >>> 0;
}

function hex(bytes) {
  return Buffer.from(bytes).toString('hex');
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

  test('with what is likely to cause trouble', async () => {
    assert.deepEqual((await compileWithSummary(FOO_TEXT)).warnings, []);

    const { warnings } = await compileWithSummary(
      'Foo DEFINITIONS ::= BEGIN\nList ::= SEQUENCE { tail List OPTIONAL }\nEND\n',
    );

    assert.equal(warnings.length, 1);
    assert.equal(warnings[0].kind, 'RecursiveType');
    assert.match(warnings[0].message, /^Type 'List' in module 'Foo' refers to itself, at line 2/);
  });

  test('refuses a type defined by itself', async () => {
    await rejectsWith(
      compile('Foo DEFINITIONS ::= BEGIN A ::= A END'),
      'CompileError',
      /Type 'A' in module 'Foo' is defined by itself/,
    );
  });

  test('tells the version of the compiler it writes', async () => {
    const asni = Buffer.from(await compile(FOO_TEXT));

    assert.match(version(), /^\d+\.\d+\.\d+/);
    assert.ok(asni.includes(`asnkit-compiler ${version()} (Rust)`));
  });

  test('writes the CRC-32 of the file', async () => {
    const asni = await compile(FOO_TEXT);
    const view = new DataView(asni.buffer, asni.byteOffset, asni.byteLength);

    assert.deepEqual([...asni.subarray(0, 6)], [...Buffer.from('ASNI\x02\x00', 'latin1')]);
    assert.equal(view.getUint32(6), crc32(asni.subarray(10)));

    asni[asni.length - 1] ^= 1;
    await rejectsWith(load(asni, 'der'), 'LoadError', /CRC-32/);
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
      [{ nonce: 1 }, /'nonce' is no longer supported/],
      [{ boundTo: 'demo' }, /'boundTo' is no longer supported/],
      [{ signingKey: new Uint8Array(32) }, /'signingKey' is no longer supported/],
      [{ signingKeyId: new Uint8Array(16) }, /'signingKeyId' is no longer supported/],
      [{ encryptionKey: new Uint8Array(32) }, /'encryptionKey' is no longer supported/],
      [{ encryptionKeyId: new Uint8Array(16) }, /'encryptionKeyId' is no longer supported/],
    ]) {
      await assert.rejects(compile(FOO_TEXT, options), (error) => {
        assert.ok(error instanceof TypeError, String(error));
        assert.match(error.message, pattern);

        return true;
      });
    }
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

describe('schema', () => {
  const ANY = 'Foo DEFINITIONS ::= BEGIN\nWrapped ::= SEQUENCE { id INTEGER, body ANY }\n'
    + 'Plain ::= SEQUENCE { id INTEGER (0..9) }\nEND\n';

  test('is a JSON Schema of JER by default', async () => {
    const asni = await compile(FOO_TEXT);
    const made = await schema(asni, { types: 'Question' });
    const document = JSON.parse(made.text);

    assert.equal(made.types, 1);
    assert.deepEqual(made.skipped, []);
    assert.equal(document.$schema, 'https://json-schema.org/draft/2020-12/schema');
    assert.equal(document.$ref, '#/$defs/Question');
    assert.deepEqual(Object.keys(document.$defs), ['Question']);
    assert.deepEqual(document.$defs.Question.required, ['id', 'question']);
    assert.equal(document.$comment, 'schema generated by asnkit');
    assert.deepEqual(Object.keys(JSON.parse((await schema(asni)).text).$defs), ['Question', 'Answer']);
  });

  test('has the comment of the pragma schema_comment', async () => {
    const text = `--@ schema_comment = Greeter v1\n${FOO_TEXT}`;
    const asni = await compile(text);
    const made = await schema(asni, { specification: text });
    const dtd = await schema(asni, { format: 'dtd', specification: text });

    assert.equal(JSON.parse(made.text).$comment, 'Greeter v1');
    assert.match(dtd.text, /^<!-- Greeter v1 -->\n/);
    await assert.rejects(schema(asni, { specification: 1 }), TypeError);
  });

  test('is an XML Schema or a DTD of XER', async () => {
    const asni = await compile(FOO_TEXT);
    const xsd = await schema(asni, { format: 'xsd', types: ['Foo.Question', 'Answer'] });
    const dtd = await schema(asni, { format: 'dtd' });

    assert.equal(xsd.types, 2);
    assert.match(xsd.text, /^<\?xml version="1\.0" encoding="UTF-8"\?>/);
    assert.match(xsd.text, /<xs:element name="Question" type="Question"\/>/);
    assert.match(dtd.text, /<!ELEMENT Question \(id, question\)>/);
  });

  test('leaves out what it can not tell, if asked to', async () => {
    const asni = await compile(ANY);

    await rejectsWith(schema(asni), 'SchemaError', /Wrapped/);
    await rejectsWith(
      schema(asni, { types: 'Wrapped', skipUnsupported: true }),
      'SchemaError',
      /Wrapped/,
    );

    const made = await schema(asni, { skipUnsupported: true });

    assert.equal(made.types, 1);
    assert.deepEqual(made.skipped.map((skipped) => skipped.type), ['Wrapped']);
    assert.match(made.skipped[0].reason, /ANY/);
    assert.deepEqual(Object.keys(JSON.parse(made.text).$defs), ['Plain']);
  });

  test('checks its options and the types', async () => {
    const asni = await compile(FOO_TEXT);

    await rejectsWith(schema(asni, { types: 'Nope' }), 'SchemaError', /no type 'Nope'/);
    await rejectsWith(schema(new Uint8Array(8)), 'SchemaError', /the specification/);

    for (const [options, pattern] of [
      ['yes', /options to be an object/],
      [{ format: 'yaml' }, /unknown format 'yaml'/],
      [{ format: 1 }, /'format' to be a string/],
      [{ types: [1] }, /'types' to be a string or an array/],
      [{ skipUnsupported: 'yes' }, /'skipUnsupported' to be a boolean/],
    ]) {
      await assert.rejects(schema(asni, options), (error) => {
        assert.ok(error instanceof TypeError, String(error));
        assert.match(error.message, pattern);

        return true;
      });
    }
  });
});
