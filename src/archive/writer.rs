use std::io::{Seek, SeekFrom, Write};

use crate::{ArchiveResult, BinaryEncode};

pub struct ArchiveWriter<W> {
    inner: W,
}

impl<W> ArchiveWriter<W>
where
    W: Write + Seek,
{
    pub fn new(inner: W) -> Self {
        Self { inner }
    }

    /// Returns the wrapped writer after all buffered data has been flushed.
    pub fn into_inner(self) -> W {
        self.inner
    }

    pub fn write<T>(&mut self, value: &T) -> ArchiveResult<()>
    where
        T: BinaryEncode + ?Sized,
    {
        value.encode(self)
    }

    /// Writes the short Rust type name before the value as its format magic.
    pub fn write_named<T>(&mut self, value: &T) -> ArchiveResult<()>
    where
        T: BinaryEncode,
    {
        self.write(crate::type_name::<T>())?;
        self.write(value)
    }

    pub fn position(&mut self) -> ArchiveResult<u64> {
        Ok(self.inner.stream_position()?)
    }

    pub fn seek(&mut self, position: u64) -> ArchiveResult<()> {
        self.inner.seek(SeekFrom::Start(position))?;

        Ok(())
    }

    pub fn flush(&mut self) -> ArchiveResult<()> {
        self.inner.flush()?;
        Ok(())
    }

    pub(crate) fn write_raw(&mut self, data: &[u8]) -> ArchiveResult<()> {
        self.inner.write_all(data)?;
        Ok(())
    }
}
