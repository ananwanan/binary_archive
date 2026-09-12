use binary_archive::{ArchiveResult, BinaryArchive};

#[derive(Debug, PartialEq, BinaryArchive)]
#[binary_archive(version = 1)]
struct Project {
    name: String,
    visible: bool,
    points: Vec<Point>,
}

#[derive(Debug, PartialEq, BinaryArchive)]
struct Point(f64, f64, f64);

fn main() -> ArchiveResult<()> {
    let project = Project {
        name: "Demo".into(),
        visible: true,
        points: vec![Point(1.0, 2.0, 3.0), Point(4.0, 5.0, 6.0)],
    };
    let bytes = project.to_bytes()?;
    let loaded = Project::from_bytes(&bytes)?;
    assert_eq!(project, loaded);
    println!("{loaded:#?}\n{} bytes, serialization OK", bytes.len());
    Ok(())
}
