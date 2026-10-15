# Copyright (c) 2025 BoxTheta
#
# SPDX-License-Identifier: Apache-2.0

"""Checks the DER codec with X.509 against OpenSSL, which is not written
by the same people nor from the same reading of the standard.

- Certificates and CRLs made by OpenSSL are decoded and encoded again,
  and must come out byte for byte as OpenSSL wrote them.

- What is encoded from values must be accepted by OpenSSL, and a
  certificate made from nothing but values and a signature by OpenSSL
  must verify with it.

The tests are skipped if there is no ``openssl`` to run.

"""

import datetime
import os
import shutil
import subprocess
import tempfile
import unittest

import asnkitrt

OPENSSL = shutil.which("openssl")

# Which compiled specification of rfc5280 is tried. A file made by another
# compiler can be given, and what made it expected, to tell it was that
# file that was tried:
#
#   ASNKIT_RFC5280_ASNI=/tmp/rfc5280.asni ASNKIT_EXPECT_GENERATOR=Rust \
#       python -m unittest tests.test_openssl_x509
RFC5280_ASNI_PATH = os.environ.get(
    "ASNKIT_RFC5280_ASNI", os.path.join("examples", "compiled", "rfc5280.asni")
)
EXPECT_GENERATOR = os.environ.get("ASNKIT_EXPECT_GENERATOR")

# The type of the value of each extension that is decoded.
EXTENSION_TYPES = {
    "2.5.29.9": "SubjectDirectoryAttributes",
    "2.5.29.14": "SubjectKeyIdentifier",
    "2.5.29.15": "KeyUsage",
    "2.5.29.17": "SubjectAltName",
    "2.5.29.18": "IssuerAltName",
    "2.5.29.19": "BasicConstraints",
    "2.5.29.20": "CRLNumber",
    "2.5.29.30": "NameConstraints",
    "2.5.29.31": "CRLDistributionPoints",
    "2.5.29.32": "CertificatePolicies",
    "2.5.29.35": "AuthorityKeyIdentifier",
    "2.5.29.36": "PolicyConstraints",
    "2.5.29.37": "ExtKeyUsageSyntax",
    "2.5.29.54": "InhibitAnyPolicy",
    "1.3.6.1.5.5.7.1.1": "AuthorityInfoAccessSyntax",
}

# The attributes of a name holding a DirectoryString.
DIRECTORY_STRING_ATTRIBUTES = {
    "2.5.4.3",  # commonName
    "2.5.4.7",  # localityName
    "2.5.4.8",  # stateOrProvinceName
    "2.5.4.10",  # organizationName
    "2.5.4.11",  # organizationalUnitName
}

EXTENSIONS = (
    "basicConstraints=critical,CA:TRUE,pathlen:3",
    "keyUsage=critical,digitalSignature,keyCertSign,cRLSign",
    "extendedKeyUsage=serverAuth,clientAuth,codeSigning,1.2.3.4",
    "subjectAltName=DNS:a.example,DNS:*.b.example,IP:10.1.2.3,"
    "IP:2001:db8::1,email:x@y.example,URI:https://z.example/p,"
    "RID:1.2.3.4,otherName:1.2.3.4;UTF8:foo",
    "nameConstraints=permitted;DNS:.example,permitted;email:.example,"
    "excluded;IP:10.0.0.0/255.0.0.0",
    "certificatePolicies=1.2.3.4,2.23.140.1.2.1",
    "crlDistributionPoints=URI:http://crl.example/a.crl,URI:http://crl.example/b.crl",
    "authorityInfoAccess=OCSP;URI:http://ocsp.example,caIssuers;URI:http://ca.example/ca.crt",
    "issuerAltName=DNS:issuer.example",
    "policyConstraints=requireExplicitPolicy:1,inhibitPolicyMapping:2",
    "subjectKeyIdentifier=hash",
    "authorityKeyIdentifier=keyid:always",
)

