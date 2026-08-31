//! Radio-record presentation actor state machine.

use super::{
    PresentationLine, PresentationLineFlags, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper,
};

/// Deferred action selected after the radio line completes.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioActorDeferredAction {
    /// This handler has not replaced the previous action kind.
    #[default]
    Unchanged,
    /// Process the deferred radio record.
    RadioRecord,
}

/// Actor-presentation state published while the radio line plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum RadioActorPresentation {
    /// This handler has not published a presentation state.
    #[default]
    Unchanged,
    /// Radio-record presentation is active.
    Presenting,
}

/// Mutable record and presentation state owned by actor handler 4.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RadioActorState<RecordLink> {
    pending_record: Option<RecordLink>,
    deferred_record: Option<RecordLink>,
    deferred_action: RadioActorDeferredAction,
    presentation: RadioActorPresentation,
    redraw_requested: bool,
}

impl<RecordLink> Default for RadioActorState<RecordLink> {
    fn default() -> Self {
        Self::new(None, None, false)
    }
}

impl<RecordLink> RadioActorState<RecordLink> {
    /// Build state from typed pending and deferred VM record relationships.
    pub fn new(
        pending_record: Option<RecordLink>,
        deferred_record: Option<RecordLink>,
        redraw_requested: bool,
    ) -> Self {
        Self {
            pending_record,
            deferred_record,
            deferred_action: RadioActorDeferredAction::Unchanged,
            presentation: RadioActorPresentation::Unchanged,
            redraw_requested,
        }
    }

    /// Return the pending record, when present.
    pub const fn pending_record(&self) -> Option<&RecordLink> {
        self.pending_record.as_ref()
    }

    /// Return the record deferred by this actor.
    pub const fn deferred_record(&self) -> Option<&RecordLink> {
        self.deferred_record.as_ref()
    }

    /// Return the semantic deferred action kind.
    pub const fn deferred_action(&self) -> RadioActorDeferredAction {
        self.deferred_action
    }

    /// Return the actor-presentation state.
    pub const fn presentation(&self) -> RadioActorPresentation {
        self.presentation
    }

    /// Return whether the bridge needs a redraw.
    pub const fn redraw_requested(&self) -> bool {
        self.redraw_requested
    }

    /// Synchronize the VM-owned pending presentation record before an update.
    pub fn set_pending_record(&mut self, pending_record: Option<RecordLink>) {
        self.pending_record = pending_record;
    }

    /// Synchronize the shared bridge redraw bit before this actor runs.
    pub fn set_redraw_requested(&mut self, redraw_requested: bool) {
        self.redraw_requested = redraw_requested;
    }

    /// Consume the record promoted to deferred C4 ownership by this actor.
    pub fn take_deferred_record(&mut self) -> Option<RecordLink> {
        self.deferred_record.take()
    }
}

/// Line, audio, entity, and sound-bank services used by the radio actor.
pub trait RadioActorBackend: PresentationLineStepper {
    /// Publish the radio hand animation through the shared MANU3 selector.
    fn request_radio_hand_animation(&mut self);

    /// Play the radio-line completion clip.
    fn play_radio_completion_clip(&mut self);

    /// Publish the pending-to-deferred C4 record transfer.
    fn transfer_pending_radio_record(&mut self);

    /// Reset the shared presentation entity.
    fn reset_presentation_entity(&mut self);

    /// Reload the radio sound bank used after this action.
    fn reload_radio_sound_bank(&mut self);
}

/// Terminal path taken by one radio actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RadioActorOutcome {
    /// Radio presentation mode is disabled.
    Disabled,
    /// The line is not ready and has no loaded resource to continue.
    WaitingForLine,
    /// No pending or deferred record is available.
    NoRecord,
    /// Line playback remains in progress.
    Presenting,
    /// Playback completed and the pending record was deferred.
    Completed,
}

