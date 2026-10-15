# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

import unittest
from .utils import BaseTest
from tests import compiled as fixtures


class TwoModulesTest(BaseTest):
    maxDiff = None

    def test_uper(self):
        files = ["tests/files/module-1.asn", "tests/files/module-2.asn"]

        spec = fixtures.compile_files(files, "uper")

        # Message 1.
        decoded = {"priority": "high", "src": 1, "dst": 2, "num": 0, "length": 256}

        encoded = b"\x14\x04\x00"

        self.assert_encode_decode(spec, "ASequence", decoded, encoded)

        # Message 2.
        decoded = {"isvalide": True, "name": "toto", "identity": 5}

        encoded = b"\x82\x74\xdf\xd3\x7d"

        self.assert_encode_decode(spec, "SeqMod2", decoded, encoded)


if __name__ == "__main__":
    unittest.main()
