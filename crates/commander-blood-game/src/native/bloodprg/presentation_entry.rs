//! Typed activation of extent-prefixed presentation queue entries.

use std::error::Error;
use std::fmt;

use super::{
    PresentationDispatchError, PresentationEntryActivationRequest, PresentationEntryStorage,
    PresentationPayload, decode_presentation_payload,
};

const WORD_BYTE_COUNT: usize = size_of::<u16>();
const LINK_ID_BYTE_COUNT: usize = size_of::<u32>();
const LINK_RECORD_BYTE_COUNT: usize = LINK_ID_BYTE_COUNT + WORD_BYTE_COUNT;
#[cfg(test)]
const FRAME_HEADER_BYTE_COUNT: usize = WORD_BYTE_COUNT * 2;
const MINIMUM_SIDE_RECORD_EXTENT: usize = WORD_BYTE_COUNT * 2;
const SOUND_RECORD_MARKER: u16 = u16::from_le_bytes([b's', b'd']);
const PALETTE_RECORD_MARKER: u16 = u16::from_le_bytes([b'p', b'l']);
const LINK_RECORD_MARKER: u16 = u16::from_le_bytes([b'm', b'm']);
const COMPRESSED_LAYOUT_FLAG: u16 = 0x0200;
const NO_COORDINATES_LAYOUT_FLAG: u16 = 0x0400;
const TRANSPARENT_ROW_MODE: u8 = u8::MAX;

/// Opaque identifier for an entry retained in the flat presentation queue.
///
/// The native queue wrote a far pointer into this four-byte field. The modern
/// refill path writes a checked queue position and resolves it without exposing
/// an address or segmented-memory convention.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PresentationLinkId(u32);

impl PresentationLinkId {
    /// Construct a link identifier from the value encoded by the flat queue.
    pub const fn new(value: u32) -> Self {
        Self(value)
    }

    /// Construct an identifier for one representable flat queue position.
    pub fn from_queue_offset(offset: usize) -> Option<Self> {
        u32::try_from(offset).ok().map(Self)
    }

    /// Return the serialized queue identifier.
    pub const fn get(self) -> u32 {
        self.0
    }

    /// Return this identifier as a host queue position.
    pub fn queue_offset(self) -> Option<usize> {
        usize::try_from(self.0).ok()
    }
}

/// Resolve one synthetic link to a complete retained queue entry.
///
/// A linked entry begins with its extent word. Checked flat slicing replaces
/// the original far-pointer dereference and rejects stale or overwritten queue
/// positions before entry activation.
pub fn resolve_presentation_queue_link(
    queue_buffer: &[u8],
    link: PresentationLinkId,
) -> Option<Box<[u8]>> {
    let start = link.queue_offset()?;
    let header_end = start.checked_add(WORD_BYTE_COUNT)?;
    let extent = usize::from(u16::from_le_bytes(
        queue_buffer.get(start..header_end)?.try_into().ok()?,
    ));
    let end = start.checked_add(extent)?;
    (extent >= WORD_BYTE_COUNT)
        .then(|| queue_buffer.get(start..end))
        .flatten()
        .map(Box::from)
}

/// Runtime gates controlling immediate versus deferred frame expansion.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationEntryPolicy {
    /// A sound side record may be published for playback.
    pub sound_enabled: bool,
    /// Back-buffer presentation is currently suppressed.
    pub skip_back_buffer_present: bool,
    /// Frames are currently composed through the back buffer.
    pub draw_via_back_buffer: bool,
}

/// Optional side records preceding a presentation frame.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationEntrySideData {
    /// Complete sound record beginning with its extent word.
    pub sound_record: Option<Box<[u8]>>,
    /// Payload of the final palette record, excluding marker and extent.
    pub palette_payload: Option<Box<[u8]>>,
}

/// Owned frame data published by entry activation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationEntryFrame {
    /// A zero-row frame retains only its typed header.
    Empty,
    /// Uncompressed coordinates and pixels following the typed frame header.
    Encoded(Box<[u8]>),
    /// A compressed frame expanded immediately into reusable storage.
    Decoded(PresentationPayload),
    /// A transparent compressed rectangle retained for direct presentation.
    DeferredTransparent(Box<[u8]>),
}

