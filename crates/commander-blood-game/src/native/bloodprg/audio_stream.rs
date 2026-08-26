//! Owned music-stream loading, startup, and double-buffer refill.

use std::fmt;

use commander_blood_formats::snd::SND_CLIP_HEADER_BYTE_COUNT;

use super::{AudioPlaybackState, AudioStreamBuffer, AudioStreamBufferStatus};

/// Bytes skipped before the Creative Voice block stream in a `.VOC` file.
pub const CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT: usize = 26;
/// Payload bytes in one original EMS/XMS/file stream page.
pub const AUDIO_STREAM_PAGE_BYTE_COUNT: usize = 16_384;
/// Authored loading message shown while a music source is prepared.
pub const AUDIO_STREAM_WAIT_PROMPT: &[u8] = b"WAIT COMMANDER ...\r";

const AUDIO_STREAM_BUFFER_COUNT: usize = 2;
const FIRST_STREAM_BUFFER_INDEX: usize = 0;
const LAST_STREAM_BUFFER_INDEX: usize = AUDIO_STREAM_BUFFER_COUNT - 1;
const FIRST_STREAM_PAGE_INDEX: u16 = 0;
const NEXT_STREAM_PAGE_INDEX: u16 = 1;
const STREAM_RATE_CODE_HEADER_INDEX: usize = 4;
const PACKED_STREAM_RATE_CODE: u8 = 211;

/// Validated, owned Creative Voice block stream after its file header.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioStreamSource {
    payload: Box<[u8]>,
    page_count: u16,
    final_page_byte_count: u16,
}

impl AudioStreamSource {
    /// Return the complete owned block stream after the 26-byte file header.
    pub fn payload(&self) -> &[u8] {
        &self.payload
    }

    /// Return the number of 16 KiB pages needed to cover the payload.
    pub const fn page_count(&self) -> u16 {
        self.page_count
    }

    /// Return the authored byte count in the final page.
    pub const fn final_page_byte_count(&self) -> u16 {
        self.final_page_byte_count
    }

    /// Borrow one existing source page by its zero-based page index.
    fn page(&self, index: u16) -> Option<&[u8]> {
        let start = usize::from(index).checked_mul(AUDIO_STREAM_PAGE_BYTE_COUNT)?;
        let end = start
            .checked_add(AUDIO_STREAM_PAGE_BYTE_COUNT)?
            .min(self.payload.len());
        (start < self.payload.len()).then(|| &self.payload[start..end])
    }
}

/// Mutable state owned by the streamed-music lifecycle.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AudioStreamState {
    /// Whether the streamed-music channel is enabled.
    pub channel_active: bool,
    /// Current owned source, if one has been loaded.
    pub source: Option<AudioStreamSource>,
    /// Source page selected by the next refill.
    pub next_page_index: u16,
    /// Header copied from page zero and prefixed to later pages.
    pub block_header: [u8; SND_CLIP_HEADER_BYTE_COUNT],
    /// Whether game state says the authored music resource changed.
    pub music_resource_changed: bool,
}

/// Result of trying to load a new stream source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamLoadOutcome {
    /// Playback or the streamed channel was disabled.
    Inactive,
    /// The source was loaded and the UI should present the authored wait text.
    Loaded {
        /// Game-font bytes without the original trailing NUL.
        wait_prompt: &'static [u8],
    },
}

/// Host operation that replaces one original sound-driver far call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioStreamSubmission {
    /// Driver operation requested by the stream lifecycle.
    pub kind: AudioStreamSubmissionKind,
    /// Stream buffer containing the submitted bytes.
    pub buffer_index: usize,
    /// Source page copied into that buffer.
    pub source_page_index: u16,
    /// Number of buffer samples exposed to the host backend.
    pub sample_count: u16,
    /// Whether the saved six-byte header was prefixed before this page.
    pub header_prefixed: bool,
    /// Buffer the start operation exposes for subsequent refill.
    pub refill_buffer_index: Option<usize>,
}

