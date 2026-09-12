# binary_archive_derive

Implementation of `#[derive(BinaryArchive)]`. Use it through
`binary_archive = { version = "0.2.1", features = ["derive"] }`.

Named, tuple, unit, and generic structs are supported. Fields are encoded in
declaration order inside one versioned chunk. Use
`#[binary_archive(version = 2)]` to override the default version of 1.
Enums and unions are intentionally rejected; define their codec explicitly.
