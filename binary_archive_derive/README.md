# binary_archive_derive

Implementation of `#[derive(BinaryArchive)]`. Use it through
`binary_archive = { version = "0.2.2", features = ["derive"] }`.

Named, tuple, unit, and generic structs are supported. Fields are encoded in
declaration order inside one versioned chunk. Use
`#[binary_archive(version = 2)]` to override the default version of 1.
Enums and unions are intentionally rejected; define their codec explicitly.

Use `#[binary_archive(versioned, version = 2)]` for named, length-delimited
fields that support schema evolution. A field's `#[binary_archive(version = 1)]`
records when it was introduced; unannotated fields inherit the struct version.
Missing newer fields use `Default`, or `#[binary_archive(default = expression)]`.
`#[binary_archive(id = "stable-name")]` preserves identity across renames.
Field annotations implicitly enable this format. See the main crate's README
for deleted-field policies and the binary format specification.
