import { load } from '../js/web.js';

const CODECS = ['ber', 'der', 'oer', 'per', 'uper', 'jer', 'xer'];
const WARM_UP_MS = 300;
const SAMPLE_MS = 10;
const SAMPLES = 50;

async function bytes(path) {
  const response = await fetch(new URL(path, import.meta.url));

  if (!response.ok) {
    throw new Error(`Could not load ${response.url}: HTTP ${response.status}`);
  }

  return new Uint8Array(await response.arrayBuffer());
}

function median(values) {
  const sorted = [...values].sort((first, second) => first - second);
  const middle = Math.floor(sorted.length / 2);

  return sorted.length % 2 === 0
    ? (sorted[middle - 1] + sorted[middle]) / 2
    : sorted[middle];
}

function measure(name, action) {
  let iterations = 0;
  const started = performance.now();

  while (performance.now() - started < WARM_UP_MS) {
    action();
    iterations++;
  }

  const batch = Math.max(1, Math.floor(SAMPLE_MS * iterations / (performance.now() - started)));
  const samples = [];

  for (let sample = 0; sample < SAMPLES; sample++) {
    const sampleStarted = performance.now();

    for (let iteration = 0; iteration < batch; iteration++) {
      action();
    }

    samples.push((performance.now() - sampleStarted) * 1e6 / batch);
  }

  const value = median(samples);
  const deviation = median(samples.map((sample) => Math.abs(sample - value)));

  return { name, unit: 'ns', value, deviation, batch, samples: SAMPLES };
}

export async function runBenchmarks(filters = []) {
  const foo = await bytes('../../../examples/compiled/foo.asni');
  const rfc5280 = await bytes('../../../examples/compiled/rfc5280.asni');
  const certificateBytes = await bytes('../../asnkitrt-rs/tests/fixtures/x509/extensions_all.der');
  const certificateSpec = await load(rfc5280, 'der');
  let certificate;

  try {
    certificate = certificateSpec.decode('Certificate', certificateBytes);
  } finally {
    certificateSpec.free();
  }

  const workloads = [
    { type: 'Question', asni: foo, value: { id: 1, question: 'Is 1+1=3?' } },
    { type: 'Certificate', asni: rfc5280, value: certificate },
  ];
  const results = [];

  for (const codec of CODECS) {
    for (const workload of workloads) {
      if (workload.type === 'Certificate' && ['jer', 'xer'].includes(codec)) {
        continue;
      }

      const spec = await load(workload.asni, codec);

      try {
        const encoded = spec.encode(workload.type, workload.value);
        const actions = {
          encode: () => spec.encode(workload.type, workload.value),
          decode: () => spec.decode(workload.type, encoded),
        };

        for (const [operation, action] of Object.entries(actions)) {
          const name = `runtime/${codec}/${operation}/${workload.type}`;

          if (filters.length === 0 || filters.some((filter) => name.includes(filter))) {
            results.push(measure(name, action));
          }
        }
      } finally {
        spec.free();
      }
    }
  }

  return results;
}
