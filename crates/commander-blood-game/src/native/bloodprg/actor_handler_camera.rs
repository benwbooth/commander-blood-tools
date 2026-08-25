//! Bridge camera-presentation actor state machine.

use super::{
    PresentationLine, PresentationLineFlags, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper,
};

/// Presentation-line frame that starts the bridge camera transition.
pub const CAMERA_TRANSITION_FRAME: u16 = 7;

/// Number of update steps in the bridge camera transition.
pub const CAMERA_VIEW_TRANSITION_STEPS: u8 = 8;

/// Other bridge actors that can defer the camera presentation.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CameraPresentationBlockers {
    /// The primary navigation actor is still busy.
    pub primary_actor_busy: bool,
    /// The secondary navigation actor is still busy.
    pub secondary_actor_busy: bool,
}

impl CameraPresentationBlockers {
    const fn any(self) -> bool {
        self.primary_actor_busy || self.secondary_actor_busy
    }
}

/// Presentation state published while the camera line plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraActorPresentation {
    /// This handler has not selected a presentation mode.
    #[default]
    Unchanged,
    /// The bridge camera presentation is active.
    CameraView,
}

/// Current bridge camera animation state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraViewAnimation {
    /// This handler has not started a camera animation.
    #[default]
    Unchanged,
    /// The camera transition is counting down to its settled view.
    Transitioning {
        /// Update steps remaining at transition start.
        steps_remaining: u8,
    },
}

/// Camera-view transition requested by a page flip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraPageFlipOutcome {
    /// Keep the current camera view.
    KeepCurrentView,
    /// Toggle between the bridge and camera views.
    ToggleCameraView,
}

/// Mutable state owned by presentation actor handler 5.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraPresentationActorState<RecordLink> {
    /// The actor is already active and may resume without a ready line.
    pub active: bool,
    /// Currently selected location, cleared when presentation begins.
    pub selected_location: Option<RecordLink>,
    /// Presentation mode published by this actor.
    pub presentation: CameraActorPresentation,
    /// Primary-button edge latch.
    pub mouse_primary_pressed: bool,
    /// Whether the camera view currently replaces the bridge view.
    pub camera_view_active: bool,
    /// Camera animation started by transition-frame playback.
    pub camera_animation: CameraViewAnimation,
    /// Whether the location information panel currently owns the bridge.
    pub location_panel_active: bool,
    /// Whether the bridge presentation region needs a redraw.
    pub redraw_requested: bool,
    /// Whether returning to the bridge requires a complete screen rebuild.
    pub screen_rebuild_pending: bool,
}

/// Line, page, audio, entity, and ship services used by the camera actor.
pub trait CameraPresentationActorBackend: PresentationLineStepper {
    /// Mark the location-panel entity for a state transition.
    fn mark_location_panel_entity_dirty(&mut self);

    /// Flip the inactive camera page and report whether it requests a view change.
    fn flip_camera_page(&mut self) -> CameraPageFlipOutcome;

    /// Play the camera transition clip.
    fn play_camera_transition_clip(&mut self);

    /// Restore the bridge ship palette and reset its camera.
    fn reset_ship_camera_and_palette(&mut self);

    /// Mark the shared presentation entity for a state transition.
    fn mark_presentation_entity_dirty(&mut self);
}

/// Terminal path taken by one camera actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraPresentationActorOutcome {
    /// Camera presentation mode is disabled.
    Disabled,
    /// The line is present but not ready.
    WaitingForLine,
    /// Another bridge actor deferred this actor.
    Blocked,
    /// Camera-line playback remains active.
    Presenting,
    /// The update switched from the bridge to the camera view.
    CameraViewActivated,
    /// The update returned from the camera to the bridge view.
    CameraViewDeactivated,
}

