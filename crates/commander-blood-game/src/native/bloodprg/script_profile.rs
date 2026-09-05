//! Loading and ownership of complete authored BloodScript profiles.

mod record_state;

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use commander_blood_formats::bas::{ScriptBas, ScriptBasError, decode_script_bas};
use commander_blood_formats::code::{
    ScriptCode, ScriptCodeError, ScriptCodeOffset, ScriptDialect, ScriptToken,
    decode_script_code_for_dialect,
};
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstructionError, decode_complete_script_instruction,
    decode_script_procedure_gate,
};
use commander_blood_formats::script::{
    ScriptDataError, ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptState,
    ScriptSymbolKind, decode_script_dictionary, decode_script_directory,
    decode_script_state_for_dialect,
};

use crate::assets::OriginalResourceStore;

use super::script_frame::{
    DecodedScriptFrameHost, ScriptFrameError, ScriptFrameOutcome, execute_decoded_script_frame,
};
use super::{
    OriginalResourceCache, OriginalResourceCatalog, ResourceCacheError, ResourceId,
    ResourceLoadStatus, ScriptActionRecord, ScriptFieldSelector, ScriptProcedureStateError,
    ScriptProcedureStates, ScriptRuntime, ScriptSelectorState, ScriptSequenceSlots,
    script_field_offset,
};

pub use record_state::{ScriptProfileRecordState, ScriptProfileRecordStateError};

/// File position of the five playable resource profiles in `BLOODPRG.EXE`.
pub const BLOODPRG_SCRIPT_PROFILE_TABLE_FILE_OFFSET: usize = 0x00D3E4;
/// Number of playable script profiles shipped by Commander Blood.
pub const ORIGINAL_SCRIPT_PROFILE_COUNT: usize = 5;
const BIG_BUG_BANG_SCRIPT_PROFILE_COUNT: usize = 17;
const BLOOD2PG_SCRIPT_PROFILE_TABLE_FILE_OFFSET: usize = 0xF744;
const SEQUEL_NATIVE_RESOURCE_ORDER: [usize; SCRIPT_PROFILE_RESOURCE_COUNT] = [2, 3, 0, 4, 1];
/// Number of companion files loaded for each script profile.
pub const SCRIPT_PROFILE_RESOURCE_COUNT: usize = 5;

const SERIALIZED_RESOURCE_ID_SIZE: usize = 2;
const SENTINEL_PROFILE_COUNT: usize = 1;
const PROCEDURE_GATE_OPCODE: u8 = 0xA9;
const BUILTIN_PLAYER_NAME: &[u8] = b"blood";
const BUILTIN_WORLD_NAME: &[u8] = b"orxx";
const BUILTIN_HORN_NAME: &[u8] = b"Honk";
const BUILTIN_MENU_NAME: &[u8] = b"menu";
const BUILTIN_ARCHETYPE_NAME: &[u8] = b"arche";
const BUILTIN_ARK_NAME: &[u8] = b"Ark";
const BUILTIN_SCRUTER_JO_NAME: &[u8] = b"Scruter_Jo";
const BUILTIN_VIDEO_STATE_NAME: &[u8] = b"vbio";

/// Zero-based identity of one playable BloodScript profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ScriptProfileId(u8);

impl ScriptProfileId {
    /// Profile selected by the executable's initial pending-request word.
    ///
    /// The shipped image stores zero at game-data offset `0x6780`; the main
    /// loop consumes that request after the blocking opening presentation.
    pub const INITIAL: Self = Self(u8::MIN);

    /// Validate a numeric profile identity against the five shipped profiles.
    pub const fn new(value: u8) -> Option<Self> {
        Self::new_for_dialect(value, ScriptDialect::CommanderBlood)
    }

    /// Validate a profile against the selected game's authored profile domain.
    pub const fn new_for_dialect(value: u8, dialect: ScriptDialect) -> Option<Self> {
        let count = match dialect {
            ScriptDialect::CommanderBlood => ORIGINAL_SCRIPT_PROFILE_COUNT,
            ScriptDialect::BigBugBang => BIG_BUG_BANG_SCRIPT_PROFILE_COUNT,
        };
        if (value as usize) < count {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Return the original zero-based numeric identity.
    pub const fn value(self) -> u8 {
        self.0
    }

    /// Iterate every playable profile in authored order.
    pub fn all() -> impl Iterator<Item = Self> {
        (u8::MIN..ORIGINAL_SCRIPT_PROFILE_COUNT as u8).map(Self)
    }

    const fn index(self) -> usize {
        self.0 as usize
    }
}

/// Semantic role of one file in a BloodScript profile.
#[repr(usize)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptProfileResourceKind {
    /// Executable VM instruction image (`SCRIPT*.COD`).
    Code = 0,
    /// Dialogue and menu instruction image (`SCRIPT*.BAS`).
    Dialogue = 1,
    /// Mutable object-state image (`SCRIPT*.VAR`).
    State = 2,
    /// Interned word dictionary (`SCRIPT*.DIC`).
    Dictionary = 3,
    /// Object, procedure, and label directory (`SCRIPT*.DEB`).
    Directory = 4,
}

impl ScriptProfileResourceKind {
    const fn index(self) -> usize {
        self as usize
    }
}

/// Five original resource IDs composing one complete BloodScript profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProfileResources {
    resources: [ResourceId; SCRIPT_PROFILE_RESOURCE_COUNT],
}

impl ScriptProfileResources {
    /// Return the resource ID assigned to one semantic companion-file role.
    pub const fn resource(self, kind: ScriptProfileResourceKind) -> ResourceId {
        self.resources[kind.index()]
    }

    /// Return all five IDs in native load order.
    pub const fn all(self) -> [ResourceId; SCRIPT_PROFILE_RESOURCE_COUNT] {
        self.resources
    }
}

/// Resource-ID matrix recovered from the original executable.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalScriptProfileCatalog {
    dialect: ScriptDialect,
    profiles: Box<[ScriptProfileResources]>,
}

