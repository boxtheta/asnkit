// Copyright (c) 2025 BoxTheta
//
// SPDX-License-Identifier: Apache-2.0

// The playground: the text of the specification is compiled by the
// compiler as WebAssembly, and what it makes is loaded by the runtime as
// WebAssembly, once for each codec. A value is decoded with one codec
// and encoded with all of them.

// By the names of the packages on npm, which index.html maps to where
// they are.
import * as compiler from 'asnkit-compiler';
import * as runtime from 'asnkitrt';

const CODECS = [
  ['ber', 'BER', 'Basic Encoding Rules'],
  ['der', 'DER', 'Distinguished Encoding Rules'],
  ['oer', 'OER', 'Octet Encoding Rules'],
  ['per', 'PER', 'Packed Encoding Rules, aligned'],
  ['uper', 'UPER', 'Packed Encoding Rules, unaligned'],
  ['jer', 'JER', 'JSON Encoding Rules'],
  ['xer', 'XER', 'XML Encoding Rules'],
];
const TEXT_CODECS = new Set(['jer', 'xer']);

const EXAMPLES = [
  {
    name: 'Sensor messages',
    type: 'Message',
    specification: `Demo DEFINITIONS AUTOMATIC TAGS ::= BEGIN

    Message ::= SEQUENCE {
        id        INTEGER (0..65535),
        sender    UTF8String (SIZE (1..32)),
        priority  Priority DEFAULT normal,
        urgent    BOOLEAN,
        payload   Payload,
        tags      SEQUENCE (SIZE (0..8)) OF IA5String OPTIONAL
    }

    Priority ::= ENUMERATED { low, normal, high }

    Payload ::= CHOICE {
        text     UTF8String,
        reading  Reading,
        raw      OCTET STRING
    }

    Reading ::= SEQUENCE {
        sensor   IA5String (SIZE (1..16)),
        celsius  INTEGER (-50..150),
        flags    BIT STRING (SIZE (4))
    }

END
`,
    value: {
      id: 42,
      sender: 'Ada',
      priority: 'high',
      urgent: true,
      payload: ['reading', { sensor: 'lab-1', celsius: 21, flags: [Uint8Array.of(0xa0), 4] }],
      tags: ['lab', 'demo'],
    },
  },
  {
    name: 'Foo protocol',
    type: 'Question',
    specification: `-- A simple protocol taken from Wikipedia.

Foo DEFINITIONS ::= BEGIN

    Question ::= SEQUENCE {
        id        INTEGER,
        question  IA5String
    }

    Answer ::= SEQUENCE {
        id        INTEGER,
        answer    BOOLEAN
    }

END
`,
    value: { id: 1, question: 'Is 1+1=3?' },
  },
];

const $ = (id) => document.getElementById(id);

const state = {
  // The specification loaded for each codec, and the .asni it is of.
  specs: null,
  asni: null,
  name: 'specification',
  // The encodings of the last value, by codec, to use as the input.
  outputs: new Map(),
};

function hex(bytes) {
  return Array.from(bytes, (byte) => byte.toString(16).padStart(2, '0')).join('');
}

/** Hexadecimal in groups of bytes, to be read. */
function spacedHex(bytes) {
  return hex(bytes).replace(/(..)(?!$)/g, '$1 ');
}

function bytesOfHex(text) {
  const digits = text.replace(/\s+/g, '').replace(/^0x/i, '');

  if (digits.length % 2 !== 0 || /[^0-9a-f]/i.test(digits)) {
    throw new TypeError('Expected the input as hexadecimal digits.');
  }

  return Uint8Array.from(digits.match(/../g) ?? [], (pair) => parseInt(pair, 16));
}

function showStatus(element, message, kind) {
  element.textContent = message;
  element.className = `status ${kind ?? ''}`;
}

function describe(error) {
  const codec = $('input-codec').value;

  // The text codecs do not tell where an error is.
  if (error?.name === 'DecodeError' && error.offset !== undefined && !TEXT_CODECS.has(codec)) {
    const unit = ['per', 'uper'].includes(codec) ? 'bit' : 'byte';

    return `${error.name}: ${error.message} (at ${unit} ${error.offset})`;
  }

  return error?.name ? `${error.name}: ${error.message}` : String(error);
}

