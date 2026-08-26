//! Dialogue and short voice-clip event selection.

use std::fmt;

const DIALOGUE_DELAY_STEP_MASK: u8 = 15;
const DIALOGUE_SELECTION_MASK: u16 = 31;
const CHATTER_RANDOM_UPPER_BOUND: u16 = 10;
const CHATTER_BANK_CLIP_OFFSET: u16 = 7;
const CHATTER_COOLDOWN: u8 = 4;
const SELECTION_ATTEMPT_LIMIT: usize = u16::MAX as usize + 1;

/// One clip request selected by the audio event coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioClipRequest {
    /// A deterministic clip in the currently streamed dialogue bank.
    StreamedDialogue {
        /// Zero-based streamed clip index.
        index: u16,
    },
    /// A short voice reaction in the resident sound bank.
    VoiceReaction {
        /// Zero-based resident bank clip index.
        bank_index: u16,
    },
}

/// Mutable dialogue and chatter selection state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioEventState {
    /// Whether voice playback is enabled.
    pub playback_enabled: bool,
    /// Newly published menu words must be hashed into a dialogue seed.
    pub menu_words_pending: bool,
    /// Deterministic streamed-dialogue selection is armed.
    pub dialogue_armed: bool,
    /// A subtitle requested a short voice reaction.
    pub voice_reaction_requested: bool,
    /// Remaining short-voice cooldown in game ticks.
    pub voice_cooldown: u8,
    /// Remaining deterministic-dialogue delay in game ticks.
    pub dialogue_delay: u16,
    /// Persistent deterministic selection seed.
    pub dialogue_seed: u16,
    /// Most recently selected clip index, shared by both selection paths.
    pub last_clip: u16,
}

/// Read-only inputs for one audio event update.
#[derive(Clone, Copy, Debug)]
pub struct AudioEventContext<'a> {
    /// Game mode suppresses menu-derived dialogue but not voice reactions.
    pub dialogue_suppressed: bool,
    /// Current menu words in authored game-font bytes.
    pub menu_words: &'a [Box<[u8]>],
    /// Number of clips in the streamed dialogue bank.
    pub streamed_dialogue_clip_count: u16,
    /// Base delay from the active sound-bank header.
    pub dialogue_delay_base: u8,
    /// Inclusive maximum delay from the active sound-bank header.
    pub dialogue_delay_limit: u8,
}

/// Malformed bank state or random output that cannot produce a clip.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioEventError {
    /// Repeated halving can never bring the base delay below the limit.
    DialogueDelayUnreachable {
        /// Authored base delay.
        base: u8,
        /// Authored inclusive delay limit.
        limit: u8,
    },
    /// No nonduplicate streamed dialogue clip was reachable.
    DialogueSelectionUnreachable {
        /// Number of available streamed clips.
        clip_count: u16,
        /// Clip excluded as the previous selection.
        last_clip: u16,
    },
    /// The random provider returned a value outside its requested domain.
    RandomResultOutsideRange {
        /// Exclusive upper bound supplied to the provider.
        upper_bound: u16,
        /// Invalid returned value.
        result: u16,
    },
    /// The random provider never returned a clip distinct from the last one.
    VoiceSelectionUnreachable {
        /// Clip excluded as the previous selection.
        last_clip: u16,
    },
}

impl fmt::Display for AudioEventError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AudioEventError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct DialogueSelection {
    clip_index: u16,
    next_seed: u16,
    attempts: usize,
}