impl OriginalScriptProfileCatalog {
    /// Decode the five playable rows and verify the following all-zero sentinel.
    pub fn decode_bloodprg(executable: &[u8]) -> Result<Self, ScriptProfileError> {
        let serialized_profile_size = SCRIPT_PROFILE_RESOURCE_COUNT * SERIALIZED_RESOURCE_ID_SIZE;
        let required = BLOODPRG_SCRIPT_PROFILE_TABLE_FILE_OFFSET
            + (ORIGINAL_SCRIPT_PROFILE_COUNT + SENTINEL_PROFILE_COUNT) * serialized_profile_size;
        if executable.len() < required {
            return Err(ScriptProfileError::ExecutableTooShort {
                required,
                actual: executable.len(),
            });
        }

        let mut profiles = Vec::with_capacity(ORIGINAL_SCRIPT_PROFILE_COUNT);
        for profile_index in 0..ORIGINAL_SCRIPT_PROFILE_COUNT {
            let start =
                BLOODPRG_SCRIPT_PROFILE_TABLE_FILE_OFFSET + profile_index * serialized_profile_size;
            let resources = std::array::from_fn(|resource_index| {
                let position = start + resource_index * SERIALIZED_RESOURCE_ID_SIZE;
                ResourceId::new(u16::from_le_bytes(
                    executable[position..position + SERIALIZED_RESOURCE_ID_SIZE]
                        .try_into()
                        .expect("validated script-profile resource ID"),
                ))
            });
            profiles.push(ScriptProfileResources { resources });
        }

        let sentinel_start = BLOODPRG_SCRIPT_PROFILE_TABLE_FILE_OFFSET
            + ORIGINAL_SCRIPT_PROFILE_COUNT * serialized_profile_size;
        let sentinel_end = sentinel_start + serialized_profile_size;
        if executable[sentinel_start..sentinel_end]
            .iter()
            .any(|byte| *byte != u8::MIN)
        {
            return Err(ScriptProfileError::InvalidProfileSentinel);
        }

        Ok(Self {
            dialect: ScriptDialect::CommanderBlood,
            profiles: profiles.into_boxed_slice(),
        })
    }

    /// Decode the sequel's native VAR/DEB/COD/BAS/DIC rows into semantic roles.
    pub fn decode_blood2pg(executable: &[u8]) -> Result<Self, ScriptProfileError> {
        let row_bytes = SCRIPT_PROFILE_RESOURCE_COUNT * SERIALIZED_RESOURCE_ID_SIZE;
        let sentinel = BLOOD2PG_SCRIPT_PROFILE_TABLE_FILE_OFFSET
            + BIG_BUG_BANG_SCRIPT_PROFILE_COUNT * row_bytes;
        let required = sentinel + SERIALIZED_RESOURCE_ID_SIZE;
        if executable.len() < required {
            return Err(ScriptProfileError::ExecutableTooShort {
                required,
                actual: executable.len(),
            });
        }
        if executable[sentinel..required] != [0, 0] {
            return Err(ScriptProfileError::InvalidProfileSentinel);
        }
        let profiles = (0..BIG_BUG_BANG_SCRIPT_PROFILE_COUNT)
            .map(|profile| {
                let start = BLOOD2PG_SCRIPT_PROFILE_TABLE_FILE_OFFSET + profile * row_bytes;
                let resources = SEQUEL_NATIVE_RESOURCE_ORDER.map(|native_index| {
                    let offset = start + native_index * SERIALIZED_RESOURCE_ID_SIZE;
                    ResourceId::new(u16::from_le_bytes(
                        executable[offset..offset + SERIALIZED_RESOURCE_ID_SIZE]
                            .try_into()
                            .unwrap(),
                    ))
                });
                ScriptProfileResources { resources }
            })
            .collect();
        Ok(Self {
            dialect: ScriptDialect::BigBugBang,
            profiles,
        })
    }

    /// Native VM and state-layout dialect carried by this catalog.
    pub const fn dialect(&self) -> ScriptDialect {
        self.dialect
    }

    /// Return one playable profile's resource IDs.
    pub fn profile(&self, profile: ScriptProfileId) -> ScriptProfileResources {
        self.profiles[profile.index()]
    }
}

/// Typed bindings for the names treated specially by the native VM.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ScriptProfileBuiltins {
    /// Global player object named `blood`.
    pub player: Option<ScriptObjectId>,
    /// Global world-state object named `orxx`.
    pub world: Option<ScriptObjectId>,
    /// Horn-control object named `Honk`.
    pub horn: Option<ScriptObjectId>,
    /// Menu-control object named `menu`.
    pub menu: Option<ScriptObjectId>,
    /// Archetype or current-position object named `arche`.
    pub archetype: Option<ScriptObjectId>,
    /// Ark navigation object named `Ark`.
    pub ark: Option<ScriptObjectId>,
    /// Character object named `Scruter_Jo`, absent from profile one.
    pub scruter_jo: Option<ScriptObjectId>,
    /// State-image byte position named `vbio`, absent from profile one.
    pub video_state_offset: Option<u16>,
}

impl ScriptProfileBuiltins {
    fn bind(directory: &ScriptDirectory) -> Self {
        let video_state_offset = directory.entries().iter().find_map(|entry| {
            (entry.kind == ScriptSymbolKind::StateLabel && entry.name() == BUILTIN_VIDEO_STATE_NAME)
                .then_some(entry.value)
        });
        Self {
            player: directory.find_active_object(BUILTIN_PLAYER_NAME),
            world: directory.find_active_object(BUILTIN_WORLD_NAME),
            horn: directory.find_active_object(BUILTIN_HORN_NAME),
            menu: directory.find_active_object(BUILTIN_MENU_NAME),
            archetype: directory.find_active_object(BUILTIN_ARCHETYPE_NAME),
            ark: directory.find_active_object(BUILTIN_ARK_NAME),
            scruter_jo: directory.find_active_object(BUILTIN_SCRUTER_JO_NAME),
            video_state_offset,
        }
    }
}

/// One completely decoded and independently owned BloodScript profile.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LoadedScriptProfile {
    id: ScriptProfileId,
    resources: ScriptProfileResources,
    code: ScriptCode,
    instructions: Box<[DecodedScriptInstruction]>,
    dialogue: ScriptBas,
    state: ScriptState,
    dictionary: ScriptDictionary,
    directory: ScriptDirectory,
    builtins: ScriptProfileBuiltins,
    procedures: ScriptProcedureStates,
    runtime: ScriptRuntime,
    selector_state: ScriptSelectorState,
    sequence_slots: ScriptSequenceSlots,
    record_state: ScriptProfileRecordState,
}

