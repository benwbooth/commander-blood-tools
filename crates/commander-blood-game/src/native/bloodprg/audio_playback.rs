//! Direct sound playback and voice-over-stream mixing.

use std::fmt;

use commander_blood_formats::snd::{SND_CLIP_HEADER_BYTE_COUNT, SndBank};

use super::AudioClipRequest;

const AUDIO_STREAM_BUFFER_COUNT: usize = 2;
const FIRST_STREAM_BUFFER_INDEX: usize = 0;
const LAST_STREAM_BUFFER_INDEX: usize = AUDIO_STREAM_BUFFER_COUNT - 1;
const RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT: usize = 1;
const MIX_BOUNDARY_EXCLUSION_BYTE_COUNT: u16 = 1;

/// The two request bits formerly packed into the DOS sound-driver pending byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AudioDriverRequests {
    /// A newly loaded stream is waiting to be started.
    pub stream_start_requested: bool,
    /// Double-buffered stream playback is active.
    pub stream_active: bool,
}

impl AudioDriverRequests {
    fn clear(&mut self) {
        self.stream_start_requested = false;
        self.stream_active = false;
    }
}

/// Authored state of one double-buffer descriptor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AudioStreamBufferStatus {
    /// The buffer can be refilled.
    #[default]
    Free,
    /// Refilled data is ready for the host audio driver.
    Ready,
    /// The driver owns a buffer that is not marked ready.
    DriverOwned,
    /// The driver owns ready data, making this the active voice-over mix target.
    ReadyAndDriverOwned,
}

impl AudioStreamBufferStatus {
    const fn accepts_voice_mix(self) -> bool {
        matches!(self, Self::ReadyAndDriverOwned)
    }
}

/// One owned music-stream buffer with its Creative Voice block header separated
/// from the unsigned 8-bit samples that voice clips may modify.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioStreamBuffer {
    /// Six-byte Creative Voice block header retained for stream submission.
    pub header: [u8; SND_CLIP_HEADER_BYTE_COUNT],
    /// Unsigned 8-bit mono samples following the header.
    pub samples: Box<[u8]>,
    /// Current relationship between the buffer and the host audio driver.
    pub status: AudioStreamBufferStatus,
}

/// Mutable playback state shared by direct clips and the music stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioPlaybackState {
    /// Whether digital sound playback is enabled.
    pub playback_enabled: bool,
    /// Pending stream work represented without a packed flag byte.
    pub driver_requests: AudioDriverRequests,
    /// Whether one source byte is stretched across two destination samples.
    pub packed_stream_samples: bool,
    /// The two music buffers alternated by the host audio driver.
    pub stream_buffers: [AudioStreamBuffer; AUDIO_STREAM_BUFFER_COUNT],
}

/// Resident and streamed banks used by one playback request.
#[derive(Clone, Copy, Debug)]
pub struct AudioPlaybackBanks<'a> {
    /// Short effects and voice reactions retained in memory by the DOS game.
    pub resident_effects: &'a SndBank,
    /// Dialogue clips that the DOS game staged through EMS, XMS, or a file.
    pub streamed_dialogue: &'a SndBank,
}

/// Opaque Creative Voice block bytes submitted for one-shot playback.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectSoundPlayback {
    /// Logical source and authored clip index.
    pub request: AudioClipRequest,
    /// Encoded block sequence consumed by the replacement audio backend.
    pub encoded_clip: Box<[u8]>,
}

/// One contiguous destination range changed by voice-over-stream mixing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioMixOperation {
    /// Index in [`AudioPlaybackState::stream_buffers`].
    pub buffer_index: usize,
    /// Number of destination samples averaged with the voice clip.
    pub sample_count: usize,
}

/// Why a stream-mix attempt did or did not change samples.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioMixStatus {
    /// All applicable spans were mixed.
    Mixed,
    /// Neither stream buffer was in the driver's active mix state.
    NoActiveBuffer,
    /// The audio backend could not report a current playback position.
    PositionUnavailable,
}

