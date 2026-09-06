use std::io::{Read, Seek, SeekFrom};

use crate::{ArchiveResult, BinaryDecode};

pub struct ArchiveReader<R> {
    inner: R,
    max_allocation: u64,
}

impl<R> ArchiveReader<R>
where
    R: Read + Seek,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            max_allocation: 64 * 1024 * 1024,
        }
    }

    /// Sets the maximum number of bytes a length-prefixed value may allocate.
    pub fn with_max_allocation(mut self, max_bytes: u64) -> Self {
        self.max_allocation = max_bytes;
        self
    }

    pub(crate) fn checked_length(&self, length: u64, kind: &'static str) -> ArchiveResult<usize> {
        if length > self.max_allocation {
            return Err(crate::ArchiveError::LimitExceeded {
                kind,
                value: length,
                limit: self.max_allocation,
            });
        }
        usize::try_from(length).map_err(|_| crate::ArchiveError::LengthOverflow { length })
    }

    pub fn read<T>(&mut self) -> ArchiveResult<T>
    where
        T: BinaryDecode,
    {
        T::decode(self)
    }

    /// Reads and validates the short Rust type name before decoding the value.
    pub fn read_named<T>(&mut self) -> ArchiveResult<T>
    where
        T: BinaryDecode,
    {
        let actual: String = self.read()?;
        let expected = crate::type_name::<T>();
        if actual != expected {
            return Err(crate::ArchiveError::InvalidMagic {
                expected: expected.to_owned(),
                actual,
            });
        }
        self.read()
    }

    pub fn position(&mut self) -> ArchiveResult<u64> {
        Ok(self.inner.stream_position()?)
    }

    pub(crate) fn end_position(&mut self) -> ArchiveResult<u64> {
        Ok(self.inner.seek(SeekFrom::End(0))?)
    }

    pub fn seek(&mut self, position: u64) -> ArchiveResult<()> {
        self.inner.seek(SeekFrom::Start(position))?;

        Ok(())
    }

    pub fn skip(&mut self, length: u64) -> ArchiveResult<()> {
        let position = self
            .position()?
            .checked_add(length)
            .ok_or_else(|| crate::ArchiveError::InvalidData("seek position overflow".into()))?;

        self.seek(position)
    }

    pub(crate) fn read_raw(&mut self, data: &mut [u8]) -> ArchiveResult<()> {
        self.inner.read_exact(data)?;
        Ok(())
    }
}
