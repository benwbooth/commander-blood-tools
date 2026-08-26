//! Encounter CD-audio metadata and playback state.

const CD_FRAMES_PER_SECOND: u32 = 75;
const SECONDS_PER_MINUTE: u32 = 60;
const CD_FRAMES_PER_MINUTE: u32 = CD_FRAMES_PER_SECOND * SECONDS_PER_MINUTE;
const CD_LEAD_IN_FRAMES: u32 = 150;

/// Physical CD track selected by the original 3D encounter.
pub const ENCOUNTER_CD_TRACK_NUMBER: u8 = 2;
/// Original per-channel volume programmed before encounter playback.
pub const ENCOUNTER_CD_CHANNEL_VOLUME: u8 = 80;

/// One source channel in the original four-channel CD mixer request.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdAudioInputChannel {
    /// Input channel zero.
    Zero,
    /// Input channel one.
    One,
    /// Input channel two.
    Two,
    /// Input channel three.
    Three,
}

/// One original CD input-to-output routing entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CdAudioChannelMix {
    /// Input channel routed to the corresponding output channel.
    pub input: CdAudioInputChannel,
    /// Linear MSCDEX volume retained as authored metadata.
    pub volume: u8,
}

/// Four-channel encounter mix installed during game startup.
pub const ENCOUNTER_CD_CHANNEL_MIX: [CdAudioChannelMix; 4] = [
    CdAudioChannelMix {
        input: CdAudioInputChannel::Zero,
        volume: ENCOUNTER_CD_CHANNEL_VOLUME,
    },
    CdAudioChannelMix {
        input: CdAudioInputChannel::One,
        volume: ENCOUNTER_CD_CHANNEL_VOLUME,
    },
    CdAudioChannelMix {
        input: CdAudioInputChannel::Two,
        volume: ENCOUNTER_CD_CHANNEL_VOLUME,
    },
    CdAudioChannelMix {
        input: CdAudioInputChannel::Three,
        volume: ENCOUNTER_CD_CHANNEL_VOLUME,
    },
];

/// Packed frame, second, and minute position returned by MSCDEX.
///
/// The low three bytes are frame, second, and minute respectively. The native
/// routine deliberately ignores the high byte and performs wrapping `u32`
/// arithmetic for malformed positions.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PackedCdPosition(u32);

impl PackedCdPosition {
    /// Preserve one raw position read from original disc metadata.
    pub const fn from_raw(raw: u32) -> Self {
        Self(raw)
    }

    /// Return the original packed representation.
    pub const fn raw(self) -> u32 {
        self.0
    }

    /// Convert the packed position to the native absolute-frame value.
    pub fn absolute_frame(self) -> u32 {
        let frame = self.0 & u32::from(u8::MAX);
        let second = (self.0 >> u8::BITS) & u32::from(u8::MAX);
        let minute = (self.0 >> (u8::BITS * 2)) & u32::from(u8::MAX);
        minute
            .wrapping_mul(CD_FRAMES_PER_MINUTE)
            .wrapping_add(second.wrapping_mul(CD_FRAMES_PER_SECOND))
            .wrapping_add(frame)
            .wrapping_sub(CD_LEAD_IN_FRAMES)
    }
}

/// Source metadata needed to reproduce encounter-track playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EncounterCdTrackMetadata {
    /// Packed start position reported for physical track 2.
    pub start_position: PackedCdPosition,
    /// Packed disc lead-out position used as the playback endpoint.
    pub lead_out_position: PackedCdPosition,
}

impl EncounterCdTrackMetadata {
    /// Build metadata supplied by a modern track-2 asset loader.
    pub const fn new(
        start_position: PackedCdPosition,
        lead_out_position: PackedCdPosition,
    ) -> Self {
        Self {
            start_position,
            lead_out_position,
        }
    }

    fn playback_span(self) -> CdAudioTrackSpan {
        let start_frame = self.start_position.absolute_frame();
        let end_frame = self.lead_out_position.absolute_frame();
        CdAudioTrackSpan {
            track_number: ENCOUNTER_CD_TRACK_NUMBER,
            start_position: self.start_position,
            start_frame,
            frame_count: end_frame.wrapping_sub(start_frame),
        }
    }
}