/// Update presentation actor 4 and transfer its pending radio record.
///
/// This translates `nav_actor_handler_4` at BLOODPRG routine offset `0x0081FB`.
/// Typed optional record links and semantic actions replace zero-valued links,
/// a numeric record tag, UI masks, and packed presentation-line flags.
pub fn update_radio_actor<RecordLink: Clone, Backend: RadioActorBackend>(
    enabled: bool,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    state: &mut RadioActorState<RecordLink>,
    backend: &mut Backend,
) -> Result<RadioActorOutcome, Backend::Error> {
    if !enabled {
        return Ok(RadioActorOutcome::Disabled);
    }

    line.flags.present = true;
    if !line.flags.resource_loaded {
        if !line.flags.ready {
            return Ok(RadioActorOutcome::WaitingForLine);
        }
        if state.deferred_record.is_none() && state.pending_record.is_none() {
            line.flags = present_only();
            return Ok(RadioActorOutcome::NoRecord);
        }
    }

    state.presentation = RadioActorPresentation::Presenting;
    backend.request_radio_hand_animation();
    let line_outcome = backend.update_line(line, line_playback)?;
    state.redraw_requested = line_playback.redraw_requested;
    if line_outcome != PresentationLineOutcome::Completed {
        return Ok(RadioActorOutcome::Presenting);
    }

    backend.play_radio_completion_clip();
    state.deferred_record = state.pending_record.take();
    state.deferred_action = RadioActorDeferredAction::RadioRecord;
    backend.transfer_pending_radio_record();
    line.flags = present_only();
    backend.reset_presentation_entity();
    state.redraw_requested = true;
    backend.reload_radio_sound_bank();
    Ok(RadioActorOutcome::Completed)
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

    const ORACLE_VECTOR_COUNT: usize = 9;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_before: u8,
        ui_after: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        pending_before: u16,
        pending_after: u16,
        deferred_before: u16,
        deferred_after: u16,
        deferred_type_after: u16,
        line_helper_called: bool,
        line_helper_completed: bool,
        sound_clip: Option<u8>,
        entity_transition_id: Option<u8>,
        sound_bank_mode: Option<u8>,
        sound_bank_path: Option<String>,
    }

    #[derive(Default)]
    struct OracleBackend {
        line_called: bool,
        completed: bool,
        hand_animation_requested: bool,
        clip_called: bool,
        entity_called: bool,
        bank_called: bool,
        transfer_called: bool,
        call_order: Vec<&'static str>,
    }

    impl PresentationLineStepper for OracleBackend {
        type Error = std::convert::Infallible;

        fn update_line(
            &mut self,
            _line: &mut PresentationLine,
            _playback: &mut PresentationLinePlayback,
        ) -> Result<PresentationLineOutcome, Self::Error> {
            self.line_called = true;
            self.call_order.push("line");
            Ok(if self.completed {
                PresentationLineOutcome::Completed
            } else {
                PresentationLineOutcome::Advanced
            })
        }
    }

    impl RadioActorBackend for OracleBackend {
        fn request_radio_hand_animation(&mut self) {
            self.hand_animation_requested = true;
            self.call_order.push("hand");
        }

        fn play_radio_completion_clip(&mut self) {
            self.clip_called = true;
            self.call_order.push("clip");
        }

        fn transfer_pending_radio_record(&mut self) {
            self.transfer_called = true;
            self.call_order.push("transfer");
        }

        fn reset_presentation_entity(&mut self) {
            self.entity_called = true;
            self.call_order.push("entity");
        }

        fn reload_radio_sound_bank(&mut self) {
            self.bank_called = true;
            self.call_order.push("bank");
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_81fb_natural.json"
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
            let mut state = RadioActorState::new(
                nonzero(vector.pending_before),
                nonzero(vector.deferred_before),
                vector.ui_before & 4 != u8::MIN,
            );
            let mut backend = OracleBackend {
                completed: vector.line_helper_completed,
                ..OracleBackend::default()
            };

            update_radio_actor(
                vector.ui_before & 0x20 != u8::MIN,
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
                backend.clip_called,
                vector.sound_clip == Some(2),
                "{}",
                vector.name
            );
            assert_eq!(
                backend.transfer_called,
                vector.sound_clip == Some(2),
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
                backend.bank_called,
                vector.sound_bank_mode == Some(1),
                "{}",
                vector.name
            );
            assert_eq!(
                backend.bank_called,
                vector.sound_bank_path.as_deref() == Some("sn\\radio.snd"),
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
                state.pending_record().copied().unwrap_or_default(),
                vector.pending_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_record().copied().unwrap_or_default(),
                vector.deferred_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.redraw_requested(),
                vector.ui_after & 4 != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                state.deferred_action() == RadioActorDeferredAction::RadioRecord,
                vector.deferred_type_after == 196,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation() == RadioActorPresentation::Presenting,
                vector.line_helper_called,
                "{}",
                vector.name
            );
            let expected_order: &[&str] = if vector.sound_clip == Some(2) {
                &["hand", "line", "clip", "transfer", "entity", "bank"]
            } else if vector.line_helper_called {
                &["hand", "line"]
            } else {
                &[]
            };
            assert_eq!(backend.call_order, expected_order, "{}", vector.name);
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