/// Update presentation actor 5 and coordinate bridge camera-view transitions.
///
/// This translates `nav_actor_handler_5` at BLOODPRG routine offset `0x008082`.
/// Typed record ownership, booleans, semantic page feedback, and an explicit
/// transition countdown replace packed UI bytes, object offsets, and numeric
/// actor state. All state remains ordinary flat Rust data.
pub fn update_camera_presentation_actor<RecordLink, Backend: CameraPresentationActorBackend>(
    enabled: bool,
    blockers: CameraPresentationBlockers,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    state: &mut CameraPresentationActorState<RecordLink>,
    backend: &mut Backend,
) -> Result<CameraPresentationActorOutcome, Backend::Error> {
    if !enabled {
        return Ok(CameraPresentationActorOutcome::Disabled);
    }

    if !state.active {
        line.flags.present = true;
        let transition_requested = line.flags.transition_latched;
        if !line.flags.ready {
            return Ok(if transition_requested {
                toggle_camera_view(line, state, backend)
            } else {
                CameraPresentationActorOutcome::WaitingForLine
            });
        }
    }

    if blockers.any() {
        state.active = true;
        state.location_panel_active = false;
        return Ok(CameraPresentationActorOutcome::Blocked);
    }

    backend.mark_location_panel_entity_dirty();
    state.selected_location = None;
    state.presentation = CameraActorPresentation::CameraView;
    state.mouse_primary_pressed = false;

    let mut transition_requested = false;
    let line_outcome = backend.update_line(line, line_playback)?;
    if line.frame == CAMERA_TRANSITION_FRAME {
        if !state.camera_view_active {
            transition_requested =
                backend.flip_camera_page() == CameraPageFlipOutcome::ToggleCameraView;
        }
        backend.play_camera_transition_clip();
        state.camera_animation = CameraViewAnimation::Transitioning {
            steps_remaining: CAMERA_VIEW_TRANSITION_STEPS,
        };
        state.redraw_requested = true;
    }

    if line_outcome == PresentationLineOutcome::Completed {
        state.active = false;
        line.flags = completed_transition();
        transition_requested = true;
    }

    if transition_requested {
        Ok(toggle_camera_view(line, state, backend))
    } else {
        Ok(CameraPresentationActorOutcome::Presenting)
    }
}

fn toggle_camera_view<RecordLink, Backend: CameraPresentationActorBackend>(
    line: &mut PresentationLine,
    state: &mut CameraPresentationActorState<RecordLink>,
    backend: &mut Backend,
) -> CameraPresentationActorOutcome {
    state.camera_view_active = !state.camera_view_active;
    state.redraw_requested = state.camera_view_active;
    let outcome = if state.camera_view_active {
        CameraPresentationActorOutcome::CameraViewActivated
    } else {
        backend.reset_ship_camera_and_palette();
        state.screen_rebuild_pending = true;
        CameraPresentationActorOutcome::CameraViewDeactivated
    };

    state.location_panel_active = false;
    line.flags = present_only();
    backend.mark_presentation_entity_dirty();
    outcome
}

