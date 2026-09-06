//! Hyperjump navigation presentation actor state machine.

use super::{
    CAMERA_VIEW_TRANSITION_STEPS, PresentationLine, PresentationLineFlags, PresentationLineOutcome,
    PresentationLinePlayback, PresentationLineStepper, PresentationResourceId,
};

/// Idle hyperjump presentation resource authored as `bhyper.spr`.
pub const HYPERJUMP_IDLE_PRESENTATION_RESOURCE: PresentationResourceId =
    PresentationResourceId::new(18);

/// Hyperjump transition resource authored as `aphyper.spr`.
pub const HYPERJUMP_TRANSITION_PRESENTATION_RESOURCE: PresentationResourceId =
    PresentationResourceId::new(20);

/// Presentation-line frame that starts the camera transition countdown.
const CAMERA_TRANSITION_LINE_FRAME: u16 = 1;

/// Live location-panel state consulted at distinct actor gates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct HyperjumpLocationPanelState {
    /// The visible location panel owns completed-line presentation.
    pub active: bool,
    /// A panel transition or visible panel enables deferred playback.
    pub blocks_playback: bool,
}

/// Deferred action published when the hyperjump entry reaches its terminal state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyperjumpDeferredAction {
    /// This handler has not replaced the existing deferred action.
    #[default]
    Unchanged,
    /// Process the pending navigation request.
    Navigate,
}

/// Presentation state published while the first hyperjump line plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum HyperjumpActorPresentation {
    /// This handler has not selected a presentation mode.
    #[default]
    Unchanged,
    /// The hyperjump entry presentation is active.
    Entry,
}

/// Mutable state owned by presentation actor handler 0.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HyperjumpPresentationActorState<RecordLink> {
    /// Record awaiting navigation processing.
    pub deferred_record: Option<RecordLink>,
    /// Semantic deferred action kind.
    pub deferred_action: HyperjumpDeferredAction,
    /// The target presentation latch was cleared before first-pass playback.
    pub target_presentation_cleared: bool,
    /// Presentation mode published by this actor.
    pub presentation: HyperjumpActorPresentation,
    /// A completed first pass requested the navigation transition.
    pub transition_pending: bool,
}

impl<RecordLink> Default for HyperjumpPresentationActorState<RecordLink> {
    fn default() -> Self {
        Self {
            deferred_record: None,
            deferred_action: HyperjumpDeferredAction::default(),
            target_presentation_cleared: false,
            presentation: HyperjumpActorPresentation::default(),
            transition_pending: false,
        }
    }
}

/// Dynamic panel, camera, line, entity, and audio services used by this actor.
pub trait HyperjumpPresentationActorBackend: PresentationLineStepper {
    /// Sequel travel option and whether its deferred link names Arche itself.
    fn sequel_travel_control(&self) -> Option<(bool, bool)> {
        None
    }

    /// Clear the current selector and request the hyperjump hand animation.
    fn restart_hyperjump_hand_animation(&mut self);

    /// Return the current location-panel ownership and playback gates.
    fn location_panel_state(&self) -> HyperjumpLocationPanelState;

    /// Return whether the camera transition countdown is active.
    fn camera_transition_active(&self) -> bool;

    /// Start the camera transition countdown.
    fn start_camera_transition(&mut self, steps: u8);

    /// Close the location panel and clear its transition gate.
    fn close_location_panel(&mut self);

    /// Mark the location-panel entity for a state transition.
    fn mark_location_panel_entity_dirty(&mut self);

    /// Mark the shared presentation entity for a state transition.
    fn mark_presentation_entity_dirty(&mut self);

    /// Play the hyperjump transition clip.
    fn play_hyperjump_transition_clip(&mut self);
}

