//! Flat presentation-resource selection, palette, and ownership state.

use std::error::Error;
use std::fmt;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

use super::{IndexedGamePalette, PresentationQueueState};

const PRESENTATION_ACTIVE_FLAG: u8 = 1;
const PRESENTATION_REQUEST_FLAG: u8 = 2;
const SHIP_BRIDGE_REDRAW_FLAG: u16 = 8;
const PALETTE_BLOCK_HEADER_BYTE_COUNT: usize = 2;
const PALETTE_BLOCK_START_MASK: u16 = u8::MAX as u16;
const PALETTE_BLOCK_COUNT_SHIFT: u32 = u8::BITS;
const PALETTE_BLOCK_TERMINATOR: u16 = u16::MAX;
const PALETTE_SNAPSHOT_COPY_SUPPRESSED_FLAG: u8 = 1;
const PALETTE_METRIC_DIVISOR: usize = 4;
const PALETTE_METRIC_TRAILER_UNITS: usize = 2;
const FIXED_WORD_COPY_COUNT: usize = 4;

/// Number of leading palette colors mirrored into presentation render state.
pub const PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT: usize = 128;

/// Mutable presentation flags owned by the game loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationUpdateState {
    /// Low bit marks an active streamed presentation; higher bits are retained.
    pub gate_flags: u8,
    /// Existing bridge redraw state, overwritten only by the authored ship gate.
    pub bridge_redraw_pending: u8,
    /// Active dialogue line, or no line after presentation teardown.
    pub active_line: Option<u16>,
    /// Pending presentation requests; only the authored request bit is cleared.
    pub request_flags: u8,
}

/// Observable result of one presentation teardown check.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationUpdateOutcome {
    /// The presentation gate was inactive and no state changed.
    Inactive,
    /// Active presentation state was released.
    Finished {
        /// The queue became empty and its owned source may be dropped.
        close_source: bool,
    },
}

/// Finish an active streamed presentation and release its request state.
///
/// This translates `presentation_update_1fb2` at BLOODPRG offset `0x009F53`.
/// Typed optional line state replaces the native maximum-word sentinel.
pub fn finish_presentation_update(
    queue: &mut PresentationQueueState,
    state: &mut PresentationUpdateState,
    ship_flags: u16,
) -> PresentationUpdateOutcome {
    if state.gate_flags & PRESENTATION_ACTIVE_FLAG == u8::MIN {
        return PresentationUpdateOutcome::Inactive;
    }

    let close_source = queue.finish_source();
    if ship_flags & SHIP_BRIDGE_REDRAW_FLAG != u16::MIN {
        state.bridge_redraw_pending = PRESENTATION_ACTIVE_FLAG;
    }
    state.active_line = None;
    state.gate_flags = u8::MIN;
    state.request_flags &= !PRESENTATION_REQUEST_FLAG;
    PresentationUpdateOutcome::Finished { close_source }
}

/// Resolve an authored presentation resource without 16-bit table aliasing.
///
/// This translates `lookup_table_1fb5` at `0x009F80`. Invalid indices return
/// `None` instead of wrapping a near pointer into unrelated game data.
pub fn presentation_resource_descriptor<T>(descriptors: &[T], index: u16) -> Option<&T> {
    descriptors.get(usize::from(index))
}

/// Palette state shared by streamed presentation decoding and indexed drawing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationPaletteState {
    /// Complete palette used by indexed game artwork.
    pub live: IndexedGamePalette,
    /// Presentation renderer's retained palette snapshot.
    pub render_snapshot: IndexedGamePalette,
    /// Palette upload request consumed by the renderer.
    pub dirty: bool,
}

impl Default for PresentationPaletteState {
    fn default() -> Self {
        Self {
            live: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            render_snapshot: [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            dirty: false,
        }
    }
}

/// Malformed serialized palette-block data.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PaletteBlockDecodeError {
    /// The stream ended before a complete two-byte block header.
    TruncatedHeader {
        /// Byte position at which the header should begin.
        position: usize,
    },
    /// A block would extend beyond the indexed palette.
    ColorsOutOfRange {
        /// First palette index selected by the block.
        first_color: usize,
        /// Number of colors selected by the block.
        color_count: usize,
    },
    /// The stream ended before all RGB components declared by a block.
    TruncatedComponents {
        /// Number of component bytes declared by the block.
        required: usize,
        /// Number of component bytes remaining in the stream.
        available: usize,
    },
}