# Subject, extra extensions, serial and validity of each certificate
# made, with the name of what is special about it.
SUBJECTS = {
    "plain": "/C=SE/O=Foo/CN=bar",
    "unicode": "/C=PL/O=Zażółć/CN=gęślą jaźń",
    "multivalued": "/C=SE/O=Foo/OU=a+CN=b+L=c",
    "email": "/C=SE/CN=bar/emailAddress=bar@foo.example",
    "serial_number": "/C=SE/CN=bar/serialNumber=1234",
    "long": "/CN=" + "x" * 60 + "/O=" + "y" * 60,
}


def run(*args, input=None):
    return subprocess.run(args, input=input, check=True, capture_output=True).stdout


def make_certificate(
    directory, name, key_args, subject, extensions=(), days=30, **kwargs
):
    """Returns the DER of a self-signed certificate."""

    key = os.path.join(directory, name + ".key")
    der = os.path.join(directory, name + ".der")
    command = [
        OPENSSL,
        "req",
        "-x509",
        "-nodes",
        "-days",
        str(days),
        "-keyout",
        key,
        "-outform",
        "DER",
        "-out",
        der,
        "-subj",
        subject,
    ]
    command += key_args

    for extension in extensions:
        command += ["-addext", extension]

    for option, value in kwargs.items():
        command += ["-" + option, value]

    subprocess.run(command, check=True, capture_output=True)

    with open(der, "rb") as fin:
        return fin.read()


@unittest.skipIf(OPENSSL is None, "openssl is not installed")
class CompiledSpecificationTest(unittest.TestCase):
    def test_the_file_tried_is_the_one_asked_for(self):
        with open(RFC5280_ASNI_PATH, "rb") as fin:
            header = asnkitrt.syntax.read_header(fin.read())

        if EXPECT_GENERATOR is not None:
            self.assertIn(EXPECT_GENERATOR, header.metadata.generator)


