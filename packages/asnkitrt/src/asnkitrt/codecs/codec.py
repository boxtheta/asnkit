# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

from typing import Any, Dict


class Codec:
    """
    Base interface type for codecs.
    """

    @staticmethod
    def compile_dict(
        specification: Dict[str, Any],
        numeric_enums: bool = False,
        lenient: bool = False,
    ) -> Dict[str, Dict[str, Any]]:
        raise NotImplementedError("Subclasses must implement this method.")

    @staticmethod
    def decode_full_length(data: bytes | bytearray) -> int | None:
        raise NotImplementedError("Subclasses must implement this method.")
