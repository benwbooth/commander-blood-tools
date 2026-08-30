//! Synchronous flat-runtime host for the recovered interactive alien overlays.

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::{AlienAsset, AlienXdbKind};

use crate::native::alien::{AlienMouseSample, AlienSceneFrame, AlienSceneRuntime};
use crate::native::bloodprg::{
    AlienOverlayCycleHost, AlienOverlayCycleOutcome, AlienOverlayCycleState,
    AlienOverlaySharedState, AlienOverlaySoundBank, GameLifecycleState, LoadedSoundBank,
    PointerButtons, run_alien_overlay_cycle,
};

use super::{ModernGameServices, RuntimeAssetLoadStatus, RuntimePlatformHost};

const ALIEN_DRIVER_WIDTH: u32 = 640;
const ALIEN_DRIVER_HEIGHT: u32 = 1_024;
const ALIEN_OVERLAY_TRIGGER: u8 = 1;
const SHIP_SEQUENCE_ACTIVE: u16 = 1;
const ALIEN_SOUND_BANK_NAME: &[u8] = b"3D.snd";
const BRIDGE_SOUND_BANK_NAME: &[u8] = b"tb.snd";

/// Mouse and keyboard input consumed by one recovered alien main-loop frame.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeAlienOverlayFrameInput {
    /// Mouse sample in the original XDB driver's 640-by-1024 coordinate range.
    pub mouse: AlienMouseSample,
    /// BIOS-compatible key words in SDL arrival order.
    pub key_events: Vec<u16>,
}

impl RuntimeAlienOverlayFrameInput {
    /// Publish one virtual XDB-driver pointer without moving the host cursor.
    pub fn from_driver_pointer(
        position: [f32; 2],
        buttons: PointerButtons,
        key_events: Vec<u16>,
    ) -> Self {
        Self {
            mouse: AlienMouseSample {
                x: position[0].clamp(0.0, ALIEN_DRIVER_WIDTH as f32) as u16,
                y: position[1].clamp(0.0, ALIEN_DRIVER_HEIGHT as f32) as u16,
                buttons: buttons.bits(),
            },
            key_events,
        }
    }
}

/// State returned after one alien XDB main loop exits.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RuntimeAlienOverlayOutcome {
    /// Timing word normalized by the overlay API entry and returned on exit.
    pub timing_scale: u16,
    /// Wrapping timer value retained for the next overlay invocation.
    pub frame_clock: u32,
    /// Number of full 3D frames submitted during this invocation.
    pub presented_frames: u64,
}

/// Platform services needed while BLOODPRG is synchronously inside an alien XDB.
pub trait RuntimeAlienOverlayFrameHost {
    /// Install GPU resources for the selected decoded overlay.
    fn begin_overlay(&mut self, asset: &AlienAsset) -> Result<()>;
    /// Pump SDL and return input for the next recovered overlay frame.
    fn poll_frame(&mut self) -> Result<RuntimeAlienOverlayFrameInput>;
    /// Submit one recovered 3D frame before its sound callback runs.
    fn present_frame(&mut self, frame: &AlienSceneFrame) -> Result<()>;
    /// Play one clip from the temporary `SN\3D.SND` bank.
    fn play_sound_clip(&mut self, clip_index: u16, clock: u32) -> Result<()>;
    /// Pace an overlay frame using the same recovered PIT-derived budget as the game.
    fn pace_frame(&mut self) -> Result<()>;
    /// Release temporary GPU state even when frame processing fails.
    fn finish_overlay(&mut self) -> Result<()>;
}