/// Select dialogue and short voice clips for the current game tick.
///
/// This translates `audio_process_ade` at BLOODPRG routine offset `0x00B7E3`.
/// Interned word bytes, semantic event flags, typed clip requests, and an
/// injected bounded random source replace dictionary offsets, split data
/// segments, high-bit clip tagging, and a far playback callback. Signed-byte
/// hashing and all wrapping seed arithmetic are retained because they select
/// authored sounds.
pub fn process_audio_events<Random>(
    state: &mut AudioEventState,
    context: AudioEventContext<'_>,
    mut random_below: Random,
) -> Result<Box<[AudioClipRequest]>, AudioEventError>
where
    Random: FnMut(u16) -> u16,
{
    let mut requests = Vec::with_capacity(2);
    if !state.playback_enabled {
        return Ok(requests.into_boxed_slice());
    }

    if !context.dialogue_suppressed {
        if state.menu_words_pending {
            state.menu_words_pending = false;
            state.dialogue_seed = hash_dialogue_words(context.menu_words);
            state.dialogue_armed = true;
        } else if state.dialogue_armed && state.dialogue_delay == 0 {
            let (delay, _delay_attempts) = select_dialogue_delay(
                state.dialogue_seed,
                context.dialogue_delay_base,
                context.dialogue_delay_limit,
            )?;
            let selection = select_dialogue_clip(
                state.dialogue_seed,
                context.streamed_dialogue_clip_count,
                state.last_clip,
            )?;
            state.dialogue_delay = delay;
            state.dialogue_seed = selection.next_seed;
            state.last_clip = selection.clip_index;
            requests.push(AudioClipRequest::StreamedDialogue {
                index: selection.clip_index,
            });
        }
    }

    if state.voice_reaction_requested && state.voice_cooldown == 0 {
        state.voice_cooldown = CHATTER_COOLDOWN;
        let mut selected = None;
        for _ in 0..SELECTION_ATTEMPT_LIMIT {
            let candidate = random_below(CHATTER_RANDOM_UPPER_BOUND);
            if candidate >= CHATTER_RANDOM_UPPER_BOUND {
                return Err(AudioEventError::RandomResultOutsideRange {
                    upper_bound: CHATTER_RANDOM_UPPER_BOUND,
                    result: candidate,
                });
            }
            if candidate != state.last_clip {
                selected = Some(candidate);
                break;
            }
        }
        let selected = selected.ok_or(AudioEventError::VoiceSelectionUnreachable {
            last_clip: state.last_clip,
        })?;
        state.last_clip = selected;
        requests.push(AudioClipRequest::VoiceReaction {
            bank_index: selected.wrapping_add(CHATTER_BANK_CLIP_OFFSET),
        });
    }

    Ok(requests.into_boxed_slice())
}

fn hash_dialogue_words(words: &[Box<[u8]>]) -> u16 {
    let mut hash = 0_u16;
    let mut word_count = 0_u16;
    for word in words {
        for byte in word {
            hash = hash.wrapping_add((*byte as i8 as i16) as u16);
        }
        word_count = word_count.wrapping_add(1);
    }
    hash.wrapping_add(word_count) >> 4
}

fn select_dialogue_delay(seed: u16, base: u8, limit: u8) -> Result<(u16, usize), AudioEventError> {
    if base > limit {
        return Err(AudioEventError::DialogueDelayUnreachable { base, limit });
    }

    let mut delay_step = seed as u8 & DIALOGUE_DELAY_STEP_MASK;
    for attempts in 1..=u8::BITS as usize {
        let delay = base.wrapping_add(delay_step);
        if delay <= limit {
            return Ok(((delay as i8 as i16) as u16, attempts));
        }
        delay_step >>= 1;
    }
    unreachable!("a zero delay step must make an admissible base delay")
}

