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

/// Generates `BinaryEncode` and `BinaryDecode` for a struct from its fields.
/// The struct must implement `Default`; fields are encoded in declaration order.
#[macro_export]
macro_rules! impl_binary_archive {
    ($ty:ty, version = $version:expr, fields { $( $field:ident : $field_ty:ty ),+ $(,)? }) => {
        impl $crate::BinaryEncode for $ty {
            fn encode<W>(&self, writer: &mut $crate::ArchiveWriter<W>) -> $crate::ArchiveResult<()>
            where W: ::std::io::Write + ::std::io::Seek {
                writer.write_chunk($version, |chunk| {
                    $( chunk.write(&self.$field)?; )+
                    Ok(())
                })?;
                Ok(())
            }
        }
        impl $crate::BinaryDecode for $ty {
            fn decode<R>(reader: &mut $crate::ArchiveReader<R>) -> $crate::ArchiveResult<Self>
            where R: ::std::io::Read + ::std::io::Seek {
                let mut value = <$ty as ::std::default::Default>::default();
                reader.read_chunks(|version, chunk| {
                    if version == $version {
                        $( value.$field = chunk.read::<$field_ty>()?; )+
                        chunk.finish()?;
                    }
                    Ok(())
                })?;
                Ok(value)
            }
        }
    };
}

pub use archive::{ArchiveReader, ArchiveWriter, ChunkHeader, ChunkReader, ChunkWriter};

pub use codec::{BinaryDecode, BinaryEncode};

pub use error::{ArchiveError, ArchiveResult};