/// Complete active frame independent of queue or resource-buffer lifetimes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActivatedPresentationEntry {
    /// Authored frame layout and flags.
    pub layout: u16,
    /// Low-byte row count and high-byte presentation mode.
    pub row_mode: u16,
    /// Reusable buffer selected by the authored layout.
    pub storage: PresentationEntryStorage,
    /// Owned encoded or decoded frame representation.
    pub frame: PresentationEntryFrame,
}

/// Terminal result of parsing one ready queue entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationEntryDisposition {
    /// The entry was accepted and published for presentation.
    Active(ActivatedPresentationEntry),
    /// A linked resource changed generation and the stale queue entry must be
    /// consumed without becoming active.
    RejectedLink {
        /// Retained queue entry requested by the link record.
        link: PresentationLinkId,
        /// Generation key captured when the link record was synthesized.
        expected_key: u16,
        /// Current generation key found in the linked resource.
        actual_key: u16,
    },
}

/// Side data and frame state produced by one activation attempt.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationEntryActivation {
    /// Queue-buffer position selected after checked extent normalization.
    pub parsed_queue_offset: usize,
    /// Optional sound and palette records encountered before the frame.
    pub side_data: PresentationEntrySideData,
    /// Accepted frame or rejected stale link.
    pub disposition: PresentationEntryDisposition,
}

/// Invalid presentation entry grammar or unavailable linked resource.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationEntryError {
    /// A fixed-width field extends beyond the current owned entry.
    SourceTruncated {
        /// Position of the failed read.
        position: usize,
        /// Bytes required by the field.
        required: usize,
        /// Bytes remaining in the current source.
        available: usize,
    },
    /// A side record cannot contain its marker and extent fields.
    SideRecordExtentTooSmall {
        /// Marker identifying the invalid record.
        marker: u16,
        /// Authored inclusive extent.
        extent: usize,
    },
    /// A side record extends beyond the ready queue entry.
    SideRecordExceedsEntry {
        /// Marker identifying the invalid record.
        marker: u16,
        /// Start of the record in the queue buffer.
        start: usize,
        /// Authored inclusive extent.
        extent: usize,
        /// Exclusive end of the owned queue entry.
        entry_end: usize,
    },
    /// The flat resource store could not resolve a linked entry.
    LinkedEntryUnavailable {
        /// Requested retained queue entry.
        link: PresentationLinkId,
    },
    /// Immediate payload expansion rejected the compressed stream.
    Decode(PresentationDispatchError),
}

impl fmt::Display for PresentationEntryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation queue entry: {self:?}")
    }
}