@unittest.skipIf(OPENSSL is None, "openssl is not installed")
class OpenSslX509Test(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        cls.rfc5280 = asnkitrt.load_asni_file(RFC5280_ASNI_PATH, "der")
        cls.certificates = {}

        keys = {
            "rsa2048": ["-newkey", "rsa:2048"],
            "rsa3072_sha512": ["-newkey", "rsa:3072", "-sha512"],
            "rsa_pss": [
                "-newkey",
                "rsa-pss",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
                "-sigopt",
                "rsa_padding_mode:pss",
            ],
            "p256": ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1"],
            "p384": ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:secp384r1"],
            "p521": ["-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:secp521r1"],
            "ed25519": ["-newkey", "ed25519"],
            "ed448": ["-newkey", "ed448"],
        }

        for key_name, key_args in keys.items():
            cls.make(
                "key_" + key_name,
                key_args,
                SUBJECTS["plain"],
                ("subjectAltName=DNS:a.example",),
            )

        for subject_name, subject in SUBJECTS.items():
            cls.make("subject_" + subject_name, keys["p256"], subject)

        # All the extensions at once, and each alone.
        cls.make("extensions_all", keys["p256"], SUBJECTS["plain"], EXTENSIONS)

        for extension in EXTENSIONS:
            cls.make(
                "extension_{}".format(extension.split("=")[0]),
                keys["p256"],
                SUBJECTS["plain"],
                (extension,),
            )

        # An empty subject, which needs a critical subjectAltName.
        cls.make(
            "empty_subject",
            keys["p256"],
            "/",
            ("subjectAltName=critical,DNS:a.example",),
        )

        # Serial numbers of one byte, of 20 bytes and with the most
        # significant bit set, which is padded with a zero byte.
        for serial in ["0x01", "0x7f", "0x80", "0xff", "0x8000", "0x" + "ff" * 20]:
            cls.make(
                "serial_" + serial[2:6],
                keys["p256"],
                SUBJECTS["plain"],
                set_serial=serial,
            )

        # Dates before 2050 are UTCTime and from 2050 GeneralizedTime.
        for name, not_before, not_after in [
            ("utc", "20200101000000Z", "20491231235959Z"),
            ("generalized", "20500101000000Z", "20600101000000Z"),
            ("mixed", "20200101000000Z", "20600101000000Z"),
        ]:
            cls.make(
                "validity_" + name,
                keys["p256"],
                SUBJECTS["plain"],
                not_before=not_before,
                not_after=not_after,
            )

    @classmethod
    def make(cls, name, key_args, subject, extensions=(), **kwargs):
        try:
            cls.certificates[name] = make_certificate(
                cls.directory.name, name, key_args, subject, extensions, **kwargs
            )
        except subprocess.CalledProcessError as error:
            # Not every OpenSSL has every option and algorithm. What
            # is not made is told by the tests that use it.
            cls.certificates[name] = error.stderr.decode(errors="replace")

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def certificates_made(self):
        """Yields the name and DER of each certificate OpenSSL made."""

        for name, der in sorted(self.certificates.items()):
            if isinstance(der, bytes):
                yield name, der

    def test_all_certificates_were_made(self):
        # A few failing is no problem, but if all do the tests tell
        # nothing.
        made = [name for name, _ in self.certificates_made()]
        failed = {
            name: error
            for name, error in self.certificates.items()
            if isinstance(error, str)
        }

        self.assertGreater(len(made), 20, failed)

        # Those that every OpenSSL 3 makes.
        for name in ["key_rsa2048", "key_p256", "key_ed25519", "extensions_all"]:
            self.assertIn(name, made, failed.get(name))

    def test_round_trip(self):
        """What OpenSSL writes is decoded and encoded to the same bytes."""

        for name, der in self.certificates_made():
            with self.subTest(certificate=name):
                decoded = self.rfc5280.decode("Certificate", der)
                self.assertEqual(self.rfc5280.encode("Certificate", decoded), der)

    def test_round_trip_with_constraints_checked(self):
        for name, der in self.certificates_made():
            with self.subTest(certificate=name):
                decoded = self.rfc5280.decode("Certificate", der)
                self.assertEqual(
                    self.rfc5280.encode("Certificate", decoded, check_constraints=True),
                    der,
                )

    def test_round_trip_of_the_parts(self):
        """The to-be-signed part, which is what is signed, comes out as
        it went in, and so does the length of the whole.

        """

        for name, der in self.certificates_made():
            with self.subTest(certificate=name):
                decoded = self.rfc5280.decode("Certificate", der)
                tbs = self.rfc5280.encode("TBSCertificate", decoded["tbsCertificate"])
                self.assertIn(tbs, der)
                self.assertEqual(
                    self.rfc5280.decode("TBSCertificate", tbs),
                    decoded["tbsCertificate"],
                )
                self.assertEqual(self.rfc5280.decode_length(der), len(der))
                self.assertEqual(self.rfc5280.decode_length(der[:5]), len(der))
                self.assertIsNone(self.rfc5280.decode_length(der[:1]))

    def test_round_trip_of_extensions_and_names(self):
        """The contents of the open types: the value of each extension
        and of each attribute of a name.

        """

        seen = set()

        for name, der in self.certificates_made():
            with self.subTest(certificate=name):
                tbs = self.rfc5280.decode("Certificate", der)["tbsCertificate"]

                for extension in tbs.get("extensions", []):
                    type_name = EXTENSION_TYPES.get(extension["extnID"])

                    if type_name is None:
                        continue

                    seen.add(type_name)
                    value = bytes(extension["extnValue"])
                    decoded = self.rfc5280.decode(type_name, value)
                    self.assertEqual(
                        self.rfc5280.encode(type_name, decoded),
                        value,
                        (extension["extnID"], type_name),
                    )

                for field in ["issuer", "subject"]:
                    for rdn in tbs[field][1]:
                        for attribute in rdn:
                            if attribute["type"] not in DIRECTORY_STRING_ATTRIBUTES:
                                continue

                            value = bytes(attribute["value"])
                            decoded = self.rfc5280.decode("DirectoryString", value)
                            self.assertEqual(
                                self.rfc5280.encode("DirectoryString", decoded),
                                value,
                            )

        # Make sure the extensions were in the certificates to begin
        # with.
        self.assertGreaterEqual(len(seen), 10, seen)

    def test_values_are_what_openssl_tells(self):
        """Compare what is decoded with what OpenSSL prints."""

        der = self.certificates["key_p256"]
        tbs = self.rfc5280.decode("Certificate", der)["tbsCertificate"]
        serial = run(OPENSSL, "x509", "-inform", "DER", "-noout", "-serial", input=der)
        self.assertEqual(int(serial.split(b"=")[1], 16), tbs["serialNumber"])
        self.assertEqual(tbs["version"], 2)

        dates = run(
            OPENSSL, "x509", "-inform", "DER", "-noout", "-dates", input=der
        ).decode()
        not_before = datetime.datetime.strptime(
            dates.split("notBefore=")[1].split("\n")[0], "%b %d %H:%M:%S %Y GMT"
        )
        not_after = datetime.datetime.strptime(
            dates.split("notAfter=")[1].split("\n")[0], "%b %d %H:%M:%S %Y GMT"
        )
        self.assertEqual(tbs["validity"]["notBefore"], ("utcTime", not_before))
        self.assertEqual(tbs["validity"]["notAfter"], ("utcTime", not_after))

        # The big serial number, with its most significant bit set.
        der = self.certificates["serial_ffff"]
        self.assertIsInstance(der, bytes, der)
        tbs = self.rfc5280.decode("Certificate", der)["tbsCertificate"]
        self.assertEqual(tbs["serialNumber"], int("ff" * 20, 16))

        # The times from 2050.
        der = self.certificates.get("validity_generalized")

        if isinstance(der, bytes):
            tbs = self.rfc5280.decode("Certificate", der)["tbsCertificate"]
            self.assertEqual(
                tbs["validity"],
                {
                    "notBefore": ("generalTime", datetime.datetime(2050, 1, 1)),
                    "notAfter": ("generalTime", datetime.datetime(2060, 1, 1)),
                },
            )

    def test_openssl_accepts_what_is_encoded(self):
        """What is encoded from decoded values, changed, parses and
        shows as changed in OpenSSL.

        """

        der = self.certificates["extensions_all"]
        self.assertIsInstance(der, bytes, der)
        decoded = self.rfc5280.decode("Certificate", der)
        decoded["tbsCertificate"]["serialNumber"] = 0x0123456789ABCDEF0123456789
        decoded["tbsCertificate"]["subject"] = (
            "rdnSequence",
            [
                [{"type": "2.5.4.3", "value": b"\x0c\x05hello"}],
                [{"type": "2.5.4.10", "value": b"\x0c\x05world"}],
            ],
        )
        decoded["tbsCertificate"]["validity"] = {
            "notBefore": ("utcTime", datetime.datetime(2030, 1, 2, 3, 4, 5)),
            "notAfter": ("generalTime", datetime.datetime(2070, 6, 7, 8, 9, 10)),
        }
        encoded = self.rfc5280.encode("Certificate", decoded)
        run(OPENSSL, "asn1parse", "-inform", "DER", input=encoded)
        text = run(
            OPENSSL, "x509", "-inform", "DER", "-noout", "-text", input=encoded
        ).decode()
        self.assertIn("0123456789abcdef0123456789", text.lower().replace(":", ""))
        self.assertIn("CN=hello", text.replace(" = ", "="))
        self.assertIn("Jan  2 03:04:05 2030 GMT", text)
        self.assertIn("Jun  7 08:09:10 2070 GMT", text)

        # Every extension is still printed.
        self.assertIn("X509v3 Basic Constraints: critical", text)
        self.assertIn("Name Constraints", text)

    def test_certificate_made_of_values_verifies(self):
        """A certificate made of nothing but values, signed by OpenSSL
        over the bytes encoded, verifies with OpenSSL.

        """

        directory = self.directory.name
        key = os.path.join(directory, "built.key")
        run(OPENSSL, "ecparam", "-name", "prime256v1", "-genkey", "-noout", "-out", key)
        public_key = run(OPENSSL, "pkey", "-in", key, "-pubout", "-outform", "DER")
        subject_public_key_info = self.rfc5280.decode(
            "SubjectPublicKeyInfo", public_key
        )

        # The encoding of what is decoded is that of OpenSSL, as a
        # check of the parts.
        self.assertEqual(
            self.rfc5280.encode("SubjectPublicKeyInfo", subject_public_key_info),
            public_key,
        )

        name = (
            "rdnSequence",
            [
                [{"type": "2.5.4.6", "value": b"\x13\x02SE"}],
                [{"type": "2.5.4.10", "value": b"\x0c\x03Foo"}],
                [{"type": "2.5.4.3", "value": b"\x0c\x05Built"}],
            ],
        )
        ecdsa_with_sha256 = {"algorithm": "1.2.840.10045.4.3.2"}
        now = datetime.datetime.now(datetime.UTC).replace(tzinfo=None, microsecond=0)
        extensions = [
            {
                "extnID": "2.5.29.19",
                "critical": True,
                "extnValue": self.rfc5280.encode(
                    "BasicConstraints", {"cA": True, "pathLenConstraint": 2}
                ),
            },
            {
                "extnID": "2.5.29.15",
                "critical": True,
                # keyCertSign and cRLSign.
                "extnValue": self.rfc5280.encode("KeyUsage", (b"\x06", 7)),
            },
            {
                "extnID": "2.5.29.17",
                "critical": False,
                "extnValue": self.rfc5280.encode(
                    "SubjectAltName",
                    [("dNSName", "built.example"), ("iPAddress", b"\x0a\x00\x00\x01")],
                ),
            },
        ]
        tbs = {
            "version": 2,
            "serialNumber": 2**100 + 12345,
            "signature": ecdsa_with_sha256,
            "issuer": name,
            "validity": {
                "notBefore": ("utcTime", now - datetime.timedelta(days=1)),
                "notAfter": ("utcTime", now + datetime.timedelta(days=1)),
            },
            "subject": name,
            "subjectPublicKeyInfo": subject_public_key_info,
            "extensions": extensions,
        }
        encoded_tbs = self.rfc5280.encode("TBSCertificate", tbs, check_constraints=True)
        tbs_file = os.path.join(directory, "built.tbs")

        with open(tbs_file, "wb") as fout:
            fout.write(encoded_tbs)

        signature = run(OPENSSL, "dgst", "-sha256", "-sign", key, tbs_file)
        certificate = self.rfc5280.encode(
            "Certificate",
            {
                "tbsCertificate": tbs,
                "signatureAlgorithm": ecdsa_with_sha256,
                "signature": (signature, 8 * len(signature)),
            },
            check_constraints=True,
        )
        der_file = os.path.join(directory, "built.der")
        pem_file = os.path.join(directory, "built.pem")

        with open(der_file, "wb") as fout:
            fout.write(certificate)

        run(OPENSSL, "asn1parse", "-inform", "DER", "-in", der_file)
        run(OPENSSL, "x509", "-inform", "DER", "-in", der_file, "-out", pem_file)
        output = run(
            OPENSSL, "verify", "-check_ss_sig", "-CAfile", pem_file, pem_file
        ).decode()
        self.assertIn("OK", output)

        # The one signed and then changed does not verify, which shows
        # that verifying is checking something.
        tbs["serialNumber"] += 1
        changed = self.rfc5280.encode(
            "Certificate",
            {
                "tbsCertificate": tbs,
                "signatureAlgorithm": ecdsa_with_sha256,
                "signature": (signature, 8 * len(signature)),
            },
        )

        with open(der_file, "wb") as fout:
            fout.write(changed)

        run(OPENSSL, "x509", "-inform", "DER", "-in", der_file, "-out", pem_file)
        result = subprocess.run(
            [OPENSSL, "verify", "-check_ss_sig", "-CAfile", pem_file, pem_file],
            capture_output=True,
        )
        self.assertNotEqual(result.returncode, 0)

    def test_cut_short_certificate_is_rejected(self):
        der = self.certificates["key_p256"]

        with self.assertRaises(asnkitrt.DecodeError):
            self.rfc5280.decode("Certificate", der[:-1])

    def test_trailing_data_is_rejected(self):
        der = self.certificates["key_p256"]

        with self.assertRaises(asnkitrt.DecodeError):
            self.rfc5280.decode("Certificate", der + b"\x00")

        # The first of several values can still be had.
        decoded, length = self.rfc5280.types["Certificate"].decode_with_length(
            der + der
        )
        self.assertEqual(length, len(der))
        self.assertEqual(self.rfc5280.encode("Certificate", decoded), der)

    def test_length_not_minimal_is_rejected_by_der(self):
        der = self.certificates["key_p256"]

        # The length of the outer sequence with a leading zero byte.
        header_length = 2 if der[1] < 0x80 else 2 + (der[1] & 0x7F)
        long_form = b"\x30\x83\x00" + der[2:]
        self.assertEqual(long_form[header_length + 1 :], der[header_length:])

        with self.assertRaises(asnkitrt.DecodeError):
            self.rfc5280.decode("Certificate", long_form)

        # The length of the version, which is 3, in the long form.
        i = der.index(b"\xa0\x03\x02\x01\x02")
        changed = der[:i] + b"\xa0\x81\x03\x02\x01\x02" + der[i + 5 :]
        changed = b"\x30\x82" + (len(changed) - 4).to_bytes(2, "big") + changed[4:]

        with self.assertRaises(asnkitrt.DecodeError):
            self.rfc5280.decode("Certificate", changed)

    def test_indefinite_length_is_rejected_by_der(self):
        """DER only has definite lengths, so BER with an indefinite one
        is not DER. Accepting it would let two different encodings be
        the same certificate.

        """

        der = self.certificates["key_p256"]
        header_length = 2 if der[1] < 0x80 else 2 + (der[1] & 0x7F)
        ber = b"\x30\x80" + der[header_length:] + b"\x00\x00"

        with self.assertRaises(asnkitrt.DecodeError):
            self.rfc5280.decode("Certificate", ber)


@unittest.skipIf(OPENSSL is None, "openssl is not installed")
class OpenSslCrlTest(unittest.TestCase):
    """A CRL with entries, made with ``openssl ca``."""

    @classmethod
    def setUpClass(cls):
        cls.directory = tempfile.TemporaryDirectory()
        directory = cls.directory.name
        cls.rfc5280 = asnkitrt.load_asni_file(RFC5280_ASNI_PATH, "der")
        configuration = os.path.join(directory, "ca.cnf")

        with open(configuration, "w") as fout:
            fout.write(
                """
[ca]
default_ca = CA_default

[CA_default]
database = {d}/index.txt
new_certs_dir = {d}
serial = {d}/serial
crlnumber = {d}/crlnumber
default_md = sha256
default_crl_days = 30
default_days = 30
policy = policy_any
certificate = {d}/ca.pem
private_key = {d}/ca.key
unique_subject = no
crl_extensions = crl_ext

[policy_any]
commonName = supplied

[crl_ext]
authorityKeyIdentifier = keyid:always
""".format(d=directory)
            )

        for name, content in [
            ("index.txt", ""),
            ("serial", "1000\n"),
            ("crlnumber", "01\n"),
        ]:
            with open(os.path.join(directory, name), "w") as fout:
                fout.write(content)

        def openssl(*args):
            return subprocess.run(
                [OPENSSL] + list(args), check=True, capture_output=True, cwd=directory
            )

        openssl(
            "req", "-x509", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1",
            "-nodes", "-keyout", "ca.key", "-out", "ca.pem", "-subj", "/CN=ca",
            "-days", "30", "-addext", "basicConstraints=critical,CA:TRUE",
        )  # fmt: skip

        reasons = ["keyCompromise", "CACompromise", "affiliationChanged", "superseded",
                   "cessationOfOperation", "certificateHold", "unspecified"]  # fmt: skip

        for i, reason in enumerate(reasons):
            name = "leaf{}".format(i)
            openssl(
                "req", "-newkey", "ec", "-pkeyopt", "ec_paramgen_curve:prime256v1",
                "-nodes", "-keyout", name + ".key", "-out", name + ".csr",
                "-subj", "/CN=" + name,
            )  # fmt: skip
            openssl("ca", "-config", "ca.cnf", "-batch", "-in", name + ".csr",
                    "-out", name + ".pem", "-notext")  # fmt: skip
            openssl("ca", "-config", "ca.cnf", "-revoke", name + ".pem",
                    "-crl_reason", reason)  # fmt: skip

        openssl("ca", "-config", "ca.cnf", "-gencrl", "-out", "crl.pem")
        openssl("crl", "-in", "crl.pem", "-outform", "DER", "-out", "crl.der")

        with open(os.path.join(directory, "crl.der"), "rb") as fin:
            cls.crl = fin.read()

        with open(os.path.join(directory, "ca.pem"), "rb") as fin:
            cls.ca = fin.read()

    @classmethod
    def tearDownClass(cls):
        cls.directory.cleanup()

    def test_round_trip(self):
        decoded = self.rfc5280.decode("CertificateList", self.crl)
        self.assertEqual(self.rfc5280.encode("CertificateList", decoded), self.crl)
        self.assertEqual(
            self.rfc5280.encode("CertificateList", decoded, check_constraints=True),
            self.crl,
        )

    def test_entries_are_what_openssl_tells(self):
        decoded = self.rfc5280.decode("CertificateList", self.crl)
        revoked = decoded["tbsCertList"]["revokedCertificates"]
        self.assertEqual(len(revoked), 7)
        self.assertEqual(
            [entry["userCertificate"] for entry in revoked], list(range(0x1000, 0x1007))
        )

        text = run(OPENSSL, "crl", "-inform", "DER", "-noout", "-text", input=self.crl)
        self.assertEqual(text.count(b"Serial Number:"), 7)

        # The reason of each entry.
        reasons = []

        for entry in revoked:
            for extension in entry["crlEntryExtensions"]:
                if extension["extnID"] == "2.5.29.21":
                    value = bytes(extension["extnValue"])
                    reason = self.rfc5280.decode("CRLReason", value)
                    self.assertEqual(self.rfc5280.encode("CRLReason", reason), value)
                    reasons.append(reason)

        self.assertEqual(
            reasons,
            [
                "keyCompromise",
                "cACompromise",
                "affiliationChanged",
                "superseded",
                "cessationOfOperation",
                "certificateHold",
                "unspecified",
            ],
        )

    def test_changed_crl_is_accepted_by_openssl(self):
        decoded = self.rfc5280.decode("CertificateList", self.crl)
        decoded["tbsCertList"]["revokedCertificates"].append(
            {
                "userCertificate": 0xABCDEF,
                "revocationDate": ("utcTime", datetime.datetime(2031, 2, 3, 4, 5, 6)),
            }
        )
        encoded = self.rfc5280.encode("CertificateList", decoded)
        run(OPENSSL, "asn1parse", "-inform", "DER", input=encoded)
        text = run(OPENSSL, "crl", "-inform", "DER", "-noout", "-text", input=encoded)
        self.assertIn(b"ABCDEF", text.upper())
        self.assertEqual(text.count(b"Serial Number:"), 8)


if __name__ == "__main__":
    unittest.main()