fn select_dialogue_clip(
    seed: u16,
    clip_count: u16,
    last_clip: u16,
) -> Result<DialogueSelection, AudioEventError> {
    let mut selection_state = seed;
    let mut clip_index = seed;
    let mut next_seed = seed;

    for attempts in 1..=SELECTION_ATTEMPT_LIMIT {
        selection_state = selection_state.wrapping_sub(2);
        clip_index = clip_index.wrapping_sub(selection_state & DIALOGUE_SELECTION_MASK);
        if (clip_index as i16) < 0 {
            clip_index = 0_u16.wrapping_sub(clip_index);
        }
        if clip_index >= clip_count {
            continue;
        }

        next_seed = next_seed.wrapping_add(1);
        if clip_index != last_clip {
            return Ok(DialogueSelection {
                clip_index,
                next_seed,
                attempts,
            });
        }
    }

    Err(AudioEventError::DialogueSelectionUnreachable {
        clip_count,
        last_clip,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 11;

    #[derive(Deserialize)]
    struct AudioOracle {
        name: String,
        hash_words: Vec<u16>,
        dialogue_seed_after: u16,
        dialogue_delay_after: u16,
        primary_selection_iterations: usize,
        delay_attempts: usize,
        prng_results: Vec<u16>,
        play_calls: Vec<u16>,
        last_clip_after: u16,
        cooldown_after: u8,
        split_ds_gs: bool,
    }

    struct Case {
        playback_enabled: bool,
        dialogue_suppressed: bool,
        menu_words_pending: bool,
        dialogue_armed: bool,
        voice_reaction_requested: bool,
        voice_cooldown: u8,
        dialogue_delay: u16,
        dialogue_seed: u16,
        last_clip: u16,
        clip_count: u16,
        delay_base: u8,
        delay_limit: u8,
    }

    #[test]
    fn selector_matches_every_original_dialogue_and_chatter_vector() {
        let vectors: Vec<AudioOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b7e3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let case = case_for(&vector.name);
            let words = words_for(&vector.hash_words);
            let mut state = AudioEventState {
                playback_enabled: case.playback_enabled,
                menu_words_pending: case.menu_words_pending,
                dialogue_armed: case.dialogue_armed,
                voice_reaction_requested: case.voice_reaction_requested,
                voice_cooldown: case.voice_cooldown,
                dialogue_delay: case.dialogue_delay,
                dialogue_seed: case.dialogue_seed,
                last_clip: case.last_clip,
            };
            let mut random_results = vector.prng_results.iter().copied();
            let mut random_calls = Vec::new();

            let requests = process_audio_events(
                &mut state,
                AudioEventContext {
                    dialogue_suppressed: case.dialogue_suppressed,
                    menu_words: &words,
                    streamed_dialogue_clip_count: case.clip_count,
                    dialogue_delay_base: case.delay_base,
                    dialogue_delay_limit: case.delay_limit,
                },
                |upper_bound| {
                    random_calls.push(upper_bound);
                    random_results.next().unwrap()
                },
            )
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(
                state.dialogue_seed, vector.dialogue_seed_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.dialogue_delay, vector.dialogue_delay_after,
                "{}",
                vector.name
            );
            assert_eq!(state.last_clip, vector.last_clip_after, "{}", vector.name);
            assert_eq!(
                state.voice_cooldown, vector.cooldown_after,
                "{}",
                vector.name
            );
            assert_eq!(
                requests.as_ref(),
                expected_requests(&vector.play_calls).as_slice(),
                "{}",
                vector.name
            );
            assert_eq!(
                random_calls,
                vec![CHATTER_RANDOM_UPPER_BOUND; vector.prng_results.len()],
                "{}",
                vector.name
            );
            assert!(random_results.next().is_none(), "{}", vector.name);

            if vector.primary_selection_iterations != 0 {
                let selection =
                    select_dialogue_clip(case.dialogue_seed, case.clip_count, case.last_clip)
                        .unwrap();
                assert_eq!(
                    selection.attempts, vector.primary_selection_iterations,
                    "{}",
                    vector.name
                );
            }
            if vector.delay_attempts != 0 {
                let (_, attempts) =
                    select_dialogue_delay(case.dialogue_seed, case.delay_base, case.delay_limit)
                        .unwrap();
                assert_eq!(attempts, vector.delay_attempts, "{}", vector.name);
            }
            assert_eq!(
                vector.split_ds_gs,
                vector.name == "split_ds_gs_hash",
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_selection_inputs_return_bounded_errors() {
        let mut state = AudioEventState {
            playback_enabled: true,
            menu_words_pending: false,
            dialogue_armed: true,
            voice_reaction_requested: false,
            voice_cooldown: 0,
            dialogue_delay: 0,
            dialogue_seed: 7,
            last_clip: 0,
        };
        assert_eq!(
            process_audio_events(
                &mut state,
                AudioEventContext {
                    dialogue_suppressed: false,
                    menu_words: &[],
                    streamed_dialogue_clip_count: 0,
                    dialogue_delay_base: 20,
                    dialogue_delay_limit: 10,
                },
                |_| 0,
            ),
            Err(AudioEventError::DialogueDelayUnreachable {
                base: 20,
                limit: 10,
            })
        );
    }

    fn words_for(offsets: &[u16]) -> Vec<Box<[u8]>> {
        offsets
            .iter()
            .take_while(|offset| **offset != 0 && **offset != u16::MAX)
            .map(|&offset| match offset {
                32 => Box::<[u8]>::from(*b"A\x80\xff"),
                64 => Box::<[u8]>::from(*b"Commander"),
                96 => Box::<[u8]>::default(),
                unknown => panic!("unknown dictionary fixture {unknown}"),
            })
            .collect()
    }

    fn expected_requests(play_calls: &[u16]) -> Vec<AudioClipRequest> {
        play_calls
            .iter()
            .map(|clip| {
                if clip & 32_768 != 0 {
                    AudioClipRequest::StreamedDialogue {
                        index: clip & 32_767,
                    }
                } else {
                    AudioClipRequest::VoiceReaction { bank_index: *clip }
                }
            })
            .collect()
    }

    fn case_for(name: &str) -> Case {
        match name {
            "sound_disabled" => case(false, false, true, true, true, 0, 0, 7, 3, 12, 4, 20),
            "mode_suppresses_dialogue" => {
                case(true, true, true, true, false, 0, 0, 19, 5, 9, 3, 18)
            }
            "mode_allows_chatter" => case(true, true, true, true, true, 0, 0, 21, 5, 10, 4, 20),
            "hash_empty" => case(
                true, false, true, false, false, 0, 4_660, 17_767, 8, 16, 5, 24,
            ),
            "hash_signed_words" => case(
                true, false, true, false, false, 2, 17_185, 39_321, 7, 17, 6, 24,
            ),
            "split_ds_gs_hash" => case(
                true, false, true, false, false, 3, 8_738, 13_107, 4, 14, 7, 25,
            ),
            "armed_delay_busy" => case(true, false, false, true, true, 2, 3, 7, 2, 20, 4, 20),
            "armed_primary" => case(true, false, false, true, false, 0, 0, 7, 9, 32, 4, 20),
            "primary_range_and_duplicate_retry" => {
                case(true, false, false, true, false, 0, 0, 40, 2, 10, 18, 20)
            }
            "voice_prng_reroll" => case(true, false, false, false, true, 0, 0, 11, 3, 18, 3, 17),
            "primary_then_voice" => case(true, false, false, true, true, 0, 0, 7, 9, 32, 4, 20),
            unknown => panic!("unknown oracle case {unknown}"),
        }
    }

    #[allow(clippy::too_many_arguments)]
    const fn case(
        playback_enabled: bool,
        dialogue_suppressed: bool,
        menu_words_pending: bool,
        dialogue_armed: bool,
        voice_reaction_requested: bool,
        voice_cooldown: u8,
        dialogue_delay: u16,
        dialogue_seed: u16,
        last_clip: u16,
        clip_count: u16,
        delay_base: u8,
        delay_limit: u8,
    ) -> Case {
        Case {
            playback_enabled,
            dialogue_suppressed,
            menu_words_pending,
            dialogue_armed,
            voice_reaction_requested,
            voice_cooldown,
            dialogue_delay,
            dialogue_seed,
            last_clip,
            clip_count,
            delay_base,
            delay_limit,
        }
    }
}
