use std::io::{Read, Seek, Write};

use crate::{ArchiveReader, ArchiveResult, ArchiveWriter};

pub trait BinaryEncode {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek;
}

pub trait BinaryDecode: Sized {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek;
}