impl Error for PresentationEntryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Decode(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PresentationDispatchError> for PresentationEntryError {
    fn from(error: PresentationDispatchError) -> Self {
        Self::Decode(error)
    }
}

fn truncated(
    source: &[u8],
    position: usize,
    end: usize,
    required: usize,
) -> PresentationEntryError {
    PresentationEntryError::SourceTruncated {
        position,
        required,
        available: end.min(source.len()).saturating_sub(position),
    }
}

fn read_word(source: &[u8], cursor: &mut usize, end: usize) -> Result<u16, PresentationEntryError> {
    let field_end = cursor
        .checked_add(WORD_BYTE_COUNT)
        .ok_or_else(|| truncated(source, *cursor, end, WORD_BYTE_COUNT))?;
    let bytes = source
        .get(*cursor..field_end)
        .filter(|_| field_end <= end)
        .ok_or_else(|| truncated(source, *cursor, end, WORD_BYTE_COUNT))?;
    *cursor = field_end;
    Ok(u16::from_le_bytes(
        bytes.try_into().expect("validated two-byte entry field"),
    ))
}

fn read_link_id(
    source: &[u8],
    cursor: &mut usize,
    end: usize,
) -> Result<PresentationLinkId, PresentationEntryError> {
    let field_end = cursor
        .checked_add(LINK_ID_BYTE_COUNT)
        .ok_or_else(|| truncated(source, *cursor, end, LINK_ID_BYTE_COUNT))?;
    let bytes = source
        .get(*cursor..field_end)
        .filter(|_| field_end <= end)
        .ok_or_else(|| truncated(source, *cursor, end, LINK_ID_BYTE_COUNT))?;
    *cursor = field_end;
    Ok(PresentationLinkId::new(u32::from_le_bytes(
        bytes
            .try_into()
            .expect("validated four-byte link identifier"),
    )))
}

fn side_record_end(
    marker: u16,
    start: usize,
    extent: usize,
    entry_end: usize,
) -> Result<usize, PresentationEntryError> {
    if extent < MINIMUM_SIDE_RECORD_EXTENT {
        return Err(PresentationEntryError::SideRecordExtentTooSmall { marker, extent });
    }
    let end = start
        .checked_add(extent)
        .ok_or(PresentationEntryError::SideRecordExceedsEntry {
            marker,
            start,
            extent,
            entry_end,
        })?;
    if end > entry_end {
        return Err(PresentationEntryError::SideRecordExceedsEntry {
            marker,
            start,
            extent,
            entry_end,
        });
    }
    Ok(end)
}

#[derive(Clone, Copy)]
struct PresentationFrameSource<'a> {
    bytes: &'a [u8],
    end: usize,
    row_mode_offset: usize,
}

fn activate_frame(
    source: PresentationFrameSource<'_>,
    layout: u16,
    requested_storage: PresentationEntryStorage,
    policy: PresentationEntryPolicy,
    decode: &mut impl FnMut(&[u8]) -> Result<PresentationPayload, PresentationDispatchError>,
) -> Result<ActivatedPresentationEntry, PresentationEntryError> {
    let storage = if layout & NO_COORDINATES_LAYOUT_FLAG == 0 {
        requested_storage
    } else {
        PresentationEntryStorage::Default
    };
    let mut cursor = source.row_mode_offset;
    let row_mode = read_word(source.bytes, &mut cursor, source.end)?;
    let rows = row_mode.to_le_bytes()[0];
    let mode = row_mode.to_le_bytes()[1];

    let frame = if rows == 0 {
        PresentationEntryFrame::Empty
    } else if layout & COMPRESSED_LAYOUT_FLAG == 0 {
        let bytes = source
            .bytes
            .get(cursor..source.end)
            .ok_or_else(|| truncated(source.bytes, cursor, source.end, 0))?;
        PresentationEntryFrame::Encoded(bytes.into())
    } else if !policy.skip_back_buffer_present
        && !policy.draw_via_back_buffer
        && mode == TRANSPARENT_ROW_MODE
    {
        let payload = source
            .bytes
            .get(cursor..source.end)
            .ok_or_else(|| truncated(source.bytes, cursor, source.end, 0))?;
        PresentationEntryFrame::DeferredTransparent(payload.into())
    } else {
        let payload = source
            .bytes
            .get(cursor..source.end)
            .ok_or_else(|| truncated(source.bytes, cursor, source.end, 0))?;
        PresentationEntryFrame::Decoded(decode(payload)?)
    };

    Ok(ActivatedPresentationEntry {
        layout,
        row_mode,
        storage,
        frame,
    })
}

