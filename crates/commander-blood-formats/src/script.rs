//! Lossless typed decoding for Commander Blood script directories and dictionaries.

use std::collections::BTreeMap;
use std::fmt;

const DIRECTORY_ENTRY_SIZE: usize = 20;
const DIRECTORY_NAME_CAPACITY: usize = 16;
const DIRECTORY_VALUE_FIELD: usize = 16;
const DIRECTORY_KIND_FIELD: usize = 18;
const WORD_SIZE: usize = 2;
const DICTIONARY_TERMINATOR: u8 = 0;
const SOURCE_OFFSET_VALUE_COUNT: usize = 1;
const MAXIMUM_SCRIPT_IMAGE_SIZE: usize = u16::MAX as usize + SOURCE_OFFSET_VALUE_COUNT;
const OBJECT_KIND_FIELD: usize = 0;

/// Stable identity of one word interned from a script dictionary.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptWordId(usize);

impl ScriptWordId {
    /// Return the zero-based word index in the decoded dictionary.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Stable identity of one active state object in a script directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptObjectId(usize);

impl ScriptObjectId {
    /// Return the zero-based directory index of this state object.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Proven kind word and fixed record shape for one VAR state object.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptObjectKind {
    /// Global player state named `blood` in every shipped profile.
    Player,
    /// Character or interactive actor.
    Actor,
    /// Planet or other named celestial destination.
    CelestialBody,
    /// Ship or other entity used by navigation logic.
    NavigationEntity,
    /// Auxiliary `baby` state used by the original scripts.
    Auxiliary,
    /// Local place within a world.
    Location,
    /// Black-hole destination.
    BlackHole,
    /// Global world state named `orxx`.
    WorldState,
    /// Inventory or transferable object.
    InventoryItem,
}

impl ScriptObjectKind {
    /// Decode a VAR kind word proven by all five shipped state images.
    pub const fn decode(value: u16) -> Option<Self> {
        match value {
            0x0001 => Some(Self::Player),
            0x0002 => Some(Self::Actor),
            0x0008 => Some(Self::CelestialBody),
            0x0010 => Some(Self::NavigationEntity),
            0x0040 => Some(Self::Auxiliary),
            0x0080 => Some(Self::Location),
            0x0100 => Some(Self::BlackHole),
            0x0200 => Some(Self::WorldState),
            0x0400 => Some(Self::InventoryItem),
            _ => None,
        }
    }

    /// Return the original one-hot kind word used by field selection.
    pub const fn mask(self) -> u16 {
        match self {
            Self::Player => 0x0001,
            Self::Actor => 0x0002,
            Self::CelestialBody => 0x0008,
            Self::NavigationEntity => 0x0010,
            Self::Auxiliary => 0x0040,
            Self::Location => 0x0080,
            Self::BlackHole => 0x0100,
            Self::WorldState => 0x0200,
            Self::InventoryItem => 0x0400,
        }
    }

    /// Return this kind's fixed byte size in SCRIPT*.VAR.
    pub const fn record_size(self) -> usize {
        match self {
            Self::Player => 34,
            Self::Actor => 72,
            Self::CelestialBody => 30,
            Self::NavigationEntity => 36,
            Self::Auxiliary => 20,
            Self::Location => 24,
            Self::BlackHole => 32,
            Self::WorldState => 38,
            Self::InventoryItem => 24,
        }
    }
}

/// One owned state object decoded from a VAR image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptStateObject {
    /// Stable identity shared with the active DEB object prefix.
    pub id: ScriptObjectId,
    /// Proven fixed record kind.
    pub kind: ScriptObjectKind,
    bytes: Box<[u8]>,
}

impl ScriptStateObject {
    /// Return the object's exact authored record bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Complete decoded VAR state image partitioned into typed objects and trailing data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptState {
    objects: Vec<ScriptStateObject>,
    trailing_data: Box<[u8]>,
}

impl ScriptState {
    /// Return every state object in active-directory order.
    pub fn objects(&self) -> &[ScriptStateObject] {
        &self.objects
    }

    /// Resolve a typed object identity to its owned state record.
    pub fn object(&self, object: ScriptObjectId) -> Option<&ScriptStateObject> {
        self.objects.get(object.index())
    }

    /// Return authored VAR data following the final fixed-size object.
    pub fn trailing_data(&self) -> &[u8] {
        &self.trailing_data
    }