/// Semantic replacement for original play and service driver vectors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamSubmissionKind {
    /// Start the first stream buffer.
    Start,
    /// Queue a refill while playback remains active.
    Service,
    /// Restart playback because no usable position was reported.
    Restart,
}

/// Result of trying to start a loaded source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamStartOutcome {
    /// A native gate was closed and no state changed.
    Inactive,
    /// Page zero and both stream descriptors were prepared for the host.
    Started(AudioStreamSubmission),
}

/// Semantic playback position supplied by the SDL audio backend.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamPlaybackPosition {
    /// No stream is currently playing, corresponding to native position zero.
    Stopped,
    /// The backend cannot report a position, corresponding to native `0xFFFF`.
    Unavailable,
    /// A stream is active with this backend-specific remaining-byte count.
    Playing(u16),
}

/// Result of one bounded modern refill step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamRefillOutcome {
    /// A native gate was closed and no state changed.
    Inactive,
    /// Both buffers remain owned by an actively playing backend.
    BothBuffersOwned,
    /// One page was prepared and must be submitted to the host backend.
    Submitted(AudioStreamSubmission),
}

/// Malformed source or lifecycle state rejected by the flat implementation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudioStreamError {
    /// The fixed Creative Voice file header is incomplete.
    FileHeaderTruncated {
        /// Available source bytes.
        actual: usize,
    },
    /// The native empty-source loops produce backend-dependent underflow state.
    EmptyPayload,
    /// The flat page count exceeds the original word-sized state field.
    PageCountOverflow {
        /// Number of pages required by the source.
        page_count: usize,
    },
    /// Stream startup was requested before a source was loaded.
    SourceUnavailable,
    /// Page zero cannot provide the six-byte block header.
    BlockHeaderTruncated {
        /// Available bytes in page zero.
        actual: usize,
    },
    /// A wrapped or corrupt page index lies outside the owned source.
    PageOutsideSource {
        /// Invalid requested page.
        page_index: u16,
        /// Number of pages in the source.
        page_count: u16,
    },
}

impl fmt::Display for AudioStreamError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AudioStreamError {}

/// Load and own the Creative Voice block stream after its file header.
///
/// This translates `snd_stream_source_load` at BLOODPRG routine offset
/// `0x00BDB7`. Resource lookup supplies complete bytes, so EMS, XMS, temporary
/// files, seeks, and 32 KiB staging chunks disappear. The gates, 26-byte skip,
/// wait prompt, page accounting, music-change clear, and start request remain.
/// Empty sources are rejected instead of preserving the original backend-
/// dependent page-count underflow.
pub fn load_audio_stream_source(
    playback: &mut AudioPlaybackState,
    stream: &mut AudioStreamState,
    encoded_voc: &[u8],
) -> Result<AudioStreamLoadOutcome, AudioStreamError> {
    if !playback.playback_enabled || !stream.channel_active {
        return Ok(AudioStreamLoadOutcome::Inactive);
    }
    if encoded_voc.len() < CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT {
        return Err(AudioStreamError::FileHeaderTruncated {
            actual: encoded_voc.len(),
        });
    }
    let payload = &encoded_voc[CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT..];
    if payload.is_empty() {
        return Err(AudioStreamError::EmptyPayload);
    }
    let page_count = payload.len().div_ceil(AUDIO_STREAM_PAGE_BYTE_COUNT);
    let page_count = u16::try_from(page_count)
        .map_err(|_| AudioStreamError::PageCountOverflow { page_count })?;
    let remainder = payload.len() % AUDIO_STREAM_PAGE_BYTE_COUNT;
    let final_page_byte_count = if remainder == 0 {
        AUDIO_STREAM_PAGE_BYTE_COUNT as u16
    } else {
        remainder as u16
    };

    stream.source = Some(AudioStreamSource {
        payload: payload.into(),
        page_count,
        final_page_byte_count,
    });
    stream.next_page_index = FIRST_STREAM_PAGE_INDEX;
    stream.music_resource_changed = false;
    playback.driver_requests.stream_start_requested = true;
    playback.driver_requests.stream_active = false;
    Ok(AudioStreamLoadOutcome::Loaded {
        wait_prompt: AUDIO_STREAM_WAIT_PROMPT,
    })
}