const fn present_only() -> PresentationLineFlags {
    PresentationLineFlags {
        present: true,
        transition_latched: false,
        resource_loaded: false,
        ready: false,
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
    use crate::native::bloodprg::PresentationResourceId;

    const ORACLE_VECTOR_COUNT: usize = 14;
    const TEST_RECORD_LINK: u16 = 93;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_before: u8,
        ui_after: u8,
        active_before: u8,
        active_after: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        frame_before: u16,
        frame_after_helper: u16,
        blocker_value: Option<u8>,
        line_helper_called: bool,
        line_helper_completed: bool,
        page_flip_called: bool,
        page_flip_result: Option<u8>,
        sound_clip: Option<u8>,
        view_before: u8,
        view_after: u8,
        ship_reset_called: bool,
        entity_transitions: Vec<u8>,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum BackendEvent {
        LocationPanelEntity,
        LinePlayback,
        PageFlip,
        CameraClip,
        ShipReset,
        PresentationEntity,
    }

    struct OracleBackend {
        frame_after_helper: u16,
        helper_completed: bool,
        page_flip_outcome: CameraPageFlipOutcome,
        events: Vec<BackendEvent>,
    }

    impl PresentationLineStepper for OracleBackend {
        type Error = std::convert::Infallible;

        fn update_line(
            &mut self,
            line: &mut PresentationLine,
            _playback: &mut PresentationLinePlayback,
        ) -> Result<PresentationLineOutcome, Self::Error> {
            self.events.push(BackendEvent::LinePlayback);
            line.frame = self.frame_after_helper;
            Ok(if self.helper_completed {
                PresentationLineOutcome::Completed
            } else {
                PresentationLineOutcome::Advanced
            })
        }
    }

    impl CameraPresentationActorBackend for OracleBackend {
        fn mark_location_panel_entity_dirty(&mut self) {
            self.events.push(BackendEvent::LocationPanelEntity);
        }

        fn flip_camera_page(&mut self) -> CameraPageFlipOutcome {
            self.events.push(BackendEvent::PageFlip);
            self.page_flip_outcome
        }

        fn play_camera_transition_clip(&mut self) {
            self.events.push(BackendEvent::CameraClip);
        }

        fn reset_ship_camera_and_palette(&mut self) {
            self.events.push(BackendEvent::ShipReset);
        }

        fn mark_presentation_entity_dirty(&mut self) {
            self.events.push(BackendEvent::PresentationEntity);
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8082_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let blockers = blockers_for(&vector);
            let mut line = PresentationLine {
                flags: decode_line_flags(vector.line_flags_before),
                resource: PresentationResourceId::new(1),
                terminal_frame: CAMERA_TRANSITION_FRAME,
                frame: vector.frame_before,
                position: [0, 0],
            };
            let mut line_playback = PresentationLinePlayback::default();
            let mut state = CameraPresentationActorState {
                active: vector.active_before & 1 != u8::MIN,
                selected_location: Some(TEST_RECORD_LINK),
                presentation: CameraActorPresentation::Unchanged,
                mouse_primary_pressed: true,
                camera_view_active: vector.view_before & 1 != u8::MIN,
                camera_animation: CameraViewAnimation::Unchanged,
                location_panel_active: true,
                redraw_requested: vector.ui_before & 4 != u8::MIN,
                screen_rebuild_pending: false,
            };
            let mut backend = OracleBackend {
                frame_after_helper: vector.frame_after_helper,
                helper_completed: vector.line_helper_completed,
                page_flip_outcome: decode_page_flip(vector.page_flip_result),
                events: Vec::new(),
            };

            update_camera_presentation_actor(
                vector.ui_before & 16 != u8::MIN,
                blockers,
                &mut line,
                &mut line_playback,
                &mut state,
                &mut backend,
            )
            .unwrap();

            assert_eq!(
                state.active,
                vector.active_after & 1 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                encode_line_flags(line.flags),
                vector.line_flags_after & 15,
                "{}",
                vector.name
            );
            assert_eq!(line.frame, vector.frame_after_helper, "{}", vector.name);
            assert_eq!(
                state.camera_view_active,
                vector.view_after & 1 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.redraw_requested,
                vector.ui_after & 4 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.events.contains(&BackendEvent::LinePlayback),
                vector.line_helper_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.events.contains(&BackendEvent::PageFlip),
                vector.page_flip_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.events.contains(&BackendEvent::CameraClip),
                vector.sound_clip == Some(3),
                "{}",
                vector.name
            );
            assert_eq!(
                backend.events.contains(&BackendEvent::ShipReset),
                vector.ship_reset_called,
                "{}",
                vector.name
            );
            assert_eq!(
                entity_transitions(&backend.events),
                vector.entity_transitions,
                "{}",
                vector.name
            );

            if vector.line_helper_called {
                assert_eq!(state.selected_location, None, "{}", vector.name);
                assert_eq!(
                    state.presentation,
                    CameraActorPresentation::CameraView,
                    "{}",
                    vector.name
                );
                assert!(!state.mouse_primary_pressed, "{}", vector.name);
            } else {
                assert_eq!(
                    state.selected_location,
                    Some(TEST_RECORD_LINK),
                    "{}",
                    vector.name
                );
            }

            if vector.sound_clip == Some(3) {
                assert_eq!(
                    state.camera_animation,
                    CameraViewAnimation::Transitioning {
                        steps_remaining: CAMERA_VIEW_TRANSITION_STEPS,
                    },
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(
                    state.camera_animation,
                    CameraViewAnimation::Unchanged,
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                state.screen_rebuild_pending, vector.ship_reset_called,
                "{}",
                vector.name
            );
            assert_event_order(&backend.events, &vector.name);
        }
    }

    fn blockers_for(vector: &ActorOracle) -> CameraPresentationBlockers {
        CameraPresentationBlockers {
            primary_actor_busy: vector.blocker_value == Some(128),
            secondary_actor_busy: vector.blocker_value == Some(1),
        }
    }

    fn decode_page_flip(result: Option<u8>) -> CameraPageFlipOutcome {
        if result.is_some_and(|value| value & 2 != u8::MIN) {
            CameraPageFlipOutcome::ToggleCameraView
        } else {
            CameraPageFlipOutcome::KeepCurrentView
        }
    }

    fn entity_transitions(events: &[BackendEvent]) -> Vec<u8> {
        events
            .iter()
            .filter_map(|event| match event {
                BackendEvent::LocationPanelEntity => Some(0),
                BackendEvent::PresentationEntity => Some(4),
                _ => None,
            })
            .collect()
    }

    fn assert_event_order(events: &[BackendEvent], name: &str) {
        let expected_order = [
            BackendEvent::LocationPanelEntity,
            BackendEvent::LinePlayback,
            BackendEvent::PageFlip,
            BackendEvent::CameraClip,
            BackendEvent::ShipReset,
            BackendEvent::PresentationEntity,
        ];
        let ranks: Vec<_> = events
            .iter()
            .map(|event| {
                expected_order
                    .iter()
                    .position(|candidate| candidate == event)
                    .unwrap()
            })
            .collect();
        assert!(ranks.windows(2).all(|pair| pair[0] < pair[1]), "{name}");
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
