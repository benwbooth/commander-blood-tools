//! Flat game-timer state and the recovered multi-rate countdown cadence.

use commander_blood_formats::instruction::ScriptTimerSlot;

use super::ScriptRuntime;

const CHATTER_TICK_MASK: u16 = 1;
const SUBTITLE_TICK_MASK: u16 = 3;
const DIALOGUE_TICK_MASK: u16 = 7;
const NAVIGATION_TICK_MASK: u16 = 15;
const PERIODIC_TICK_MASK: u16 = 31;
const GAME_SUBTICK_RELOAD: u16 = 25;
const SCRIPT_COUNTDOWN_SLOT_COUNT: u8 = 30;
const MOUSE_IDLE_COUNTER_HIGH_BYTE_MASK: u16 = 0xff00;

/// Semantic state of the two-bit PC-speaker pulse request.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SpeakerPulseState {
    /// Whether the native speaker gate is currently enabled.
    pub enabled: bool,
    /// Whether an enable/disable pulse sequence is still active.
    pub active: bool,
}

impl SpeakerPulseState {
    /// Request a complete enable-then-disable pulse sequence.
    pub fn request(&mut self) {
        self.active = true;
    }

    fn advance(&mut self) -> Option<SpeakerGateAction> {
        if !self.active {
            return None;
        }
        if self.enabled {
            self.enabled = false;
            self.active = false;
            Some(SpeakerGateAction::Disable)
        } else {
            self.enabled = true;
            Some(SpeakerGateAction::Enable)
        }
    }
}

/// Host audio operation produced at the slowest native timer cadence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpeakerGateAction {
    /// Enable the host replacement for the PC-speaker gate.
    Enable,
    /// Disable the host replacement for the PC-speaker gate.
    Disable,
}

/// Gameplay state formerly updated from DOS interrupt 8.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameTimerState {
    /// Whether the modern host should advance this timer.
    pub running: bool,
    /// Wrapping low timer word used by every cadence mask.
    pub tick: u16,
    /// Main-loop delay remaining at the fastest cadence.
    pub frame_delay_ticks: u16,
    /// Short voice cooldown remaining at the two-tick cadence.
    pub chatter_cooldown: u16,
    /// Subtitle character delay remaining at the four-tick cadence.
    pub subtitle_reveal_delay: u16,
    /// Dialogue delay remaining at the eight-tick cadence.
    pub dialogue_delay: u16,
    /// Presentation hold remaining at the sixteen-tick cadence.
    pub dialogue_hold_countdown: u16,
    /// Opening-frame pulse remaining at the thirty-two-tick cadence.
    pub subtitle_opening_frame_pulse: u16,
    /// Audio playback state remaining at the twenty-five-subtick cadence.
    pub clip_playback_state: u16,
    /// Wrapping count of elapsed twenty-five-subtick intervals.
    pub mouse_motion_idle_counter: u16,
    /// Ticks until the next slow game-state update.
    pub subtick_countdown: u16,
    /// Wrapping animation phase consumed by the navigation chart.
    pub navigation_animation_phase: u8,
    /// Whether the slow periodic game update is ready.
    pub periodic_update_ready: bool,
    /// Typed state replacing the native two-bit speaker request byte.
    pub speaker_pulse: SpeakerPulseState,
}

impl Default for GameTimerState {
    fn default() -> Self {
        Self {
            running: false,
            tick: u16::MIN,
            frame_delay_ticks: u16::MIN,
            chatter_cooldown: u16::MIN,
            subtitle_reveal_delay: u16::MIN,
            dialogue_delay: u16::MIN,
            dialogue_hold_countdown: u16::MIN,
            subtitle_opening_frame_pulse: u16::MIN,
            clip_playback_state: u16::MIN,
            mouse_motion_idle_counter: u16::MIN,
            subtick_countdown: GAME_SUBTICK_RELOAD,
            navigation_animation_phase: u8::MIN,
            periodic_update_ready: false,
            speaker_pulse: SpeakerPulseState::default(),
        }
    }
}

impl GameTimerState {
    /// Clear the complete idle counter after real pointer motion.
    pub fn reset_mouse_idle_counter(&mut self) {
        self.mouse_motion_idle_counter = u16::MIN;
    }

    /// Clear only the low byte written by BloodScript A8 at native address `0x0B3B`.
    pub fn clear_mouse_idle_counter_low_byte(&mut self) {
        self.mouse_motion_idle_counter &= MOUSE_IDLE_COUNTER_HIGH_BYTE_MASK;
    }

