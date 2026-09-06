//! Lossless original save-game and save-slot directory formats.

use std::error::Error;
use std::fmt;

use commander_blood_formats::script::{ScriptDataError, decode_script_state};

use super::{
    LoadedScriptProfile, ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT, SAVE_SLOT_NAME_LENGTH,
    SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT, SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT,
    SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT, SaveSlotName, ScriptProcedureStateError, ScriptProfileId,
    ScriptProfileRecordStateError, ScriptSequenceSaveError, apply_procedure_patch_stream,
    build_procedure_patch_stream,
};

/// Byte count of the saved zero-based script-profile identity.
pub const ORIGINAL_SAVE_PROFILE_BYTE_COUNT: usize = 2;
/// Number of user-visible save slots, including the quick-save slot.
pub const ORIGINAL_SAVE_SLOT_COUNT: usize = 10;
/// Zero-based index of the quick-save slot.
pub const ORIGINAL_QUICK_SAVE_SLOT_INDEX: usize = ORIGINAL_SAVE_SLOT_COUNT - 1;
/// Byte count of one display-name and filename directory record.
pub const ORIGINAL_SAVE_SLOT_RECORD_BYTE_COUNT: usize = SAVE_SLOT_NAME_LENGTH * 2;
/// Exact byte count of the complete `BLOOD.SAV` slot directory.
pub const ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT: usize =
    ORIGINAL_SAVE_SLOT_COUNT * ORIGINAL_SAVE_SLOT_RECORD_BYTE_COUNT;
// Startup at BLOOD2PG file 0x1043 passes GS:0x287B to the optional file loader.
const SEQUEL_INITIAL_SAVE_DIRECTORY_FILE_OFFSET: usize = 0x1206B;
/// Fixed byte count preceding the profile-specific state and procedure blocks.
pub const ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT: usize = ORIGINAL_SAVE_PROFILE_BYTE_COUNT
    + SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT
    + SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT;

/// One exact 32-byte record in the original `BLOOD.SAV` directory.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OriginalSaveSlot {
    display_name: SaveSlotName,
    filename_field: [u8; SAVE_SLOT_NAME_LENGTH],
}

impl OriginalSaveSlot {
    /// Return the complete fixed-width editable display-name field.
    pub const fn display_name(self) -> SaveSlotName {
        self.display_name
    }

    /// Replace the complete fixed-width editable display-name field.
    pub fn set_display_name(&mut self, display_name: SaveSlotName) {
        self.display_name = display_name;
    }

    /// Return the filename bytes before their first NUL terminator.
    pub fn filename_bytes(&self) -> Option<&[u8]> {
        self.filename_field
            .iter()
            .position(|byte| *byte == u8::MIN)
            .map(|length| &self.filename_field[..length])
    }

    /// Return the complete fixed-width filename field for exact serialization.
    pub const fn filename_field(self) -> [u8; SAVE_SLOT_NAME_LENGTH] {
        self.filename_field
    }
}

/// All ten exact save-slot records stored in `BLOOD.SAV`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalSaveSlotDirectory {
    slots: [OriginalSaveSlot; ORIGINAL_SAVE_SLOT_COUNT],
}

impl OriginalSaveSlotDirectory {
    /// Decode the executable-owned defaults retained when the sequel has no BLOOD.SAV.
    pub fn decode_blood2pg_initial(
        executable: &[u8],
    ) -> Result<Self, OriginalSaveSlotDirectoryError> {
        let start = SEQUEL_INITIAL_SAVE_DIRECTORY_FILE_OFFSET;
        let bytes = executable
            .get(start..start + ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT)
            .ok_or(OriginalSaveSlotDirectoryError::InvalidByteCount {
                expected: ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT,
                actual: executable.len().saturating_sub(start),
            })?;
        Self::decode(bytes)
    }

    /// Decode exactly ten fixed-width save-slot records.
    pub fn decode(data: &[u8]) -> Result<Self, OriginalSaveSlotDirectoryError> {
        if data.len() != ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT {
            return Err(OriginalSaveSlotDirectoryError::InvalidByteCount {
                expected: ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT,
                actual: data.len(),
            });
        }

        let slots = data
            .chunks_exact(ORIGINAL_SAVE_SLOT_RECORD_BYTE_COUNT)
            .map(|record| {
                let display_name = SaveSlotName::from_bytes(
                    record[..SAVE_SLOT_NAME_LENGTH]
                        .try_into()
                        .expect("validated fixed save-slot name field"),
                );
                let filename_field = record[SAVE_SLOT_NAME_LENGTH..]
                    .try_into()
                    .expect("validated fixed save-slot filename field");
                OriginalSaveSlot {
                    display_name,
                    filename_field,
                }
            })
            .collect::<Vec<_>>()
            .try_into()
            .expect("decoded exactly ten save-slot records");
        Ok(Self { slots })
    }

