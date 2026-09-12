# Changelog

## 0.2.2

- Add opt-in field versioning with `#[binary_archive(versioned, version = N)]`.
  Field annotations also enable the format. Unannotated fields inherit the
  struct version; existing fields must keep their original introduction version.
- Newer schemas read older records by defaulting fields introduced later.
  Each field supports `default = expression`, otherwise its type needs `Default`.
- Store stable field IDs, introduction versions, and payload lengths so fields
  can be reordered or removed without shifting subsequent reads. Explicit
  `id = "..."` supports renamed fields and stable tuple positions.
- Warn by default when a stored field is absent from the current schema. Add
  `DeletedFieldPolicy::{Warn, Error, Panic}`, reader policy/warning-handler
  configuration, and `BinaryArchive::from_bytes_with_policy`. Nested reads
  inherit these settings. Fields introduced after the reader schema are skipped.
- Reject duplicate IDs, mismatched introduction versions, missing existing
  fields, malformed lengths, and truncated payloads.
- Keep the original positional derive/macro behavior and bytes unchanged.
  Existing positional files require explicit migration to the new field format.
- Add cross-version examples, integration tests, and invalid-attribute doctests.

## 0.2.1

- Add the `BinaryArchive` convenience trait with `to_bytes`, `from_bytes`, and
  `from_bytes_with_limit`, automatically available to existing codec types.
- Add optional `derive` support for named, tuple, unit, generic, and recursive
  structs; generate both codec implementations without requiring `Default`.
- Preserve existing public signatures, macro syntax, and encoded bytes.
- Fix `impl_binary_archive!` decoding to consume one value instead of the entire
  stream, enabling sequential and nested records. Unknown versions still
  default; missing/truncated records now return errors.
- Propagate allocation limits into nested chunks, account for collection
  element sizes, check multiplication overflow, and report allocation failures.
- Reject truncated/overflowing chunks when skipping without allocation.
- Prevent `ChunkReader::remaining` from underflowing after a custom decoder
  seeks beyond the payload, and reject that state in `finish`.
- Preserve wrapped I/O errors through `Error::source` and correct the
  `ArchiveWriter::into_inner` documentation (it does not flush).
- Add compatibility, malformed-input, and derive tests plus Windows/Linux CI.