    /// Start modern timer delivery and restore the native slow-update period.
    ///
    /// This is the gameplay-state replacement for `install_timer_isr_hook` at
    /// BLOODPRG routine offset `0x00079C`. SDL owns scheduling, so no interrupt
    /// vector, PIT programming, or saved handler exists.
    pub fn start(&mut self) {
        self.running = true;
        self.subtick_countdown = GAME_SUBTICK_RELOAD;
    }

    /// Stop modern timer delivery.
    ///
    /// This is the gameplay-state replacement for `restore_timer_isr_hook` at
    /// BLOODPRG routine offset `0x0007EA`. Dropping or stopping the host timer
    /// needs no DOS vector restoration.
    pub fn stop(&mut self) {
        self.running = false;
    }
}

/// Non-timer state sampled by one high-frequency game tick.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GameTimerContext {
    /// Whether gameplay timer updates are paused.
    pub paused: bool,
    /// Whether the native pending-record link blocks script countdowns.
    pub pending_record_link: bool,
}

/// Reason one host timer delivery returned and its optional audio action.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameTimerTickOutcome {
    /// State gate that handled the delivery.
    pub status: GameTimerTickStatus,
    /// PC-speaker gate transition requested on this delivery.
    pub speaker_gate: Option<SpeakerGateAction>,
}

/// State gate selected by one timer delivery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameTimerTickStatus {
    /// Timer delivery is stopped.
    Stopped,
    /// Gameplay is paused; all countdown state remains unchanged.
    Paused,
    /// One active gameplay tick was applied.
    Advanced,
}

/// Advance all native gameplay timer cadences by one host-delivered tick.
///
/// This translates `bloodprg_timer_isr` at BLOODPRG routine offset `0x000813`.
/// It retains the wrapping low timer word, nested cadence masks, positive-signed
/// script countdown scan, navigation gate, and speaker pulse state machine. SDL
/// scheduling replaces the interrupt hook; BIOS chaining and PIC acknowledgement
/// are host-only operations and do not exist in this flat state model.
pub fn advance_game_timer_tick(
    state: &mut GameTimerState,
    script: &mut ScriptRuntime,
    context: GameTimerContext,
) -> GameTimerTickOutcome {
    if !state.running {
        return GameTimerTickOutcome {
            status: GameTimerTickStatus::Stopped,
            speaker_gate: None,
        };
    }
    if context.paused {
        return GameTimerTickOutcome {
            status: GameTimerTickStatus::Paused,
            speaker_gate: None,
        };
    }

    state.periodic_update_ready = false;
    decrement_nonzero(&mut state.frame_delay_ticks);
    state.tick = state.tick.wrapping_add(1);

    let mut speaker_gate = None;
    if state.tick & CHATTER_TICK_MASK == u16::MIN {
        decrement_nonzero(&mut state.chatter_cooldown);

        if state.tick & SUBTITLE_TICK_MASK == u16::MIN {
            decrement_nonzero(&mut state.subtitle_reveal_delay);

            if state.tick & DIALOGUE_TICK_MASK == u16::MIN {
                decrement_nonzero(&mut state.dialogue_delay);
                state.subtick_countdown = state.subtick_countdown.wrapping_sub(1);
                if state.subtick_countdown == u16::MIN {
                    state.mouse_motion_idle_counter =
                        state.mouse_motion_idle_counter.wrapping_add(1);
                    if !context.pending_record_link {
                        decrement_positive_script_countdowns(script);
                    }
                    state.subtick_countdown = GAME_SUBTICK_RELOAD;
                    decrement_nonzero(&mut state.clip_playback_state);
                }

                if state.tick & NAVIGATION_TICK_MASK == u16::MIN {
                    state.navigation_animation_phase =
                        state.navigation_animation_phase.wrapping_add(1);
                    decrement_nonzero(&mut state.dialogue_hold_countdown);

                    if state.tick & PERIODIC_TICK_MASK == u16::MIN {
                        speaker_gate = state.speaker_pulse.advance();
                        state.periodic_update_ready = true;
                        decrement_nonzero(&mut state.subtitle_opening_frame_pulse);
                    }
                }
            }
        }
    }

    GameTimerTickOutcome {
        status: GameTimerTickStatus::Advanced,
        speaker_gate,
    }
}

fn decrement_nonzero(value: &mut u16) {
    if *value != u16::MIN {
        *value -= 1;
    }
}