    /// Re-encode the state image byte for byte.
    pub fn encode(&self) -> Vec<u8> {
        let object_bytes: usize = self.objects.iter().map(|object| object.bytes.len()).sum();
        let mut output = Vec::with_capacity(object_bytes + self.trailing_data.len());
        for object in &self.objects {
            output.extend_from_slice(&object.bytes);
        }
        output.extend_from_slice(&self.trailing_data);
        output
    }
}

/// Proven meaning of a fixed-size DEB directory record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptSymbolKind {
    /// End of the active object prefix.
    Sentinel,
    /// State object stored in the companion VAR image.
    Object,
    /// One-based procedure entry in the COD image.
    Procedure,
    /// Label within the COD image.
    CodeLabel,
    /// Label within the VAR image.
    StateLabel,
    /// Unrecognized value retained for lossless round trips.
    Unknown(u16),
}

impl ScriptSymbolKind {
    const fn decode(value: u16) -> Self {
        match value {
            0 => Self::Sentinel,
            1 => Self::Object,
            2 => Self::Procedure,
            4 => Self::CodeLabel,
            5 => Self::StateLabel,
            other => Self::Unknown(other),
        }
    }

    const fn encode(self) -> u16 {
        match self {
            Self::Sentinel => 0,
            Self::Object => 1,
            Self::Procedure => 2,
            Self::CodeLabel => 4,
            Self::StateLabel => 5,
            Self::Unknown(value) => value,
        }
    }
}

/// One losslessly decoded DEB directory entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDirectoryEntry {
    name_field: [u8; DIRECTORY_NAME_CAPACITY],
    /// Object, procedure, or label value stored by the original compiler.
    pub value: u16,
    /// Semantic class of this entry.
    pub kind: ScriptSymbolKind,
}

impl ScriptDirectoryEntry {
    /// Return the name bytes before the first NUL terminator.
    pub fn name(&self) -> &[u8] {
        let length = self
            .name_field
            .iter()
            .position(|byte| *byte == DICTIONARY_TERMINATOR)
            .unwrap_or(DIRECTORY_NAME_CAPACITY);
        &self.name_field[..length]
    }
}

/// Complete fixed-record DEB directory.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDirectory {
    entries: Vec<ScriptDirectoryEntry>,
}

impl ScriptDirectory {
    /// Return every directory entry in authored order.
    pub fn entries(&self) -> &[ScriptDirectoryEntry] {
        &self.entries
    }

    /// Iterate the contiguous object prefix consumed by native VM object scans.
    pub fn active_objects(&self) -> impl Iterator<Item = (ScriptObjectId, &ScriptDirectoryEntry)> {
        self.entries
            .iter()
            .take_while(|entry| entry.kind == ScriptSymbolKind::Object)
            .enumerate()
            .map(|(index, entry)| (ScriptObjectId(index), entry))
    }

    /// Resolve a typed object identity back to its directory entry.
    pub fn object(&self, object: ScriptObjectId) -> Option<&ScriptDirectoryEntry> {
        self.entries
            .get(object.index())
            .filter(|entry| entry.kind == ScriptSymbolKind::Object)
    }

    /// Find an active object by its exact byte name.
    pub fn find_active_object(&self, name: &[u8]) -> Option<ScriptObjectId> {
        self.active_objects()
            .find_map(|(object, entry)| (entry.name() == name).then_some(object))
    }

    /// Re-encode the directory byte for byte.
    pub fn encode(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(self.entries.len() * DIRECTORY_ENTRY_SIZE);
        for entry in &self.entries {
            output.extend_from_slice(&entry.name_field);
            output.extend_from_slice(&entry.value.to_le_bytes());
            output.extend_from_slice(&entry.kind.encode().to_le_bytes());
        }
        output
    }
}

/// Complete DIC lexicon with each source word interned once.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptDictionary {
    words: Vec<Box<[u8]>>,
    source_offsets: BTreeMap<u16, ScriptWordId>,
}

impl ScriptDictionary {
    /// Return the number of authored dictionary entries, including intentional empties.
    pub fn len(&self) -> usize {
        self.words.len()
    }

    /// Return whether the dictionary contains no entries.
    pub fn is_empty(&self) -> bool {
        self.words.is_empty()
    }

    /// Resolve an encoded DIC byte position during instruction decoding.
    pub fn resolve_source_offset(&self, source_offset: u16) -> Option<ScriptWordId> {
        self.source_offsets.get(&source_offset).copied()
    }

    /// Return the bytes owned by one interned word identity.
    pub fn word(&self, word: ScriptWordId) -> Option<&[u8]> {
        self.words.get(word.index()).map(AsRef::as_ref)
    }

