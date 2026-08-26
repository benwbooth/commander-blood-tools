//! BLOODPRG coordinator for the three interactive alien overlays.

use commander_blood_formats::alien::AlienXdbKind;

const ALIEN_OVERLAY_TRIGGER: u8 = 1;
const SHIP_SEQUENCE_ACTIVE: u16 = 1;
const ORIGINAL_VIEWPORT_WIDTH: u16 = 320;
const ORIGINAL_VIEWPORT_HEIGHT: u16 = 200;
const ORIGINAL_VIEWPORT_FIRST_PLANE: u16 = 0;
const ORIGINAL_VIEWPORT_PLANE_COUNT: u16 = 1;
const ORIGINAL_VIEWPORT_ROW_STEP: u32 = 4;
const ORIGINAL_VIEWPORT_BASE: u32 = 0;

/// Sound archive selected around an interactive alien overlay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienOverlaySoundBank {
    /// Temporary `sn\3D.snd` archive used by AMER, CROOLIS, and SCRUT.
    AlienScene,
    /// Normal `sn\tb.snd` bridge archive restored after the overlay.
    Bridge,
}

/// Flat logical viewport restored after an alien overlay returns.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienOverlayViewport {
    /// First logical destination plane.
    pub first_plane: u16,
    /// Number of logical destination planes.
    pub plane_count: u16,
    /// Original row-step descriptor value.
    pub row_step: u32,
    /// Logical width in pixels.
    pub width: u16,
    /// Logical height in pixels.
    pub height: u16,
    /// Original framebuffer-relative base.
    pub base: u32,
}

impl AlienOverlayViewport {
    const ORIGINAL: Self = Self {
        first_plane: ORIGINAL_VIEWPORT_FIRST_PLANE,
        plane_count: ORIGINAL_VIEWPORT_PLANE_COUNT,
        row_step: ORIGINAL_VIEWPORT_ROW_STEP,
        width: ORIGINAL_VIEWPORT_WIDTH,
        height: ORIGINAL_VIEWPORT_HEIGHT,
        base: ORIGINAL_VIEWPORT_BASE,
    };
}

/// State intentionally shared with a running alien overlay.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienOverlaySharedState {
    /// VM-owned timing word normalized and returned by the XDB API entry.
    pub timing_scale: u16,
    /// Ship-sequence flags that the alien callback may change.
    pub sequence_flags: u16,
    /// Logical mouse position restored after the overlay exits.
    pub mouse_position: [i16; 2],
}

/// Mutable BLOODPRG state owned by the alien-overlay cycle coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienOverlayCycleState {
    /// Bit-zero request consumed by this routine.
    pub trigger_flags: u8,
    /// Whether the surrounding ship presentation still considers an overlay armed.
    pub overlay_armed: bool,
    /// Overlay selected by the next cycle.
    pub next_overlay: AlienXdbKind,
    /// State visible to the overlay and its game callback.
    pub shared: AlienOverlaySharedState,
    /// Sound-loader flags restored around CD and overlay execution.
    pub sound_loader_flags: u16,
    /// Pending sound-driver request cleared before CD playback.
    pub sound_driver_pending: bool,
    /// Logical viewport restored after MANU3 is reloaded.
    pub viewport: AlienOverlayViewport,
    /// Mouse-idle counter reset when bridge control resumes.
    pub mouse_idle_frames: u16,
    /// Palette upload request raised after bridge restoration.
    pub palette_dirty: bool,
    /// Whether the bridge depth-band effect may copy its two regions.
    pub plane_band_enabled: bool,
    /// Whether a prior scene-image resource remains selected.
    pub loaded_scene_resource: bool,
    /// PBM palette-refresh option reset on the non-sequence tail.
    pub pbm_palette_refresh: bool,
    /// PBM transparent-zero option reset on the non-sequence tail.
    pub pbm_transparent_zero: bool,
}

