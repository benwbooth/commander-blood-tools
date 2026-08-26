//! Ship-view navigation candidate selection, presentation, and teardown.

use super::{ChoiceListRect, PresentationWordChoicePhase};

const NAVIGATION_LIST_UI_FLAG: u16 = 4;
const NAVIGATION_LIST_TRANSITION_STEPS: u16 = 6;
const NAVIGATION_SCENE_TOP_ROW: u16 = 35;
const DEPTH_CLOSING_STEP: u8 = 2;
const RESET_UI_STATE: u16 = 9;
const RESET_BRIDGE_SEEK_DISTANCE: u16 = 50;
const PRESENTATION_REQUEST_LOW_BITS: u8 = 3;
const FULL_PALETTE_LAST_INDEX: u8 = u8::MAX;
const PALETTE_TRANSITION_INCREMENT: u16 = 10;

/// Candidate relation after decoding native record offsets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ShipNavigationRelation<RecordId> {
    /// The candidate is related to the root of the decoded record directory.
    RecordDirectoryRoot,
    /// The candidate is related to another typed object.
    Object(RecordId),
    /// A relation not relevant to this coordinator.
    Other,
}

/// One candidate produced by the separately recovered navigation traversal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipNavigationCandidate<RecordId> {
    /// Stable identity of the candidate record.
    pub record: RecordId,
    /// Typed replacement for its native relation offset.
    pub relation: ShipNavigationRelation<RecordId>,
}

/// Access counter owned by the current target or its redirect record.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipNavigationAccessCounter {
    /// The current target owns its access count directly.
    Direct(u16),
    /// A linked counter record owns the count.
    Redirected(u16),
}

impl ShipNavigationAccessCounter {
    fn increment(&mut self) {
        match self {
            Self::Direct(count) | Self::Redirected(count) => {
                *count = count.wrapping_add(1);
            }
        }
    }
}

/// Read-only candidate-filter identities for one navigation update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipNavigationContext<RecordId> {
    /// Ark's typed object identity.
    pub ark: RecordId,
    /// Root filter bit that permits any candidate relation.
    pub unrestricted_candidates: bool,
}