fn activate_presentation_entry_with_decoder(
    queue_buffer: &[u8],
    request: PresentationEntryActivationRequest,
    policy: PresentationEntryPolicy,
    mut resolve_link: impl FnMut(PresentationLinkId) -> Option<Box<[u8]>>,
    mut decode: impl FnMut(&[u8]) -> Result<PresentationPayload, PresentationDispatchError>,
) -> Result<PresentationEntryActivation, PresentationEntryError> {
    let parsed_queue_offset = request
        .payload_offset
        .checked_add(request.entry_extent)
        .filter(|end| *end <= queue_buffer.len())
        .map_or(0, |_| request.payload_offset);
    let entry_end = parsed_queue_offset
        .checked_add(request.entry_extent)
        .filter(|end| *end <= queue_buffer.len())
        .unwrap_or(queue_buffer.len());
    let mut cursor = parsed_queue_offset;
    let mut marker_start = cursor;
    let mut layout = read_word(queue_buffer, &mut cursor, entry_end)?;
    let mut side_data = PresentationEntrySideData::default();

    if layout == SOUND_RECORD_MARKER {
        let extent_position = cursor;
        let extent = usize::from(read_word(queue_buffer, &mut cursor, entry_end)?);
        let record_end = side_record_end(layout, marker_start, extent, entry_end)?;
        if policy.sound_enabled {
            side_data.sound_record = Some(queue_buffer[extent_position..record_end].into());
        }
        cursor = record_end;
        marker_start = cursor;
        layout = read_word(queue_buffer, &mut cursor, entry_end)?;
    }

    while layout == PALETTE_RECORD_MARKER {
        let extent = usize::from(read_word(queue_buffer, &mut cursor, entry_end)?);
        let payload_start = cursor;
        let record_end = side_record_end(layout, marker_start, extent, entry_end)?;
        side_data.palette_payload = Some(queue_buffer[payload_start..record_end].into());
        cursor = record_end;
        marker_start = cursor;
        layout = read_word(queue_buffer, &mut cursor, entry_end)?;
    }

    let disposition = if layout == LINK_RECORD_MARKER {
        let link_record_end = cursor
            .checked_add(LINK_RECORD_BYTE_COUNT)
            .ok_or_else(|| truncated(queue_buffer, cursor, entry_end, LINK_RECORD_BYTE_COUNT))?;
        if link_record_end > entry_end {
            return Err(truncated(
                queue_buffer,
                cursor,
                entry_end,
                LINK_RECORD_BYTE_COUNT,
            ));
        }
        let link = read_link_id(queue_buffer, &mut cursor, entry_end)?;
        let expected_key = read_word(queue_buffer, &mut cursor, entry_end)?;
        let linked =
            resolve_link(link).ok_or(PresentationEntryError::LinkedEntryUnavailable { link })?;
        let mut linked_cursor = 0;
        let actual_key = read_word(&linked, &mut linked_cursor, linked.len())?;
        let linked_layout = read_word(&linked, &mut linked_cursor, linked.len())?;
        if actual_key != expected_key {
            PresentationEntryDisposition::RejectedLink {
                link,
                expected_key,
                actual_key,
            }
        } else {
            PresentationEntryDisposition::Active(activate_frame(
                PresentationFrameSource {
                    bytes: &linked,
                    end: linked.len(),
                    row_mode_offset: linked_cursor,
                },
                linked_layout,
                request.storage,
                policy,
                &mut decode,
            )?)
        }
    } else {
        PresentationEntryDisposition::Active(activate_frame(
            PresentationFrameSource {
                bytes: queue_buffer,
                end: entry_end,
                row_mode_offset: cursor,
            },
            layout,
            request.storage,
            policy,
            &mut decode,
        )?)
    };

    Ok(PresentationEntryActivation {
        parsed_queue_offset,
        side_data,
        disposition,
    })
}

