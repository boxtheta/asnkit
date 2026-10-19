Playground
==========

A web page that compiles an ASN.1 specification in the browser, with
the compiler as WebAssembly (``packages/asnkit-compiler-wasm``), and
loads what it makes with the runtime as WebAssembly
(``packages/asnkitrt-wasm``). A value is decoded with one codec and
encoded with all of them, BER, DER, OER, PER, UPER, JER, XER and
GSER, to compare their sizes. Clicking any encoding makes it the input.
GSER accepts both bare values and the value assignments shown in its row.
Changing the input codec also converts a valid input to that encoding.
If the input cannot be decoded or encoded with the selected codec, its
text is preserved so it can be edited in the selected format.
The compilation status shows separate timings for compiling the schema
and loading the resulting ``.asni`` into all eight codec instances.
Each codec row also shows encode and decode times in milliseconds,
measured once with the current constraint-checking setting. Decode is
measured on the compact encoded bytes; display formatting is excluded
from both timings. These are single-call measurements, not benchmarks.
They measure the WebAssembly runtime in the browser, not native runtime
performance. Native runtime execution is typically faster.
Very short calls may show zero at the browser's timer resolution.

Samples include sensor messages, the Foo protocol, a Sketch-Pad drawing
journal, an SNMPv1 get request, and an X.509 certificate. The Sketch-Pad
schema is the full ``tests/files/sketch.asn`` test protocol; its journal
combines the drawing and edit test values, including recursive groups,
mirrored shapes, defaults and extension alternatives. Text fields are
UTF-8 encoded octet strings. The SNMP schema covers request and response PDUs,
not traps. The X.509 schema contains the core certificate types from the
repository's ``examples/compiled/rfc5280.asn``; it leaves attribute values
and algorithm parameters as ``ANY``, and extension contents as octets.
Its DER input is the existing Ed25519 fixture
``packages/asnkitrt-rs/tests/fixtures/x509/key_ed25519.der``. It starts in
DER rather than JER because not every codec supports opaque ``ANY``
values; those codecs display their errors. Decoding a certificate does
not verify its signature or trust.

From any working directory, run the repository's ``webdemo.sh`` to build
both WebAssembly packages and start a loopback-only server. It prints the
playground URL before serving and stops with Ctrl-C. An optional port
defaults to 8000:

.. code-block:: text

   ./webdemo.sh
   ./webdemo.sh 8080

The launcher requires Python 3, npm, Cargo, the Rust
``wasm32-unknown-unknown`` target and ``wasm-bindgen``. See the package
READMEs for their setup instructions.

Build both packages, as their READMEs describe, and serve the
repository, as the page imports them from ``packages/``:

.. code-block:: text

   (cd packages/asnkitrt-wasm && npm run build)
   (cd packages/asnkit-compiler-wasm && npm run build)
   python3 -m http.server 8000

Then open http://localhost:8000/examples/playground/.

It is online at https://asnkit.boxtheta.org/playground/. The
documentation on GitHub Pages has it under ``playground/``, with
the packages copied next to it by ``build-site.sh``, which the workflow
``docs.yml`` runs.