/// Prepare page zero and the double-buffer descriptors for stream playback.
///
/// This translates `snd_stream_start` at BLOODPRG routine offset `0x00BBB3`.
/// The source's first page, packed-rate marker, saved block header, fixed buffer
/// capacities, request transition, and first-buffer submission are retained.
pub fn start_audio_stream(
    playback: &mut AudioPlaybackState,
    stream: &mut AudioStreamState,
) -> Result<AudioStreamStartOutcome, AudioStreamError> {
    if !playback.playback_enabled
        || !stream.channel_active
        || (!playback.driver_requests.stream_start_requested
            && !playback.driver_requests.stream_active)
    {
        return Ok(AudioStreamStartOutcome::Inactive);
    }

    let source = stream
        .source
        .as_ref()
        .ok_or(AudioStreamError::SourceUnavailable)?;
    let first_page = source
        .page(FIRST_STREAM_PAGE_INDEX)
        .ok_or(AudioStreamError::SourceUnavailable)?;
    if first_page.len() < SND_CLIP_HEADER_BYTE_COUNT {
        return Err(AudioStreamError::BlockHeaderTruncated {
            actual: first_page.len(),
        });
    }
    let mut header = [0_u8; SND_CLIP_HEADER_BYTE_COUNT];
    header.copy_from_slice(&first_page[..SND_CLIP_HEADER_BYTE_COUNT]);
    let first_buffer = buffer_from_page_zero(
        first_page,
        AUDIO_STREAM_PAGE_BYTE_COUNT,
        AudioStreamBufferStatus::Ready,
        None,
    );
    let second_buffer = AudioStreamBuffer {
        header: [0; SND_CLIP_HEADER_BYTE_COUNT],
        samples: vec![0; AUDIO_STREAM_PAGE_BYTE_COUNT].into_boxed_slice(),
        status: AudioStreamBufferStatus::Free,
    };

    stream.block_header = header;
    stream.next_page_index = NEXT_STREAM_PAGE_INDEX;
    playback.packed_stream_samples =
        header[STREAM_RATE_CODE_HEADER_INDEX] == PACKED_STREAM_RATE_CODE;
    playback.stream_buffers = [first_buffer, second_buffer];
    playback.driver_requests.stream_start_requested = false;
    playback.driver_requests.stream_active = true;

    Ok(AudioStreamStartOutcome::Started(AudioStreamSubmission {
        kind: AudioStreamSubmissionKind::Start,
        buffer_index: FIRST_STREAM_BUFFER_INDEX,
        source_page_index: FIRST_STREAM_PAGE_INDEX,
        sample_count: AUDIO_STREAM_PAGE_BYTE_COUNT as u16,
        header_prefixed: false,
        refill_buffer_index: Some(LAST_STREAM_BUFFER_INDEX),
    }))
}

