#![cfg(feature = "derive")]

use binary_archive::{
    ArchiveError, ArchiveReader, ArchiveWriter, BinaryArchive, DeletedFieldPolicy,
};
use std::io::Cursor;
use std::sync::Mutex;

#[derive(Debug, PartialEq, BinaryArchive)]
#[binary_archive(versioned, version = 1)]
struct V1 {
    name: String,
    removed: u64,
    count: u32,
}

#[derive(Debug, PartialEq, BinaryArchive)]
#[binary_archive(versioned, version = 2)]
struct V2 {
    // Reordering is safe: fields are identified by stable names.
    #[binary_archive(version = 1)]
    count: u32,
    #[binary_archive(version = 1)]
    name: String,
    visible: bool,
    #[binary_archive(default = String::from("new field"))]
    description: String,
}

fn old() -> V1 {
    V1 {
        name: "demo".into(),
        removed: 99,
        count: 7,
    }
}
fn new() -> V2 {
    V2 {
        count: 7,
        name: "demo".into(),
        visible: true,
        description: "saved".into(),
    }
}

#[test]
fn version_two_reads_version_one_and_defaults_new_fields() {
    let result = V2::from_bytes(&old().to_bytes().unwrap()).unwrap();
    assert_eq!(
        result,
        V2 {
            count: 7,
            name: "demo".into(),
            visible: false,
            description: "new field".into()
        }
    );
}

#[test]
fn default_warning_can_be_redirected_and_describes_deleted_field() {
    static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
    fn record(message: &str) {
        WARNINGS.lock().unwrap().push(message.to_owned());
    }
    assert_eq!(DeletedFieldPolicy::default(), DeletedFieldPolicy::Warn);
    let bytes = old().to_bytes().unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(bytes)).with_warning_handler(record);
    reader.read::<V2>().unwrap();
    let messages = WARNINGS.lock().unwrap();
    assert_eq!(messages.len(), 1);
    assert!(messages[0].contains("V2.removed"));
    assert!(messages[0].contains("introduced in version 1"));
}

#[test]
fn deleted_field_can_return_error_or_panic() {
    let bytes = old().to_bytes().unwrap();
    let err = V2::from_bytes_with_policy(&bytes, DeletedFieldPolicy::Error).unwrap_err();
    assert!(err.to_string().contains("V2.removed"));
    let panic =
        std::panic::catch_unwind(|| V2::from_bytes_with_policy(&bytes, DeletedFieldPolicy::Panic));
    assert!(panic.is_err());
}

#[test]
fn older_reader_silently_skips_fields_introduced_by_newer_schema() {
    #[derive(Debug, PartialEq, BinaryArchive)]
    #[binary_archive(versioned, version = 1)]
    struct Earlier {
        name: String,
        count: u32,
    }
    let value =
        Earlier::from_bytes_with_policy(&new().to_bytes().unwrap(), DeletedFieldPolicy::Panic)
            .unwrap();
    assert_eq!(
        value,
        Earlier {
            name: "demo".into(),
            count: 7
        }
    );
}

#[test]
fn same_version_round_trip_and_field_metadata_inherit_struct_version() {
    assert_eq!(V2::from_bytes(&new().to_bytes().unwrap()).unwrap(), new());
    let mut reader = ArchiveReader::new(Cursor::new(new().to_bytes().unwrap()));
    let mut chunk = reader.read_chunk().unwrap();
    assert_eq!(chunk.header().version, 2);
    let mut fields = Vec::new();
    chunk
        .read_versioned_fields(|name, version, _| {
            fields.push((name.to_owned(), version));
            Ok(())
        })
        .unwrap();
    assert_eq!(
        fields,
        [
            ("count".into(), 1),
            ("name".into(), 1),
            ("visible".into(), 2),
            ("description".into(), 2)
        ]
    );
}

#[test]
fn introduction_version_must_remain_stable() {
    #[derive(Debug, BinaryArchive)]
    #[binary_archive(versioned, version = 2)]
    struct Incorrect {
        name: String,
    }
    assert!(
        Incorrect::from_bytes(&old().to_bytes().unwrap())
            .unwrap_err()
            .to_string()
            .contains("introduction version changed")
    );
}

fn encoded_fields(version: u32, fields: &[(&str, u32, u32)]) -> Vec<u8> {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer
        .write_chunk(version, |chunk| {
            chunk.write(b"BARFLD01")?;
            for (name, since, value) in fields {
                chunk.write_versioned_field(name, *since, value)?;
            }
            Ok(())
        })
        .unwrap();
    writer.into_inner().into_inner()
}

#[derive(Debug, BinaryArchive)]
#[binary_archive(versioned)]
struct Required {
    value: u32,
}

#[test]
fn missing_existing_fields_are_errors_not_defaults() {
    let bytes = encoded_fields(1, &[]);
    assert!(
        Required::from_bytes(&bytes)
            .unwrap_err()
            .to_string()
            .contains("missing field")
    );
}

#[test]
fn invalid_field_metadata_and_duplicates_are_rejected() {
    for fields in [
        vec![("value", 2, 1)],
        vec![("value", 1, 1), ("value", 1, 2)],
        vec![("unknown", 1, 1), ("unknown", 1, 2)],
    ] {
        assert!(matches!(
            Required::from_bytes(&encoded_fields(1, &fields)),
            Err(ArchiveError::InvalidData(_))
        ));
    }
}

