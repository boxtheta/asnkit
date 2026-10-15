# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

EXPECTED = {
    "AllTypes": {
        "extensibility-implied": False,
        "imports": {},
        "object-classes": {},
        "object-sets": {},
        "tags": "AUTOMATIC",
        "types": {
            "Sequence2": {
                "members": [
                    {"components-of": "Sequence9"},
                    {"name": "c", "type": "INTEGER"},
                ],
                "type": "SEQUENCE",
            },
            "Sequence3": {
                "members": [
                    {"components-of": "Sequence2"},
                    {"name": "d", "type": "BOOLEAN"},
                ],
                "type": "SEQUENCE",
            },
            "Sequence9": {
                "members": [
                    {"name": "a", "type": "INTEGER"},
                    {"name": "b", "optional": True, "type": "BOOLEAN"},
                ],
                "type": "SEQUENCE",
            },
        },
        "values": {},
    }
}