/// Exact source span requested for encounter playback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CdAudioTrackSpan {
    /// Physical source track.
    pub track_number: u8,
    /// Original packed start position retained for asset matching.
    pub start_position: PackedCdPosition,
    /// Native absolute-frame start after subtracting the lead-in.
    pub start_frame: u32,
    /// Wrapping frame count from track start through disc lead-out.
    pub frame_count: u32,
}

/// Result of preparing the optional original encounter track.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdAudioPreparationOutcome {
    /// No track-2 asset is available, matching the native disabled gate.
    Unavailable,
    /// Track metadata and the authored channel mix are ready.
    Prepared {
        /// Source span selected from track-2 and lead-out metadata.
        span: CdAudioTrackSpan,
        /// Authored channel routing and volume.
        channel_mix: [CdAudioChannelMix; 4],
    },
}

/// Command emitted to the modern audio host.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CdAudioPlaybackCommand {
    /// Start the full encounter track span.
    Play(CdAudioTrackSpan),
    /// Stop physical track 2.
    Stop {
        /// Track whose playback ownership is released.
        track_number: u8,
    },
}

/// Flat runtime state replacing MSCDEX request blocks and drive globals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CdAudioState {
    prepared_track: Option<CdAudioTrackSpan>,
    playing: bool,
}

impl CdAudioState {
    /// Return whether the encounter track currently owns host playback.
    pub const fn is_playing(self) -> bool {
        self.playing
    }

    /// Return the prepared track span, if the source asset was found.
    pub const fn prepared_track(self) -> Option<CdAudioTrackSpan> {
        self.prepared_track
    }
}

/// Translate BLOODPRG routine `0x000B32` to typed source availability.
///
/// The original tested whether MSCDEX reported at least one drive. The modern
/// port instead tests whether the required track-2 source and metadata were
/// actually resolved; an ISO-9660 data-track image alone is not sufficient.
pub const fn detect_cd_audio_source(source: Option<&EncounterCdTrackMetadata>) -> bool {
    source.is_some()
}

/// Translate BLOODPRG routine `0x001344` over flat track metadata.
///
/// The three MSCDEX requests for disc information, track-2 information, and
/// channel routing collapse into one supplied metadata value plus the exact
/// authored four-channel mix. No request buffers or segment aliases survive.
pub fn prepare_cd_audio(
    state: &mut CdAudioState,
    source: Option<EncounterCdTrackMetadata>,
) -> CdAudioPreparationOutcome {
    let Some(source) = source else {
        return CdAudioPreparationOutcome::Unavailable;
    };
    let span = source.playback_span();
    state.prepared_track = Some(span);
    CdAudioPreparationOutcome::Prepared {
        span,
        channel_mix: ENCOUNTER_CD_CHANNEL_MIX,
    }
}

/// Translate BLOODPRG routine `0x0013C4` to a host playback command.
///
/// The returned frame span preserves the original packed-position conversion,
/// ignored high byte, lead-in subtraction, and wrapping endpoint subtraction.
pub fn play_cd_audio_track_two(state: &mut CdAudioState) -> Option<CdAudioPlaybackCommand> {
    let span = state.prepared_track?;
    state.playing = true;
    Some(CdAudioPlaybackCommand::Play(span))
}

