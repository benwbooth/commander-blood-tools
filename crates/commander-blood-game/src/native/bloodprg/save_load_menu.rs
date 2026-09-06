//! Save/load menu coordination over typed profile state and host files.

use std::error::Error;
use std::fmt;

use anyhow::{Context, Result as AnyResult};

use super::{
    FramebufferTransitionState, LoadedScriptProfile, ORIGINAL_QUICK_SAVE_SLOT_INDEX,
    ORIGINAL_SAVE_SLOT_COUNT, ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT, OriginalResourceCache,
    OriginalResourceCatalog, OriginalSaveGame, OriginalSaveSlotDirectory, SAVE_SLOT_NAME_LENGTH,
    SaveSlotName, ScriptProfileManager, original_save_state_block_byte_count,
};
use crate::assets::OriginalResourceStore;

const SAVE_LOAD_UI_ACTIVE_MASK: u8 = 0x04;
const SAVE_LOAD_TRANSITION_STEP_COUNT: u8 = 6;
const SAVE_SLOT_EMPTY_BYTE: u8 = b' ';
const QUICK_SAVE_NAME_PREFIX: [u8; 8] = *b"LAST\0PAU";

/// Pending save operations, with quick-save retaining native priority.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SaveLoadRequests {
    /// Open or continue the ordinary save menu.
    pub save: bool,
    /// Open or continue the ordinary load menu.
    pub load: bool,
    /// Save immediately into the reserved tenth slot.
    pub quick_save: bool,
}

/// Semantic phase of the save/load list transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveLoadMenuPhase {
    /// The list is interactive or inactive.
    #[default]
    Ready,
    /// The list must be measured and its transition initialized.
    LayoutPending,
    /// The six-step rectangle transition is active.
    Transitioning,
}

/// State retained between calls to the save/load coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SaveLoadMenuState {
    /// Current save, load, and quick-save requests.
    pub requests: SaveLoadRequests,
    /// Current list-transition phase.
    pub phase: SaveLoadMenuPhase,
    /// Shared UI flags containing the save/load active bit.
    pub ui_flags: u8,
    /// Current six-step rectangle transition progress.
    pub transition: FramebufferTransitionState,
    /// Whether the shared list layout should preserve prior widths.
    pub preserve_layout_widths: bool,
    /// Slot whose name is currently highlighted, when initialized.
    pub selected_slot: Option<usize>,
    /// Slot receiving an Enter-key name commit or quick save.
    pub active_slot: Option<usize>,
    /// Fixed-width editable name displayed over the active list row.
    pub edit_name: SaveSlotName,
    /// Name length through the first NUL or space during the latest save step.
    pub name_length: usize,
    /// Whether the navigation scene must redraw after a successful load.
    pub redraw_pending: bool,
    /// Whether the restored palette must be uploaded.
    pub palette_dirty: bool,
}

impl SaveLoadMenuState {
    /// Construct an inactive coordinator while preserving shared UI flags.
    pub const fn new(ui_flags: u8) -> Self {
        Self {
            requests: SaveLoadRequests {
                save: false,
                load: false,
                quick_save: false,
            },
            phase: SaveLoadMenuPhase::Ready,
            ui_flags,
            transition: FramebufferTransitionState {
                total_steps: SAVE_LOAD_TRANSITION_STEP_COUNT,
                current_step: u8::MIN,
            },
            preserve_layout_widths: false,
            selected_slot: None,
            active_slot: None,
            edit_name: SaveSlotName::from_bytes([u8::MIN; SAVE_SLOT_NAME_LENGTH]),
            name_length: usize::MIN,
            redraw_pending: false,
            palette_dirty: false,
        }
    }

    /// Request the ordinary save list and its opening transition.
    pub fn request_save(&mut self) {
        self.requests.save = true;
        self.phase = SaveLoadMenuPhase::LayoutPending;
    }

    /// Request the ordinary load list and its opening transition.
    pub fn request_load(&mut self) {
        self.requests.load = true;
        self.phase = SaveLoadMenuPhase::LayoutPending;
    }

