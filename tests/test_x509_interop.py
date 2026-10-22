# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""X.509 with both runtimes, with and without mappings of ANY DEFINED BY,
against OpenSSL, cryptography, asn1crypto and pyasn1, none of which is
written by the same people nor from the same reading of the standard.

- Certificates and a CRL made by OpenSSL come out of the runtime for
  Python and the runtime for Rust (through ``asnkitcc convert``) byte for
  byte as they went in, and decode to what cryptography and asn1crypto
  read. Their extensions are also taken from one runtime to the other
  in JER, and the signature verifies over the TBS the runtime encodes.

- Certificates built from values, with extensions encoded by both
  runtimes, are accepted by ``openssl verify -x509_strict`` and by the
  path validation of cryptography, and are DER as pyasn1 encodes it.

- Encodings that are not DER are rejected by both runtimes, as by
  cryptography, and the lenient readers of both make the same DER of
  those that BER allows.

The tests need ``openssl``, ``asnkitcc`` (found as tests/compiled.py
says) and the dependency group ``interop``. Without them they are
skipped, unless ASNKIT_INTEROP_REQUIRED is set, as in CI:

    cargo build --release --bin asnkitcc
    uv run --group interop python -m unittest tests.test_x509_interop

"""

import datetime
import os
import shutil
import subprocess
import tempfile
import unittest

import asnkitrt

from tests.compiled import ROOT, _binary

try:
    from asn1crypto import crl as asn1crypto_crl
    from asn1crypto import x509 as asn1crypto_x509
    from asn1crypto.parser import parse as parse_tlv
    from cryptography import x509
    from cryptography.hazmat.primitives import hashes, serialization
    from cryptography.hazmat.primitives.asymmetric import ec, padding, rsa
    from cryptography.x509 import verification
    from pyasn1.codec.der import decoder as pyasn1_decoder
    from pyasn1.codec.der import encoder as pyasn1_encoder
    from pyasn1_modules import rfc5280 as pyasn1_rfc5280

    MISSING_MODULE = None
except ImportError as error:
    MISSING_MODULE = error.name

OPENSSL = shutil.which("openssl")
ASNKITCC = _binary()
FILES = os.path.join(ROOT, "tests", "files", "x509_interop")
RFC5280_ASN = os.path.join(ROOT, "examples", "compiled", "rfc5280.asn")
UTC = datetime.timezone.utc

# What JER says of an ANY with no mapping, which it can not encode.
JER_NEEDS_TYPE = "JER needs the type of the value of an ANY"

UPN = "1.3.6.1.4.1.311.20.2.3"

# The type of the value of each extension that is decoded.
EXTENSION_TYPES = {
    "2.5.29.14": "SubjectKeyIdentifier",
    "2.5.29.15": "KeyUsage",
    "2.5.29.17": "SubjectAltName",
    "2.5.29.18": "IssuerAltName",
    "2.5.29.19": "BasicConstraints",
    "2.5.29.20": "CRLNumber",
    "2.5.29.21": "CRLReason",
    "2.5.29.30": "NameConstraints",
    "2.5.29.31": "CRLDistributionPoints",
    "2.5.29.32": "CertificatePolicies",
    "2.5.29.35": "AuthorityKeyIdentifier",
    "2.5.29.37": "ExtKeyUsageSyntax",
    "1.3.6.1.5.5.7.1.1": "AuthorityInfoAccessSyntax",
}
EXTENSION_OIDS = {name: oid for oid, name in EXTENSION_TYPES.items()}

OPENSSL_CONFIG = """\
[req]
distinguished_name = dn
[dn]

[root]
basicConstraints = critical,CA:TRUE,pathlen:1
keyUsage = critical,keyCertSign,cRLSign
subjectKeyIdentifier = hash

[name_constraints]
basicConstraints = critical,CA:TRUE
keyUsage = critical,keyCertSign
subjectKeyIdentifier = hash
nameConstraints = critical,permitted;DNS:example.com,\
permitted;IP:10.0.0.0/255.0.0.0,excluded;email:.bad.example

[full]
basicConstraints = CA:FALSE
keyUsage = critical,digitalSignature,keyEncipherment,keyAgreement
extendedKeyUsage = serverAuth,clientAuth,codeSigning,emailProtection,1.3.6.1.4.1.99999.7
subjectKeyIdentifier = hash
authorityKeyIdentifier = keyid,issuer
subjectAltName = @san
certificatePolicies = 2.23.140.1.2.1,@policy
authorityInfoAccess = OCSP;URI:http://ocsp.example.com,caIssuers;URI:http://ca.example.com/ca.crt
crlDistributionPoints = URI:http://crl.example.com/ca.crl
1.3.6.1.4.1.99999.9 = ASN1:UTF8String:custom extension

[san]
DNS.1 = www.example.com
DNS.2 = *.example.com
IP.1 = 10.1.2.3
IP.2 = 2001:db8::1
email.1 = admin@example.com
URI.1 = https://example.com/path?q=1
otherName.1 = 1.3.6.1.4.1.311.20.2.3;UTF8:upn@example.com
dirName.1 = directory_name
RID.1 = 1.2.3.4

[directory_name]
C = PT
O = Dir Org
CN = Dir Name

[policy]
policyIdentifier = 1.3.6.1.4.1.99999.1.1
CPS.1 = https://example.com/cps
userNotice.1 = @notice

[notice]
explicitText = "Explicit notice text"
organization = "Example Org"
noticeNumbers = 1, 2, 3

