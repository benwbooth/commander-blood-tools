//! Ship HUD initialization and per-frame target-selection coordinator.

use std::ops::Range;

use super::{INITIAL_BRIDGE_VIEW_FRAME, ShipTargetSelectionOutcome};

const SHIP_HUD_UI_FLAG: u16 = 8;
const TARGET_LIST_INITIAL_PHASE: u8 = 1;
const TARGET_LIST_CENTER_X: i16 = 80;
const TARGET_LIST_TRANSITION_STEPS: u16 = 10;
const SHIP_HUD_ACTIVE_LINE: u16 = 3;
const SHIP_HUD_BAND_TOP: u16 = 35;
const SHIP_HUD_BAND_BOTTOM: u16 = 165;
const SHIP_ENTITY_FIRST: u16 = 0;
const SHIP_ENTITY_LAST: u16 = 31;
const PALETTE_TRANSITION_COMPLETE: u16 = 100;
const PALETTE_TRANSITION_INCREMENT: u16 = 10;
const PALETTE_TRANSITION_FIRST: u8 = 0;
const PALETTE_TRANSITION_LAST: u8 = 192;
const SHIP_PRESENTATION_CLOSED_STATE: u16 = 17;

/// Original whose ship-HUD coordination semantics are required.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipHudCoordinatorVariant {
    /// Commander Blood builds and interactively selects its HUD target list.
    CommanderBlood,
    /// Big Bug Bang consumes the script-owned current target directly.
    BigBugBang,
}

/// Target-list layout state configured during first-time HUD initialization.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipHudTargetListState {
    /// Native low-bit phase used by the target selector.
    pub phase: u8,
    /// Horizontal center in original logical-screen coordinates.
    pub center_x: i16,
    /// Preserve each target label's measured width.
    pub preserve_label_widths: bool,
    /// Include the synthetic final cancel row.
    pub include_cancel_entry: bool,
    /// Number of opening interpolation steps.
    pub transition_steps: u16,
}

/// Palette transition prepared when the HUD first opens.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipHudPaletteTransition {
    /// Whether live colors and the staged HUD tail have been captured.
    pub staged: bool,
    /// Current completion percentage.
    pub percent: u16,
    /// Per-frame percentage increment.
    pub increment: u16,
    /// First affected palette entry.
    pub first: u8,
    /// Last affected palette entry.
    pub last: u8,
}

/// Typed inputs used only by the first HUD initialization pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipHudInitializationContext<RecordId> {
    /// Arche record used to build the first presentable target list.
    pub arche: RecordId,
    /// Record linked from Arche's native navigation field.
    pub arche_link: RecordId,
    /// Whether the linked record itself is a directly selectable target.
    pub linked_record_is_direct_target: bool,
    /// Target list already owned by sequel script state before HUD entry.
    pub prebuilt_presentable_targets: Vec<RecordId>,
    /// Script-selected target published outside the HUD coordinator this frame.
    pub active_script_target: Option<RecordId>,
}

/// Outputs written by the C2 DESCRIPT lookup used by the ship HUD.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipHudDescriptionOutcome {
    /// Whether the selected DESCRIPT record replaced the music source.
    pub music_source_changed: bool,
    /// Scene row written to the shared `resource_vertical_offset` global.
    pub scene_top_row: u16,
}

/// Mutable state owned by the ship HUD coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipHudCoordinatorState<RecordId> {
    /// Whether the one-time HUD setup has completed.
    pub initialized: bool,
    /// Whether camera navigation requested fresh HUD palette staging.
    pub initialization_pending: bool,
    /// Subtitle display mode cleared during pending initialization.
    pub subtitle_display_mode: bool,
    /// Whether the pyramid/HUD palette tail has been staged.
    pub hud_palette_staged: bool,
    /// Bridge steering arc reset during initialization.
    pub bridge_seek_target_arc: u16,
    /// Current bridge panorama frame.
    pub bridge_view_frame: u16,
    /// Shared bridge UI flags.
    pub ui_state: u16,
    /// Whether MANU3 animation selector one has been requested.
    pub manu3_animation_requested: bool,
    /// Target-list geometry and phase configuration.
    pub target_list: ShipHudTargetListState,
    /// Current presentable records with no native sentinel or name offsets.
    pub presentable_targets: Vec<RecordId>,
    /// Current selected target record.
    pub current_target: RecordId,
    /// Whether the native current-target offset denotes a positive record.
    pub current_target_valid: bool,
    /// Whether scene dispatch remains blocked by the HUD presentation.
    pub scene_dispatch_blocked: bool,
    /// Current scripted presentation line.
    pub active_line: Option<u16>,
    /// Whether the ship depth-band composition is enabled.
    pub depth_band_enabled: bool,
    /// Current resource vertical offset.
    pub resource_vertical_offset: u16,
    /// C2 presentation gate.
    pub presentation_gate: u16,
    /// Palette transition staged by first-time initialization.
    pub palette_transition: ShipHudPaletteTransition,
    /// Whether the HUD is closing.
    pub exit_pending: bool,
    /// Whether the depth door is still opening.
    pub depth_opening: bool,
    /// Low-byte movement selected for the shared depth transition.
    pub depth_step: u8,
    /// Whether entity geometry has been snapshotted for clipping.
    pub clip_snapshot_ready: bool,
    /// Whether subtitle presentation currently owns the text surface.
    pub text_display_active: bool,
    /// Whether the text reveal cursor has reached its terminal NUL.
    pub text_reveal_complete: bool,
    /// Whether this frame reached target selection.
    pub frame_presented: bool,
    /// Whether the selected description changed the music source.
    pub music_source_changed: bool,
    /// Target published as the deferred C1 navigation command.
    pub deferred_navigation_target: Option<RecordId>,
    /// Top-level ship presentation state.
    pub ship_active_flags: u16,
    /// Whether a VM sequence remains active.
    pub sequence_active: bool,
    /// Whether bridge redraw remains pending.
    pub bridge_redraw_pending: bool,
    /// Whether the script VM may advance after presentation dispatch.
    pub vm_execution_enabled: bool,
}

