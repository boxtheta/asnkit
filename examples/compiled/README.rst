Compiled specifications and syntax trees
========================================

The files in this folder are made from the ASN.1 files next to them.
``rfc5280`` is a real specification, the X.509 certificate profile of
RFC 5280.

An ``.asnc`` file is a compiled specification: all that is needed to
encode and decode its types, with nothing left to look up or work
out. Loading one needs no parser.

.. code-block:: text

   $ asnkitcc compile foo.asn

.. code-block:: python

   >>> import asnkitrt
   >>> foo = asnkitrt.load_asnc_file('foo.asnc', 'uper')
   >>> foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
   b'\x01\x01\t\x93\xcd\x03\x15l^\xb3~'

An ``.asnst`` file is the syntax tree of a specification: everything
found in it, as it is written. It is what plugins are given, and it
comes with the text of the ASN.1 files it was made from, and with
where in them each module, assignment, member and field of an object
class is found. An ``.asnc`` file tells no such thing, and never has
the text.

.. code-block:: text

   $ asnkitcc parse foo.asn

.. code-block:: python

   >>> tree = asnkitrt.syntax.from_asnst_file('foo.asnst')
   >>> tree.modules[0].types[0].location  # File, line and column.
   (0, 5, 5)

Both tell what they are, the version of their layout, and where they
come from:

.. code-block:: python

   >>> with open('foo.asnc', 'rb') as fin:
   ...     asnkitrt.syntax.read_header(fin.read())
   Header(kind='asnc', version=(1, 0), metadata=Metadata(generator='asnkit-compiler 1.0.0 (Rust)', sources=(Source(name='foo.asn', sha256='c3904ec2...', text=None),), bound_to=None, other={}), nonce=None, signed_by=None, encrypted_with=None)

Look at either as JSON:

.. code-block:: text

   $ python -m asnkitrt.syntax foo.asnc
   {
       "kind": "CompiledSpecification",
       "modules": [
           {
               "kind": "CompiledModule",
               "name": "Foo",
   ...
   $ python -m asnkitrt.syntax --header foo.asnc
