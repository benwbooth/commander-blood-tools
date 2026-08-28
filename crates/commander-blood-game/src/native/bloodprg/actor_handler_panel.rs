//! Presentation-panel close actor state machine.

use super::{
    PresentationLine, PresentationLineFlags, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper,
};

/// Actor-presentation state published while the panel-close line plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PanelCloseActorPresentation {
    /// This handler has not published a presentation state.
    #[default]
    Unchanged,
    /// Panel-close presentation is active.
    Presenting,
}

/// Mutable state owned by presentation actor handler 3.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PanelCloseActorState {
    /// The presentation panel currently owns the bridge.
    pub panel_active: bool,
    /// A scene presentation remains queued.
    pub scene_queued: bool,
    /// Primary-button edge latch.
    pub mouse_primary_pressed: bool,
    /// Pending mouse-press latch.
    pub mouse_press_pending: bool,
    /// The bridge needs a redraw.
    pub redraw_requested: bool,
    /// Actor processing observed an active loaded panel line.
    pub completion_latched: bool,
    /// Published actor presentation state.
    pub presentation: PanelCloseActorPresentation,
}

/// Line, queue, and entity services used by the panel-close actor.
pub trait PanelCloseActorBackend: PresentationLineStepper {
    /// Request the panel-close hand animation through the shared selector.
    fn request_panel_close_hand_animation(&mut self);

    /// Move the shared presentation phase to the first closing frame when it is
    /// still in a pre-close phase.
    fn begin_panel_close_if_open(&mut self) -> bool;

    /// Finalize the currently queued scene presentation.
    fn finalize_scene_presentation(&mut self);

    /// Reset the shared presentation entity.
    fn reset_presentation_entity(&mut self);
}

/// Terminal path taken by one panel-close actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PanelCloseActorOutcome {
    /// Secondary panel mode is disabled.
    Disabled,
    /// The line is present but not ready.
    WaitingForLine,
    /// Line playback remains in progress.
    Presenting,
    /// Playback completed and panel ownership is active.
    Completed,
}

/// Update presentation actor 3 and coordinate panel close/finalization state.
///
/// This translates `nav_actor_handler_3` at BLOODPRG routine offset `0x00817E`.
/// Signed zoom state, booleans, and reusable line playback replace UI masks,
/// byte-wide mode ownership, numeric actor states, and packed line flags.
pub fn update_panel_close_actor<Backend: PanelCloseActorBackend>(
    enabled: bool,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    state: &mut PanelCloseActorState,
    backend: &mut Backend,
) -> Result<PanelCloseActorOutcome, Backend::Error> {
    if !enabled {
        return Ok(PanelCloseActorOutcome::Disabled);
    }

    line.flags.present = true;
    let outcome = if line.flags.ready {
        state.presentation = PanelCloseActorPresentation::Presenting;
        backend.request_panel_close_hand_animation();
        if state.panel_active && backend.begin_panel_close_if_open() {
            if state.scene_queued {
                backend.finalize_scene_presentation();
                state.scene_queued = false;
            }
        }

        state.mouse_primary_pressed = false;
        state.mouse_press_pending = false;
        let line_outcome = backend.update_line(line, line_playback)?;
        state.redraw_requested = line_playback.redraw_requested;
        if line_outcome == PresentationLineOutcome::Completed {
            backend.reset_presentation_entity();
            line.flags = present_only();
            if !state.panel_active {
                state.panel_active = true;
                state.redraw_requested = true;
            }
            PanelCloseActorOutcome::Completed
        } else {
            PanelCloseActorOutcome::Presenting
        }
    } else {
        PanelCloseActorOutcome::WaitingForLine
    };

    if state.panel_active && line.flags.resource_loaded {
        state.completion_latched = true;
    }
    Ok(outcome)
}

const fn present_only() -> PresentationLineFlags {
    PresentationLineFlags {
        present: true,
        transition_latched: false,
        resource_loaded: false,
        ready: false,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::PresentationResourceId;

    const ORACLE_VECTOR_COUNT: usize = 10;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_before: u8,
        ui_after: u8,
        mode_before: u8,
        mode_after: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        zoom_before: u16,
        zoom_after: u16,
        presentation_finalizer_called: bool,
        line_helper_called: bool,
        line_helper_completed: bool,
        entity_transition_id: Option<u8>,
        mouse_after: [u8; 2],
        completion_latch_after: u8,
    }

    #[derive(Default)]
    struct OracleBackend {
        line_called: bool,
        completed: bool,
        finalizer_called: bool,
        entity_called: bool,
        hand_animation_requested: bool,
        panel_phase: i16,
    }

    impl PresentationLineStepper for OracleBackend {
        type Error = std::convert::Infallible;

        fn update_line(
            &mut self,
            _line: &mut PresentationLine,
            _playback: &mut PresentationLinePlayback,
        ) -> Result<PresentationLineOutcome, Self::Error> {
            self.line_called = true;
            Ok(if self.completed {
                PresentationLineOutcome::Completed
            } else {
                PresentationLineOutcome::Advanced
            })
        }
    }

    impl PanelCloseActorBackend for OracleBackend {
        fn request_panel_close_hand_animation(&mut self) {
            self.hand_animation_requested = true;
        }

        fn begin_panel_close_if_open(&mut self) -> bool {
            if self.panel_phase >= 100 {
                return false;
            }
            self.panel_phase = 106;
            true
        }

        fn finalize_scene_presentation(&mut self) {
            self.finalizer_called = true;
        }

        fn reset_presentation_entity(&mut self) {
            self.entity_called = true;
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_817e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut line = PresentationLine {
                flags: decode_line_flags(vector.line_flags_before),
                resource: PresentationResourceId::new(1),
                terminal_frame: 4,
                frame: 1,
                position: [0, 0],
            };
            let mut playback = PresentationLinePlayback::default();
            let mut state = PanelCloseActorState {
                panel_active: vector.mode_before & 1 != u8::MIN,
                scene_queued: vector.presentation_finalizer_called,
                mouse_primary_pressed: true,
                mouse_press_pending: true,
                redraw_requested: vector.ui_before & 4 != u8::MIN,
                completion_latched: false,
                presentation: PanelCloseActorPresentation::Unchanged,
            };
            let mut backend = OracleBackend {
                completed: vector.line_helper_completed,
                panel_phase: vector.zoom_before as i16,
                ..OracleBackend::default()
            };

            update_panel_close_actor(
                vector.ui_before & 0x40 != u8::MIN,
                &mut line,
                &mut playback,
                &mut state,
                &mut backend,
            )
            .unwrap();

            assert_eq!(
                backend.line_called, vector.line_helper_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.hand_animation_requested, vector.line_helper_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.finalizer_called, vector.presentation_finalizer_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.entity_called,
                vector.entity_transition_id == Some(4),
                "{}",
                vector.name
            );
            assert_eq!(
                state.panel_active,
                vector.mode_after & 1 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.panel_phase as u16, vector.zoom_after,
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
                [
                    state.mouse_primary_pressed as u8,
                    state.mouse_press_pending as u8
                ],
                vector.mouse_after.map(|value| u8::from(value != u8::MIN)),
                "{}",
                vector.name
            );
            assert_eq!(
                state.completion_latched,
                vector.completion_latch_after == 1,
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
                state.presentation == PanelCloseActorPresentation::Presenting,
                vector.line_helper_called,
                "{}",
                vector.name
            );
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
