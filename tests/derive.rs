#![cfg(feature = "derive")]

use binary_archive::{ArchiveError, ArchiveReader, ArchiveWriter, BinaryArchive};
use std::io::Cursor;

#[derive(Debug, PartialEq, BinaryArchive)]
#[binary_archive(version = 7)]
struct Project {
    name: String,
    visible: bool,
}

#[derive(Debug, PartialEq, BinaryArchive)]
struct Tuple<T, const N: usize>(T, [u32; N])
where
    T: PartialEq;

#[derive(Debug, PartialEq, BinaryArchive)]
struct Empty;

#[derive(Debug, PartialEq, BinaryArchive)]
struct Node {
    value: u32,
    next: Option<Box<Node>>,
}

#[derive(Debug, PartialEq, BinaryArchive)]
struct Collision<__BinaryArchiveIo> {
    value: __BinaryArchiveIo,
}

#[test]
fn derives_named_tuple_unit_generic_and_recursive_structs_without_default() {
    let project = Project {
        name: "demo".into(),
        visible: true,
    };
    assert_eq!(
        Project::from_bytes(&project.to_bytes().unwrap()).unwrap(),
        project
    );
    let tuple = Tuple(project, [1, 2]);
    assert_eq!(
        Tuple::<Project, 2>::from_bytes(&tuple.to_bytes().unwrap()).unwrap(),
        tuple
    );
    assert_eq!(
        Empty::from_bytes(&Empty.to_bytes().unwrap()).unwrap(),
        Empty
    );
    let node = Node {
        value: 1,
        next: Some(Box::new(Node {
            value: 2,
            next: None,
        })),
    };
    assert_eq!(Node::from_bytes(&node.to_bytes().unwrap()).unwrap(), node);
    let collision = Collision { value: 12u32 };
    assert_eq!(
        Collision::<u32>::from_bytes(&collision.to_bytes().unwrap()).unwrap(),
        collision
    );
}

#[test]
fn derive_matches_legacy_encoding_and_can_read_legacy_data() {
    #[derive(Default)]
    struct Legacy {
        name: String,
        visible: bool,
    }
    binary_archive::impl_binary_archive!(
        Legacy,
        version = 7,
        fields {
            name: String,
            visible: bool
        }
    );
    let legacy = Legacy {
        name: "demo".into(),
        visible: true,
    };
    let project = Project {
        name: legacy.name.clone(),
        visible: legacy.visible,
    };
    assert_eq!(project.to_bytes().unwrap(), legacy.to_bytes().unwrap());
    assert_eq!(
        Project::from_bytes(&legacy.to_bytes().unwrap()).unwrap(),
        project
    );
    assert_eq!(
        Legacy::from_bytes(&project.to_bytes().unwrap())
            .unwrap()
            .name,
        "demo"
    );
}

#[test]
fn unknown_versions_and_extra_payload_are_errors() {
    let mut bytes = Empty.to_bytes().unwrap();
    bytes[0] = 2;
    assert!(matches!(
        Empty::from_bytes(&bytes),
        Err(ArchiveError::InvalidData(_))
    ));
    bytes[0] = 1;
    bytes[4] = 1;
    bytes.push(0);
    assert!(matches!(
        Empty::from_bytes(&bytes),
        Err(ArchiveError::ChunkNotFullyConsumed { remaining: 1 })
    ));
}

#[test]
fn derived_records_work_in_a_stream_and_with_named_io() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer.write_named(&Empty).unwrap();
    writer.write(&Empty).unwrap();
    writer.write(&123u32).unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()));
    assert_eq!(reader.read_named::<Empty>().unwrap(), Empty);
    assert_eq!(reader.read::<Empty>().unwrap(), Empty);
    assert_eq!(reader.read::<u32>().unwrap(), 123);
}