/// Observable result of a voice-over-stream attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioMixReport {
    /// Terminal status of this attempt.
    pub status: AudioMixStatus,
    /// Logical destination samples covered by the clip after packed expansion.
    pub source_output_sample_count: u16,
    /// Physical source bytes consumed by the original packed/unpacked cadence.
    pub source_byte_count_consumed: usize,
    /// Ordered buffer mutations.
    pub operations: Box<[AudioMixOperation]>,
}

/// Result of processing one selected sound clip.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AudioPlaybackOutcome {
    /// Playback was disabled and no state changed.
    PlaybackDisabled,
    /// Stop the current driver activity, then play this encoded clip once.
    StopAndPlay(DirectSoundPlayback),
    /// Mix the selected clip over active streamed audio.
    StreamMix(AudioMixReport),
}

/// Invalid bank, clip, or host buffer state that cannot be represented safely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioPlaybackError {
    /// The requested authored clip index does not exist in its bank.
    ClipNotFound {
        /// Logical request that could not be resolved.
        request: AudioClipRequest,
    },
    /// A resident clip has no byte to exclude as the driver's terminator.
    ResidentClipEmpty {
        /// Zero-based resident bank index.
        index: u16,
    },
    /// A clip mixed as PCM is shorter than its six-byte block header.
    ClipHeaderTruncated {
        /// Logical request containing the malformed clip.
        request: AudioClipRequest,
        /// Available encoded bytes.
        actual: usize,
    },
    /// A clip or stream buffer exceeds the original routine's word-sized count.
    WordCountOverflow {
        /// Number of bytes that cannot be represented.
        byte_count: usize,
    },
    /// The exact original cadence would read past the owned bank payload.
    MixSourceTruncated {
        /// Logical request whose source bytes are incomplete.
        request: AudioClipRequest,
        /// Minimum source bytes needed by the planned mix spans.
        required: usize,
        /// Source bytes available in flat storage.
        available: usize,
    },
}

impl fmt::Display for AudioPlaybackError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AudioPlaybackError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PlannedMixSpan {
    buffer_index: usize,
    sample_offset: usize,
    sample_count: u16,
}

#[derive(Clone, Copy)]
struct MixSource<'a> {
    bytes: &'a [u8],
    output_sample_count: u16,
}

