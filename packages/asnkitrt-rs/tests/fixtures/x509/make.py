# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Makes the certificates in this folder with OpenSSL, as the tests of
the runtime for Python do: the same keys, names, extensions and serial
numbers. Run it from the root of the repository:

    uv run python packages/asnkitrt-rs/tests/fixtures/x509/make.py

The keys are not kept. The certificates are different every time, as
their keys and their validity are.

"""

import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, *[".."] * 5))
os.chdir(ROOT)
sys.path.insert(0, ROOT)

from tests.test_openssl_x509 import OpenSslX509Test  # noqa: E402

OpenSslX509Test.setUpClass()

try:
    for name in os.listdir(HERE):
        if name.endswith(".der"):
            os.remove(os.path.join(HERE, name))

    count = 0

    for name, der in sorted(OpenSslX509Test.certificates.items()):
        if isinstance(der, bytes):
            with open(os.path.join(HERE, name + ".der"), "wb") as fout:
                fout.write(der)

            count += 1

    print("{} certificates".format(count))
finally:
    OpenSslX509Test.tearDownClass()
