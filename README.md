# binary_archive

Small, deterministic binary serialization for Rust with versioned,
length-delimited chunks.

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
