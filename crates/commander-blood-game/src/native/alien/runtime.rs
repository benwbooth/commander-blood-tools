//! Flat host coordinator for the three recovered alien-overlay entry loops.

use commander_blood_formats::alien::AlienAsset;

use super::{AlienControlLatch, AlienMouseSample, AlienScene, AlienSceneError, AlienSceneFrame};

const TIMING_SCALE_SHIFT: u32 = 3;
const MAXIMUM_SCALED_TIMING: u16 = 127;
const METHOD_DELTA_BIAS: u16 = 4;
const FRAME_CLOCK_ADVANCE: u32 = 8;
const INITIAL_CALLBACK_AGE: u32 = 620;
const IDLE_CALLBACK_INTERVAL: u32 = 600;
const IDLE_CALLBACK_BACKDATE: u32 = 1_000;
const CONTROL_CALLBACK_EVENT: u16 = 2;
const ESCAPE_CHARACTER: u8 = 27;
const PAUSE_CHARACTER_LOWER: u8 = b'p';
const PAUSE_CHARACTER_UPPER: u8 = b'P';

/// One callback emitted by an alien overlay after rendering a frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienFrameCallback {
    /// Original callback event word.
    pub event: u16,
    /// Wrapping 32-bit overlay clock at the callback point.
    pub clock: u32,
}

/// Lifecycle state after one modern alien-overlay host pass.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AlienRuntimeStatus {
    /// The native pause loop is waiting for another `P` key.
    Paused,
    /// Another frame may run.
    Running,
    /// The scene had already stopped before this host pass.
    Stopped,
}

/// Result of one modern host pass through an alien overlay's native main loop.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienRuntimeStep {
    /// Render-facing output, absent while paused or after a prior stop.
    pub frame: Option<AlienSceneFrame>,
    /// At most one callback is emitted by the native timer tail per frame.
    pub callback: Option<AlienFrameCallback>,
    /// Overlay lifecycle state after processing the supplied input.
    pub status: AlienRuntimeStatus,
}

/// Flat replacement for the AMER, CROOLIS, and SCRUT API-entry and main loops.
///
/// The decoded XDB owns ordinary Rust collections, so segment relocation,
/// renderer continuation patching, VGA page rotation, and DOS mouse bounds do
/// not survive into this type. Timing normalization, frame ordering, callback
/// cadence, keyboard draining, pause behavior, and the mutable scene state are
/// retained because they are observable game rules.
#[derive(Clone, Debug)]
pub struct AlienSceneRuntime {
    scene: AlienScene,
    frame_clock: u32,
    last_callback_clock: u32,
    running: bool,
    paused: bool,
}

impl AlienSceneRuntime {
    /// Enter one decoded overlay through the behavior shared by native routine
    /// `0x000000` in AMER, CROOLIS, and SCRUT.
    ///
    /// `frame_clock` is supplied by the owning game runtime just as the native
    /// segment directory supplied it to each overlay. The returned runtime has
    /// no code, object, palette, raster, or framebuffer segment identities.
    pub fn enter(asset: AlienAsset, timing_scale: u16, frame_clock: u32) -> Self {
        let mut scene = AlienScene::from_asset(asset);
        scene.callback_state.method_delta = method_delta_from_timing_scale(timing_scale);
        Self {
            scene,
            frame_clock,
            last_callback_clock: frame_clock.wrapping_sub(INITIAL_CALLBACK_AGE),
            running: true,
            paused: false,
        }
    }

    /// Advance the recovered alien `main` routine by one rendered frame.
    ///
    /// This is the semantic port of native routine `0x0000A3` in all three
    /// overlays. `key_events` are BIOS-compatible words in arrival order. They
    /// are drained after rendering and callback dispatch, matching the binary.
    pub fn step(
        &mut self,
        mouse: AlienMouseSample,
        key_events: &[u16],
    ) -> Result<AlienRuntimeStep, AlienSceneError> {
        if !self.running {
            return Ok(AlienRuntimeStep {
                frame: None,
                callback: None,
                status: AlienRuntimeStatus::Stopped,
            });
        }
        if self.paused {
            if key_events
                .iter()
                .copied()
                .any(|key| is_pause_character(key as u8))
            {
                self.paused = false;
            }
            return Ok(AlienRuntimeStep {
                frame: None,
                callback: None,
                status: if self.paused {
                    AlienRuntimeStatus::Paused
                } else {
                    AlienRuntimeStatus::Running
                },
            });
        }

        let frame = self.scene.step(mouse)?;
        if self.scene.exit_requested() {
            self.running = false;
            return Ok(AlienRuntimeStep {
                frame: Some(frame),
                callback: None,
                status: AlienRuntimeStatus::Stopped,
            });
        }

        let callback = self.advance_callback_clock();
        self.drain_keyboard(key_events);
        Ok(AlienRuntimeStep {
            frame: Some(frame),
            callback,
            status: if !self.running {
                AlienRuntimeStatus::Stopped
            } else if self.paused {
                AlienRuntimeStatus::Paused
            } else {
                AlienRuntimeStatus::Running
            },
        })
    }

