# Versioning

All packages of asnkit are released together, with one version
`MAJOR.MINOR.PATCH`, as `RELEASING.md` says. The version tells what an
upgrade can change: the public API, or the bytes the runtimes write and
read.

| Release | Public API          | What is encoded and decoded |
|---------|---------------------|-----------------------------|
| Major   | may break           | may change                  |
| Minor   | only grows          | may change                  |
| Patch   | stays the same      | stays the same              |

## Major: 2.0.0

A major release carries the changes that break code, files or scripts
that worked with the release before it, such as:

- removing what was deprecated, in a runtime, a plugin, the compiler or
  the command line,
- renaming or removing a function, a type, a method, an argument or an
  option, or changing what it takes or returns,
- changing the code a plugin generates in a way that code using it has
  to change,
- changing the `.asni` or `.asnst` formats so that a runtime no longer
  loads a file made by an older compiler, or an older runtime no longer
  loads a file made by the new compiler,
- raising the oldest version of Python, Rust, Node.js or another
  platform a package supports.

Something is deprecated in a minor release first, with a warning where
the language has one and a line in `CHANGELOG.md`, and removed no
earlier than the next major release.

## Minor: 1.2.0

A minor release may change what is encoded and decoded, but code that
built and ran with the release before it still does. It carries:

- fixes that change the bytes a codec writes, or what a decoder accepts
  or rejects, as one that makes a codec follow its standard where it did
  not,
- new codecs, new features and new API,
- new options of the command line and of the plugins,
- deprecations,
- additions to the `.asni` and `.asnst` formats that older runtimes
  still load.

Each change to what is encoded or decoded is listed in `CHANGELOG.md`,
with the values it affects, so that an application that stores or
exchanges them can tell whether it is affected before it upgrades.

## Patch: 1.2.1

A patch release only fixes internal logic. A value is encoded to the
same bytes, and the same input is decoded to the same value or rejected,
as with the release before it. It carries fixes such as:

- a crash, a hang or a leak,
- a wrong result that does not reach what is encoded or decoded, as in
  the compiler's diagnostics or the command line's output,
- speed and memory use,
- the wording of errors and of the documentation,
- the build and the packaging.

A fix that would change what is encoded or decoded waits for the next
minor release, however small it is. The one exception is a security
fix that cannot be made otherwise: it may change what a decoder
rejects, in a patch release and in a back-port to an older minor
release (see `SECURITY.md`), and `CHANGELOG.md` says so.

## Release candidates

A release candidate uses `MAJOR.MINOR.PATCH-rcN`, for example
`2.0.0-rc1`, in Git tags, Cargo and npm. Python uses the equivalent
PEP 440 spelling `2.0.0rc1`. Candidates may still change before the
stable release. Install them explicitly: npm publishes them under
`next`, and GitHub marks them as prereleases.

## Earlier releases

This policy applies to the releases after 1.1.0. 1.0.1 changed what
the runtimes encode, in a patch release, before it was in place.