/// Ordered renderer, resource, input, and audio work used by the HUD loop.
pub trait ShipHudCoordinatorHost<RecordId> {
    /// Clear the back buffer before first-time HUD setup.
    fn clear_back_buffer(&mut self);
    /// Publish the recovered bridge seek target and panorama frame.
    fn initialize_bridge_view(&mut self, seek_target_arc: u16, view_frame: u16);
    /// Rebuild VM navigation state after HUD activation.
    fn process_vm_state(&mut self);
    /// Build presentable typed records below one root.
    fn build_presentable_targets(&mut self, root: &RecordId) -> Vec<RecordId>;
    /// Load one selected record's description assets and return its shared outputs.
    fn load_target_description(&mut self, target: &RecordId) -> ShipHudDescriptionOutcome;
    /// Dispatch the initial authored ship scene line and publish its immediate shared writes.
    fn dispatch_ship_scene_line(
        &mut self,
        vertical_offset: u16,
        state: &mut ShipHudCoordinatorState<RecordId>,
    );
    /// Copy the display surface into the back buffer.
    fn copy_display_to_back_buffer(&mut self);
    /// Compose the separately recovered two-band depth effect.
    fn compose_depth_band(&mut self);
    /// Update bridge steering for the current frame.
    fn update_bridge_steering(&mut self);
    /// Rebuild the HUD's dark palette remap under the supplied temporary clip.
    fn prepare_hud_remap(&mut self, rows: Range<u16>);
    /// Commit dirty ship entities within the supplied inclusive range.
    fn commit_ship_entities(&mut self, entities: Range<u16>);
    /// Copy dirty work-surface pixels into the display surface.
    fn copy_dirty_regions(&mut self);
    /// Advance the target selector and return its updated depth-transition state.
    fn update_target_selection(&mut self) -> (ShipTargetSelectionOutcome<RecordId>, bool, u8);
    /// Reset the audio driver after a music-source change.
    fn reset_audio_driver(&mut self);
    /// Load the new presentation music source.
    fn load_music_source(&mut self);
    /// Start or resume the selected target's audio stream.
    fn start_audio_stream(&mut self);
}

/// Terminal path selected by one HUD coordinator update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipHudCoordinatorOutcome {
    /// Closing waits for the depth opening transition to complete.
    CloseDeferred,
    /// Closing completed and returned control to ship presentation state 17.
    Closed,
    /// Subtitle display is inactive.
    TextInactive,
    /// Subtitle reveal has not reached its terminal character.
    TextRevealing,
    /// Target selection or its opening interpolation remains incomplete.
    NoSelection,
    /// A selected target was published as a deferred navigation command.
    TargetQueued,
}

/// Invalid decoded state encountered during HUD initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipHudCoordinatorError {
    /// Arche's presentable list was empty when the linked record was not direct.
    MissingInitialTarget,
}

