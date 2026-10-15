# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: Apache-2.0

EXPECTED = {
    "Sketch-Pad": {
        "extensibility-implied": True,
        "imports": {},
        "object-classes": {},
        "object-sets": {},
        "tags": "IMPLICIT",
        "types": {
            "Box": {
                "members": [
                    {"name": "corner", "type": "Point"},
                    {"name": "opposite", "type": "Point"},
                    {"components-of": "Stroke"},
                    {
                        "name": "fill",
                        "optional": True,
                        "tag": {"number": 2},
                        "type": "Colour",
                    },
                ],
                "type": "SEQUENCE",
            },
            "Colour": {
                "size": [3],
                "tag": {"class": "APPLICATION", "number": 7},
                "type": "OCTET STRING",
            },
            "Coordinate": {"restricted-to": [(0, "gridSize")], "type": "INTEGER"},
            "Drawing": {
                "members": [
                    {"name": "title", "type": "Text"},
                    {
                        "name": "units",
                        "type": "ENUMERATED",
                        "values": [
                            ("millimetre", 1),
                            ("point", 3),
                            ("pixel", 10),
                            None,
                        ],
                    },
                    {
                        "element": {"type": "Layer"},
                        "element_name": "layer",
                        "name": "layers",
                        "type": "SEQUENCE OF",
                    },
                ],
                "tag": {"class": "APPLICATION", "number": 1},
                "type": "SEQUENCE",
            },
            "Edit": {
                "members": [
                    {"name": "snapshot", "type": "Drawing"},
                    {"name": "addLayer", "tag": {"number": 0}, "type": "Layer"},
                    {"name": "lock", "tag": {"number": 1}, "type": "LockedLayer"},
                    {"name": "paint", "type": "Colour"},
                    {"name": "erase", "type": "Eraser"},
                    None,
                    {"name": "undo", "type": "Undo"},
                ],
                "type": "CHOICE",
            },
            "Eraser": {"tag": {"class": "APPLICATION", "number": 12}, "type": "NULL"},
            "Journal": {
                "members": [
                    {
                        "name": "revision",
                        "restricted-to": [(0, "MAX")],
                        "type": "INTEGER",
                    },
                    {
                        "element": {"type": "Edit"},
                        "element_name": "edit",
                        "name": "edits",
                        "type": "SEQUENCE OF",
                    },
                ],
                "type": "SEQUENCE",
            },
            "Label": {
                "members": [
                    {"name": "anchor", "type": "Point"},
                    {
                        "element": {
                            "members": [
                                {"name": "plain", "tag": {"number": 0}, "type": "Text"},
                                {"name": "bold", "tag": {"number": 1}, "type": "Text"},
                                {
                                    "name": "glyph",
                                    "restricted-to": [(0, "gridSize")],
                                    "tag": {"number": 2},
                                    "type": "INTEGER",
                                },
                            ],
                            "type": "CHOICE",
                        },
                        "element_name": "run",
                        "name": "runs",
                        "size": [(1, "MAX")],
                        "type": "SEQUENCE OF",
                    },
                    {
                        "name": "align",
                        "type": "ENUMERATED",
                        "values": [
                            ("left", 0),
                            ("centre", 1),
                            ("right", 2),
                            ("justified", 5),
                            None,
                        ],
                    },
                ],
                "type": "SEQUENCE",
            },
            "Layer": {
                "members": [
                    {"name": "name", "type": "LayerName"},
                    {
                        "default": False,
                        "name": "hidden",
                        "tag": {"number": 0},
                        "type": "BOOLEAN",
                    },
                    {
                        "name": "opacity",
                        "optional": True,
                        "restricted-to": [(0, 100)],
                        "tag": {"number": 1},
                        "type": "INTEGER",
                    },
                    {
                        "element": {"type": "Shape"},
                        "element_name": "shape",
                        "name": "shapes",
                        "type": "SEQUENCE OF",
                    },
                ],
                "type": "SEQUENCE",
            },
            "LayerName": {"type": "Text"},
            "LockedLayer": {
                "type": "Layer",
                "with-components": [[{"name": "shapes", "size": [(1, "MAX")]}]],
            },
            "Point": {
                "members": [
                    {"name": "x", "type": "Coordinate"},
                    {"name": "y", "type": "Coordinate"},
                ],
                "type": "SEQUENCE",
            },
            "Shape": {
                "members": [
                    {"name": "dot", "tag": {"number": 0}, "type": "Point"},
                    {
                        "element": {"type": "Point"},
                        "element_name": "vertex",
                        "name": "path",
                        "size": [(2, "MAX")],
                        "tag": {"number": 1},
                        "type": "SEQUENCE OF",
                    },
                    {"name": "box", "tag": {"number": 3}, "type": "Box"},
                    {"name": "label", "tag": {"number": 4}, "type": "Label"},
                    {
                        "element": {"type": "Shape"},
                        "element_name": "member",
                        "name": "group",
                        "size": [(1, "MAX")],
                        "tag": {"number": 8},
                        "type": "SET OF",
                    },
                    {"name": "mirrored", "tag": {"number": 9}, "type": "Shape"},
                    None,
                ],
                "type": "CHOICE",
            },
            "Stroke": {
                "members": [
                    {"name": "colour", "type": "Colour"},
                    {
                        "default": 1,
                        "name": "width",
                        "restricted-to": [(1, 64)],
                        "type": "INTEGER",
                    },
                    {"default": False, "name": "dashed", "type": "BOOLEAN"},
                ],
                "type": "SEQUENCE",
            },
            "Text": {"type": "OCTET STRING"},
            "Undo": {
                "members": [
                    {"name": "steps", "restricted-to": [(1, 64)], "type": "INTEGER"},
                    {"name": "reason", "optional": True, "type": "Text"},
                ],
                "tag": {"class": "APPLICATION", "number": 30},
                "type": "SEQUENCE",
            },
        },
        "values": {"gridSize": {"type": "INTEGER", "value": 4096}},
    }
}
