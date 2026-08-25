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

/// Stable identity of one procedure declared by a script directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptProcedureId(usize);

impl ScriptProcedureId {
    /// Return the zero-based procedure index in directory order.
    pub const fn index(self) -> usize {
        self.0
    }
}

/// Typed word within one owned script state object.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptStateWord {
    owner: ScriptStateOwner,
    word_index: usize,
}

/// Typed adjacent word pair within one owned script state region.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptStateWordPair {
    owner: ScriptStateOwner,
    first_word_index: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum ScriptStateOwner {
    Object(ScriptObjectId),
    TrailingState,
}

impl ScriptStateWord {
    /// Return the object that owns this word.
    pub const fn object(self) -> Option<ScriptObjectId> {
        match self.owner {
            ScriptStateOwner::Object(object) => Some(object),
            ScriptStateOwner::TrailingState => None,
        }
    }

    /// Return the zero-based word index within the owning object.
    pub const fn word_index(self) -> usize {
        self.word_index
    }

    /// Return whether this word belongs to the trailing profile-state block.
    pub const fn is_trailing_state(self) -> bool {
        matches!(self.owner, ScriptStateOwner::TrailingState)
    }
}

impl ScriptStateWordPair {
    /// Return the object that owns both words.
    pub const fn object(self) -> Option<ScriptObjectId> {
        match self.owner {
            ScriptStateOwner::Object(object) => Some(object),
            ScriptStateOwner::TrailingState => None,
        }
    }

    /// Return the zero-based index of the first word within the owning region.
    pub const fn first_word_index(self) -> usize {
        self.first_word_index
    }

    /// Return whether this pair belongs to the trailing profile-state block.
    pub const fn is_trailing_state(self) -> bool {
        matches!(self.owner, ScriptStateOwner::TrailingState)
    }
}

/// Typed byte within one owned script state object or trailing state block.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptStateByte {
    owner: ScriptStateOwner,
    byte_index: usize,
}

/// Typed interpretation of a VAR word used as an object relation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptStateObjectReference {
    /// Reference to one decoded profile object.
    Object(ScriptObjectId),
    /// Original `0xFFFF` terminator or contextual fallback marker.
    Sentinel,
}

impl ScriptStateByte {
    /// Return the object that owns this byte.
    pub const fn object(self) -> Option<ScriptObjectId> {
        match self.owner {
            ScriptStateOwner::Object(object) => Some(object),
            ScriptStateOwner::TrailingState => None,
        }
    }

    /// Return the zero-based byte index within the owning state region.
    pub const fn byte_index(self) -> usize {
        self.byte_index
    }

    /// Return whether this byte belongs to the trailing profile-state block.
    pub const fn is_trailing_state(self) -> bool {
        matches!(self.owner, ScriptStateOwner::TrailingState)
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
    source_offset: usize,
    bytes: Box<[u8]>,
}

impl ScriptStateObject {
    /// Return this record's byte position in the serialized VAR image.
    pub const fn source_offset(&self) -> usize {
        self.source_offset
    }

    /// Return the object's exact authored record bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }
}

/// Complete decoded VAR state image partitioned into typed objects and trailing data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScriptState {
    objects: Vec<ScriptStateObject>,
    trailing_source_offset: usize,
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

    /// Resolve a bounded word index within one typed object record.
    pub fn object_word(
        &self,
        object: ScriptObjectId,
        word_index: usize,
    ) -> Option<ScriptStateWord> {
        let state_object = self.object(object)?;
        let byte_offset = word_index.checked_mul(WORD_SIZE)?;
        let word_end = byte_offset.checked_add(WORD_SIZE)?;
        (word_end <= state_object.bytes.len()).then_some(ScriptStateWord {
            owner: ScriptStateOwner::Object(object),
            word_index,
        })
    }

    /// Resolve a bounded byte index within one typed object record.
    pub fn object_byte(
        &self,
        object: ScriptObjectId,
        byte_index: usize,
    ) -> Option<ScriptStateByte> {
        let state_object = self.object(object)?;
        (byte_index < state_object.bytes.len()).then_some(ScriptStateByte {
            owner: ScriptStateOwner::Object(object),
            byte_index,
        })
    }

    /// Resolve two adjacent words within one typed object record.
    pub fn object_word_pair(
        &self,
        object: ScriptObjectId,
        first_word_index: usize,
    ) -> Option<ScriptStateWordPair> {
        let state_object = self.object(object)?;
        let byte_offset = first_word_index.checked_mul(WORD_SIZE)?;
        let pair_end = byte_offset.checked_add(WORD_SIZE * 2)?;
        (pair_end <= state_object.bytes.len()).then_some(ScriptStateWordPair {
            owner: ScriptStateOwner::Object(object),
            first_word_index,
        })
    }

