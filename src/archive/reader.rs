use std::io::{Read, Seek, SeekFrom};

use crate::{ArchiveResult, BinaryDecode};

pub struct ArchiveReader<R> {
    inner: R,
    pub(crate) max_allocation: u64,
    pub(crate) options: super::policy::DecodeOptions,
}

impl<R> ArchiveReader<R>
where
    R: Read + Seek,
{
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            max_allocation: 64 * 1024 * 1024,
            options: super::policy::DecodeOptions::default(),
        }
    }

    /// Sets the maximum number of bytes a single buffer may allocate.
    /// Nested chunk readers inherit this limit. This is not a total-memory budget.
    pub fn with_max_allocation(mut self, max_bytes: u64) -> Self {
        self.max_allocation = max_bytes;
        self
    }

    /// Sets the handling of deleted fields in versioned structs, including nested values.
    pub fn with_deleted_field_policy(mut self, policy: super::DeletedFieldPolicy) -> Self {
        self.options.deleted_fields = policy;
        self
    }

    /// Redirects schema warnings to a function instead of standard error.
    /// The handler is inherited by all nested chunk readers.
    pub fn with_warning_handler(mut self, handler: fn(&str)) -> Self {
        self.options.warning_handler = Some(handler);
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
        let position = self.position()?;
        let end = self.inner.seek(SeekFrom::End(0))?;
        self.seek(position)?;
        Ok(end)
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

    pub(crate) fn read_bytes(&mut self, length: u64, kind: &'static str) -> ArchiveResult<Vec<u8>> {
        let size = self.checked_length(length, kind)?;
        let mut bytes = Vec::new();
        bytes.try_reserve_exact(size).map_err(|err| {
            crate::ArchiveError::InvalidData(format!("cannot allocate {kind}: {err}"))
        })?;
        bytes.resize(size, 0);
        self.read_raw(&mut bytes)?;
        Ok(bytes)
    }

    pub(crate) fn checked_count<T>(&self, count: u64) -> ArchiveResult<usize> {
        // Count zero-sized elements too, to bound the number of decoder calls.
        let size = std::mem::size_of::<T>().max(1) as u64;
        let bytes = count
            .checked_mul(size)
            .ok_or(crate::ArchiveError::LengthOverflow { length: count })?;
        self.checked_length(bytes, "collection allocation")?;
        usize::try_from(count).map_err(|_| crate::ArchiveError::LengthOverflow { length: count })
    }
}