    /// Return all save slots in their original order.
    pub const fn slots(&self) -> &[OriginalSaveSlot; ORIGINAL_SAVE_SLOT_COUNT] {
        &self.slots
    }

    /// Mutably return all save slots for name editing and quick-save updates.
    pub fn slots_mut(&mut self) -> &mut [OriginalSaveSlot; ORIGINAL_SAVE_SLOT_COUNT] {
        &mut self.slots
    }

    /// Re-encode the complete directory byte for byte.
    pub fn encode(&self) -> [u8; ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT] {
        let mut output = [u8::MIN; ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT];
        for (slot, record) in self
            .slots
            .iter()
            .zip(output.chunks_exact_mut(ORIGINAL_SAVE_SLOT_RECORD_BYTE_COUNT))
        {
            record[..SAVE_SLOT_NAME_LENGTH].copy_from_slice(&slot.display_name.bytes());
            record[SAVE_SLOT_NAME_LENGTH..].copy_from_slice(&slot.filename_field);
        }
        output
    }
}

/// Invalid `BLOOD.SAV` slot-directory bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OriginalSaveSlotDirectoryError {
    /// The image is not exactly ten 32-byte records.
    InvalidByteCount {
        /// Required directory byte count.
        expected: usize,
        /// Supplied directory byte count.
        actual: usize,
    },
}

impl fmt::Display for OriginalSaveSlotDirectoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Commander Blood save-slot directory: {self:?}"
        )
    }
}

impl Error for OriginalSaveSlotDirectoryError {}

/// One complete original `GAME*.SAV` image partitioned at native boundaries.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalSaveGame {
    profile: ScriptProfileId,
    timer_block: [u8; SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT],
    sequence_block: [u8; SCRIPT_SEQUENCE_SAVE_BLOCK_BYTE_COUNT],
    state_block: Box<[u8]>,
    procedure_patch_stream: Box<[u8]>,
}

impl OriginalSaveGame {
    /// Decode only the leading profile identity needed before loading its resources.
    pub fn decode_profile(data: &[u8]) -> Result<ScriptProfileId, OriginalSaveGameError> {
        if data.len() < ORIGINAL_SAVE_PROFILE_BYTE_COUNT {
            return Err(OriginalSaveGameError::Truncated {
                required: ORIGINAL_SAVE_PROFILE_BYTE_COUNT,
                actual: data.len(),
            });
        }
        let encoded_profile = u16::from_le_bytes(
            data[..ORIGINAL_SAVE_PROFILE_BYTE_COUNT]
                .try_into()
                .expect("validated two-byte save profile"),
        );
        u8::try_from(encoded_profile)
            .ok()
            .and_then(ScriptProfileId::new)
            .ok_or(OriginalSaveGameError::InvalidProfile { encoded_profile })
    }

    /// Decode one save after profile loading has supplied its rounded state size.
    ///
    /// The original obtains `state_block_byte_count` from `resource_get_field4`
    /// after selecting the saved profile. That value is the 16-byte-rounded VAR
    /// allocation size, not the loose `SCRIPT*.VAR` file length.
    pub fn decode(
        data: &[u8],
        state_block_byte_count: usize,
    ) -> Result<Self, OriginalSaveGameError> {
        let required = ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT
            .checked_add(state_block_byte_count)
            .ok_or(OriginalSaveGameError::StateBlockTooLarge {
                byte_count: state_block_byte_count,
            })?;
        if data.len() < required {
            return Err(OriginalSaveGameError::Truncated {
                required,
                actual: data.len(),
            });
        }

        let profile = Self::decode_profile(data)?;
        let timer_start = ORIGINAL_SAVE_PROFILE_BYTE_COUNT;
        let sequence_start = timer_start + SCRIPT_TIMER_SAVE_BLOCK_BYTE_COUNT;
        let state_start = ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT;
        let state_end = state_start + state_block_byte_count;
        let procedure_patch_stream = &data[state_end..];
        if !procedure_patch_stream
            .len()
            .is_multiple_of(SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT)
        {
            return Err(OriginalSaveGameError::InvalidProcedurePatchByteCount {
                byte_count: procedure_patch_stream.len(),
            });
        }

        Ok(Self {
            profile,
            timer_block: data[timer_start..sequence_start]
                .try_into()
                .expect("validated fixed timer save block"),
            sequence_block: data[sequence_start..state_start]
                .try_into()
                .expect("validated fixed sequence save block"),
            state_block: Box::from(&data[state_start..state_end]),
            procedure_patch_stream: Box::from(procedure_patch_stream),
        })
    }

