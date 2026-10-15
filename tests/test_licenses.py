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
            ("NOTICE", "asnkitrt"),
            ("NOTICE", "asnkitrt-rs"),
            ("NOTICE", "asnkitrt-wasm"),
            ("NOTICE", "asnkit-compiler"),
            ("NOTICE", "asnkitcc"),
            ("NOTICE", "asnkit-gen-python"),
            ("NOTICE", "asnkit-gen-rust"),
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
            "LICENSE-DOCUMENTATION": b"Creative Commons Attribution 4.0 International",
        }

        for path, start in starts.items():
            with self.subTest(path=path):
                self.assertTrue(read(path).startswith(start))

    def test_copying_says_it_is_not_legal_advice(self):
        copying = read("COPYING").decode()

        self.assertIn("NOT LEGAL ADVICE", copying)

        for name in [
            "LICENSE-COMPILER",
            "LICENSE-RUNTIMES",
            "LICENSE-DOCUMENTATION",
        ]:
            self.assertIn(name, copying)


if __name__ == "__main__":
    unittest.main()