/// Parse and activate one complete flat presentation queue entry.
///
/// This translates `list_d8c_activate_entry` at BLOODPRG offset `0x00A552`.
/// The authored `sd`, `pl`, and `mm` grammar, storage selection, stale-link
/// rejection, and immediate/deferred decode gates are retained. Owned slices
/// replace active far pointers, and logical forward byte order replaces the
/// original routine's accidental dependence on the x86 direction flag.
pub fn activate_presentation_entry(
    queue_buffer: &[u8],
    request: PresentationEntryActivationRequest,
    policy: PresentationEntryPolicy,
    resolve_link: impl FnMut(PresentationLinkId) -> Option<Box<[u8]>>,
) -> Result<PresentationEntryActivation, PresentationEntryError> {
    activate_presentation_entry_with_decoder(
        queue_buffer,
        request,
        policy,
        resolve_link,
        decode_presentation_payload,
    )
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ACTIVATE_VECTOR_COUNT: usize = 14;
    const ORACLE_BUFFER_CAPACITY: usize = u16::MAX as usize + 1;
    const DEFAULT_ENTRY_EXTENT: usize = 64;
    const SOUND_RECORD_EXTENT: u16 = 8;
    const SHORT_SOUND_RECORD_EXTENT: u16 = 6;
    const PALETTE_RECORD_EXTENT: u16 = 8;
    const SHORT_PALETTE_RECORD_EXTENT: u16 = 6;
    const SECOND_PALETTE_OFFSET: usize = 12;
    const FRAME_AFTER_SOUND_PALETTE: usize = 16;
    const FRAME_AFTER_SOUND_TWO_PALETTES: usize = 20;
    const TEST_LINK_ID: PresentationLinkId = PresentationLinkId::new(0x4400_2400);
    const TEST_LINK_KEY: u16 = 0x7100;
    const AB_EMPTY_STREAM: &[u8] = &[
        0x31, 0x42, 0x53, 0x64, 0x75, 0x0C, 0x02, 0x00, 0x00, 0x00, 0x00,
    ];

    #[derive(Deserialize)]
    struct ActivateOracle {
        name: String,
        kind: String,
        entry_extent: usize,
        input_source: [u16; 2],
        parse_source: [u16; 2],
        layout: u16,
        row_mode: u16,
        direction: String,
        sound_offset: Option<u16>,
        palette_offsets: Vec<u16>,
        result_kind: String,
    }

    fn write_word(buffer: &mut [u8], offset: usize, value: u16) {
        buffer[offset..offset + WORD_BYTE_COUNT].copy_from_slice(&value.to_le_bytes());
    }

    fn write_link_id(buffer: &mut [u8], offset: usize, link: PresentationLinkId) {
        buffer[offset..offset + LINK_ID_BYTE_COUNT].copy_from_slice(&link.get().to_le_bytes());
    }

    fn build_vector_entry(vector: &ActivateOracle, buffer: &mut [u8], parse_offset: usize) {
        match vector.kind.as_str() {
            "direct" => {
                write_word(buffer, parse_offset, vector.layout);
                write_word(buffer, parse_offset + WORD_BYTE_COUNT, vector.row_mode);
            }
            "sound_palette" => {
                write_word(buffer, parse_offset, SOUND_RECORD_MARKER);
                write_word(buffer, parse_offset + WORD_BYTE_COUNT, SOUND_RECORD_EXTENT);
                write_word(
                    buffer,
                    parse_offset + usize::from(SOUND_RECORD_EXTENT),
                    PALETTE_RECORD_MARKER,
                );
                write_word(
                    buffer,
                    parse_offset + usize::from(SOUND_RECORD_EXTENT) + WORD_BYTE_COUNT,
                    PALETTE_RECORD_EXTENT,
                );
                write_word(
                    buffer,
                    parse_offset + FRAME_AFTER_SOUND_PALETTE,
                    vector.layout,
                );
                write_word(
                    buffer,
                    parse_offset + FRAME_AFTER_SOUND_PALETTE + WORD_BYTE_COUNT,
                    vector.row_mode,
                );
            }
            "sound_two_palettes" => {
                write_word(buffer, parse_offset, SOUND_RECORD_MARKER);
                write_word(
                    buffer,
                    parse_offset + WORD_BYTE_COUNT,
                    SHORT_SOUND_RECORD_EXTENT,
                );
                let first_palette = parse_offset + usize::from(SHORT_SOUND_RECORD_EXTENT);
                write_word(buffer, first_palette, PALETTE_RECORD_MARKER);
                write_word(
                    buffer,
                    first_palette + WORD_BYTE_COUNT,
                    SHORT_PALETTE_RECORD_EXTENT,
                );
                let second_palette = parse_offset + SECOND_PALETTE_OFFSET;
                write_word(buffer, second_palette, PALETTE_RECORD_MARKER);
                write_word(
                    buffer,
                    second_palette + WORD_BYTE_COUNT,
                    PALETTE_RECORD_EXTENT,
                );
                write_word(
                    buffer,
                    parse_offset + FRAME_AFTER_SOUND_TWO_PALETTES,
                    vector.layout,
                );
                write_word(
                    buffer,
                    parse_offset + FRAME_AFTER_SOUND_TWO_PALETTES + WORD_BYTE_COUNT,
                    vector.row_mode,
                );
            }
            "link" => {
                write_word(buffer, parse_offset, LINK_RECORD_MARKER);
                write_link_id(buffer, parse_offset + WORD_BYTE_COUNT, TEST_LINK_ID);
                write_word(
                    buffer,
                    parse_offset + WORD_BYTE_COUNT + LINK_ID_BYTE_COUNT,
                    TEST_LINK_KEY,
                );
            }
            kind => panic!("unknown activation oracle kind {kind}"),
        }
    }

    #[test]
    fn activation_semantics_match_every_original_vector() {
        let vectors: Vec<ActivateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a552_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ACTIVATE_VECTOR_COUNT);

        for (index, vector) in vectors.into_iter().enumerate() {
            let input_offset = usize::from(vector.input_source[1]);
            let expected_parse_offset = usize::from(vector.parse_source[1]);
            let capacity = if expected_parse_offset == 0
                && input_offset
                    .checked_add(vector.entry_extent)
                    .is_some_and(|end| end <= ORACLE_BUFFER_CAPACITY)
            {
                input_offset + vector.entry_extent - 1
            } else {
                ORACLE_BUFFER_CAPACITY
            };
            let mut buffer = vec![0xCC; capacity];
            build_vector_entry(&vector, &mut buffer, expected_parse_offset);
            let link_matches = vector.result_kind != "consumed_mismatched_link";
            let mut decode_calls = 0;
            let activation = activate_presentation_entry_with_decoder(
                &buffer,
                PresentationEntryActivationRequest {
                    entry_extent: vector.entry_extent,
                    payload_offset: input_offset,
                    storage: PresentationEntryStorage::Alternate,
                },
                PresentationEntryPolicy {
                    sound_enabled: vector.sound_offset.is_some(),
                    skip_back_buffer_present: vector.name == "compressed_dispatch_gate",
                    draw_via_back_buffer: false,
                },
                |link| {
                    assert_eq!(link, TEST_LINK_ID, "{}", vector.name);
                    let mut linked = vec![0xCC; DEFAULT_ENTRY_EXTENT];
                    write_word(
                        &mut linked,
                        0,
                        if link_matches {
                            TEST_LINK_KEY
                        } else {
                            TEST_LINK_KEY ^ u16::from(u8::MAX)
                        },
                    );
                    write_word(&mut linked, WORD_BYTE_COUNT, vector.layout);
                    write_word(&mut linked, WORD_BYTE_COUNT * 2, vector.row_mode);
                    Some(linked.into_boxed_slice())
                },
                |_| {
                    decode_calls += 1;
                    Ok(PresentationPayload::Unrecognized { checksum: u8::MIN })
                },
            )
            .unwrap();

            assert_eq!(
                activation.parsed_queue_offset, expected_parse_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                activation.side_data.sound_record.is_some(),
                vector.sound_offset.is_some(),
                "{}",
                vector.name
            );
            assert_eq!(
                activation.side_data.palette_payload.is_some(),
                !vector.palette_offsets.is_empty(),
                "{}",
                vector.name
            );
            assert!(
                matches!(vector.direction.as_str(), "forward" | "backward"),
                "{}",
                vector.name
            );

            match (vector.result_kind.as_str(), activation.disposition) {
                (
                    "consumed_mismatched_link",
                    PresentationEntryDisposition::RejectedLink {
                        link,
                        expected_key,
                        actual_key,
                    },
                ) => {
                    assert_eq!(link, TEST_LINK_ID, "{}", vector.name);
                    assert_eq!(expected_key, TEST_LINK_KEY, "{}", vector.name);
                    assert_ne!(actual_key, expected_key, "{}", vector.name);
                }
                (expected_kind, PresentationEntryDisposition::Active(active)) => {
                    assert_eq!(active.layout, vector.layout, "{}", vector.name);
                    assert_eq!(active.row_mode, vector.row_mode, "{}", vector.name);
                    assert_eq!(
                        active.storage,
                        if vector.layout & NO_COORDINATES_LAYOUT_FLAG == 0 {
                            PresentationEntryStorage::Alternate
                        } else {
                            PresentationEntryStorage::Default
                        },
                        "{}",
                        vector.name
                    );
                    let actual_kind = match active.frame {
                        PresentationEntryFrame::Empty => "stored_empty",
                        PresentationEntryFrame::Encoded(_) => "published_source",
                        PresentationEntryFrame::Decoded(_) => "decoded_storage",
                        PresentationEntryFrame::DeferredTransparent(_) => "deferred_rect",
                    };
                    assert_eq!(actual_kind, expected_kind, "{}", vector.name);
                }
                (expected, actual) => {
                    panic!("{}: expected {expected}, got {actual:?}", vector.name)
                }
            }
            assert_eq!(
                decode_calls,
                usize::from(vector.result_kind == "decoded_storage"),
                "vector {index}: {}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_side_records_are_rejected_without_partial_activation() {
        let mut buffer = vec![0; DEFAULT_ENTRY_EXTENT];
        write_word(&mut buffer, 0, PALETTE_RECORD_MARKER);
        write_word(&mut buffer, WORD_BYTE_COUNT, 2);
        let result = activate_presentation_entry(
            &buffer,
            PresentationEntryActivationRequest {
                entry_extent: buffer.len(),
                payload_offset: 0,
                storage: PresentationEntryStorage::Default,
            },
            PresentationEntryPolicy::default(),
            |_| None,
        );
        assert_eq!(
            result,
            Err(PresentationEntryError::SideRecordExtentTooSmall {
                marker: PALETTE_RECORD_MARKER,
                extent: 2,
            })
        );
    }

    #[test]
    fn public_activation_path_runs_the_selected_payload_decoder() {
        let layout = COMPRESSED_LAYOUT_FLAG;
        let row_mode = u16::from_le_bytes([1, 1]);
        let mut buffer = vec![0; FRAME_HEADER_BYTE_COUNT + AB_EMPTY_STREAM.len()];
        write_word(&mut buffer, 0, layout);
        write_word(&mut buffer, WORD_BYTE_COUNT, row_mode);
        buffer[FRAME_HEADER_BYTE_COUNT..].copy_from_slice(AB_EMPTY_STREAM);

        let activation = activate_presentation_entry(
            &buffer,
            PresentationEntryActivationRequest {
                entry_extent: buffer.len(),
                payload_offset: 0,
                storage: PresentationEntryStorage::Default,
            },
            PresentationEntryPolicy::default(),
            |_| None,
        )
        .unwrap();
        assert!(matches!(
            activation.disposition,
            PresentationEntryDisposition::Active(ActivatedPresentationEntry {
                frame: PresentationEntryFrame::Decoded(PresentationPayload::Ab(ref outcome)),
                ..
            }) if outcome.bytes.is_empty()
        ));
    }

    #[test]
    fn unavailable_flat_links_are_reported_explicitly() {
        let mut buffer = vec![0; DEFAULT_ENTRY_EXTENT];
        write_word(&mut buffer, 0, LINK_RECORD_MARKER);
        write_link_id(&mut buffer, WORD_BYTE_COUNT, TEST_LINK_ID);
        write_word(
            &mut buffer,
            WORD_BYTE_COUNT + LINK_ID_BYTE_COUNT,
            TEST_LINK_KEY,
        );
        let result = activate_presentation_entry(
            &buffer,
            PresentationEntryActivationRequest {
                entry_extent: buffer.len(),
                payload_offset: 0,
                storage: PresentationEntryStorage::Default,
            },
            PresentationEntryPolicy::default(),
            |_| None,
        );
        assert_eq!(
            result,
            Err(PresentationEntryError::LinkedEntryUnavailable { link: TEST_LINK_ID })
        );
    }
}