/// Terminal path taken by one hyperjump actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HyperjumpPresentationActorOutcome {
    /// Hyperjump presentation mode is disabled.
    Disabled,
    /// Actor 0 is already busy.
    Busy,
    /// The visible location panel retains the present line.
    LocationPanelActive,
    /// No deferred navigation record is available.
    NoDeferredRecord,
    /// Neither reverse playback nor panel transition enables the second pass.
    Inactive,
    /// First- or second-pass line playback remains active.
    Presenting,
    /// The first pass reached its terminal navigation state.
    NavigationQueued,
    /// The sequel consumed the C1 kind and returned directly to the bridge.
    BridgeReset,
    /// Second-pass completion cleared the consumed deferred record.
    Completed,
    /// Second-pass completion returned to panel-owned idle presentation.
    CompletedForLocationPanel,
}

/// Update presentation actor 0 and coordinate the hyperjump navigation line.
///
/// This translates `nav_actor_handler_0` at BLOODPRG routine offset `0x007F9C`.
/// Semantic panel gates deliberately distinguish visible ownership from a
/// broader in-progress transition, retaining the original two decisions
/// without preserving their packed byte. Named resources, optional record
/// ownership, and backend camera state replace numeric tags and shared memory.
pub fn update_hyperjump_presentation_actor<
    RecordLink,
    Backend: HyperjumpPresentationActorBackend,
>(
    enabled: bool,
    actor_busy: bool,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    state: &mut HyperjumpPresentationActorState<RecordLink>,
    backend: &mut Backend,
) -> Result<HyperjumpPresentationActorOutcome, Backend::Error> {
    if !enabled {
        return Ok(HyperjumpPresentationActorOutcome::Disabled);
    }
    if actor_busy {
        return Ok(HyperjumpPresentationActorOutcome::Busy);
    }

    let original_flags = line.flags;
    let mut second_pass_prepared = original_flags.resource_loaded;
    if original_flags.present {
        if original_flags.ready {
            state.target_presentation_cleared = true;
            state.presentation = HyperjumpActorPresentation::Entry;
            backend.restart_hyperjump_hand_animation();
            backend.mark_location_panel_entity_dirty();
            backend.mark_presentation_entity_dirty();
            let _first_pass_outcome = backend.update_line(line, line_playback)?;
            second_pass_prepared = true;

            if line.frame == CAMERA_TRANSITION_LINE_FRAME {
                if let Some((true, same_target)) = backend.sequel_travel_control() {
                    state.deferred_action = HyperjumpDeferredAction::Navigate;
                    if same_target {
                        state.deferred_action = HyperjumpDeferredAction::Unchanged;
                        state.transition_pending = false;
                        backend.mark_presentation_entity_dirty();
                        backend.close_location_panel();
                        line_playback.redraw_requested = true;
                        return Ok(HyperjumpPresentationActorOutcome::BridgeReset);
                    }
                }
                backend.start_camera_transition(CAMERA_VIEW_TRANSITION_STEPS);
            } else if !backend.camera_transition_active() {
                line.flags = completed_transition();
                state.deferred_action = HyperjumpDeferredAction::Navigate;
                state.transition_pending = true;
                backend.mark_presentation_entity_dirty();
                backend.close_location_panel();
                line_playback.redraw_requested = true;
                if backend
                    .sequel_travel_control()
                    .is_some_and(|(enabled, _)| enabled)
                {
                    state.deferred_action = HyperjumpDeferredAction::Unchanged;
                    state.transition_pending = false;
                    return Ok(HyperjumpPresentationActorOutcome::BridgeReset);
                }
                return Ok(HyperjumpPresentationActorOutcome::NavigationQueued);
            }
        }

        if backend.location_panel_state().active {
            return Ok(HyperjumpPresentationActorOutcome::LocationPanelActive);
        }
        line.resource = HYPERJUMP_TRANSITION_PRESENTATION_RESOURCE;
        line_playback.reverse = true;
        line.flags = empty_line();
        line_playback.redraw_requested = true;
    }

    if state.deferred_record.is_none() {
        return Ok(HyperjumpPresentationActorOutcome::NoDeferredRecord);
    }
    if !line_playback.reverse && !backend.location_panel_state().blocks_playback {
        return Ok(HyperjumpPresentationActorOutcome::Inactive);
    }

    if !second_pass_prepared {
        backend.mark_presentation_entity_dirty();
        line.resource = HYPERJUMP_TRANSITION_PRESENTATION_RESOURCE;
        backend.play_hyperjump_transition_clip();
    }

    if backend.update_line(line, line_playback)? != PresentationLineOutcome::Completed {
        return Ok(HyperjumpPresentationActorOutcome::Presenting);
    }
    if !backend.location_panel_state().active {
        state.deferred_record = None;
        backend.mark_presentation_entity_dirty();
        line.flags = empty_line();
        Ok(HyperjumpPresentationActorOutcome::Completed)
    } else {
        line.flags = present_only();
        line_playback.redraw_requested = true;
        line.resource = HYPERJUMP_IDLE_PRESENTATION_RESOURCE;
        Ok(HyperjumpPresentationActorOutcome::CompletedForLocationPanel)
    }
}

