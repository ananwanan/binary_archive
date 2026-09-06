use std::fs::File;
use std::io::{BufReader, BufWriter, Read, Seek, Write};

use binary_archive::{
    ArchiveError, ArchiveReader, ArchiveResult, ArchiveWriter, BinaryDecode, BinaryEncode,
};

const MAGIC: &str = "Project";

const VERSION_INIT: u32 = 1;

const END_VERSION: u32 = u32::MAX;

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
        writer.write(MAGIC)?;

        writer.write(&VERSION_INIT)?;

        let length_position = writer.position()?;

        writer.write(&0u64)?;

        let data_begin = writer.position()?;

        writer.write(&self.description)?;

        writer.write(&self.insert_point)?;

        writer.write(&self.angle)?;

        writer.write(&self.visible)?;

        writer.write(&self.layer)?;

        let data_end = writer.position()?;

        let data_length = data_end - data_begin;

        writer.seek(length_position)?;

        writer.write(&data_length)?;

        writer.seek(data_end)?;

        writer.write(&END_VERSION)?;

        Ok(())
    }
}

impl BinaryDecode for Project {
    fn decode<R>(reader: &mut ArchiveReader<R>) -> ArchiveResult<Self>
    where
        R: Read + Seek,
    {
        let magic: String = reader.read()?;

        if magic != MAGIC {
            return Err(ArchiveError::InvalidMagic {
                expected: MAGIC.to_owned(),

                actual: magic,
            });
        }

        let mut result = Self::default();

        loop {
            let version: u32 = reader.read()?;

            if version == END_VERSION {
                break;
            }

            let data_length: u64 = reader.read()?;

            match version {
                VERSION_INIT => {
                    result.description = reader.read()?;

                    result.insert_point = reader.read()?;

                    result.angle = reader.read()?;

                    result.visible = reader.read()?;

                    result.layer = reader.read()?;
                }

                _ => {
                    reader.skip(data_length)?;
                }
            }
        }

        Ok(result)
    }
}

fn main() -> ArchiveResult<()> {
    let ucs = Project {
        description: "New Project".into(),

        insert_point: [100.0, 200.0, 300.0],

        angle: 45.0,

        visible: true,

        layer: Some("Default".into()),
    };

    {
        let file = File::create("project.bin")?;

        let file = BufWriter::new(file);

        let mut writer = ArchiveWriter::new(file);

        writer.write(&ucs)?;

        writer.flush()?;
    }

    let loaded = {
        let file = File::open("project.bin")?;

        let file = BufReader::new(file);

        let mut reader = ArchiveReader::new(file);

        reader.read::<Project>()?
    };

    println!("{loaded:#?}");

    assert_eq!(ucs, loaded);

    println!("serialization OK");

    Ok(())
}