    /// Capture all state written by the original save path from one loaded profile.
    pub fn capture(profile: &LoadedScriptProfile) -> Result<Self, OriginalSaveGameError> {
        let mut state_block = profile
            .synchronized_state()
            .map_err(OriginalSaveGameError::RecordState)?
            .encode();
        let state_block_byte_count = rounded_state_block_byte_count(state_block.len())?;
        state_block.resize(state_block_byte_count, u8::MIN);
        let procedure_patch_stream =
            build_procedure_patch_stream(profile.directory(), profile.procedures())
                .map_err(OriginalSaveGameError::Procedure)?;

        Ok(Self {
            profile: profile.id(),
            timer_block: profile.runtime().encode_timer_save_block(),
            sequence_block: profile.sequence_slots().encode_save_block(),
            state_block: state_block.into_boxed_slice(),
            procedure_patch_stream: procedure_patch_stream.into_boxed_slice(),
        })
    }

    /// Restore all persistent blocks into the already selected matching profile.
    ///
    /// Validation is transactional: malformed state, sequence fields, or patch
    /// records leave the loaded profile unchanged.
    pub fn restore_into(
        &self,
        profile: &mut LoadedScriptProfile,
    ) -> Result<(), OriginalSaveGameError> {
        if profile.id() != self.profile {
            return Err(OriginalSaveGameError::ProfileMismatch {
                saved: self.profile,
                loaded: profile.id(),
            });
        }

        let state = decode_script_state(&self.state_block, profile.directory())
            .map_err(OriginalSaveGameError::State)?;
        let mut procedures = profile.procedures().clone();
        apply_procedure_patch_stream(
            &self.procedure_patch_stream,
            profile.directory(),
            &mut procedures,
        )
        .map_err(OriginalSaveGameError::Procedure)?;
        let mut sequence_slots = profile.sequence_slots().clone();
        sequence_slots
            .restore_save_block(&self.sequence_block)
            .map_err(OriginalSaveGameError::Sequence)?;
        let mut runtime = profile.runtime().clone();
        runtime.restore_timer_save_block(&self.timer_block);

        profile
            .replace_state(state)
            .map_err(OriginalSaveGameError::RecordState)?;
        *profile.procedures_mut() = procedures;
        *profile.sequence_slots_mut() = sequence_slots;
        *profile.runtime_mut() = runtime;
        Ok(())
    }

    /// Return the saved zero-based script-profile identity.
    pub const fn profile(&self) -> ScriptProfileId {
        self.profile
    }

    /// Return the exact rounded VAR allocation bytes.
    pub fn state_block(&self) -> &[u8] {
        &self.state_block
    }

    /// Return the exact typed-procedure patch stream.
    pub fn procedure_patch_stream(&self) -> &[u8] {
        &self.procedure_patch_stream
    }

    /// Re-encode the complete save image in native write order.
    pub fn encode(&self) -> Vec<u8> {
        let capacity = ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT
            + self.state_block.len()
            + self.procedure_patch_stream.len();
        let mut output = Vec::with_capacity(capacity);
        output.extend_from_slice(&u16::from(self.profile.value()).to_le_bytes());
        output.extend_from_slice(&self.timer_block);
        output.extend_from_slice(&self.sequence_block);
        output.extend_from_slice(&self.state_block);
        output.extend_from_slice(&self.procedure_patch_stream);
        output
    }
}

/// Return the state byte count queried by the original save/load routine.
pub fn original_save_state_block_byte_count(
    profile: &LoadedScriptProfile,
) -> Result<usize, OriginalSaveGameError> {
    rounded_state_block_byte_count(profile.state().encode().len())
}

fn rounded_state_block_byte_count(byte_count: usize) -> Result<usize, OriginalSaveGameError> {
    byte_count
        .checked_add(ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT - 1)
        .map(|value| {
            value / ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT * ORIGINAL_RESOURCE_ALLOCATION_ALIGNMENT
        })
        .ok_or(OriginalSaveGameError::StateBlockTooLarge { byte_count })
}

