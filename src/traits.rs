use std::io::Cursor;

use crate::{
    ArchiveError, ArchiveReader, ArchiveResult, ArchiveWriter, BinaryDecode, BinaryEncode,
};

/// Convenience methods automatically available to every encodable, decodable type.
///
/// Enable the `derive` feature and use `#[derive(BinaryArchive)]` to generate
/// both codec implementations for a struct. Existing manual implementations and
/// `impl_binary_archive!` also receive these methods automatically.
///
/// ```
/// use binary_archive::BinaryArchive;
/// let bytes = vec![1u32, 2, 3].to_bytes()?;
/// assert_eq!(Vec::<u32>::from_bytes(&bytes)?, vec![1, 2, 3]);
/// # Ok::<(), binary_archive::ArchiveError>(())
/// ```
pub trait BinaryArchive: BinaryEncode + BinaryDecode {
    /// Encodes one value into an owned byte buffer, without a type-name prefix.
    fn to_bytes(&self) -> ArchiveResult<Vec<u8>> {
        let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
        writer.write(self)?;
        Ok(writer.into_inner().into_inner())
    }

    /// Decodes exactly one value with the default 64 MiB allocation limit.
    /// Trailing bytes are rejected; use `ArchiveReader` for a stream of values.
    fn from_bytes(bytes: &[u8]) -> ArchiveResult<Self> {
        Self::from_bytes_with_limit(bytes, 64 * 1024 * 1024)
    }

    /// Decodes exactly one value using a custom per-allocation limit in bytes.
    fn from_bytes_with_limit(bytes: &[u8], max_bytes: u64) -> ArchiveResult<Self> {
        let mut reader = ArchiveReader::new(Cursor::new(bytes)).with_max_allocation(max_bytes);
        let value = reader.read()?;
        if reader.position()? != bytes.len() as u64 {
            return Err(ArchiveError::InvalidData(
                "trailing bytes after value".into(),
            ));
        }
        Ok(value)
    }
}

impl<T: BinaryEncode + BinaryDecode> BinaryArchive for T {}
