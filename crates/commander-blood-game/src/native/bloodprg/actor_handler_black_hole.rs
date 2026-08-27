//! Black-hole travel presentation actor state machine.

use commander_blood_formats::script::ScriptObjectKind;

use super::{
    PresentationLine, PresentationLineFlags, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper, PresentationResourceId,
};

/// Idle black-hole presentation resource authored as `bpol.spr`.
pub const BLACK_HOLE_IDLE_PRESENTATION_RESOURCE: PresentationResourceId =
    PresentationResourceId::new(19);

/// Black-hole transition resource authored as `appol.spr`.
pub const BLACK_HOLE_TRANSITION_PRESENTATION_RESOURCE: PresentationResourceId =
    PresentationResourceId::new(21);

/// Current navigation target resolved through Arche's typed location relation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackHoleNavigationTarget<RecordLink> {
    /// Stable identity of the current navigation destination.
    pub record: RecordLink,
    /// Decoded object kind used by the exact black-hole gate.
    pub kind: ScriptObjectKind,
}

/// Live bridge owners that can force or terminate the second presentation pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BlackHolePresentationBlockers {
    /// The location information panel currently owns the presentation region.
    pub location_panel_active: bool,
    /// Camera presentation actor 5 currently owns the presentation region.
    pub camera_presentation_active: bool,
}

impl BlackHolePresentationBlockers {
    const fn any(self) -> bool {
        self.location_panel_active || self.camera_presentation_active
    }
}

/// Deferred action published after the black-hole entry line completes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlackHoleDeferredAction {
    /// This handler has not replaced the existing deferred action.
    #[default]
    Unchanged,
    /// Travel through the selected black hole.
    Travel,
}

/// Presentation state published during the first black-hole line pass.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum BlackHoleActorPresentation {
    /// This handler has not selected a presentation mode.
    #[default]
    Unchanged,
    /// The black-hole entry presentation is active.
    Entry,
}

/// Mutable state owned by presentation actor handler 1.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlackHolePresentationActorState<RecordLink> {
    /// Last exact black-hole target accepted from Arche's current location.
    pub target_record: Option<RecordLink>,
    /// Record staged for deferred travel processing.
    pub deferred_record: Option<RecordLink>,
    /// Semantic deferred action kind.
    pub deferred_action: BlackHoleDeferredAction,
    /// The target presentation latch was cleared before first-pass playback.
    pub target_presentation_cleared: bool,
    /// Presentation mode published by this actor.
    pub presentation: BlackHoleActorPresentation,
    /// The later travel-transition phase must restart from its beginning.
    pub transition_phase_reset: bool,
}

impl<RecordLink> Default for BlackHolePresentationActorState<RecordLink> {
    fn default() -> Self {
        Self {
            target_record: None,
            deferred_record: None,
            deferred_action: BlackHoleDeferredAction::default(),
            target_presentation_cleared: false,
            presentation: BlackHoleActorPresentation::default(),
            transition_phase_reset: false,
        }
    }
}

impl<RecordLink> BlackHolePresentationActorState<RecordLink> {
    /// Consume the target promoted to deferred C6 ownership by this actor.
    pub fn take_deferred_record(&mut self) -> Option<RecordLink> {
        self.deferred_record.take()
    }
}

/// Read-only bridge and navigation inputs for one actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BlackHolePresentationActorContext<'a, RecordLink> {
    /// Whether the bridge UI enables this actor.
    pub enabled: bool,
    /// Whether actor 1 is already busy.
    pub actor_busy: bool,
    /// Whether reverse playback or camera state enables an absent line.
    pub camera_state_enables_absent_line: bool,
    /// Arche's current typed navigation destination, when resolved.
    pub current_target: Option<&'a BlackHoleNavigationTarget<RecordLink>>,
}

/// Dynamic bridge state and presentation services used by the black-hole actor.
pub trait BlackHolePresentationActorBackend: PresentationLineStepper {
    /// Read the live blockers, including changes made by the previous line step.
    fn presentation_blockers(&self) -> BlackHolePresentationBlockers;

    /// Mark the shared presentation entity for a state transition.
    fn mark_presentation_entity_dirty(&mut self);

    /// Play the black-hole transition clip.
    fn play_black_hole_transition_clip(&mut self);
}

/// Terminal path taken by one black-hole actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlackHolePresentationActorOutcome {
    /// Black-hole presentation mode is disabled.
    Disabled,
    /// Actor 1 is already busy.
    Busy,
    /// A present line is not ready and no bridge owner requests retargeting.
    WaitingForLine,
    /// First- or second-pass line playback remains active.
    Presenting,
    /// Arche's current target is absent or is not a black hole.
    NoBlackHoleTarget,
    /// Neither reverse playback nor camera state enables an absent line.
    Inactive,
    /// Camera actor 5 blocks loading the transition resource.
    CameraPresentationBlocked,
    /// Second-pass completion was cleared because another bridge owner is active.
    CompletedWhileBlocked,
    /// Second-pass completion returned the line to its idle black-hole resource.
    Completed,
}

