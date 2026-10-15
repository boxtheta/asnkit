asnkit-gen-python
=================

A plugin for `asnkitcc`_ that generates Python code from ASN.1. The
generated module imports `asnkitrt`_ and, by default, embeds the compiled
specification as bytes. It has a data class for each ``SEQUENCE`` and
``SET``, an enumeration for each ``ENUMERATED`` and a class for each
alternative of a ``CHOICE``, all with methods to encode and decode.

.. code-block:: text

   $ pip install asnkit-gen-python
   $ asnkitcc generate --plugin python -o out foo.asn    # Writes out/foo.py.

.. code-block:: python

   >>> from foo import Question
   >>> encoded = Question(id=1, question='Is 1+1=3?').encode('uper')
   >>> Question.decode(encoded, 'uper')
   Question(id=1, question='Is 1+1=3?')

Options, given with ``--plugin-opt KEY=VALUE``:

``name``
   The name of the module, without ``.py``. Default: the name of the
   first ASN.1 file.

``codec``
   The codec used when none is given. Default: ``ber``.

``embed``
   ``false`` to leave the compiled specification out of the module, as
   for one sent signed or encrypted. The module then has
   ``use_asni(data, ...)``, which takes the contents of the ``.asni``
   file with the options of ``asnkitrt.load_asni()`` (``nonce``,
   ``bound_to``, ``verification_keys``, ``decryption_keys`` and those
   to accept any), checks it once and uses it for every codec. Until it
   is called, encoding and decoding raise ``RuntimeError``. The file has
   to be compiled from the same specification. Default: ``true``.

Names are converted to Python style: ``fooBar`` becomes ``foo_bar`` and
a type ``Foo-Bar`` becomes ``FooBar``. If two names end up the same in
Python, generation fails with an error.

.. _asnkitcc: https://crates.io/crates/asnkitcc

.. _asnkitrt: https://pypi.org/project/asnkitrt/

License
-------

Apache License 2.0 (``LICENSE-COMPILER``). The generated code, with the
helper code copied into it, is yours to use under terms of your choice,
as the header of each file it writes says. The generated code depends on
``asnkitrt``,
which is MIT licensed.
