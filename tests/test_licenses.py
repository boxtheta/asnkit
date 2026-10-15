# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

import unittest


def read(path):
    with open(path, "rb") as fin:
        return fin.read()


class LicensesTest(unittest.TestCase):
    def test_package_copies_are_those_of_the_root(self):
        """Each package is built with a copy of its license."""

        for path, package in [
            ("LICENSE-RUNTIMES", "asnkitrt"),
            ("LICENSE-RUNTIMES", "asnkitrt-rs"),
            ("LICENSE-COMPILER", "asnkit-compiler"),
            ("LICENSE-COMPILER", "asnkitcc"),
            ("LICENSE-COMPILER", "asnkit-gen-python"),
            ("LICENSE-COMPILER", "asnkit-gen-rust"),
        ]:
            with self.subTest(path=path, package=package):
                self.assertEqual(
                    read(path), read("packages/{}/{}".format(package, path))
                )

    def test_license_files_hold_nothing_but_the_license(self):
        """So that tools can tell which license a file holds."""

        starts = {
            "LICENSE-COMPILER": b"                                 Apache License\n",
            "LICENSE-RUNTIMES": b"MIT License\n",
        }

        for path, start in starts.items():
            with self.subTest(path=path):
                self.assertTrue(read(path).startswith(start))

    def test_notice_is_with_the_code_of_asn1tools(self):
        """The MIT license of asn1tools stays with the codecs of the runtime
        for Python, which derive from it, and is built into its package.

        """

        notice = read("packages/asnkitrt/NOTICE").decode()

        self.assertIn("asn1tools", notice)
        self.assertIn("Copyright (c) 2017-2019 Erik Moqvist", notice)
        self.assertIn('"NOTICE"', read("packages/asnkitrt/pyproject.toml").decode())


if __name__ == "__main__":
    unittest.main()