/// Mutable state owned by the ship navigation coordinator.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipNavigationState<RecordId> {
    /// One-shot request to build candidates and enter navigation presentation.
    pub trigger_requested: bool,
    /// Whether the staged navigation sequence owns the presentation surface.
    pub sequence_active: bool,
    /// Whether navigation is leaving the ship-view presentation.
    pub exit_pending: bool,
    /// Whether the depth door is currently opening.
    pub depth_opening: bool,
    /// Presentation defer latch that blocks idle exit arming.
    pub presentation_deferred: bool,
    /// Whether another presentation currently blocks frame advancement.
    pub presentation_active: bool,
    /// Current typed navigation target.
    pub current_target: RecordId,
    /// Counter selected by the current record's redirect policy.
    pub access_counter: ShipNavigationAccessCounter,
    /// Presentation actor state restored when a trigger begins.
    pub presentation_actor_state: u16,
    /// Previous presentation actor state restored by the trigger.
    pub previous_presentation_actor_state: u16,
    /// Candidate published as the deferred C4 navigation action.
    pub deferred_navigation_record: Option<RecordId>,
    /// Shared bridge UI flags.
    pub ui_state: u16,
    /// Current target-list interpolation step.
    pub transition_step: u16,
    /// Target-list interpolation duration.
    pub transition_total_steps: u16,
    /// Measured open target for the navigation trigger list.
    pub choice_target_rect: ChoiceListRect,
    /// Whether a previously loaded scene image remains reusable.
    pub scene_image_cached: bool,
    /// Top row used for the staged navigation resource.
    pub resource_vertical_offset: u16,
    /// Whether a text menu is pending.
    pub text_menu_pending: bool,
    /// Current text-menu selection.
    pub text_selection: Option<usize>,
    /// Whether the depth door is closing over the staged scene.
    pub depth_closing: bool,
    /// Current depth movement step.
    pub depth_step: u8,
    /// Whether the staged frame has been presented at least once.
    pub frame_presented: bool,
    /// Whether the navigation image's decoded palette is staged.
    pub navigation_palette_staged: bool,
    /// Bridge steering destination restored during teardown.
    pub bridge_seek_target_arc: u16,
    /// Initial bridge steering distance restored during teardown.
    pub bridge_seek_initial_distance: u16,
    /// Whether the bridge navigation screen must be rebuilt.
    pub navigation_screen_rebuild_pending: bool,
    /// Whether bridge state must be snapshotted after navigation.
    pub navigation_snapshot_pending: bool,
    /// Top-level ship presentation flags.
    pub ship_active_flags: u16,
    /// Current scripted presentation line.
    pub active_line: Option<u16>,
    /// C2 presentation gate.
    pub presentation_gate: u16,
    /// Whether the ship HUD remains initialized.
    pub hud_initialized: bool,
    /// Whether subtitle text is still active.
    pub text_display_active: bool,
    /// Dialogue hold-completion latch.
    pub presentation_hold_ready: bool,
    /// Whether the ship depth-band crop is enabled.
    pub depth_band_enabled: bool,
    /// Packed request flags retained by other presentation owners.
    pub presentation_request_flags: u8,
    /// Dialogue word-choice lifecycle reset during teardown.
    pub word_choice_phase: PresentationWordChoicePhase,
    /// Whether bridge-panorama-to-black palette data has been prepared.
    pub bridge_palette_transition_staged: bool,
    /// Last palette index affected by the teardown transition.
    pub palette_transition_last: u8,
    /// Current palette transition percentage.
    pub palette_transition_percent: u16,
    /// Per-frame palette transition increment.
    pub palette_transition_increment: u16,
}

/// Ordered renderer, resource, and interaction work used by navigation.
pub trait ShipNavigationHost<RecordId> {
    /// Build typed candidates below the current target.
    fn build_navigation_candidates(
        &mut self,
        current_target: &RecordId,
    ) -> Vec<ShipNavigationCandidate<RecordId>>;
    /// Load the accepted candidate's description record.
    fn load_candidate_description(&mut self, candidate: &RecordId);
    /// Measure the trigger list and return its current rectangle.
    fn measure_navigation_trigger_list(&mut self) -> ChoiceListRect;
    /// Clear the original navigation display band.
    fn clear_navigation_band(&mut self);
    /// Decode the navigation background into the modern render surface.
    fn load_navigation_background(&mut self);
    /// Build the darkening remap used by the staged navigation frame.
    fn build_navigation_palette_remap(&mut self);
    /// Advance the alien overlay cycle before bridge steering.
    fn run_alien_overlay_cycle(&mut self);
    /// Update bridge steering for the current frame.
    fn update_bridge_steering(&mut self);
    /// Present the staged navigation frame.
    fn present_navigation_frame(&mut self);
    /// Advance the list-opening transition and report completion.
    fn advance_navigation_list_transition(&mut self) -> bool;
    /// Update the open trigger list and report whether a row was selected.
    fn navigation_trigger_selected(&mut self) -> bool;
    /// Clear the bridge display during final teardown.
    fn clear_bridge_display(&mut self);
    /// Clear the scene palette during final teardown.
    fn clear_scene_palette(&mut self);
    /// Recreate the bridge back buffer after navigation.
    fn initialize_bridge_back_buffer(&mut self);
    /// Capture HUD palette colors and reset the 3D camera.
    fn snapshot_hud_palette_and_reset_camera(&mut self);
}

