# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""How long the runtime for Python takes to load a specification, to
encode and decode a small message and a certificate with each codec, and
to check the constraints of the certificate, to see when that changes.
It is measured as the benchmarks of the runtime for Rust are, see
packages/asnkitrt-rs/benches/harness/mod.rs:

  uv run python tests/benchmarks.py [FILTER...]

If ASNKIT_BENCH_OUT is a directory, the results are also written there
as JSON, to python.json.

"""

import json
import os
import statistics
import sys
import time

import asnkitrt

FOO = os.path.join("examples", "compiled", "foo.asni")
RFC5280 = os.path.join("examples", "compiled", "rfc5280.asni")
# A certificate with all the extensions OpenSSL makes.
CERTIFICATE = os.path.join(
    "packages", "asnkitrt-rs", "tests", "fixtures", "x509", "extensions_all.der"
)

# The codecs, and if they do the certificate: JER and XER need the types
# of the values of its ANYs.
CODECS = [
    ("ber", True),
    ("der", True),
    ("oer", True),
    ("per", True),
    ("uper", True),
    ("jer", False),
    ("xer", False),
]

WARM_UP = 0.3
SAMPLE = 0.01
SAMPLES = 50


class Benches:
    def __init__(self, group, filters):
        self.group = group
        self.filters = filters
        self.results = []

    def run(self, name, f):
        name = f"{self.group}/{name}"

        if self.filters and not any(filter_ in name for filter_ in self.filters):
            return

        iterations = 0
        start = time.perf_counter()

        while time.perf_counter() - start < WARM_UP:
            f()
            iterations += 1

        per_iteration = (time.perf_counter() - start) / iterations
        batch = max(int(SAMPLE / per_iteration), 1)
        samples = []

        for _ in range(SAMPLES):
            start = time.perf_counter_ns()

            for _ in range(batch):
                f()

            samples.append((time.perf_counter_ns() - start) / batch)

        median = statistics.median(samples)
        deviation = statistics.median(abs(sample - median) for sample in samples)

        print(f"{name:<40} {median:>14.1f} ns  ± {deviation:.1f}", file=sys.stderr)
        self.results.append(
            {
                "name": name,
                "unit": "ns",
                "value": round(median, 1),
                "range": f"± {deviation:.1f}",
            }
        )

    def write(self):
        directory = os.environ.get("ASNKIT_BENCH_OUT")

        if directory:
            with open(os.path.join(directory, f"{self.group}.json"), "w") as fout:
                json.dump(self.results, fout, indent=2)
                fout.write("\n")


def run_codec(benches, codec, foo_data, rfc5280_data, question, certificate):
    foo = asnkitrt.load_asni(foo_data, codec)
    encoded = foo.encode("Question", question)

    benches.run(f"{codec}/encode/Question", lambda: foo.encode("Question", question))
    benches.run(f"{codec}/decode/Question", lambda: foo.decode("Question", encoded))

    if certificate is None:
        return

    rfc5280 = asnkitrt.load_asni(rfc5280_data, codec)
    encoded = rfc5280.encode("Certificate", certificate)

    benches.run(
        f"{codec}/encode/Certificate",
        lambda: rfc5280.encode("Certificate", certificate),
    )
    benches.run(
        f"{codec}/decode/Certificate",
        lambda: rfc5280.decode("Certificate", encoded),
    )


def main():
    benches = Benches("python", sys.argv[1:])

    with open(FOO, "rb") as fin:
        foo_data = fin.read()

    with open(RFC5280, "rb") as fin:
        rfc5280_data = fin.read()

    with open(CERTIFICATE, "rb") as fin:
        certificate_data = fin.read()

    benches.run("load/foo", lambda: asnkitrt.load_asni(foo_data, "ber"))
    benches.run("load/rfc5280", lambda: asnkitrt.load_asni(rfc5280_data, "der"))

    question = {"id": 1, "question": "Is 1+1=3?"}
    rfc5280 = asnkitrt.load_asni(rfc5280_data, "der")
    certificate = rfc5280.decode("Certificate", certificate_data)

    for codec, does_certificate in CODECS:
        run_codec(
            benches,
            codec,
            foo_data,
            rfc5280_data,
            question,
            certificate if does_certificate else None,
        )

    certificate_type = rfc5280.types["Certificate"]

    benches.run(
        "check_constraints/Certificate",
        lambda: certificate_type.check_constraints(certificate),
    )
    benches.write()


if __name__ == "__main__":
    main()