    /// Resolve an encoded VAR byte position to an aligned owned object word.
    pub fn resolve_word_source_offset(&self, source_offset: u16) -> Option<ScriptStateWord> {
        let source_offset = usize::from(source_offset);
        self.objects
            .iter()
            .find_map(|object| {
                let relative = source_offset.checked_sub(object.source_offset)?;
                let word_end = relative.checked_add(WORD_SIZE)?;
                (relative.is_multiple_of(WORD_SIZE) && word_end <= object.bytes.len()).then_some(
                    ScriptStateWord {
                        owner: ScriptStateOwner::Object(object.id),
                        word_index: relative / WORD_SIZE,
                    },
                )
            })
            .or_else(|| {
                let relative = source_offset.checked_sub(self.trailing_source_offset)?;
                let word_end = relative.checked_add(WORD_SIZE)?;
                (relative.is_multiple_of(WORD_SIZE) && word_end <= self.trailing_data.len())
                    .then_some(ScriptStateWord {
                        owner: ScriptStateOwner::TrailingState,
                        word_index: relative / WORD_SIZE,
                    })
            })
    }

    /// Read one resolved object word.
    pub fn word(&self, field: ScriptStateWord) -> Option<u16> {
        let offset = field.word_index.checked_mul(WORD_SIZE)?;
        let bytes = match field.owner {
            ScriptStateOwner::Object(object) => {
                self.object(object)?.bytes.get(offset..offset + WORD_SIZE)?
            }
            ScriptStateOwner::TrailingState => {
                self.trailing_data.get(offset..offset + WORD_SIZE)?
            }
        };
        Some(u16::from_le_bytes(bytes.try_into().ok()?))
    }

    /// Assign one resolved object word.
    pub fn set_word(&mut self, field: ScriptStateWord, value: u16) -> bool {
        let Some(offset) = field.word_index.checked_mul(WORD_SIZE) else {
            return false;
        };
        let bytes = match field.owner {
            ScriptStateOwner::Object(object) => {
                let Some(object) = self.objects.get_mut(object.index()) else {
                    return false;
                };
                object.bytes.get_mut(offset..offset + WORD_SIZE)
            }
            ScriptStateOwner::TrailingState => {
                self.trailing_data.get_mut(offset..offset + WORD_SIZE)
            }
        };
        let Some(bytes) = bytes else { return false };
        bytes.copy_from_slice(&value.to_le_bytes());
        true
    }

    /// Resolve an encoded VAR byte position to two adjacent owned words.
    pub fn resolve_word_pair_source_offset(
        &self,
        source_offset: u16,
    ) -> Option<ScriptStateWordPair> {
        let source_offset = usize::from(source_offset);
        self.objects
            .iter()
            .find_map(|object| {
                let relative = source_offset.checked_sub(object.source_offset)?;
                let pair_end = relative.checked_add(WORD_SIZE * 2)?;
                (relative.is_multiple_of(WORD_SIZE) && pair_end <= object.bytes.len()).then_some(
                    ScriptStateWordPair {
                        owner: ScriptStateOwner::Object(object.id),
                        first_word_index: relative / WORD_SIZE,
                    },
                )
            })
            .or_else(|| {
                let relative = source_offset.checked_sub(self.trailing_source_offset)?;
                let pair_end = relative.checked_add(WORD_SIZE * 2)?;
                (relative.is_multiple_of(WORD_SIZE) && pair_end <= self.trailing_data.len())
                    .then_some(ScriptStateWordPair {
                        owner: ScriptStateOwner::TrailingState,
                        first_word_index: relative / WORD_SIZE,
                    })
            })
    }

    /// Read one resolved adjacent word pair.
    pub fn word_pair(&self, field: ScriptStateWordPair) -> Option<[u16; 2]> {
        let offset = field.first_word_index.checked_mul(WORD_SIZE)?;
        let bytes = match field.owner {
            ScriptStateOwner::Object(object) => self
                .object(object)?
                .bytes
                .get(offset..offset + WORD_SIZE * 2)?,
            ScriptStateOwner::TrailingState => {
                self.trailing_data.get(offset..offset + WORD_SIZE * 2)?
            }
        };
        Some([
            u16::from_le_bytes(bytes[..WORD_SIZE].try_into().ok()?),
            u16::from_le_bytes(bytes[WORD_SIZE..].try_into().ok()?),
        ])
    }