/// Refill at most one available stream buffer and return one host submission.
///
/// This translates `snd_stream_refill` at BLOODPRG routine offset `0x00BC50`.
/// Buffer preference, driver-owned tests, position-zero/unavailable restart,
/// saved-header prefixing, page advance, final-page length, and play-versus-
/// service choice remain exact. The native routine's synchronous second poll
/// after a driver callback becomes the next host update, avoiding a blocking
/// poll loop around SDL's asynchronous audio device.
pub fn refill_audio_stream<Position>(
    playback: &mut AudioPlaybackState,
    stream: &mut AudioStreamState,
    mut playback_position: Position,
) -> Result<AudioStreamRefillOutcome, AudioStreamError>
where
    Position: FnMut() -> AudioStreamPlaybackPosition,
{
    if !playback.playback_enabled
        || !stream.channel_active
        || !playback.driver_requests.stream_active
    {
        return Ok(AudioStreamRefillOutcome::Inactive);
    }

    let position = playback_position();
    let first_owned = driver_owns(playback.stream_buffers[FIRST_STREAM_BUFFER_INDEX].status);
    let selected_buffer_index = if first_owned {
        LAST_STREAM_BUFFER_INDEX
    } else {
        FIRST_STREAM_BUFFER_INDEX
    };
    if first_owned
        && driver_owns(playback.stream_buffers[LAST_STREAM_BUFFER_INDEX].status)
        && matches!(position, AudioStreamPlaybackPosition::Playing(_))
    {
        return Ok(AudioStreamRefillOutcome::BothBuffersOwned);
    }

    let source = stream
        .source
        .as_ref()
        .ok_or(AudioStreamError::SourceUnavailable)?;
    let page_index = stream.next_page_index;
    let page = source
        .page(page_index)
        .ok_or(AudioStreamError::PageOutsideSource {
            page_index,
            page_count: source.page_count,
        })?;
    let next_page_candidate = page_index.wrapping_add(1);
    let reaches_end = next_page_candidate >= source.page_count;
    let sample_count = if reaches_end {
        source.final_page_byte_count
    } else {
        AUDIO_STREAM_PAGE_BYTE_COUNT as u16
    };
    let header_prefixed = page_index != FIRST_STREAM_PAGE_INDEX;
    let prior_samples = &playback.stream_buffers[selected_buffer_index].samples;
    let replacement = if header_prefixed {
        buffer_from_prefixed_page(
            stream.block_header,
            page,
            usize::from(sample_count),
            AudioStreamBufferStatus::Ready,
        )
    } else {
        if page.len() < SND_CLIP_HEADER_BYTE_COUNT {
            return Err(AudioStreamError::BlockHeaderTruncated { actual: page.len() });
        }
        buffer_from_page_zero(
            page,
            usize::from(sample_count),
            AudioStreamBufferStatus::Ready,
            Some(prior_samples),
        )
    };

    stream.next_page_index = if reaches_end {
        FIRST_STREAM_PAGE_INDEX
    } else {
        next_page_candidate
    };
    playback.stream_buffers[selected_buffer_index] = replacement;
    let kind = match position {
        AudioStreamPlaybackPosition::Playing(_) => AudioStreamSubmissionKind::Service,
        AudioStreamPlaybackPosition::Stopped | AudioStreamPlaybackPosition::Unavailable => {
            let other_buffer_index = LAST_STREAM_BUFFER_INDEX - selected_buffer_index;
            playback.stream_buffers[other_buffer_index].status = AudioStreamBufferStatus::Free;
            AudioStreamSubmissionKind::Restart
        }
    };

    Ok(AudioStreamRefillOutcome::Submitted(AudioStreamSubmission {
        kind,
        buffer_index: selected_buffer_index,
        source_page_index: page_index,
        sample_count,
        header_prefixed,
        refill_buffer_index: None,
    }))
}

const fn driver_owns(status: AudioStreamBufferStatus) -> bool {
    matches!(
        status,
        AudioStreamBufferStatus::DriverOwned | AudioStreamBufferStatus::ReadyAndDriverOwned
    )
}

fn buffer_from_page_zero(
    page: &[u8],
    sample_count: usize,
    status: AudioStreamBufferStatus,
    prior_samples: Option<&[u8]>,
) -> AudioStreamBuffer {
    let mut header = [0_u8; SND_CLIP_HEADER_BYTE_COUNT];
    header.copy_from_slice(&page[..SND_CLIP_HEADER_BYTE_COUNT]);
    let mut samples = vec![0_u8; sample_count];
    if let Some(prior) = prior_samples {
        let retained = prior.len().min(samples.len());
        samples[..retained].copy_from_slice(&prior[..retained]);
    }
    let source_samples = &page[SND_CLIP_HEADER_BYTE_COUNT..];
    let copied = source_samples.len().min(samples.len());
    samples[..copied].copy_from_slice(&source_samples[..copied]);
    AudioStreamBuffer {
        header,
        samples: samples.into_boxed_slice(),
        status,
    }
}

