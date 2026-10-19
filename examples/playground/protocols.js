export const protocols = [
  {
    name: 'SNMPv1 get request',
    type: 'Message',
    description: 'A request for sysDescr.0. This subset covers request and response PDUs, without traps.',
    specification: `SNMP DEFINITIONS IMPLICIT TAGS ::= BEGIN

    Message ::= SEQUENCE {
        version    INTEGER { version-1(0) },
        community  OCTET STRING,
        data       PDUs
    }

    PDUs ::= CHOICE {
        get-request       [0] PDU,
        get-next-request  [1] PDU,
        get-response      [2] PDU,
        set-request       [3] PDU
    }

    PDU ::= SEQUENCE {
        request-id         INTEGER,
        error-status       INTEGER {
            noError(0), tooBig(1), noSuchName(2),
            badValue(3), readOnly(4), genErr(5)
        },
        error-index        INTEGER,
        variable-bindings  SEQUENCE OF VarBind
    }

    VarBind ::= SEQUENCE {
        name   OBJECT IDENTIFIER,
        value  ObjectSyntax
    }

    ObjectSyntax ::= CHOICE {
        number     INTEGER,
        string     OCTET STRING,
        object     OBJECT IDENTIFIER,
        empty      NULL,
        address    [APPLICATION 0] OCTET STRING (SIZE (4)),
        counter    [APPLICATION 1] INTEGER (0..4294967295),
        gauge      [APPLICATION 2] INTEGER (0..4294967295),
        ticks      [APPLICATION 3] INTEGER (0..4294967295),
        arbitrary  [APPLICATION 4] OCTET STRING
    }

END
`,
    value: {
      version: 0,
      community: new TextEncoder().encode('public'),
      data: ['get-request', {
        'request-id': 1,
        'error-status': 0,
        'error-index': 0,
        'variable-bindings': [{ name: '1.3.6.1.2.1.1.1.0', value: ['empty', null] }],
      }],
    },
  },
  {
    name: 'X.509 certificate (DER)',
    type: 'Certificate',
    description: 'A real Ed25519 certificate fixture with the core RFC 5280 certificate types. ANY fields stay opaque; unsupported encodings report errors. Decoding does not verify the signature.',
    specification: `X509 DEFINITIONS EXPLICIT TAGS ::= BEGIN

    Certificate ::= SEQUENCE {
        tbsCertificate      TBSCertificate,
        signatureAlgorithm  AlgorithmIdentifier,
        signature           BIT STRING
    }

    TBSCertificate ::= SEQUENCE {
        version               [0] Version DEFAULT v1,
        serialNumber          INTEGER,
        signature             AlgorithmIdentifier,
        issuer                Name,
        validity              Validity,
        subject               Name,
        subjectPublicKeyInfo  SubjectPublicKeyInfo,
        issuerUniqueID        [1] IMPLICIT BIT STRING OPTIONAL,
        subjectUniqueID       [2] IMPLICIT BIT STRING OPTIONAL,
        extensions            [3] Extensions OPTIONAL
    }

    Version ::= INTEGER { v1(0), v2(1), v3(2) }

    AlgorithmIdentifier ::= SEQUENCE {
        algorithm   OBJECT IDENTIFIER,
        parameters  ANY OPTIONAL
    }

    Name ::= CHOICE {
        rdnSequence  RDNSequence
    }

    RDNSequence ::= SEQUENCE OF RelativeDistinguishedName

    RelativeDistinguishedName ::= SET SIZE (1..MAX) OF AttributeTypeAndValue

    AttributeTypeAndValue ::= SEQUENCE {
        type   OBJECT IDENTIFIER,
        value  ANY
    }

    Validity ::= SEQUENCE {
        notBefore  Time,
        notAfter   Time
    }

    Time ::= CHOICE {
        utcTime      UTCTime,
        generalTime  GeneralizedTime
    }

    SubjectPublicKeyInfo ::= SEQUENCE {
        algorithm         AlgorithmIdentifier,
        subjectPublicKey  BIT STRING
    }

    Extensions ::= SEQUENCE SIZE (1..MAX) OF Extension

    Extension ::= SEQUENCE {
        extnID     OBJECT IDENTIFIER,
        critical   BOOLEAN DEFAULT FALSE,
        extnValue  OCTET STRING
    }

END
`,
    codec: 'der',
    input: `3082017d3082012fa00302010202147f9a141bdddbee1a90cd3e7d389276
5e9bfaad5c300506032b65703029310b3009060355040613025345310c30
0a060355040a0c03466f6f310c300a06035504030c03626172301e170d32
36313031343132353234375a170d3236313131333132353234375a302931
0b3009060355040613025345310c300a060355040a0c03466f6f310c300a
06035504030c03626172302a300506032b6570032100773a5e3ff5c21339
b7f334a3aaf096e7f21de8e0155c75c710d5927a36921d52a3693067301d
0603551d0e0416041450e9713bc315ea67006cb70287de1277e2f5882b30
1f0603551d2304183016801450e9713bc315ea67006cb70287de1277e2f5
882b300f0603551d130101ff040530030101ff30140603551d11040d300b
8209612e6578616d706c65300506032b6570034100e436e8f97c8d249317
573a760334077f06398387ecc8109df4d5776c8466d1a52e918a94335021
85da1709c7da14672d6f8cb57beb847f26ce3e49f924e53502`,
  },
];