    /// Current mutable scene owned by the overlay runtime.
    pub fn scene(&self) -> &AlienScene {
        &self.scene
    }

    /// Current mutable scene owned by the overlay runtime.
    pub fn scene_mut(&mut self) -> &mut AlienScene {
        &mut self.scene
    }

    /// Current wrapping frame clock published to the game callback.
    pub fn frame_clock(&self) -> u32 {
        self.frame_clock
    }

    /// Most recent callback or native throttle reference clock.
    pub fn last_callback_clock(&self) -> u32 {
        self.last_callback_clock
    }

    /// Timing-scale word returned by the native API entry after `main` exits.
    pub fn timing_scale(&self) -> u16 {
        timing_scale_from_method_delta(self.scene.callback_state.method_delta)
    }

    /// Whether the overlay remains inside its native main loop.
    pub fn is_running(&self) -> bool {
        self.running
    }

    /// Whether keyboard processing is waiting for a second `P` key.
    pub fn is_paused(&self) -> bool {
        self.paused
    }

    fn advance_callback_clock(&mut self) -> Option<AlienFrameCallback> {
        advance_callback_clock(
            &mut self.frame_clock,
            &mut self.last_callback_clock,
            &mut self.scene.callback_state.callback_countdown,
            self.scene.callback_state.control_latch != AlienControlLatch::Inactive,
        )
    }

    fn drain_keyboard(&mut self, key_events: &[u16]) {
        drain_keyboard(
            &mut self.running,
            &mut self.paused,
            &mut self.scene.control.key_event,
            key_events,
        );
    }
}

fn advance_callback_clock(
    frame_clock: &mut u32,
    last_callback_clock: &mut u32,
    callback_countdown: &mut u16,
    control_active: bool,
) -> Option<AlienFrameCallback> {
    *frame_clock = frame_clock.wrapping_add(FRAME_CLOCK_ADVANCE);
    let event = callback_countdown.wrapping_sub(1);
    *callback_countdown = u16::MIN;

    if event as i16 >= 0 {
        *last_callback_clock = *frame_clock;
        return Some(AlienFrameCallback {
            event,
            clock: *frame_clock,
        });
    }

    if frame_clock.wrapping_sub(*last_callback_clock) < IDLE_CALLBACK_INTERVAL {
        return None;
    }

    *last_callback_clock = frame_clock.wrapping_sub(IDLE_CALLBACK_BACKDATE);
    if !control_active {
        return None;
    }

    *last_callback_clock = *frame_clock;
    Some(AlienFrameCallback {
        event: CONTROL_CALLBACK_EVENT,
        clock: *frame_clock,
    })
}

fn drain_keyboard(running: &mut bool, paused: &mut bool, key_event: &mut u16, key_events: &[u16]) {
    let mut keys = key_events.iter().copied();
    while let Some(key) = keys.next() {
        *key_event = key;
        let character = key as u8;
        if is_pause_character(character) {
            *paused = true;
            if keys.any(|key| is_pause_character(key as u8)) {
                *paused = false;
            }
            break;
        }
        if character == ESCAPE_CHARACTER {
            *running = false;
            break;
        }
    }
}

fn method_delta_from_timing_scale(timing_scale: u16) -> i16 {
    let shifted = timing_scale.wrapping_shl(TIMING_SCALE_SHIFT);
    let scaled = if (shifted as i16).is_negative() {
        u16::MIN
    } else {
        shifted.min(MAXIMUM_SCALED_TIMING)
    };
    scaled.wrapping_sub(METHOD_DELTA_BIAS) as i16
}

fn timing_scale_from_method_delta(method_delta: i16) -> u16 {
    (method_delta as u16).wrapping_add(METHOD_DELTA_BIAS) >> TIMING_SCALE_SHIFT
}