/// Terminal path selected by one navigation coordinator update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipNavigationOutcome {
    /// Idle state armed the opening depth transition and exit latch.
    ExitArmed,
    /// Idle state was blocked by the presentation defer latch.
    Deferred,
    /// Another presentation owns the frame after overlay and steering updates.
    PresentationBlocked,
    /// A non-list sequence frame was presented.
    FramePresented,
    /// List interpolation has not completed.
    Transitioning,
    /// The open trigger list has no selection.
    AwaitingSelection,
    /// A trigger-list selection ended the sequence and armed exit.
    SelectionAccepted,
    /// Closed exit state restored the bridge and palette transition.
    ResetToBridge,
}

/// Run native BLOODPRG ship-navigation coordinator `0x00B34E`.
///
/// Typed record identities replace the native record heap and scratch offset
/// list. Resource decoding, palette work, alien animation, steering, and list
/// interaction remain explicit host operations in their recovered call order.
pub fn update_ship_navigation<RecordId, Host>(
    state: &mut ShipNavigationState<RecordId>,
    context: &ShipNavigationContext<RecordId>,
    host: &mut Host,
) -> ShipNavigationOutcome
where
    RecordId: Clone + Eq,
    Host: ShipNavigationHost<RecordId>,
{
    if state.trigger_requested {
        begin_navigation(state, context, host);
    }

    if state.exit_pending {
        if !state.depth_opening {
            reset_to_bridge(state, host);
            return ShipNavigationOutcome::ResetToBridge;
        }
    } else if !state.sequence_active {
        if state.presentation_deferred {
            return ShipNavigationOutcome::Deferred;
        }
        state.exit_pending = true;
        state.depth_opening = true;
        return ShipNavigationOutcome::ExitArmed;
    }

    host.run_alien_overlay_cycle();
    host.update_bridge_steering();
    if state.presentation_active {
        return ShipNavigationOutcome::PresentationBlocked;
    }

    host.present_navigation_frame();
    state.frame_presented = true;
    if state.transition_total_steps != NAVIGATION_LIST_TRANSITION_STEPS {
        return ShipNavigationOutcome::FramePresented;
    }
    if !host.advance_navigation_list_transition() {
        return ShipNavigationOutcome::Transitioning;
    }
    if !host.navigation_trigger_selected() {
        return ShipNavigationOutcome::AwaitingSelection;
    }

    state.sequence_active = false;
    state.exit_pending = true;
    ShipNavigationOutcome::SelectionAccepted
}

fn begin_navigation<RecordId, Host>(
    state: &mut ShipNavigationState<RecordId>,
    context: &ShipNavigationContext<RecordId>,
    host: &mut Host,
) where
    RecordId: Clone + Eq,
    Host: ShipNavigationHost<RecordId>,
{
    state.presentation_actor_state = state.previous_presentation_actor_state;
    state.access_counter.increment();
    let candidates = host.build_navigation_candidates(&state.current_target);
    let accepted = first_accepted_candidate(
        &state.current_target,
        &context.ark,
        context.unrestricted_candidates,
        &candidates,
    );

    if let Some(candidate) = accepted {
        state.deferred_navigation_record = Some(candidate.clone());
        host.load_candidate_description(candidate);
    } else {
        state.deferred_navigation_record = None;
        state.ui_state |= NAVIGATION_LIST_UI_FLAG;
        state.transition_step = u16::MIN;
        state.transition_total_steps = NAVIGATION_LIST_TRANSITION_STEPS;
        let measured = host.measure_navigation_trigger_list();
        state.choice_target_rect.origin[0] = measured.origin[0];
        state.choice_target_rect.size[0] = measured.size[0];
    }

    state.trigger_requested = false;
    state.sequence_active = true;
    state.resource_vertical_offset = NAVIGATION_SCENE_TOP_ROW;
    state.scene_image_cached = false;
    host.clear_navigation_band();
    host.load_navigation_background();
    state.navigation_palette_staged = true;
    state.text_menu_pending = false;
    state.text_selection = None;
    state.depth_closing = true;
    state.depth_step = DEPTH_CLOSING_STEP;
    host.build_navigation_palette_remap();
}

