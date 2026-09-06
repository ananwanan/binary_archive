//! `binary_archive` provides deterministic little-endian binary serialization.
//!
//! [`ChunkWriter`] and [`ChunkReader`] add versioned, length-delimited records
//! so readers can safely skip chunks introduced by newer producers.

pub mod archive;
pub mod codec;
pub mod error;

pub(crate) fn type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("unknown")
}

pub use archive::{ArchiveReader, ArchiveWriter, ChunkHeader, ChunkReader, ChunkWriter};

pub use codec::{BinaryDecode, BinaryEncode};

pub use error::{ArchiveError, ArchiveResult};
