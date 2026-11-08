# asnkitrt (JVM)

The asnkit runtime for the JVM, written in Kotlin, for Kotlin, Java and
the other languages of the JVM. It loads the `.asni` files produced by
`asnkitcc` and uses them to encode and decode ASN.1. It contains no
parser, and needs Java 17 or later.

The codecs are BER, DER, JER and XER. It can also check values against
their constraints. The other codecs of the runtime for Rust (OER, PER,
UPER and GSER) are not done yet.

```kotlin
import org.boxtheta.asnkitrt.Codec
import org.boxtheta.asnkitrt.Options
import org.boxtheta.asnkitrt.Specification
import org.boxtheta.asnkitrt.Value
import java.io.File

val foo = Specification.load(File("foo.asni"), Options(Codec.DER))
val question = Value.Map("id" to Value.Integer(1), "question" to Value.Text("Is 1+1=3?"))
val encoded = foo.encode("Question", question)  // 30 0e 02 01 01 16 09 49 73 ...
val decoded = foo.decode("Question", encoded)

foo.checkConstraints("Question", decoded)
```

From Java:

```java
Specification foo = Specification.load(new File("foo.asni"), new Options(Codec.DER, false, false));
Value decoded = foo.decode("Question", encoded);
```

It is published to Maven Central as `org.boxtheta:asnkitrt`, with the
version of the other packages of asnkit:

```kotlin
dependencies {
    implementation("org.boxtheta:asnkitrt:2.0.0-rc7")
}
```

- **Values** are `Value`s, as in the runtimes for Python and Rust: a
  `Value.Map` for a `SEQUENCE`, a `Value.Choice` for a `CHOICE`, and so
  on. An `INTEGER` is a `Value.Integer` of a `BigInteger`, of any size. An
  `OBJECT IDENTIFIER` is a `Value.ObjectIdentifier`, made with
  `Value.ObjectIdentifier.parse("1.2.840.113549")`. Dates and times are
  those of `java.time`, held to the microsecond, with their offset from
  UTC in minutes, or none for a local time.
- **Corrupt files** do not load: the CRC-32 in the header of an `.asni`
  file is checked before any of it is parsed; see `Asni.readHeader`.
- **Untrusted input** fails with an `AsnkitException` and nothing else,
  in time and memory proportional to the input, with nesting limited:
  the CBOR of a file to 64 levels by default (`Checks`), and values to 128.
- **Errors** tell where in the value they are: an `EncodeException`,
  a `DecodeException` or a `ConstraintsException` has a `location`, and
  its message is as `Question.question: Expected data of type str.`.
- **Indented**: `encodeIndented` writes JER and XER with spaces for each
  level, as the runtime for Python does.
- **Thread-safe**: a `Specification` does not change after it is loaded.

## Dependencies

[Jackson](https://github.com/FasterXML/jackson)
(`jackson-dataformat-cbor`, with `jackson-core`) reads the CBOR of the
`.asni` files and the JSON of JER. The reader checks each CBOR item
against what `docs/file_formats.md` allows, as Jackson takes more: no
indefinite lengths, no byte strings but those of bignums, no other
tags, no simple values but `false`, `true` and `null`, and only text as
the keys of maps.
[Woodstox](https://github.com/FasterXML/woodstox) (`woodstox-core`,
with `stax2-api`) reads the XML of XER, with no document type
declaration, and so no entity but the five of XML, nor any that is
fetched, and elements nested 128 levels at most. Kotlin's standard
library is the only other dependency.

## Building

It is a Gradle build of its own, with the wrapper:

```text
cd packages/asnkitrt-jvm
./gradlew build
```

The tests read the vectors and the examples of the repository, and so
run from a checkout of it.

## Limitations

Not supported: `ANY` and open types in JER and XER, which need their type, and
the types `EMBEDDED PDV` and `CHARACTER STRING`. These return an error;
none produces a wrong value.

It is held to the vectors in
[`tests/vectors`](https://github.com/boxtheta/asnkit/blob/trunk/tests/vectors)
of the runtime for Rust, the one of reference: all of those of BER, DER,
JER and XER, including constraint checks, give the same bytes and values.
`SpecificationTest` decodes and encodes again the X.509 certificates the
runtime for Rust checks against OpenSSL.

Licensed under the MIT License. See `LICENSE-RUNTIMES`.