    /// Request an immediate save into the reserved tenth slot.
    pub fn request_quick_save(&mut self) {
        self.requests.quick_save = true;
    }

    /// Close any active save/load interaction through the shared cleanup path.
    pub fn cancel(&mut self) {
        close_save_load_menu(self);
        self.requests.quick_save = false;
    }

    /// Return whether any save/load operation currently owns input.
    pub const fn is_active(&self) -> bool {
        self.requests.save || self.requests.load || self.requests.quick_save
    }
}

impl Default for SaveLoadMenuState {
    fn default() -> Self {
        Self::new(u8::MIN)
    }
}

/// Meaning of one list-widget result after removing pointer sentinels.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveLoadSelection {
    /// No row has committed a selection this frame.
    None,
    /// One of the ten save records was selected.
    Slot(usize),
    /// The authored trailing cancel row was selected.
    Close,
}

/// Distinct list passes made by the native coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveLoadListPass {
    /// Measure the list while substituting the editable name field.
    MeasureEditingName,
    /// Poll and draw the interactive list.
    Poll,
}

/// Observable result of one coordinator call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveLoadMenuOutcome {
    /// No save or load request was active.
    Inactive,
    /// The opening transition advanced and needs another frame.
    Transitioning,
    /// The menu remains open for another input frame.
    Waiting,
    /// A save was written to the named slot.
    Saved {
        /// Zero-based slot written by the operation.
        slot: usize,
    },
    /// A save was restored from the named slot.
    Loaded {
        /// Zero-based slot read by the operation.
        slot: usize,
    },
    /// Cancel, file-open failure, or file-create failure closed the menu.
    Closed,
}

/// UI and filesystem operations supplied by the modern host.
pub trait SaveLoadHost {
    /// Measure or poll the authored save-slot list.
    fn layout_save_slots(
        &mut self,
        pass: SaveLoadListPass,
        directory: &OriginalSaveSlotDirectory,
        active_slot: Option<usize>,
        edit_name: SaveSlotName,
    ) -> AnyResult<SaveLoadSelection>;

    /// Render and advance one rectangle-transition step.
    fn advance_save_transition(
        &mut self,
        transition: &mut FramebufferTransitionState,
    ) -> AnyResult<()>;

    /// Run the already translated save-name editor for one input frame.
    fn edit_save_slot_name(
        &mut self,
        selected_slot: Option<usize>,
        edit_name: &mut SaveSlotName,
        name_length: usize,
    ) -> AnyResult<bool>;

    /// Create or truncate a save file, returning false for native open failure.
    fn create_save_file(&mut self, filename: &[u8]) -> AnyResult<bool>;

    /// Write the complete codec-produced image to the file just created.
    fn write_created_save_file(&mut self, data: &[u8]) -> AnyResult<()>;

    /// Read one complete save file, returning `None` for native open failure.
    fn read_save_file(&mut self, filename: &[u8]) -> AnyResult<Option<Box<[u8]>>>;

    /// Replace `BLOOD.SAV` with the complete exact slot directory.
    fn write_save_slot_directory(
        &mut self,
        data: &[u8; ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT],
    ) -> AnyResult<()>;
}

/// Capture and restore behavior supplied by the typed BloodScript owner.
pub trait SaveProfileBackend {
    /// Capture the currently loaded profile in the original save format.
    fn capture_save_game(&mut self) -> AnyResult<OriginalSaveGame>;

    /// Select, initialize, restore, and rebuild the profile encoded by a save.
    fn restore_save_game(&mut self, data: &[u8]) -> AnyResult<()>;
}

/// Runtime work that surrounds persistent-block restoration.
pub trait SavedProfileLifecycle {
    /// Run the newly selected profile once before persistent blocks are applied.
    fn initialize_loaded_profile(&mut self, profile: &mut LoadedScriptProfile) -> AnyResult<()>;

    /// Rebuild derived record, HUD, palette, and camera state after restoration.
    fn rebuild_loaded_state(&mut self, profile: &mut LoadedScriptProfile) -> AnyResult<()>;
}

