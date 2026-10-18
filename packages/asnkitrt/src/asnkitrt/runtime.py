# Copyright (c) 2025 BoxTheta
# Copyright (c) 2017-2019 Erik Moqvist
#
# SPDX-License-Identifier: MIT

"""Encode and decode ASN.1 types of a specification that has already
been parsed. Nothing in here, or in what it imports, needs the
parser.

"""

import importlib
import typing
from typing import Any, Callable, Dict, Literal, Set, Tuple

from . import syntax
from .syntax.convert import _compiled_to_dict
from .codecs.codec import Codec
from .errors import CompileError, DecodeError, EncodeError

CodecType = Literal["ber", "der", "gser", "jer", "oer", "per", "uper", "xer"]

# The module of a codec in the codecs package has its name.
CODEC_CLASS_NAMES: Dict[str, str] = {
    "ber": "BERCodec",
    "der": "DERCodec",
    "gser": "GSERCodec",
    "jer": "JERCodec",
    "oer": "OERCodec",
    "per": "PERCodec",
    "uper": "UPERCodec",
    "xer": "XERCodec",
}

# A parsed specification, as returned by the parser: module name to
# module contents.
SpecificationDict = Dict[str, Any]

# Compiled types, type checkers or constraints checkers: module name
# to type name to compiled object.
CompiledModules = Dict[str, Dict[str, Any]]


