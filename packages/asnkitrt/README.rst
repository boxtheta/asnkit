asnkitrt
========

Encodes and decodes ASN.1 using compiled specifications: ``.asni``
files produced by the `asnkitcc`_ compiler. It never parses ASN.1 and
has no dependencies.

.. code-block:: python

   >>> import asnkitrt
   >>> foo = asnkitrt.load_asni_file('foo.asni', 'uper')
   >>> encoded = foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
   >>> foo.decode('Question', encoded)
   {'id': 1, 'question': 'Is 1+1=3?'}

The supported codecs are BER, DER, JER, OER, PER, UPER and XER, and
GSER for encoding only. Each is imported only when used. A Rust runtime
with the same behavior, and WebAssembly and C runtimes built from it,
live in the same
`repository`_, which also documents the formats and the known
limitations.

The runtime reads version 2 ``.asni`` files. Recompile version 1 files
from their ASN.1 sources. A file whose CRC-32 does not match is
corrupt, and is not loaded. On ``load``, ``load_asni`` and
``load_asni_file``, options after ``numeric_enums`` are keyword-only.
Signing, encryption, nonces, binding and their options have been
removed. The CRC-32 is not authentication; provide any required
cryptographic protection outside the runtime.

.. _asnkitcc: https://crates.io/crates/asnkitcc

.. _repository: https://github.com/boxtheta/asnkit

License
-------

MIT. The compiler, ``asnkitcc``, is Apache-2.0, and an application that
only uses ``asnkitrt`` does not depend on it.