/// Invalid original save bytes or an incompatible loaded profile.
#[derive(Debug)]
pub enum OriginalSaveGameError {
    /// The file ends before the fixed header and rounded state block.
    Truncated {
        /// Minimum byte count implied by the selected profile.
        required: usize,
        /// Actual file byte count.
        actual: usize,
    },
    /// The saved word is not one of the five playable zero-based profiles.
    InvalidProfile {
        /// Unrecognized serialized profile word.
        encoded_profile: u16,
    },
    /// Rounding a profile state image exceeded the host collection domain.
    StateBlockTooLarge {
        /// Unrounded state byte count.
        byte_count: usize,
    },
    /// The trailing procedure data ends within a three-byte record.
    InvalidProcedurePatchByteCount {
        /// Actual trailing byte count.
        byte_count: usize,
    },
    /// Restore was attempted before selecting the profile named by the save.
    ProfileMismatch {
        /// Profile encoded by the save.
        saved: ScriptProfileId,
        /// Currently loaded profile.
        loaded: ScriptProfileId,
    },
    /// The saved VAR allocation is not valid for the selected directory.
    State(ScriptDataError),
    /// Saved VAR records could not be rebuilt into coherent typed handler state.
    RecordState(ScriptProfileRecordStateError),
    /// The saved procedure patch stream is not valid for the selected directory.
    Procedure(ScriptProcedureStateError),
    /// One saved sequence-name field is not safely terminated.
    Sequence(ScriptSequenceSaveError),
}

impl fmt::Display for OriginalSaveGameError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Commander Blood save game: {self:?}")
    }
}