class Specification(object):
    """This class is used to encode and decode ASN.1 types found in an
    ASN.1 specification.

    Instances of this class are created by the functions `load()`,
    `load_asni()` and `load_asni_file()`.

    """

    def __init__(
        self,
        modules: CompiledModules,
        decode_length: Callable[[bytes | bytearray], int | None],
        type_checkers: CompiledModules,
        constraints_checkers: CompiledModules,
    ) -> None:
        self._modules = modules
        self._decode_length = decode_length
        self._types: Dict[str, Any] = {}

        duplicated: Set[str] = set()

        for module_name in modules:
            types = modules[module_name]

            for type_name, type_ in types.items():
                type_.type_checker = type_checkers[module_name][type_name]
                type_.constraints_checker = constraints_checkers[module_name][type_name]

                if type_name in duplicated:
                    continue

                if type_name in self._types:
                    del self._types[type_name]
                    duplicated.add(type_name)
                    continue

                self._types[type_name] = type_

    @property
    def types(self) -> Dict[str, Any]:
        """A dictionary of all unique types in the specification. Types found
        in two or more modules are not part of this dictionary.

        ```python
        >>> question = foo.types['Question']
        >>> question
        Sequence(Question, [Integer(id), IA5String(question)])
        >>> question.encode({'id': 1, 'question': 'Is 1+1=3?'})
        b'0\\x0e\\x02\\x01\\x01\\x16\\x09Is 1+1=3?'
        ```

        """

        return self._types

    @property
    def modules(self) -> CompiledModules:
        """A dictionary of all modules in the specification. Unlike
        `types`, this attribute contains every type, even if
        the type name was found in two or more modules.

        ```python
        >>> question = foo.modules['Foo']['Question']
        >>> question
        Sequence(Question, [Integer(id), IA5String(question)])
        >>> question.encode({'id': 1, 'question': 'Is 1+1=3?'})
        b'0\\x0e\\x02\\x01\\x01\\x16\\x09Is 1+1=3?'
        ```

        """

        return self._modules

    def encode(
        self,
        name: str,
        data: Any,
        check_types: bool = True,
        check_constraints: bool = False,
        **kwargs: Any,
    ) -> bytes:
        """Encode given dictionary `data` as given type `name` and return the
        encoded data as a bytes object.

        If `check_types` is `True` all objects in `data` are checked
        against the expected Python type for its ASN.1 type. Set
        `check_types` to `False` to minimize the runtime overhead,
        but instead get less informative error messages.

        See Types (in the documentation) for a mapping table from ASN.1 types to Python
        types.

        If `check_constraints` is `True` all objects in `data` are
        checked against their ASN.1 type constraints. A
        ConstraintsError exception is raised if the constraints are
        not fulfilled. Set `check_constraints` to `False` to skip
        the constraints check and minimize the runtime overhead, but
        instead get less informative error messages and allow encoding
        of values not fulfilling the constraints.

        ```python
        >>> foo.encode('Question', {'id': 1, 'question': 'Is 1+1=3?'})
        b'0\\x0e\\x02\\x01\\x01\\x16\\x09Is 1+1=3?'
        ```

        """

        try:
            type_ = self._types[name]
        except KeyError:
            raise EncodeError("Type '{}' not found in types dictionary.".format(name))

        if check_types:
            type_.check_types(data)

        if check_constraints:
            type_.check_constraints(data)

        return bytes(type_.encode(data, **kwargs))

    def decode(
        self, name: str, data: bytes | bytearray, check_constraints: bool = False
    ) -> Any:
        """Decode given bytes object `data` as given type `name` and return
        the decoded data as a dictionary.

        If `check_constraints` is `True` all objects in `data` are
        checked against their ASN.1 type constraints. A
        ConstraintsError exception is raised if the constraints are
        not fulfilled. Set `check_constraints` to `False` to skip
        the constraints check and minimize the runtime overhead, but
        instead allow decoding of values not fulfilling the
        constraints.

        ```python
        >>> foo.decode('Question', b'0\\x0e\\x02\\x01\\x01\\x16\\x09Is 1+1=3?')
        {'id': 1, 'question': 'Is 1+1=3?'}
        ```

        """

        try:
            type_ = self._types[name]
        except KeyError:
            raise DecodeError("Type '{}' not found in types dictionary.".format(name))

        decoded = type_.decode(data)

        if check_constraints:
            type_.check_constraints(decoded)

        return decoded

    def decode_with_length(
        self, name: str, data: bytes | bytearray, check_constraints: bool = False
    ) -> Tuple[Any, int]:
        """Same as `decode`, but also
        returns the byte length of the decoded data.

        Use to get the length of indefinite-length BER encoded data.

        ```python
        >>> foo.decode_with_length('Question',
                                    b'0\\x0e\\x02\\x01\\x01\\x16\\x09Is 1+1=3?')
        ({'id': 1, 'question': 'Is 1+1=3?'}, 16)
        ```

        """

        try:
            type_ = self._types[name]
        except KeyError:
            raise DecodeError("Type '{}' not found in types dictionary.".format(name))

        decoded, length = type_.decode_with_length(data)

        if check_constraints:
            type_.check_constraints(decoded)

        return decoded, length

    def decode_length(self, data: bytes | bytearray) -> int | None:
        """Decode the length of given data `data`. Returns None if not enough
        data was given to decode the length.

        This method only works for BER and DER codecs with definite
        length in the first data encoding. Other codecs and
        combinations lacks length information in the data.

        ```python
        >>> foo.decode_length(b'\\x30\\x0e\\x02\\x01\\x01')
        16
        ```

        """

        return self._decode_length(data)


def _load_dict(
    specification: SpecificationDict,
    codec: CodecType = "ber",
    numeric_enums: bool = False,
    lenient: bool = False,
) -> Specification:
    """`load()` of the dictionary of a compiled specification."""

    if codec not in CODEC_CLASS_NAMES:
        raise CompileError("Unsupported codec '{}'.".format(codec))

    # A codec is imported when first used, so that only those used
    # are, and what they need.
    codec_class: typing.Type[Codec] = getattr(
        importlib.import_module(".codecs." + codec, __package__),
        CODEC_CLASS_NAMES[codec],
    )
    type_checker = importlib.import_module(".codecs.type_checker", __package__)
    constraints_checker = importlib.import_module(
        ".codecs.constraints_checker", __package__
    )

    return Specification(
        codec_class.compile_dict(specification, numeric_enums, lenient),
        codec_class.decode_full_length,
        type_checker.compile_dict(specification, numeric_enums),
        constraints_checker.compile_dict(specification, numeric_enums),
    )