/// Run one decoded AMER, CROOLIS, or SCRUT main loop to completion.
pub fn run_runtime_alien_overlay<Host: RuntimeAlienOverlayFrameHost>(
    asset: AlienAsset,
    timing_scale: u16,
    frame_clock: u32,
    host: &mut Host,
) -> Result<RuntimeAlienOverlayOutcome> {
    host.begin_overlay(&asset)?;
    let mut runtime = AlienSceneRuntime::enter(asset, timing_scale, frame_clock);
    let run_result = (|| {
        let mut presented_frames = u64::MIN;
        while runtime.is_running() {
            let input = host.poll_frame()?;
            let step = runtime
                .step(input.mouse, &input.key_events)
                .context("advancing the recovered alien XDB main loop")?;
            if let Some(frame) = step.frame.as_ref() {
                host.present_frame(frame)?;
                presented_frames = presented_frames.wrapping_add(1);
            }
            if let Some(callback) = step.callback {
                host.play_sound_clip(callback.event, callback.clock)?;
            }
            host.pace_frame()?;
        }
        Ok(RuntimeAlienOverlayOutcome {
            timing_scale: runtime.timing_scale(),
            frame_clock: runtime.frame_clock(),
            presented_frames,
        })
    })();
    let finish_result = host.finish_overlay();
    match (run_result, finish_result) {
        (Ok(outcome), Ok(())) => Ok(outcome),
        (Err(error), Ok(())) => Err(error),
        (Ok(_), Err(error)) => Err(error.context("releasing alien-overlay GPU resources")),
        (Err(error), Err(finish_error)) => Err(error.context(format!(
            "alien-overlay cleanup also failed: {finish_error:#}"
        ))),
    }
}

/// Persistent round-robin phase and timer state for BLOODPRG's overlay coordinator.
#[derive(Default)]
pub struct RuntimeAlienOverlayCycle {
    state: AlienOverlayCycleState,
    frame_clock: u32,
}

impl RuntimeAlienOverlayCycle {
    /// Borrow the recovered coordinator state for diagnostics and regression tests.
    pub const fn state(&self) -> &AlienOverlayCycleState {
        &self.state
    }

    /// Consume and run a pending overlay, restoring bridge resources before returning.
    pub fn run<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        lifecycle: &mut GameLifecycleState,
        platform: &mut RuntimePlatformHost<'window>,
    ) -> Result<AlienOverlayCycleOutcome> {
        let (overlay_armed, trigger_pending) = services.alien_overlay_flags()?;
        self.state.trigger_flags = u8::from(trigger_pending) * ALIEN_OVERLAY_TRIGGER;
        self.state.overlay_armed = overlay_armed;
        self.state.shared.sequence_flags =
            u16::from(lifecycle.presentation.sequence_active) * SHIP_SEQUENCE_ACTIVE;
        self.state.shared.mouse_position = services.input().pointer_sample().position;

        if !trigger_pending {
            return Ok(AlienOverlayCycleOutcome::Inactive);
        }
        self.state.shared.timing_scale = services.read_alien_timing_scale()?;

        let result = {
            let mut host = RuntimeAlienOverlayCycleBackend {
                services,
                platform,
                loaded_asset: None,
                frame_clock: &mut self.frame_clock,
            };
            run_alien_overlay_cycle(&mut self.state, &mut host)
        };

        services.set_alien_overlay_flags(self.state.overlay_armed, false)?;
        services.write_alien_timing_scale(self.state.shared.timing_scale)?;
        lifecycle.presentation.sequence_active =
            self.state.shared.sequence_flags & SHIP_SEQUENCE_ACTIVE != u16::MIN;
        if matches!(&result, Ok(AlienOverlayCycleOutcome::Ran { .. })) {
            services.publish_alien_overlay_bridge_restoration();
        }
        result
    }
}

struct RuntimeAlienOverlayCycleBackend<'services, 'window, 'platform, 'clock> {
    services: &'services mut ModernGameServices<'window>,
    platform: &'platform mut RuntimePlatformHost<'window>,
    loaded_asset: Option<(AlienXdbKind, AlienAsset)>,
    frame_clock: &'clock mut u32,
}

impl AlienOverlayCycleHost for RuntimeAlienOverlayCycleBackend<'_, '_, '_, '_> {
    type SoundHeader = LoadedSoundBank;
    type Error = anyhow::Error;

    fn load_alien_overlay(&mut self, overlay: AlienXdbKind) -> Result<()> {
        let asset = self.services.runtime().load_alien_overlay(overlay)?;
        self.loaded_asset = Some((overlay, asset));
        Ok(())
    }