[minimal]
basicConstraints = CA:FALSE
"""

CA_CONFIG = """\
[ca]
default_ca = default
[default]
database = index.txt
crlnumber = crlnumber
default_md = sha256
default_crl_days = 30
crl_extensions = crl_extensions
[crl_extensions]
authorityKeyIdentifier = keyid
"""

# The certificates signed by the RSA root: (name, key, options of the
# signature, subject, extensions, options of the request, serial).
LEAVES = [
    ("full-rsa", "rsa", ["-sha256"],
     "/C=PT/ST=Lisboa/L=Lisboa/O=Example, Inc./OU=Eng/CN=www.example.com"
     "/emailAddress=admin@example.com", "full", [], None),
    ("full-p256", "p256", ["-sha384"], "/CN=p256.example.com", "full", [], None),
    ("pss-key", "pss", ["-sha256"], "/CN=pss-key.example.com", "full", [], None),
    ("pss-signature", "rsa",
     ["-sha256", "-sigopt", "rsa_padding_mode:pss", "-sigopt", "rsa_pss_saltlen:32"],
     "/CN=pss-signature.example.com", "full", [], None),
    ("ed25519", "ed25519", [], "/CN=ed25519.example.com", "minimal", [], None),
    ("ed448", "ed448", [], "/CN=ed448.example.com", "minimal", [], None),
    ("multi-valued-rdn", "p256", ["-sha256"], "/O=A+OU=B/CN=multi+serialNumber=42",
     "minimal", ["-multivalue-rdn"], None),
    ("serial-1", "p256", ["-sha256"], "/CN=serial1", "minimal", [], "1"),
    ("serial-20-bytes", "p256", ["-sha256"], "/CN=bigserial", "minimal", [],
     "0x7fffffffffffffffffffffffffffffffffffffff"),
]  # fmt: skip

KEYS = {
    "rsa": ["-algorithm", "RSA", "-pkeyopt", "rsa_keygen_bits:2048"],
    "pss": ["-algorithm", "RSA-PSS", "-pkeyopt", "rsa_keygen_bits:2048"],
    "p256": ["-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-256"],
    "p384": ["-algorithm", "EC", "-pkeyopt", "ec_paramgen_curve:P-384"],
    "ed25519": ["-algorithm", "ED25519"],
    "ed448": ["-algorithm", "ED448"],
}


def missing_requirement():
    if OPENSSL is None:
        return "openssl is not installed"

    if ASNKITCC is None:
        return "asnkitcc is not built"

    if MISSING_MODULE is not None:
        return "{} is not installed (uv sync --group interop)".format(MISSING_MODULE)

    return None


class RustError(Exception):
    pass


class Specification:
    """A compiled specification, as both runtimes load it."""

    def __init__(self, path):
        self.path = path
        self.der = asnkitrt.load_asni_file(path, "der")
        self.lenient = asnkitrt.load_asni_file(path, "der", lenient=True)
        self.jer = asnkitrt.load_asni_file(path, "jer")

    def rust(self, type_name, data, source="der", target="der", lenient=False):
        """What the runtime for Rust converts `data` to."""

        arguments = [ASNKITCC, "convert", "-t", type_name, "--from", source]
        arguments += ["--to", target]

        if lenient:
            arguments.append("--lenient")

        if source == "der":
            arguments.append("--input-hex")
            data = data.hex().encode()

        if target == "der":
            arguments.append("--output-hex")

        arguments.append(self.path)
        result = subprocess.run(arguments, input=data, capture_output=True)

        if result.returncode != 0:
            raise RustError(result.stderr.decode().strip())

        if target == "der":
            return bytes.fromhex(result.stdout.decode().strip())

        return result.stdout


def utc(time):
    """A datetime of a Time, which is a CHOICE of two."""

    _, value = time

    return value if value.tzinfo else value.replace(tzinfo=UTC)


def inner_tlv(der):
    """What is inside an EXPLICIT tag."""

    return parse_tlv(der)[4]


def general_name(name, specification):
    """A GeneralName as decoded by asnkit, as general_name_of_asn1crypto
    gives it."""

    kind, value = name

    if kind == "directoryName":
        return kind, bytes(specification.encode("Name", value))

    if kind == "otherName":
        inner = value["value"]

        if isinstance(inner, (bytes, bytearray)):
            inner = bytes(inner)

        return kind, value["type-id"], inner

    if isinstance(value, (bytes, bytearray)):
        return kind, bytes(value)

    return kind, value


ASN1CRYPTO_GENERAL_NAMES = {
    "other_name": "otherName",
    "rfc822_name": "rfc822Name",
    "dns_name": "dNSName",
    "directory_name": "directoryName",
    "uniform_resource_identifier": "uniformResourceIdentifier",
    "ip_address": "iPAddress",
    "registered_id": "registeredID",
}


def general_name_of_asn1crypto(name, mapped):
    kind = ASN1CRYPTO_GENERAL_NAMES[name.name]
    chosen = name.chosen

    if kind == "directoryName":
        return kind, inner_tlv(chosen.dump())

    if kind == "otherName":
        value = inner_tlv(chosen["value"].dump())

        if mapped and chosen["type_id"].dotted == UPN:
            value = parse_tlv(value)[4].decode()

        return kind, chosen["type_id"].dotted, value

    if kind == "registeredID":
        return kind, chosen.dotted

    if kind == "iPAddress":
        return kind, chosen.contents

    # The text as it is: .native of asn1crypto changes a URI to an IRI.
    return kind, chosen.contents.decode("latin-1")


def display_text(value):
    return None if value is None else value[1]


def extension_of_asnkit(type_name, value, specification, mapped):
    """An extension decoded by asnkit, as extension_of_asn1crypto gives
    it, or None if it is not compared."""

    if type_name == "BasicConstraints":
        return value.get("cA", False), value.get("pathLenConstraint")

    if type_name == "KeyUsage":
        data, number_of_bits = value
        bits = "".join("{:08b}".format(byte) for byte in data)[:number_of_bits]

        return {i for i, bit in enumerate(bits) if bit == "1"}

    if type_name == "ExtKeyUsageSyntax":
        return list(value)

    if type_name in ("SubjectAltName", "IssuerAltName"):
        return [general_name(name, specification) for name in value]

    if type_name == "AuthorityInfoAccessSyntax":
        return [
            (item["accessMethod"], general_name(item["accessLocation"], specification))
            for item in value
        ]

    if type_name == "CRLDistributionPoints":
        return [
            [
                general_name(name, specification)
                for name in point["distributionPoint"][1]
            ]
            for point in value
        ]

    if type_name == "CertificatePolicies":
        policies = []

        for policy in value:
            qualifiers = []

            for item in policy.get("policyQualifiers", []):
                qualifier = item["qualifier"]

                if not mapped:
                    qualifier = bytes(qualifier)
                elif not isinstance(qualifier, str):
                    reference = qualifier.get("noticeRef")
                    qualifier = (
                        display_text(qualifier.get("explicitText")),
                        reference and display_text(reference["organization"]),
                        reference and list(reference["noticeNumbers"]),
                    )

                qualifiers.append((item["policyQualifierId"], qualifier))

            policies.append((policy["policyIdentifier"], qualifiers))

        return policies

    if type_name == "CRLReason":
        return value.lower()

    if type_name == "CRLNumber":
        return value

    return None


def extension_of_asn1crypto(type_name, parsed, mapped):
    if type_name == "BasicConstraints":
        return parsed["ca"].native, parsed["path_len_constraint"].native

    if type_name == "KeyUsage":
        names = parsed.native
        bits = asn1crypto_x509.KeyUsage._map

        return {bit for bit, name in bits.items() if name in names}

    if type_name == "ExtKeyUsageSyntax":
        return [purpose.dotted for purpose in parsed]

    if type_name in ("SubjectAltName", "IssuerAltName"):
        return [general_name_of_asn1crypto(name, mapped) for name in parsed]

    if type_name == "AuthorityInfoAccessSyntax":
        return [
            (
                item["access_method"].dotted,
                general_name_of_asn1crypto(item["access_location"], mapped),
            )
            for item in parsed
        ]

    if type_name == "CRLDistributionPoints":
        return [
            [
                general_name_of_asn1crypto(name, mapped)
                for name in point["distribution_point"].chosen
            ]
            for point in parsed
        ]

    if type_name == "CertificatePolicies":
        policies = []

        for policy in parsed:
            qualifiers = []

            for item in policy["policy_qualifiers"] or []:
                qualifier = item["qualifier"]

                if not mapped:
                    qualifier = qualifier.dump()
                elif item["policy_qualifier_id"].native == "user_notice":
                    notice = qualifier.native
                    reference = notice["notice_ref"]
                    qualifier = (
                        notice["explicit_text"],
                        reference and reference["organization"],
                        reference and reference["notice_numbers"],
                    )
                else:
                    qualifier = qualifier.native

                qualifiers.append((item["policy_qualifier_id"].dotted, qualifier))

            policies.append((policy["policy_identifier"].dotted, qualifiers))

        return policies

    if type_name == "CRLReason":
        return parsed.native.replace("_", "").lower()

    if type_name == "CRLNumber":
        return parsed.native

    return None


def verify_signature(public_key, certificate, data):
    parameters = certificate.signature_algorithm_parameters

    if isinstance(public_key, rsa.RSAPublicKey):
        public_key.verify(
            certificate.signature,
            data,
            parameters,
            certificate.signature_hash_algorithm,
        )
    elif isinstance(public_key, ec.EllipticCurvePublicKey):
        public_key.verify(certificate.signature, data, parameters)
    else:
        public_key.verify(certificate.signature, data)


# A small tree of TLVs, to make encodings that are not DER of those
# that are. The lengths are worked out again when it is dumped.
class Tlv:
    def __init__(self, tag, content=None, children=None):
        self.tag = tag
        self.content = content
        self.children = children
        # "minimal", "long" (one more byte than needed) or "indefinite".
        self.length_form = "minimal"


def parse_tlvs(data, offset=0, end=None):
    end = len(data) if end is None else end
    tlvs = []

    while offset < end:
        tag = data[offset]
        first = data[offset + 1]
        offset += 2

        if first < 0x80:
            length = first
        else:
            length = int.from_bytes(data[offset : offset + (first & 0x7F)], "big")
            offset += first & 0x7F

        if tag & 0x20:
            tlvs.append(Tlv(tag, children=parse_tlvs(data, offset, offset + length)))
        else:
            tlvs.append(Tlv(tag, content=bytes(data[offset : offset + length])))

        offset += length

    return tlvs


def dump_tlv(tlv):
    if tlv.children is None:
        contents = tlv.content
    else:
        contents = b"".join(dump_tlv(child) for child in tlv.children)

    if tlv.length_form == "indefinite":
        return bytes([tlv.tag, 0x80]) + contents + b"\x00\x00"

    length = len(contents)
    size = (length.bit_length() + 7) // 8

    if tlv.length_form == "long":
        header = bytes([0x80 | (size + 1)]) + length.to_bytes(size + 1, "big")
    elif length < 0x80:
        header = bytes([length])
    else:
        header = bytes([0x80 | size]) + length.to_bytes(size, "big")

    return bytes([tlv.tag]) + header + contents


def tbs_of(certificate):
    return certificate.children[0].children


def extension_of(certificate, extension_id):
    extensions = tbs_of(certificate)[7].children[0].children

    return next(e for e in extensions if e.children[0].content == extension_id)


KEY_USAGE_ID = bytes.fromhex("551d0f")
BASIC_CONSTRAINTS_ID = bytes.fromhex("551d13")
SUBJECT_KEY_IDENTIFIER_ID = bytes.fromhex("551d0e")


def non_minimal_length(certificate):
    tbs_of(certificate)[0].length_form = "long"


def indefinite_length(certificate):
    certificate.length_form = "indefinite"


def boolean_true_as_one(certificate):
    extension_of(certificate, KEY_USAGE_ID).children[1].content = b"\x01"


def default_encoded(certificate):
    extension = extension_of(certificate, BASIC_CONSTRAINTS_ID)
    extension.children.insert(1, Tlv(0x01, content=b"\x00"))


def integer_leading_zero(certificate):
    serial = tbs_of(certificate)[1]
    serial.content = b"\x00" + serial.content


def utc_time_without_seconds(certificate):
    not_before = tbs_of(certificate)[4].children[0]
    not_before.content = not_before.content[:10] + b"Z"


def utc_time_with_offset(certificate):
    not_before = tbs_of(certificate)[4].children[0]
    not_before.content = not_before.content[:12] + b"+0000"


def generalized_time_with_fraction(certificate):
    not_after = tbs_of(certificate)[4].children[1]
    assert not_after.tag == 0x18
    not_after.content = not_after.content[:-1] + b".0Z"


def set_of_not_sorted(certificate):
    attributes = tbs_of(certificate)[5].children[0].children
    assert len(attributes) == 2
    attributes.reverse()


def constructed_octet_string(certificate):
    extension = extension_of(certificate, SUBJECT_KEY_IDENTIFIER_ID)
    value = extension.children[-1].content
    extension.children[-1] = Tlv(
        0x24,
        children=[Tlv(0x04, content=value[:5]), Tlv(0x04, content=value[5:])],
    )


def object_identifier_padded(certificate):
    algorithm = certificate.children[1].children[0]
    algorithm.content = algorithm.content[:1] + b"\x80" + algorithm.content[1:]


def trailing_bytes(certificate):
    return dump_tlv(certificate) + b"\x00\x00"


def unmapped_selector(certificate):
    # sha224WithRSAEncryption, which is DER but not in x509.toml.
    for algorithm in (tbs_of(certificate)[2], certificate.children[1]):
        algorithm.children[0].content = bytes.fromhex("2a864886f70d01010e")


# (name, certificate, mutation, if BER allows it, if a lenient reader
# makes the DER of OpenSSL of it again).
NOT_DER = [
    ("non-minimal length", "full-p256", non_minimal_length, True, True),
    ("indefinite length", "full-p256", indefinite_length, True, True),
    ("BOOLEAN TRUE as 0x01", "full-p256", boolean_true_as_one, True, True),
    ("DEFAULT value encoded", "full-p256", default_encoded, True, True),
    ("UTCTime without seconds", "full-p256", utc_time_without_seconds, True, False),
    ("UTCTime with +0000", "full-p256", utc_time_with_offset, True, True),
    ("GeneralizedTime with .0", "root-rsa", generalized_time_with_fraction, True, True),
    ("SET OF not sorted", "multi-valued-rdn", set_of_not_sorted, True, True),
    ("constructed OCTET STRING", "full-p256", constructed_octet_string, True, True),
    ("INTEGER with a leading 0x00", "serial-1", integer_leading_zero, False, None),
    ("OBJECT IDENTIFIER with 0x80", "full-p256", object_identifier_padded, False, None),
    ("trailing bytes", "full-p256", trailing_bytes, False, None),
]  # fmt: skip


def openssl(directory, *arguments, check=True):
    return subprocess.run(
        [OPENSSL, *arguments],
        cwd=directory,
        capture_output=True,
        check=check,
    )


def read(directory, name):
    with open(os.path.join(directory, name), "rb") as fin:
        return fin.read()


def make_certificates(directory):
    """Keys, certificates and a CRL made by OpenSSL, the DER of which is
    returned by name."""

    with open(os.path.join(directory, "openssl.cnf"), "w") as fout:
        fout.write(OPENSSL_CONFIG)

    with open(os.path.join(directory, "ca.cnf"), "w") as fout:
        fout.write(CA_CONFIG)

    for name, options in KEYS.items():
        openssl(directory, "genpkey", *options, "-out", name + ".key")

    # Valid until after 2050, for a notAfter in GeneralizedTime.
    openssl(directory, "req", "-new", "-x509", "-key", "rsa.key", "-sha256",
            "-days", "18000", "-utf8", "-subj",
            "/C=PT/O=Interop Root Ç/CN=Interop Root CA",
            "-config", "openssl.cnf", "-extensions", "root", "-out", "root-rsa.pem")  # fmt: skip
    openssl(directory, "req", "-new", "-x509", "-key", "p384.key", "-sha384",
            "-days", "3650", "-subj", "/CN=Name Constraints Root",
            "-config", "openssl.cnf", "-extensions", "name_constraints",
            "-out", "root-p384-name-constraints.pem")  # fmt: skip

    for name, key, signature, subject, extensions, request, serial in LEAVES:
        openssl(directory, "req", "-new", "-key", key + ".key", "-subj", subject,
                *request, "-config", "openssl.cnf", "-out", name + ".csr")  # fmt: skip
        openssl(directory, "x509", "-req", "-in", name + ".csr",
                "-CA", "root-rsa.pem", "-CAkey", "rsa.key", *signature,
                "-days", "397", "-extfile", "openssl.cnf", "-extensions", extensions,
                "-set_serial", serial or "0x" + os.urandom(16).hex(),
                "-out", name + ".pem")  # fmt: skip

    open(os.path.join(directory, "index.txt"), "w").close()

    with open(os.path.join(directory, "crlnumber"), "w") as fout:
        fout.write("1000\n")

    for name, reason in (("full-p256", "keyCompromise"), ("ed25519", "superseded")):
        openssl(directory, "ca", "-config", "ca.cnf", "-cert", "root-rsa.pem",
                "-keyfile", "rsa.key", "-revoke", name + ".pem",
                "-crl_reason", reason)  # fmt: skip

    openssl(directory, "ca", "-gencrl", "-config", "ca.cnf", "-cert", "root-rsa.pem",
            "-keyfile", "rsa.key", "-out", "root-rsa.crl.pem")  # fmt: skip

    certificates = {}

    for name in ["root-rsa", "root-p384-name-constraints"] + [
        leaf[0] for leaf in LEAVES
    ]:
        openssl(directory, "x509", "-in", name + ".pem", "-outform", "der",
                "-out", name + ".der")  # fmt: skip
        certificates[name] = read(directory, name + ".der")

    openssl(directory, "crl", "-in", "root-rsa.crl.pem", "-outform", "der",
            "-out", "root-rsa.crl.der")  # fmt: skip

    return certificates, read(directory, "root-rsa.crl.der")


def compile_specification(directory, name, mapped):
    arguments = [ASNKITCC, "compile", "-o", os.path.join(directory, name)]

    if mapped:
        arguments += ["--any-defined-by", os.path.join(FILES, "x509.toml")]

    arguments.append(RFC5280_ASN)

    if mapped:
        arguments.append(os.path.join(FILES, "pkcs1_pss.asn"))

    subprocess.run(arguments, check=True, capture_output=True)

    return Specification(os.path.join(directory, name))


class X509InteropTest(unittest.TestCase):
    maxDiff = None

    @classmethod
    def setUpClass(cls):
        reason = missing_requirement()

        if reason is not None:
            if os.environ.get("ASNKIT_INTEROP_REQUIRED"):
                raise RuntimeError(reason)

            raise unittest.SkipTest(reason)

        cls.directory = tempfile.TemporaryDirectory()
        cls.path = cls.directory.name
        cls.certificates, cls.crl = make_certificates(cls.path)
        cls.specifications = {
            "plain": compile_specification(cls.path, "plain.asni", False),
            "mapped": compile_specification(cls.path, "mapped.asni", True),
        }

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def key(self, name):
        return serialization.load_pem_private_key(read(self.path, name + ".key"), None)

    def issuer_of(self, name):
        return self.certificates[name if name.startswith("root-") else "root-rsa"]

    def check_extension(self, specification, type_name, data, parsed):
        """Both runtimes decode the value of an extension, encode it as it
        was, take it from each other in JER, and decode it to what
        asn1crypto reads."""

        mapped = specification is self.specifications["mapped"]
        rust = specification.rust

        self.assertEqual(rust(type_name, data), data, "Rust")

        der = specification.der
        value = der.decode(type_name, data)
        self.assertEqual(der.encode(type_name, value), data, "Python")

        try:
            jer = rust(type_name, data, target="jer")
        except RustError as error:
            # An ANY without a mapping has no JER.
            self.assertIn(JER_NEEDS_TYPE, str(error))
        else:
            from_rust = specification.jer.decode(type_name, jer)
            self.assertEqual(der.encode(type_name, from_rust), data, "JER of Rust")
            jer = specification.jer.encode(type_name, value)
            self.assertEqual(rust(type_name, jer, source="jer"), data, "JER to Rust")

        expected = extension_of_asn1crypto(type_name, parsed, mapped)
        actual = extension_of_asnkit(type_name, value, der, mapped)
        self.assertEqual(actual, expected, "asn1crypto")

    def check_extensions(self, specification, extensions, parsed_extensions):
        for extension, parsed in zip(extensions, parsed_extensions):
            type_name = EXTENSION_TYPES.get(extension["extnID"])

            if type_name is None:
                continue

            with self.subTest(extension=type_name):
                self.check_extension(
                    specification,
                    type_name,
                    bytes(extension["extnValue"]),
                    parsed["extn_value"].parsed,
                )

    def test_certificates(self):
        for name, data in self.certificates.items():
            reference = x509.load_der_x509_certificate(data)
            parsed = asn1crypto_x509.Certificate.load(data)
            tbs_of_asn1crypto = parsed["tbs_certificate"]
            issuer = x509.load_der_x509_certificate(self.issuer_of(name))

            for kind, specification in self.specifications.items():
                with self.subTest(certificate=name, specification=kind):
                    der = specification.der
                    self.assertEqual(specification.rust("Certificate", data), data)
                    value = der.decode("Certificate", data)
                    self.assertEqual(der.encode("Certificate", value), data)

                    tbs = value["tbsCertificate"]
                    self.assertEqual(tbs["serialNumber"], reference.serial_number)
                    self.assertEqual(
                        value["signatureAlgorithm"]["algorithm"],
                        reference.signature_algorithm_oid.dotted_string,
                    )
                    self.assertEqual(bytes(value["signature"][0]), reference.signature)
                    self.assertEqual(
                        utc(tbs["validity"]["notBefore"]),
                        reference.not_valid_before_utc,
                    )
                    self.assertEqual(
                        utc(tbs["validity"]["notAfter"]),
                        reference.not_valid_after_utc,
                    )

                    for field in ("issuer", "subject"):
                        self.assertEqual(
                            [
                                [(a["type"], bytes(a["value"])) for a in rdn]
                                for rdn in tbs[field][1]
                            ],
                            [
                                [(a["type"].dotted, a["value"].dump()) for a in rdn]
                                for rdn in tbs_of_asn1crypto[field].chosen
                            ],
                        )

                    self.assertEqual(
                        der.encode("SubjectPublicKeyInfo", tbs["subjectPublicKeyInfo"]),
                        tbs_of_asn1crypto["subject_public_key_info"].dump(),
                    )
                    self.assertEqual(
                        [
                            (
                                e["extnID"],
                                e.get("critical", False),
                                bytes(e["extnValue"]),
                            )
                            for e in tbs.get("extensions", [])
                        ],
                        [
                            (
                                e["extn_id"].dotted,
                                e["critical"].native,
                                e["extn_value"].contents,
                            )
                            for e in tbs_of_asn1crypto["extensions"]
                        ],
                    )

                    # What was signed is the TBS as the runtime encodes it.
                    verify_signature(
                        issuer.public_key(),
                        reference,
                        bytes(der.encode("TBSCertificate", tbs)),
                    )

                    self.check_extensions(
                        specification,
                        tbs.get("extensions", []),
                        tbs_of_asn1crypto["extensions"],
                    )

    def test_crl(self):
        reference = x509.load_der_x509_crl(self.crl)
        parsed = asn1crypto_crl.CertificateList.load(self.crl)["tbs_cert_list"]
        issuer = x509.load_der_x509_certificate(self.certificates["root-rsa"])

        for kind, specification in self.specifications.items():
            with self.subTest(specification=kind):
                der = specification.der
                self.assertEqual(
                    specification.rust("CertificateList", self.crl), self.crl
                )
                value = der.decode("CertificateList", self.crl)
                self.assertEqual(der.encode("CertificateList", value), self.crl)

                tbs = value["tbsCertList"]
                self.assertEqual(utc(tbs["thisUpdate"]), reference.last_update_utc)
                self.assertEqual(utc(tbs["nextUpdate"]), reference.next_update_utc)
                revoked = tbs["revokedCertificates"]
                self.assertEqual(
                    [(r["userCertificate"], utc(r["revocationDate"])) for r in revoked],
                    [(r.serial_number, r.revocation_date_utc) for r in reference],
                )
                issuer.public_key().verify(
                    reference.signature,
                    bytes(der.encode("TBSCertList", tbs)),
                    reference.signature_algorithm_parameters,
                    reference.signature_hash_algorithm,
                )

                self.check_extensions(
                    specification, tbs["crlExtensions"], parsed["crl_extensions"]
                )

                for entry, parsed_entry in zip(revoked, parsed["revoked_certificates"]):
                    self.check_extensions(
                        specification,
                        entry["crlEntryExtensions"],
                        parsed_entry["crl_entry_extensions"],
                    )

    def build(self, issuer_name, issuer_key, subject_key, algorithm, sign, dns_name):
        """A certificate built from values with the mapped specification,
        the subject alternative name of which is encoded by the runtime
        for Rust, from JER, and the rest by the runtime for Python."""

        specification = self.specifications["mapped"]
        der = specification.der
        issuer = der.decode("Certificate", self.certificates[issuer_name])

        def key_identifier(key):
            public_key = key.public_key()

            return x509.SubjectKeyIdentifier.from_public_key(public_key).digest

        def extension(type_name, value, critical=False):
            if isinstance(value, str):
                encoded = specification.rust(type_name, value.encode(), source="jer")
            else:
                encoded = bytes(der.encode(type_name, value))

            extension = {"extnID": EXTENSION_OIDS[type_name], "extnValue": encoded}

            if critical:
                extension["critical"] = True

            return extension

        def utf8_string(text):
            data = text.encode()

            return bytes([0x0C, len(data)]) + data

        public_key = subject_key.public_key().public_bytes(
            serialization.Encoding.DER,
            serialization.PublicFormat.SubjectPublicKeyInfo,
        )
        tbs = {
            "version": 2,
            "serialNumber": int.from_bytes(os.urandom(16), "big") >> 1,
            "signature": algorithm,
            "issuer": issuer["tbsCertificate"]["subject"],
            "validity": {
                "notBefore": ("utcTime", datetime.datetime(2026, 1, 1, tzinfo=UTC)),
                "notAfter": ("generalTime", datetime.datetime(2051, 1, 1, tzinfo=UTC)),
            },
            "subject": (
                "rdnSequence",
                [
                    [{"type": "2.5.4.6", "value": b"\x13\x02PT"}],
                    [{"type": "2.5.4.10", "value": utf8_string("asnkit ção")}],
                    [{"type": "2.5.4.3", "value": utf8_string(dns_name)}],
                ],
            ),
            "subjectPublicKeyInfo": der.decode("SubjectPublicKeyInfo", public_key),
            "extensions": [
                # The default is left out.
                extension("BasicConstraints", {"cA": False}),
                # A trailing zero bit, which DER leaves out.
                extension("KeyUsage", (b"\xa0", 4), critical=True),
                extension("ExtKeyUsageSyntax", ["1.3.6.1.5.5.7.3.1"]),
                extension("SubjectKeyIdentifier", key_identifier(subject_key)),
                extension(
                    "AuthorityKeyIdentifier",
                    {"keyIdentifier": key_identifier(issuer_key)},
                ),
                extension(
                    "SubjectAltName",
                    '[{"dNSName":"%s"},{"iPAddress":"0a000001"},'
                    '{"otherName":{"type-id":"%s","value":"upn@example.com"}}]'
                    % (dns_name, UPN),
                ),
                extension(
                    "IssuerAltName",
                    [
                        ("directoryName", issuer["tbsCertificate"]["subject"]),
                        ("registeredID", "1.2.3.4.5"),
                    ],
                ),
                extension(
                    "CertificatePolicies",
                    [
                        {
                            "policyIdentifier": "1.3.6.1.4.1.99999.1.1",
                            "policyQualifiers": [
                                {
                                    "policyQualifierId": "1.3.6.1.5.5.7.2.1",
                                    "qualifier": "https://example.com/cps",
                                },
                                {
                                    "policyQualifierId": "1.3.6.1.5.5.7.2.2",
                                    "qualifier": {
                                        "noticeRef": {
                                            "organization": (
                                                "visibleString",
                                                "Example Org",
                                            ),
                                            "noticeNumbers": [7, 300],
                                        },
                                        "explicitText": (
                                            "utf8String",
                                            "Aviso explícito",
                                        ),
                                    },
                                },
                            ],
                        },
                    ],
                ),
            ],
        }
        signature = sign(bytes(der.encode("TBSCertificate", tbs)))

        return bytes(
            der.encode(
                "Certificate",
                {
                    "tbsCertificate": tbs,
                    "signatureAlgorithm": algorithm,
                    "signature": (signature, 8 * len(signature)),
                },
            )
        )

    def test_built_certificates(self):
        rsa_key = self.key("rsa")
        p384_key = self.key("p384")
        sha256 = {"algorithm": "2.16.840.1.101.3.4.2.1", "parameters": None}

        def pkcs1(data):
            return rsa_key.sign(data, padding.PKCS1v15(), hashes.SHA256())

        def pss(data):
            return rsa_key.sign(
                data, padding.PSS(padding.MGF1(hashes.SHA256()), 32), hashes.SHA256()
            )

        def ecdsa(data):
            return p384_key.sign(data, ec.ECDSA(hashes.SHA384()))

        # (name, issuer, its key, algorithm, signer, name of the subject,
        # if the issuer accepts it).
        cases = [
            ("PKCS #1", "root-rsa", rsa_key,
             {"algorithm": "1.2.840.113549.1.1.11", "parameters": None},
             pkcs1, "www.example.com", True),
            ("PSS", "root-rsa", rsa_key,
             {"algorithm": "1.2.840.113549.1.1.10",
              "parameters": {
                  "hashAlgorithm": sha256,
                  "maskGenAlgorithm": {
                      "algorithm": "1.2.840.113549.1.1.8", "parameters": sha256,
                  },
                  "saltLength": 32}},
             pss, "www.example.com", True),
            ("ECDSA within the name constraints", "root-p384-name-constraints",
             p384_key, {"algorithm": "1.2.840.10045.4.3.3"}, ecdsa,
             "ok.example.com", True),
            ("ECDSA out of the name constraints", "root-p384-name-constraints",
             p384_key, {"algorithm": "1.2.840.10045.4.3.3"}, ecdsa,
             "evil.test", False),
        ]  # fmt: skip

        for case, issuer_name, issuer_key, algorithm, sign, dns_name, valid in cases:
            with self.subTest(case=case):
                data = self.build(
                    issuer_name, issuer_key, self.key("p256"), algorithm, sign, dns_name
                )
                path = os.path.join(self.path, "built.der")

                with open(path, "wb") as fout:
                    fout.write(data)

                text = openssl(self.path, "x509", "-inform", "der", "-in", path,
                               "-noout", "-text").stdout.decode(errors="replace")  # fmt: skip

                # How OpenSSL writes an otherName is not the same in all
                # versions.
                self.assertRegex(text, "othername: ?UPN:+upn@example.com")

                for expected in (
                    "IP Address:10.0.0.1",
                    "CPS: https://example.com/cps",
                    "Numbers: 7, 300",
                    "Explicit Text: Aviso explícito",
                ):
                    self.assertIn(expected, text)

                pem = os.path.join(self.path, "built.pem")
                openssl(self.path, "x509", "-inform", "der", "-in", path, "-out", pem)
                result = openssl(self.path, "verify", "-x509_strict", "-CAfile",
                                 issuer_name + ".pem", pem, check=False)  # fmt: skip
                output = (result.stdout + result.stderr).decode()

                if valid:
                    self.assertEqual(result.returncode, 0, output)
                else:
                    self.assertIn("permitted subtree violation", output)

                certificate = x509.load_der_x509_certificate(data)
                issuer = x509.load_der_x509_certificate(self.certificates[issuer_name])
                certificate.verify_directly_issued_by(issuer)
                other_name = certificate.extensions.get_extension_for_class(
                    x509.SubjectAlternativeName
                ).value.get_values_for_type(x509.OtherName)[0]
                self.assertEqual(other_name.value, b"\x0c\x0fupn@example.com")
                key_usage = certificate.extensions.get_extension_for_class(
                    x509.KeyUsage
                ).value
                self.assertTrue(
                    key_usage.digital_signature and key_usage.key_encipherment
                )

                verifier = (
                    verification.PolicyBuilder()
                    .store(verification.Store([issuer]))
                    .time(datetime.datetime.now(UTC) + datetime.timedelta(hours=1))
                    .build_server_verifier(x509.DNSName(dns_name))
                )

                if valid:
                    verifier.verify(certificate, [])
                else:
                    with self.assertRaisesRegex(
                        verification.VerificationError, "name constraint"
                    ):
                        verifier.verify(certificate, [])

                # DER as asn1crypto and pyasn1 encode it, the extensions too.
                parsed = asn1crypto_x509.Certificate.load(data)
                self.assertIsInstance(parsed.native, dict)
                self.assertEqual(parsed.dump(force=True), data)
                decoded, rest = pyasn1_decoder.decode(
                    data, asn1Spec=pyasn1_rfc5280.Certificate()
                )
                self.assertEqual(rest, b"")
                self.assertEqual(pyasn1_encoder.encode(decoded), data)

                for extension in decoded["tbsCertificate"]["extensions"]:
                    extension_type = pyasn1_rfc5280.certificateExtensionsMap.get(
                        extension["extnID"]
                    )

                    if extension_type is not None:
                        value = bytes(extension["extnValue"])
                        inner, _ = pyasn1_decoder.decode(value, asn1Spec=extension_type)
                        self.assertEqual(pyasn1_encoder.encode(inner), value)

                self.assertEqual(
                    self.specifications["mapped"].rust("Certificate", data), data
                )

    def not_der(self, name, mutation):
        certificate = parse_tlvs(self.certificates[name])[0]
        data = mutation(certificate)

        return dump_tlv(certificate) if data is None else data

    def test_not_der_is_rejected(self):
        for description, name, mutation, _, _ in NOT_DER:
            data = self.not_der(name, mutation)

            with self.subTest(description):
                with self.assertRaises(ValueError, msg="cryptography"):
                    certificate = x509.load_der_x509_certificate(data)
                    list(certificate.extensions)

                for kind, specification in self.specifications.items():
                    with self.assertRaises(asnkitrt.DecodeError, msg="Python " + kind):
                        specification.der.decode("Certificate", data)

                    with self.assertRaises(RustError, msg="Rust " + kind):
                        specification.rust("Certificate", data)

        # The unused bits of a BIT STRING, and the trailing zero bits of
        # one with named bits, which the Trustwave Global ECC roots have.
        for description, data in (
            ("unused bits set", bytes.fromhex("030205a1")),
            ("trailing zero bits", bytes.fromhex("0303070600")),
        ):
            with self.subTest(description):
                for kind, specification in self.specifications.items():
                    with self.assertRaises(asnkitrt.DecodeError, msg="Python " + kind):
                        specification.der.decode("KeyUsage", data)

                    with self.assertRaises(RustError, msg="Rust " + kind):
                        specification.rust("KeyUsage", data)

    def lenient_der(self, data, type_name="Certificate"):
        """The DER that the lenient readers of Python and of Rust make."""

        specification = self.specifications["mapped"]
        lenient = specification.lenient
        python = bytes(lenient.encode(type_name, lenient.decode(type_name, data)))
        rust = specification.rust(type_name, data, lenient=True)

        return python, rust

    def test_lenient_makes_the_same_der(self):
        for description, name, mutation, ber, same in NOT_DER:
            if not ber:
                continue

            with self.subTest(description):
                python, rust = self.lenient_der(self.not_der(name, mutation))
                self.assertEqual(python, rust)

                if same:
                    self.assertEqual(python, self.certificates[name])

        with self.subTest("KeyUsage"):
            self.assertEqual(
                self.lenient_der(bytes.fromhex("030205a1"), "KeyUsage"),
                (bytes.fromhex("030205a0"),) * 2,
            )
            self.assertEqual(
                self.lenient_der(bytes.fromhex("0303070600"), "KeyUsage"),
                (bytes.fromhex("03020106"),) * 2,
            )
            # A BIT STRING in parts, the first without bits.
            self.assertEqual(
                self.lenient_der(bytes.fromhex("2307030100030205a0"), "KeyUsage"),
                (bytes.fromhex("030205a0"),) * 2,
            )

    def test_unmapped_selector_is_rejected(self):
        """A selector that is not mapped is rejected, even when lenient,
        and is not with no mappings."""

        data = self.not_der("full-rsa", unmapped_selector)
        x509.load_der_x509_certificate(data)
        mapped = self.specifications["mapped"]

        for lenient in (False, True):
            reader = mapped.lenient if lenient else mapped.der

            with self.assertRaisesRegex(asnkitrt.DecodeError, "Bad AnyDefinedBy"):
                reader.decode("Certificate", data)

            with self.assertRaisesRegex(RustError, "Bad AnyDefinedBy"):
                mapped.rust("Certificate", data, lenient=lenient)

        plain = self.specifications["plain"]
        self.assertEqual(
            plain.der.encode("Certificate", plain.der.decode("Certificate", data)), data
        )
        self.assertEqual(plain.rust("Certificate", data), data)


if __name__ == "__main__":
    unittest.main()