/// Disjoint flat borrows needed to execute one loaded BloodScript profile.
pub struct LoadedScriptExecutionParts<'a> {
    /// Losslessly framed COD image used for source-position traversal.
    pub code: &'a ScriptCode,
    /// Pre-bound semantic instruction parallel to each framed COD token.
    pub instructions: &'a [DecodedScriptInstruction],
    /// Decoded BAS dialogue and menu program.
    pub dialogue: &'a ScriptBas,
    /// Mutable VAR object and trailing state.
    pub state: &'a mut ScriptState,
    /// Interned DIC words referenced by COD and BAS instructions.
    pub dictionary: &'a ScriptDictionary,
    /// DEB object, procedure, and label bindings.
    pub directory: &'a ScriptDirectory,
    /// Native specially named object bindings.
    pub builtins: ScriptProfileBuiltins,
    /// Mutable procedure enable-state table.
    pub procedures: &'a mut ScriptProcedureStates,
    /// Main COD control-flow and timer state.
    pub runtime: &'a mut ScriptRuntime,
    /// BAS selector branches and concept history.
    pub selector_state: &'a mut ScriptSelectorState,
    /// Persistent DESCRIPT sequence-name bindings.
    pub sequence_slots: &'a mut ScriptSequenceSlots,
    /// VAR-backed typed values shared by record and post-frame action handlers.
    pub record_state: &'a mut ScriptProfileRecordState,
}

impl LoadedScriptProfile {
    /// Return this profile's zero-based identity.
    pub const fn id(&self) -> ScriptProfileId {
        self.id
    }

    /// Return the five original resource IDs backing this profile.
    pub const fn resources(&self) -> ScriptProfileResources {
        self.resources
    }

    /// Borrow the decoded COD instruction image.
    pub const fn code(&self) -> &ScriptCode {
        &self.code
    }

    /// Borrow the typed COD instructions in the same order as the lossless tokens.
    pub fn instructions(&self) -> &[DecodedScriptInstruction] {
        &self.instructions
    }

    /// Resolve a source position to its pre-bound semantic instruction.
    pub fn instruction_at(
        &self,
        source_offset: ScriptCodeOffset,
    ) -> Option<&DecodedScriptInstruction> {
        let index = self
            .code
            .tokens()
            .binary_search_by_key(&source_offset, ScriptToken::source_offset)
            .ok()?;
        self.instructions.get(index)
    }

    /// Borrow the decoded BAS dialogue and menu image.
    pub const fn dialogue(&self) -> &ScriptBas {
        &self.dialogue
    }

    /// Borrow the mutable profile object-state image.
    pub const fn state(&self) -> &ScriptState {
        &self.state
    }

    /// Mutably borrow the profile object-state image.
    pub fn state_mut(&mut self) -> &mut ScriptState {
        &mut self.state
    }

    /// Borrow the interned profile dictionary.
    pub const fn dictionary(&self) -> &ScriptDictionary {
        &self.dictionary
    }

    /// Borrow the decoded object and procedure directory.
    pub const fn directory(&self) -> &ScriptDirectory {
        &self.directory
    }

    /// Return the native VM's specially named object bindings.
    pub const fn builtins(&self) -> ScriptProfileBuiltins {
        self.builtins
    }

    /// Borrow the canonical typed record stores recovered from this profile's VAR image.
    pub const fn record_state(&self) -> &ScriptProfileRecordState {
        &self.record_state
    }

    /// Return the related object in `blood`'s live C4 presentation record.
    ///
    /// The native scene coordinators read this action slot directly every frame;
    /// it is independent of whichever DESCRIPT lookup most recently populated
    /// persistent presentation assets.
    pub fn active_actor_presentation_related(&self) -> Option<ScriptObjectId> {
        let player = self.builtins.player?;
        let player_kind = self.state.object(player)?.kind;
        let action_offset = script_field_offset(player_kind, ScriptFieldSelector::ACTION)?;
        let slot = self
            .state
            .object_word_triple(player, action_offset / size_of::<u16>())?;
        match self.record_state.action_records.record(slot) {
            ScriptActionRecord::ActorPresentation(related) => Some(related),
            _ => None,
        }
    }

    /// Borrow the profile's mutable procedure-gate state.
    pub const fn procedures(&self) -> &ScriptProcedureStates {
        &self.procedures
    }

    /// Mutably borrow procedure-gate state for A9, AB, and save restoration.
    pub fn procedures_mut(&mut self) -> &mut ScriptProcedureStates {
        &mut self.procedures
    }

    /// Borrow the fresh control-flow runtime associated with this profile.
    pub const fn runtime(&self) -> &ScriptRuntime {
        &self.runtime
    }

    /// Mutably borrow this profile's control-flow runtime.
    pub fn runtime_mut(&mut self) -> &mut ScriptRuntime {
        &mut self.runtime
    }

    /// Borrow the profile's dialogue selector and concept-history state.
    pub const fn selector_state(&self) -> &ScriptSelectorState {
        &self.selector_state
    }

    /// Mutably borrow the profile's dialogue selector and concept-history state.
    pub fn selector_state_mut(&mut self) -> &mut ScriptSelectorState {
        &mut self.selector_state
    }

    /// Borrow the profile's six DESCRIPT sequence-name bindings.
    pub const fn sequence_slots(&self) -> &ScriptSequenceSlots {
        &self.sequence_slots
    }

    /// Mutably borrow the profile's sequence-name bindings for CC assignments.
    pub fn sequence_slots_mut(&mut self) -> &mut ScriptSequenceSlots {
        &mut self.sequence_slots
    }

    /// Borrow every independently owned profile component needed by one VM frame.
    pub fn execution_parts(&mut self) -> LoadedScriptExecutionParts<'_> {
        LoadedScriptExecutionParts {
            code: &self.code,
            instructions: &self.instructions,
            dialogue: &self.dialogue,
            state: &mut self.state,
            dictionary: &self.dictionary,
            directory: &self.directory,
            builtins: self.builtins,
            procedures: &mut self.procedures,
            runtime: &mut self.runtime,
            selector_state: &mut self.selector_state,
            sequence_slots: &mut self.sequence_slots,
            record_state: &mut self.record_state,
        }
    }

    /// Clone state with every typed record store committed to its VAR fields.
    pub fn synchronized_state(&self) -> Result<ScriptState, ScriptProfileRecordStateError> {
        let mut state = self.state.clone();
        self.record_state
            .synchronize_into(&mut state, &self.directory, &self.dictionary)?;
        Ok(state)
    }

    /// Replace VAR bytes and transactionally rebuild every derived record store.
    pub fn replace_state(
        &mut self,
        state: ScriptState,
    ) -> Result<(), ScriptProfileRecordStateError> {
        let record_state = ScriptProfileRecordState::recover(
            &self.instructions,
            &state,
            &self.dictionary,
            self.builtins,
        )?;
        self.state = state;
        self.record_state = record_state;
        Ok(())
    }

    /// Execute one frame through the retained semantic COD stream.
    pub fn execute_frame<Host: DecodedScriptFrameHost>(
        &mut self,
        execution_enabled: bool,
        host: &mut Host,
    ) -> Result<ScriptFrameOutcome, ScriptFrameError<Host::Error>> {
        execute_decoded_script_frame(
            &self.code,
            &self.instructions,
            execution_enabled,
            &mut self.runtime,
            host,
        )
    }
}

