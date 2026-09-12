//! Versioned, length-delimited records for forward-compatible archives.

use crate::{
    ArchiveError, ArchiveReader, ArchiveResult, ArchiveWriter, BinaryDecode, BinaryEncode,
};
use std::io::{Read, Seek, Write};

/// Fixed header written before every chunk: `version (u32)`, `length (u64)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkHeader {
    /// Schema version of this chunk payload.
    pub version: u32,
    /// Payload size in bytes, excluding this header.
    pub length: u64,
}

/// A streaming writer for one chunk payload.
pub struct ChunkWriter<'a, W> {
    archive: &'a mut ArchiveWriter<W>,
    header: ChunkHeader,
    length_pos: u64,
    payload_pos: u64,
}

impl<'a, W: Write + Seek> ChunkWriter<'a, W> {
    /// Writes a value into this chunk.
    pub fn write<T: BinaryEncode + ?Sized>(&mut self, value: &T) -> ArchiveResult<()> {
        self.archive.write(value)
    }

    /// Writes a named, length-delimited field with its introduction version.
    #[doc(hidden)]
    pub fn write_versioned_field<T: BinaryEncode + ?Sized>(
        &mut self,
        name: &str,
        version: u32,
        value: &T,
    ) -> ArchiveResult<()> {
        self.archive.write(name)?;
        self.archive
            .write_chunk(version, |chunk| chunk.write(value))?;
        Ok(())
    }

    /// Completes the chunk and patches its payload length in the header.
    pub fn finish(mut self) -> ArchiveResult<ChunkHeader> {
        let end = self.archive.position()?;
        let length = end
            .checked_sub(self.payload_pos)
            .ok_or_else(|| ArchiveError::InvalidData("chunk position moved backwards".into()))?;
        self.archive.seek(self.length_pos)?;
        self.archive.write(&length)?;
        self.archive.seek(end)?;
        self.header.length = length;
        Ok(self.header)
    }
}

/// Starts, writes and completes one chunk. The length is patched only after the closure succeeds.
impl<W: Write + Seek> ArchiveWriter<W> {
    /// Starts a chunk and returns a streaming payload writer.
    ///
    /// Prefer [`Self::write_chunk`] when possible: it makes completion explicit
    /// and prevents accidentally leaving an unpatched length field behind.
    pub fn begin_chunk(&mut self, version: u32) -> ArchiveResult<ChunkWriter<'_, W>> {
        self.write(&version)?;
        let length_pos = self.position()?;
        self.write(&0u64)?;
        let payload_pos = self.position()?;
        Ok(ChunkWriter {
            archive: self,
            header: ChunkHeader { version, length: 0 },
            length_pos,
            payload_pos,
        })
    }

    /// Writes a complete chunk using a closure and returns its header.
    pub fn write_chunk<F>(&mut self, version: u32, f: F) -> ArchiveResult<ChunkHeader>
    where
        F: FnOnce(&mut ChunkWriter<'_, W>) -> ArchiveResult<()>,
    {
        let mut chunk = self.begin_chunk(version)?;
        f(&mut chunk)?;
        chunk.finish()
    }
}

/// A chunk payload loaded with its exact length, preventing reads into the next chunk.
pub struct ChunkReader {
    header: ChunkHeader,
    payload: std::io::Cursor<Vec<u8>>,
    max_allocation: u64,
    options: super::policy::DecodeOptions,
}

impl ChunkReader {
    /// Returns the chunk metadata.
    pub fn header(&self) -> ChunkHeader {
        self.header
    }

    /// Reads one value from the current payload position.
    pub fn read<T: BinaryDecode>(&mut self) -> ArchiveResult<T> {
        let mut reader =
            ArchiveReader::new(&mut self.payload).with_max_allocation(self.max_allocation);
        reader.options = self.options;
        T::decode(&mut reader)
    }

    /// Iterates the named fields of a versioned struct, validating their metadata.
    #[doc(hidden)]
    pub fn read_versioned_fields<F>(&mut self, mut f: F) -> ArchiveResult<()>
    where
        F: FnMut(&str, u32, &mut ChunkReader) -> ArchiveResult<()>,
    {
        let marker: [u8; 8] = self.read()?;
        if &marker != b"BARFLD01" {
            return Err(ArchiveError::InvalidData(
                "expected versioned field format; migrate legacy positional data first".into(),
            ));
        }
        let mut names = std::collections::HashSet::new();
        while self.remaining() != 0 {
            let mut reader =
                ArchiveReader::new(&mut self.payload).with_max_allocation(self.max_allocation);
            reader.options = self.options;
            let name: String = reader.read()?;
            let mut field = reader.read_chunk()?;
            let version = field.header().version;
            if version > self.header.version {
                return Err(ArchiveError::InvalidData(format!(
                    "field {name:?} version {version} exceeds stored struct version {}",
                    self.header.version
                )));
            }
            names.try_reserve(1).map_err(|err| {
                ArchiveError::InvalidData(format!("cannot allocate field index: {err}"))
            })?;
            if !names.insert(name.clone()) {
                return Err(ArchiveError::InvalidData(format!(
                    "duplicate field {name:?}"
                )));
            }
            f(&name, version, &mut field)?;
        }
        self.finish()
    }