    fn capture_sound_header(&mut self) -> Result<Self::SoundHeader> {
        self.services
            .resident_sound_bank()
            .cloned()
            .context("alien overlay started without a resident sound bank")
    }

    fn load_sound_bank(
        &mut self,
        bank: AlienOverlaySoundBank,
        _loader_flags: &mut u16,
    ) -> Result<()> {
        let name = match bank {
            AlienOverlaySoundBank::AlienScene => ALIEN_SOUND_BANK_NAME,
            AlienOverlaySoundBank::Bridge => BRIDGE_SOUND_BANK_NAME,
        };
        self.services
            .load_resident_sound_bank_resource(name)
            .with_context(|| {
                format!(
                    "loading temporary sound bank {}",
                    String::from_utf8_lossy(name)
                )
            })
    }

    fn start_cd_audio(&mut self) -> Result<()> {
        self.services.start_encounter_cd_audio()
    }

    fn run_alien_overlay(
        &mut self,
        overlay: AlienXdbKind,
        shared: &mut AlienOverlaySharedState,
    ) -> Result<()> {
        let (loaded_kind, asset) = self
            .loaded_asset
            .take()
            .context("alien overlay entry called before its XDB was decoded")?;
        if loaded_kind != overlay {
            bail!("decoded {loaded_kind:?} overlay but coordinator selected {overlay:?}");
        }
        let mut host = LiveAlienOverlayFrameHost {
            services: self.services,
            platform: self.platform,
        };
        let outcome =
            run_runtime_alien_overlay(asset, shared.timing_scale, *self.frame_clock, &mut host)?;
        shared.timing_scale = outcome.timing_scale;
        *self.frame_clock = outcome.frame_clock;
        Ok(())
    }

    fn stop_cd_audio(&mut self) -> Result<()> {
        self.services.stop_encounter_cd_audio()
    }

    fn restore_sound_header(&mut self, header: Self::SoundHeader) -> Result<()> {
        self.services.restore_resident_sound_bank(header);
        Ok(())
    }

    fn reload_manu3(&mut self) -> Result<()> {
        match self.services.load_manu3_overlay()? {
            RuntimeAssetLoadStatus::LoadedNow | RuntimeAssetLoadStatus::AlreadyLoaded => Ok(()),
        }
    }

    fn clear_transition_row(&mut self) -> Result<()> {
        self.services.clear_alien_overlay_transition_frame()
    }

    fn clear_sequence_back_buffer(&mut self) -> Result<()> {
        self.services.restore_sequence_back_buffer()
    }

    fn initialize_back_buffer(&mut self) -> Result<()> {
        self.services.initialize_back_buffer().map(|_| ())
    }

    fn reload_scene_image(&mut self) -> Result<()> {
        self.services.reload_current_scene_image()
    }
}

struct LiveAlienOverlayFrameHost<'services, 'window, 'platform> {
    services: &'services mut ModernGameServices<'window>,
    platform: &'platform mut RuntimePlatformHost<'window>,
}