/// Run a native BLOODPRG ship-HUD coordinator over flat typed state.
///
/// Typed record identities and vectors replace the native record heap, name
/// offsets, and sentinel arrays. The original initialization and frame call
/// order remains visible through [`ShipHudCoordinatorHost`]. Commander Blood's
/// body starts at `0x00B079`; Big Bug Bang's changed body starts at `0x00C859`.
pub fn update_ship_hud<RecordId, Host>(
    variant: ShipHudCoordinatorVariant,
    state: &mut ShipHudCoordinatorState<RecordId>,
    context: &ShipHudInitializationContext<RecordId>,
    host: &mut Host,
) -> Result<ShipHudCoordinatorOutcome, ShipHudCoordinatorError>
where
    RecordId: Clone + Eq,
    Host: ShipHudCoordinatorHost<RecordId>,
{
    if variant == ShipHudCoordinatorVariant::BigBugBang
        && state.initialized
        && let Some(target) = &context.active_script_target
    {
        state.current_target = target.clone();
        state.current_target_valid = true;
    }
    if !state.initialized {
        initialize_ship_hud(variant, state, context, host)?;
    }

    if state.exit_pending {
        return Ok(close_ship_hud(state));
    }

    host.update_bridge_steering();
    if state.ui_state & SHIP_HUD_UI_FLAG != u16::MIN {
        host.prepare_hud_remap(SHIP_HUD_BAND_TOP..SHIP_HUD_BAND_BOTTOM);
    }

    state.clip_snapshot_ready = true;
    host.commit_ship_entities(SHIP_ENTITY_FIRST..SHIP_ENTITY_LAST + 1);
    host.copy_dirty_regions();
    if !state.text_display_active {
        return Ok(ShipHudCoordinatorOutcome::TextInactive);
    }
    if !state.text_reveal_complete {
        return Ok(ShipHudCoordinatorOutcome::TextRevealing);
    }

    if state.palette_transition.percent == PALETTE_TRANSITION_COMPLETE
        && state.palette_transition.increment == PALETTE_TRANSITION_INCREMENT
    {
        state.palette_transition.increment = u16::MIN;
    }
    state.frame_presented = true;
    let selection = match variant {
        ShipHudCoordinatorVariant::CommanderBlood => {
            let (selection, depth_opening, depth_step) = host.update_target_selection();
            state.depth_opening = depth_opening;
            state.depth_step = depth_step;
            selection
        }
        ShipHudCoordinatorVariant::BigBugBang => {
            if state.current_target_valid {
                ShipTargetSelectionOutcome::Selected(state.current_target.clone())
            } else {
                ShipTargetSelectionOutcome::CloseRequested
            }
        }
    };
    match selection {
        ShipTargetSelectionOutcome::Transitioning | ShipTargetSelectionOutcome::NoSelection => {
            Ok(ShipHudCoordinatorOutcome::NoSelection)
        }
        ShipTargetSelectionOutcome::CloseRequested => Ok(close_ship_hud(state)),
        ShipTargetSelectionOutcome::Selected(target) => {
            if target != state.current_target {
                state.current_target = target.clone();
                let description = host.load_target_description(&target);
                state.music_source_changed = description.music_source_changed;
                state.resource_vertical_offset = description.scene_top_row;
            }

            if state.music_source_changed {
                host.compose_depth_band();
                host.reset_audio_driver();
                host.load_music_source();
                state.music_source_changed = false;
            }
            host.start_audio_stream();
            state.deferred_navigation_target = Some(target);
            state.scene_dispatch_blocked = false;
            Ok(ShipHudCoordinatorOutcome::TargetQueued)
        }
    }
}

fn initialize_ship_hud<RecordId, Host>(
    variant: ShipHudCoordinatorVariant,
    state: &mut ShipHudCoordinatorState<RecordId>,
    context: &ShipHudInitializationContext<RecordId>,
    host: &mut Host,
) -> Result<(), ShipHudCoordinatorError>
where
    RecordId: Clone,
    Host: ShipHudCoordinatorHost<RecordId>,
{
    if state.initialization_pending {
        state.initialization_pending = false;
        state.subtitle_display_mode = false;
        state.hud_palette_staged = true;
    }

    host.clear_back_buffer();
    state.bridge_seek_target_arc = u16::MIN;
    state.bridge_view_frame = INITIAL_BRIDGE_VIEW_FRAME;
    host.initialize_bridge_view(state.bridge_seek_target_arc, state.bridge_view_frame);
    state.ui_state |= SHIP_HUD_UI_FLAG;
    state.manu3_animation_requested = true;
    state.initialized = true;
    host.process_vm_state();

    state.target_list = ShipHudTargetListState {
        phase: TARGET_LIST_INITIAL_PHASE,
        center_x: TARGET_LIST_CENTER_X,
        preserve_label_widths: true,
        include_cancel_entry: true,
        transition_steps: TARGET_LIST_TRANSITION_STEPS,
    };
    state.presentable_targets = match variant {
        ShipHudCoordinatorVariant::CommanderBlood => host.build_presentable_targets(&context.arche),
        ShipHudCoordinatorVariant::BigBugBang => context.prebuilt_presentable_targets.clone(),
    };
    if context.linked_record_is_direct_target {
        state.presentable_targets = host.build_presentable_targets(&context.arche_link);
        state.current_target = context.arche_link.clone();
    } else {
        state.current_target = state
            .presentable_targets
            .first()
            .cloned()
            .ok_or(ShipHudCoordinatorError::MissingInitialTarget)?;
    }
    state.current_target_valid = true;
    let description = host.load_target_description(&state.current_target);
    state.music_source_changed = description.music_source_changed;
    state.resource_vertical_offset = description.scene_top_row;

    state.scene_dispatch_blocked = true;
    state.active_line = Some(SHIP_HUD_ACTIVE_LINE);
    state.depth_band_enabled = true;
    state.presentation_gate = u16::MIN;
    host.dispatch_ship_scene_line(state.resource_vertical_offset, state);
    if variant == ShipHudCoordinatorVariant::BigBugBang {
        state.vm_execution_enabled = true;
    }
    host.copy_display_to_back_buffer();
    host.compose_depth_band();
    state.palette_transition = ShipHudPaletteTransition {
        staged: true,
        percent: u16::MIN,
        increment: PALETTE_TRANSITION_INCREMENT,
        first: PALETTE_TRANSITION_FIRST,
        last: PALETTE_TRANSITION_LAST,
    };
    Ok(())
}

