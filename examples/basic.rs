//! Complete typed archive example.
use binary_archive::{ArchiveReader, ArchiveResult, ArchiveWriter, BinaryDecode, BinaryEncode};
use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};

const VERSION_INIT: u32 = 1;

#[derive(Debug, Default, PartialEq)]
struct Project {
    description: String,
    insert_point: [f64; 3],
    angle: f64,
    visible: bool,
    layer: Option<String>,
}

impl BinaryEncode for Project {
    fn encode<W>(&self, writer: &mut ArchiveWriter<W>) -> ArchiveResult<()>
    where
        W: Write + Seek,
    {
        writer.write_chunk(VERSION_INIT, |chunk| {
            chunk.write(&self.description)?;
            chunk.write(&self.insert_point)?;
            chunk.write(&self.angle)?;
            chunk.write(&self.visible)?;
            chunk.write(&self.layer)
        })?;
        Ok(())
    }
}

impl BinaryDecode for Project {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let mut result = Self::default();
        reader.read_chunks(|version, chunk| {
            match version {
                VERSION_INIT => {
                    result.description = chunk.read()?;
                    result.insert_point = chunk.read()?;
                    result.angle = chunk.read()?;
                    result.visible = chunk.read()?;
                    result.layer = chunk.read()?;
                    chunk.finish()?;
                }
                _ => {} // Unknown chunk payload is skipped for forward compatibility.
            }
            Ok(())
        })?;
        Ok(result)
    }
}

fn main() -> ArchiveResult<()> {
    let project = Project {
        description: "New Project".into(),
        insert_point: [100.0, 200.0, 300.0],
        angle: 45.0,
        visible: true,
        layer: Some("Default".into()),
    };
    {
        let file = BufWriter::new(File::create("project.bin")?);
        let mut writer = ArchiveWriter::new(file);
        writer.write_named(&project)?;
        writer.flush()?;
    }
    let file = BufReader::new(File::open("project.bin")?);
    let mut reader = ArchiveReader::new(file);
    let loaded = reader.read_named::<Project>()?;
    assert_eq!(project, loaded);
    println!("{loaded:#?}\nserialization OK");
    Ok(())
}
