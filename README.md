# binary_archive

Small, deterministic binary serialization for Rust with versioned,
length-delimited chunks.

Automatic serialization is available through an optional derive macro:

```toml
[dependencies]
binary_archive = { version = "0.2.2", features = ["derive"] }
```

```rust
use binary_archive::BinaryArchive;

#[derive(Debug, PartialEq, BinaryArchive)]
#[binary_archive(version = 1)]
struct Project {
    name: String,
    visible: bool,
}

let project = Project { name: "Demo".into(), visible: true };
let bytes = project.to_bytes()?;
let loaded = Project::from_bytes(&bytes)?;
assert_eq!(project, loaded);
# Ok::<(), binary_archive::ArchiveError>(())
```

The derive supports named, tuple, unit, and generic structs. It writes fields in
declaration order inside one chunk, using version 1 unless overridden. It
generates `BinaryEncode` and `BinaryDecode`, so the existing `writer.write`,
`reader.read`, and named-value APIs work unchanged. This original positional
mode does not require `Default`.
Enums and unions need manual codec implementations. Generic type parameters
receive codec bounds; any additional associated-type bounds belong in your
struct's `where` clause.

`BinaryArchive` is a convenience trait automatically implemented for every type
that implements both existing codec traits. It adds `to_bytes`, `from_bytes`,
`from_bytes_with_limit`, and `from_bytes_with_policy`. Rust does not reflect
fields from an empty trait implementation; the derive generates that field code. Do not also implement
the codecs or invoke `impl_binary_archive!` for the same derived type.

`from_bytes` rejects trailing bytes. For consecutive values, use
`ArchiveReader::read` once per value. Positional derived structs reject unknown
versions; use the field-versioned mode below for schema evolution.

## Field versions

Enable the new format on every schema version, including version 1:

```rust
use binary_archive::{BinaryArchive, DeletedFieldPolicy};

#[derive(BinaryArchive)]
#[binary_archive(versioned, version = 1)]
struct V1 {
    name: String,
    obsolete: u32,
}

#[derive(BinaryArchive)]
#[binary_archive(versioned, version = 2)]
struct V2 {
    #[binary_archive(version = 1)]
    name: String,
    visible: bool, // No annotation: introduced in version 2.
    #[binary_archive(default = String::from("not set"))]
    description: String, // Also introduced in version 2.
}

let bytes = V1 { name: "Demo".into(), obsolete: 42 }.to_bytes()?;
let value = V2::from_bytes(&bytes)?; // Warns about obsolete, then skips it.
assert_eq!(value.name, "Demo");
assert!(!value.visible);
assert_eq!(value.description, "not set");

// Return an error for deleted fields, or use Panic to panic deliberately.
assert!(V2::from_bytes_with_policy(&bytes, DeletedFieldPolicy::Error).is_err());
# Ok::<(), binary_archive::ArchiveError>(())
```

- A field's `version` is its **introduction version**, not the version when its
  value last changed. Without an annotation it inherits the struct version.
  When increasing the struct version, annotate existing fields with their
  original versions. Changing an existing field's introduction version is an error.
- When the stored struct version predates a field, the decoder does not attempt
  to read it. It uses the field type's `Default`, or the specified
  `default = expression`. The struct itself need not implement `Default`.
  A missing field that should already exist is treated as malformed data.
- Field names are stable IDs, so fields may be reordered or deleted. Use
  `#[binary_archive(id = "original_name")]` when renaming a field. Tuple fields
  default to their index as a string; give them explicit IDs before changing
  tuple positions. IDs must remain unique and must not be reused for unrelated data.
- Unknown stored fields introduced at or before the reader's schema version
  are treated as deleted. `DeletedFieldPolicy::Warn` is the default; `Error`
  returns `ArchiveError::InvalidData`, and `Panic` invokes `panic!`.
  Unknown fields introduced after the reader's version are silently skipped.
- `ArchiveReader::with_deleted_field_policy` configures the same policy for
  streaming reads. `with_warning_handler(fn(&str))` redirects warnings from
  stderr. Both settings propagate through nested structs and chunks.
- Field annotations implicitly enable this format. `versioned` explicitly
  enables it even when all fields inherit the struct version. Derives without
  either, and `impl_binary_archive!`, retain the original format and behavior.

This format cannot directly decode existing positional files written by 0.2.1:
decode them with their original schema and re-encode into a versioned type.
The versioned format does not infer type conversions; keep a field's type
compatible or implement explicit migration. Named-value I/O still requires the
same short Rust type name across versions.

The outer chunk header is unchanged. Its payload begins with the eight bytes
`BARFLD01`, followed by zero or more field records until the end of the chunk.
Each field contains `u64 name_byte_length`, UTF-8 name bytes, `u32 introduced_in`,
`u64 payload_byte_length`, and the normal encoded field value, all numbers in
little-endian order. Readers validate duplicate names, introduction versions,
and exact payload lengths. There is no field count or terminator.

Run `cargo run --example versioned --features derive` for a complete example.

## Original positional API

For ordinary structs, `impl_binary_archive!` generates the serialization and
deserialization implementations from a field list. The struct must implement
`Default`:

```rust
binary_archive::impl_binary_archive! {
    Project, version = 1, fields {
        name: String,
        visible: bool,
    }
}
```

API documentation: <https://docs.ananwanan.cc/binary_archive/>.

Use `write_named(&value)` and `read_named::<Value>()` to put the short Rust
struct name (for example `Project`) into the file as the magic field. A mismatch
returns `InvalidMagic` before the payload is decoded.

Run the demos with `cargo run --example basic` and
`cargo run --example chunks`.

The intended high-level API is:

```rust
writer.write_chunk(1, |writer| {
    writer.write(&mesh)?;
    writer.write(&material)
})?;

reader.read_chunks(|version, reader| {
    match version {
        1 => { /* decode the version 1 payload */ }
        2 => { /* decode the version 2 payload */ }
        _ => {} // unread data is skipped automatically
    }
    Ok(())
})?;
```

The chunk header is 12 bytes: `u32 version` and `u64 payload_length`,
all little-endian. Readers have a 64 MiB default allocation limit; configure it
with `ArchiveReader::with_max_allocation` for trusted larger data.
The limit applies to each individual buffer (including collection element
storage) and is inherited by nested chunks; it is not a cumulative memory or
recursion budget. Keep a finite limit when decoding untrusted input.

The existing `impl_binary_archive!` syntax and encoded bytes are unchanged.
Each generated decoder now reads exactly one chunk, allowing nested and
consecutive values. Unknown versions still produce `Default`. Empty or
truncated input is an error. Use `read_chunks` explicitly when processing a
whole stream of versioned chunks.

Run the automatic serialization example with
`cargo run --example derive --features derive`.