impl Default for AlienOverlayCycleState {
    fn default() -> Self {
        Self {
            trigger_flags: u8::MIN,
            overlay_armed: false,
            next_overlay: AlienXdbKind::Amer,
            shared: AlienOverlaySharedState::default(),
            sound_loader_flags: u16::MIN,
            sound_driver_pending: false,
            viewport: AlienOverlayViewport::ORIGINAL,
            mouse_idle_frames: u16::MIN,
            palette_dirty: false,
            plane_band_enabled: true,
            loaded_scene_resource: false,
            pbm_palette_refresh: false,
            pbm_transparent_zero: false,
        }
    }
}

/// Graphics tail selected after callback-mutated sequence flags are reread.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienOverlayGraphicsTail {
    /// Preserve the active ship sequence and clear its back buffer.
    Sequence,
    /// Reinitialize the back buffer and restore the current scene PBM.
    SceneImage,
}

/// Observable result of one overlay-cycle request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienOverlayCycleOutcome {
    /// Trigger bit zero was not set.
    Inactive,
    /// One overlay ran and bridge state was restored.
    Ran {
        /// Overlay chosen from the pre-increment cycle phase.
        overlay: AlienXdbKind,
        /// Graphics tail chosen after the overlay returned.
        tail: AlienOverlayGraphicsTail,
    },
}

/// Resource, audio, overlay, and graphics boundaries called by the coordinator.
pub trait AlienOverlayCycleHost {
    /// Opaque sound-header state restored after the bridge bank reload.
    type SoundHeader;
    /// Host failure returned without inventing fallback game behavior.
    type Error;

    /// Load the selected original alien XDB into owned host storage.
    fn load_alien_overlay(&mut self, overlay: AlienXdbKind) -> Result<(), Self::Error>;

    /// Capture the currently active sound-header state.
    fn capture_sound_header(&mut self) -> Self::SoundHeader;

    /// Load one original sound archive and publish its loader flags.
    fn load_sound_bank(
        &mut self,
        bank: AlienOverlaySoundBank,
        loader_flags: &mut u16,
    ) -> Result<(), Self::Error>;

    /// Start the original CD track used during the 3D encounter.
    fn start_cd_audio(&mut self) -> Result<(), Self::Error>;

    /// Run the decoded overlay with direct access to its intended shared state.
    fn run_alien_overlay(
        &mut self,
        overlay: AlienXdbKind,
        shared: &mut AlienOverlaySharedState,
    ) -> Result<(), Self::Error>;

    /// Stop the encounter CD track.
    fn stop_cd_audio(&mut self) -> Result<(), Self::Error>;

    /// Restore the sound header captured before the temporary bank load.
    fn restore_sound_header(&mut self, header: Self::SoundHeader);

    /// Reload the MANU3 hand model into owned decoded storage.
    fn reload_manu3(&mut self) -> Result<(), Self::Error>;

    /// Clear the native transition row after restoring MANU3.
    fn clear_transition_row(&mut self);

    /// Clear the active sequence back buffer while band composition is disabled.
    fn clear_sequence_back_buffer(&mut self);

    /// Initialize the ordinary bridge back buffer.
    fn initialize_back_buffer(&mut self);

    /// Reload the current scene image into the initialized back buffer.
    fn reload_scene_image(&mut self) -> Result<(), Self::Error>;
}