    /// Assign one resolved adjacent word pair atomically.
    pub fn set_word_pair(&mut self, field: ScriptStateWordPair, value: [u16; 2]) -> bool {
        let Some(offset) = field.first_word_index.checked_mul(WORD_SIZE) else {
            return false;
        };
        let bytes = match field.owner {
            ScriptStateOwner::Object(object) => self
                .objects
                .get_mut(object.index())
                .and_then(|object| object.bytes.get_mut(offset..offset + WORD_SIZE * 2)),
            ScriptStateOwner::TrailingState => {
                self.trailing_data.get_mut(offset..offset + WORD_SIZE * 2)
            }
        };
        let Some(bytes) = bytes else { return false };
        bytes[..WORD_SIZE].copy_from_slice(&value[0].to_le_bytes());
        bytes[WORD_SIZE..].copy_from_slice(&value[1].to_le_bytes());
        true
    }

    /// Interpret one resolved word as a typed object identity or sentinel.
    pub fn object_reference(&self, field: ScriptStateWord) -> Option<ScriptStateObjectReference> {
        let encoded = self.word(field)?;
        if encoded == u16::MAX {
            return Some(ScriptStateObjectReference::Sentinel);
        }
        self.objects.iter().find_map(|object| {
            (object.source_offset == usize::from(encoded))
                .then_some(ScriptStateObjectReference::Object(object.id))
        })
    }

    /// Resolve an encoded VAR byte position to one bounded owned byte.
    pub fn resolve_byte_source_offset(&self, source_offset: u16) -> Option<ScriptStateByte> {
        let source_offset = usize::from(source_offset);
        self.objects
            .iter()
            .find_map(|object| {
                let byte_index = source_offset.checked_sub(object.source_offset)?;
                (byte_index < object.bytes.len()).then_some(ScriptStateByte {
                    owner: ScriptStateOwner::Object(object.id),
                    byte_index,
                })
            })
            .or_else(|| {
                let byte_index = source_offset.checked_sub(self.trailing_source_offset)?;
                (byte_index < self.trailing_data.len()).then_some(ScriptStateByte {
                    owner: ScriptStateOwner::TrailingState,
                    byte_index,
                })
            })
    }

    /// Read one resolved state byte.
    pub fn byte(&self, field: ScriptStateByte) -> Option<u8> {
        match field.owner {
            ScriptStateOwner::Object(object) => {
                self.object(object)?.bytes.get(field.byte_index).copied()
            }
            ScriptStateOwner::TrailingState => self.trailing_data.get(field.byte_index).copied(),
        }
    }

    /// Assign one resolved state byte.
    pub fn set_byte(&mut self, field: ScriptStateByte, value: u8) -> bool {
        let byte = match field.owner {
            ScriptStateOwner::Object(object) => self
                .objects
                .get_mut(object.index())
                .and_then(|object| object.bytes.get_mut(field.byte_index)),
            ScriptStateOwner::TrailingState => self.trailing_data.get_mut(field.byte_index),
        };
        let Some(byte) = byte else { return false };
        *byte = value;
        true
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

    /// Iterate procedures in authored directory order.
    pub fn procedures(&self) -> impl Iterator<Item = (ScriptProcedureId, &ScriptDirectoryEntry)> {
        self.entries
            .iter()
            .filter(|entry| entry.kind == ScriptSymbolKind::Procedure)
            .enumerate()
            .map(|(index, entry)| (ScriptProcedureId(index), entry))
    }

    /// Resolve a typed procedure identity back to its directory entry.
    pub fn procedure(&self, procedure: ScriptProcedureId) -> Option<&ScriptDirectoryEntry> {
        self.procedures()
            .nth(procedure.index())
            .map(|(_procedure, entry)| entry)
    }

    /// Resolve an encoded procedure enabled-byte target to a typed identity.
    ///
    /// Kind-2 DEB values are one-based COD entry positions. They therefore
    /// equal both a procedure's start plus one and the AB instruction target
    /// for its mutable A9 enabled flag.
    pub fn resolve_procedure_activation_target(
        &self,
        encoded_target: u16,
    ) -> Option<ScriptProcedureId> {
        self.procedures()
            .find_map(|(procedure, entry)| (entry.value == encoded_target).then_some(procedure))
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
            source_offset: cursor,
            bytes: Box::from(&data[cursor..end]),
        });
        cursor = end;
    }
    Ok(ScriptState {
        objects,
        trailing_source_offset: cursor,
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