/// Resource effects produced by selecting one script profile.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScriptProfileLoadOutcome {
    /// Whether the selected identity differs from the prior loaded profile.
    pub profile_changed: bool,
    /// Number of prior profile resources removed before loading.
    pub released_resources: usize,
    /// Fresh-or-resident result for each file in semantic resource-kind order.
    pub resource_statuses: [ResourceLoadStatus; SCRIPT_PROFILE_RESOURCE_COUNT],
}

/// Owner of the active decoded profile and its replacement lifecycle.
#[derive(Clone, Debug)]
pub struct ScriptProfileManager {
    catalog: OriginalScriptProfileCatalog,
    current: Option<LoadedScriptProfile>,
}

impl ScriptProfileManager {
    /// Construct a manager from the executable's decoded profile matrix.
    pub const fn new(catalog: OriginalScriptProfileCatalog) -> Self {
        Self {
            catalog,
            current: None,
        }
    }

    /// Load, decode, bind, and reset one complete script profile.
    ///
    /// This is the typed ownership translation of `vm_resource_profile_select`
    /// at BLOODPRG file offset `0x0053A0`. Selecting a different profile first
    /// releases all five old resource IDs. Selecting the same profile reuses the
    /// bytes but still rebuilds decoded state and a fresh [`ScriptRuntime`]. The
    /// sequence-name fields and reserved half of the save block remain global,
    /// matching the native data region that profile selection does not clear.
    pub fn select(
        &mut self,
        profile: ScriptProfileId,
        cache: &mut OriginalResourceCache,
        store: &OriginalResourceStore,
        resources: &OriginalResourceCatalog,
    ) -> Result<ScriptProfileLoadOutcome, ScriptProfileError> {
        let mut profile_resources = *self
            .catalog
            .profiles
            .get(profile.index())
            .ok_or(ScriptProfileError::ProfileOutsideCatalog { profile })?;
        let skip_state_resource = self.catalog.dialect == ScriptDialect::BigBugBang
            && profile != ScriptProfileId::INITIAL;
        // Native cache hits retain live VAR bytes, including a repeated initial
        // profile selection. Only switching back to profile zero reloads VAR.
        let retain_state = skip_state_resource
            || (self.catalog.dialect == ScriptDialect::BigBugBang
                && self
                    .current
                    .as_ref()
                    .is_some_and(|current| current.id == profile));
        let retained_state = if retain_state {
            let current = self
                .current
                .as_ref()
                .ok_or(ScriptProfileError::MissingPersistentState)?;
            profile_resources.resources[ScriptProfileResourceKind::State.index()] =
                current.resources.resource(ScriptProfileResourceKind::State);
            Some((
                current
                    .synchronized_state()
                    .map_err(ScriptProfileError::RecordState)?,
                current.directory.clone(),
            ))
        } else {
            None
        };
        let previous_runtime = self.current.as_ref().map(|current| current.runtime.clone());
        let retained_sequence_slots = self
            .current
            .as_ref()
            .map(|current| current.sequence_slots.clone())
            .unwrap_or_default();
        let profile_changed = self
            .current
            .as_ref()
            .is_none_or(|current| current.id != profile);
        let released_resources = if profile_changed {
            self.current
                .take()
                .map(|current| {
                    current
                        .resources
                        .all()
                        .into_iter()
                        .filter(|resource| {
                            !retain_state
                                || *resource
                                    != current.resources.resource(ScriptProfileResourceKind::State)
                        })
                        .filter(|resource| cache.release(*resource))
                        .count()
                })
                .unwrap_or(usize::MIN)
        } else {
            usize::MIN
        };

        let mut resource_statuses =
            [ResourceLoadStatus::AlreadyLoaded; SCRIPT_PROFILE_RESOURCE_COUNT];
        for (index, resource) in profile_resources.all().into_iter().enumerate() {
            if skip_state_resource && index == ScriptProfileResourceKind::State.index() {
                continue;
            }
            resource_statuses[index] = cache
                .load_by_id(store, resources, resource)
                .map_err(ScriptProfileError::Resource)?;
        }

        let mut loaded = decode_loaded_profile(
            profile,
            profile_resources,
            cache,
            self.catalog.dialect,
            retained_state,
        )?;
        if let Some(previous_runtime) = &previous_runtime {
            if skip_state_resource {
                loaded
                    .runtime
                    .restore_timer_save_block(&previous_runtime.encode_timer_save_block());
            } else {
                loaded
                    .runtime
                    .preserve_timer_save_reserved_bytes(previous_runtime);
            }
        }
        loaded.sequence_slots = retained_sequence_slots;
        self.current = Some(loaded);
        Ok(ScriptProfileLoadOutcome {
            profile_changed,
            released_resources,
            resource_statuses,
        })
    }

    /// Borrow the currently selected profile, when one has loaded successfully.
    pub const fn current(&self) -> Option<&LoadedScriptProfile> {
        self.current.as_ref()
    }

    /// Mutably borrow the currently selected profile.
    pub fn current_mut(&mut self) -> Option<&mut LoadedScriptProfile> {
        self.current.as_mut()
    }
}

/// Companion image that failed typed profile decoding.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScriptProfileDataKind {
    /// Mutable VAR object-state image.
    State,
    /// DIC interned-word image.
    Dictionary,
    /// DEB object and procedure directory.
    Directory,
}