/** The names of the types of the text, in the order they are defined. */
function typeNames(specification) {
  const names = [];
  const text = specification.replace(/--.*?(--|$)/gm, '').replace(/\/\*[\s\S]*?\*\//g, '');

  for (const match of text.matchAll(/^\s*([A-Z][\w-]*)\s*(?:\{[^}]*\})?\s*::=/gm)) {
    if (!names.includes(match[1])) {
      names.push(match[1]);
    }
  }

  return names;
}

function freeSpecs() {
  for (const spec of Object.values(state.specs ?? {})) {
    spec.free();
  }

  state.specs = null;
}

async function compileSpecification() {
  const text = $('spec').value;
  const status = $('compile-status');

  try {
    const started = performance.now();
    const { asni, modules } = await compiler.compileWithSummary(text);
    const compiled = performance.now() - started;
    const specs = {};

    for (const [codec] of CODECS) {
      specs[codec] = await runtime.load(asni, codec);
    }

    freeSpecs();
    state.specs = specs;
    state.asni = asni;
    state.name = modules[0]?.name ?? 'specification';

    showStatus(
      status,
      `Compiled in ${compiled.toFixed(1)} ms to ${asni.length.toLocaleString()} bytes of .asni, loaded for ${CODECS.length} codecs.`,
      'ok',
    );

    $('modules').replaceChildren(
      ...modules.map((module) => {
        const item = document.createElement('li');
        const name = document.createElement('b');

        name.textContent = module.name;
        item.append(name, ` · ${module.types} types`);

        if (module.objectClasses) {
          item.append(` · ${module.objectClasses} classes`);
        }

        return item;
      }),
    );
    $('compile-meta').hidden = false;

    const select = $('type');
    const previous = select.value;
    const names = typeNames(text);

    select.replaceChildren(...names.map((name) => new Option(name, name)));

    if (names.includes(previous)) {
      select.value = previous;
    }

    return true;
  } catch (error) {
    freeSpecs();
    state.asni = null;
    $('compile-meta').hidden = true;
    showStatus(status, describe(error), 'error');
    renderOutputs(null);

    return false;
  }
}

function inputData() {
  const codec = $('input-codec').value;
  const text = $('input').value;

  return TEXT_CODECS.has(codec) ? text.trim() : bytesOfHex(text);
}

function decodeInput() {
  const status = $('decode-status');

  if (!state.specs) {
    showStatus(status, 'Compile the specification first.', 'error');
    renderOutputs(null);

    return;
  }

  const codec = $('input-codec').value;
  const type = $('type').value;
  const options = { checkConstraints: $('check-constraints').checked };

  try {
    const value = state.specs[codec].decode(type, inputData(), options);

    showStatus(status, `Decoded a ${type} from ${codec.toUpperCase()}.`, 'ok');
    renderOutputs(value, type, options);
  } catch (error) {
    showStatus(status, describe(error), 'error');
    renderOutputs(null);
  }
}

