# Changelog

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