    /// Iterate every interned word in authored order.
    pub fn words(&self) -> impl Iterator<Item = (ScriptWordId, &[u8])> {
        self.words
            .iter()
            .enumerate()
            .map(|(index, word)| (ScriptWordId(index), word.as_ref()))
    }

    /// Re-encode the dictionary byte for byte.
    pub fn encode(&self) -> Vec<u8> {
        let capacity = self
            .words
            .iter()
            .map(|word| word.len().saturating_add(1))
            .sum();
        let mut output = Vec::with_capacity(capacity);
        for word in &self.words {
            output.extend_from_slice(word);
            output.push(DICTIONARY_TERMINATOR);
        }
        output
    }
}

/// Failure while decoding a script companion image.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScriptDataError {
    /// DEB length is not a whole number of fixed records.
    InvalidDirectoryLength {
        /// Actual byte length.
        length: usize,
    },
    /// DIC exceeds the offset domain encoded by script operands.
    DictionaryTooLarge {
        /// Actual byte length.
        length: usize,
    },
    /// Final DIC entry has no NUL terminator.
    UnterminatedDictionaryWord {
        /// Byte position where the unterminated entry begins.
        source_offset: usize,
    },
    /// An active DEB object does not begin after the preceding fixed record.
    NonContiguousStateObject {
        /// Typed object identity.
        object: ScriptObjectId,
        /// Required byte position.
        expected: usize,
        /// Encoded DEB value.
        actual: usize,
    },
    /// A VAR object header contains an object kind not present in shipped data.
    UnknownStateObjectKind {
        /// Typed object identity.
        object: ScriptObjectId,
        /// Unrecognized kind word.
        kind: u16,
    },
    /// A fixed-size state object extends beyond the VAR image.
    TruncatedStateObject {
        /// Typed object identity.
        object: ScriptObjectId,
        /// Required exclusive end position.
        required_end: usize,
        /// Available image length.
        available: usize,
    },
}

impl fmt::Display for ScriptDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ScriptDataError {}

/// Decode a complete fixed-record SCRIPT*.DEB directory.
pub fn decode_script_directory(data: &[u8]) -> Result<ScriptDirectory, ScriptDataError> {
    if !data.len().is_multiple_of(DIRECTORY_ENTRY_SIZE) {
        return Err(ScriptDataError::InvalidDirectoryLength { length: data.len() });
    }

    let entries = data
        .chunks_exact(DIRECTORY_ENTRY_SIZE)
        .map(|record| {
            let mut name_field = [u8::MIN; DIRECTORY_NAME_CAPACITY];
            name_field.copy_from_slice(&record[..DIRECTORY_NAME_CAPACITY]);
            ScriptDirectoryEntry {
                name_field,
                value: read_word(record, DIRECTORY_VALUE_FIELD),
                kind: ScriptSymbolKind::decode(read_word(record, DIRECTORY_KIND_FIELD)),
            }
        })
        .collect();
    Ok(ScriptDirectory { entries })
}

/// Decode and intern every entry in a complete SCRIPT*.DIC lexicon.
pub fn decode_script_dictionary(data: &[u8]) -> Result<ScriptDictionary, ScriptDataError> {
    if data.len() > MAXIMUM_SCRIPT_IMAGE_SIZE {
        return Err(ScriptDataError::DictionaryTooLarge { length: data.len() });
    }

    let mut words = Vec::new();
    let mut source_offsets = BTreeMap::new();
    let mut cursor = usize::MIN;
    while cursor < data.len() {
        let Some(relative_end) = data[cursor..]
            .iter()
            .position(|byte| *byte == DICTIONARY_TERMINATOR)
        else {
            return Err(ScriptDataError::UnterminatedDictionaryWord {
                source_offset: cursor,
            });
        };
        let end = cursor + relative_end;
        let id = ScriptWordId(words.len());
        source_offsets.insert(
            u16::try_from(cursor).expect("validated dictionary offset"),
            id,
        );
        words.push(Box::<[u8]>::from(&data[cursor..end]));
        cursor = end.saturating_add(1);
    }
    Ok(ScriptDictionary {
        words,
        source_offsets,
    })
}