/// Concrete save backend owning the original profile-selection lifecycle.
pub struct OriginalSaveProfileBackend<'a, Lifecycle> {
    manager: &'a mut ScriptProfileManager,
    cache: &'a mut OriginalResourceCache,
    store: &'a OriginalResourceStore,
    resources: &'a OriginalResourceCatalog,
    lifecycle: &'a mut Lifecycle,
}

impl<'a, Lifecycle> OriginalSaveProfileBackend<'a, Lifecycle> {
    /// Bind profile ownership, resource loading, and derived-state lifecycle.
    pub fn new(
        manager: &'a mut ScriptProfileManager,
        cache: &'a mut OriginalResourceCache,
        store: &'a OriginalResourceStore,
        resources: &'a OriginalResourceCatalog,
        lifecycle: &'a mut Lifecycle,
    ) -> Self {
        Self {
            manager,
            cache,
            store,
            resources,
            lifecycle,
        }
    }
}

impl<Lifecycle: SavedProfileLifecycle> SaveProfileBackend
    for OriginalSaveProfileBackend<'_, Lifecycle>
{
    fn capture_save_game(&mut self) -> AnyResult<OriginalSaveGame> {
        let profile = self
            .manager
            .current()
            .context("cannot save without a loaded BloodScript profile")?;
        OriginalSaveGame::capture(profile).map_err(Into::into)
    }

    fn restore_save_game(&mut self, data: &[u8]) -> AnyResult<()> {
        let dialect = self.manager.dialect();
        let profile_id = OriginalSaveGame::decode_profile_for_dialect(data, dialect)?;
        self.manager
            .select(profile_id, self.cache, self.store, self.resources)?;
        {
            let profile = self
                .manager
                .current_mut()
                .context("selected BloodScript profile was not retained")?;
            self.lifecycle.initialize_loaded_profile(profile)?;
        }
        let state_block_byte_count = original_save_state_block_byte_count(
            self.manager
                .current()
                .context("initialized BloodScript profile disappeared")?,
        )?;
        let save = OriginalSaveGame::decode_for_dialect(data, state_block_byte_count, dialect)?;
        let profile = self
            .manager
            .current_mut()
            .context("initialized BloodScript profile disappeared before restore")?;
        save.restore_into(profile)?;
        self.lifecycle.rebuild_loaded_state(profile)
    }
}

/// Invalid typed state or failed host operation during save/load coordination.
#[derive(Debug)]
pub enum SaveLoadMenuError {
    /// A list result names no authored save slot.
    InvalidSlotSelection {
        /// Out-of-range zero-based selection.
        selected: usize,
    },
    /// A save commit occurred before any active slot was established.
    MissingActiveSlot,
    /// One slot filename lacks a terminator in its fixed field.
    UnterminatedFilename {
        /// Slot owning the malformed filename.
        slot: usize,
    },
    /// A UI or filesystem operation failed.
    Host {
        /// Operation being performed.
        operation: &'static str,
        /// Underlying host failure.
        source: anyhow::Error,
    },
    /// Typed profile capture or restoration failed.
    Profile {
        /// Operation being performed.
        operation: &'static str,
        /// Underlying profile failure.
        source: anyhow::Error,
    },
}

impl fmt::Display for SaveLoadMenuError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "save/load menu failed: {self:?}")
    }
}