/// Play a selected clip directly or average it into the active music buffers.
///
/// This translates `snd_play_clip` at BLOODPRG routine offset `0x00B8CD`.
/// Owned banks and sample buffers replace conventional, EMS, XMS, file, and far
/// pointer storage. The resident loader's `clip_length - 1` count, six-byte
/// source skip, one-sample-short destination bounds, packed source cadence,
/// active-buffer preference, and stop-before-direct-play behavior are retained.
pub fn update_audio_playback<Position>(
    state: &mut AudioPlaybackState,
    request: AudioClipRequest,
    banks: AudioPlaybackBanks<'_>,
    mut playback_position: Position,
) -> Result<AudioPlaybackOutcome, AudioPlaybackError>
where
    Position: FnMut() -> Option<u16>,
{
    if !state.playback_enabled {
        return Ok(AudioPlaybackOutcome::PlaybackDisabled);
    }

    if !state.driver_requests.stream_active {
        let encoded_clip = resolve_direct_clip(request, banks)?;
        state.driver_requests.clear();
        return Ok(AudioPlaybackOutcome::StopAndPlay(DirectSoundPlayback {
            request,
            encoded_clip: encoded_clip.into(),
        }));
    }

    let source = resolve_mix_source(request, banks, state.packed_stream_samples)?;
    let Some(active_buffer_index) = state
        .stream_buffers
        .iter()
        .position(|buffer| buffer.status.accepts_voice_mix())
    else {
        return Ok(AudioPlaybackOutcome::StreamMix(AudioMixReport {
            status: AudioMixStatus::NoActiveBuffer,
            source_output_sample_count: source.output_sample_count,
            source_byte_count_consumed: 0,
            operations: Box::new([]),
        }));
    };

    let Some(position) = playback_position() else {
        return Ok(AudioPlaybackOutcome::StreamMix(AudioMixReport {
            status: AudioMixStatus::PositionUnavailable,
            source_output_sample_count: source.output_sample_count,
            source_byte_count_consumed: 0,
            operations: Box::new([]),
        }));
    };

    let buffer_lengths = [
        word_count(
            state.stream_buffers[FIRST_STREAM_BUFFER_INDEX]
                .samples
                .len(),
        )?,
        word_count(state.stream_buffers[LAST_STREAM_BUFFER_INDEX].samples.len())?,
    ];
    let spans = plan_mix_spans(
        active_buffer_index,
        buffer_lengths,
        position,
        source.output_sample_count,
    );
    let (source_byte_count_consumed, required_source_bytes) =
        source_requirements(&spans, state.packed_stream_samples);
    if required_source_bytes > source.bytes.len() {
        return Err(AudioPlaybackError::MixSourceTruncated {
            request,
            required: required_source_bytes,
            available: source.bytes.len(),
        });
    }

    apply_mix_spans(
        &mut state.stream_buffers,
        &spans,
        source.bytes,
        state.packed_stream_samples,
    );
    let operations = spans
        .iter()
        .map(|span| AudioMixOperation {
            buffer_index: span.buffer_index,
            sample_count: usize::from(span.sample_count),
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    Ok(AudioPlaybackOutcome::StreamMix(AudioMixReport {
        status: AudioMixStatus::Mixed,
        source_output_sample_count: source.output_sample_count,
        source_byte_count_consumed,
        operations,
    }))
}

fn resolve_direct_clip<'a>(
    request: AudioClipRequest,
    banks: AudioPlaybackBanks<'a>,
) -> Result<&'a [u8], AudioPlaybackError> {
    let clip = clip_for_request(request, banks)?;
    match request {
        AudioClipRequest::VoiceReaction { bank_index } => {
            let byte_count = clip
                .encoded()
                .len()
                .checked_sub(RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT)
                .ok_or(AudioPlaybackError::ResidentClipEmpty { index: bank_index })?;
            word_count(byte_count)?;
            Ok(&clip.encoded()[..byte_count])
        }
        AudioClipRequest::StreamedDialogue { .. } => {
            word_count(clip.encoded().len())?;
            Ok(clip.encoded())
        }
    }
}

fn resolve_mix_source<'a>(
    request: AudioClipRequest,
    banks: AudioPlaybackBanks<'a>,
    packed: bool,
) -> Result<MixSource<'a>, AudioPlaybackError> {
    let clip = clip_for_request(request, banks)?;
    let encoded = clip.encoded();
    if encoded.len() < SND_CLIP_HEADER_BYTE_COUNT {
        return Err(AudioPlaybackError::ClipHeaderTruncated {
            request,
            actual: encoded.len(),
        });
    }

    let (bytes, physical_source_count) = match request {
        AudioClipRequest::VoiceReaction { bank_index } => {
            let physical_source_count = encoded
                .len()
                .checked_sub(RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT)
                .ok_or(AudioPlaybackError::ResidentClipEmpty { index: bank_index })?;
            let source_start = banks.resident_effects.offsets()[usize::from(bank_index)]
                + SND_CLIP_HEADER_BYTE_COUNT;
            (
                &banks.resident_effects.payload()[source_start..],
                physical_source_count,
            )
        }
        AudioClipRequest::StreamedDialogue { .. } => (
            &encoded[SND_CLIP_HEADER_BYTE_COUNT..],
            encoded.len() - SND_CLIP_HEADER_BYTE_COUNT,
        ),
    };
    let physical_source_count = word_count(physical_source_count)?;
    let output_sample_count = if packed {
        physical_source_count.wrapping_add(physical_source_count)
    } else {
        physical_source_count
    };
    Ok(MixSource {
        bytes,
        output_sample_count,
    })
}

fn clip_for_request<'a>(
    request: AudioClipRequest,
    banks: AudioPlaybackBanks<'a>,
) -> Result<commander_blood_formats::snd::SndClip<'a>, AudioPlaybackError> {
    let clip = match request {
        AudioClipRequest::VoiceReaction { bank_index } => {
            banks.resident_effects.clip(usize::from(bank_index))
        }
        AudioClipRequest::StreamedDialogue { index } => {
            banks.streamed_dialogue.clip(usize::from(index))
        }
    };
    clip.ok_or(AudioPlaybackError::ClipNotFound { request })
}