/// Decode SCRIPT*.VAR using the active object prefix from its companion DEB directory.
pub fn decode_script_state(
    data: &[u8],
    directory: &ScriptDirectory,
) -> Result<ScriptState, ScriptDataError> {
    let mut objects = Vec::new();
    let mut cursor = usize::MIN;
    for (object, entry) in directory.active_objects() {
        let actual = usize::from(entry.value);
        if actual != cursor {
            return Err(ScriptDataError::NonContiguousStateObject {
                object,
                expected: cursor,
                actual,
            });
        }
        let kind_word_end = cursor.saturating_add(WORD_SIZE);
        if kind_word_end > data.len() {
            return Err(ScriptDataError::TruncatedStateObject {
                object,
                required_end: kind_word_end,
                available: data.len(),
            });
        }
        let kind_word = read_word(data, cursor + OBJECT_KIND_FIELD);
        let kind =
            ScriptObjectKind::decode(kind_word).ok_or(ScriptDataError::UnknownStateObjectKind {
                object,
                kind: kind_word,
            })?;
        let end = cursor.saturating_add(kind.record_size());
        if end > data.len() {
            return Err(ScriptDataError::TruncatedStateObject {
                object,
                required_end: end,
                available: data.len(),
            });
        }
        objects.push(ScriptStateObject {
            id: object,
            kind,
            bytes: Box::from(&data[cursor..end]),
        });
        cursor = end;
    }
    Ok(ScriptState {
        objects,
        trailing_data: Box::from(&data[cursor..]),
    })
}

fn read_word(data: &[u8], offset: usize) -> u16 {
    let bytes: [u8; WORD_SIZE] = data[offset..offset + WORD_SIZE]
        .try_into()
        .expect("fixed directory field");
    u16::from_le_bytes(bytes)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const PROFILE_COUNT: usize = 5;
    const EXPECTED_DIRECTORY_COUNTS: [usize; PROFILE_COUNT] = [137, 342, 353, 244, 244];
    const EXPECTED_OBJECT_COUNTS: [usize; PROFILE_COUNT] = [122, 122, 130, 136, 130];

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    #[test]
    fn every_original_directory_and_dictionary_round_trips_exactly() {
        for profile in 1..=PROFILE_COUNT {
            let deb = std::fs::read(original_asset(&format!("SCRIPT{profile}.DEB"))).unwrap();
            let dic = std::fs::read(original_asset(&format!("SCRIPT{profile}.DIC"))).unwrap();
            let var = std::fs::read(original_asset(&format!("SCRIPT{profile}.VAR"))).unwrap();
            let directory = decode_script_directory(&deb).unwrap();
            let dictionary = decode_script_dictionary(&dic).unwrap();
            let state = decode_script_state(&var, &directory).unwrap();

            assert_eq!(
                directory.entries().len(),
                EXPECTED_DIRECTORY_COUNTS[profile - 1]
            );
            assert_eq!(
                directory.active_objects().count(),
                EXPECTED_OBJECT_COUNTS[profile - 1]
            );
            assert_eq!(directory.encode(), deb);
            assert_eq!(dictionary.encode(), dic);
            assert_eq!(state.objects().len(), EXPECTED_OBJECT_COUNTS[profile - 1]);
            assert_eq!(state.encode(), var);
            assert!(!dictionary.is_empty());
            assert_eq!(
                dictionary.resolve_source_offset(u16::MIN).unwrap().index(),
                0
            );
        }
    }

    #[test]
    fn malformed_companion_images_are_rejected() {
        assert_eq!(
            decode_script_directory(&[u8::MIN]).unwrap_err(),
            ScriptDataError::InvalidDirectoryLength { length: 1 }
        );
        assert_eq!(
            decode_script_dictionary(b"unterminated").unwrap_err(),
            ScriptDataError::UnterminatedDictionaryWord {
                source_offset: usize::MIN,
            }
        );

        let mut directory_bytes = [u8::MIN; DIRECTORY_ENTRY_SIZE];
        directory_bytes[DIRECTORY_NAME_CAPACITY..DIRECTORY_NAME_CAPACITY + WORD_SIZE]
            .copy_from_slice(&u16::MIN.to_le_bytes());
        directory_bytes[DIRECTORY_KIND_FIELD..DIRECTORY_KIND_FIELD + WORD_SIZE]
            .copy_from_slice(&u16::from(true).to_le_bytes());
        let directory = decode_script_directory(&directory_bytes).unwrap();
        assert_eq!(
            decode_script_state(&u16::MAX.to_le_bytes(), &directory).unwrap_err(),
            ScriptDataError::UnknownStateObjectKind {
                object: ScriptObjectId(usize::MIN),
                kind: u16::MAX,
            }
        );
    }
}
