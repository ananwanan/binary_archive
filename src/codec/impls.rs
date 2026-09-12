use std::io::{Read, Seek, Write};

use crate::{
    ArchiveError, ArchiveReader, ArchiveResult, ArchiveWriter, BinaryDecode, BinaryEncode,
};

macro_rules! impl_number {
    ($ty:ty) => {
        impl BinaryEncode for $ty {
            fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
            where
                W: Write + Seek,
            {
                writer.write_raw(&self.to_le_bytes())
            }
        }

        impl BinaryDecode for $ty {
            fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
            where
                R: Read + Seek,
            {
                let mut buffer = [0u8; std::mem::size_of::<$ty>()];

                reader.read_raw(&mut buffer)?;

                Ok(<$ty>::from_le_bytes(buffer))
            }
        }
    };
}

impl_number!(u16);
impl_number!(u32);
impl_number!(u64);

impl_number!(i16);
impl_number!(i32);
impl_number!(i64);

impl_number!(f32);
impl_number!(f64);

impl BinaryEncode for u8 {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        writer.write_raw(&[*self])
    }
}

impl BinaryDecode for u8 {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let mut buffer = [0u8; 1];

        reader.read_raw(&mut buffer)?;

        Ok(buffer[0])
    }
}

impl BinaryEncode for i8 {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        writer.write_raw(&[*self as u8])
    }
}

impl BinaryDecode for i8 {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let value: u8 = reader.read()?;

        Ok(value as i8)
    }
}

impl BinaryEncode for bool {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        let value: u8 = if *self { 1 } else { 0 };

        writer.write(&value)
    }
}

impl BinaryDecode for bool {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let value: u8 = reader.read()?;

        match value {
            0 => Ok(false),
            1 => Ok(true),

            _ => Err(ArchiveError::InvalidData(format!(
                "invalid bool value: {value}"
            ))),
        }
    }
}

impl BinaryEncode for String {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        self.as_str().encode(writer)
    }
}

impl BinaryEncode for str {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        let bytes = self.as_bytes();

        let length = bytes.len() as u64;

        writer.write(&length)?;

        writer.write_raw(bytes)
    }
}

impl BinaryDecode for String {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let length: u64 = reader.read()?;

        let bytes = reader.read_bytes(length, "string")?;

        String::from_utf8(bytes)
            .map_err(|err| ArchiveError::InvalidData(format!("invalid UTF-8: {err}")))
    }
}

impl<T> BinaryEncode for Option<T>
where
    T: BinaryEncode,
{
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        match self {
            Some(value) => {
                writer.write(&1u8)?;
                writer.write(value)?;
            }

            None => {
                writer.write(&0u8)?;
            }
        }

        Ok(())
    }
}

impl<T> BinaryDecode for Option<T>
where
    T: BinaryDecode,
{
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let marker: u8 = reader.read()?;

        match marker {
            0 => Ok(None),

            1 => {
                let value = reader.read()?;

                Ok(Some(value))
            }

            _ => Err(ArchiveError::InvalidData(format!(
                "invalid Option marker: {marker}"
            ))),
        }
    }
}

impl<T> BinaryEncode for Vec<T>
where
    T: BinaryEncode,
{
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        let count = self.len() as u64;

        writer.write(&count)?;

        for value in self {
            writer.write(value)?;
        }

        Ok(())
    }
}

impl<T> BinaryDecode for Vec<T>
where
    T: BinaryDecode,
{
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let count: u64 = reader.read()?;

        let count = reader.checked_count::<T>(count)?;

        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|err| ArchiveError::InvalidData(format!("cannot allocate vector: {err}")))?;

        for _ in 0..count {
            values.push(reader.read()?);
        }

        Ok(values)
    }
}

impl<T, const N: usize> BinaryEncode for [T; N]
where
    T: BinaryEncode,
{
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        for value in self {
            writer.write(value)?;
        }

        Ok(())
    }
}
impl<T, const N: usize> BinaryDecode for [T; N]
where
    T: BinaryDecode,
{
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        reader.checked_count::<T>(N as u64)?;
        let mut values = Vec::new();
        values.try_reserve_exact(N).map_err(|err| {
            ArchiveError::InvalidData(format!("cannot allocate array buffer: {err}"))
        })?;

        for _ in 0..N {
            values.push(reader.read()?);
        }

        values
            .try_into()
            .map_err(|_| ArchiveError::InvalidData(format!("failed to create array of length {N}")))
    }
}

impl<T> BinaryEncode for Box<T>
where
    T: BinaryEncode,
{
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        writer.write(self.as_ref())
    }
}

impl<T> BinaryDecode for Box<T>
where
    T: BinaryDecode,
{
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        Ok(Box::new(reader.read()?))
    }
}