/// Run BLOODPRG routine `0x00B591` over typed owned state.
///
/// The original stable overlay-buffer pointer is replaced by independent owned
/// XDB and MANU3 loads. Callback-mutated timing, sequence, and mouse state remain
/// explicit, and every observable audio/resource/graphics call retains native
/// order.
pub fn run_alien_overlay_cycle<H: AlienOverlayCycleHost>(
    state: &mut AlienOverlayCycleState,
    host: &mut H,
) -> Result<AlienOverlayCycleOutcome, H::Error> {
    if state.trigger_flags & ALIEN_OVERLAY_TRIGGER == u8::MIN {
        return Ok(AlienOverlayCycleOutcome::Inactive);
    }

    state.trigger_flags = u8::MIN;
    state.overlay_armed = false;
    let saved_mouse_position = state.shared.mouse_position;
    let overlay = state.next_overlay;
    state.next_overlay = next_overlay(overlay);

    host.load_alien_overlay(overlay)?;
    let saved_sound_header = host.capture_sound_header();
    host.load_sound_bank(
        AlienOverlaySoundBank::AlienScene,
        &mut state.sound_loader_flags,
    )?;

    let saved_loader_flags = state.sound_loader_flags;
    state.sound_driver_pending = false;
    host.start_cd_audio()?;
    host.run_alien_overlay(overlay, &mut state.shared)?;
    host.stop_cd_audio()?;
    state.sound_loader_flags = saved_loader_flags;

    host.load_sound_bank(AlienOverlaySoundBank::Bridge, &mut state.sound_loader_flags)?;
    host.restore_sound_header(saved_sound_header);
    host.reload_manu3()?;
    host.clear_transition_row();

    state.viewport = AlienOverlayViewport::ORIGINAL;
    state.mouse_idle_frames = u16::MIN;
    state.palette_dirty = true;
    state.shared.mouse_position = saved_mouse_position;

    let tail = if state.shared.sequence_flags & SHIP_SEQUENCE_ACTIVE != u16::MIN {
        state.plane_band_enabled = false;
        host.clear_sequence_back_buffer();
        state.plane_band_enabled = true;
        state.loaded_scene_resource = false;
        AlienOverlayGraphicsTail::Sequence
    } else {
        host.initialize_back_buffer();
        state.pbm_palette_refresh = false;
        state.pbm_transparent_zero = false;
        host.reload_scene_image()?;
        AlienOverlayGraphicsTail::SceneImage
    };

    Ok(AlienOverlayCycleOutcome::Ran { overlay, tail })
}