fn close_ship_hud<RecordId>(
    state: &mut ShipHudCoordinatorState<RecordId>,
) -> ShipHudCoordinatorOutcome {
    state.exit_pending = true;
    if state.depth_opening {
        return ShipHudCoordinatorOutcome::CloseDeferred;
    }

    state.ship_active_flags = SHIP_PRESENTATION_CLOSED_STATE;
    state.sequence_active = false;
    state.text_display_active = false;
    state.scene_dispatch_blocked = false;
    state.bridge_redraw_pending = false;
    state.exit_pending = false;
    ShipHudCoordinatorOutcome::Closed
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;

    const ARCHE_RECORD: u16 = 4_608;
    const ARCHE_LINK: u16 = 4_660;
    const FIRST_PRESENTABLE_TARGET: u16 = 9_029;
    const INITIAL_CURRENT_TARGET: u16 = 9_029;
    const SCENE_TOP_ROW: u16 = 4_951;

    #[derive(Clone, Deserialize)]
    struct HudVector {
        name: String,
        initialized: u8,
        init_pending: u8,
        exit_pending: u8,
        opening: u8,
        ui_flags: u16,
        text_active: u8,
        text_character: u8,
        selection: u16,
        music_changed: u8,
        probe_eax: u32,
        probe_mask: u16,
        calls: Vec<CallVector>,
        current_target_after: u16,
        c1_record: [u16; 3],
    }

    #[derive(Clone, Deserialize)]
    struct BigBugBangHudVector {
        name: String,
        initialized: u8,
        init_pending: u8,
        exit_pending: u8,
        opening: u8,
        ui_flags: u16,
        text_active: u8,
        text_character: u8,
        current_target_before: u16,
        current_target_after: u16,
        music_changed: u8,
        transition_percent: u16,
        transition_increment: u16,
        probe_eax: u32,
        probe_mask: u16,
        vm_enabled_before: u8,
        outcome: String,
        calls: Vec<CallVector>,
        c1_record: [u16; 3],
        state_after: BigBugBangHudState,
    }

    #[derive(Clone, Deserialize)]
    struct BigBugBangHudState {
        initialized: u8,
        sequence_active: u8,
        scene_blocked: u8,
        depth_band: u8,
        depth_opening: u8,
        exit_pending: u8,
        init_pending: u8,
        ship_flags: u16,
        subtitle_mode: u8,
        ui_flags: u16,
        redraw_pending: u8,
        current_target: u16,
        vm_enabled: u8,
        active_line: u16,
        resource_vertical: u16,
        presentation_gate: u16,
        frame_presented: u16,
        transition_percent: u16,
        transition_increment: u16,
        transition_first: u8,
        transition_last: u8,
        text_active: u8,
    }

    #[derive(Clone, Deserialize)]
    struct CallVector {
        name: String,
    }

    struct OracleHost {
        calls: VecDeque<String>,
        selection: u16,
        music_changed: bool,
        depth_opening: bool,
        depth_step: u8,
    }

    impl OracleHost {
        fn expect(&mut self, name: &str) {
            assert_eq!(self.calls.pop_front().as_deref(), Some(name));
        }
    }

    impl ShipHudCoordinatorHost<u16> for OracleHost {
        fn clear_back_buffer(&mut self) {
            self.expect("backclear");
        }

        fn initialize_bridge_view(&mut self, seek_target_arc: u16, view_frame: u16) {
            assert_eq!(seek_target_arc, u16::MIN);
            assert_eq!(view_frame, INITIAL_BRIDGE_VIEW_FRAME);
        }

        fn process_vm_state(&mut self) {
            self.expect("state");
        }

        fn build_presentable_targets(&mut self, _root: &u16) -> Vec<u16> {
            self.expect("presentable");
            vec![FIRST_PRESENTABLE_TARGET]
        }

        fn load_target_description(&mut self, _target: &u16) -> ShipHudDescriptionOutcome {
            self.expect("c2");
            ShipHudDescriptionOutcome {
                music_source_changed: self.music_changed,
                scene_top_row: SCENE_TOP_ROW,
            }
        }

        fn dispatch_ship_scene_line(
            &mut self,
            vertical_offset: u16,
            _state: &mut ShipHudCoordinatorState<u16>,
        ) {
            self.expect("dispatch");
            assert_eq!(vertical_offset, SCENE_TOP_ROW);
        }

        fn copy_display_to_back_buffer(&mut self) {
            self.expect("fullscreen");
        }

        fn compose_depth_band(&mut self) {
            self.expect("band");
        }

        fn update_bridge_steering(&mut self) {
            self.expect("bridge");
        }

        fn prepare_hud_remap(&mut self, rows: Range<u16>) {
            self.expect("palette");
            assert_eq!(rows, SHIP_HUD_BAND_TOP..SHIP_HUD_BAND_BOTTOM);
        }

        fn commit_ship_entities(&mut self, entities: Range<u16>) {
            self.expect("commit");
            assert_eq!(entities, SHIP_ENTITY_FIRST..SHIP_ENTITY_LAST + 1);
        }

        fn copy_dirty_regions(&mut self) {
            self.expect("dirty");
        }

        fn update_target_selection(&mut self) -> (ShipTargetSelectionOutcome<u16>, bool, u8) {
            self.expect("target");
            let outcome = match self.selection {
                u16::MIN => ShipTargetSelectionOutcome::NoSelection,
                u16::MAX => {
                    self.depth_opening = true;
                    self.depth_step = 6;
                    ShipTargetSelectionOutcome::CloseRequested
                }
                target => ShipTargetSelectionOutcome::Selected(target),
            };
            (outcome, self.depth_opening, self.depth_step)
        }

        fn reset_audio_driver(&mut self) {
            self.expect("driver");
        }

        fn load_music_source(&mut self) {
            self.expect("source");
        }

        fn start_audio_stream(&mut self) {
            self.expect("stream");
        }
    }

    #[test]
    fn hud_coordinator_matches_every_original_vector() {
        let vectors: Vec<HudVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b079_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 15);

        for vector in vectors {
            assert_eq!(vector.probe_eax as u16, ARCHE_LINK);
            let mut state = initial_state(&vector);
            let transition_before = state.palette_transition;
            let linked_record_is_direct_target = vector.probe_mask & 320 == u16::MIN;
            let context = ShipHudInitializationContext {
                arche: ARCHE_RECORD,
                arche_link: ARCHE_LINK,
                linked_record_is_direct_target,
                prebuilt_presentable_targets: vec![FIRST_PRESENTABLE_TARGET],
                active_script_target: None,
            };
            let calls = vector.calls.iter().map(|call| call.name.clone()).collect();
            let mut host = OracleHost {
                calls,
                selection: vector.selection,
                music_changed: vector.music_changed != u8::MIN,
                depth_opening: vector.opening != u8::MIN,
                depth_step: u8::MIN,
            };

            let outcome = update_ship_hud(
                ShipHudCoordinatorVariant::CommanderBlood,
                &mut state,
                &context,
                &mut host,
            )
            .unwrap();

            assert!(host.calls.is_empty(), "{}", vector.name);
            assert_eq!(outcome, expected_outcome(&vector), "{}", vector.name);
            assert_eq!(
                state.current_target, vector.current_target_after,
                "{}",
                vector.name
            );
            assert_vector_state(&vector, &state, transition_before, outcome);
        }
    }

    #[test]
    fn sequel_hud_coordinator_matches_every_direct_original_vector() {
        let vectors: Vec<BigBugBangHudVector> =
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_ship_hud.jsonl")
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        assert_eq!(vectors.len(), 15);

        for vector in vectors {
            assert_eq!(vector.probe_eax as u16, ARCHE_LINK, "{}", vector.name);
            let mut state = initial_sequel_state(&vector);
            let context = ShipHudInitializationContext {
                arche: ARCHE_RECORD,
                arche_link: ARCHE_LINK,
                linked_record_is_direct_target: vector.probe_mask & 320 == u16::MIN,
                prebuilt_presentable_targets: vec![FIRST_PRESENTABLE_TARGET],
                active_script_target: None,
            };
            let calls = vector.calls.iter().map(|call| call.name.clone()).collect();
            let mut host = OracleHost {
                calls,
                selection: u16::MIN,
                music_changed: vector.music_changed != u8::MIN,
                depth_opening: vector.opening != u8::MIN,
                depth_step: u8::MIN,
            };

            let outcome = update_ship_hud(
                ShipHudCoordinatorVariant::BigBugBang,
                &mut state,
                &context,
                &mut host,
            )
            .unwrap();

            assert!(host.calls.is_empty(), "{}", vector.name);
            assert_eq!(outcome, sequel_outcome(&vector.outcome), "{}", vector.name);
            assert_eq!(
                state.current_target, vector.current_target_after,
                "{}",
                vector.name
            );
            assert_sequel_state(&vector, &state, outcome);
        }
    }

    #[test]
    fn sequel_imports_a_script_owned_target_without_running_the_commander_selector() {
        const SCRIPT_TARGET: u16 = 0x4567;
        let vectors: Vec<BigBugBangHudVector> =
            include_str!("../../../../../re/tools/oracle_vectors/big_bug_bang_ship_hud.jsonl")
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        let vector = vectors
            .into_iter()
            .find(|vector| {
                vector.name == "positive_current_target_queues_without_selector_or_lookup"
            })
            .unwrap();
        let mut state = initial_sequel_state(&vector);
        let calls = vector.calls.iter().map(|call| call.name.clone()).collect();
        let mut host = OracleHost {
            calls,
            selection: u16::MIN,
            music_changed: false,
            depth_opening: true,
            depth_step: u8::MIN,
        };

        let outcome = update_ship_hud(
            ShipHudCoordinatorVariant::BigBugBang,
            &mut state,
            &ShipHudInitializationContext {
                arche: ARCHE_RECORD,
                arche_link: ARCHE_LINK,
                linked_record_is_direct_target: false,
                prebuilt_presentable_targets: vec![FIRST_PRESENTABLE_TARGET],
                active_script_target: Some(SCRIPT_TARGET),
            },
            &mut host,
        )
        .unwrap();

        assert!(host.calls.is_empty());
        assert_eq!(outcome, ShipHudCoordinatorOutcome::TargetQueued);
        assert_eq!(state.current_target, SCRIPT_TARGET);
        assert!(state.current_target_valid);
        assert_eq!(state.deferred_navigation_target, Some(SCRIPT_TARGET));
    }

    #[test]
    fn missing_initial_target_is_reported_without_offset_indexing() {
        struct EmptyHost;

        impl ShipHudCoordinatorHost<u16> for EmptyHost {
            fn clear_back_buffer(&mut self) {}
            fn initialize_bridge_view(&mut self, _seek_target_arc: u16, _view_frame: u16) {}
            fn process_vm_state(&mut self) {}
            fn build_presentable_targets(&mut self, _root: &u16) -> Vec<u16> {
                Vec::new()
            }
            fn load_target_description(&mut self, _target: &u16) -> ShipHudDescriptionOutcome {
                panic!("missing target must fail before lookup");
            }
            fn dispatch_ship_scene_line(
                &mut self,
                _vertical_offset: u16,
                _state: &mut ShipHudCoordinatorState<u16>,
            ) {
            }
            fn copy_display_to_back_buffer(&mut self) {}
            fn compose_depth_band(&mut self) {}
            fn update_bridge_steering(&mut self) {}
            fn prepare_hud_remap(&mut self, _rows: Range<u16>) {}
            fn commit_ship_entities(&mut self, _entities: Range<u16>) {}
            fn copy_dirty_regions(&mut self) {}
            fn update_target_selection(&mut self) -> (ShipTargetSelectionOutcome<u16>, bool, u8) {
                (ShipTargetSelectionOutcome::NoSelection, false, u8::MIN)
            }
            fn reset_audio_driver(&mut self) {}
            fn load_music_source(&mut self) {}
            fn start_audio_stream(&mut self) {}
        }

        let vector = HudVector {
            name: "missing".to_owned(),
            initialized: 0,
            init_pending: 0,
            exit_pending: 0,
            opening: 1,
            ui_flags: 0,
            text_active: 1,
            text_character: 0,
            selection: 0,
            music_changed: 0,
            probe_eax: u32::from(ARCHE_LINK),
            probe_mask: 320,
            calls: Vec::new(),
            current_target_after: 0,
            c1_record: [0; 3],
        };
        let mut state = initial_state(&vector);
        let error = update_ship_hud(
            ShipHudCoordinatorVariant::CommanderBlood,
            &mut state,
            &ShipHudInitializationContext {
                arche: ARCHE_RECORD,
                arche_link: ARCHE_LINK,
                linked_record_is_direct_target: false,
                prebuilt_presentable_targets: Vec::new(),
                active_script_target: None,
            },
            &mut EmptyHost,
        )
        .unwrap_err();
        assert_eq!(error, ShipHudCoordinatorError::MissingInitialTarget);
    }

    fn initial_state(vector: &HudVector) -> ShipHudCoordinatorState<u16> {
        let (percent, increment) = match vector.name.as_str() {
            "transition_complete_resets_increment" => (100, 10),
            "transition_percent_alone_does_not_reset_increment" => (100, 9),
            _ => (37, 6),
        };
        ShipHudCoordinatorState {
            initialized: vector.initialized != u8::MIN,
            initialization_pending: vector.init_pending != u8::MIN,
            subtitle_display_mode: true,
            hud_palette_staged: false,
            bridge_seek_target_arc: 39_067,
            bridge_view_frame: 38_293,
            ui_state: vector.ui_flags,
            manu3_animation_requested: false,
            target_list: ShipHudTargetListState::default(),
            presentable_targets: Vec::new(),
            current_target: INITIAL_CURRENT_TARGET,
            current_target_valid: true,
            scene_dispatch_blocked: true,
            active_line: Some(34_952),
            depth_band_enabled: false,
            resource_vertical_offset: 42_919,
            presentation_gate: 45_746,
            palette_transition: ShipHudPaletteTransition {
                staged: false,
                percent,
                increment,
                first: 81,
                last: 82,
            },
            exit_pending: vector.exit_pending != u8::MIN,
            depth_opening: vector.opening != u8::MIN,
            depth_step: u8::MIN,
            clip_snapshot_ready: false,
            text_display_active: vector.text_active != u8::MIN,
            text_reveal_complete: vector.text_character == u8::MIN,
            frame_presented: false,
            music_source_changed: vector.music_changed != u8::MIN,
            deferred_navigation_target: None,
            ship_active_flags: 62_451,
            sequence_active: true,
            bridge_redraw_pending: true,
            vm_execution_enabled: false,
        }
    }

    fn initial_sequel_state(vector: &BigBugBangHudVector) -> ShipHudCoordinatorState<u16> {
        ShipHudCoordinatorState {
            initialized: vector.initialized != u8::MIN,
            initialization_pending: vector.init_pending != u8::MIN,
            subtitle_display_mode: true,
            hud_palette_staged: false,
            bridge_seek_target_arc: 39_067,
            bridge_view_frame: 38_293,
            ui_state: vector.ui_flags,
            manu3_animation_requested: false,
            target_list: ShipHudTargetListState::default(),
            presentable_targets: Vec::new(),
            current_target: vector.current_target_before,
            current_target_valid: vector.current_target_before != u16::MIN
                && vector.current_target_before < 0x8000,
            scene_dispatch_blocked: true,
            active_line: Some(0x5A5A),
            depth_band_enabled: true,
            resource_vertical_offset: 0xA7A7,
            presentation_gate: 0x00B2,
            palette_transition: ShipHudPaletteTransition {
                staged: false,
                percent: vector.transition_percent,
                increment: vector.transition_increment,
                first: 0x51,
                last: 0x52,
            },
            exit_pending: vector.exit_pending != u8::MIN,
            depth_opening: vector.opening != u8::MIN,
            depth_step: u8::MIN,
            clip_snapshot_ready: false,
            text_display_active: vector.text_active != u8::MIN,
            text_reveal_complete: vector.text_character == u8::MIN,
            frame_presented: true,
            music_source_changed: vector.music_changed != u8::MIN,
            deferred_navigation_target: None,
            ship_active_flags: 0xF3F3,
            sequence_active: true,
            bridge_redraw_pending: true,
            vm_execution_enabled: vector.vm_enabled_before != u8::MIN,
        }
    }

    fn sequel_outcome(outcome: &str) -> ShipHudCoordinatorOutcome {
        match outcome {
            "close_deferred" => ShipHudCoordinatorOutcome::CloseDeferred,
            "closed" => ShipHudCoordinatorOutcome::Closed,
            "text_inactive" => ShipHudCoordinatorOutcome::TextInactive,
            "text_revealing" => ShipHudCoordinatorOutcome::TextRevealing,
            "target_queued" => ShipHudCoordinatorOutcome::TargetQueued,
            other => panic!("unknown sequel HUD outcome {other}"),
        }
    }

    fn assert_sequel_state(
        vector: &BigBugBangHudVector,
        state: &ShipHudCoordinatorState<u16>,
        outcome: ShipHudCoordinatorOutcome,
    ) {
        let expected = &vector.state_after;
        assert_eq!(
            state.initialized,
            expected.initialized != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.sequence_active,
            expected.sequence_active != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.scene_dispatch_blocked,
            expected.scene_blocked != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.depth_band_enabled,
            expected.depth_band != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.depth_opening,
            expected.depth_opening != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.exit_pending,
            expected.exit_pending != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.initialization_pending,
            expected.init_pending != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.ship_active_flags, expected.ship_flags,
            "{}",
            vector.name
        );
        assert_eq!(
            state.subtitle_display_mode,
            expected.subtitle_mode != 0,
            "{}",
            vector.name
        );
        assert_eq!(state.ui_state, expected.ui_flags, "{}", vector.name);
        assert_eq!(
            state.bridge_redraw_pending,
            expected.redraw_pending != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.current_target, expected.current_target,
            "{}",
            vector.name
        );
        assert_eq!(
            state.vm_execution_enabled,
            expected.vm_enabled != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.active_line,
            Some(expected.active_line),
            "{}",
            vector.name
        );
        assert_eq!(
            state.resource_vertical_offset, expected.resource_vertical,
            "{}",
            vector.name
        );
        assert_eq!(
            state.presentation_gate, expected.presentation_gate,
            "{}",
            vector.name
        );
        assert_eq!(
            state.frame_presented,
            expected.frame_presented != 0,
            "{}",
            vector.name
        );
        assert_eq!(
            state.palette_transition.percent, expected.transition_percent,
            "{}",
            vector.name
        );
        assert_eq!(
            state.palette_transition.increment, expected.transition_increment,
            "{}",
            vector.name
        );
        assert_eq!(state.palette_transition.first, expected.transition_first);
        assert_eq!(state.palette_transition.last, expected.transition_last);
        assert_eq!(
            state.text_display_active,
            expected.text_active != 0,
            "{}",
            vector.name
        );
        if outcome == ShipHudCoordinatorOutcome::TargetQueued {
            assert_eq!(
                state.deferred_navigation_target,
                Some(vector.current_target_after),
                "{}",
                vector.name
            );
            assert_eq!(vector.c1_record, [193, vector.current_target_after, 0]);
        } else {
            assert_eq!(state.deferred_navigation_target, None, "{}", vector.name);
        }
    }

    fn expected_outcome(vector: &HudVector) -> ShipHudCoordinatorOutcome {
        match vector.name.as_str() {
            "exit_pending_finishes_close" => ShipHudCoordinatorOutcome::Closed,
            "exit_pending_defers_close_while_opening"
            | "negative_selection_closes_presentation" => ShipHudCoordinatorOutcome::CloseDeferred,
            "inactive_text_returns_after_frame_present" => ShipHudCoordinatorOutcome::TextInactive,
            "nonzero_text_cursor_blocks_target_selection" => {
                ShipHudCoordinatorOutcome::TextRevealing
            }
            "same_target_queues_c1_without_lookup"
            | "new_target_runs_descript_lookup"
            | "changed_music_rebuilds_plane_and_audio_source" => {
                ShipHudCoordinatorOutcome::TargetQueued
            }
            _ => ShipHudCoordinatorOutcome::NoSelection,
        }
    }

    fn assert_vector_state(
        vector: &HudVector,
        state: &ShipHudCoordinatorState<u16>,
        transition_before: ShipHudPaletteTransition,
        outcome: ShipHudCoordinatorOutcome,
    ) {
        if vector.initialized == u8::MIN {
            assert!(state.initialized, "{}", vector.name);
            assert_eq!(state.bridge_seek_target_arc, u16::MIN, "{}", vector.name);
            assert_eq!(
                state.bridge_view_frame, INITIAL_BRIDGE_VIEW_FRAME,
                "{}",
                vector.name
            );
            assert_ne!(state.ui_state & SHIP_HUD_UI_FLAG, u16::MIN);
            assert!(state.manu3_animation_requested, "{}", vector.name);
            assert_eq!(
                state.target_list,
                ShipHudTargetListState {
                    phase: TARGET_LIST_INITIAL_PHASE,
                    center_x: TARGET_LIST_CENTER_X,
                    preserve_label_widths: true,
                    include_cancel_entry: true,
                    transition_steps: TARGET_LIST_TRANSITION_STEPS,
                },
                "{}",
                vector.name
            );
            assert!(state.scene_dispatch_blocked, "{}", vector.name);
            assert_eq!(state.active_line, Some(SHIP_HUD_ACTIVE_LINE));
            assert!(state.depth_band_enabled, "{}", vector.name);
            assert_eq!(state.resource_vertical_offset, SCENE_TOP_ROW);
            assert_eq!(state.presentation_gate, u16::MIN);
            assert_eq!(
                state.palette_transition,
                ShipHudPaletteTransition {
                    staged: true,
                    percent: u16::MIN,
                    increment: PALETTE_TRANSITION_INCREMENT,
                    first: PALETTE_TRANSITION_FIRST,
                    last: PALETTE_TRANSITION_LAST,
                }
            );
            if vector.init_pending != u8::MIN {
                assert!(!state.initialization_pending);
                assert!(!state.subtitle_display_mode);
                assert!(state.hud_palette_staged);
            }
        } else if vector.name == "transition_complete_resets_increment" {
            assert_eq!(state.palette_transition.increment, u16::MIN);
        } else if vector.name == "transition_percent_alone_does_not_reset_increment" {
            assert_eq!(state.palette_transition, transition_before);
        }

        match outcome {
            ShipHudCoordinatorOutcome::Closed => {
                assert_eq!(state.ship_active_flags, SHIP_PRESENTATION_CLOSED_STATE);
                assert!(!state.sequence_active);
                assert!(!state.text_display_active);
                assert!(!state.scene_dispatch_blocked);
                assert!(!state.bridge_redraw_pending);
                assert!(!state.exit_pending);
            }
            ShipHudCoordinatorOutcome::CloseDeferred => assert!(state.exit_pending),
            ShipHudCoordinatorOutcome::TextInactive | ShipHudCoordinatorOutcome::TextRevealing => {
                assert!(state.clip_snapshot_ready);
                assert!(!state.frame_presented);
            }
            ShipHudCoordinatorOutcome::NoSelection => {
                assert!(state.clip_snapshot_ready);
                assert!(state.frame_presented);
                assert_eq!(state.deferred_navigation_target, None);
            }
            ShipHudCoordinatorOutcome::TargetQueued => {
                assert_eq!(
                    state.deferred_navigation_target,
                    Some(vector.selection),
                    "{}",
                    vector.name
                );
                assert!(!state.scene_dispatch_blocked);
                assert_eq!(vector.c1_record, [193, vector.selection, 0]);
            }
        }
    }
}
