//! Decoder for the `BLOOD.DAT` resource archive.
//!
//! The archive starts with a two-byte directory header followed by packed
//! records. Serialized payload positions are validated and retained privately;
//! callers select a typed resource name and receive a borrowed byte slice.

use std::fmt;
use std::ops::Range;

const DIRECTORY_HEADER_SIZE: usize = 2;
const DIRECTORY_SCAN_LIMIT: usize = u16::MAX as usize + 1;
const RESOURCE_NAME_FIELD_SIZE: usize = 16;
const RESOURCE_NAME_MAXIMUM_LENGTH: usize = RESOURCE_NAME_FIELD_SIZE - 1;
const BYTE_COUNT_FIELD_SIZE: usize = 4;
const FILE_POSITION_FIELD_SIZE: usize = 4;
const RESERVED_FIELD_SIZE: usize = 1;
const DIRECTORY_ENTRY_SIZE: usize = RESOURCE_NAME_FIELD_SIZE
    + BYTE_COUNT_FIELD_SIZE
    + FILE_POSITION_FIELD_SIZE
    + RESERVED_FIELD_SIZE;
const BYTE_COUNT_FIELD_OFFSET: usize = RESOURCE_NAME_FIELD_SIZE;
const FILE_POSITION_FIELD_OFFSET: usize = BYTE_COUNT_FIELD_OFFSET + BYTE_COUNT_FIELD_SIZE;
const ASCII_LOWERCASE_A: u8 = b'a';
const ASCII_CASE_BIT: u8 = 0x20;

/// A validated nonempty DOS resource name accepted by the archive lookup.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BloodResourceName(Box<[u8]>);

impl BloodResourceName {
    /// Validate and own one archive or loose-file resource name.
    pub fn new(name: impl AsRef<[u8]>) -> Result<Self, BloodArchiveError> {
        let name = name.as_ref();
        if name.is_empty() {
            return Err(BloodArchiveError::EmptyResourceName);
        }
        if name.len() > RESOURCE_NAME_MAXIMUM_LENGTH {
            return Err(BloodArchiveError::ResourceNameTooLong {
                actual: name.len(),
                maximum: RESOURCE_NAME_MAXIMUM_LENGTH,
            });
        }
        if let Some(index) = name.iter().position(|byte| *byte == u8::MIN) {
            return Err(BloodArchiveError::ResourceNameContainsNul { index });
        }
        if let Some((index, byte)) = name
            .iter()
            .copied()
            .enumerate()
            .find(|(_index, byte)| !byte.is_ascii())
        {
            return Err(BloodArchiveError::NonAsciiResourceName { index, byte });
        }
        Ok(Self(Box::from(name)))
    }

    /// Return the authored case-preserving resource-name bytes.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Return the exact byte folding used by the native archive search.
    ///
    /// This intentionally clears the ASCII case bit for every byte at or above
    /// lowercase `a`, including `{`, `|`, `}`, and `~`.
    pub fn archive_lookup_key(&self) -> Box<[u8]> {
        fold_archive_lookup_name(self.as_bytes())
    }
}

/// One validated archive directory record without exposed file positions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BloodArchiveEntry {
    name: BloodResourceName,
    lookup_key: Box<[u8]>,
    payload: Range<usize>,
}

impl BloodArchiveEntry {
    /// Return this entry's authored resource name.
    pub fn name(&self) -> &BloodResourceName {
        &self.name
    }

    /// Return this entry's validated payload byte count.
    pub fn byte_count(&self) -> usize {
        self.payload.len()
    }
}

