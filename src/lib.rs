//! `binary_archive` provides deterministic little-endian binary serialization.
//!
//! [`ChunkWriter`] and [`ChunkReader`] add versioned, length-delimited records
//! so readers can safely skip chunks introduced by newer producers.

// Also resolves derives used inside this crate; downstream renames are detected
// by the procedural macro from the caller's Cargo manifest.
extern crate self as binary_archive;

pub mod archive;
pub mod codec;
pub mod error;
mod traits;

pub(crate) fn type_name<T>() -> &'static str {
    std::any::type_name::<T>()
        .rsplit("::")
        .next()
        .unwrap_or("unknown")
}

/// Generates `BinaryEncode` and `BinaryDecode` for a struct from its fields.
/// The struct must implement `Default`; fields are encoded in the listed order.
/// Each value occupies one chunk. Unknown versions decode to `Default`.
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
                let mut chunk = reader.read_chunk()?;
                if chunk.header().version == $version {
                    $( value.$field = chunk.read::<$field_ty>()?; )+
                    chunk.finish()?;
                }
                Ok(value)
            }
        }
    };
}

pub use archive::{ArchiveReader, ArchiveWriter, ChunkHeader, ChunkReader, ChunkWriter};

pub use codec::{BinaryDecode, BinaryEncode};
pub use traits::BinaryArchive;

/// Derives versioned encoding and decoding for named, tuple, and unit structs.
/// Set `#[binary_archive(version = 2)]` to override the default version of 1.
/// All fields must implement the corresponding codec trait; `Default` is not required.
///
/// ```
/// use binary_archive::BinaryArchive;
/// #[derive(Debug, PartialEq, BinaryArchive)]
/// #[binary_archive(version = 1)]
/// struct Project { name: String, visible: bool }
/// let value = Project { name: "Demo".into(), visible: true };
/// assert_eq!(Project::from_bytes(&value.to_bytes()?)?, value);
/// # Ok::<(), binary_archive::ArchiveError>(())
/// ```
///
/// Enums require explicit codec implementations:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// enum Unsupported { First, Second }
/// ```
///
/// Versions must fit in a `u32`:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// #[binary_archive(version = 4294967296)]
/// struct Invalid;
/// ```
///
/// Unknown settings are rejected:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// #[binary_archive(unknown = 1)]
/// struct Invalid;
/// ```
///
/// Duplicate versions are rejected:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// #[binary_archive(version = 1, version = 2)]
/// struct Invalid;
/// ```
#[cfg(feature = "derive")]
pub use binary_archive_derive::BinaryArchive;

pub use error::{ArchiveError, ArchiveResult};
