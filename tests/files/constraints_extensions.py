# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

EXPECTED = {
    "ConstraintsExtensions": {
        "extensibility-implied": False,
        "imports": {},
        "object-classes": {},
        "object-sets": {},
        "types": {
            "Foo": {
                "members": [
                    {"name": "a", "size": [(1, 2), None], "type": "OCTET STRING"},
                    {"name": "b", "size": [(1, 2), None], "type": "OCTET STRING"},
                    {
                        "element": {"type": "BOOLEAN"},
                        "name": "c",
                        "size": [(9, 9), None],
                        "type": "SEQUENCE OF",
                    },
                    {
                        "element": {"restricted-to": [(1, 5), None], "type": "INTEGER"},
                        "name": "d",
                        "size": [(2, 3), None],
                        "type": "SEQUENCE OF",
                    },
                    {
                        "element": {"restricted-to": [(1, 5), None], "type": "INTEGER"},
                        "name": "e",
                        "size": [(2, 3), None],
                        "type": "SEQUENCE OF",
                    },
                ],
                "type": "SEQUENCE",
            }
        },
        "values": {},
    }
}