const fn empty_line() -> PresentationLineFlags {
    PresentationLineFlags {
        present: false,
        transition_latched: false,
        resource_loaded: false,
        ready: false,
    }
}

const fn present_only() -> PresentationLineFlags {
    PresentationLineFlags {
        present: true,
        ..empty_line()
    }
}

const fn completed_transition() -> PresentationLineFlags {
    PresentationLineFlags {
        present: true,
        transition_latched: true,
        resource_loaded: true,
        ready: false,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 24;
    const TEST_DEFERRED_RECORD: u16 = 4_660;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_before: u8,
        ui_after: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        line_resource_before: u16,
        line_resource_after: u16,
        frame_before: u16,
        frame_after: u16,
        view_state_after: u8,
        deferred_type_after: u16,
        deferred_link_after: u16,
        #[serde(default)]
        helper_results: Vec<bool>,
        #[serde(default)]
        call_sequence: Vec<String>,
    }

    struct OracleBackend {
        sequel: Option<(bool, bool)>,
        name: String,
        helper_results: Vec<bool>,
        helper_index: usize,
        panel: HyperjumpLocationPanelState,
        camera_steps_remaining: Option<u8>,
        hand_animation_restarted: bool,
        calls: Vec<String>,
    }

    impl PresentationLineStepper for OracleBackend {
        type Error = std::convert::Infallible;

        fn update_line(
            &mut self,
            line: &mut PresentationLine,
            _playback: &mut PresentationLinePlayback,
        ) -> Result<PresentationLineOutcome, Self::Error> {
            self.calls.push(String::from("presentation_line_helper"));
            let result = self.helper_results[self.helper_index];
            if self.helper_index == usize::MIN {
                match self.name.as_str() {
                    "present_ready_helper_sets_frame_one" => {
                        line.frame = CAMERA_TRANSITION_LINE_FRAME;
                    }
                    "present_ready_helper_clears_view_terminal" => {
                        self.camera_steps_remaining = None;
                    }
                    "present_not_ready_helper_arms_panel" | "absent_unloaded_helper_arms_panel" => {
                        self.panel = HyperjumpLocationPanelState {
                            active: true,
                            blocks_playback: true,
                        };
                    }
                    _ => {}
                }
            }
            self.helper_index += 1;
            Ok(if result {
                PresentationLineOutcome::Completed
            } else {
                PresentationLineOutcome::Advanced
            })
        }
    }

    impl HyperjumpPresentationActorBackend for OracleBackend {
        fn sequel_travel_control(&self) -> Option<(bool, bool)> {
            self.sequel
        }

        fn restart_hyperjump_hand_animation(&mut self) {
            self.hand_animation_restarted = true;
        }

        fn location_panel_state(&self) -> HyperjumpLocationPanelState {
            self.panel
        }

        fn camera_transition_active(&self) -> bool {
            self.camera_steps_remaining.is_some()
        }

        fn start_camera_transition(&mut self, steps: u8) {
            self.camera_steps_remaining = Some(steps);
        }

        fn close_location_panel(&mut self) {
            self.panel = HyperjumpLocationPanelState::default();
        }

        fn mark_location_panel_entity_dirty(&mut self) {
            self.calls
                .push(String::from("entity_flag_state_transition"));
        }

        fn mark_presentation_entity_dirty(&mut self) {
            self.calls
                .push(String::from("entity_flag_state_transition"));
        }

        fn play_hyperjump_transition_clip(&mut self) {
            self.calls.push(String::from("snd_play_clip"));
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7f9c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let setup = CaseSetup::for_name(&vector.name, vector.deferred_link_after);
            let navigation_queued = vector.deferred_type_after == 193;
            let mut state = HyperjumpPresentationActorState {
                deferred_record: setup.deferred_record,
                deferred_action: HyperjumpDeferredAction::Unchanged,
                target_presentation_cleared: false,
                presentation: HyperjumpActorPresentation::Unchanged,
                transition_pending: false,
            };
            let mut line = PresentationLine {
                flags: decode_line_flags(vector.line_flags_before),
                resource: PresentationResourceId::new(vector.line_resource_before),
                terminal_frame: u16::MIN,
                frame: vector.frame_before,
                position: [u16::MIN; 2],
            };
            let mut line_playback = PresentationLinePlayback {
                busy: false,
                reverse: setup.reverse_playback,
                redraw_requested: vector.ui_before & 4 != u8::MIN,
            };
            let mut backend = OracleBackend {
                sequel: None,
                name: vector.name.clone(),
                helper_results: vector.helper_results.clone(),
                helper_index: usize::MIN,
                panel: setup.panel,
                camera_steps_remaining: setup.camera_steps_remaining,
                hand_animation_restarted: false,
                calls: Vec::new(),
            };

            update_hyperjump_presentation_actor(
                vector.ui_before & 16 != u8::MIN,
                setup.actor_busy,
                &mut line,
                &mut line_playback,
                &mut state,
                &mut backend,
            )
            .unwrap();

            assert_eq!(
                backend.hand_animation_restarted,
                state.presentation == HyperjumpActorPresentation::Entry,
                "{}",
                vector.name
            );

            assert_eq!(backend.calls, vector.call_sequence, "{}", vector.name);
            assert_eq!(
                backend.helper_index,
                vector.helper_results.len(),
                "{}",
                vector.name
            );
            assert_eq!(
                encode_line_flags(line.flags),
                vector.line_flags_after & 15,
                "{}",
                vector.name
            );
            assert_eq!(
                line.resource.get(),
                vector.line_resource_after,
                "{}",
                vector.name
            );
            assert_eq!(line.frame, vector.frame_after, "{}", vector.name);
            assert_eq!(
                backend.camera_steps_remaining.unwrap_or(u8::MIN),
                vector.view_state_after,
                "{}",
                vector.name
            );
            assert_eq!(
                line_playback.redraw_requested,
                vector.ui_after & 4 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_action == HyperjumpDeferredAction::Navigate,
                navigation_queued,
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_record,
                nonzero(vector.deferred_link_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.transition_pending, navigation_queued,
                "{}",
                vector.name
            );

            let first_pass_started =
                vector.line_flags_before & 9 == 9 && !vector.helper_results.is_empty();
            assert_eq!(
                state.target_presentation_cleared, first_pass_started,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation == HyperjumpActorPresentation::Entry,
                first_pass_started,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn sequel_travel_hyperjump_matches_original_option_branches() {
        let rows = include_str!(
            "../../../../../re/tools/oracle_vectors/big_bug_bang_travel_options.jsonl"
        );
        let mut count = 0;
        for row in rows
            .lines()
            .map(|line| serde_json::from_str::<serde_json::Value>(line).unwrap())
        {
            let first_frame = row["gate"] == "first_frame";
            if !first_frame && row["gate"] != "completion" {
                continue;
            }
            count += 1;
            let enabled = row["travel_flag"] == 1;
            let mut state = HyperjumpPresentationActorState {
                deferred_record: Some(TEST_DEFERRED_RECORD),
                ..Default::default()
            };
            let mut line = PresentationLine {
                flags: decode_line_flags(9),
                resource: HYPERJUMP_IDLE_PRESENTATION_RESOURCE,
                terminal_frame: 8,
                frame: if first_frame { 1 } else { 2 },
                position: [0; 2],
            };
            let mut playback = PresentationLinePlayback::default();
            let mut backend = OracleBackend {
                sequel: Some((enabled, row["same_target"] == true)),
                name: String::new(),
                helper_results: vec![false],
                helper_index: 0,
                panel: HyperjumpLocationPanelState {
                    active: true,
                    blocks_playback: true,
                },
                camera_steps_remaining: None,
                hand_animation_restarted: false,
                calls: Vec::new(),
            };
            let result = update_hyperjump_presentation_actor(
                true,
                false,
                &mut line,
                &mut playback,
                &mut state,
                &mut backend,
            )
            .unwrap();
            let reset = row["outcome"] == "reset_bridge";
            assert_eq!(
                result == HyperjumpPresentationActorOutcome::BridgeReset,
                reset,
                "{row}"
            );
            assert_eq!(
                state.deferred_record,
                Some(TEST_DEFERRED_RECORD),
                "reset retains the link"
            );
            if first_frame {
                assert_eq!(
                    backend.camera_steps_remaining,
                    if reset { None } else { Some(8) },
                    "{row}"
                );
                assert_eq!(
                    state.deferred_action == HyperjumpDeferredAction::Navigate,
                    enabled && !reset,
                    "{row}"
                );
                assert_eq!(
                    line.flags,
                    decode_line_flags(9),
                    "first frame preserves line flags"
                );
            } else {
                assert_eq!(state.transition_pending, !reset, "{row}");
                assert_eq!(
                    line.flags,
                    decode_line_flags(7),
                    "completion publishes flags before the gate"
                );
            }
        }
        assert_eq!(count, 6);
    }

    #[derive(Clone, Copy)]
    struct CaseSetup {
        actor_busy: bool,
        reverse_playback: bool,
        deferred_record: Option<u16>,
        panel: HyperjumpLocationPanelState,
        camera_steps_remaining: Option<u8>,
    }

    impl CaseSetup {
        fn for_name(name: &str, deferred_link_after: u16) -> Self {
            let deferred_before = match name {
                "present_not_ready_no_deferred"
                | "present_ready_helper_sets_frame_one"
                | "present_ready_existing_view_state"
                | "absent_no_deferred" => None,
                _ if deferred_link_after == u16::MIN => Some(TEST_DEFERRED_RECORD),
                _ => nonzero(deferred_link_after),
            };
            let mut panel = HyperjumpLocationPanelState::default();
            if matches!(
                name,
                "present_not_ready_panel_active"
                    | "present_ready_frame_one_panel_active"
                    | "absent_loaded_panel_complete"
            ) {
                panel = HyperjumpLocationPanelState {
                    active: true,
                    blocks_playback: true,
                };
            } else if name == "absent_panel_high_bit_opens_gate" {
                panel.blocks_playback = true;
            }

            Self {
                actor_busy: name == "actor_zero_busy_high_bit",
                reverse_playback: matches!(
                    name,
                    "absent_loaded_incomplete"
                        | "absent_loaded_complete"
                        | "absent_unloaded_incomplete"
                        | "absent_unloaded_complete"
                        | "absent_unloaded_helper_arms_panel"
                ),
                deferred_record: deferred_before,
                panel,
                camera_steps_remaining: if matches!(
                    name,
                    "present_ready_existing_view_state"
                        | "present_ready_helper_clears_view_terminal"
                ) {
                    Some(CAMERA_VIEW_TRANSITION_STEPS)
                } else {
                    None
                },
            }
        }
    }

    const fn nonzero(value: u16) -> Option<u16> {
        if value == u16::MIN { None } else { Some(value) }
    }

    const fn decode_line_flags(flags: u8) -> PresentationLineFlags {
        PresentationLineFlags {
            present: flags & 1 != u8::MIN,
            transition_latched: flags & 2 != u8::MIN,
            resource_loaded: flags & 4 != u8::MIN,
            ready: flags & 8 != u8::MIN,
        }
    }

    const fn encode_line_flags(flags: PresentationLineFlags) -> u8 {
        flags.present as u8
            | (flags.transition_latched as u8) << 1
            | (flags.resource_loaded as u8) << 2
            | (flags.ready as u8) << 3
    }
}
