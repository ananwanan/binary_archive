# binary_archive

Small, deterministic binary serialization for Rust with versioned,
length-delimited chunks.

Automatic serialization is available through an optional derive macro:

```toml
[dependencies]
binary_archive = { version = "0.2.1", features = ["derive"] }
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
`reader.read`, and named-value APIs work unchanged. No `Default` is required.
Enums and unions need manual codec implementations. Generic type parameters
receive codec bounds; any additional associated-type bounds belong in your
struct's `where` clause.

`BinaryArchive` is a convenience trait automatically implemented for every type
that implements both existing codec traits. It adds `to_bytes`, `from_bytes`,
and `from_bytes_with_limit`. Rust does not reflect fields from an empty trait
implementation; the derive generates that field code. Do not also implement
the codecs or invoke `impl_binary_archive!` for the same derived type.

`from_bytes` rejects trailing bytes. For consecutive values, use
`ArchiveReader::read` once per value. A derived struct rejects unknown versions;
use manual codecs and `read_chunks` for custom schema migration logic.

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
