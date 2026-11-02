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

The supported codecs are BER, DER, GSER, JER, OER, PER, UPER and XER.
Each is imported only when used. A Rust runtime with the same behavior,
and a WebAssembly one for browsers and Node.js, live in the same
`repository`_, which also documents the formats and the known
limitations.

The runtime reads version 2 ``.asni`` files. Recompile version 1 files
from their ASN.1 sources. A file may be bound to a string; loads accept
only unbound files by default. Give the expected binding explicitly:

.. code-block:: python

   spec = asnkitrt.load_asni_file('bound.asni', 'uper', bound_to='corpus')

``accept_any_binding=True`` explicitly accepts any binding. On ``load``,
``load_asni`` and ``load_asni_file``, options after ``numeric_enums`` are
keyword-only. Signing, encryption, nonces and their install extras have
been removed. Binding is not authentication; provide any required
cryptographic protection outside the runtime.

.. _asnkitcc: https://crates.io/crates/asnkitcc

.. _repository: https://github.com/boxtheta/asnkit

License
-------

MIT. The compiler, ``asnkitcc``, is Apache-2.0, and an application that
only uses ``asnkitrt`` does not depend on it.