#[test]
fn stored_field_payload_is_bounded_and_must_be_fully_read() {
    let mut bytes = encoded_fields(1, &[("value", 1, 1)]);
    // Outer chunk length is patched to add one extra payload byte to the field.
    let outer_length = bytes.len() as u64 - 12 + 1;
    bytes[4..12].copy_from_slice(&outer_length.to_le_bytes());
    let field_length_position = 12 + 8 + 8 + "value".len() + 4;
    bytes[field_length_position..field_length_position + 8].copy_from_slice(&5u64.to_le_bytes());
    bytes.push(0);
    assert!(matches!(
        Required::from_bytes(&bytes),
        Err(ArchiveError::ChunkNotFullyConsumed { remaining: 1 })
    ));
}

#[test]
fn every_truncated_versioned_record_is_rejected() {
    let bytes = new().to_bytes().unwrap();
    for length in 0..bytes.len() {
        assert!(V2::from_bytes(&bytes[..length]).is_err(), "length {length}");
    }
}

#[test]
fn legacy_positional_format_requires_explicit_migration() {
    #[derive(BinaryArchive)]
    struct Legacy {
        value: u32,
    }
    assert!(Required::from_bytes(&Legacy { value: 7 }.to_bytes().unwrap()).is_err());
}

#[test]
fn nested_fields_inherit_policy_and_allocation_limits() {
    #[derive(BinaryArchive)]
    struct OldContainer {
        nested: V1,
    }
    #[derive(BinaryArchive)]
    struct NewContainer {
        nested: V2,
    }
    let bytes = OldContainer { nested: old() }.to_bytes().unwrap();
    assert!(matches!(
        NewContainer::from_bytes_with_policy(&bytes, DeletedFieldPolicy::Error),
        Err(ArchiveError::InvalidData(_))
    ));

    #[derive(Debug, BinaryArchive)]
    #[binary_archive(versioned)]
    struct Large {
        value: String,
    }
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer
        .write_chunk(1, |chunk| {
            chunk.write(b"BARFLD01")?;
            chunk.write_versioned_field("value", 1, &1000u64)
        })
        .unwrap();
    let bytes = writer.into_inner().into_inner();
    assert!(matches!(
        Large::from_bytes_with_limit(&bytes, 100),
        Err(ArchiveError::LimitExceeded { limit: 100, .. })
    ));
}

#[test]
fn renamed_and_tuple_fields_can_keep_explicit_stable_ids() {
    #[derive(Debug, PartialEq, BinaryArchive)]
    #[binary_archive(versioned, version = 2)]
    struct Renamed {
        #[binary_archive(version = 1, id = "value")]
        renamed: u32,
    }
    let bytes = encoded_fields(1, &[("value", 1, 42)]);
    assert_eq!(
        Renamed::from_bytes(&bytes).unwrap(),
        Renamed { renamed: 42 }
    );

    #[derive(Debug, PartialEq, BinaryArchive)]
    #[binary_archive(versioned, version = 2)]
    struct Tuple(#[binary_archive(version = 1, id = "value")] u32, bool);
    assert_eq!(Tuple::from_bytes(&bytes).unwrap(), Tuple(42, false));
}

#[test]
fn custom_defaults_support_non_default_field_types_and_generic_structs() {
    #[derive(Debug, PartialEq, BinaryArchive)]
    struct NoDefault {
        value: u32,
    }
    #[derive(Debug, PartialEq, BinaryArchive)]
    #[binary_archive(versioned, version = 2)]
    struct Generic<T> {
        #[binary_archive(version = 1)]
        value: T,
        #[binary_archive(default = NoDefault { value: 99 })]
        added: NoDefault,
    }
    let value = Generic::<u32>::from_bytes(&encoded_fields(1, &[("value", 1, 3)])).unwrap();
    assert_eq!(
        value,
        Generic {
            value: 3,
            added: NoDefault { value: 99 }
        }
    );
    assert_eq!(
        Generic::<u32>::from_bytes(&value.to_bytes().unwrap()).unwrap(),
        value
    );
}

#[test]
fn field_annotations_enable_new_format_and_empty_structs_work() {
    #[derive(Debug, PartialEq, BinaryArchive)]
    struct Implicit {
        #[binary_archive(version = 1)]
        value: u32,
    }
    assert_eq!(
        Implicit::from_bytes(&encoded_fields(1, &[("value", 1, 3)])).unwrap(),
        Implicit { value: 3 }
    );
    #[derive(Debug, PartialEq, BinaryArchive)]
    #[binary_archive(versioned)]
    struct Empty;
    assert_eq!(Empty::from_bytes(&encoded_fields(1, &[])).unwrap(), Empty);
}

#[test]
fn consecutive_versioned_objects_leave_stream_aligned() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer.write_named(&new()).unwrap();
    writer.write_named(&new()).unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()));
    assert_eq!(reader.read_named::<V2>().unwrap(), new());
    assert_eq!(reader.read_named::<V2>().unwrap(), new());
}
