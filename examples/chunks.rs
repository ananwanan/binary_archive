//! A forward-compatible archive with versioned chunks.
use binary_archive::{ArchiveReader, ArchiveResult, ArchiveWriter};
use std::io::Cursor;

fn main() -> ArchiveResult<()> {
    let mut bytes = Cursor::new(Vec::new());
    {
        let mut writer = ArchiveWriter::new(&mut bytes);
        writer.write_chunk(1, |chunk| {
            chunk.write("World")?;
            chunk.write(&[100.0_f64, 200.0, 300.0])
        })?;
        writer.write_chunk(2, |chunk| chunk.write("created by v2"))?;
        writer.flush()?;
    }

    bytes.set_position(0);
    let mut reader = ArchiveReader::new(bytes);
    reader.read_chunks(|version, chunk| {
        match version {
            1 => {
                let name: String = chunk.read()?;
                let point: [f64; 3] = chunk.read()?;
                chunk.finish()?;
                assert_eq!((name, point), ("World".into(), [100.0, 200.0, 300.0]));
            }
            2 => {} // Future data: read_chunks skips its unread payload automatically.
            _ => {}
        }
        Ok(())
    })?;
    println!("read known chunk and skipped future chunk successfully");
    Ok(())
}