def load(
    compiled: Any,
    codec: CodecType = "ber",
    numeric_enums: bool = False,
    nonce: syntax.NonceCheck = None,
    accept_any_nonce: bool = False,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
    verification_keys: syntax.Keys | None = None,
    accept_any_signature: bool = False,
    decryption_keys: syntax.Keys | None = None,
    lenient: bool = False,
) -> Specification:
    """Return a `Specification` object that can be used to
    encode and decode data structures of given compiled specification
    `compiled` with given codec `codec`. `codec` may be one of
    `'ber'`, `'der'`, `'gser'`, `'jer'`, `oer`, `'per'`,
    `'uper'` and `'xer'`.

    `compiled` is a `CompiledSpecification`
    or the contents of an `.asni` file as bytes. Nothing else is
    loaded. A syntax tree, or an `.asnst` file, has to be compiled
    first.

    Give `numeric_enums` as `True` for numeric enumeration values
    instead of strings.

    Give `lenient` as `True` to have the `'der'` codec decode what
    BER allows and DER does not, as it only does if it is told to.
    DER wants a BOOLEAN of 0x00 or 0xff, an INTEGER, an ENUMERATED
    and an OBJECT IDENTIFIER in as few bytes as possible, a length in
    as few bytes as possible and never indefinite, a BIT STRING with
    unused bits of zero and no trailing zero bits in a list of named
    bits, a SET OF in ascending order of its elements, members in the
    order they are declared in, nothing encoded that is a DEFAULT, a
    REAL in its canonical form, and a UTCTime and a GeneralizedTime in
    UTC with seconds and no trailing zeros. What the codec encodes is
    DER whatever `lenient` is, and no other codec looks at it. What is
    left after the data, as in `decode`, is never accepted.

    `nonce`, `accept_any_nonce`, `bound_to`, `accept_any_binding`,
    `verification_keys`, `accept_any_signature` and `decryption_keys`
    tell which files to read, as for
    `from_asni()`. A file that is signed,
    encrypted, bound to something or has a nonce to check is not read
    unless told what to expect.

    ```python
    >>> with open('foo.asni', 'rb') as fin:
    ...     foo = asnkitrt.runtime.load(fin.read())
    ```

    """

    if isinstance(compiled, (bytes, bytearray)):
        compiled = syntax.from_asni(
            compiled,
            nonce,
            accept_any_nonce,
            bound_to,
            accept_any_binding,
            verification_keys,
            accept_any_signature,
            decryption_keys,
        )

    if not isinstance(compiled, syntax.CompiledSpecification):
        raise CompileError("Expected a compiled specification.")

    return _load_dict(_compiled_to_dict(compiled), codec, numeric_enums, lenient)


def load_asni(
    data: bytes | bytearray,
    codec: CodecType = "ber",
    numeric_enums: bool = False,
    nonce: syntax.NonceCheck = None,
    accept_any_nonce: bool = False,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
    verification_keys: syntax.Keys | None = None,
    accept_any_signature: bool = False,
    decryption_keys: syntax.Keys | None = None,
    lenient: bool = False,
) -> Specification:
    """Same as `load()`, but for given contents of an `.asni`
    file only.

    ```python
    >>> foo = asnkitrt.runtime.load_asni(data)
    ```

    """

    return load(
        syntax.from_asni(
            data,
            nonce,
            accept_any_nonce,
            bound_to,
            accept_any_binding,
            verification_keys,
            accept_any_signature,
            decryption_keys,
        ),
        codec,
        numeric_enums,
        lenient=lenient,
    )


def load_asni_file(
    filename: str,
    codec: CodecType = "ber",
    numeric_enums: bool = False,
    nonce: syntax.NonceCheck = None,
    accept_any_nonce: bool = False,
    bound_to: str | None = None,
    accept_any_binding: bool = False,
    verification_keys: syntax.Keys | None = None,
    accept_any_signature: bool = False,
    decryption_keys: syntax.Keys | None = None,
    lenient: bool = False,
) -> Specification:
    """Same as `load()`, but for the `.asni` file with given
    name.

    ```python
    >>> foo = asnkitrt.runtime.load_asni_file('foo.asni')
    ```

    """

    return load(
        syntax.from_asni_file(
            filename,
            nonce,
            accept_any_nonce,
            bound_to,
            accept_any_binding,
            verification_keys,
            accept_any_signature,
            decryption_keys,
        ),
        codec,
        numeric_enums,
        lenient=lenient,
    )
