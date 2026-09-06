use std::io::{
    Read,
    Seek,
    Write,
};

use crate::{
    ArchiveReader,
    ArchiveResult,
    ArchiveWriter,
};

pub trait BinarySerialize {
    fn serialize<W>(
        &self,
        writer: &mut ArchiveWriter<W>,
    ) -> ArchiveResult<()>
    where
        W: Write + Seek;
}

pub trait BinaryDeserialize: Sized {
    fn deserialize<R>(
        reader: &mut ArchiveReader<R>,
    ) -> ArchiveResult<Self>
    where
        R: Read + Seek;
}