impl Error for SaveLoadMenuError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Host { source, .. } | Self::Profile { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Translate `save_load_menu_step` at BLOODPRG file offset `0x001B4B`.
///
/// The routine retains quick-save priority, pre-call transition completion,
/// edit-before-new-selection ordering, the reserved tenth save row, and the
/// profile-select/initialize/restore/rebuild load sequence. Fixed native
/// pointers and DOS handles are replaced by typed slots, exact codecs, and host
/// filesystem transactions.
pub fn update_save_load_menu<Host: SaveLoadHost, Profiles: SaveProfileBackend>(
    state: &mut SaveLoadMenuState,
    directory: &mut OriginalSaveSlotDirectory,
    host: &mut Host,
    profiles: &mut Profiles,
) -> Result<SaveLoadMenuOutcome, SaveLoadMenuError> {
    if state.requests.quick_save {
        let quick_slot = &mut directory.slots_mut()[ORIGINAL_QUICK_SAVE_SLOT_INDEX];
        let mut display_name = quick_slot.display_name().bytes();
        display_name[..QUICK_SAVE_NAME_PREFIX.len()].copy_from_slice(&QUICK_SAVE_NAME_PREFIX);
        quick_slot.set_display_name(SaveSlotName::from_bytes(display_name));
        state.active_slot = Some(ORIGINAL_QUICK_SAVE_SLOT_INDEX);
        state.requests.quick_save = false;
        return save_active_slot(state, directory, host, profiles);
    }

    if !state.requests.save && !state.requests.load {
        return Ok(SaveLoadMenuOutcome::Inactive);
    }

    state.ui_flags |= SAVE_LOAD_UI_ACTIVE_MASK;
    if state.phase == SaveLoadMenuPhase::LayoutPending {
        host.layout_save_slots(
            SaveLoadListPass::MeasureEditingName,
            directory,
            state.active_slot,
            state.edit_name,
        )
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "measuring save-slot list",
            source,
        })?;
        state.preserve_layout_widths = false;
        state.transition.current_step = u8::MIN;
        state.transition.total_steps = SAVE_LOAD_TRANSITION_STEP_COUNT;
        state.active_slot = Some(usize::MIN);
        state.selected_slot = Some(usize::MIN);
        state.edit_name = directory.slots()[usize::MIN].display_name();
        state.phase = SaveLoadMenuPhase::Transitioning;
    }

    if state.phase == SaveLoadMenuPhase::Transitioning {
        let transition_complete = state.transition.current_step == state.transition.total_steps;
        host.advance_save_transition(&mut state.transition)
            .map_err(|source| SaveLoadMenuError::Host {
                operation: "advancing save-slot transition",
                source,
            })?;
        if !transition_complete {
            return Ok(SaveLoadMenuOutcome::Transitioning);
        }
        state.phase = SaveLoadMenuPhase::Ready;
    }

    let selection = host
        .layout_save_slots(
            SaveLoadListPass::Poll,
            directory,
            state.active_slot,
            state.edit_name,
        )
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "polling save-slot list",
            source,
        })?;

    if state.requests.save {
        state.name_length = save_name_length(state.edit_name);
        let committed = host
            .edit_save_slot_name(state.selected_slot, &mut state.edit_name, state.name_length)
            .map_err(|source| SaveLoadMenuError::Host {
                operation: "editing save-slot name",
                source,
            })?;
        if committed {
            let active_slot = state
                .active_slot
                .ok_or(SaveLoadMenuError::MissingActiveSlot)?;
            validate_slot(active_slot)?;
            directory.slots_mut()[active_slot].set_display_name(state.edit_name);
            return save_active_slot(state, directory, host, profiles);
        }

        match selection {
            SaveLoadSelection::None | SaveLoadSelection::Slot(ORIGINAL_QUICK_SAVE_SLOT_INDEX) => {
                return Ok(SaveLoadMenuOutcome::Waiting);
            }
            SaveLoadSelection::Close => {
                close_save_load_menu(state);
                return Ok(SaveLoadMenuOutcome::Closed);
            }
            SaveLoadSelection::Slot(slot) => {
                validate_slot(slot)?;
                state.selected_slot = Some(slot);
                state.active_slot = Some(slot);
                state.edit_name = directory.slots()[slot].display_name();
                return Ok(SaveLoadMenuOutcome::Waiting);
            }
        }
    }

    let slot = match selection {
        SaveLoadSelection::None => return Ok(SaveLoadMenuOutcome::Waiting),
        SaveLoadSelection::Close => {
            close_save_load_menu(state);
            return Ok(SaveLoadMenuOutcome::Closed);
        }
        SaveLoadSelection::Slot(slot) => {
            validate_slot(slot)?;
            slot
        }
    };
    let filename = slot_filename(directory, slot)?.to_vec();
    let Some(data) = host
        .read_save_file(&filename)
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "reading save file",
            source,
        })?
    else {
        close_save_load_menu(state);
        return Ok(SaveLoadMenuOutcome::Closed);
    };
    profiles
        .restore_save_game(&data)
        .map_err(|source| SaveLoadMenuError::Profile {
            operation: "restoring save profile",
            source,
        })?;
    state.redraw_pending = true;
    state.palette_dirty = true;
    close_save_load_menu(state);
    Ok(SaveLoadMenuOutcome::Loaded { slot })
}