fn next_overlay(overlay: AlienXdbKind) -> AlienXdbKind {
    match overlay {
        AlienXdbKind::Amer => AlienXdbKind::Croolis,
        AlienXdbKind::Croolis => AlienXdbKind::Scrut,
        AlienXdbKind::Scrut => AlienXdbKind::Amer,
    }
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct CycleVector {
        name: String,
        trigger: u8,
        phase_before: u8,
        phase_after: u8,
        vbio_timing_before: u16,
        vbio_timing_after: u16,
        sequence_before: u16,
        sequence_after_callbacks: u16,
        tail: String,
        calls: Vec<CallVector>,
    }

    #[derive(Deserialize)]
    struct CallVector {
        call: String,
    }

    #[derive(Default)]
    struct RecordingHost {
        calls: Vec<&'static str>,
        callback_timing: u16,
        callback_sequence: u16,
    }

    impl AlienOverlayCycleHost for RecordingHost {
        type SoundHeader = u32;
        type Error = Infallible;

        fn load_alien_overlay(&mut self, _overlay: AlienXdbKind) -> Result<(), Self::Error> {
            self.calls.push("resource_file_load_overlay");
            Ok(())
        }

        fn capture_sound_header(&mut self) -> Self::SoundHeader {
            0
        }

        fn load_sound_bank(
            &mut self,
            bank: AlienOverlaySoundBank,
            loader_flags: &mut u16,
        ) -> Result<(), Self::Error> {
            match bank {
                AlienOverlaySoundBank::AlienScene => {
                    self.calls.push("snd_bank_loader_3d");
                    *loader_flags = loader_flags.wrapping_add(1);
                }
                AlienOverlaySoundBank::Bridge => self.calls.push("snd_bank_loader_tb"),
            }
            Ok(())
        }

        fn start_cd_audio(&mut self) -> Result<(), Self::Error> {
            self.calls.push("cdrom_audio_play_track_2");
            Ok(())
        }

        fn run_alien_overlay(
            &mut self,
            _overlay: AlienXdbKind,
            shared: &mut AlienOverlaySharedState,
        ) -> Result<(), Self::Error> {
            self.calls.push("alien_overlay_entry");
            shared.timing_scale = self.callback_timing;
            shared.sequence_flags = self.callback_sequence;
            shared.mouse_position = [i16::MIN, i16::MAX];
            Ok(())
        }

        fn stop_cd_audio(&mut self) -> Result<(), Self::Error> {
            self.calls.push("cdrom_audio_stop");
            Ok(())
        }

        fn restore_sound_header(&mut self, _header: Self::SoundHeader) {}

        fn reload_manu3(&mut self) -> Result<(), Self::Error> {
            self.calls.push("resource_file_load_manu3");
            Ok(())
        }

        fn clear_transition_row(&mut self) {
            self.calls.push("blit_fill_row_5221");
        }

        fn clear_sequence_back_buffer(&mut self) {
            self.calls.push("backbuffer_clear_flags");
        }

        fn initialize_back_buffer(&mut self) {
            self.calls.push("back_buffer_init");
        }

        fn reload_scene_image(&mut self) -> Result<(), Self::Error> {
            self.calls.push("pbm_image_load_and_decode");
            Ok(())
        }
    }

    #[test]
    fn cycle_matches_every_original_coordinator_vector() {
        let vectors: Vec<CycleVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b591_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 12);
        for vector in vectors {
            let mouse_position = [1_234, -2_345];
            let mut state = AlienOverlayCycleState {
                trigger_flags: vector.trigger,
                overlay_armed: true,
                next_overlay: overlay(vector.phase_before),
                shared: AlienOverlaySharedState {
                    timing_scale: vector.vbio_timing_before,
                    sequence_flags: vector.sequence_before,
                    mouse_position,
                },
                sound_loader_flags: 50_000,
                sound_driver_pending: true,
                viewport: AlienOverlayViewport {
                    width: 1,
                    height: 1,
                    ..AlienOverlayViewport::ORIGINAL
                },
                mouse_idle_frames: u16::MAX,
                palette_dirty: false,
                plane_band_enabled: true,
                loaded_scene_resource: true,
                pbm_palette_refresh: true,
                pbm_transparent_zero: true,
            };
            let mut host = RecordingHost {
                callback_timing: vector.vbio_timing_after,
                callback_sequence: vector.sequence_after_callbacks,
                ..RecordingHost::default()
            };
            let outcome = run_alien_overlay_cycle(&mut state, &mut host).unwrap();

            assert_eq!(
                state.next_overlay,
                overlay(vector.phase_after),
                "{}",
                vector.name
            );
            assert_eq!(
                state.shared.timing_scale, vector.vbio_timing_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.shared.sequence_flags, vector.sequence_after_callbacks,
                "{}",
                vector.name
            );
            let expected_calls = vector
                .calls
                .iter()
                .map(|call| call.call.as_str())
                .collect::<Vec<_>>();
            assert_eq!(host.calls, expected_calls, "{}", vector.name);

            if vector.tail == "inactive" {
                assert_eq!(
                    outcome,
                    AlienOverlayCycleOutcome::Inactive,
                    "{}",
                    vector.name
                );
                assert_eq!(state.trigger_flags, vector.trigger, "{}", vector.name);
                continue;
            }

            assert_eq!(state.trigger_flags, u8::MIN, "{}", vector.name);
            assert!(!state.overlay_armed, "{}", vector.name);
            assert_eq!(
                state.shared.mouse_position, mouse_position,
                "{}",
                vector.name
            );
            assert_eq!(
                state.viewport,
                AlienOverlayViewport::ORIGINAL,
                "{}",
                vector.name
            );
            assert_eq!(state.mouse_idle_frames, u16::MIN, "{}", vector.name);
            assert!(state.palette_dirty, "{}", vector.name);
            assert_eq!(
                outcome,
                AlienOverlayCycleOutcome::Ran {
                    overlay: overlay(vector.phase_before),
                    tail: if vector.tail == "sequence" {
                        AlienOverlayGraphicsTail::Sequence
                    } else {
                        AlienOverlayGraphicsTail::SceneImage
                    },
                },
                "{}",
                vector.name
            );
        }
    }

    fn overlay(phase: u8) -> AlienXdbKind {
        match phase {
            0 => AlienXdbKind::Amer,
            1 => AlienXdbKind::Croolis,
            2 => AlienXdbKind::Scrut,
            _ => panic!("invalid original overlay phase {phase}"),
        }
    }
}