impl fmt::Display for PaletteBlockDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation palette blocks: {self:?}")
    }
}

impl Error for PaletteBlockDecodeError {}

/// Invalid presentation-specific palette update state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationPaletteError {
    /// Serialized palette blocks were malformed.
    Decode(PaletteBlockDecodeError),
    /// Native metric subtraction would underflow before unit conversion.
    EntryMetricUnderflow {
        /// Metric value before consuming this stream.
        metric: usize,
        /// Number of bytes consumed through the terminator.
        consumed: usize,
    },
    /// Native metric conversion would underflow its two-unit trailer.
    EntryMetricTrailerUnderflow {
        /// Remaining bytes after consuming this stream.
        remaining: usize,
    },
    /// A queue payload position lies outside its owned byte buffer.
    PayloadOffsetOutOfRange {
        /// Requested payload start.
        payload_offset: usize,
        /// Available queue-buffer length.
        buffer_len: usize,
    },
}

impl fmt::Display for PresentationPaletteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation palette update: {self:?}")
    }
}

impl Error for PresentationPaletteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PaletteBlockDecodeError> for PresentationPaletteError {
    fn from(source: PaletteBlockDecodeError) -> Self {
        Self::Decode(source)
    }
}

/// Result of applying one terminated palette-block stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationPaletteOutcome {
    /// Bytes consumed through and including the terminator.
    pub consumed_bytes: usize,
    /// Whether live colors were mirrored into presentation render state.
    pub copied_render_snapshot: bool,
}

/// Decode terminated `{start, count, RGB...}` blocks into a staged palette.
///
/// This shared parser is also used by the ordinary resource cache. Callers own
/// transactionality by decoding into a palette copy before publication.
pub(crate) fn decode_palette_blocks(
    source: &[u8],
    mut cursor: usize,
    palette: &mut IndexedGamePalette,
) -> Result<usize, PaletteBlockDecodeError> {
    loop {
        let header_end = cursor
            .checked_add(PALETTE_BLOCK_HEADER_BYTE_COUNT)
            .ok_or(PaletteBlockDecodeError::TruncatedHeader { position: cursor })?;
        let header_bytes = source
            .get(cursor..header_end)
            .ok_or(PaletteBlockDecodeError::TruncatedHeader { position: cursor })?;
        cursor = header_end;
        let header = u16::from_le_bytes(
            header_bytes
                .try_into()
                .expect("validated two-byte palette-block header"),
        );
        if header == PALETTE_BLOCK_TERMINATOR {
            return Ok(cursor);
        }

        let first_color = usize::from(header & PALETTE_BLOCK_START_MASK);
        let color_count = usize::from(header >> PALETTE_BLOCK_COUNT_SHIFT);
        let end_color = first_color
            .checked_add(color_count)
            .filter(|end| *end <= PALETTE_ENTRY_COUNT)
            .ok_or(PaletteBlockDecodeError::ColorsOutOfRange {
                first_color,
                color_count,
            })?;
        let component_byte_count = color_count * RGB_COMPONENT_COUNT;
        let component_end = cursor.checked_add(component_byte_count).ok_or(
            PaletteBlockDecodeError::TruncatedComponents {
                required: component_byte_count,
                available: source.len().saturating_sub(cursor),
            },
        )?;
        let available = source.len().saturating_sub(cursor);
        let components = source.get(cursor..component_end).ok_or(
            PaletteBlockDecodeError::TruncatedComponents {
                required: component_byte_count,
                available,
            },
        )?;
        for (destination, color) in palette[first_color..end_color]
            .iter_mut()
            .zip(components.chunks_exact(RGB_COMPONENT_COUNT))
        {
            destination.copy_from_slice(color);
        }
        cursor = component_end;
    }
}