/// Update presentation actor 1 and stage a typed black-hole travel action.
///
/// This translates `nav_actor_handler_1` at BLOODPRG routine offset `0x007EC0`.
/// The original two-pass ordering is retained, including live blocker reads
/// after each callback. Typed object relationships, semantic deferred actions,
/// and named resources replace object offsets, numeric action tags, and packed
/// shared state; no source-address representation enters the runtime model.
pub fn update_black_hole_presentation_actor<
    RecordLink: Clone,
    Backend: BlackHolePresentationActorBackend,
>(
    context: BlackHolePresentationActorContext<'_, RecordLink>,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    state: &mut BlackHolePresentationActorState<RecordLink>,
    backend: &mut Backend,
) -> Result<BlackHolePresentationActorOutcome, Backend::Error> {
    if !context.enabled {
        return Ok(BlackHolePresentationActorOutcome::Disabled);
    }
    if context.actor_busy {
        return Ok(BlackHolePresentationActorOutcome::Busy);
    }

    let original_flags = line.flags;
    if original_flags.present {
        if original_flags.ready {
            state.target_presentation_cleared = true;
            state.presentation = BlackHoleActorPresentation::Entry;
            if backend.update_line(line, line_playback)? == PresentationLineOutcome::Completed {
                state.deferred_record = state.target_record.clone();
                state.deferred_action = BlackHoleDeferredAction::Travel;
                state.transition_phase_reset = true;
                line.flags = empty_line();
                retarget_transition_line(line, line_playback);
                return complete_second_pass(line, line_playback, state, backend);
            }
        }

        if !backend.presentation_blockers().any() {
            return Ok(if original_flags.ready {
                BlackHolePresentationActorOutcome::Presenting
            } else {
                BlackHolePresentationActorOutcome::WaitingForLine
            });
        }

        retarget_transition_line(line, line_playback);
        return complete_second_pass(line, line_playback, state, backend);
    }

    let Some(target) = context
        .current_target
        .filter(|target| target.kind == ScriptObjectKind::BlackHole)
    else {
        return Ok(BlackHolePresentationActorOutcome::NoBlackHoleTarget);
    };
    state.target_record = Some(target.record.clone());

    if !line_playback.reverse && !context.camera_state_enables_absent_line {
        return Ok(BlackHolePresentationActorOutcome::Inactive);
    }

    if !original_flags.resource_loaded {
        if backend.presentation_blockers().camera_presentation_active {
            return Ok(BlackHolePresentationActorOutcome::CameraPresentationBlocked);
        }
        backend.mark_presentation_entity_dirty();
        line.resource = BLACK_HOLE_TRANSITION_PRESENTATION_RESOURCE;
        backend.play_black_hole_transition_clip();
    }

    complete_second_pass(line, line_playback, state, backend)
}

fn retarget_transition_line(
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
) {
    line.resource = BLACK_HOLE_TRANSITION_PRESENTATION_RESOURCE;
    line_playback.reverse = true;
    line_playback.redraw_requested = true;
}