fn first_accepted_candidate<'a, RecordId: Eq>(
    current_target: &RecordId,
    ark: &RecordId,
    unrestricted: bool,
    candidates: &'a [ShipNavigationCandidate<RecordId>],
) -> Option<&'a RecordId> {
    for candidate in candidates {
        if !unrestricted && candidate.relation != ShipNavigationRelation::RecordDirectoryRoot {
            continue;
        }
        if ark != current_target
            && matches!(
                &candidate.relation,
                ShipNavigationRelation::Object(object) if object == ark
            )
        {
            break;
        }
        return Some(&candidate.record);
    }
    None
}

fn reset_to_bridge<RecordId, Host: ShipNavigationHost<RecordId>>(
    state: &mut ShipNavigationState<RecordId>,
    host: &mut Host,
) {
    host.clear_bridge_display();
    host.clear_scene_palette();
    state.ui_state = RESET_UI_STATE;
    state.bridge_seek_target_arc = u16::MIN;
    state.bridge_seek_initial_distance = RESET_BRIDGE_SEEK_DISTANCE;
    state.navigation_screen_rebuild_pending = true;
    state.navigation_snapshot_pending = true;
    state.ship_active_flags = u16::MIN;
    state.resource_vertical_offset = u16::MIN;
    state.text_selection = None;
    state.active_line = None;
    state.presentation_gate = u16::MIN;
    state.exit_pending = false;
    state.hud_initialized = false;
    state.text_display_active = false;
    state.presentation_deferred = false;
    state.presentation_hold_ready = false;
    state.depth_band_enabled = false;
    state.sequence_active = false;
    state.presentation_request_flags &= !PRESENTATION_REQUEST_LOW_BITS;
    state.word_choice_phase = PresentationWordChoicePhase::Closed;
    host.initialize_bridge_back_buffer();
    host.snapshot_hud_palette_and_reset_camera();
    state.bridge_palette_transition_staged = true;
    state.palette_transition_last = FULL_PALETTE_LAST_INDEX;
    state.palette_transition_percent = u16::MIN;
    state.palette_transition_increment = PALETTE_TRANSITION_INCREMENT;
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;

    const CURRENT_TARGET: u16 = 4_608;
    const ARK_TARGET: u16 = 17_476;
    const RECORD_BASE_RELATION: u16 = 256;
    const INITIAL_DIRECT_ACCESS_COUNT: u16 = 0;
    const INITIAL_REDIRECTED_ACCESS_COUNT: u16 = 32_766;
    const MEASURED_LIST_X: i16 = 6_699;
    const MEASURED_LIST_WIDTH: u16 = 14_923;

    #[derive(Clone, Deserialize)]
    struct NavigationVector {
        name: String,
        trigger: u8,
        sequence: u8,
        exit_pending: u8,
        opening: u8,
        defer: u8,
        presentation_active: u8,
        duration: u16,
        interpolation_complete: u8,
        layout_result: u16,
        candidate: u16,
        candidate_relation: u16,
        record_base_offset: u16,
        filter_flags: u8,
        calls: Vec<CallVector>,
    }

    #[derive(Clone, Deserialize)]
    struct CallVector {
        name: String,
    }

    struct OracleHost {
        calls: VecDeque<String>,
        candidates: Vec<ShipNavigationCandidate<u16>>,
        interpolation_complete: bool,
        layout_selected: bool,
    }

    impl OracleHost {
        fn expect(&mut self, name: &str) {
            assert_eq!(self.calls.pop_front().as_deref(), Some(name));
        }
    }

    impl ShipNavigationHost<u16> for OracleHost {
        fn build_navigation_candidates(
            &mut self,
            current_target: &u16,
        ) -> Vec<ShipNavigationCandidate<u16>> {
            self.expect("candidate_build");
            assert_eq!(*current_target, CURRENT_TARGET);
            self.candidates.clone()
        }

        fn load_candidate_description(&mut self, _candidate: &u16) {
            self.expect("c2");
        }

        fn measure_navigation_trigger_list(&mut self) -> ChoiceListRect {
            self.expect("layout");
            ChoiceListRect {
                origin: [MEASURED_LIST_X, 51],
                size: [MEASURED_LIST_WIDTH, 68],
            }
        }

        fn clear_navigation_band(&mut self) {
            self.expect("back_fill");
        }

        fn load_navigation_background(&mut self) {
            self.expect("pbm");
        }

        fn build_navigation_palette_remap(&mut self) {
            self.expect("palette");
        }

        fn run_alien_overlay_cycle(&mut self) {
            self.expect("alien");
        }

        fn update_bridge_steering(&mut self) {
            self.expect("bridge");
        }

        fn present_navigation_frame(&mut self) {
            self.expect("fullscreen");
        }

        fn advance_navigation_list_transition(&mut self) -> bool {
            self.expect("interpolate");
            self.interpolation_complete
        }

        fn navigation_trigger_selected(&mut self) -> bool {
            self.expect("layout");
            self.layout_selected
        }

        fn clear_bridge_display(&mut self) {
            self.expect("display_fill");
        }

        fn clear_scene_palette(&mut self) {
            self.expect("palette_clear");
        }

        fn initialize_bridge_back_buffer(&mut self) {
            self.expect("back_init");
        }

        fn snapshot_hud_palette_and_reset_camera(&mut self) {
            self.expect("vm_stop");
        }
    }

    #[test]
    fn navigation_coordinator_matches_every_original_vector() {
        let vectors: Vec<NavigationVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b34e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 15);

        for vector in vectors {
            assert_eq!(vector.record_base_offset, RECORD_BASE_RELATION);
            let redirected = vector.name == "trigger_redirects_access_counter";
            let relation = relation_for_vector(&vector);
            let candidates = (vector.candidate != u16::MIN)
                .then_some(ShipNavigationCandidate {
                    record: vector.candidate,
                    relation,
                })
                .into_iter()
                .collect();
            let calls = vector.calls.iter().map(|call| call.name.clone()).collect();
            let mut host = OracleHost {
                calls,
                candidates,
                interpolation_complete: vector.interpolation_complete != u8::MIN,
                layout_selected: vector.layout_result < i16::MAX as u16 + 1,
            };
            let mut state = initial_state(&vector, redirected);
            let initial_choice_target = state.choice_target_rect;
            let context = ShipNavigationContext {
                ark: ARK_TARGET,
                unrestricted_candidates: vector.filter_flags & 2 != u8::MIN,
            };

            let outcome = update_ship_navigation(&mut state, &context, &mut host);

            assert!(host.calls.is_empty(), "{}", vector.name);
            assert_eq!(outcome, expected_outcome(&vector.name), "{}", vector.name);
            assert_vector_state(&vector, &state, redirected, initial_choice_target, outcome);
        }
    }

    fn initial_state(vector: &NavigationVector, redirected: bool) -> ShipNavigationState<u16> {
        ShipNavigationState {
            trigger_requested: vector.trigger != u8::MIN,
            sequence_active: vector.sequence != u8::MIN,
            exit_pending: vector.exit_pending != u8::MIN,
            depth_opening: vector.opening != u8::MIN,
            presentation_deferred: vector.defer != u8::MIN,
            presentation_active: vector.presentation_active != u8::MIN,
            current_target: CURRENT_TARGET,
            access_counter: if redirected {
                ShipNavigationAccessCounter::Redirected(INITIAL_REDIRECTED_ACCESS_COUNT)
            } else {
                ShipNavigationAccessCounter::Direct(INITIAL_DIRECT_ACCESS_COUNT)
            },
            presentation_actor_state: 41_526,
            previous_presentation_actor_state: 41_526,
            deferred_navigation_record: None,
            ui_state: 37_779,
            transition_step: 155,
            transition_total_steps: vector.duration,
            choice_target_rect: ChoiceListRect {
                origin: [19_789, 20_561],
                size: [20_817, 21_585],
            },
            scene_image_cached: true,
            resource_vertical_offset: 41_895,
            text_menu_pending: true,
            text_selection: Some(43_947),
            depth_closing: false,
            depth_step: 49,
            frame_presented: false,
            navigation_palette_staged: false,
            bridge_seek_target_arc: 39_067,
            bridge_seek_initial_distance: 40_445,
            navigation_screen_rebuild_pending: false,
            navigation_snapshot_pending: false,
            ship_active_flags: 62_451,
            active_line: Some(34_952),
            presentation_gate: 65_535,
            hud_initialized: true,
            text_display_active: true,
            presentation_hold_ready: true,
            depth_band_enabled: true,
            presentation_request_flags: 171,
            word_choice_phase: PresentationWordChoicePhase::Closing,
            bridge_palette_transition_staged: false,
            palette_transition_last: 82,
            palette_transition_percent: 20_303,
            palette_transition_increment: 19_789,
        }
    }

    fn relation_for_vector(vector: &NavigationVector) -> ShipNavigationRelation<u16> {
        if vector.candidate_relation == vector.record_base_offset {
            ShipNavigationRelation::RecordDirectoryRoot
        } else if vector.candidate_relation == ARK_TARGET {
            ShipNavigationRelation::Object(ARK_TARGET)
        } else if vector.candidate_relation == CURRENT_TARGET {
            ShipNavigationRelation::Object(CURRENT_TARGET)
        } else {
            ShipNavigationRelation::Other
        }
    }

    fn expected_outcome(name: &str) -> ShipNavigationOutcome {
        match name {
            "idle_arms_opening_and_exit" => ShipNavigationOutcome::ExitArmed,
            "idle_defer_gate_blocks_opening" => ShipNavigationOutcome::Deferred,
            "active_sequence_blocked_before_frame_copy"
            | "exit_while_opening_reenters_active_sequence"
            | "trigger_accepts_unrestricted_candidate"
            | "trigger_opens_list_when_no_candidate_exists"
            | "trigger_accepts_candidate_related_to_record_base"
            | "trigger_rejects_candidate_related_only_to_current"
            | "trigger_ark_relation_opens_target_list"
            | "trigger_redirects_access_counter" => ShipNavigationOutcome::PresentationBlocked,
            "active_sequence_nonlayout_duration_copies_frame" => {
                ShipNavigationOutcome::FramePresented
            }
            "active_sequence_waits_for_interpolation" => ShipNavigationOutcome::Transitioning,
            "completed_interpolation_negative_query_keeps_sequence" => {
                ShipNavigationOutcome::AwaitingSelection
            }
            "completed_interpolation_selection_arms_exit" => {
                ShipNavigationOutcome::SelectionAccepted
            }
            "closed_exit_runs_final_reset" => ShipNavigationOutcome::ResetToBridge,
            _ => panic!("unknown navigation oracle case {name}"),
        }
    }

    fn assert_vector_state(
        vector: &NavigationVector,
        state: &ShipNavigationState<u16>,
        redirected: bool,
        initial_choice_target: ChoiceListRect,
        outcome: ShipNavigationOutcome,
    ) {
        if vector.trigger != u8::MIN {
            assert!(!state.trigger_requested, "{}", vector.name);
            assert_eq!(state.presentation_actor_state, 41_526, "{}", vector.name);
            assert_eq!(
                state.access_counter,
                if redirected {
                    ShipNavigationAccessCounter::Redirected(INITIAL_REDIRECTED_ACCESS_COUNT + 1)
                } else {
                    ShipNavigationAccessCounter::Direct(INITIAL_DIRECT_ACCESS_COUNT + 1)
                },
                "{}",
                vector.name
            );
            assert!(state.sequence_active, "{}", vector.name);
            assert_eq!(
                state.resource_vertical_offset, NAVIGATION_SCENE_TOP_ROW,
                "{}",
                vector.name
            );
            assert!(!state.scene_image_cached, "{}", vector.name);
            assert!(!state.text_menu_pending, "{}", vector.name);
            assert_eq!(state.text_selection, None, "{}", vector.name);
            assert!(state.depth_closing, "{}", vector.name);
            assert_eq!(state.depth_step, DEPTH_CLOSING_STEP, "{}", vector.name);
            assert!(state.navigation_palette_staged, "{}", vector.name);

            let accepted = vector.candidate != u16::MIN
                && (vector.filter_flags & 2 != u8::MIN
                    || vector.candidate_relation == vector.record_base_offset)
                && vector.candidate_relation != ARK_TARGET;
            assert_eq!(
                state.deferred_navigation_record,
                accepted.then_some(vector.candidate),
                "{}",
                vector.name
            );
            if accepted {
                assert_eq!(
                    state.choice_target_rect, initial_choice_target,
                    "{}",
                    vector.name
                );
            } else {
                assert_ne!(state.ui_state & NAVIGATION_LIST_UI_FLAG, u16::MIN);
                assert_eq!(state.transition_step, u16::MIN, "{}", vector.name);
                assert_eq!(
                    state.transition_total_steps, NAVIGATION_LIST_TRANSITION_STEPS,
                    "{}",
                    vector.name
                );
                assert_eq!(state.choice_target_rect.origin[0], MEASURED_LIST_X);
                assert_eq!(state.choice_target_rect.size[0], MEASURED_LIST_WIDTH);
                assert_eq!(
                    state.choice_target_rect.origin[1],
                    initial_choice_target.origin[1]
                );
                assert_eq!(
                    state.choice_target_rect.size[1],
                    initial_choice_target.size[1]
                );
            }
        }

        match outcome {
            ShipNavigationOutcome::ExitArmed => {
                assert!(state.exit_pending);
                assert!(state.depth_opening);
            }
            ShipNavigationOutcome::SelectionAccepted => {
                assert!(!state.sequence_active);
                assert!(state.exit_pending);
                assert!(state.frame_presented);
            }
            ShipNavigationOutcome::ResetToBridge => assert_bridge_reset(state),
            ShipNavigationOutcome::FramePresented
            | ShipNavigationOutcome::Transitioning
            | ShipNavigationOutcome::AwaitingSelection => assert!(state.frame_presented),
            ShipNavigationOutcome::Deferred | ShipNavigationOutcome::PresentationBlocked => {}
        }
    }

    fn assert_bridge_reset(state: &ShipNavigationState<u16>) {
        assert_eq!(state.ui_state, RESET_UI_STATE);
        assert_eq!(state.bridge_seek_target_arc, u16::MIN);
        assert_eq!(
            state.bridge_seek_initial_distance,
            RESET_BRIDGE_SEEK_DISTANCE
        );
        assert!(state.navigation_screen_rebuild_pending);
        assert!(state.navigation_snapshot_pending);
        assert_eq!(state.ship_active_flags, u16::MIN);
        assert_eq!(state.resource_vertical_offset, u16::MIN);
        assert_eq!(state.text_selection, None);
        assert_eq!(state.active_line, None);
        assert_eq!(state.presentation_gate, u16::MIN);
        assert!(!state.exit_pending);
        assert!(!state.hud_initialized);
        assert!(!state.text_display_active);
        assert!(!state.presentation_deferred);
        assert!(!state.presentation_hold_ready);
        assert!(!state.depth_band_enabled);
        assert!(!state.sequence_active);
        assert_eq!(state.presentation_request_flags, 168);
        assert_eq!(state.word_choice_phase, PresentationWordChoicePhase::Closed);
        assert!(state.bridge_palette_transition_staged);
        assert_eq!(state.palette_transition_last, FULL_PALETTE_LAST_INDEX);
        assert_eq!(state.palette_transition_percent, u16::MIN);
        assert_eq!(
            state.palette_transition_increment,
            PALETTE_TRANSITION_INCREMENT
        );
    }
}
