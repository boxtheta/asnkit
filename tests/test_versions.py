# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""All the packages are released together, with one version, which the
tag of a release (v1.2.3) names.

"""

import json
import os
import re
import tomllib
import unittest

PACKAGES = os.path.join(os.path.dirname(__file__), "..", "packages")


def read_toml(*path):
    with open(os.path.join(PACKAGES, *path), "rb") as fin:
        return tomllib.load(fin)


def versions():
    """The version of each package, by the file that says it."""

    found = {}

    for package in ["asnkitrt", "asnkit-gen-python"]:
        found[package + "/pyproject.toml"] = read_toml(package, "pyproject.toml")[
            "project"
        ]["version"]

    for package in [
        "asnkitrt-rs",
        "asnkitrt-c",
        "asnkitrt-wasm",
        "asnkit-compiler",
        "asnkit-compiler-wasm",
        "asnkitcc",
        "asnkit-gen-rust",
        "asnkit-gen-rust-codec",
    ]:
        found[package + "/Cargo.toml"] = read_toml(package, "Cargo.toml")["package"][
            "version"
        ]

    for package in ["asnkitrt-wasm", "asnkit-compiler-wasm"]:
        with open(os.path.join(PACKAGES, package, "package.json")) as fin:
            found[package + "/package.json"] = json.load(fin)["version"]

    with open(
        os.path.join(
            PACKAGES, "asnkit-gen-python", "src", "asnkit_gen_python", "version.py"
        )
    ) as fin:
        found["asnkit_gen_python/version.py"] = re.search(
            r'__version__ = "(.*)"', fin.read()
        ).group(1)

    with open(
        os.path.join(PACKAGES, "asnkitrt", "src", "asnkitrt", "version.py")
    ) as fin:
        found["asnkitrt/version.py"] = re.search(
            r'__version__ = "(.*)"', fin.read()
        ).group(1)

    return found


def release_version(version):
    """Compare Python's PEP 440 rc spelling with Cargo/npm SemVer."""
    return re.sub(r"(?<=\d)rc(\d+)$", r"-rc\1", version)


class VersionsTest(unittest.TestCase):
    def test_release_candidate_version_spellings(self):
        self.assertEqual(release_version("2.0.0rc1"), "2.0.0-rc1")
        self.assertEqual(release_version("2.0.0-rc1"), "2.0.0-rc1")
        self.assertEqual(release_version("2.0.0"), "2.0.0")

    def test_all_packages_have_one_version(self):
        found = versions()

        self.assertEqual(len({release_version(v) for v in found.values()}), 1, found)

    def test_dependencies_between_packages_are_that_version(self):
        version = read_toml("asnkitrt", "pyproject.toml")["project"]["version"]

        self.assertIn(
            "asnkitrt=={}".format(version),
            read_toml("asnkit-gen-python", "pyproject.toml")["project"]["dependencies"],
        )

        for package, dependency in [
            ("asnkitcc", "asnkit-compiler"),
            ("asnkitcc", "asnkitrt"),
            ("asnkit-gen-rust", "asnkitrt"),
            ("asnkit-gen-rust-codec", "asnkitrt"),
            ("asnkit-gen-rust-codec", "asnkit-gen-rust"),
        ]:
            with self.subTest(package=package):
                self.assertEqual(
                    read_toml(package, "Cargo.toml")["dependencies"][dependency][
                        "version"
                    ],
                    release_version(version),
                )


if __name__ == "__main__":
    unittest.main()
