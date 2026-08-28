//! Loading and ownership of complete authored BloodScript profiles.

mod record_state;

use std::error::Error;
use std::fmt;
use std::mem::size_of;

use commander_blood_formats::bas::{ScriptBas, ScriptBasError, decode_script_bas};
use commander_blood_formats::code::{
    ScriptCode, ScriptCodeError, ScriptCodeOffset, ScriptToken, decode_script_code,
};
use commander_blood_formats::instruction::{
    DecodedScriptInstruction, ScriptInstructionError, decode_complete_script_instruction,
    decode_script_procedure_gate,
};
use commander_blood_formats::script::{
    ScriptDataError, ScriptDictionary, ScriptDirectory, ScriptObjectId, ScriptState,
    ScriptSymbolKind, decode_script_dictionary, decode_script_directory, decode_script_state,
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
        if value < ORIGINAL_SCRIPT_PROFILE_COUNT as u8 {
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
    profiles: [ScriptProfileResources; ORIGINAL_SCRIPT_PROFILE_COUNT],
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
            profiles: profiles
                .try_into()
                .expect("decoded exactly five playable script profiles"),
        })
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
    /// Fresh-or-resident result for each file in native load order.
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
                        .filter(|resource| cache.release(*resource))
                        .count()
                })
                .unwrap_or(usize::MIN)
        } else {
            usize::MIN
        };

        let profile_resources = self.catalog.profile(profile);
        let mut resource_statuses =
            [ResourceLoadStatus::AlreadyLoaded; SCRIPT_PROFILE_RESOURCE_COUNT];
        for (index, resource) in profile_resources.all().into_iter().enumerate() {
            resource_statuses[index] = cache
                .load_by_id(store, resources, resource)
                .map_err(ScriptProfileError::Resource)?;
        }

        let mut loaded = decode_loaded_profile(profile, profile_resources, cache)?;
        if let Some(previous_runtime) = &previous_runtime {
            loaded
                .runtime
                .preserve_timer_save_reserved_bytes(previous_runtime);
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
        write!(
            formatter,
            "invalid Commander Blood script profile: {self:?}"
        )
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
    let code = decode_script_code(loaded_resource(
        cache,
        resources.resource(ScriptProfileResourceKind::Code),
    )?)
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
    let state = decode_script_state(
        loaded_resource(cache, resources.resource(ScriptProfileResourceKind::State))?,
        &directory,
    )
    .map_err(|source| ScriptProfileError::Data {
        kind: ScriptProfileDataKind::State,
        source,
    })?;
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

    use commander_blood_formats::instruction::{
        ScriptTimerSlot, decode_script_sequence_slot_assignment,
    };

    use super::*;

    const FIRST_PROFILE: ScriptProfileId = ScriptProfileId(0);
    const SECOND_PROFILE: ScriptProfileId = ScriptProfileId(1);
    const FIRST_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [2, 3, 4, 5, 6];
    const SECOND_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [37, 38, 39, 40, 41];
    const FINAL_PROFILE_RESOURCES: [u16; SCRIPT_PROFILE_RESOURCE_COUNT] = [86, 87, 88, 89, 90];
    const SEQUENCE_SLOT_ASSIGNMENT_OPCODE: u8 = 0xCC;

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
