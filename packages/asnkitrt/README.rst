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

An ``.asni`` file can carry a nonce, be bound to a string, and be signed
and encrypted. Such a file only loads when the caller says what to
expect. Signed and encrypted files need the `cryptography`_ package,
which is an optional extra:

.. code-block:: text

   pip install asnkitrt[signing]
   pip install asnkitrt[encryption]

.. _asnkitcc: https://crates.io/crates/asnkitcc

.. _repository: https://github.com/boxtheta/asnkit

.. _cryptography: https://cryptography.io

License
-------

MIT. The compiler, ``asnkitcc``, is Apache-2.0, and an application that
only uses ``asnkitrt`` does not depend on it.