    /// Applies the inherited policy to a stored field missing from the local schema.
    #[doc(hidden)]
    pub fn handle_deleted_field(
        &self,
        struct_name: &str,
        field_name: &str,
        target_version: u32,
    ) -> ArchiveResult<()> {
        if self.header.version > target_version {
            return Ok(());
        }
        self.options.deleted_field(format!("stored field {struct_name}.{field_name} (introduced in version {}) is absent from schema version {target_version}; field discarded", self.header.version))
    }
    /// Returns the number of unread payload bytes.
    pub fn remaining(&self) -> u64 {
        (self.payload.get_ref().len() as u64).saturating_sub(self.payload.position())
    }
    /// Requires that the decoder consumed the complete payload.
    pub fn finish(&self) -> ArchiveResult<()> {
        if self.payload.position() > self.payload.get_ref().len() as u64 {
            return Err(ArchiveError::InvalidData(
                "decoder moved beyond chunk payload".into(),
            ));
        }
        if self.remaining() == 0 {
            Ok(())
        } else {
            Err(ArchiveError::ChunkNotFullyConsumed {
                remaining: self.remaining(),
            })
        }
    }

    /// Discards unread payload bytes so iteration can continue with the next chunk.
    pub fn skip_remaining(&mut self) {
        self.payload
            .set_position(self.payload.get_ref().len() as u64);
    }
}

impl<R: Read + Seek> ArchiveReader<R> {
    /// Reads the next chunk, including exactly its declared payload bytes.
    pub fn read_chunk(&mut self) -> ArchiveResult<ChunkReader> {
        let version = self.read()?;
        let length: u64 = self.read()?;
        let payload = self.read_bytes(length, "chunk payload")?;
        Ok(ChunkReader {
            header: ChunkHeader { version, length },
            payload: std::io::Cursor::new(payload),
            max_allocation: self.max_allocation,
            options: self.options,
        })
    }

    /// Reads and discards the next chunk without allocating its payload.
    pub fn skip_chunk(&mut self) -> ArchiveResult<ChunkHeader> {
        let version = self.read()?;
        let length: u64 = self.read()?;
        let target = self
            .position()?
            .checked_add(length)
            .ok_or_else(|| ArchiveError::InvalidData("chunk end position overflow".into()))?;
        if target > self.end_position()? {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "truncated chunk payload",
            )
            .into());
        }
        self.seek(target)?;
        Ok(ChunkHeader { version, length })
    }

    /// Reads every chunk until the end of the stream.
    ///
    /// Each callback receives the chunk schema version and a reader bounded to
    /// that chunk's payload. Unknown versions can be ignored safely by simply
    /// not reading from the callback reader.
    pub fn read_chunks<F>(&mut self, mut f: F) -> ArchiveResult<()>
    where
        F: FnMut(u32, &mut ChunkReader) -> ArchiveResult<()>,
    {
        let end = self.end_position()?;
        while self.position()? < end {
            let mut chunk = self.read_chunk()?;
            let version = chunk.header().version;
            f(version, &mut chunk)?;
            // Unknown versions may intentionally leave their payload unread.
            chunk.skip_remaining();
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn chunks_round_trip_and_skip() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ArchiveWriter::new(&mut cursor);
            let first = writer.write_chunk(3, |chunk| chunk.write("hello")).unwrap();
            let second = writer
                .write_chunk(7, |chunk| chunk.write(&1234u32))
                .unwrap();
            assert_eq!(first.length, 13);
            assert_eq!(second.length, 4);
        }

        cursor.set_position(0);
        let mut reader = ArchiveReader::new(cursor);
        let mut chunk = reader.read_chunk().unwrap();
        assert_eq!(chunk.read::<String>().unwrap(), "hello");
        chunk.finish().unwrap();
        assert_eq!(reader.skip_chunk().unwrap().version, 7);
    }

    #[test]
    fn chunk_payload_is_bounded() {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&100u64.to_le_bytes());
        bytes.extend_from_slice(&[0; 3]);
        let mut reader = ArchiveReader::new(Cursor::new(bytes));
        assert!(matches!(reader.read_chunk(), Err(ArchiveError::Io(_))));
    }

    #[test]
    fn named_values_validate_type_name() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = ArchiveWriter::new(&mut cursor);
            writer.write_named(&123u32).unwrap();
        }
        cursor.set_position(0);
        let mut reader = ArchiveReader::new(cursor);
        assert_eq!(reader.read_named::<u32>().unwrap(), 123);
    }
}