/// Mirror the low presentation palette when render updates are not suppressed.
///
/// This translates `flag_gated_2751` at `0x00A117`. Direct palette slices
/// replace its segment swap and fixed-address 384-byte copy.
pub fn synchronize_presentation_palette_snapshot(
    render_update_flags: u8,
    state: &mut PresentationPaletteState,
) -> bool {
    if render_update_flags & PALETTE_SNAPSHOT_COPY_SUPPRESSED_FLAG != u8::MIN {
        return false;
    }

    state.render_snapshot[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT]
        .copy_from_slice(&state.live[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT]);
    true
}

/// Apply one terminated palette-block stream transactionally.
///
/// This translates `resource_palette_blocks_apply` at `0x00A0C3`. Checked
/// slices replace far cursors; malformed streams and native metric underflow
/// leave palette and metric state unchanged.
pub fn apply_presentation_palette_blocks(
    stream: &[u8],
    state: &mut PresentationPaletteState,
    render_update_flags: u8,
    read_wrap_index: u16,
    entry_metric: &mut usize,
) -> Result<PresentationPaletteOutcome, PresentationPaletteError> {
    let mut staged = state.clone();
    staged.dirty = true;
    let consumed_bytes = decode_palette_blocks(stream, usize::MIN, &mut staged.live)?;

    let next_metric = if read_wrap_index == u16::MIN {
        let remaining = entry_metric.checked_sub(consumed_bytes).ok_or(
            PresentationPaletteError::EntryMetricUnderflow {
                metric: *entry_metric,
                consumed: consumed_bytes,
            },
        )?;
        Some(
            (remaining / PALETTE_METRIC_DIVISOR)
                .checked_sub(PALETTE_METRIC_TRAILER_UNITS)
                .ok_or(PresentationPaletteError::EntryMetricTrailerUnderflow { remaining })?,
        )
    } else {
        None
    };

    let copied_render_snapshot =
        synchronize_presentation_palette_snapshot(render_update_flags, &mut staged);
    *state = staged;
    if let Some(next_metric) = next_metric {
        *entry_metric = next_metric;
    }
    Ok(PresentationPaletteOutcome {
        consumed_bytes,
        copied_render_snapshot,
    })
}

/// Apply palette blocks beginning at one owned queue payload position.
///
/// This translates `list_d8c_palette_blocks_apply` at `0x00A778`. A flat
/// buffer and checked index replace the native segment-plus-offset assembly.
pub fn apply_queued_presentation_palette_blocks(
    queue_buffer: &[u8],
    payload_offset: usize,
    state: &mut PresentationPaletteState,
    render_update_flags: u8,
    read_wrap_index: u16,
    entry_metric: &mut usize,
) -> Result<PresentationPaletteOutcome, PresentationPaletteError> {
    let stream = queue_buffer.get(payload_offset..).ok_or(
        PresentationPaletteError::PayloadOffsetOutOfRange {
            payload_offset,
            buffer_len: queue_buffer.len(),
        },
    )?;
    apply_presentation_palette_blocks(
        stream,
        state,
        render_update_flags,
        read_wrap_index,
        entry_metric,
    )
}

/// Ownership class of the active presentation byte source.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum PresentationSourceLease {
    /// No active source is retained.
    #[default]
    Closed,
    /// Bytes belong to the shared main archive and remain available globally.
    SharedArchive,
    /// Bytes belong only to this presentation and may be released.
    Owned,
}

/// Drop an owned presentation source and reset its queue range bounds.
///
/// This translates `close_file_d5b` at `0x00A141`. Ownership replaces DOS
/// handle identity; dropping host-owned bytes cannot reproduce a DOS close
/// error, while the original unconditional post-close bounds reset remains.
pub fn close_owned_presentation_source(
    source: &mut PresentationSourceLease,
    queue: &mut PresentationQueueState,
) -> bool {
    if *source != PresentationSourceLease::Owned {
        return false;
    }

    *source = PresentationSourceLease::Closed;
    queue.initialize_bounds();
    true
}

/// Invalid fixed-word transfer over one flat owned buffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FixedWordCopyError {
    /// The four-word source range lies outside the buffer.
    SourceOutOfRange {
        /// First requested source word.
        source: usize,
        /// Available word count.
        buffer_len: usize,
    },
    /// The four-word destination range lies outside the buffer.
    DestinationOutOfRange {
        /// First requested destination word.
        destination: usize,
        /// Available word count.
        buffer_len: usize,
    },
}