fn word_count(byte_count: usize) -> Result<u16, AudioPlaybackError> {
    u16::try_from(byte_count).map_err(|_| AudioPlaybackError::WordCountOverflow { byte_count })
}

fn plan_mix_spans(
    active_buffer_index: usize,
    buffer_lengths: [u16; AUDIO_STREAM_BUFFER_COUNT],
    playback_position: u16,
    source_output_sample_count: u16,
) -> Vec<PlannedMixSpan> {
    let other_buffer_index = LAST_STREAM_BUFFER_INDEX - active_buffer_index;
    let active_buffer_length = buffer_lengths[active_buffer_index];
    let position_delta = playback_position.wrapping_sub(active_buffer_length);
    let sample_offset = if (position_delta as i16).is_negative() {
        0_u16.wrapping_sub(position_delta)
    } else {
        position_delta
    };
    let mut remaining = source_output_sample_count;
    let mut spans = Vec::with_capacity(AUDIO_STREAM_BUFFER_COUNT);

    if sample_offset < active_buffer_length {
        let available = active_buffer_length - sample_offset;
        remaining = remaining.wrapping_sub(available);
        let mix_count = if (remaining as i16).is_negative() {
            source_output_sample_count
        } else {
            available
        }
        .wrapping_sub(MIX_BOUNDARY_EXCLUSION_BYTE_COUNT);
        if (mix_count as i16).is_positive() {
            spans.push(PlannedMixSpan {
                buffer_index: active_buffer_index,
                sample_offset: usize::from(sample_offset),
                sample_count: mix_count,
            });
        }
    }

    if (remaining as i16).is_positive() {
        let mix_count = remaining
            .min(buffer_lengths[other_buffer_index])
            .wrapping_sub(MIX_BOUNDARY_EXCLUSION_BYTE_COUNT);
        if (mix_count as i16).is_positive() {
            spans.push(PlannedMixSpan {
                buffer_index: other_buffer_index,
                sample_offset: 0,
                sample_count: mix_count,
            });
        }
    }
    spans
}

fn source_requirements(spans: &[PlannedMixSpan], packed: bool) -> (usize, usize) {
    let mut cursor = 0_usize;
    let mut required = 0_usize;
    for span in spans {
        let mut remaining = span.sample_count;
        while remaining != 0 {
            required = required.max(cursor + 1);
            if !packed || remaining.is_multiple_of(2) {
                cursor += 1;
            }
            remaining -= 1;
        }
    }
    (cursor, required)
}

