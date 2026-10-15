Dump an X.509 certificate as JSON
=================================

Reads a certificate in PEM or DER, from a file or from standard input,
and prints it as JSON. It decodes with the precompiled RFC 5280
specification in ``../compiled/rfc5280.asnc``, so no ASN.1 parser is
needed.

.. code-block:: text

   $ python main.py certificate.pem
   $ openssl x509 -in certificate.pem -outform der | python main.py

Octet strings, which includes the values of names and extensions, are
written as hex, bit strings as a string of ``0`` and ``1``, and times
as ISO 8601.