/// Malformed `BLOOD.DAT` data or invalid resource name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BloodArchiveError {
    /// The archive cannot contain its directory header.
    ArchiveTooShort(usize),
    /// A nonempty directory record ends before all packed fields are present.
    TruncatedDirectoryEntry {
        /// Zero-based record index.
        entry: usize,
        /// Bytes available before the native directory scan limit.
        available: usize,
    },
    /// A directory name fills all sixteen bytes without a terminator.
    UnterminatedDirectoryName {
        /// Zero-based record index.
        entry: usize,
    },
    /// A signed directory byte count is negative.
    NegativeByteCount {
        /// Zero-based record index.
        entry: usize,
        /// Invalid signed value.
        value: i32,
    },
    /// A signed directory payload position is negative.
    NegativeFilePosition {
        /// Zero-based record index.
        entry: usize,
        /// Invalid signed value.
        value: i32,
    },
    /// A directory record points outside the archive.
    PayloadOutOfBounds {
        /// Zero-based record index.
        entry: usize,
        /// Decoded payload byte count.
        byte_count: usize,
    },
    /// No empty record appeared within the directory region visible natively.
    MissingDirectoryTerminator,
    /// Runtime names must not be empty because an empty archive name terminates
    /// the directory.
    EmptyResourceName,
    /// A resource name cannot fit in the packed directory field.
    ResourceNameTooLong {
        /// Supplied byte count.
        actual: usize,
        /// Largest accepted byte count.
        maximum: usize,
    },
    /// A runtime resource name contains an embedded terminator.
    ResourceNameContainsNul {
        /// Byte index of the embedded terminator.
        index: usize,
    },
    /// A runtime resource name contains a byte outside ASCII.
    NonAsciiResourceName {
        /// Byte index of the invalid value.
        index: usize,
        /// Invalid byte.
        byte: u8,
    },
}

impl fmt::Display for BloodArchiveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BloodArchiveError {}

/// Owned and validated `BLOOD.DAT` archive.
#[derive(Clone, Debug)]
pub struct BloodArchive {
    data: Box<[u8]>,
    entries: Box<[BloodArchiveEntry]>,
}

impl BloodArchive {
    /// Decode every packed directory record visible to the original lookup.
    pub fn decode(data: Box<[u8]>) -> Result<Self, BloodArchiveError> {
        if data.len() < DIRECTORY_HEADER_SIZE {
            return Err(BloodArchiveError::ArchiveTooShort(data.len()));
        }

        let scan_limit = data.len().min(DIRECTORY_SCAN_LIMIT);
        let mut cursor = DIRECTORY_HEADER_SIZE;
        let mut entries = Vec::new();
        while cursor < scan_limit {
            if data[cursor] == u8::MIN {
                return Ok(Self {
                    data,
                    entries: entries.into_boxed_slice(),
                });
            }

            let entry_index = entries.len();
            let end = cursor
                .checked_add(DIRECTORY_ENTRY_SIZE)
                .filter(|end| *end <= scan_limit)
                .ok_or(BloodArchiveError::TruncatedDirectoryEntry {
                    entry: entry_index,
                    available: scan_limit - cursor,
                })?;
            let name_field = &data[cursor..cursor + RESOURCE_NAME_FIELD_SIZE];
            let name_length = name_field
                .iter()
                .position(|byte| *byte == u8::MIN)
                .ok_or(BloodArchiveError::UnterminatedDirectoryName { entry: entry_index })?;
            let name = BloodResourceName::new(&name_field[..name_length])?;
            let byte_count =
                read_nonnegative_i32(&data, cursor + BYTE_COUNT_FIELD_OFFSET, entry_index, true)?;
            let file_position = read_nonnegative_i32(
                &data,
                cursor + FILE_POSITION_FIELD_OFFSET,
                entry_index,
                false,
            )?;
            let payload_end = file_position
                .checked_add(byte_count)
                .filter(|end| *end <= data.len());
            let Some(payload_end) = payload_end else {
                return Err(BloodArchiveError::PayloadOutOfBounds {
                    entry: entry_index,
                    byte_count,
                });
            };
            entries.push(BloodArchiveEntry {
                lookup_key: fold_archive_lookup_name(name.as_bytes()),
                name,
                payload: file_position..payload_end,
            });
            cursor = end;
        }

        Err(BloodArchiveError::MissingDirectoryTerminator)
    }

