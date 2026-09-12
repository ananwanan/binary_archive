use binary_archive::{ArchiveError, ArchiveReader, ArchiveWriter, BinaryArchive, BinaryDecode};
use std::io::{Cursor, Read, Seek};

#[derive(Debug, Default, PartialEq)]
struct Record {
    id: u32,
    name: String,
}
binary_archive::impl_binary_archive!(
    Record,
    version = 1,
    fields {
        id: u32,
        name: String
    }
);

#[derive(Debug, Default, PartialEq)]
struct Pair {
    first: Record,
    second: Record,
}
binary_archive::impl_binary_archive!(
    Pair,
    version = 1,
    fields {
        first: Record,
        second: Record
    }
);

fn record(id: u32) -> Record {
    Record {
        id,
        name: "hi".into(),
    }
}

#[test]
fn legacy_wire_format_is_unchanged() {
    let bytes = record(42).to_bytes().unwrap();
    let expected = [
        1, 0, 0, 0, 14, 0, 0, 0, 0, 0, 0, 0, 42, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, b'h', b'i',
    ];
    assert_eq!(bytes, expected);
    assert_eq!(Record::from_bytes(&expected).unwrap(), record(42));
}

#[test]
fn sequential_named_records_leave_the_next_value_unread() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer.write_named(&record(1)).unwrap();
    writer.write_named(&record(2)).unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()));
    assert_eq!(reader.read_named::<Record>().unwrap(), record(1));
    assert_eq!(reader.read_named::<Record>().unwrap(), record(2));
}

#[test]
fn nested_records_and_vectors_round_trip() {
    let pair = Pair {
        first: record(1),
        second: record(2),
    };
    assert_eq!(Pair::from_bytes(&pair.to_bytes().unwrap()).unwrap(), pair);
    let records = vec![record(1), record(2)];
    assert_eq!(
        Vec::<Record>::from_bytes(&records.to_bytes().unwrap()).unwrap(),
        records
    );
}

#[test]
fn unknown_legacy_version_defaults_and_consumes_only_one_chunk() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer.write_chunk(99, |chunk| chunk.write(&7u32)).unwrap();
    writer.write(&record(2)).unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()));
    assert_eq!(reader.read::<Record>().unwrap(), Record::default());
    assert_eq!(reader.read::<Record>().unwrap(), record(2));
}

#[test]
fn every_truncation_of_a_record_is_rejected() {
    let bytes = record(42).to_bytes().unwrap();
    for len in 0..bytes.len() {
        assert!(Record::from_bytes(&bytes[..len]).is_err(), "length {len}");
    }
}

#[test]
fn trailing_bytes_are_rejected_by_convenience_decoder() {
    let mut bytes = 42u32.to_bytes().unwrap();
    bytes.push(0);
    assert!(matches!(
        u32::from_bytes(&bytes),
        Err(ArchiveError::InvalidData(_))
    ));
}

#[test]
fn collections_are_limited_by_memory_size() {
    let bytes = vec![1u64, 2].to_bytes().unwrap();
    assert!(matches!(
        Vec::<u64>::from_bytes_with_limit(&bytes, 15),
        Err(ArchiveError::LimitExceeded {
            value: 16,
            limit: 15,
            ..
        })
    ));
    assert_eq!(
        Vec::<u64>::from_bytes_with_limit(&bytes, 16).unwrap(),
        vec![1, 2]
    );
    assert!(matches!(
        <[u64; 2]>::from_bytes_with_limit(&[0; 16], 15),
        Err(ArchiveError::LimitExceeded { .. })
    ));
    assert!(matches!(
        Vec::<u64>::from_bytes_with_limit(&u64::MAX.to_le_bytes(), u64::MAX),
        Err(ArchiveError::LengthOverflow { .. })
    ));
}

#[test]
fn nested_chunks_inherit_allocation_limits() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    // Small payload containing an oversized string length prefix.
    writer.write_chunk(1, |chunk| chunk.write(&100u64)).unwrap();
    let bytes = writer.into_inner().into_inner();
    let mut reader = ArchiveReader::new(Cursor::new(bytes)).with_max_allocation(16);
    let mut chunk = reader.read_chunk().unwrap();
    assert!(matches!(
        chunk.read::<String>(),
        Err(ArchiveError::LimitExceeded { limit: 16, .. })
    ));
}

#[test]
fn nested_chunks_also_inherit_limits_above_the_default() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer
        .write_chunk(1, |chunk| chunk.write(&(65 * 1024 * 1024u64)))
        .unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()))
        .with_max_allocation(128 * 1024 * 1024);
    let mut chunk = reader.read_chunk().unwrap();
    // The configured limit allows the request; the missing bytes cause EOF.
    assert!(matches!(chunk.read::<String>(), Err(ArchiveError::Io(_))));
}

#[test]
fn skip_chunk_rejects_truncation_and_overflow_without_allocating() {
    for length in [5, u64::MAX] {
        let mut bytes = 1u32.to_le_bytes().to_vec();
        bytes.extend_from_slice(&length.to_le_bytes());
        let mut reader = ArchiveReader::new(Cursor::new(bytes));
        assert!(reader.skip_chunk().is_err());
    }
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer
        .write_chunk(1, |chunk| chunk.write(&[0u8; 32]))
        .unwrap();
    let mut reader =
        ArchiveReader::new(Cursor::new(writer.into_inner().into_inner())).with_max_allocation(0);
    assert_eq!(reader.skip_chunk().unwrap().length, 32);
}

struct SeekPastEnd;
impl BinaryDecode for SeekPastEnd {
    fn decode<R: Read + Seek>(
        reader: &mut ArchiveReader<R>,
    ) -> binary_archive::ArchiveResult<Self> {
        reader.seek(100)?;
        Ok(Self)
    }
}

#[test]
fn custom_decoder_cannot_panic_remaining_or_pass_finish_after_overseek() {
    let mut writer = ArchiveWriter::new(Cursor::new(Vec::new()));
    writer.write_chunk(1, |_| Ok(())).unwrap();
    let mut reader = ArchiveReader::new(Cursor::new(writer.into_inner().into_inner()));
    let mut chunk = reader.read_chunk().unwrap();
    chunk.read::<SeekPastEnd>().unwrap();
    assert_eq!(chunk.remaining(), 0);
    assert!(matches!(chunk.finish(), Err(ArchiveError::InvalidData(_))));
}

#[test]
fn invalid_markers_utf8_and_io_sources_are_preserved() {
    assert!(bool::from_bytes(&[2]).is_err());
    assert!(Option::<u8>::from_bytes(&[2]).is_err());
    let mut bytes = 1u64.to_le_bytes().to_vec();
    bytes.push(255);
    assert!(String::from_bytes(&bytes).is_err());
    let err = u32::from_bytes(&[]).unwrap_err();
    assert!(std::error::Error::source(&err).is_some());
}