fn apply_mix_spans(
    buffers: &mut [AudioStreamBuffer; AUDIO_STREAM_BUFFER_COUNT],
    spans: &[PlannedMixSpan],
    source: &[u8],
    packed: bool,
) {
    let mut source_cursor = 0_usize;
    for span in spans {
        let destination_end = span.sample_offset + usize::from(span.sample_count);
        let destination =
            &mut buffers[span.buffer_index].samples[span.sample_offset..destination_end];
        let mut remaining = span.sample_count;
        for destination_sample in destination {
            let source_sample = source[source_cursor];
            if !packed || remaining.is_multiple_of(2) {
                source_cursor += 1;
            }
            *destination_sample =
                ((u16::from(source_sample) + u16::from(*destination_sample)) / 2) as u8;
            remaining -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 15;
    const ORIGINAL_STREAMED_CLIP_MARKER: u16 = 32_768;
    const ORIGINAL_STREAMED_CLIP_INDEX_MASK: u16 = 16_383;
    const TEST_CLIP_DATA_BYTE_COUNT: usize = 128;
    const TEST_BANK_DELAY: u8 = 0;

    #[derive(Deserialize)]
    struct PlaybackOracle {
        name: String,
        clip_index: u16,
        mode: String,
        backend: String,
        descriptor: Option<[usize; 3]>,
        position_calls: usize,
        mix_operations: Vec<MixOperationOracle>,
        source_bytes: u16,
        source_data_bytes_consumed: usize,
    }

    #[derive(Deserialize)]
    struct MixOperationOracle {
        buffer: usize,
        bytes: usize,
    }

    #[derive(Clone, Copy)]
    struct MixCase {
        statuses: [AudioStreamBufferStatus; AUDIO_STREAM_BUFFER_COUNT],
        lengths: [usize; AUDIO_STREAM_BUFFER_COUNT],
        position: Option<u16>,
        packed: bool,
    }

    #[test]
    fn playback_matches_all_original_backend_and_mixer_vectors() {
        let vectors: Vec<PlaybackOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b8cd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mix_case = mix_case(&vector.name);
            let playback_enabled = vector.descriptor.is_some() || vector.mode == "mix";
            let stream_active = vector.mode == "mix";
            let request = clip_request(vector.clip_index);
            let target_encoded_len = target_encoded_len(&vector, mix_case.packed);
            let clip_data = generated_clip_data(case_index);
            let selected_bank =
                bank_with_clip(request_index(request), target_encoded_len, &clip_data);
            let empty_bank = empty_bank();
            let banks = match request {
                AudioClipRequest::VoiceReaction { .. } => AudioPlaybackBanks {
                    resident_effects: &selected_bank,
                    streamed_dialogue: &empty_bank,
                },
                AudioClipRequest::StreamedDialogue { .. } => AudioPlaybackBanks {
                    resident_effects: &empty_bank,
                    streamed_dialogue: &selected_bank,
                },
            };
            let mut state = playback_state(
                playback_enabled,
                stream_active,
                vector.name == "idle_memory",
                mix_case,
                case_index,
            );
            let initial_state = state.clone();
            let mut position_calls = 0_usize;

            let outcome = update_audio_playback(&mut state, request, banks, || {
                position_calls += 1;
                mix_case.position
            })
            .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(position_calls, vector.position_calls, "{}", vector.name);
            assert!(
                matches!(vector.backend.as_str(), "memory" | "ems" | "xms" | "file"),
                "{}",
                vector.name
            );
            match outcome {
                AudioPlaybackOutcome::PlaybackDisabled => {
                    assert!(!playback_enabled, "{}", vector.name);
                    assert_eq!(state, initial_state, "{}", vector.name);
                }
                AudioPlaybackOutcome::StopAndPlay(playback) => {
                    assert!(playback_enabled && !stream_active, "{}", vector.name);
                    assert_eq!(playback.request, request, "{}", vector.name);
                    let expected_len = vector.descriptor.unwrap()[2];
                    assert_eq!(playback.encoded_clip.len(), expected_len, "{}", vector.name);
                    assert_eq!(
                        playback.encoded_clip.as_ref(),
                        &clip_data[..expected_len],
                        "{}",
                        vector.name
                    );
                    assert_eq!(state.driver_requests, AudioDriverRequests::default());
                    assert_eq!(state.stream_buffers, initial_state.stream_buffers);
                }
                AudioPlaybackOutcome::StreamMix(report) => {
                    assert!(stream_active, "{}", vector.name);
                    assert_eq!(
                        report.source_output_sample_count, vector.source_bytes,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        report.source_byte_count_consumed, vector.source_data_bytes_consumed,
                        "{}",
                        vector.name
                    );
                    let expected_operations = vector
                        .mix_operations
                        .iter()
                        .map(|operation| AudioMixOperation {
                            buffer_index: operation.buffer,
                            sample_count: operation.bytes,
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        report.operations.as_ref(),
                        expected_operations,
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        report.status,
                        expected_mix_status(&vector.name),
                        "{}",
                        vector.name
                    );
                    let expected_buffers = expected_mixed_buffers(
                        &initial_state.stream_buffers,
                        mix_case,
                        &vector.mix_operations,
                        &clip_data[SND_CLIP_HEADER_BYTE_COUNT..],
                    );
                    assert_eq!(state.stream_buffers, expected_buffers, "{}", vector.name);
                    assert_eq!(state.driver_requests, initial_state.driver_requests);
                }
            }
        }
    }

    #[test]
    fn malformed_mix_source_is_rejected_before_any_buffer_changes() {
        let bank = bank_with_clip(
            0,
            SND_CLIP_HEADER_BYTE_COUNT + RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT,
            &[0; SND_CLIP_HEADER_BYTE_COUNT + RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT],
        );
        let mut state = playback_state(
            true,
            true,
            false,
            MixCase {
                statuses: [
                    AudioStreamBufferStatus::ReadyAndDriverOwned,
                    AudioStreamBufferStatus::Free,
                ],
                lengths: [12, 8],
                position: Some(8),
                packed: false,
            },
            0,
        );
        let before = state.clone();
        let request = AudioClipRequest::VoiceReaction { bank_index: 0 };
        let error = update_audio_playback(
            &mut state,
            request,
            AudioPlaybackBanks {
                resident_effects: &bank,
                streamed_dialogue: &empty_bank(),
            },
            || Some(8),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            AudioPlaybackError::MixSourceTruncated { request: actual, .. }
                if actual == request
        ));
        assert_eq!(state, before);
    }

    fn clip_request(original_index: u16) -> AudioClipRequest {
        if original_index & ORIGINAL_STREAMED_CLIP_MARKER == 0 {
            AudioClipRequest::VoiceReaction {
                bank_index: original_index,
            }
        } else {
            AudioClipRequest::StreamedDialogue {
                index: original_index & ORIGINAL_STREAMED_CLIP_INDEX_MASK,
            }
        }
    }

    const fn request_index(request: AudioClipRequest) -> usize {
        match request {
            AudioClipRequest::VoiceReaction { bank_index } => bank_index as usize,
            AudioClipRequest::StreamedDialogue { index } => index as usize,
        }
    }

    fn target_encoded_len(vector: &PlaybackOracle, packed: bool) -> usize {
        if let Some(descriptor) = vector.descriptor {
            return if vector.backend == "memory" {
                descriptor[2] + RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT
            } else {
                descriptor[2]
            };
        }
        let physical_source_count = if packed {
            usize::from(vector.source_bytes / 2)
        } else {
            usize::from(vector.source_bytes)
        };
        if vector.backend == "memory" {
            physical_source_count + RESIDENT_DRIVER_TERMINATOR_BYTE_COUNT
        } else {
            physical_source_count + SND_CLIP_HEADER_BYTE_COUNT
        }
    }

    fn generated_clip_data(case_index: usize) -> [u8; TEST_CLIP_DATA_BYTE_COUNT] {
        std::array::from_fn(|index| (index * 29 + case_index * 37 + 49) as u8)
    }

    fn bank_with_clip(index: usize, encoded_len: usize, payload: &[u8]) -> SndBank {
        assert!(encoded_len <= payload.len());
        let clip_count = index + 2;
        let mut offsets = vec![0_usize; clip_count + 1];
        offsets[index + 1] = encoded_len;
        offsets[index + 2] = payload.len();
        let mut encoded = Vec::new();
        encoded.extend_from_slice(&(clip_count as u16).to_le_bytes());
        encoded.extend_from_slice(&[TEST_BANK_DELAY, TEST_BANK_DELAY]);
        for offset in offsets {
            encoded.extend_from_slice(&(offset as u32).to_le_bytes());
        }
        encoded.extend_from_slice(payload);
        SndBank::decode(&encoded).unwrap()
    }

    fn empty_bank() -> SndBank {
        SndBank::decode(&[0, 0, TEST_BANK_DELAY, TEST_BANK_DELAY, 0, 0, 0, 0]).unwrap()
    }

    fn playback_state(
        playback_enabled: bool,
        stream_active: bool,
        stream_start_requested: bool,
        case: MixCase,
        case_index: usize,
    ) -> AudioPlaybackState {
        let buffers = std::array::from_fn(|buffer_index| {
            let header = std::array::from_fn(|index| {
                (index * (11 + buffer_index * 6) + case_index * 19 + 85) as u8
            });
            let samples = (0..case.lengths[buffer_index])
                .map(|sample_index| {
                    let original_index = SND_CLIP_HEADER_BYTE_COUNT + sample_index;
                    (original_index * (11 + buffer_index * 6) + case_index * 19 + 85) as u8
                })
                .collect::<Vec<_>>()
                .into_boxed_slice();
            AudioStreamBuffer {
                header,
                samples,
                status: case.statuses[buffer_index],
            }
        });
        AudioPlaybackState {
            playback_enabled,
            driver_requests: AudioDriverRequests {
                stream_start_requested,
                stream_active,
            },
            packed_stream_samples: case.packed,
            stream_buffers: buffers,
        }
    }

    fn mix_case(name: &str) -> MixCase {
        use AudioStreamBufferStatus::{DriverOwned, Free, Ready, ReadyAndDriverOwned};

        match name {
            "mix_memory_spans_buffers" | "mix_memory_adds_bank_offset" => MixCase {
                statuses: [ReadyAndDriverOwned, Free],
                lengths: [12, 8],
                position: Some(8),
                packed: false,
            },
            "mix_memory_packed" => MixCase {
                statuses: [ReadyAndDriverOwned, Free],
                lengths: [10, 10],
                position: Some(7),
                packed: true,
            },
            "mix_second_buffer_selected" => MixCase {
                statuses: [Ready, ReadyAndDriverOwned],
                lengths: [9, 11],
                position: Some(9),
                packed: false,
            },
            "mix_no_active_buffer" => MixCase {
                statuses: [Ready, DriverOwned],
                lengths: [12, 12],
                position: Some(4),
                packed: false,
            },
            "mix_position_unavailable" => MixCase {
                statuses: [ReadyAndDriverOwned, Ready],
                lengths: [12, 12],
                position: None,
                packed: false,
            },
            "mix_ems_cross_page" => MixCase {
                statuses: [ReadyAndDriverOwned, Free],
                lengths: [20, 10],
                position: Some(20),
                packed: false,
            },
            "mix_xms_spans_buffers" => MixCase {
                statuses: [ReadyAndDriverOwned, Free],
                lengths: [15, 10],
                position: Some(15),
                packed: false,
            },
            "mix_file_short_read" => MixCase {
                statuses: [ReadyAndDriverOwned, Free],
                lengths: [12, 10],
                position: Some(12),
                packed: false,
            },
            _ => MixCase {
                statuses: [Free, Free],
                lengths: [16, 16],
                position: Some(0),
                packed: false,
            },
        }
    }

    fn expected_mix_status(name: &str) -> AudioMixStatus {
        match name {
            "mix_no_active_buffer" => AudioMixStatus::NoActiveBuffer,
            "mix_position_unavailable" => AudioMixStatus::PositionUnavailable,
            _ => AudioMixStatus::Mixed,
        }
    }

    fn expected_mixed_buffers(
        initial: &[AudioStreamBuffer; AUDIO_STREAM_BUFFER_COUNT],
        case: MixCase,
        operations: &[MixOperationOracle],
        source: &[u8],
    ) -> [AudioStreamBuffer; AUDIO_STREAM_BUFFER_COUNT] {
        let mut expected = initial.clone();
        let active_index = case
            .statuses
            .iter()
            .position(|status| status.accepts_voice_mix());
        let mut source_cursor = 0_usize;
        for (operation_index, operation) in operations.iter().enumerate() {
            let destination_offset =
                if operation_index == 0 && Some(operation.buffer) == active_index {
                    let length = case.lengths[operation.buffer] as u16;
                    let delta = case.position.unwrap().wrapping_sub(length);
                    usize::from(if (delta as i16).is_negative() {
                        0_u16.wrapping_sub(delta)
                    } else {
                        delta
                    })
                } else {
                    0
                };
            let destination = &mut expected[operation.buffer].samples
                [destination_offset..destination_offset + operation.bytes];
            let mut remaining = operation.bytes as u16;
            for sample in destination {
                let source_sample = source[source_cursor];
                if !case.packed || remaining.is_multiple_of(2) {
                    source_cursor += 1;
                }
                *sample = ((u16::from(*sample) + u16::from(source_sample)) / 2) as u8;
                remaining -= 1;
            }
        }
        expected
    }
}
