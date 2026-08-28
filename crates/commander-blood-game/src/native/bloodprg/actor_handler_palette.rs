//! Ship-palette presentation actor state machine.

use super::{
    PresentationLine, PresentationLineFlags, PresentationLineOutcome, PresentationLinePlayback,
    PresentationLineStepper,
};

/// Number of bytes in the actor's 192-color RGB palette snapshot.
pub const SHIP_ACTOR_PALETTE_BYTES: usize = 576;

/// Actor-presentation state published while the ship palette line plays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShipPaletteActorPresentation {
    /// This handler has not published a presentation state.
    #[default]
    Unchanged,
    /// The ship palette transition line is active.
    Presenting,
}

/// Mutable ship state owned by presentation actor handler 2.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipPaletteActorState {
    presentation: ShipPaletteActorPresentation,
    ship_active: bool,
    bridge_palette: [u8; SHIP_ACTOR_PALETTE_BYTES],
    ship_depth_offset: u16,
}

impl Default for ShipPaletteActorState {
    fn default() -> Self {
        Self {
            presentation: ShipPaletteActorPresentation::default(),
            ship_active: false,
            bridge_palette: [u8::MIN; SHIP_ACTOR_PALETTE_BYTES],
            ship_depth_offset: u16::MIN,
        }
    }
}

impl ShipPaletteActorState {
    /// Return the presentation state published by this actor.
    pub const fn presentation(&self) -> ShipPaletteActorPresentation {
        self.presentation
    }

    /// Return whether ship rendering was activated after line completion.
    pub const fn ship_active(&self) -> bool {
        self.ship_active
    }

    /// Return the flat palette snapshot used by bridge ship rendering.
    pub const fn bridge_palette(&self) -> &[u8; SHIP_ACTOR_PALETTE_BYTES] {
        &self.bridge_palette
    }

    /// Return the reset ship depth offset.
    pub const fn ship_depth_offset(&self) -> u16 {
        self.ship_depth_offset
    }
}

/// Line playback and audio services used by the ship-palette actor.
pub trait ShipPaletteActorBackend: PresentationLineStepper {
    /// Request the ship-palette hand animation through the shared selector.
    fn request_ship_palette_hand_animation(&mut self);

    /// Play the ship-activation completion clip.
    fn play_ship_activation_clip(&mut self);
}

/// Terminal path taken by one ship-palette actor update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipPaletteActorOutcome {
    /// Neither presentation UI mode enables this actor.
    Disabled,
    /// The line was marked present but is not ready.
    WaitingForLine,
    /// Line playback remains in progress.
    Presenting,
    /// Playback completed and ship palette state was activated.
    Completed,
}

/// Update presentation actor 2 and publish its completed ship palette.
///
/// This translates `nav_actor_handler_2` at BLOODPRG routine offset `0x00813A`.
/// Semantic UI eligibility, typed line flags, and a flat palette array replace
/// UI masks, packed line bytes, numeric presentation state, and memory copying.
pub fn update_ship_palette_actor<Backend: ShipPaletteActorBackend>(
    enabled: bool,
    line: &mut PresentationLine,
    line_playback: &mut PresentationLinePlayback,
    live_palette: &[u8; SHIP_ACTOR_PALETTE_BYTES],
    state: &mut ShipPaletteActorState,
    backend: &mut Backend,
) -> Result<ShipPaletteActorOutcome, Backend::Error> {
    if !enabled {
        return Ok(ShipPaletteActorOutcome::Disabled);
    }

    line.flags.present = true;
    if !line.flags.ready {
        return Ok(ShipPaletteActorOutcome::WaitingForLine);
    }

    state.presentation = ShipPaletteActorPresentation::Presenting;
    backend.request_ship_palette_hand_animation();
    if backend.update_line(line, line_playback)? != PresentationLineOutcome::Completed {
        return Ok(ShipPaletteActorOutcome::Presenting);
    }

    backend.play_ship_activation_clip();
    state.ship_active = true;
    state.bridge_palette.copy_from_slice(live_palette);
    state.ship_depth_offset = u16::MIN;
    line.flags = PresentationLineFlags {
        present: true,
        transition_latched: true,
        resource_loaded: true,
        ready: false,
    };
    Ok(ShipPaletteActorOutcome::Completed)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::PresentationResourceId;

    const ORACLE_VECTOR_COUNT: usize = 6;

    #[derive(Deserialize)]
    struct ActorOracle {
        name: String,
        ui_flags: u8,
        line_flags_before: u8,
        line_flags_after: u8,
        helper_called: bool,
        helper_completed: bool,
        presentation_state_after: u16,
        sound_clip: Option<u8>,
        ship_flags_after: u16,
        depth_after: u16,
        palette_bytes_copied: usize,
    }

    struct OracleBackend {
        line_called: bool,
        completed: bool,
        sound_called: bool,
        hand_animation_requested: bool,
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

    impl ShipPaletteActorBackend for OracleBackend {
        fn request_ship_palette_hand_animation(&mut self) {
            self.hand_animation_requested = true;
        }

        fn play_ship_activation_clip(&mut self) {
            self.sound_called = true;
        }
    }

    #[test]
    fn actor_matches_every_original_semantic_vector() {
        let vectors: Vec<ActorOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_813a_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let live_palette = std::array::from_fn(|index| index as u8);

        for vector in vectors {
            let enabled = vector.ui_flags & 0x90 != u8::MIN;
            let mut line = PresentationLine {
                flags: decode_line_flags(vector.line_flags_before),
                resource: PresentationResourceId::new(2),
                terminal_frame: 7,
                frame: 3,
                position: [0, 0],
            };
            let mut playback = PresentationLinePlayback::default();
            let mut state = ShipPaletteActorState {
                ship_depth_offset: vector.depth_after,
                ..ShipPaletteActorState::default()
            };
            let initial_palette = state.bridge_palette;
            let mut backend = OracleBackend {
                line_called: false,
                completed: vector.helper_completed,
                sound_called: false,
                hand_animation_requested: false,
            };

            let outcome = update_ship_palette_actor(
                enabled,
                &mut line,
                &mut playback,
                &live_palette,
                &mut state,
                &mut backend,
            )
            .unwrap();

            assert_eq!(backend.line_called, vector.helper_called, "{}", vector.name);
            assert_eq!(
                backend.hand_animation_requested, vector.helper_called,
                "{}",
                vector.name
            );
            assert_eq!(
                backend.sound_called,
                vector.sound_clip == Some(5),
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
                state.presentation() == ShipPaletteActorPresentation::Presenting,
                vector.presentation_state_after == 16,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ship_active(),
                vector.ship_flags_after == 1,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ship_depth_offset(),
                vector.depth_after,
                "{}",
                vector.name
            );
            if vector.palette_bytes_copied == SHIP_ACTOR_PALETTE_BYTES {
                assert_eq!(state.bridge_palette(), &live_palette, "{}", vector.name);
                assert_eq!(
                    outcome,
                    ShipPaletteActorOutcome::Completed,
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(state.bridge_palette(), &initial_palette, "{}", vector.name);
            }
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