fn buffer_from_prefixed_page(
    header: [u8; SND_CLIP_HEADER_BYTE_COUNT],
    page: &[u8],
    sample_count: usize,
    status: AudioStreamBufferStatus,
) -> AudioStreamBuffer {
    let mut samples = vec![0_u8; sample_count];
    let copied = page.len().min(samples.len());
    samples[..copied].copy_from_slice(&page[..copied]);
    AudioStreamBuffer {
        header,
        samples: samples.into_boxed_slice(),
        status,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::AudioDriverRequests;

    const LOAD_ORACLE_VECTOR_COUNT: usize = 11;
    const START_ORACLE_VECTOR_COUNT: usize = 6;
    const REFILL_ORACLE_VECTOR_COUNT: usize = 9;
    const TEST_FILE_HEADER_BYTE: u8 = 90;
    const TEST_BLOCK_HEADER: [u8; SND_CLIP_HEADER_BYTE_COUNT] = [34, 17, 68, 51, 102, 85];

    #[derive(Deserialize)]
    struct LoadOracle {
        name: String,
        sound_enabled: u8,
        channel_active: u8,
        active: bool,
        source_kind: Option<String>,
        backend: Option<String>,
        payload_bytes: Option<usize>,
        read_chunks: Vec<usize>,
        page_count: Option<u16>,
        final_page_bytes: Option<u16>,
    }

    #[derive(Deserialize)]
    struct StartOracle {
        name: String,
        sound_enabled: u8,
        channel_active: u8,
        start_request: u8,
        started: bool,
        first_page: Option<u16>,
        next_page: Option<u16>,
        packed_header: Option<bool>,
        first_buffer_state: Option<u8>,
        second_buffer_state: Option<u8>,
        pending_after: u8,
    }

    #[derive(Deserialize)]
    struct RefillOracle {
        name: String,
        sound_enabled: u8,
        channel_active: u8,
        stream_pending: u8,
        position: Option<u16>,
        selected_buffer: Option<usize>,
        page_read: Option<u16>,
        header_prefixed: Option<bool>,
        driver_action: Option<String>,
        next_page: u16,
        selected_length: Option<u16>,
    }

    #[test]
    fn source_loader_matches_every_valid_original_storage_vector() {
        let vectors: Vec<LoadOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_bdb7_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), LOAD_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let payload_len = vector.payload_bytes.unwrap_or_default();
            let payload = generated_page_bytes(payload_len, case_index);
            let encoded = encoded_voc(&payload);
            let mut playback = playback_state(vector.sound_enabled & 1 != 0);
            let mut stream = stream_state(vector.channel_active & 1 != 0, None);
            let before_playback = playback.clone();
            let before_stream = stream.clone();
            let result = load_audio_stream_source(&mut playback, &mut stream, &encoded);

            if !vector.active {
                assert_eq!(
                    result,
                    Ok(AudioStreamLoadOutcome::Inactive),
                    "{}",
                    vector.name
                );
                assert_eq!(playback, before_playback, "{}", vector.name);
                assert_eq!(stream, before_stream, "{}", vector.name);
                continue;
            }
            assert!(
                matches!(
                    vector.source_kind.as_deref(),
                    Some("embedded" | "standalone")
                ),
                "{}",
                vector.name
            );
            assert!(
                matches!(vector.backend.as_deref(), Some("ems" | "xms" | "file")),
                "{}",
                vector.name
            );
            assert_eq!(vector.read_chunks.iter().sum::<usize>(), payload_len);
            if payload_len == 0 {
                assert_eq!(
                    result,
                    Err(AudioStreamError::EmptyPayload),
                    "{}",
                    vector.name
                );
                assert_eq!(playback, before_playback, "{}", vector.name);
                assert_eq!(stream, before_stream, "{}", vector.name);
                assert!(vector.page_count.is_some() && vector.final_page_bytes.is_some());
                continue;
            }

            assert_eq!(
                result,
                Ok(AudioStreamLoadOutcome::Loaded {
                    wait_prompt: AUDIO_STREAM_WAIT_PROMPT,
                }),
                "{}",
                vector.name
            );
            let source = stream.source.as_ref().unwrap();
            assert_eq!(source.payload(), payload, "{}", vector.name);
            assert_eq!(
                source.page_count(),
                vector.page_count.unwrap(),
                "{}",
                vector.name
            );
            assert_eq!(
                source.final_page_byte_count(),
                vector.final_page_bytes.unwrap(),
                "{}",
                vector.name
            );
            assert!(!stream.music_resource_changed);
            assert_eq!(
                playback.driver_requests,
                AudioDriverRequests {
                    stream_start_requested: true,
                    stream_active: false,
                }
            );
        }
    }

    #[test]
    fn stream_start_matches_every_original_gate_and_descriptor_vector() {
        let vectors: Vec<StartOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_bbb3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), START_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mut page = generated_page_bytes(AUDIO_STREAM_PAGE_BYTE_COUNT, case_index);
            let packed = vector.packed_header.unwrap_or(false);
            if packed {
                page[STREAM_RATE_CODE_HEADER_INDEX] = PACKED_STREAM_RATE_CODE;
            } else if page[STREAM_RATE_CODE_HEADER_INDEX] == PACKED_STREAM_RATE_CODE {
                page[STREAM_RATE_CODE_HEADER_INDEX] ^= 128;
            }
            let source = source_from_payload(&page).unwrap();
            let mut playback = playback_state(vector.sound_enabled & 1 != 0);
            playback.driver_requests = AudioDriverRequests {
                stream_start_requested: vector.start_request & 1 != 0,
                stream_active: vector.start_request & 2 != 0,
            };
            let mut stream = stream_state(vector.channel_active & 1 != 0, Some(source));
            let before_playback = playback.clone();
            let before_stream = stream.clone();
            let outcome = start_audio_stream(&mut playback, &mut stream)
                .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            if !vector.started {
                assert_eq!(
                    outcome,
                    AudioStreamStartOutcome::Inactive,
                    "{}",
                    vector.name
                );
                assert_eq!(playback, before_playback, "{}", vector.name);
                assert_eq!(stream, before_stream, "{}", vector.name);
                assert_eq!(vector.pending_after, vector.start_request);
                continue;
            }

            assert_eq!(
                outcome,
                AudioStreamStartOutcome::Started(AudioStreamSubmission {
                    kind: AudioStreamSubmissionKind::Start,
                    buffer_index: FIRST_STREAM_BUFFER_INDEX,
                    source_page_index: vector.first_page.unwrap(),
                    sample_count: AUDIO_STREAM_PAGE_BYTE_COUNT as u16,
                    header_prefixed: false,
                    refill_buffer_index: Some(LAST_STREAM_BUFFER_INDEX),
                }),
                "{}",
                vector.name
            );
            assert_eq!(stream.next_page_index, vector.next_page.unwrap());
            assert_eq!(stream.block_header, page[..SND_CLIP_HEADER_BYTE_COUNT]);
            assert_eq!(playback.packed_stream_samples, packed);
            assert_eq!(
                playback.stream_buffers[FIRST_STREAM_BUFFER_INDEX].status,
                status_from_original(vector.first_buffer_state.unwrap())
            );
            assert_eq!(
                playback.stream_buffers[LAST_STREAM_BUFFER_INDEX].status,
                status_from_original(vector.second_buffer_state.unwrap())
            );
            assert_eq!(
                playback.stream_buffers[FIRST_STREAM_BUFFER_INDEX].samples
                    [..AUDIO_STREAM_PAGE_BYTE_COUNT - SND_CLIP_HEADER_BYTE_COUNT],
                page[SND_CLIP_HEADER_BYTE_COUNT..]
            );
            assert_eq!(vector.pending_after, 2);
            assert_eq!(
                playback.driver_requests,
                AudioDriverRequests {
                    stream_start_requested: false,
                    stream_active: true,
                }
            );
        }
    }

    #[test]
    fn refill_matches_valid_original_selection_page_and_driver_vectors() {
        let vectors: Vec<RefillOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_bc50_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), REFILL_ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let (statuses, page_count, final_page_byte_count) = refill_case(&vector.name);
            let payload_len =
                (page_count - 1) * AUDIO_STREAM_PAGE_BYTE_COUNT + final_page_byte_count;
            let payload = generated_page_bytes(payload_len, case_index);
            let source = source_from_payload(&payload).unwrap();
            let mut playback = playback_state(vector.sound_enabled & 1 != 0);
            playback.driver_requests = AudioDriverRequests {
                stream_start_requested: vector.stream_pending & 1 != 0,
                stream_active: vector.stream_pending & 2 != 0,
            };
            playback.stream_buffers[FIRST_STREAM_BUFFER_INDEX].status = statuses[0];
            playback.stream_buffers[LAST_STREAM_BUFFER_INDEX].status = statuses[1];
            let mut stream = stream_state(vector.channel_active & 1 != 0, Some(source));
            stream.block_header = TEST_BLOCK_HEADER;
            stream.next_page_index = vector.page_read.unwrap_or(vector.next_page);
            if vector.name == "both_busy_return" {
                stream.next_page_index = 1;
            }
            let before_playback = playback.clone();
            let before_stream = stream.clone();
            let mut position_calls = 0_usize;
            let position = playback_position(vector.position);
            let outcome = refill_audio_stream(&mut playback, &mut stream, || {
                position_calls += 1;
                position
            });

            if vector.name == "page_word_wrap" {
                assert_eq!(
                    outcome,
                    Err(AudioStreamError::PageOutsideSource {
                        page_index: u16::MAX,
                        page_count: page_count as u16,
                    })
                );
                assert_eq!(position_calls, 1);
                assert_eq!(playback, before_playback);
                assert_eq!(stream, before_stream);
                continue;
            }
            if vector.position.is_none() {
                assert_eq!(
                    outcome,
                    Ok(AudioStreamRefillOutcome::Inactive),
                    "{}",
                    vector.name
                );
                assert_eq!(position_calls, 0, "{}", vector.name);
                assert_eq!(playback, before_playback, "{}", vector.name);
                assert_eq!(stream, before_stream, "{}", vector.name);
                continue;
            }
            assert_eq!(position_calls, 1, "{}", vector.name);
            if vector.selected_buffer.is_none() {
                assert_eq!(
                    outcome,
                    Ok(AudioStreamRefillOutcome::BothBuffersOwned),
                    "{}",
                    vector.name
                );
                assert_eq!(playback, before_playback, "{}", vector.name);
                assert_eq!(stream, before_stream, "{}", vector.name);
                continue;
            }

            let selected = vector.selected_buffer.unwrap();
            let page_index = vector.page_read.unwrap();
            let expected_kind = match vector.driver_action.as_deref() {
                Some("service") => AudioStreamSubmissionKind::Service,
                Some("play") => AudioStreamSubmissionKind::Restart,
                other => panic!("{}: unexpected driver action {other:?}", vector.name),
            };
            assert_eq!(
                outcome,
                Ok(AudioStreamRefillOutcome::Submitted(AudioStreamSubmission {
                    kind: expected_kind,
                    buffer_index: selected,
                    source_page_index: page_index,
                    sample_count: vector.selected_length.unwrap(),
                    header_prefixed: vector.header_prefixed.unwrap(),
                    refill_buffer_index: None,
                })),
                "{}",
                vector.name
            );
            assert_eq!(stream.next_page_index, vector.next_page, "{}", vector.name);
            let page_start = usize::from(page_index) * AUDIO_STREAM_PAGE_BYTE_COUNT;
            let source_page =
                &payload[page_start..payload.len().min(page_start + AUDIO_STREAM_PAGE_BYTE_COUNT)];
            if vector.header_prefixed.unwrap() {
                assert_eq!(playback.stream_buffers[selected].header, TEST_BLOCK_HEADER);
                assert_eq!(
                    &playback.stream_buffers[selected].samples[..source_page.len()],
                    source_page,
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(
                    playback.stream_buffers[selected].header,
                    source_page[..SND_CLIP_HEADER_BYTE_COUNT]
                );
                assert_eq!(
                    &playback.stream_buffers[selected].samples
                        [..source_page.len() - SND_CLIP_HEADER_BYTE_COUNT],
                    &source_page[SND_CLIP_HEADER_BYTE_COUNT..],
                    "{}",
                    vector.name
                );
            }
            assert_eq!(
                playback.stream_buffers[selected].samples.len(),
                usize::from(vector.selected_length.unwrap())
            );
            assert_eq!(
                playback.stream_buffers[selected].status,
                AudioStreamBufferStatus::Ready
            );
            if expected_kind == AudioStreamSubmissionKind::Restart {
                assert_eq!(
                    playback.stream_buffers[LAST_STREAM_BUFFER_INDEX - selected].status,
                    AudioStreamBufferStatus::Free
                );
            }
        }
    }

    fn playback_state(playback_enabled: bool) -> AudioPlaybackState {
        let buffer = |status| AudioStreamBuffer {
            header: [0; SND_CLIP_HEADER_BYTE_COUNT],
            samples: vec![128; AUDIO_STREAM_PAGE_BYTE_COUNT].into_boxed_slice(),
            status,
        };
        AudioPlaybackState {
            playback_enabled,
            driver_requests: AudioDriverRequests::default(),
            packed_stream_samples: false,
            stream_buffers: [
                buffer(AudioStreamBufferStatus::Free),
                buffer(AudioStreamBufferStatus::Free),
            ],
        }
    }

    fn stream_state(channel_active: bool, source: Option<AudioStreamSource>) -> AudioStreamState {
        AudioStreamState {
            channel_active,
            source,
            next_page_index: FIRST_STREAM_PAGE_INDEX,
            block_header: [0; SND_CLIP_HEADER_BYTE_COUNT],
            music_resource_changed: true,
        }
    }

    fn encoded_voc(payload: &[u8]) -> Vec<u8> {
        let mut encoded = vec![TEST_FILE_HEADER_BYTE; CREATIVE_VOICE_FILE_HEADER_BYTE_COUNT];
        encoded.extend_from_slice(payload);
        encoded
    }

    fn source_from_payload(payload: &[u8]) -> Result<AudioStreamSource, AudioStreamError> {
        let mut playback = playback_state(true);
        let mut stream = stream_state(true, None);
        load_audio_stream_source(&mut playback, &mut stream, &encoded_voc(payload))?;
        Ok(stream.source.unwrap())
    }

    fn generated_page_bytes(byte_count: usize, case_index: usize) -> Vec<u8> {
        (0..byte_count)
            .map(|index| (index * 29 + case_index * 37 + 5) as u8)
            .collect()
    }

    const fn status_from_original(state: u8) -> AudioStreamBufferStatus {
        match state {
            0 => AudioStreamBufferStatus::Free,
            1 => AudioStreamBufferStatus::Ready,
            2 => AudioStreamBufferStatus::DriverOwned,
            3 => AudioStreamBufferStatus::ReadyAndDriverOwned,
            _ => panic!("unsupported original stream-buffer state"),
        }
    }

    fn playback_position(position: Option<u16>) -> AudioStreamPlaybackPosition {
        match position {
            Some(0) => AudioStreamPlaybackPosition::Stopped,
            Some(u16::MAX) => AudioStreamPlaybackPosition::Unavailable,
            Some(position) => AudioStreamPlaybackPosition::Playing(position),
            None => AudioStreamPlaybackPosition::Unavailable,
        }
    }

    fn refill_case(
        name: &str,
    ) -> (
        [AudioStreamBufferStatus; AUDIO_STREAM_BUFFER_COUNT],
        usize,
        usize,
    ) {
        use AudioStreamBufferStatus::{DriverOwned, Free};

        match name {
            "both_busy_return" => ([DriverOwned, DriverOwned], 4, 8_738),
            "first_page_service" => ([Free, DriverOwned], 4, 8_738),
            "prefixed_page_service" => ([Free, DriverOwned], 5, 8_738),
            "second_buffer_play" => ([DriverOwned, Free], 5, 8_738),
            "busy_minus_one_final" => ([DriverOwned, DriverOwned], 4, 291),
            "page_word_wrap" => ([Free, DriverOwned], 5, 13_398),
            _ => ([Free, Free], 4, 8_738),
        }
    }
}