/// Translate BLOODPRG routine `0x001397` to a host stop command.
///
/// As in the original, a stop is emitted whenever CD audio is available, even
/// if no prior play command is represented in the local state.
pub fn stop_cd_audio(state: &mut CdAudioState) -> Option<CdAudioPlaybackCommand> {
    state.prepared_track?;
    state.playing = false;
    Some(CdAudioPlaybackCommand::Stop {
        track_number: ENCOUNTER_CD_TRACK_NUMBER,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const HEX_RADIX: u32 = 16;
    const TEST_TRACK_START: PackedCdPosition = PackedCdPosition::from_raw(0x00120320);
    const TEST_DISC_LEAD_OUT: PackedCdPosition = PackedCdPosition::from_raw(0x003A1025);

    #[derive(Deserialize)]
    struct DetectionVector {
        drive_count: u16,
        cdrom_present: u8,
    }

    #[derive(Deserialize)]
    struct PreparationVector {
        name: String,
        cdrom_present: u8,
        interrupts: Vec<serde_json::Value>,
    }

    #[derive(Deserialize)]
    struct PlaybackVector {
        name: String,
        cdrom_present: u8,
        start_position: String,
        end_position: String,
        start_frame: u32,
        duration: u32,
        interrupts: Vec<serde_json::Value>,
    }

    #[derive(Deserialize)]
    struct StopVector {
        name: String,
        cdrom_present: u8,
        interrupts: Vec<serde_json::Value>,
    }

    #[test]
    fn source_detection_matches_every_original_drive_count_vector() {
        let vectors: Vec<DetectionVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_0b32_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 5);
        let metadata = metadata(TEST_TRACK_START, TEST_DISC_LEAD_OUT);
        for vector in vectors {
            let source = (vector.drive_count != u16::MIN).then_some(&metadata);
            assert_eq!(
                detect_cd_audio_source(source),
                vector.cdrom_present != u8::MIN
            );
        }
    }

    #[test]
    fn preparation_matches_every_original_gate_and_channel_vector() {
        let vectors: Vec<PreparationVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1344_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 5);
        for vector in vectors {
            let enabled = vector.cdrom_present & 1 != u8::MIN;
            let source = enabled.then_some(metadata(TEST_TRACK_START, TEST_DISC_LEAD_OUT));
            let mut state = CdAudioState::default();
            let outcome = prepare_cd_audio(&mut state, source);
            assert_eq!(
                matches!(outcome, CdAudioPreparationOutcome::Prepared { .. }),
                enabled,
                "{}",
                vector.name
            );
            assert_eq!(
                vector.interrupts.len(),
                if enabled { 3 } else { 0 },
                "{}",
                vector.name
            );
            if let CdAudioPreparationOutcome::Prepared { span, channel_mix } = outcome {
                assert_eq!(
                    span,
                    metadata(TEST_TRACK_START, TEST_DISC_LEAD_OUT).playback_span()
                );
                assert_eq!(channel_mix, ENCOUNTER_CD_CHANNEL_MIX);
                assert_eq!(state.prepared_track(), Some(span));
            } else {
                assert_eq!(state, CdAudioState::default());
            }
        }
    }

    #[test]
    fn playback_matches_every_original_packed_position_vector() {
        let vectors: Vec<PlaybackVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_13c4_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 6);
        for vector in vectors {
            let enabled = vector.cdrom_present & 1 != u8::MIN;
            let source = enabled.then_some(metadata(
                parse_position(&vector.start_position),
                parse_position(&vector.end_position),
            ));
            let mut state = CdAudioState::default();
            prepare_cd_audio(&mut state, source);
            let command = play_cd_audio_track_two(&mut state);
            assert_eq!(command.is_some(), enabled, "{}", vector.name);
            assert_eq!(
                vector.interrupts.len(),
                usize::from(enabled),
                "{}",
                vector.name
            );
            if let Some(CdAudioPlaybackCommand::Play(span)) = command {
                assert_eq!(span.start_frame, vector.start_frame, "{}", vector.name);
                assert_eq!(span.frame_count, vector.duration, "{}", vector.name);
                assert!(state.is_playing(), "{}", vector.name);
            } else {
                assert!(!state.is_playing(), "{}", vector.name);
            }
        }
    }

    #[test]
    fn stop_matches_every_original_availability_vector() {
        let vectors: Vec<StopVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1397_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 4);
        for vector in vectors {
            let enabled = vector.cdrom_present & 1 != u8::MIN;
            let source = enabled.then_some(metadata(TEST_TRACK_START, TEST_DISC_LEAD_OUT));
            let mut state = CdAudioState::default();
            prepare_cd_audio(&mut state, source);
            let command = stop_cd_audio(&mut state);
            assert_eq!(command.is_some(), enabled, "{}", vector.name);
            assert_eq!(
                vector.interrupts.len(),
                usize::from(enabled),
                "{}",
                vector.name
            );
            assert!(!state.is_playing(), "{}", vector.name);
        }
    }

    fn metadata(
        start_position: PackedCdPosition,
        lead_out_position: PackedCdPosition,
    ) -> EncounterCdTrackMetadata {
        EncounterCdTrackMetadata::new(start_position, lead_out_position)
    }

    fn parse_position(value: &str) -> PackedCdPosition {
        PackedCdPosition::from_raw(
            u32::from_str_radix(value.trim_start_matches("0x"), HEX_RADIX).unwrap(),
        )
    }
}
