EXPECTED = {
    "PKCS-10": {
        "extensibility-implied": False,
        "imports": {
            "AuthenticationFramework": ["ALGORITHM"],
            "InformationFramework": ["ATTRIBUTE", "Name"],
            "UsefulDefinitions": ["authenticationFramework", "informationFramework"],
        },
        "object-classes": {},
        "object-sets": {
            "CRIAttributes": {"class": "ATTRIBUTE", "members": [None]},
            "PKInfoAlgorithms": {"class": "ALGORITHM", "members": [None]},
            "SignatureAlgorithms": {"class": "ALGORITHM", "members": [None]},
        },
        "tags": "IMPLICIT",
        "types": {
            "AlgorithmIdentifier": {
                "members": [
                    {
                        "name": "algorithm",
                        "table": {"type": "IOSet"},
                        "type": "ALGORITHM.&id",
                    },
                    {
                        "name": "parameters",
                        "optional": True,
                        "table": ["IOSet", ["algorithm"]],
                        "type": "ALGORITHM.&Type",
                    },
                ],
                "parameter-governors": [{"type": "ALGORITHM"}],
                "parameters": ["IOSet"],
                "type": "SEQUENCE",
            },
            "Attribute": {
                "members": [
                    {
                        "name": "type",
                        "table": {"type": "IOSet"},
                        "type": "ATTRIBUTE.&id",
                    },
                    {
                        "element": {
                            "table": ["IOSet", ["type"]],
                            "type": "ATTRIBUTE.&Type",
                        },
                        "name": "values",
                        "size": [(1, "MAX")],
                        "type": "SET OF",
                    },
                ],
                "parameter-governors": [{"type": "ATTRIBUTE"}],
                "parameters": ["IOSet"],
                "type": "SEQUENCE",
            },
            "Attributes": {
                "element": {
                    "actual-parameters": [{"object-set": ["IOSet"]}],
                    "type": "Attribute",
                },
                "parameter-governors": [{"type": "ATTRIBUTE"}],
                "parameters": ["IOSet"],
                "type": "SET OF",
            },
            "CertificationRequest": {
                "members": [
                    {
                        "name": "certificationRequestInfo",
                        "type": "CertificationRequestInfo",
                    },
                    {
                        "actual-parameters": [{"object-set": ["SignatureAlgorithms"]}],
                        "name": "signatureAlgorithm",
                        "type": "AlgorithmIdentifier",
                    },
                    {"name": "signature", "type": "BIT STRING"},
                ],
                "type": "SEQUENCE",
            },
            "CertificationRequestInfo": {
                "members": [
                    {
                        "name": "version",
                        "named-numbers": {"v1": 0},
                        "restricted-to": ["v1", None],
                        "type": "INTEGER",
                    },
                    {"name": "subject", "type": "Name"},
                    {
                        "actual-parameters": [{"object-set": ["PKInfoAlgorithms"]}],
                        "name": "subjectPKInfo",
                        "type": "SubjectPublicKeyInfo",
                    },
                    {
                        "actual-parameters": [{"object-set": ["CRIAttributes"]}],
                        "name": "attributes",
                        "tag": {"number": 0},
                        "type": "Attributes",
                    },
                ],
                "type": "SEQUENCE",
            },
            "SubjectPublicKeyInfo": {
                "members": [
                    {
                        "actual-parameters": [{"object-set": ["IOSet"]}],
                        "name": "algorithm",
                        "type": "AlgorithmIdentifier",
                    },
                    {"name": "subjectPublicKey", "type": "BIT STRING"},
                ],
                "parameter-governors": [{"type": "ALGORITHM"}],
                "parameters": ["IOSet"],
                "type": "SEQUENCE",
            },
        },
        "values": {},
    }
}