impl RuntimeAlienOverlayFrameHost for LiveAlienOverlayFrameHost<'_, '_, '_> {
    fn begin_overlay(&mut self, asset: &AlienAsset) -> Result<()> {
        self.services.begin_alien_overlay(asset)?;
        if let Err(error) = self.platform.begin_alien_overlay_input() {
            self.services.finish_alien_overlay();
            return Err(error);
        }
        Ok(())
    }

    fn poll_frame(&mut self) -> Result<RuntimeAlienOverlayFrameInput> {
        self.platform.poll_alien_overlay_frame(self.services)
    }

    fn present_frame(&mut self, frame: &AlienSceneFrame) -> Result<()> {
        self.services.present_alien_overlay_frame(frame)
    }

    fn play_sound_clip(&mut self, clip_index: u16, _clock: u32) -> Result<()> {
        let clip_index = u8::try_from(clip_index)
            .context("alien-overlay sound callback index exceeds the SND table")?;
        self.services.play_loaded_sound_bank_clip(clip_index)
    }

    fn pace_frame(&mut self) -> Result<()> {
        self.platform.pace_frame()
    }

    fn finish_overlay(&mut self) -> Result<()> {
        let input_released = self.platform.finish_alien_overlay_input();
        let renderer_released = self.services.finish_alien_overlay();
        if !input_released {
            bail!("alien-overlay virtual pointer was not active");
        }
        if !renderer_released {
            bail!("alien-overlay renderer was not installed");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::decode_alien_xdb;

    use crate::native::bloodprg::PointerButtons;
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    use super::*;

    #[test]
    fn virtual_pointer_clamps_to_the_original_alien_driver_domain() {
        let center = RuntimeAlienOverlayFrameInput::from_driver_pointer(
            [320.0, 512.0],
            PointerButtons::NONE,
            vec![0x4800],
        );
        assert_eq!(center.mouse.x, 320);
        assert_eq!(center.mouse.y, 512);
        assert_eq!(center.key_events, [0x4800]);

        let maximum = RuntimeAlienOverlayFrameInput::from_driver_pointer(
            [f32::MAX, f32::MAX],
            PointerButtons::from_bits(3),
            Vec::new(),
        );
        assert_eq!(maximum.mouse.x, 640);
        assert_eq!(maximum.mouse.y, 1_024);
        assert_eq!(maximum.mouse.buttons, 3);
    }

    #[derive(Default)]
    struct ExitAfterFirstFrameHost {
        selected_kind: Option<AlienXdbKind>,
        poll_count: usize,
        presented_frames: usize,
        pace_count: usize,
        finish_count: usize,
    }

    const DRIVEN_FRAME_COUNT: usize = 1_000;
    const DRIVEN_CLICK_PERIOD: usize = 4;
    const DRIVEN_KEY_PERIOD: usize = 3;
    const DRIVEN_TIMING_SCALE: u16 = 10;
    const DRIVEN_INITIAL_CLOCK: u32 = 1_000;
    const CENTERED_MOUSE_X: u16 = 320;
    const CENTERED_MOUSE_Y: u16 = 512;
    const BIOS_CURSOR_UP: u16 = 0x4800;
    const BIOS_CURSOR_DOWN: u16 = 0x5000;
    const BIOS_SPACE: u16 = 0x3920;
    const BIOS_ESCAPE: u16 = 0x011b;

    struct DrivenAlienOverlayHost {
        poll_count: usize,
        presented_frames: usize,
        sound_callbacks: usize,
        pace_count: usize,
        finish_count: usize,
    }

    impl DrivenAlienOverlayHost {
        fn new() -> Self {
            Self {
                poll_count: usize::MIN,
                presented_frames: usize::MIN,
                sound_callbacks: usize::MIN,
                pace_count: usize::MIN,
                finish_count: usize::MIN,
            }
        }
    }

    impl RuntimeAlienOverlayFrameHost for DrivenAlienOverlayHost {
        fn begin_overlay(&mut self, _asset: &AlienAsset) -> Result<()> {
            Ok(())
        }

        fn poll_frame(&mut self) -> Result<RuntimeAlienOverlayFrameInput> {
            self.poll_count += 1;
            let key = if self.poll_count >= DRIVEN_FRAME_COUNT {
                BIOS_ESCAPE
            } else {
                match self.poll_count % DRIVEN_KEY_PERIOD {
                    0 => BIOS_CURSOR_UP,
                    1 => BIOS_CURSOR_DOWN,
                    _ => BIOS_SPACE,
                }
            };
            Ok(RuntimeAlienOverlayFrameInput {
                mouse: AlienMouseSample {
                    x: CENTERED_MOUSE_X,
                    y: CENTERED_MOUSE_Y,
                    buttons: u16::from(self.poll_count % DRIVEN_CLICK_PERIOD == usize::MIN),
                },
                key_events: vec![key],
            })
        }

        fn present_frame(&mut self, _frame: &AlienSceneFrame) -> Result<()> {
            self.presented_frames += 1;
            Ok(())
        }

        fn play_sound_clip(&mut self, _clip_index: u16, _clock: u32) -> Result<()> {
            self.sound_callbacks += 1;
            Ok(())
        }

        fn pace_frame(&mut self) -> Result<()> {
            self.pace_count += 1;
            Ok(())
        }

        fn finish_overlay(&mut self) -> Result<()> {
            self.finish_count += 1;
            Ok(())
        }
    }

    impl RuntimeAlienOverlayFrameHost for ExitAfterFirstFrameHost {
        fn begin_overlay(&mut self, asset: &AlienAsset) -> Result<()> {
            self.selected_kind = Some(asset.kind);
            Ok(())
        }

        fn poll_frame(&mut self) -> Result<RuntimeAlienOverlayFrameInput> {
            self.poll_count += 1;
            Ok(RuntimeAlienOverlayFrameInput {
                mouse: AlienMouseSample {
                    x: 320,
                    y: 512,
                    buttons: u16::MIN,
                },
                key_events: vec![0x011b],
            })
        }

        fn present_frame(&mut self, _frame: &AlienSceneFrame) -> Result<()> {
            self.presented_frames += 1;
            Ok(())
        }

        fn play_sound_clip(&mut self, _clip_index: u16, _clock: u32) -> Result<()> {
            bail!("first-frame Escape must return before an alien sound callback")
        }

        fn pace_frame(&mut self) -> Result<()> {
            self.pace_count += 1;
            Ok(())
        }

        fn finish_overlay(&mut self) -> Result<()> {
            self.finish_count += 1;
            Ok(())
        }
    }

    #[test]
    fn shipped_amer_overlay_runs_and_cleans_up_through_the_flat_frame_host() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(
            paths,
            std::env::temp_dir().join("commander-blood-alien-overlay-test"),
        )
        .unwrap();
        let encoded = data.load_named_resource(b"AMER.XDB").unwrap();
        let asset = decode_alien_xdb(&encoded, AlienXdbKind::Amer).unwrap();
        let mut host = ExitAfterFirstFrameHost::default();

        let outcome = run_runtime_alien_overlay(asset, 10, 1_000, &mut host).unwrap();

        assert_eq!(host.selected_kind, Some(AlienXdbKind::Amer));
        assert_eq!(host.poll_count, 1);
        assert_eq!(host.presented_frames, 1);
        assert_eq!(host.pace_count, 1);
        assert_eq!(host.finish_count, 1);
        assert_eq!(outcome.presented_frames, 1);
        assert_eq!(outcome.frame_clock, 1_008);
    }

    #[test]
    fn all_shipped_alien_overlays_survive_a_driven_production_runtime_campaign() {
        let Ok(paths) = OriginalGameDataPaths::discover(None) else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(
            paths,
            std::env::temp_dir().join("commander-blood-alien-campaign-test"),
        )
        .unwrap();
        let cases = [
            (b"AMER.XDB".as_slice(), AlienXdbKind::Amer),
            (b"CROOLIS.XDB".as_slice(), AlienXdbKind::Croolis),
            (b"SCRUT.XDB".as_slice(), AlienXdbKind::Scrut),
        ];

        for (filename, kind) in cases {
            let encoded = data.load_named_resource(filename).unwrap();
            let asset = decode_alien_xdb(&encoded, kind).unwrap();
            let mut host = DrivenAlienOverlayHost::new();

            let outcome = run_runtime_alien_overlay(
                asset,
                DRIVEN_TIMING_SCALE,
                DRIVEN_INITIAL_CLOCK,
                &mut host,
            )
            .unwrap();

            assert_eq!(host.poll_count, DRIVEN_FRAME_COUNT);
            assert_eq!(host.presented_frames, DRIVEN_FRAME_COUNT);
            assert_eq!(host.pace_count, DRIVEN_FRAME_COUNT);
            assert_eq!(host.finish_count, 1);
            assert!(host.sound_callbacks > usize::MIN);
            assert_eq!(outcome.presented_frames, DRIVEN_FRAME_COUNT as u64);
        }
    }
}