impl Error for OriginalSaveGameError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::State(source) => Some(source),
            Self::RecordState(source) => Some(source),
            Self::Procedure(source) => Some(source),
            Self::Sequence(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::super::{
        OriginalResourceCache, OriginalResourceCatalog, OriginalScriptProfileCatalog,
        ScriptProfileManager,
    };
    use crate::assets::OriginalResourceStore;

    use super::*;

    const SECOND_PROFILE: ScriptProfileId = ScriptProfileId::new(1).unwrap();
    const SECOND_PROFILE_STATE_FILE_BYTE_COUNT: usize = 4_882;
    const SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT: usize = 4_896;
    const SECOND_PROFILE_PROCEDURE_COUNT: usize = 127;

    fn original_asset_root() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
    }

    fn original_asset(name: &str) -> PathBuf {
        original_asset_root().join(name)
    }

    fn load_profile(profile: ScriptProfileId) -> LoadedScriptProfile {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let resource_catalog = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let profile_catalog = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let store = OriginalResourceStore::new(original_asset_root(), None, [], true);
        let mut cache = OriginalResourceCache::new();
        let mut manager = ScriptProfileManager::new(profile_catalog);
        manager
            .select(profile, &mut cache, &store, &resource_catalog)
            .unwrap();
        manager.current().unwrap().clone()
    }

    #[test]
    #[ignore = "requires the original sequel executable and captured startup snapshots"]
    fn sequel_initial_save_directory_matches_live_missing_file_startup() {
        use sha2::{Digest, Sha256};
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang");
        let executable = std::fs::read(root.join("disc/BLOOD2PG.EXE")).unwrap();
        let directory = crate::game::GameVariant::BigBugBang
            .decode_initial_save_directory(&executable)
            .unwrap();
        assert_eq!(
            format!("{:x}", Sha256::digest(directory.encode())),
            "ee6edd76ecc5c2bb204661451de5c97a8971f8a0033ff6483345ec2027e1fc2b"
        );
        for (index, slot) in directory.slots().iter().enumerate() {
            assert_eq!(
                slot.filename_bytes().unwrap(),
                format!("game{}.sav", index + 1).as_bytes()
            );
            assert_eq!(slot.display_name().bytes(), *b"               \0");
        }
        let capture_root = root.join("startup-capture-15");
        let capture: serde_json::Value =
            serde_json::from_slice(&std::fs::read(capture_root.join("capture.json")).unwrap())
                .unwrap();
        assert_eq!(
            capture["executable_sha256"],
            format!("{:x}", Sha256::digest(&executable))
        );
        assert!(!capture_root.join("cdrive/cblood/blood.sav").exists());
        assert!(!capture_root.join("cdrive/cblood/BLOOD.SAV").exists());
        let mut checked = 0;
        for sample in capture["samples"].as_array().unwrap() {
            let (Some(dump), Some(segment)) = (
                sample["guest_dump"].as_str(),
                sample["global_segment"].as_u64(),
            ) else {
                continue;
            };
            let guest = std::fs::read(capture_root.join(dump)).unwrap();
            let start = segment as usize * 16 + 0x287B;
            assert_eq!(
                &guest[start..start + ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT],
                &directory.encode()
            );
            checked += 1;
        }
        assert_eq!(checked, 7);
    }

    #[test]
    fn shipped_slot_directory_round_trips_every_raw_byte() {
        let data = std::fs::read(original_asset("BLOOD.SAV")).unwrap();
        let directory = OriginalSaveSlotDirectory::decode(&data).unwrap();

        assert_eq!(directory.encode().as_slice(), data);
        assert_eq!(
            directory.slots()[usize::MIN].filename_bytes().unwrap(),
            b"game1.sav"
        );
        assert_eq!(
            directory.slots()[ORIGINAL_QUICK_SAVE_SLOT_INDEX]
                .filename_bytes()
                .unwrap(),
            b"game10.sav"
        );
        assert_eq!(
            &directory.slots()[ORIGINAL_QUICK_SAVE_SLOT_INDEX]
                .display_name()
                .bytes()[..4],
            b"LAST"
        );
    }

    #[test]
    fn shipped_save_uses_rounded_var_allocation_and_round_trips_exactly() {
        let data = std::fs::read(original_asset("GAME1.SAV")).unwrap();
        let state_file = std::fs::read(original_asset("SCRIPT2.VAR")).unwrap();
        assert_eq!(state_file.len(), SECOND_PROFILE_STATE_FILE_BYTE_COUNT);
        assert_eq!(
            OriginalSaveGame::decode_profile(&data).unwrap(),
            SECOND_PROFILE
        );
        let save = OriginalSaveGame::decode(&data, SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT).unwrap();

        assert_eq!(save.profile(), SECOND_PROFILE);
        assert_eq!(
            save.state_block().len(),
            SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT
        );
        assert_eq!(
            save.procedure_patch_stream().len(),
            SECOND_PROFILE_PROCEDURE_COUNT * SCRIPT_PROCEDURE_PATCH_RECORD_BYTE_COUNT
        );
        assert_eq!(
            &save.state_block()[SECOND_PROFILE_STATE_FILE_BYTE_COUNT..],
            &[u8::MIN;
                SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT - SECOND_PROFILE_STATE_FILE_BYTE_COUNT]
        );
        assert_eq!(save.encode(), data);
    }

    #[test]
    fn shipped_save_restores_and_recaptures_the_typed_profile_exactly() {
        let data = std::fs::read(original_asset("GAME1.SAV")).unwrap();
        let mut profile = load_profile(SECOND_PROFILE);
        let state_block_byte_count = original_save_state_block_byte_count(&profile).unwrap();
        assert_eq!(
            state_block_byte_count,
            SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT
        );
        let save = OriginalSaveGame::decode(&data, state_block_byte_count).unwrap();

        save.restore_into(&mut profile).unwrap();
        let recaptured = OriginalSaveGame::capture(&profile).unwrap();

        assert_eq!(recaptured.encode(), data);
    }

    #[test]
    fn malformed_boundaries_and_profile_mismatches_are_rejected() {
        let data = std::fs::read(original_asset("GAME1.SAV")).unwrap();
        assert!(matches!(
            OriginalSaveGame::decode_profile(&data[..ORIGINAL_SAVE_PROFILE_BYTE_COUNT - 1]),
            Err(OriginalSaveGameError::Truncated { .. })
        ));
        assert!(matches!(
            OriginalSaveGame::decode(&data[..ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT - 1], 0),
            Err(OriginalSaveGameError::Truncated { .. })
        ));
        assert!(matches!(
            OriginalSaveGame::decode(&data, SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT + 1),
            Err(OriginalSaveGameError::InvalidProcedurePatchByteCount { .. })
        ));
        assert!(matches!(
            OriginalSaveSlotDirectory::decode(
                &[u8::MIN; ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT - 1]
            ),
            Err(OriginalSaveSlotDirectoryError::InvalidByteCount { .. })
        ));

        let save = OriginalSaveGame::decode(&data, SECOND_PROFILE_STATE_BLOCK_BYTE_COUNT).unwrap();
        let mut wrong_profile = load_profile(ScriptProfileId::new(2).unwrap());
        assert!(matches!(
            save.restore_into(&mut wrong_profile),
            Err(OriginalSaveGameError::ProfileMismatch { .. })
        ));
    }
}
