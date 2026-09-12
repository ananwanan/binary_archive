use binary_archive::{ArchiveResult, BinaryArchive, DeletedFieldPolicy};

#[derive(BinaryArchive)]
#[binary_archive(versioned, version = 1)]
struct V1 {
    name: String,
    obsolete: u32,
}

#[derive(Debug, BinaryArchive)]
#[binary_archive(versioned, version = 2)]
struct V2 {
    #[binary_archive(version = 1)]
    name: String,
    visible: bool, // No annotation: introduced in the struct's version (2).
    #[binary_archive(default = String::from("not set"))]
    description: String,
}

fn main() -> ArchiveResult<()> {
    let bytes = V1 {
        name: "Demo".into(),
        obsolete: 42,
    }
    .to_bytes()?;
    // Warns about obsolete, preserves name, defaults visible and description.
    let loaded = V2::from_bytes(&bytes)?;
    assert_eq!(loaded.name, "Demo");
    assert!(!loaded.visible);
    assert_eq!(loaded.description, "not set");
    println!("{loaded:#?}");

    assert!(V2::from_bytes_with_policy(&bytes, DeletedFieldPolicy::Error).is_err());
    // Select DeletedFieldPolicy::Panic to panic on a deleted field instead.
    Ok(())
}