fn is_pause_character(character: u8) -> bool {
    matches!(character, PAUSE_CHARACTER_LOWER | PAUSE_CHARACTER_UPPER)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct ApiEntryVector {
        timing_scale_in: u16,
        entry_method_delta: u16,
        main_method_delta: Option<u16>,
        timing_scale_out: u16,
    }

    #[derive(Clone, Copy)]
    struct MainInput {
        name: &'static str,
        clock: u32,
        countdown: u16,
        frame_count: usize,
        exit_after_frame: Option<usize>,
        active_control_frames: u8,
    }

    #[derive(Deserialize)]
    struct MainVector {
        name: String,
        clock_after: u32,
        last_callback_after: u32,
        countdown_after: u16,
        callbacks: Vec<CallbackVector>,
    }

    #[derive(Deserialize)]
    struct CallbackVector {
        event: u16,
        clock: u32,
    }

    const MAIN_INPUTS: [MainInput; 8] = [
        MainInput {
            name: "exit_before_timer",
            clock: 305_419_896,
            countdown: 17_185,
            frame_count: 1,
            exit_after_frame: Some(1),
            active_control_frames: 0,
        },
        MainInput {
            name: "positive_countdown_escape",
            clock: 270_544_960,
            countdown: 3,
            frame_count: 1,
            exit_after_frame: None,
            active_control_frames: 0,
        },
        MainInput {
            name: "negative_adjusted_no_callback",
            clock: 1_432_778_632,
            countdown: 0,
            frame_count: 1,
            exit_after_frame: None,
            active_control_frames: 0,
        },
        MainInput {
            name: "negative_active_callback",
            clock: 2_309_737_967,
            countdown: 0,
            frame_count: 1,
            exit_after_frame: None,
            active_control_frames: 1,
        },
        MainInput {
            name: "countdown_and_clock_wrap",
            clock: 4_294_967_292,
            countdown: 32_768,
            frame_count: 1,
            exit_after_frame: None,
            active_control_frames: 0,
        },
        MainInput {
            name: "ordinary_key_drain",
            clock: 826_366_246,
            countdown: 0,
            frame_count: 2,
            exit_after_frame: Some(2),
            active_control_frames: 0,
        },
        MainInput {
            name: "pause_until_matching_key",
            clock: 655_894_552,
            countdown: 0,
            frame_count: 2,
            exit_after_frame: Some(2),
            active_control_frames: 0,
        },
        MainInput {
            name: "callback_then_throttle",
            clock: 16_909_060,
            countdown: 0,
            frame_count: 3,
            exit_after_frame: Some(3),
            active_control_frames: 1,
        },
    ];

    #[test]
    fn all_api_entry_timing_vectors_match() {
        for source in [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0000_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0000_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0000_natural.json"),
        ] {
            let vectors: Vec<ApiEntryVector> = serde_json::from_str(source).unwrap();
            assert_eq!(vectors.len(), 8);
            for vector in vectors {
                let delta = method_delta_from_timing_scale(vector.timing_scale_in);
                assert_eq!(delta as u16, vector.entry_method_delta);
                let returned_delta = vector.main_method_delta.unwrap_or(delta as u16) as i16;
                assert_eq!(
                    timing_scale_from_method_delta(returned_delta),
                    vector.timing_scale_out
                );
            }
        }
    }

    #[test]
    fn all_main_timer_vectors_match() {
        for source in [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_00a3_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_00a3_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_00a3_natural.json"),
        ] {
            let vectors: Vec<MainVector> = serde_json::from_str(source).unwrap();
            assert_eq!(vectors.len(), MAIN_INPUTS.len());
            for vector in vectors {
                let input = MAIN_INPUTS
                    .iter()
                    .find(|input| input.name == vector.name)
                    .unwrap();
                let mut frame_clock = input.clock;
                let mut last_callback_clock = frame_clock.wrapping_sub(INITIAL_CALLBACK_AGE);
                let mut countdown = input.countdown;
                let mut callbacks = Vec::new();
                for frame_index in 1..=input.frame_count {
                    if input.exit_after_frame == Some(frame_index) {
                        break;
                    }
                    let control_active =
                        input.active_control_frames & (1 << (frame_index - 1)) != 0;
                    if let Some(callback) = advance_callback_clock(
                        &mut frame_clock,
                        &mut last_callback_clock,
                        &mut countdown,
                        control_active,
                    ) {
                        callbacks.push(callback);
                    }
                }
                assert_eq!(frame_clock, vector.clock_after, "{}", vector.name);
                assert_eq!(
                    last_callback_clock, vector.last_callback_after,
                    "{}",
                    vector.name
                );
                assert_eq!(countdown, vector.countdown_after, "{}", vector.name);
                assert_eq!(
                    callbacks,
                    vector
                        .callbacks
                        .iter()
                        .map(|callback| AlienFrameCallback {
                            event: callback.event,
                            clock: callback.clock,
                        })
                        .collect::<Vec<_>>(),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn keyboard_drain_retains_native_pause_and_escape_rules() {
        let mut running = true;
        let mut paused = false;
        let mut key_event = u16::MIN;
        drain_keyboard(&mut running, &mut paused, &mut key_event, &[7_777]);
        assert_eq!(key_event, 7_777);
        assert!(running);

        drain_keyboard(
            &mut running,
            &mut paused,
            &mut key_event,
            &[6_512, 11_640, 6_480],
        );
        assert_eq!(key_event, 6_512);
        assert!(!paused);

        drain_keyboard(&mut running, &mut paused, &mut key_event, &[6_512]);
        assert!(paused);
        assert!([6_480].into_iter().any(|key| is_pause_character(key as u8)));
        paused = false;

        drain_keyboard(&mut running, &mut paused, &mut key_event, &[283]);
        assert!(!running);
        assert_eq!(key_event, 283);
    }
}
