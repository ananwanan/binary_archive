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

pub use archive::DeletedFieldPolicy;
pub use archive::{ArchiveReader, ArchiveWriter, ChunkHeader, ChunkReader, ChunkWriter};

pub use codec::{BinaryDecode, BinaryEncode};
pub use traits::BinaryArchive;

/// Derives versioned encoding and decoding for named, tuple, and unit structs.
/// Set `#[binary_archive(version = 2)]` to override the default version of 1.
/// All fields must implement the corresponding codec trait. The original
/// positional mode does not require `Default`.
///
/// `#[binary_archive(versioned, version = 2)]` enables named, versioned fields.
/// Field `version` means introduction version and defaults to the struct version.
/// Keep old fields' introduction versions explicit when increasing the struct version.
/// When reading older data, new fields use `Default` or their `default = expression`.
/// Field annotations also enable this format. `id = "stable-name"` preserves
/// identity across renames (and tuple position changes).
///
/// Deleted fields warn by default; see [`DeletedFieldPolicy`]. This opt-in
/// format requires migration of existing positional archives.
///
/// ```
/// use binary_archive::BinaryArchive;
/// #[derive(BinaryArchive)]
/// #[binary_archive(versioned, version = 1)]
/// struct V1 { name: String }
/// #[derive(BinaryArchive)]
/// #[binary_archive(versioned, version = 2)]
/// struct V2 {
///     #[binary_archive(version = 1)]
///     name: String,
///     visible: bool, // introduced in version 2
/// }
/// let bytes = V1 { name: "Demo".into() }.to_bytes()?;
/// let loaded = V2::from_bytes(&bytes)?;
/// assert_eq!(loaded.name, "Demo");
/// assert!(!loaded.visible);
/// # Ok::<(), binary_archive::ArchiveError>(())
/// ```
///
/// Field versions cannot exceed the struct version:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// struct Invalid { #[binary_archive(version = 2)] field: u32 }
/// ```
/// Field IDs must be unique:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// struct Invalid {
///     #[binary_archive(id = "same")] first: u32,
///     #[binary_archive(id = "same")] second: u32,
/// }
/// ```
/// Field versions cannot be declared twice:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// struct Invalid { #[binary_archive(version = 1, version = 1)] field: u32 }
/// ```
/// Unknown field settings are rejected:
/// ```compile_fail
/// #[derive(binary_archive::BinaryArchive)]
/// struct Invalid { #[binary_archive(unknown = 1)] field: u32 }
/// ```
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