function renderOutputs(value, type, options) {
  const body = $('outputs').tBodies[0];

  state.outputs.clear();

  if (value === null) {
    body.replaceChildren();

    return;
  }

  const results = CODECS.map(([codec, label, title]) => {
    try {
      // The size is that of the encoding as it is sent, and the text
      // codecs are shown indented, to be read.
      const encoded = state.specs[codec].encode(type, value, options);
      const shown = TEXT_CODECS.has(codec)
        ? state.specs[codec].encode(type, value, { ...options, indent: 2 })
        : encoded;

      state.outputs.set(codec, shown);

      return { codec, label, title, encoded, shown };
    } catch (error) {
      return { codec, label, title, error };
    }
  });
  const largest = Math.max(1, ...results.map((result) => result.encoded?.length ?? 0));
  const current = $('input-codec').value;

  body.replaceChildren(
    ...results.map(({ codec, label, title, encoded, shown, error }) => {
      const row = document.createElement('tr');
      const name = document.createElement('td');
      const size = document.createElement('td');
      const barCell = document.createElement('td');
      const out = document.createElement('td');

      row.dataset.codec = codec;
      row.classList.toggle('current', codec === current);
      name.className = 'codec';
      name.textContent = label;
      name.append(Object.assign(document.createElement('small'), { textContent: title }));
      size.className = 'num';
      barCell.className = 'bar-col';
      out.className = 'out';

      if (error) {
        size.textContent = '–';
        out.append(Object.assign(document.createElement('span'), {
          className: 'err',
          textContent: describe(error),
        }));
      } else {
        size.textContent = encoded.length.toLocaleString();

        const bar = document.createElement('div');
        bar.className = 'bar';
        bar.style.width = `${(100 * encoded.length) / largest}%`;
        barCell.append(bar);
        out.append(Object.assign(document.createElement('span'), {
          className: 'clamp',
          textContent: TEXT_CODECS.has(codec)
            ? new TextDecoder().decode(shown)
            : spacedHex(encoded),
        }));
      }

      row.append(name, size, barCell, out);
      row.title = error ? '' : `Use the ${label} encoding as the input`;
      size.title = TEXT_CODECS.has(codec) && !error ? 'Without indentation' : '';

      return row;
    }),
  );
}

/** Makes the encoding of a codec the input. */
function useAsInput(codec) {
  const encoded = state.outputs.get(codec);

  if (!encoded) {
    return;
  }

  $('input-codec').value = codec;
  $('input').value = TEXT_CODECS.has(codec)
    ? new TextDecoder().decode(encoded)
    : spacedHex(encoded);
  updateHint();
  decodeInput();
}

function updateHint() {
  const codec = $('input-codec').value;

  $('input-hint').textContent = TEXT_CODECS.has(codec)
    ? `${codec.toUpperCase()} text`
    : 'Hexadecimal bytes; spaces are ignored';
}

async function loadExample(example) {
  $('spec').value = example.specification;

  if (await compileSpecification()) {
    $('type').value = example.type;
    $('input-codec').value = 'jer';
    $('input').value = new TextDecoder().decode(
      state.specs.jer.encode(example.type, example.value, { indent: 2 }),
    );
    updateHint();
    decodeInput();
  }
}

function debounce(action, delay) {
  let timer;

  return () => {
    clearTimeout(timer);
    timer = setTimeout(action, delay);
  };
}

async function main() {
  for (const [codec, label] of CODECS) {
    $('input-codec').append(new Option(label, codec));
  }

  for (const [index, example] of EXAMPLES.entries()) {
    $('example').append(new Option(example.name, index));
  }

  await Promise.all([compiler.ready(), runtime.ready()]);
  $('versions').textContent = `compiler ${compiler.version()} · WebAssembly`;

  $('compile').addEventListener('click', async () => {
    if (await compileSpecification()) {
      decodeInput();
    }
  });
  $('spec').addEventListener('keydown', (event) => {
    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) {
      event.preventDefault();
      $('compile').click();
    }
  });
  $('example').addEventListener('change', () => loadExample(EXAMPLES[$('example').value]));
  $('type').addEventListener('change', decodeInput);
  $('input-codec').addEventListener('change', () => {
    updateHint();
    decodeInput();
  });
  $('check-constraints').addEventListener('change', decodeInput);
  $('input').addEventListener('input', debounce(decodeInput, 250));
  $('outputs').addEventListener('click', (event) => {
    const row = event.target.closest('tr[data-codec]');

    if (row) {
      useAsInput(row.dataset.codec);
    }
  });
  $('download').addEventListener('click', () => {
    const link = document.createElement('a');

    link.href = URL.createObjectURL(new Blob([state.asni], { type: 'application/octet-stream' }));
    link.download = `${state.name}.asni`;
    link.click();
    URL.revokeObjectURL(link.href);
  });

  await loadExample(EXAMPLES[0]);
}

main().catch((error) => {
  $('versions').textContent = 'Could not load the WebAssembly.';
  showStatus($('compile-status'), `${describe(error)}\n\nBuild both packages first: see README.rst.`, 'error');
});