    /// Return the decoded directory in authored order.
    pub fn entries(&self) -> &[BloodArchiveEntry] {
        &self.entries
    }

    /// Return the first directory record matching a resource name.
    pub fn member_entry(&self, name: &BloodResourceName) -> Option<&BloodArchiveEntry> {
        let lookup_key = name.archive_lookup_key();
        self.entries
            .iter()
            .find(|entry| entry.lookup_key == lookup_key)
    }

    /// Borrow a matching member's validated payload bytes.
    ///
    /// This is the flat-data equivalent of `resource_archive_match` at
    /// BLOODPRG file offset `0x0026CF`. Memory-backend selection and mutable
    /// filename cursors do not cross the decoder boundary.
    pub fn member(&self, name: &BloodResourceName) -> Option<&[u8]> {
        let entry = self.member_entry(name)?;
        Some(&self.data[entry.payload.clone()])
    }
}

fn fold_archive_lookup_name(name: &[u8]) -> Box<[u8]> {
    name.iter()
        .map(|byte| {
            if *byte >= ASCII_LOWERCASE_A {
                *byte & !ASCII_CASE_BIT
            } else {
                *byte
            }
        })
        .collect()
}

fn read_nonnegative_i32(
    data: &[u8],
    start: usize,
    entry: usize,
    byte_count: bool,
) -> Result<usize, BloodArchiveError> {
    let value = i32::from_le_bytes(
        data[start..start + BYTE_COUNT_FIELD_SIZE]
            .try_into()
            .expect("validated archive directory field"),
    );
    if value < 0 {
        return if byte_count {
            Err(BloodArchiveError::NegativeByteCount { entry, value })
        } else {
            Err(BloodArchiveError::NegativeFilePosition { entry, value })
        };
    }
    Ok(usize::try_from(value).expect("nonnegative i32 fits usize"))
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use serde::Deserialize;

    use super::*;

    const TEST_TRAILING_BYTE: u8 = 0x5A;
    const SHIPPED_ARCHIVE_ENTRY_COUNT: usize = 974;
    const SHIPPED_FIRST_MEMBER_BYTE_COUNT: usize = 139_169;
    const ORIGINAL_ARCHIVE_ROOT_ENVIRONMENT_VARIABLE: &str = "CBLOOD_ORIGINAL_ARCHIVE_ROOT";
    const REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE: &str = "CBLOOD_REQUIRE_ACCURACY_TESTS";

    #[derive(Deserialize)]
    struct ArchiveLookupOracle {
        input_filename_hex: String,
        matched_record: Option<usize>,
        records: Vec<ArchiveRecordOracle>,
    }

    #[derive(Deserialize)]
    struct ArchiveRecordOracle {
        filename_hex: String,
    }

    fn decode_hex(encoded: &str) -> Vec<u8> {
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|digits| {
                let digits = std::str::from_utf8(digits).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect()
    }

    fn archive_bytes(records: &[(Vec<u8>, Vec<u8>)]) -> Box<[u8]> {
        let directory_size =
            DIRECTORY_HEADER_SIZE + records.len() * DIRECTORY_ENTRY_SIZE + RESERVED_FIELD_SIZE;
        let payload_size: usize = records.iter().map(|(_name, payload)| payload.len()).sum();
        let mut archive = vec![u8::MIN; directory_size + payload_size];
        archive[..DIRECTORY_HEADER_SIZE]
            .copy_from_slice(&u16::try_from(records.len()).unwrap().to_le_bytes());
        let mut payload_position = directory_size;
        for (entry, (name, payload)) in records.iter().enumerate() {
            let cursor = DIRECTORY_HEADER_SIZE + entry * DIRECTORY_ENTRY_SIZE;
            archive[cursor..cursor + name.len()].copy_from_slice(name);
            archive[cursor + BYTE_COUNT_FIELD_OFFSET..cursor + FILE_POSITION_FIELD_OFFSET]
                .copy_from_slice(&i32::try_from(payload.len()).unwrap().to_le_bytes());
            archive[cursor + FILE_POSITION_FIELD_OFFSET
                ..cursor + FILE_POSITION_FIELD_OFFSET + FILE_POSITION_FIELD_SIZE]
                .copy_from_slice(&i32::try_from(payload_position).unwrap().to_le_bytes());
            archive[cursor + DIRECTORY_ENTRY_SIZE - RESERVED_FIELD_SIZE] = TEST_TRAILING_BYTE;
            archive[payload_position..payload_position + payload.len()].copy_from_slice(payload);
            payload_position += payload.len();
        }
        archive.into_boxed_slice()
    }

    #[test]
    fn archive_search_matches_every_native_oracle_vector() {
        let vectors: Vec<ArchiveLookupOracle> = serde_json::from_str(include_str!(
            "../../../re/tools/oracle_vectors/func_26cf_natural.json"
        ))
        .unwrap();

        for vector in vectors {
            let records: Vec<_> = vector
                .records
                .iter()
                .enumerate()
                .map(|(index, record)| {
                    (
                        decode_hex(&record.filename_hex),
                        vec![u8::try_from(index + 1).unwrap()],
                    )
                })
                .collect();
            let archive = BloodArchive::decode(archive_bytes(&records)).unwrap();
            let input = decode_hex(&vector.input_filename_hex);
            let name = BloodResourceName::new(&input).unwrap();
            let matched = archive.member_entry(&name).and_then(|entry| {
                archive
                    .entries()
                    .iter()
                    .position(|candidate| candidate == entry)
            });

            assert_eq!(matched, vector.matched_record);
            assert_eq!(name.as_bytes(), input);
        }
    }

    #[test]
    fn rejects_payload_positions_outside_the_owned_archive() {
        let mut bytes = archive_bytes(&[(b"BAD.DAT".to_vec(), vec![TEST_TRAILING_BYTE])]);
        let offset = DIRECTORY_HEADER_SIZE + FILE_POSITION_FIELD_OFFSET;
        bytes[offset..offset + FILE_POSITION_FIELD_SIZE].copy_from_slice(&i32::MAX.to_le_bytes());
        assert!(matches!(
            BloodArchive::decode(bytes),
            Err(BloodArchiveError::PayloadOutOfBounds { entry: 0, .. })
        ));
    }

    #[test]
    fn shipped_archive_directory_and_first_member_decode() {
        let Some(path) = shipped_archive() else {
            return;
        };
        let archive = BloodArchive::decode(
            std::fs::read(&path)
                .unwrap_or_else(|error| panic!("reading {}: {error}", path.display()))
                .into_boxed_slice(),
        )
        .unwrap();
        let first_name = BloodResourceName::new(r"SN\CROLLIS.SND").unwrap();

        assert_eq!(archive.entries().len(), SHIPPED_ARCHIVE_ENTRY_COUNT);
        assert_eq!(
            archive.member(&first_name).unwrap().len(),
            SHIPPED_FIRST_MEMBER_BYTE_COUNT
        );
    }

    fn shipped_archive() -> Option<PathBuf> {
        if let Some(root) = std::env::var_os(ORIGINAL_ARCHIVE_ROOT_ENVIRONMENT_VARIABLE) {
            let path = PathBuf::from(root).join("BLOOD.DAT");
            assert!(
                path.is_file(),
                "configured original BLOOD.DAT does not exist: {}",
                path.display()
            );
            return Some(path);
        }
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/_tmp_iso/BLOOD.DAT");
        if path.is_file() {
            return Some(path);
        }
        assert!(
            std::env::var_os(REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE).is_none(),
            "{REQUIRE_ACCURACY_TESTS_ENVIRONMENT_VARIABLE}=1 requires {ORIGINAL_ARCHIVE_ROOT_ENVIRONMENT_VARIABLE}"
        );
        None
    }
}
