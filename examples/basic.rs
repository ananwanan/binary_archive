//! Complete typed archive example.
use binary_archive::{ArchiveReader, ArchiveResult, ArchiveWriter};
use std::fs::File;
use std::io::{BufReader, BufWriter};

const VERSION_INIT: u32 = 1;

#[derive(Debug, Default, PartialEq)]
struct Project {
    description: String,
    insert_point: [f64; 3],
    angle: f64,
    visible: bool,
    layer: Option<String>,
}

binary_archive::impl_binary_archive! {
    Project, version = VERSION_INIT, fields {
        description: String,
        insert_point: [f64; 3],
        angle: f64,
        visible: bool,
        layer: Option<String>,
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