/// Invalid profile matrix, resource operation, or companion image.
#[derive(Debug)]
pub enum ScriptProfileError {
    /// A valid identity from another game exceeds this catalog's profile domain.
    ProfileOutsideCatalog {
        /// Requested identity that is not authored by this catalog.
        profile: ScriptProfileId,
    },
    /// Noninitial sequel profiles require the already-loaded live VAR image.
    MissingPersistentState,
    /// A new profile changes the identities or boundaries of retained state objects.
    IncompatiblePersistentDirectory,
    /// The executable ends before the matrix and sentinel are complete.
    ExecutableTooShort {
        /// Minimum required executable byte count.
        required: usize,
        /// Actual executable byte count.
        actual: usize,
    },
    /// The sixth nonplayable matrix row was not entirely zero.
    InvalidProfileSentinel,
    /// Loading or releasing one original resource failed.
    Resource(ResourceCacheError),
    /// A loaded cache entry disappeared before profile decoding.
    MissingLoadedResource {
        /// Missing original resource identifier.
        resource: ResourceId,
    },
    /// The COD instruction image failed lossless framing.
    Code(ScriptCodeError),
    /// A procedure entry could not be resolved against the decoded directory.
    ProcedureInstruction(ScriptInstructionError),
    /// A COD token could not be bound to complete typed profile state.
    Instruction(ScriptInstructionError),
    /// VAR record fields could not be recovered into coherent typed handler state.
    RecordState(ScriptProfileRecordStateError),
    /// Procedure gates do not form one complete typed state table.
    ProcedureState(ScriptProcedureStateError),
    /// The BAS dialogue image failed typed decoding.
    Dialogue(ScriptBasError),
    /// A VAR, DIC, or DEB companion image failed typed decoding.
    Data {
        /// Companion image being decoded.
        kind: ScriptProfileDataKind,
        /// Underlying format error.
        source: ScriptDataError,
    },
}

impl fmt::Display for ScriptProfileError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid BloodScript profile: {self:?}")
    }
}

impl Error for ScriptProfileError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Resource(source) => Some(source),
            Self::Code(source) => Some(source),
            Self::ProcedureInstruction(source) => Some(source),
            Self::Instruction(source) => Some(source),
            Self::RecordState(source) => Some(source),
            Self::ProcedureState(source) => Some(source),
            Self::Dialogue(source) => Some(source),
            Self::Data { source, .. } => Some(source),
            _ => None,
        }
    }
}

fn decode_loaded_profile(
    profile: ScriptProfileId,
    resources: ScriptProfileResources,
    cache: &OriginalResourceCache,
    dialect: ScriptDialect,
    retained_state: Option<(ScriptState, ScriptDirectory)>,
) -> Result<LoadedScriptProfile, ScriptProfileError> {
    let directory_bytes = loaded_resource(
        cache,
        resources.resource(ScriptProfileResourceKind::Directory),
    )?;
    let directory =
        decode_script_directory(directory_bytes).map_err(|source| ScriptProfileError::Data {
            kind: ScriptProfileDataKind::Directory,
            source,
        })?;
    let dictionary_bytes = loaded_resource(
        cache,
        resources.resource(ScriptProfileResourceKind::Dictionary),
    )?;
    let dictionary =
        decode_script_dictionary(dictionary_bytes).map_err(|source| ScriptProfileError::Data {
            kind: ScriptProfileDataKind::Dictionary,
            source,
        })?;
    let code = decode_script_code_for_dialect(
        loaded_resource(cache, resources.resource(ScriptProfileResourceKind::Code))?,
        dialect,
    )
    .map_err(ScriptProfileError::Code)?;
    let procedure_gates = code
        .tokens()
        .iter()
        .filter(|token| token.opcode().byte() == PROCEDURE_GATE_OPCODE)
        .map(|token| decode_script_procedure_gate(token, &directory))
        .collect::<Result<Vec<_>, _>>()
        .map_err(ScriptProfileError::ProcedureInstruction)?;
    let procedures = ScriptProcedureStates::from_gates(&procedure_gates)
        .map_err(ScriptProfileError::ProcedureState)?;
    let dialogue = decode_script_bas(
        loaded_resource(
            cache,
            resources.resource(ScriptProfileResourceKind::Dialogue),
        )?,
        &dictionary,
    )
    .map_err(ScriptProfileError::Dialogue)?;
    let state = if let Some((state, previous_directory)) = retained_state {
        if state.dialect() != dialect
            || !previous_directory
                .active_objects()
                .eq(directory.active_objects())
        {
            return Err(ScriptProfileError::IncompatiblePersistentDirectory);
        }
        state
    } else {
        decode_script_state_for_dialect(
            loaded_resource(cache, resources.resource(ScriptProfileResourceKind::State))?,
            &directory,
            dialect,
        )
        .map_err(|source| ScriptProfileError::Data {
            kind: ScriptProfileDataKind::State,
            source,
        })?
    };
    let instructions = code
        .tokens()
        .iter()
        .map(|token| decode_complete_script_instruction(token, &state, &directory, &dictionary))
        .collect::<Result<Box<[_]>, _>>()
        .map_err(ScriptProfileError::Instruction)?;
    let builtins = ScriptProfileBuiltins::bind(&directory);
    let record_state =
        ScriptProfileRecordState::recover(&instructions, &state, &dictionary, builtins)
            .map_err(ScriptProfileError::RecordState)?;

    Ok(LoadedScriptProfile {
        id: profile,
        resources,
        code,
        instructions,
        dialogue,
        state,
        dictionary,
        directory,
        builtins,
        procedures,
        runtime: ScriptRuntime::new(),
        selector_state: ScriptSelectorState::default(),
        sequence_slots: ScriptSequenceSlots::default(),
        record_state,
    })
}

