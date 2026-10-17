Playground
==========

A web page that compiles an ASN.1 specification in the browser, with
the compiler as WebAssembly (``packages/asnkit-compiler-wasm``), and
loads what it makes with the runtime as WebAssembly
(``packages/asnkitrt-wasm``). A value is decoded with one codec and
encoded with all of them, BER, DER, OER, PER, UPER, JER and XER, to
compare their sizes. Clicking an encoding makes it the input.

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