fn complete_second_pass<RecordLink, Backend: BlackHolePresentationActorBackend>(
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    _state: &mut BlackHolePresentationActorState<RecordLink>,
    backend: &mut Backend,
) -> Result<BlackHolePresentationActorOutcome, Backend::Error> {
    if backend.update_line(line, line_playback)? != PresentationLineOutcome::Completed {
        return Ok(BlackHolePresentationActorOutcome::Presenting);
    }

    if backend.presentation_blockers().any() {
        backend.mark_presentation_entity_dirty();
        line.flags = empty_line();
        Ok(BlackHolePresentationActorOutcome::CompletedWhileBlocked)
    } else {
        line.flags = present_only();
        line_playback.redraw_requested = true;
        line.resource = BLACK_HOLE_IDLE_PRESENTATION_RESOURCE;
        Ok(BlackHolePresentationActorOutcome::Completed)
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

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 21;
    const TEST_TARGET_RECORD: u16 = 48_000;
    const TEST_PREVIOUS_DEFERRED_RECORD: u16 = 49_000;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_before: u8,
        ui_after: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        line_resource_before: u16,
        line_resource_after: u16,
        target_kind: Option<u16>,
        target_record_after: u16,
        deferred_type_after: u16,
        deferred_link_after: u16,
        #[serde(default)]
        helper_results: Vec<bool>,
        #[serde(default)]
        call_sequence: Vec<String>,
    }

    struct OracleBackend {
        name: String,
        helper_results: Vec<bool>,
        helper_index: usize,
        blockers: BlackHolePresentationBlockers,
        calls: Vec<String>,
    }

    impl PresentationLineStepper for OracleBackend {
        type Error = std::convert::Infallible;

        fn update_line(
            &mut self,
            _line: &mut PresentationLine,
            _playback: &mut PresentationLinePlayback,
        ) -> Result<PresentationLineOutcome, Self::Error> {
            self.calls.push(String::from("presentation_line_helper"));
            let result = self.helper_results[self.helper_index];
            if self.helper_index == usize::MIN {
                if self.name == "present_ready_helper_arms_actor_five" {
                    self.blockers.camera_presentation_active = true;
                }
                if self.name == "absent_unloaded_helper_arms_panel" {
                    self.blockers.location_panel_active = true;
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

    impl BlackHolePresentationActorBackend for OracleBackend {
        fn presentation_blockers(&self) -> BlackHolePresentationBlockers {
            self.blockers
        }

        fn mark_presentation_entity_dirty(&mut self) {
            self.calls
                .push(String::from("entity_flag_state_transition"));
        }

        fn play_black_hole_transition_clip(&mut self) {
            self.calls.push(String::from("snd_play_clip"));
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7ec0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let setup = CaseSetup::for_name(&vector.name);
            let target_is_black_hole = vector.target_kind == Some(256);
            let current_target = vector.target_kind.map(|_| BlackHoleNavigationTarget {
                record: if target_is_black_hole {
                    vector.target_record_after
                } else {
                    TEST_TARGET_RECORD
                },
                kind: if target_is_black_hole {
                    ScriptObjectKind::BlackHole
                } else {
                    ScriptObjectKind::WorldState
                },
            });
            let target_before = if target_is_black_hole {
                TEST_TARGET_RECORD
            } else {
                vector.target_record_after
            };
            let deferred_written = vector.deferred_type_after == 198;
            let mut state = BlackHolePresentationActorState {
                target_record: Some(target_before),
                deferred_record: Some(if deferred_written {
                    TEST_PREVIOUS_DEFERRED_RECORD
                } else {
                    vector.deferred_link_after
                }),
                deferred_action: BlackHoleDeferredAction::Unchanged,
                target_presentation_cleared: false,
                presentation: BlackHoleActorPresentation::Unchanged,
                transition_phase_reset: false,
            };
            let mut line = PresentationLine {
                flags: decode_line_flags(vector.line_flags_before),
                resource: PresentationResourceId::new(vector.line_resource_before),
                terminal_frame: u16::MIN,
                frame: u16::MIN,
                position: [u16::MIN; 2],
            };
            let mut line_playback = PresentationLinePlayback {
                busy: false,
                reverse: setup.reverse_playback,
                redraw_requested: vector.ui_before & 4 != u8::MIN,
            };
            let mut backend = OracleBackend {
                name: vector.name.clone(),
                helper_results: vector.helper_results.clone(),
                helper_index: usize::MIN,
                blockers: setup.blockers,
                calls: Vec::new(),
            };

            update_black_hole_presentation_actor(
                BlackHolePresentationActorContext {
                    enabled: vector.ui_before & 16 != u8::MIN,
                    actor_busy: setup.actor_busy,
                    camera_state_enables_absent_line: setup.camera_state_enables_absent_line,
                    current_target: current_target.as_ref(),
                },
                &mut line,
                &mut line_playback,
                &mut state,
                &mut backend,
            )
            .unwrap();

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
            assert_eq!(
                state.target_record,
                Some(vector.target_record_after),
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
                state.deferred_action == BlackHoleDeferredAction::Travel,
                deferred_written,
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_record,
                Some(vector.deferred_link_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.transition_phase_reset, deferred_written,
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
                state.presentation == BlackHoleActorPresentation::Entry,
                first_pass_started,
                "{}",
                vector.name
            );
        }
    }

    #[derive(Clone, Copy)]
    struct CaseSetup {
        actor_busy: bool,
        reverse_playback: bool,
        camera_state_enables_absent_line: bool,
        blockers: BlackHolePresentationBlockers,
    }

    impl CaseSetup {
        fn for_name(name: &str) -> Self {
            let mut setup = Self {
                actor_busy: name == "actor_one_busy_high_bit",
                reverse_playback: matches!(
                    name,
                    "absent_loaded_helper_incomplete"
                        | "absent_loaded_helper_complete_idle"
                        | "absent_unloaded_actor_five_busy"
                        | "absent_wrapped_arche_link"
                ),
                camera_state_enables_absent_line: matches!(
                    name,
                    "absent_loaded_unrelated_state_bits"
                        | "absent_unloaded_incomplete"
                        | "absent_unloaded_complete_idle"
                        | "absent_unloaded_helper_arms_panel"
                ),
                blockers: BlackHolePresentationBlockers::default(),
            };
            setup.blockers.location_panel_active = matches!(
                name,
                "present_panel_second_incomplete"
                    | "present_ready_panel_two_incomplete"
                    | "present_ready_panel_two_complete"
            );
            setup.blockers.camera_presentation_active = matches!(
                name,
                "present_actor_five_second_complete" | "absent_unloaded_actor_five_busy"
            );
            setup
        }
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