fn save_active_slot<Host: SaveLoadHost, Profiles: SaveProfileBackend>(
    state: &mut SaveLoadMenuState,
    directory: &OriginalSaveSlotDirectory,
    host: &mut Host,
    profiles: &mut Profiles,
) -> Result<SaveLoadMenuOutcome, SaveLoadMenuError> {
    let slot = state
        .active_slot
        .ok_or(SaveLoadMenuError::MissingActiveSlot)?;
    validate_slot(slot)?;
    let filename = slot_filename(directory, slot)?.to_vec();
    let created = host
        .create_save_file(&filename)
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "creating save file",
            source,
        })?;
    if !created {
        close_save_load_menu(state);
        return Ok(SaveLoadMenuOutcome::Closed);
    }

    let save = profiles
        .capture_save_game()
        .map_err(|source| SaveLoadMenuError::Profile {
            operation: "capturing save profile",
            source,
        })?;
    host.write_created_save_file(&save.encode())
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "writing save file",
            source,
        })?;
    host.write_save_slot_directory(&directory.encode())
        .map_err(|source| SaveLoadMenuError::Host {
            operation: "writing save-slot directory",
            source,
        })?;
    close_save_load_menu(state);
    Ok(SaveLoadMenuOutcome::Saved { slot })
}

fn validate_slot(slot: usize) -> Result<(), SaveLoadMenuError> {
    if slot < ORIGINAL_SAVE_SLOT_COUNT {
        Ok(())
    } else {
        Err(SaveLoadMenuError::InvalidSlotSelection { selected: slot })
    }
}

fn slot_filename(
    directory: &OriginalSaveSlotDirectory,
    slot: usize,
) -> Result<&[u8], SaveLoadMenuError> {
    directory.slots()[slot]
        .filename_bytes()
        .ok_or(SaveLoadMenuError::UnterminatedFilename { slot })
}

fn save_name_length(name: SaveSlotName) -> usize {
    name.bytes()
        .iter()
        .position(|byte| matches!(*byte, u8::MIN | SAVE_SLOT_EMPTY_BYTE))
        .unwrap_or(SAVE_SLOT_NAME_LENGTH)
}

