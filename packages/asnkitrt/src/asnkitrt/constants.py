# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: MIT

"""Constants shared by the parser and the codecs."""

# The ``...`` in a specification dictionary.
EXTENSION_MARKER = None


class CString(str):
    """A character string given in quotes, as in ``DEFAULT "foo"``, in
    a specification dictionary.

    It is equal to the same plain string, but can be told apart from
    a reference to a value or any other word, which are plain strings
    too.

    """


class ValueRange(tuple):
    """The minimum and maximum of a range of which at least one is
    excluded, as in ``1<..5``, in a specification dictionary.

    It is equal to the same plain tuple, which is what a range
    including both is.

    """

    is_minimum_excluded: bool
    is_maximum_excluded: bool

    def __new__(cls, minimum, maximum, is_minimum_excluded, is_maximum_excluded):
        self = super().__new__(cls, (minimum, maximum))
        self.is_minimum_excluded = is_minimum_excluded
        self.is_maximum_excluded = is_maximum_excluded

        return self

    def __getnewargs__(self):
        return (self[0], self[1], self.is_minimum_excluded, self.is_maximum_excluded)


class Located(dict):
    """A dictionary of a specification dictionary describing something
    that is known where it is found: a module, an assignment, a member
    or a field of an object class.

    It is equal to the same plain dictionary. `location` is the index
    of the file it is found in, among those parsed, and the line and
    the column it starts at in that file, both counted from 1. A
    column is a number of characters, not bytes.

    """

    location: tuple[int, int, int] | None = None