impl fmt::Display for FixedWordCopyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid fixed four-word copy: {self:?}")
    }
}

impl Error for FixedWordCopyError {}

/// Copy exactly four words in forward order, including overlap propagation.
///
/// This translates `mem_copy_words` at `0x00A7E6`. Checked flat indices retain
/// the original sequential overlap behavior and reject 16-bit offset wrapping.
pub fn copy_four_words_forward(
    words: &mut [u16],
    source: usize,
    destination: usize,
) -> Result<(), FixedWordCopyError> {
    let source_end = source
        .checked_add(FIXED_WORD_COPY_COUNT)
        .filter(|end| *end <= words.len())
        .ok_or(FixedWordCopyError::SourceOutOfRange {
            source,
            buffer_len: words.len(),
        })?;
    destination
        .checked_add(FIXED_WORD_COPY_COUNT)
        .filter(|end| *end <= words.len())
        .ok_or(FixedWordCopyError::DestinationOutOfRange {
            destination,
            buffer_len: words.len(),
        })?;

    for relative_index in usize::MIN..source_end - source {
        words[destination + relative_index] = words[source + relative_index];
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const UPDATE_VECTOR_COUNT: usize = 8;
    const RESOURCE_LOOKUP_VECTOR_COUNT: usize = 8;
    const FLAT_RESOURCE_LOOKUP_VECTOR_COUNT: usize = 3;
    const FLAT_RESOURCE_ORACLE_INDEX_LIMIT: u16 = 10;
    const PALETTE_BLOCK_VECTOR_COUNT: usize = 5;
    const FLAT_PALETTE_BLOCK_VECTOR_COUNT: usize = 4;
    const PALETTE_COPY_VECTOR_COUNT: usize = 4;
    const SOURCE_CLOSE_VECTOR_COUNT: usize = 7;
    const QUEUED_PALETTE_VECTOR_COUNT: usize = 4;
    const WORD_COPY_VECTOR_COUNT: usize = 6;
    const FLAT_WORD_COPY_VECTOR_COUNT: usize = 4;
    const LIVE_PALETTE_SEED: usize = 5;
    const LIVE_PALETTE_STEP: usize = 17;
    const SNAPSHOT_PALETTE_SEED: usize = 7;
    const SNAPSHOT_PALETTE_STEP: usize = 29;
    const TEST_ENTRY_METRIC: usize = 64;
    const NONZERO_READ_WRAP_INDEX: u16 = 1;
    const TEST_QUEUE_BUFFER_LEN: usize = u16::MAX as usize + PALETTE_BLOCK_HEADER_BYTE_COUNT;
    const INITIAL_BOUND_READ_INDEX: u16 = 0x1111;
    const INITIAL_BOUND_WRAP_COUNT: u16 = 0x2222;
    const INITIAL_BOUND_READ_LIMIT: u16 = 0x3333;
    const INITIAL_BOUND_SECONDARY_LIMIT: u16 = 0x4444;

    #[derive(Deserialize)]
    struct UpdateOracle {
        name: String,
        initial_gate: u8,
        byte_count: u16,
        ship_flags: u16,
        initial_redraw: u8,
        initial_active_line: u16,
        initial_request_flags: u8,
        initial_list_state: u8,
        result: UpdateOracleResult,
    }

    #[derive(Deserialize)]
    struct UpdateOracleResult {
        gate: u8,
        redraw: u8,
        active_line: u16,
        request_flags: u8,
        list_state: u8,
        byte_count: u16,
    }

    #[derive(Deserialize)]
    struct ResourceLookupOracle {
        name: String,
        index: u16,
        record_offset: u16,
    }

    #[derive(Deserialize)]
    struct PaletteBlockOracle {
        name: String,
        blocks: Vec<PaletteBlockOracleEntry>,
        consumed_bytes: usize,
        copied_render_state: bool,
        initial_wrap_index: u16,
        initial_metric: usize,
        result_metric: usize,
    }

    #[derive(Deserialize)]
    struct PaletteBlockOracleEntry {
        start: u8,
        payload: Vec<u8>,
    }

    #[derive(Deserialize)]
    struct PaletteCopyOracle {
        name: String,
        state: u8,
        copied_384_bytes: bool,
    }

    #[derive(Deserialize)]
    struct SourceCloseOracle {
        name: String,
        initial_handle: u16,
        reserved_handle: u16,
        closed: bool,
        result_bounds: [u16; 4],
    }

    #[derive(Deserialize)]
    struct QueuedPaletteOracle {
        name: String,
        payload_offset: u16,
    }

    #[derive(Deserialize)]
    struct WordCopyOracle {
        name: String,
        source_offset: u16,
        destination_offset: u16,
        copied_words_in_order: [u16; FIXED_WORD_COPY_COUNT],
    }

    fn patterned_palette(seed: usize, step: usize) -> IndexedGamePalette {
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        for (byte_index, byte) in palette.iter_mut().flatten().enumerate() {
            *byte = (byte_index * step + seed) as u8;
        }
        palette
    }

    fn palette_stream(blocks: &[PaletteBlockOracleEntry]) -> Vec<u8> {
        let mut stream = Vec::new();
        for block in blocks {
            assert_eq!(block.payload.len() % RGB_COMPONENT_COUNT, usize::MIN);
            let color_count = block.payload.len() / RGB_COMPONENT_COUNT;
            let header =
                ((color_count as u16) << PALETTE_BLOCK_COUNT_SHIFT) | u16::from(block.start);
            stream.extend_from_slice(&header.to_le_bytes());
            stream.extend_from_slice(&block.payload);
        }
        stream.extend_from_slice(&PALETTE_BLOCK_TERMINATOR.to_le_bytes());
        stream
    }

    fn expected_palette(
        initial: &IndexedGamePalette,
        blocks: &[PaletteBlockOracleEntry],
    ) -> IndexedGamePalette {
        let mut expected = *initial;
        for block in blocks {
            let first = usize::from(block.start);
            for (destination, source) in expected[first..]
                .iter_mut()
                .zip(block.payload.chunks_exact(RGB_COMPONENT_COUNT))
            {
                destination.copy_from_slice(source);
            }
        }
        expected
    }

    #[test]
    fn presentation_update_matches_every_original_vector() {
        let vectors: Vec<UpdateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9f53_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), UPDATE_VECTOR_COUNT);

        for vector in vectors {
            let mut queue = PresentationQueueState {
                queued_bytes: usize::from(vector.byte_count),
                status_bits: vector.initial_list_state,
                ..PresentationQueueState::default()
            };
            let mut state = PresentationUpdateState {
                gate_flags: vector.initial_gate,
                bridge_redraw_pending: vector.initial_redraw,
                active_line: Some(vector.initial_active_line),
                request_flags: vector.initial_request_flags,
            };
            let outcome = finish_presentation_update(&mut queue, &mut state, vector.ship_flags);

            assert_eq!(state.gate_flags, vector.result.gate, "{}", vector.name);
            assert_eq!(
                state.bridge_redraw_pending, vector.result.redraw,
                "{}",
                vector.name
            );
            assert_eq!(
                state.active_line.unwrap_or(u16::MAX),
                vector.result.active_line,
                "{}",
                vector.name
            );
            assert_eq!(
                state.request_flags, vector.result.request_flags,
                "{}",
                vector.name
            );
            assert_eq!(
                queue.status_bits, vector.result.list_state,
                "{}",
                vector.name
            );
            assert_eq!(
                queue.queued_bytes,
                usize::from(vector.result.byte_count),
                "{}",
                vector.name
            );
            assert_eq!(
                matches!(outcome, PresentationUpdateOutcome::Inactive),
                vector.initial_gate & PRESENTATION_ACTIVE_FLAG == u8::MIN,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn resource_lookup_matches_flat_original_vectors_and_rejects_aliases() {
        let vectors: Vec<ResourceLookupOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9f80_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RESOURCE_LOOKUP_VECTOR_COUNT);

        let descriptor_count = usize::from(
            vectors
                .iter()
                .filter(|vector| vector.index < FLAT_RESOURCE_ORACLE_INDEX_LIMIT)
                .map(|vector| vector.index)
                .max()
                .unwrap(),
        ) + 1;
        let mut descriptors = vec![u16::MIN; descriptor_count];
        for vector in vectors
            .iter()
            .filter(|vector| vector.index < FLAT_RESOURCE_ORACLE_INDEX_LIMIT)
        {
            descriptors[usize::from(vector.index)] = vector.record_offset;
        }

        let mut matched = usize::MIN;
        for vector in vectors {
            let result = presentation_resource_descriptor(&descriptors, vector.index);
            if vector.index < descriptor_count as u16 {
                assert_eq!(result, Some(&vector.record_offset), "{}", vector.name);
                matched += 1;
            } else {
                assert_eq!(result, None, "{}", vector.name);
            }
        }
        assert_eq!(matched, FLAT_RESOURCE_LOOKUP_VECTOR_COUNT);
    }

    #[test]
    fn palette_block_application_matches_flat_original_vectors() {
        let vectors: Vec<PaletteBlockOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a0c3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PALETTE_BLOCK_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for vector in vectors {
            let live = patterned_palette(LIVE_PALETTE_SEED, LIVE_PALETTE_STEP);
            let render_snapshot = patterned_palette(SNAPSHOT_PALETTE_SEED, SNAPSHOT_PALETTE_STEP);
            let mut state = PresentationPaletteState {
                live,
                render_snapshot,
                dirty: false,
            };
            let initial_state = state.clone();
            let mut metric = vector.initial_metric;
            let stream = palette_stream(&vector.blocks);
            let result = apply_presentation_palette_blocks(
                &stream,
                &mut state,
                u8::from(!vector.copied_render_state),
                vector.initial_wrap_index,
                &mut metric,
            );

            if vector.name == "metric_underflow" {
                assert_eq!(
                    result,
                    Err(PresentationPaletteError::EntryMetricUnderflow {
                        metric: vector.initial_metric,
                        consumed: vector.consumed_bytes,
                    })
                );
                assert_eq!(state, initial_state);
                assert_eq!(metric, vector.initial_metric);
                continue;
            }

            let outcome = result.unwrap();
            assert_eq!(
                outcome.consumed_bytes, vector.consumed_bytes,
                "{}",
                vector.name
            );
            assert_eq!(
                outcome.copied_render_snapshot, vector.copied_render_state,
                "{}",
                vector.name
            );
            assert_eq!(
                state.live,
                expected_palette(&live, &vector.blocks),
                "{}",
                vector.name
            );
            let mut expected_snapshot = render_snapshot;
            if vector.copied_render_state {
                expected_snapshot[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT]
                    .copy_from_slice(&state.live[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT]);
            }
            assert_eq!(state.render_snapshot, expected_snapshot, "{}", vector.name);
            assert!(state.dirty, "{}", vector.name);
            assert_eq!(metric, vector.result_metric, "{}", vector.name);
            matched += 1;
        }
        assert_eq!(matched, FLAT_PALETTE_BLOCK_VECTOR_COUNT);
    }

    #[test]
    fn palette_snapshot_gate_matches_every_original_vector() {
        let vectors: Vec<PaletteCopyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a117_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PALETTE_COPY_VECTOR_COUNT);

        for vector in vectors {
            let mut state = PresentationPaletteState {
                live: patterned_palette(LIVE_PALETTE_SEED, LIVE_PALETTE_STEP),
                render_snapshot: patterned_palette(SNAPSHOT_PALETTE_SEED, SNAPSHOT_PALETTE_STEP),
                dirty: false,
            };
            let before = state.render_snapshot;
            let copied = synchronize_presentation_palette_snapshot(vector.state, &mut state);
            assert_eq!(copied, vector.copied_384_bytes, "{}", vector.name);
            if copied {
                assert_eq!(
                    state.render_snapshot[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT],
                    state.live[..PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT],
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.render_snapshot[PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT..],
                    before[PRESENTATION_PALETTE_SNAPSHOT_COLOR_COUNT..],
                    "{}",
                    vector.name
                );
            } else {
                assert_eq!(state.render_snapshot, before, "{}", vector.name);
            }
        }
    }

    #[test]
    fn source_close_matches_every_original_ownership_case() {
        let vectors: Vec<SourceCloseOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a141_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SOURCE_CLOSE_VECTOR_COUNT);

        for vector in vectors {
            let mut source = if vector.initial_handle == u16::MIN {
                PresentationSourceLease::Closed
            } else if vector.initial_handle == vector.reserved_handle {
                PresentationSourceLease::SharedArchive
            } else {
                PresentationSourceLease::Owned
            };
            let mut queue = PresentationQueueState {
                read_wrap_index: INITIAL_BOUND_READ_INDEX,
                wrap_count: INITIAL_BOUND_WRAP_COUNT,
                read_wrap_limit: Some(INITIAL_BOUND_READ_LIMIT),
                secondary_wrap_limit: Some(INITIAL_BOUND_SECONDARY_LIMIT),
                ..PresentationQueueState::default()
            };
            let closed = close_owned_presentation_source(&mut source, &mut queue);
            assert_eq!(closed, vector.closed, "{}", vector.name);
            assert_eq!(
                source,
                if vector.closed || vector.initial_handle == u16::MIN {
                    PresentationSourceLease::Closed
                } else {
                    PresentationSourceLease::SharedArchive
                },
                "{}",
                vector.name
            );
            assert_eq!(
                [
                    queue.read_wrap_index,
                    queue.wrap_count,
                    queue.read_wrap_limit.unwrap_or(u16::MAX),
                    queue.secondary_wrap_limit.unwrap_or(u16::MAX),
                ],
                vector.result_bounds,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn queued_palette_wrapper_uses_every_original_payload_offset() {
        let vectors: Vec<QueuedPaletteOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a778_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), QUEUED_PALETTE_VECTOR_COUNT);

        for vector in vectors {
            let payload_offset = usize::from(vector.payload_offset);
            let mut queue_buffer = vec![u8::MIN; TEST_QUEUE_BUFFER_LEN];
            queue_buffer[payload_offset..payload_offset + PALETTE_BLOCK_HEADER_BYTE_COUNT]
                .copy_from_slice(&PALETTE_BLOCK_TERMINATOR.to_le_bytes());
            let mut state = PresentationPaletteState::default();
            let mut metric = TEST_ENTRY_METRIC;
            let outcome = apply_queued_presentation_palette_blocks(
                &queue_buffer,
                payload_offset,
                &mut state,
                PALETTE_SNAPSHOT_COPY_SUPPRESSED_FLAG,
                NONZERO_READ_WRAP_INDEX,
                &mut metric,
            )
            .unwrap();
            assert_eq!(
                outcome.consumed_bytes, PALETTE_BLOCK_HEADER_BYTE_COUNT,
                "{}",
                vector.name
            );
            assert_eq!(metric, TEST_ENTRY_METRIC, "{}", vector.name);
        }
    }

    #[test]
    fn fixed_word_copy_matches_flat_original_vectors_and_rejects_wrapping() {
        let vectors: Vec<WordCopyOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a7e6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), WORD_COPY_VECTOR_COUNT);

        let mut matched = usize::MIN;
        for vector in vectors {
            let source = usize::from(vector.source_offset) / size_of::<u16>();
            let destination = usize::from(vector.destination_offset) / size_of::<u16>();
            let mut words = vec![u16::MIN; usize::from(u16::MAX) / size_of::<u16>() + 1];
            let initial = vector.copied_words_in_order;
            if source + FIXED_WORD_COPY_COUNT <= words.len() {
                words[source..source + FIXED_WORD_COPY_COUNT].copy_from_slice(&initial);
            }
            let before = words.clone();
            let result = copy_four_words_forward(&mut words, source, destination);

            if source + FIXED_WORD_COPY_COUNT > words.len()
                || destination + FIXED_WORD_COPY_COUNT > words.len()
            {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(words, before, "{}", vector.name);
                continue;
            }

            result.unwrap();
            assert_eq!(
                words[destination..destination + FIXED_WORD_COPY_COUNT],
                vector.copied_words_in_order,
                "{}",
                vector.name
            );
            matched += 1;
        }
        assert_eq!(matched, FLAT_WORD_COPY_VECTOR_COUNT);
    }
}