fn close_save_load_menu(state: &mut SaveLoadMenuState) {
    state.ui_flags &= !SAVE_LOAD_UI_ACTIVE_MASK;
    state.requests.save = false;
    state.requests.load = false;
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::path::{Path, PathBuf};
    use std::rc::Rc;

    use serde::Deserialize;
    use serde_json::Value;

    use super::*;
    use crate::native::bloodprg::OriginalScriptProfileCatalog;

    const ORACLE_VECTOR_COUNT: usize = 13;
    const INITIAL_UI_FLAGS: u8 = 0xA5;
    const INITIAL_ACTIVE_SLOT: usize = 2;
    const INITIAL_EDIT_NAME: [u8; SAVE_SLOT_NAME_LENGTH] = *b"edited name     ";

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum SemanticCall {
        Layout(SaveLoadListPass),
        Transition,
        Edit,
        Create(String),
        Capture,
        WriteSave,
        WriteDirectory,
        Read(String),
        Restore,
    }

    #[derive(Deserialize)]
    struct OracleInitial {
        save: u8,
        load: u8,
        quick: u8,
        phase: u8,
        current: u8,
        total: u8,
    }

    #[derive(Deserialize)]
    struct OracleFinal {
        save: u8,
        load: u8,
        quick: u8,
        phase: u8,
        ui_flags: u8,
        redraw: u8,
        palette_dirty: u8,
    }

    #[derive(Deserialize)]
    struct SaveLoadOracle {
        name: String,
        initial: OracleInitial,
        calls: Vec<Value>,
        #[serde(rename = "final")]
        final_state: OracleFinal,
    }

    struct MockHost {
        calls: Rc<RefCell<Vec<SemanticCall>>>,
        selections: Vec<SaveLoadSelection>,
        commit: bool,
        create_success: bool,
        open_success: bool,
    }

    impl SaveLoadHost for MockHost {
        fn layout_save_slots(
            &mut self,
            pass: SaveLoadListPass,
            _directory: &OriginalSaveSlotDirectory,
            _active_slot: Option<usize>,
            _edit_name: SaveSlotName,
        ) -> AnyResult<SaveLoadSelection> {
            self.calls.borrow_mut().push(SemanticCall::Layout(pass));
            Ok(self.selections.remove(usize::MIN))
        }

        fn advance_save_transition(
            &mut self,
            transition: &mut FramebufferTransitionState,
        ) -> AnyResult<()> {
            self.calls.borrow_mut().push(SemanticCall::Transition);
            if transition.current_step != transition.total_steps {
                transition.current_step = transition.current_step.wrapping_add(1);
            }
            Ok(())
        }

        fn edit_save_slot_name(
            &mut self,
            _selected_slot: Option<usize>,
            _edit_name: &mut SaveSlotName,
            _name_length: usize,
        ) -> AnyResult<bool> {
            self.calls.borrow_mut().push(SemanticCall::Edit);
            Ok(self.commit)
        }

        fn create_save_file(&mut self, filename: &[u8]) -> AnyResult<bool> {
            self.calls.borrow_mut().push(SemanticCall::Create(
                String::from_utf8(filename.to_vec()).unwrap(),
            ));
            Ok(self.create_success)
        }

        fn write_created_save_file(&mut self, _data: &[u8]) -> AnyResult<()> {
            self.calls.borrow_mut().push(SemanticCall::WriteSave);
            Ok(())
        }

        fn read_save_file(&mut self, filename: &[u8]) -> AnyResult<Option<Box<[u8]>>> {
            self.calls.borrow_mut().push(SemanticCall::Read(
                String::from_utf8(filename.to_vec()).unwrap(),
            ));
            Ok(self.open_success.then(|| Box::<[u8]>::from([4, 0])))
        }

        fn write_save_slot_directory(
            &mut self,
            _data: &[u8; ORIGINAL_SAVE_SLOT_DIRECTORY_BYTE_COUNT],
        ) -> AnyResult<()> {
            self.calls.borrow_mut().push(SemanticCall::WriteDirectory);
            Ok(())
        }
    }

    struct MockProfiles {
        calls: Rc<RefCell<Vec<SemanticCall>>>,
    }

    impl SaveProfileBackend for MockProfiles {
        fn capture_save_game(&mut self) -> AnyResult<OriginalSaveGame> {
            self.calls.borrow_mut().push(SemanticCall::Capture);
            let bytes = vec![u8::MIN; super::super::ORIGINAL_SAVE_FIXED_HEADER_BYTE_COUNT];
            Ok(OriginalSaveGame::decode(&bytes, usize::MIN).unwrap())
        }

        fn restore_save_game(&mut self, _data: &[u8]) -> AnyResult<()> {
            self.calls.borrow_mut().push(SemanticCall::Restore);
            Ok(())
        }
    }

    fn original_asset(name: &str) -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join("accuracy/cblood_install/cblood")
            .join(name)
    }

    fn slot_directory() -> OriginalSaveSlotDirectory {
        OriginalSaveSlotDirectory::decode(&std::fs::read(original_asset("BLOOD.SAV")).unwrap())
            .unwrap()
    }

    fn call_name(call: &Value) -> &str {
        call.get("call").and_then(Value::as_str).unwrap()
    }

    fn call_result(call: &Value, field: &str) -> Option<u64> {
        call.get(field).and_then(Value::as_u64)
    }

    fn oracle_selection(vector: &SaveLoadOracle) -> Vec<SaveLoadSelection> {
        vector
            .calls
            .iter()
            .filter(|call| call_name(call) == "list_widget_layout_unified")
            .map(|call| {
                let result = call_result(call, "result").unwrap();
                if result == u64::from(u16::MAX) {
                    SaveLoadSelection::None
                } else if vector.name.contains("sentinel") {
                    SaveLoadSelection::Close
                } else {
                    SaveLoadSelection::Slot(result as usize)
                }
            })
            .collect()
    }

    fn expected_calls(name: &str) -> Vec<SemanticCall> {
        use SaveLoadListPass::{MeasureEditingName, Poll};
        match name {
            "inactive" => vec![],
            "quicksave_create_failure" => vec![SemanticCall::Create("game10.sav".into())],
            "quicksave_serializes_all_blocks" => vec![
                SemanticCall::Create("game10.sav".into()),
                SemanticCall::Capture,
                SemanticCall::WriteSave,
                SemanticCall::WriteDirectory,
            ],
            "phase_one_initializes_and_advances" => vec![
                SemanticCall::Layout(MeasureEditingName),
                SemanticCall::Transition,
            ],
            "complete_transition_negative_save_selection" => vec![
                SemanticCall::Transition,
                SemanticCall::Layout(Poll),
                SemanticCall::Edit,
            ],
            "save_selection_begins_slot_edit"
            | "reserved_quicksave_selection_returns"
            | "save_sentinel_closes_menu" => {
                vec![SemanticCall::Layout(Poll), SemanticCall::Edit]
            }
            "name_commit_serializes_selected_slot" => vec![
                SemanticCall::Layout(Poll),
                SemanticCall::Edit,
                SemanticCall::Create("game3.sav".into()),
                SemanticCall::Capture,
                SemanticCall::WriteSave,
                SemanticCall::WriteDirectory,
            ],
            "negative_load_selection" | "load_sentinel_closes_after_directory_setup" => {
                vec![SemanticCall::Layout(Poll)]
            }
            "load_open_failure_closes_menu" => vec![
                SemanticCall::Layout(Poll),
                SemanticCall::Read("game2.sav".into()),
            ],
            "load_restores_all_blocks_and_rebuilds" => vec![
                SemanticCall::Layout(Poll),
                SemanticCall::Read("game7.sav".into()),
                SemanticCall::Restore,
            ],
            other => panic!("unknown save/load oracle scenario {other}"),
        }
    }

    #[test]
    fn coordinator_matches_all_original_control_flow_vectors() {
        let input = include_str!("../../../../../re/tools/oracle_vectors/func_1b4b_natural.json");
        let vectors: Vec<SaveLoadOracle> = serde_json::from_str(input).unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let calls = Rc::new(RefCell::new(Vec::new()));
            let commit = vector.calls.iter().any(|call| {
                call_name(call) == "save_slot_name_edit_step"
                    && call.get("commit").and_then(Value::as_bool) == Some(true)
            });
            let create_success = vector
                .calls
                .iter()
                .find(|call| call_name(call) == "dos_create_truncate")
                .and_then(|call| call.get("success"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let open_success = vector
                .calls
                .iter()
                .find(|call| call_name(call) == "dos_open_read_only")
                .and_then(|call| call.get("success"))
                .and_then(Value::as_bool)
                .unwrap_or(true);
            let mut host = MockHost {
                calls: calls.clone(),
                selections: oracle_selection(&vector),
                commit,
                create_success,
                open_success,
            };
            let mut profiles = MockProfiles {
                calls: calls.clone(),
            };
            let mut directory = slot_directory();
            let mut state = SaveLoadMenuState::new(INITIAL_UI_FLAGS);
            state.requests = SaveLoadRequests {
                save: vector.initial.save != u8::MIN,
                load: vector.initial.load != u8::MIN,
                quick_save: vector.initial.quick & 1 != u8::MIN,
            };
            state.phase = match vector.initial.phase {
                1 => SaveLoadMenuPhase::LayoutPending,
                2 => SaveLoadMenuPhase::Transitioning,
                _ => SaveLoadMenuPhase::Ready,
            };
            state.transition = FramebufferTransitionState {
                current_step: vector.initial.current,
                total_steps: vector.initial.total,
            };
            state.preserve_layout_widths = true;
            state.active_slot = Some(INITIAL_ACTIVE_SLOT);
            state.edit_name = SaveSlotName::from_bytes(INITIAL_EDIT_NAME);

            let outcome =
                update_save_load_menu(&mut state, &mut directory, &mut host, &mut profiles)
                    .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(
                *calls.borrow(),
                expected_calls(&vector.name),
                "{}",
                vector.name
            );
            assert_eq!(
                state.requests.save,
                vector.final_state.save != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.requests.load,
                vector.final_state.load != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.requests.quick_save,
                vector.final_state.quick != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ui_flags & SAVE_LOAD_UI_ACTIVE_MASK,
                vector.final_state.ui_flags & SAVE_LOAD_UI_ACTIVE_MASK,
                "{}",
                vector.name
            );
            assert_eq!(
                state.phase,
                match vector.final_state.phase {
                    1 => SaveLoadMenuPhase::LayoutPending,
                    2 => SaveLoadMenuPhase::Transitioning,
                    _ => SaveLoadMenuPhase::Ready,
                },
                "{}",
                vector.name
            );
            assert_eq!(
                state.redraw_pending,
                vector.name == "load_restores_all_blocks_and_rebuilds",
                "{} oracle redraw byte {}",
                vector.name,
                vector.final_state.redraw
            );
            assert_eq!(
                state.palette_dirty,
                vector.name == "load_restores_all_blocks_and_rebuilds",
                "{} oracle palette byte {}",
                vector.name,
                vector.final_state.palette_dirty
            );
            if vector.name == "phase_one_initializes_and_advances" {
                assert_eq!(state.selected_slot, Some(usize::MIN));
                assert_eq!(state.active_slot, Some(usize::MIN));
                assert!(!state.preserve_layout_widths);
                assert_eq!(outcome, SaveLoadMenuOutcome::Transitioning);
            }
            if vector.name == "save_selection_begins_slot_edit" {
                assert_eq!(state.selected_slot, Some(2));
                assert_eq!(state.active_slot, Some(2));
            }
            if vector.initial.save != u8::MIN
                && vector.initial.quick & 1 == u8::MIN
                && vector.initial.phase != 1
            {
                assert_eq!(state.name_length, 6, "{}", vector.name);
            }
            if vector.initial.quick & 1 != u8::MIN {
                assert_eq!(
                    &directory.slots()[ORIGINAL_QUICK_SAVE_SLOT_INDEX]
                        .display_name()
                        .bytes()[..QUICK_SAVE_NAME_PREFIX.len()],
                    &QUICK_SAVE_NAME_PREFIX,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[derive(Default)]
    struct LifecycleProbe {
        events: Vec<&'static str>,
    }

    impl SavedProfileLifecycle for LifecycleProbe {
        fn initialize_loaded_profile(
            &mut self,
            _profile: &mut LoadedScriptProfile,
        ) -> AnyResult<()> {
            self.events.push("initialize");
            Ok(())
        }

        fn rebuild_loaded_state(&mut self, _profile: &mut LoadedScriptProfile) -> AnyResult<()> {
            self.events.push("rebuild");
            Ok(())
        }
    }

    #[test]
    fn concrete_profile_backend_restores_and_recaptures_the_shipped_save() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let resources = OriginalResourceCatalog::decode_bloodprg(executable).unwrap();
        let profiles = OriginalScriptProfileCatalog::decode_bloodprg(executable).unwrap();
        let root = original_asset("")
            .parent()
            .expect("asset root has a parent")
            .join("cblood");
        let store = OriginalResourceStore::new(root, None, [], true);
        let mut manager = ScriptProfileManager::new(profiles);
        let mut cache = OriginalResourceCache::new();
        let mut lifecycle = LifecycleProbe::default();
        let data = std::fs::read(original_asset("GAME1.SAV")).unwrap();

        {
            let mut backend = OriginalSaveProfileBackend::new(
                &mut manager,
                &mut cache,
                &store,
                &resources,
                &mut lifecycle,
            );
            backend.restore_save_game(&data).unwrap();
            assert_eq!(backend.capture_save_game().unwrap().encode(), data);
        }

        assert_eq!(lifecycle.events, ["initialize", "rebuild"]);
    }
}