fn decrement_positive_script_countdowns(script: &mut ScriptRuntime) {
    for encoded in u8::MIN..SCRIPT_COUNTDOWN_SLOT_COUNT {
        let slot = ScriptTimerSlot::decode(encoded)
            .expect("native countdown prefix must remain in the typed slot domain");
        let value = script.timer(slot);
        if i16::from_ne_bytes(value.to_ne_bytes()) > 0 {
            script.assign_timer(slot, value - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const IDLE_COUNTER_WITH_BOTH_BYTES: u16 = 0xabcd;
    const IDLE_COUNTER_WITH_CLEARED_LOW_BYTE: u16 = 0xab00;

    #[test]
    fn a8_idle_alias_clear_preserves_the_counter_high_byte() {
        let mut state = GameTimerState {
            mouse_motion_idle_counter: IDLE_COUNTER_WITH_BOTH_BYTES,
            ..GameTimerState::default()
        };

        state.clear_mouse_idle_counter_low_byte();

        assert_eq!(
            state.mouse_motion_idle_counter,
            IDLE_COUNTER_WITH_CLEARED_LOW_BYTE
        );
    }

    const TIMER_ISR_ORACLE_VECTOR_COUNT: usize = 15;
    const TIMER_LIFECYCLE_ORACLE_VECTOR_COUNT: usize = 4;
    const INITIAL_COUNTER_VALUE: u16 = 3;
    const INITIAL_IDLE_COUNTER: u16 = 5;
    const INITIAL_NAVIGATION_PHASE: u8 = 254;
    const NATIVE_SPEAKER_ACTIVE_BIT: u8 = 1;
    const NATIVE_SPEAKER_ENABLED_BIT: u8 = 2;
    const VM_WORD_PATTERN: [u16; 6] = [0, 1, 2, 32_767, 32_768, 65_535];

    #[derive(Deserialize)]
    struct TimerOracle {
        name: String,
        tick_before: u32,
        tick_after: u32,
        divider_after: u8,
        subtick_after: u16,
        periodic_ready: u8,
        speaker_request_after: u8,
        vm_words_after: Vec<u16>,
        chained: bool,
        inputs: Vec<Vec<u16>>,
        outputs: Vec<Vec<u16>>,
    }

    #[derive(Deserialize)]
    struct StartOracle {
        timer_state: StartTimerState,
    }

    #[derive(Deserialize)]
    struct StartTimerState {
        active: u8,
        subtick_limit: u16,
    }

    #[derive(Deserialize)]
    struct StopOracle {
        timer_active: u8,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct ExpectedCounters {
        frame: u16,
        chatter: u16,
        subtitle: u16,
        dialogue: u16,
        hold: u16,
        opening_pulse: u16,
        clip: u16,
        idle: u16,
        navigation_phase: u8,
    }

    #[test]
    fn start_and_stop_match_every_original_lifecycle_vector() {
        let starts: Vec<StartOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_079c_natural.json"
        ))
        .unwrap();
        let stops: Vec<StopOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_07ea_natural.json"
        ))
        .unwrap();
        assert_eq!(starts.len(), TIMER_LIFECYCLE_ORACLE_VECTOR_COUNT);
        assert_eq!(stops.len(), TIMER_LIFECYCLE_ORACLE_VECTOR_COUNT);

        for vector in starts {
            let mut state = GameTimerState {
                subtick_countdown: u16::MAX,
                ..GameTimerState::default()
            };
            state.start();
            assert_eq!(state.running, vector.timer_state.active != u8::MIN);
            assert_eq!(state.subtick_countdown, vector.timer_state.subtick_limit);
        }
        for vector in stops {
            let mut state = GameTimerState {
                running: true,
                ..GameTimerState::default()
            };
            state.stop();
            assert_eq!(state.running, vector.timer_active != u8::MIN);
        }
    }

    #[test]
    fn timer_tick_matches_every_original_gameplay_vector() {
        let vectors: Vec<TimerOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0813_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TIMER_ISR_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut script = script_with_native_timer_pattern();
            let mut state = native_initial_state(&vector);
            let context = native_context(&vector.name);
            let outcome = advance_game_timer_tick(&mut state, &mut script, context);

            let tick_after = u16::try_from(vector.tick_after & u32::from(u16::MAX)).unwrap();
            assert_eq!(state.tick, tick_after, "{}", vector.name);
            assert_eq!(
                state.subtick_countdown, vector.subtick_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.periodic_update_ready,
                vector.periodic_ready != u8::MIN,
                "{}",
                vector.name
            );
            assert_eq!(
                encode_speaker_state(state.speaker_pulse),
                vector.speaker_request_after,
                "{}",
                vector.name
            );
            assert_eq!(
                script_timer_prefix(&script),
                vector.vm_words_after,
                "{}",
                vector.name
            );
            assert_eq!(outcome.status, expected_status(&vector.name));
            assert_eq!(outcome.speaker_gate, expected_speaker_gate(&vector.name));
            assert_eq!(timer_counters(&state), expected_counters(&vector.name));

            assert_hardware_observations_are_outside_flat_state(&vector);
        }
    }

    #[test]
    fn pending_phone_record_freezes_only_script_countdowns() {
        const TIMER_VALUE: u16 = 2;
        const CLIP_COUNTDOWN: u16 = 2;
        let timer_slot = ScriptTimerSlot::decode(u8::MIN).unwrap();
        let mut script = ScriptRuntime::default();
        script.assign_timer(timer_slot, TIMER_VALUE);
        let mut state = GameTimerState {
            running: true,
            tick: DIALOGUE_TICK_MASK,
            subtick_countdown: 1,
            clip_playback_state: CLIP_COUNTDOWN,
            ..GameTimerState::default()
        };

        advance_game_timer_tick(
            &mut state,
            &mut script,
            GameTimerContext {
                pending_record_link: true,
                ..GameTimerContext::default()
            },
        );

        assert_eq!(script.timer(timer_slot), TIMER_VALUE);
        assert_eq!(state.clip_playback_state, CLIP_COUNTDOWN - 1);
        assert_eq!(state.subtick_countdown, GAME_SUBTICK_RELOAD);

        state.tick = DIALOGUE_TICK_MASK;
        state.subtick_countdown = 1;
        advance_game_timer_tick(&mut state, &mut script, GameTimerContext::default());
        assert_eq!(script.timer(timer_slot), TIMER_VALUE - 1);
    }

    fn native_initial_state(vector: &TimerOracle) -> GameTimerState {
        let speaker_request = match vector.name.as_str() {
            "speaker_enable_request" => NATIVE_SPEAKER_ACTIVE_BIT,
            "speaker_disable_request" => NATIVE_SPEAKER_ACTIVE_BIT | NATIVE_SPEAKER_ENABLED_BIT,
            _ => NATIVE_SPEAKER_ENABLED_BIT,
        };
        GameTimerState {
            running: vector.name != "inactive_low_bit_chains",
            tick: u16::try_from(vector.tick_before & u32::from(u16::MAX)).unwrap(),
            frame_delay_ticks: INITIAL_COUNTER_VALUE,
            chatter_cooldown: INITIAL_COUNTER_VALUE,
            subtitle_reveal_delay: INITIAL_COUNTER_VALUE,
            dialogue_delay: INITIAL_COUNTER_VALUE,
            dialogue_hold_countdown: INITIAL_COUNTER_VALUE,
            subtitle_opening_frame_pulse: INITIAL_COUNTER_VALUE,
            clip_playback_state: INITIAL_COUNTER_VALUE,
            mouse_motion_idle_counter: INITIAL_IDLE_COUNTER,
            subtick_countdown: match vector.name.as_str() {
                "eight_tick_subtick_wait" => 2,
                "subtick_decrements_positive_vm_timers"
                | "subtick_scan_blocked_by_navigation_link"
                | "low_word_wrap_from_maximum_high_word" => 1,
                _ => INITIAL_COUNTER_VALUE,
            },
            navigation_animation_phase: INITIAL_NAVIGATION_PHASE,
            periodic_update_ready: true,
            speaker_pulse: decode_speaker_state(speaker_request),
        }
    }

    fn native_context(name: &str) -> GameTimerContext {
        GameTimerContext {
            paused: matches!(name, "paused_interrupt_acknowledged" | "paused_bios_chain"),
            pending_record_link: name == "subtick_scan_blocked_by_navigation_link",
        }
    }

    fn script_with_native_timer_pattern() -> ScriptRuntime {
        let mut script = ScriptRuntime::new();
        for encoded in u8::MIN..SCRIPT_COUNTDOWN_SLOT_COUNT {
            let slot = ScriptTimerSlot::decode(encoded).unwrap();
            script.assign_timer(
                slot,
                VM_WORD_PATTERN[usize::from(encoded) % VM_WORD_PATTERN.len()],
            );
        }
        script
    }

    fn script_timer_prefix(script: &ScriptRuntime) -> Vec<u16> {
        (u8::MIN..SCRIPT_COUNTDOWN_SLOT_COUNT)
            .map(|encoded| script.timer(ScriptTimerSlot::decode(encoded).unwrap()))
            .collect()
    }

    fn decode_speaker_state(bits: u8) -> SpeakerPulseState {
        SpeakerPulseState {
            enabled: bits & NATIVE_SPEAKER_ENABLED_BIT != u8::MIN,
            active: bits & NATIVE_SPEAKER_ACTIVE_BIT != u8::MIN,
        }
    }

    fn encode_speaker_state(state: SpeakerPulseState) -> u8 {
        (u8::from(state.active) * NATIVE_SPEAKER_ACTIVE_BIT)
            | (u8::from(state.enabled) * NATIVE_SPEAKER_ENABLED_BIT)
    }

    fn expected_status(name: &str) -> GameTimerTickStatus {
        match name {
            "inactive_low_bit_chains" => GameTimerTickStatus::Stopped,
            "paused_interrupt_acknowledged" | "paused_bios_chain" => GameTimerTickStatus::Paused,
            _ => GameTimerTickStatus::Advanced,
        }
    }

    fn expected_speaker_gate(name: &str) -> Option<SpeakerGateAction> {
        match name {
            "speaker_enable_request" => Some(SpeakerGateAction::Enable),
            "speaker_disable_request" => Some(SpeakerGateAction::Disable),
            _ => None,
        }
    }

    fn timer_counters(state: &GameTimerState) -> ExpectedCounters {
        ExpectedCounters {
            frame: state.frame_delay_ticks,
            chatter: state.chatter_cooldown,
            subtitle: state.subtitle_reveal_delay,
            dialogue: state.dialogue_delay,
            hold: state.dialogue_hold_countdown,
            opening_pulse: state.subtitle_opening_frame_pulse,
            clip: state.clip_playback_state,
            idle: state.mouse_motion_idle_counter,
            navigation_phase: state.navigation_animation_phase,
        }
    }

    fn expected_counters(name: &str) -> ExpectedCounters {
        let unchanged = ExpectedCounters {
            frame: 3,
            chatter: 3,
            subtitle: 3,
            dialogue: 3,
            hold: 3,
            opening_pulse: 3,
            clip: 3,
            idle: 5,
            navigation_phase: 254,
        };
        match name {
            "inactive_low_bit_chains" | "paused_interrupt_acknowledged" | "paused_bios_chain" => {
                unchanged
            }
            "odd_tick_frame_only" | "zero_bios_divider_wraps_without_chain" => ExpectedCounters {
                frame: 2,
                ..unchanged
            },
            "two_tick_chatter" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                ..unchanged
            },
            "four_tick_subtitle" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                subtitle: 2,
                ..unchanged
            },
            "eight_tick_subtick_wait" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                subtitle: 2,
                dialogue: 2,
                ..unchanged
            },
            "subtick_decrements_positive_vm_timers" | "subtick_scan_blocked_by_navigation_link" => {
                ExpectedCounters {
                    frame: 2,
                    chatter: 2,
                    subtitle: 2,
                    dialogue: 2,
                    clip: 2,
                    idle: 6,
                    ..unchanged
                }
            }
            "sixteen_tick_hold_and_mask" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                subtitle: 2,
                dialogue: 2,
                hold: 2,
                navigation_phase: 255,
                ..unchanged
            },
            "speaker_enable_request"
            | "speaker_disable_request"
            | "low_word_wrap_leaves_high_word" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                subtitle: 2,
                dialogue: 2,
                hold: 2,
                opening_pulse: 2,
                navigation_phase: 255,
                ..unchanged
            },
            "low_word_wrap_from_maximum_high_word" => ExpectedCounters {
                frame: 2,
                chatter: 2,
                subtitle: 2,
                dialogue: 2,
                hold: 2,
                opening_pulse: 2,
                clip: 2,
                idle: 6,
                navigation_phase: 255,
            },
            _ => panic!("unknown timer oracle vector {name}"),
        }
    }

    fn assert_hardware_observations_are_outside_flat_state(vector: &TimerOracle) {
        match vector.name.as_str() {
            "inactive_low_bit_chains" | "paused_bios_chain" => assert!(vector.chained),
            _ => {
                let _native_divider = vector.divider_after;
                assert!(!vector.chained);
            }
        }
        let speaker_io_count = usize::from(expected_speaker_gate(&vector.name).is_some());
        assert_eq!(vector.inputs.len(), speaker_io_count);
        assert_eq!(
            vector.outputs.len(),
            speaker_io_count + usize::from(!vector.chained)
        );
    }
}