fn loaded_resource(
    cache: &OriginalResourceCache,
    resource: ResourceId,
) -> Result<&[u8], ScriptProfileError> {
    cache
        .resolve(resource)
        .ok_or(ScriptProfileError::MissingLoadedResource { resource })
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU64, Ordering};

    use commander_blood_formats::instruction::{
        ScriptTimerSlot, decode_script_sequence_slot_assignment,
    };
    use serde::Deserialize;

    use super::*;

    const FIRST_PROFILE: ScriptProfileId = ScriptProfileId(0);
    const SECOND_PROFILE: ScriptProfileId = ScriptProfileId(1);
    const FIRST_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [2, 3, 4, 5, 6];
    const SECOND_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [37, 38, 39, 40, 41];
    const FINAL_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [86, 87, 88, 89, 90];
    const SEQUENCE_SLOT_ASSIGNMENT_OPCODE: u8 = 0xCC;
    const PROFILE_SELECT_ORACLE_VECTOR_COUNT: usize = 6;
    static TEMPORARY_ROOT_SEQUENCE: AtomicU64 = AtomicU64::new(u64::MIN);

    #[derive(Deserialize)]
    struct ProfileSelectOracle {
        name: String,
        profile: u32,
        current_profile: u32,
        failure_index: Option<usize>,
        split_data: bool,
        return_value: i16,
    }

    struct TemporaryProfileRoot(PathBuf);

    impl TemporaryProfileRoot {
        fn create() -> Self {
            let sequence = TEMPORARY_ROOT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
            let path = std::env::temp_dir().join(format!(
                "commander-blood-profile-test-{}-{sequence}",
                std::process::id()
            ));
            std::fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TemporaryProfileRoot {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn original_data_root() -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso"),
            Path::new("commander-blood-audio/_tmp_iso"),
            Path::new("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find(|root| root.join("SCRIPT1.COD").is_file())
        .map(Path::to_owned)
    }

    fn original_resource_catalog() -> OriginalResourceCatalog {
        OriginalResourceCatalog::decode_bloodprg(include_bytes!(
            "../../../../../re/bin/BLOODPRG.EXE"
        ))
        .unwrap()
    }

    fn numeric_resources(
        resources: ScriptProfileResources,
    ) -> [u16; SCRIPT_PROFILE_RESOURCE_COUNT] {
        resources.all().map(ResourceId::value)
    }

    fn profile_id(value: u32) -> Option<ScriptProfileId> {
        u8::try_from(value).ok().and_then(ScriptProfileId::new)
    }

    #[test]
    fn executable_profile_matrix_matches_every_authored_row_and_sentinel() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let profiles = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();

        assert_eq!(
            numeric_resources(profiles.profile(FIRST_PROFILE)),
            FIRST_PROFILE_RESOURCES
        );
        assert_eq!(
            numeric_resources(profiles.profile(SECOND_PROFILE)),
            SECOND_PROFILE_RESOURCES
        );
        assert_eq!(
            numeric_resources(
                profiles.profile(
                    ScriptProfileId::new((ORIGINAL_SCRIPT_PROFILE_COUNT - 1) as u8).unwrap()
                )
            ),
            FINAL_PROFILE_RESOURCES
        );
        assert!(ScriptProfileId::new(ORIGINAL_SCRIPT_PROFILE_COUNT as u8).is_none());
    }

    #[test]
    #[ignore = "requires original Big Bug Bang executable under output/big-bug-bang/disc"]
    fn sequel_profile_catalog_resolves_every_native_resource_role() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/disc");
        let executable = std::fs::read(root.join("BLOOD2PG.EXE")).unwrap();
        let names = OriginalResourceCatalog::decode_blood2pg(&executable).unwrap();
        let catalog = OriginalScriptProfileCatalog::decode_blood2pg(&executable).unwrap();
        assert_eq!(names.len(), 155);
        assert_eq!(catalog.profiles.len(), BIG_BUG_BANG_SCRIPT_PROFILE_COUNT);
        assert_eq!(catalog.dialect(), ScriptDialect::BigBugBang);
        for number in 0..BIG_BUG_BANG_SCRIPT_PROFILE_COUNT {
            let id = ScriptProfileId::new_for_dialect(number as u8, catalog.dialect()).unwrap();
            for (kind, extension) in [
                (ScriptProfileResourceKind::Code, "cod"),
                (ScriptProfileResourceKind::Dialogue, "bas"),
                (ScriptProfileResourceKind::State, "var"),
                (ScriptProfileResourceKind::Dictionary, "dic"),
                (ScriptProfileResourceKind::Directory, "deb"),
            ] {
                assert_eq!(
                    names
                        .name(catalog.profile(id).resource(kind))
                        .unwrap()
                        .as_bytes(),
                    format!("script{}.{extension}", number + 1).as_bytes()
                );
            }
        }
        let mut truncated = executable[..BLOOD2PG_SCRIPT_PROFILE_TABLE_FILE_OFFSET].to_vec();
        assert!(matches!(
            OriginalScriptProfileCatalog::decode_blood2pg(&truncated),
            Err(ScriptProfileError::ExecutableTooShort { .. })
        ));
        truncated = executable;
        let sentinel = BLOOD2PG_SCRIPT_PROFILE_TABLE_FILE_OFFSET
            + BIG_BUG_BANG_SCRIPT_PROFILE_COUNT
                * SCRIPT_PROFILE_RESOURCE_COUNT
                * SERIALIZED_RESOURCE_ID_SIZE;
        truncated[sentinel] = 1;
        assert!(matches!(
            OriginalScriptProfileCatalog::decode_blood2pg(&truncated),
            Err(ScriptProfileError::InvalidProfileSentinel)
        ));
    }

    #[test]
    fn sequel_profile_switches_retain_live_var_and_timers_until_returning_to_initial() {
        use commander_blood_formats::archive::BloodResourceName;
        use commander_blood_formats::instruction::ScriptTimerSlot;
        const INITIAL_WORD: u16 = 100;
        const MUTATED_WORD: u16 = 4321;
        const TIMER_VALUE: u16 = 37;
        const PLAYER_RECORD_SIZE: usize = 34;
        const TEST_WORD_OFFSET: u16 = PLAYER_RECORD_SIZE as u16;
        const DIRECTORY_ENTRY_SIZE: usize = 20;
        const DIRECTORY_KIND_OFFSET: usize = 18;
        let root = TemporaryProfileRoot::create();
        let mut names = Vec::new();
        for profile in 1..=2 {
            for extension in ["COD", "BAS", "VAR", "DIC", "DEB"] {
                let name = format!("SCRIPT{profile}.{extension}");
                // Minimal well-formed input programs exercise the real loader.
                // These fixtures do not stand in for missing production BAS files.
                let bytes = match extension {
                    "COD" | "BAS" => vec![u8::MAX],
                    "VAR" => {
                        let mut bytes = vec![0; PLAYER_RECORD_SIZE];
                        bytes[0] = 1; // Player record kind.
                        bytes.extend_from_slice(&(INITIAL_WORD + profile - 1).to_le_bytes());
                        bytes
                    }
                    "DIC" => vec![0],
                    "DEB" => {
                        let mut bytes = vec![0; DIRECTORY_ENTRY_SIZE * 2];
                        bytes[..5].copy_from_slice(b"blood");
                        bytes[DIRECTORY_KIND_OFFSET] = 1; // Object directory entry.
                        bytes
                    }
                    _ => unreachable!("fixed fixture companion list"),
                };
                std::fs::write(root.0.join(&name), bytes).unwrap();
                names.push(BloodResourceName::new(name.as_bytes()).unwrap());
            }
        }
        let catalog = OriginalScriptProfileCatalog {
            dialect: ScriptDialect::BigBugBang,
            profiles: (0..2)
                .map(|profile| ScriptProfileResources {
                    resources: std::array::from_fn(|index| {
                        ResourceId::new((profile * SCRIPT_PROFILE_RESOURCE_COUNT + index) as u16)
                    }),
                })
                .collect(),
        };
        let resources = OriginalResourceCatalog::new(names);
        let store = OriginalResourceStore::new(root.0.clone(), None, [], true);
        let mut manager = ScriptProfileManager::new(catalog);
        let mut cache = OriginalResourceCache::new();
        let timer = ScriptTimerSlot::decode(0).unwrap();
        assert!(matches!(
            manager.select(SECOND_PROFILE, &mut cache, &store, &resources),
            Err(ScriptProfileError::MissingPersistentState)
        ));
        manager
            .select(FIRST_PROFILE, &mut cache, &store, &resources)
            .unwrap();
        let loaded = manager.current_mut().unwrap();
        let word = loaded
            .state
            .resolve_word_source_offset(TEST_WORD_OFFSET)
            .unwrap();
        assert!(loaded.state.set_word(word, MUTATED_WORD));
        loaded.runtime.assign_timer(timer, TIMER_VALUE);
        let live_resource = loaded.resources.resource(ScriptProfileResourceKind::State);
        let outside_catalog =
            ScriptProfileId::new_for_dialect(2, ScriptDialect::BigBugBang).unwrap();
        assert!(matches!(
            manager.select(outside_catalog, &mut cache, &store, &resources),
            Err(ScriptProfileError::ProfileOutsideCatalog { .. })
        ));
        assert_eq!(manager.current().unwrap().id(), FIRST_PROFILE);
        assert_eq!(
            manager.current().unwrap().state.word(word),
            Some(MUTATED_WORD)
        );
        assert!(cache.is_loaded(live_resource));
        let skipped_resource = manager
            .catalog
            .profile(SECOND_PROFILE)
            .resource(ScriptProfileResourceKind::State);
        let outcome = manager
            .select(SECOND_PROFILE, &mut cache, &store, &resources)
            .unwrap();
        assert_eq!(outcome.released_resources, 4);
        assert!(cache.is_loaded(live_resource));
        assert!(!cache.is_loaded(skipped_resource));
        for _ in 0..2 {
            let loaded = manager.current().unwrap();
            assert_eq!(loaded.state.dialect(), ScriptDialect::BigBugBang);
            assert_eq!(loaded.state.word(word), Some(MUTATED_WORD));
            assert_eq!(loaded.runtime.timer(timer), TIMER_VALUE);
            assert_eq!(
                loaded.resources.resource(ScriptProfileResourceKind::State),
                live_resource
            );
            manager
                .select(SECOND_PROFILE, &mut cache, &store, &resources)
                .unwrap();
        }
        manager
            .select(FIRST_PROFILE, &mut cache, &store, &resources)
            .unwrap();
        assert_eq!(
            manager.current().unwrap().state.word(word),
            Some(INITIAL_WORD)
        );
        assert_eq!(manager.current().unwrap().runtime.timer(timer), u16::MAX);
        manager
            .current_mut()
            .unwrap()
            .state
            .set_word(word, MUTATED_WORD);
        manager
            .current_mut()
            .unwrap()
            .runtime
            .assign_timer(timer, TIMER_VALUE);
        manager
            .select(FIRST_PROFILE, &mut cache, &store, &resources)
            .unwrap();
        assert_eq!(
            manager.current().unwrap().state.word(word),
            Some(MUTATED_WORD)
        );
        assert_eq!(manager.current().unwrap().runtime.timer(timer), u16::MAX);
    }

    #[test]
    fn every_shipped_profile_loads_round_trips_and_rebinds_typed_builtins() {
        let Some(root) = original_data_root() else {
            return;
        };
        let store = OriginalResourceStore::new(root.clone(), None, [], true);
        let resources = original_resource_catalog();
        let profile_catalog = OriginalScriptProfileCatalog::decode_bloodprg(include_bytes!(
            "../../../../../re/bin/BLOODPRG.EXE"
        ))
        .unwrap();
        let mut manager = ScriptProfileManager::new(profile_catalog);
        let mut cache = OriginalResourceCache::new();
        let mut previous_resources = None;

        for profile in ScriptProfileId::all() {
            let outcome = manager
                .select(profile, &mut cache, &store, &resources)
                .unwrap();
            assert!(outcome.profile_changed);
            assert_eq!(
                outcome.resource_statuses,
                [ResourceLoadStatus::LoadedNow; SCRIPT_PROFILE_RESOURCE_COUNT]
            );
            assert_eq!(
                outcome.released_resources,
                previous_resources
                    .map(|_resources: ScriptProfileResources| SCRIPT_PROFILE_RESOURCE_COUNT)
                    .unwrap_or(usize::MIN)
            );

            let loaded = manager.current().unwrap();
            assert_eq!(loaded.id(), profile);
            let file_number = usize::from(profile.value()) + 1;
            assert_eq!(
                loaded.code().encode(),
                std::fs::read(root.join(format!("SCRIPT{file_number}.COD"))).unwrap()
            );
            assert_eq!(loaded.instructions().len(), loaded.code().tokens().len());
            for (token, instruction) in loaded.code().tokens().iter().zip(loaded.instructions()) {
                assert_eq!(
                    loaded.instruction_at(token.source_offset()),
                    Some(instruction)
                );
            }
            assert_eq!(
                loaded.dialogue().encode(),
                std::fs::read(root.join(format!("SCRIPT{file_number}.BAS"))).unwrap()
            );
            assert_eq!(
                loaded.state().encode(),
                std::fs::read(root.join(format!("SCRIPT{file_number}.VAR"))).unwrap()
            );
            assert_eq!(
                loaded.synchronized_state().unwrap().encode(),
                loaded.state().encode(),
                "profile {} record stores must preserve every authored VAR byte",
                profile.value() + 1
            );
            assert_eq!(
                loaded.dictionary().encode(),
                std::fs::read(root.join(format!("SCRIPT{file_number}.DIC"))).unwrap()
            );
            assert_eq!(
                loaded.directory().encode(),
                std::fs::read(root.join(format!("SCRIPT{file_number}.DEB"))).unwrap()
            );
            assert_eq!(
                loaded.procedures().len(),
                loaded.directory().procedures().count()
            );
            let builtins = loaded.builtins();
            assert!(builtins.player.is_some());
            assert!(builtins.world.is_some());
            assert!(builtins.horn.is_some());
            assert!(builtins.menu.is_some());
            assert!(builtins.archetype.is_some());
            assert!(builtins.ark.is_some());
            assert_eq!(builtins.scruter_jo.is_some(), profile != FIRST_PROFILE);
            assert_eq!(
                builtins.video_state_offset.is_some(),
                profile != FIRST_PROFILE
            );

            if let Some(old) = previous_resources {
                for resource in old.all() {
                    assert!(!cache.is_loaded(resource));
                }
            }
            for resource in loaded.resources().all() {
                assert!(cache.is_loaded(resource));
            }
            previous_resources = Some(loaded.resources());
        }
    }

    #[test]
    fn profile_selection_accounts_for_every_native_lifecycle_vector() {
        let vectors: Vec<ProfileSelectOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_53a0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PROFILE_SELECT_ORACLE_VECTOR_COUNT);
        let Some(original_root) = original_data_root() else {
            return;
        };
        let original_store = OriginalResourceStore::new(original_root, None, [], true);
        let resources = original_resource_catalog();
        let profile_catalog = OriginalScriptProfileCatalog::decode_bloodprg(include_bytes!(
            "../../../../../re/bin/BLOODPRG.EXE"
        ))
        .unwrap();

        for vector in vectors {
            let Some(target) = profile_id(vector.profile) else {
                assert_eq!(vector.name, "profile_multiply_wraps_to_sixteen_bits");
                assert!(u8::try_from(vector.profile).is_err());
                assert_eq!(vector.return_value, 0);
                continue;
            };
            let current = profile_id(vector.current_profile).unwrap();
            let mut manager = ScriptProfileManager::new(profile_catalog.clone());
            let mut cache = OriginalResourceCache::new();
            manager
                .select(current, &mut cache, &original_store, &resources)
                .unwrap();

            if let Some(failure_index) = vector.failure_index {
                let temporary = TemporaryProfileRoot::create();
                let incomplete_store =
                    OriginalResourceStore::new(temporary.0.clone(), None, [], true);
                for resource in profile_catalog
                    .profile(target)
                    .all()
                    .into_iter()
                    .take(failure_index)
                {
                    let name = resources.name(resource).unwrap();
                    let bytes = original_store.load(name).unwrap();
                    incomplete_store.write_loose(name, &bytes).unwrap();
                }
                assert!(
                    manager
                        .select(target, &mut cache, &incomplete_store, &resources)
                        .is_err(),
                    "{}",
                    vector.name
                );
                assert_eq!(vector.return_value, -1, "{}", vector.name);
                assert_eq!(
                    manager.current().map(LoadedScriptProfile::id),
                    (current == target).then_some(current),
                    "{}",
                    vector.name
                );
                continue;
            }

            let outcome = manager
                .select(target, &mut cache, &original_store, &resources)
                .unwrap();
            assert_eq!(vector.return_value, 0, "{}", vector.name);
            assert_eq!(
                outcome.profile_changed,
                current != target,
                "{}",
                vector.name
            );
            assert_eq!(manager.current().unwrap().id(), target, "{}", vector.name);
            if vector.split_data {
                assert_eq!(vector.name, "split_ds_gs_proves_profile_and_state_owners");
                assert!(manager.current().unwrap().builtins().world.is_some());
            }
        }
    }

    #[test]
    fn selecting_the_same_profile_resets_control_state_but_retains_global_save_fields() {
        let Some(root) = original_data_root() else {
            return;
        };
        let store = OriginalResourceStore::new(root, None, [], true);
        let resources = original_resource_catalog();
        let profile_catalog = OriginalScriptProfileCatalog::decode_bloodprg(include_bytes!(
            "../../../../../re/bin/BLOODPRG.EXE"
        ))
        .unwrap();
        let mut manager = ScriptProfileManager::new(profile_catalog);
        let mut cache = OriginalResourceCache::new();

        manager
            .select(SECOND_PROFILE, &mut cache, &store, &resources)
            .unwrap();
        manager.current_mut().unwrap().runtime_mut().request_yield();
        let assignment = manager
            .current()
            .unwrap()
            .code()
            .tokens()
            .iter()
            .find(|token| token.opcode().byte() == SEQUENCE_SLOT_ASSIGNMENT_OPCODE)
            .map(decode_script_sequence_slot_assignment)
            .unwrap()
            .unwrap();
        let assigned_slot = assignment.slot();
        let assigned_name = assignment.name().as_bytes().to_vec();
        manager
            .current_mut()
            .unwrap()
            .sequence_slots_mut()
            .assign(assignment);
        let timer_slot = ScriptTimerSlot::decode(u8::MIN).unwrap();
        let mut timer_block = manager
            .current()
            .unwrap()
            .runtime()
            .encode_timer_save_block();
        timer_block[..2].copy_from_slice(&u16::MIN.to_le_bytes());
        let reserved_start = ScriptTimerSlot::COUNT * 2;
        timer_block[reserved_start] = 0x42;
        manager
            .current_mut()
            .unwrap()
            .runtime_mut()
            .restore_timer_save_block(&timer_block);
        let selected_concept = manager
            .current()
            .unwrap()
            .dictionary()
            .words()
            .next()
            .unwrap()
            .0;
        manager
            .current_mut()
            .unwrap()
            .selector_state_mut()
            .history_mut()
            .push(selected_concept);
        manager
            .current_mut()
            .unwrap()
            .selector_state_mut()
            .replace_presentation_words([selected_concept]);
        assert!(
            manager
                .current()
                .unwrap()
                .sequence_slots()
                .name(assigned_slot)
                .is_some()
        );
        let outcome = manager
            .select(SECOND_PROFILE, &mut cache, &store, &resources)
            .unwrap();

        assert!(!outcome.profile_changed);
        assert_eq!(outcome.released_resources, usize::MIN);
        assert_eq!(
            outcome.resource_statuses,
            [ResourceLoadStatus::AlreadyLoaded; SCRIPT_PROFILE_RESOURCE_COUNT]
        );
        assert!(!manager.current().unwrap().runtime().yield_requested());
        assert_eq!(
            manager.current().unwrap().selector_state(),
            &ScriptSelectorState::default()
        );
        assert_eq!(
            manager
                .current()
                .unwrap()
                .sequence_slots()
                .name(assigned_slot)
                .unwrap()
                .as_bytes(),
            assigned_name
        );
        assert_eq!(
            manager.current().unwrap().runtime().timer(timer_slot),
            u16::MAX
        );
        assert_eq!(
            manager
                .current()
                .unwrap()
                .runtime()
                .encode_timer_save_block()[reserved_start],
            0x42
        );
    }
}
