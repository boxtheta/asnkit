# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

EXPECTED = {
    "Foo": {
        "extensibility-implied": False,
        "imports": {},
        "object-classes": {},
        "object-sets": {},
        "types": {
            "Answer": {
                "members": [
                    {"name": "id", "type": "INTEGER"},
                    {"name": "answer", "type": "BOOLEAN"},
                ],
                "type": "SEQUENCE",
            },
            "Question": {
                "members": [
                    {"name": "id", "type": "INTEGER"},
                    {"name": "question", "type": "IA5String"},
                ],
                "type": "SEQUENCE",
            },
        },
        "values": {},
    }
}
