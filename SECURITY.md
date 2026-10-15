# Security Policy

## Supported versions

Security fixes are made for the latest release of each asnkit
package, in every language it is implemented in, such as the runtime
`asnkitrt` and the compiler `asnkitcc`. Releases with an even minor
version number (1.2, 1.4, ...) are supported for longer: a fix is also
back-ported to the two newest even minor releases before the latest
one. For example, when 1.7 is the latest, fixes go to 1.7, 1.6 and
1.4. Releases with an odd minor version number are not patched once a
newer release is out.

## Reporting a vulnerability

Please do not report a vulnerability in a public issue, pull request
or discussion.

Send the report by email to **oss.disclosure@boxtheta.com**, encrypted
with our public key:

- Key: <https://pub.boxtheta.com/OSS_Security_Disclosures_0x1301406961EF217E_public.asc>
- Fingerprint: `68AB B599 3BD6 117E 5B5B  C876 1301 4069 61EF 217E`
- Key ID: `0x1301406961EF217E`

The key is also published in DNS, signed with DNSSEC, as an
`OPENPGPKEY` record (RFC 7929). GnuPG fetches it with:

```text
gpg --auto-key-locate clear,nodefault,dane --locate-keys oss.disclosure@boxtheta.com
```

Check that the key you download has this fingerprint before you use
it. If you cannot use encryption, send a short message asking for
another way to hand over the details, and leave the details out of it.

A good report tells:

- the package, its language and its version, and the version of the
  language or the platform,
- what is wrong and what an attacker gains from it,
- steps, an input file or a short program that shows it, and
- whether it is already known to others or has a deadline.

## What to expect

- We confirm that we received the report within 3 working days.
- We tell our assessment, and whether we accept it as a vulnerability,
  within 10 working days.
- We keep you posted until it is fixed, and agree with you on the date
  of the disclosure. Our aim is to have a fix released within 90 days
  of the report.
- We credit you in the advisory and the release notes, unless you
  prefer not to be named.

## Scope

In scope are flaws in the code of this repository that can be used
against an application that uses it, for example:

- decoding untrusted data (BER, DER, PER, JER, XER, GSER and so on)
  that crashes, hangs or uses unbounded time or memory,
- loading an untrusted `.asni` or `.asnst` file in a way that runs
  code or goes beyond what the format says,
- a way around the signature, the encryption or the binding of an
  `.asni` file,
- the compiler or the command `asnkitcc` doing something unexpected
  with a crafted ASN.1 specification,
- an implementation that disagrees with the file formats in
  `docs/file_formats.md` in a way that weakens a check.

Out of scope are flaws only in a dependency or in the language runtime
(report those to it, but tell us if we need to bump a version), and
anything that needs an attacker to already control the machine or the
keys.

An application that loads `.asni` files must treat an unsigned file
as being as trusted as the place it came from. Use signing and
verification keys to load files from anywhere else.

## Safe harbor

We will not take legal action against research that is done in good
faith, stays within this policy, avoids harm to others, and does not
touch data that is not yours.
