//! Flat queue refill and source-range rollover for presentation resources.

use std::error::Error;
use std::fmt;

use super::{
    PresentationLinkId, PresentationQueueError, PresentationQueueState,
    PresentationResourceDescriptor, PresentationResourceId, PresentationResourceStreamState,
    PresentationSourceError, append_presentation_source_bytes, presentation_resource_descriptor,
    read_presentation_entry_extent,
};

const SOURCE_ROLLOVER_ENABLED_FLAG: u8 = 1;
const SYNTHESIZE_LINKS_FLAG: u8 = 4;
const CACHED_RANGE_VALID_FLAG: u8 = 8;
const UNCAPPED_SOURCE_FLAG: u8 = 128;
const SOURCE_WINDOW_BYTE_COUNT: usize = 2_048;
const SOURCE_WINDOW_MASK: usize = SOURCE_WINDOW_BYTE_COUNT - 1;
const SOURCE_WINDOW_LOOKAHEAD: usize = SOURCE_WINDOW_BYTE_COUNT;
const ROLLOVER_RESERVATION_BYTE_COUNT: usize = 4_096;
const SYNTHETIC_LINK_COUNT: usize = 4;
const SYNTHETIC_LINK_EXTENT: usize = 10;
const SYNTHETIC_LINK_BODY_BYTE_COUNT: usize = SYNTHETIC_LINK_EXTENT - size_of::<u16>();
const SYNTHETIC_LINK_MARKER: u16 = u16::from_le_bytes(*b"mm");

/// Position of the next retained queue entry referenced by rollover links.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationQueueLinkCursor {
    position: usize,
}

impl PresentationQueueLinkCursor {
    /// Start linking at one validated flat queue position.
    pub const fn new(position: usize) -> Self {
        Self { position }
    }

    /// Return the next retained entry position.
    pub const fn position(self) -> usize {
        self.position
    }
}

/// Observable terminal action from one bounded refill call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationQueueRefillOutcome {
    /// One source chunk was appended to the queue.
    Transferred {
        /// Exact number of bytes appended.
        byte_count: usize,
    },
    /// The queue must be consumed before this request can fit.
    WaitingForRoom {
        /// Byte count rejected by the current queue geometry.
        byte_count: usize,
    },
    /// The current source range ended without rollover enabled.
    Finished {
        /// Whether no queue entries remain.
        queue_empty: bool,
        /// Whether a source owned only by this stream was released.
        source_released: bool,
    },
}

/// Invalid descriptor, range, source, queue, or retained-link state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationQueueRefillError {
    /// Queue geometry rejected a rollover or synthetic entry.
    Queue(PresentationQueueError),
    /// The active owned source could not satisfy an exact read.
    Source(PresentationSourceError),
    /// Absolute source-position arithmetic overflowed the host domain.
    SourceOffsetOverflow {
        /// Absolute origin of source position zero.
        absolute_origin: usize,
        /// Current flat source position.
        position: usize,
    },
    /// The active resource ID has no authored descriptor.
    DescriptorUnavailable {
        /// Requested active presentation resource.
        resource: PresentationResourceId,
    },
    /// Rollover selected a descriptor without a valid cached range.
    CachedRangeUnavailable {
        /// Descriptor requiring a cached range.
        resource: PresentationResourceId,
    },
    /// The retained primary source range is absent or empty.
    RolloverRangeUnavailable,
    /// A link target does not contain a complete extent word and entry.
    LinkTargetOutOfRange {
        /// First byte of the retained target entry.
        position: usize,
        /// Complete target extent when its header was readable.
        extent: Option<usize>,
        /// Available queue-buffer bytes.
        buffer_len: usize,
    },
    /// A retained target's extent is shorter than its own header.
    LinkTargetExtentTooSmall {
        /// First byte of the retained target entry.
        position: usize,
        /// Invalid serialized extent.
        extent: usize,
    },
    /// A queue position cannot fit the serialized four-byte link identifier.
    LinkIdentifierOutOfRange {
        /// Unrepresentable flat queue position.
        position: usize,
    },
    /// Advancing the retained-entry cursor overflowed the host domain.
    LinkCursorOverflow {
        /// Current retained-entry position.
        position: usize,
        /// Extent used to advance the position.
        extent: usize,
    },
}

impl fmt::Display for PresentationQueueRefillError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation queue refill: {self:?}")
    }
}

impl Error for PresentationQueueRefillError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Queue(source) => Some(source),
            Self::Source(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PresentationQueueError> for PresentationQueueRefillError {
    fn from(error: PresentationQueueError) -> Self {
        Self::Queue(error)
    }
}

impl From<PresentationSourceError> for PresentationQueueRefillError {
    fn from(error: PresentationSourceError) -> Self {
        Self::Source(error)
    }
}

fn require_source_bytes(
    stream: &PresentationResourceStreamState,
    byte_count: usize,
) -> Result<(), PresentationQueueRefillError> {
    let source = stream
        .source
        .as_ref()
        .ok_or(PresentationSourceError::SourceUnavailable)?;
    let remaining = stream
        .source_remaining()
        .ok_or(PresentationSourceError::SourceUnavailable)?;
    if byte_count > remaining {
        return Err(PresentationSourceError::SourceTruncated {
            position: source.position(),
            requested: byte_count,
            remaining,
        }
        .into());
    }
    Ok(())
}

fn source_window_limit(
    stream: &PresentationResourceStreamState,
) -> Result<usize, PresentationQueueRefillError> {
    let source = stream
        .source
        .as_ref()
        .ok_or(PresentationSourceError::SourceUnavailable)?;
    let absolute_offset = stream
        .absolute_origin
        .checked_add(source.position())
        .ok_or(PresentationQueueRefillError::SourceOffsetOverflow {
            absolute_origin: stream.absolute_origin,
            position: source.position(),
        })?;
    let bytes_to_boundary =
        (SOURCE_WINDOW_BYTE_COUNT - (absolute_offset & SOURCE_WINDOW_MASK)) & SOURCE_WINDOW_MASK;
    Ok(bytes_to_boundary + SOURCE_WINDOW_LOOKAHEAD)
}

fn transfer_pending_bytes(
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
    stream: &mut PresentationResourceStreamState,
) -> Result<PresentationQueueRefillOutcome, PresentationQueueRefillError> {
    let mut byte_count = queue.pending_entry_bytes;
    let low_flags = stream.flags.to_le_bytes()[0];
    if low_flags & UNCAPPED_SOURCE_FLAG == 0 {
        byte_count = byte_count.min(source_window_limit(stream)?);
    }
    if !queue.has_room(byte_count) {
        return Ok(PresentationQueueRefillOutcome::WaitingForRoom { byte_count });
    }

    require_source_bytes(stream, byte_count)?;
    append_presentation_source_bytes(stream.source.as_mut(), queue, queue_buffer, byte_count)?;
    queue.pending_entry_bytes -= byte_count;
    Ok(PresentationQueueRefillOutcome::Transferred { byte_count })
}

fn read_next_extent(
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
    stream: &mut PresentationResourceStreamState,
) -> Result<(), PresentationQueueRefillError> {
    require_source_bytes(stream, size_of::<u16>())?;
    let entry = read_presentation_entry_extent(stream.source.as_mut(), queue, queue_buffer)?;
    queue.begin_entry(entry.extent, entry.payload_cursor)?;
    Ok(())
}

fn retained_entry_extent(
    queue_buffer: &[u8],
    position: usize,
) -> Result<usize, PresentationQueueRefillError> {
    let header_end = position.checked_add(size_of::<u16>()).ok_or(
        PresentationQueueRefillError::LinkTargetOutOfRange {
            position,
            extent: None,
            buffer_len: queue_buffer.len(),
        },
    )?;
    let header = queue_buffer.get(position..header_end).ok_or(
        PresentationQueueRefillError::LinkTargetOutOfRange {
            position,
            extent: None,
            buffer_len: queue_buffer.len(),
        },
    )?;
    let extent = usize::from(u16::from_le_bytes(
        header
            .try_into()
            .expect("validated retained-entry extent word"),
    ));
    if extent < size_of::<u16>() {
        return Err(PresentationQueueRefillError::LinkTargetExtentTooSmall { position, extent });
    }
    position
        .checked_add(extent)
        .filter(|end| *end <= queue_buffer.len())
        .ok_or(PresentationQueueRefillError::LinkTargetOutOfRange {
            position,
            extent: Some(extent),
            buffer_len: queue_buffer.len(),
        })?;
    Ok(extent)
}

fn write_word(buffer: &mut [u8], position: usize, value: u16) {
    buffer[position..position + size_of::<u16>()].copy_from_slice(&value.to_le_bytes());
}

fn synthesize_rollover_links(
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
    link_cursor: &mut PresentationQueueLinkCursor,
) -> Result<(), PresentationQueueRefillError> {
    let mut staged_queue = queue.clone();
    let mut staged_buffer = queue_buffer.to_vec();
    let mut staged_cursor = *link_cursor;

    for _ in 0..SYNTHETIC_LINK_COUNT {
        let target_position = staged_cursor.position;
        let target_extent = retained_entry_extent(&staged_buffer, target_position)?;
        let link = PresentationLinkId::from_queue_offset(target_position).ok_or(
            PresentationQueueRefillError::LinkIdentifierOutOfRange {
                position: target_position,
            },
        )?;

        let header_position = staged_queue.head;
        staged_queue.enqueue(size_of::<u16>())?;
        write_word(
            &mut staged_buffer,
            header_position,
            SYNTHETIC_LINK_EXTENT as u16,
        );
        staged_queue.begin_entry(SYNTHETIC_LINK_EXTENT, staged_queue.head)?;

        let body_position = staged_queue.head;
        let body_end = body_position
            .checked_add(SYNTHETIC_LINK_BODY_BYTE_COUNT)
            .ok_or(PresentationQueueError::HeadOutsideBuffer {
                head: body_position,
                byte_count: SYNTHETIC_LINK_BODY_BYTE_COUNT,
                capacity: staged_queue.buffer_capacity,
            })?;
        if body_end > staged_buffer.len() {
            return Err(PresentationSourceError::QueueBufferTooShort {
                queue_capacity: body_end,
                buffer_len: staged_buffer.len(),
            }
            .into());
        }
        write_word(&mut staged_buffer, body_position, SYNTHETIC_LINK_MARKER);
        staged_buffer[body_position + size_of::<u16>()..body_position + 6]
            .copy_from_slice(&link.get().to_le_bytes());
        write_word(&mut staged_buffer, body_position + 6, target_extent as u16);
        staged_queue.enqueue(SYNTHETIC_LINK_BODY_BYTE_COUNT)?;

        staged_cursor.position = target_position.checked_add(target_extent).ok_or(
            PresentationQueueRefillError::LinkCursorOverflow {
                position: target_position,
                extent: target_extent,
            },
        )?;
    }

    *queue = staged_queue;
    queue_buffer.copy_from_slice(&staged_buffer);
    *link_cursor = staged_cursor;
    Ok(())
}

fn select_rollover_range(
    queue: &mut PresentationQueueState,
    stream: &mut PresentationResourceStreamState,
    descriptors: &[PresentationResourceDescriptor],
) -> Result<(), PresentationQueueRefillError> {
    let previous_wrap_count = queue.wrap_count;
    queue.reset_wrap_bounds();
    queue.read_wrap_limit = Some(previous_wrap_count);
    queue.rollover_latched = false;

    if stream.active != stream.requested {
        let active = stream
            .active
            .ok_or(PresentationQueueRefillError::RolloverRangeUnavailable)?;
        let descriptor = presentation_resource_descriptor(descriptors, active.get())
            .ok_or(PresentationQueueRefillError::DescriptorUnavailable { resource: active })?;
        stream.requested = Some(active);
        let cached_range = descriptor
            .cached_range
            .filter(|_| descriptor.flags & CACHED_RANGE_VALID_FLAG != 0);
        let cached_range = cached_range
            .ok_or(PresentationQueueRefillError::CachedRangeUnavailable { resource: active })?;
        stream.range = Some(cached_range);
        stream.flags = u16::from_le_bytes([descriptor.flags, stream.flags.to_le_bytes()[1]]);
    }

    let range = stream
        .range
        .filter(|range| range.remaining != 0)
        .ok_or(PresentationQueueRefillError::RolloverRangeUnavailable)?;
    stream.select_range(range)?;
    Ok(())
}

/// Refill one bounded chunk of the active streamed presentation queue.
///
/// This translates `list_d8c_refill` at BLOODPRG offset `0x00A2AB`. Pending
/// body reads, the original 2,048-byte transport window, queue backpressure,
/// source completion, descriptor-cache rollover, and four synthetic `mm` links
/// remain explicit game logic. Owned byte ranges, checked queue positions, and
/// `PresentationLinkId` replace EMS pages and far pointers.
pub fn refill_presentation_queue(
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
    stream: &mut PresentationResourceStreamState,
    descriptors: &[PresentationResourceDescriptor],
    link_cursor: &mut PresentationQueueLinkCursor,
) -> Result<PresentationQueueRefillOutcome, PresentationQueueRefillError> {
    if queue.buffer_capacity > queue_buffer.len() {
        return Err(PresentationSourceError::QueueBufferTooShort {
            queue_capacity: queue.buffer_capacity,
            buffer_len: queue_buffer.len(),
        }
        .into());
    }

    let mut check_source_first = false;
    loop {
        if !check_source_first && queue.pending_entry_bytes != 0 {
            return transfer_pending_bytes(queue, queue_buffer, stream);
        }
        check_source_first = false;

        if queue.secondary_wrap_limit != Some(queue.wrap_count)
            && stream
                .source_remaining()
                .is_some_and(|remaining| remaining != 0)
        {
            read_next_extent(queue, queue_buffer, stream)?;
            continue;
        }

        let low_flags = stream.flags.to_le_bytes()[0];
        if low_flags & SOURCE_ROLLOVER_ENABLED_FLAG == 0 {
            let queue_empty = queue.finish_source();
            let source_released = queue_empty && stream.close_owned_source(queue);
            return Ok(PresentationQueueRefillOutcome::Finished {
                queue_empty,
                source_released,
            });
        }
        if !queue.has_room(ROLLOVER_RESERVATION_BYTE_COUNT) {
            return Ok(PresentationQueueRefillOutcome::WaitingForRoom {
                byte_count: ROLLOVER_RESERVATION_BYTE_COUNT,
            });
        }

        select_rollover_range(queue, stream, descriptors)?;
        if stream.flags.to_le_bytes()[0] & SYNTHESIZE_LINKS_FLAG != 0 {
            synthesize_rollover_links(queue, queue_buffer, link_cursor)?;
            queue.rollover_latched = true;
        }
        check_source_first = true;
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{
        PresentationSourceLease, PresentationSourceRange, resolve_presentation_queue_link,
    };
    use commander_blood_formats::archive::BloodResourceName;

    const REFILL_VECTOR_COUNT: usize = 13;
    const QUEUE_BUFFER_BYTE_COUNT: usize = 65_536;
    const INITIAL_HEAD: usize = 256;
    const INITIAL_TAIL: usize = 64;
    const INITIAL_QUEUED_BYTE_COUNT: usize = 32;
    const INITIAL_LINK_POSITION: usize = 1_280;
    const RETAINED_LINK_EXTENTS: [usize; SYNTHETIC_LINK_COUNT] = [6, 8, 10, 12];

    #[derive(Deserialize)]
    struct RefillOracle {
        name: String,
        initial_pending: usize,
        initial_flags: u16,
        initial_source_offset: usize,
        link_target_offset: usize,
        result_link_target_offset: usize,
    }

    fn descriptor(
        flags: u8,
        cached_range: Option<PresentationSourceRange>,
    ) -> PresentationResourceDescriptor {
        PresentationResourceDescriptor {
            flags,
            filename: BloodResourceName::new(b"RESOURCE.DAT").unwrap(),
            cached_range,
        }
    }

    fn build_queue(pending: usize) -> PresentationQueueState {
        PresentationQueueState {
            head: INITIAL_HEAD,
            tail: INITIAL_TAIL,
            queued_bytes: INITIAL_QUEUED_BYTE_COUNT,
            pending_entry_bytes: pending,
            buffer_capacity: QUEUE_BUFFER_BYTE_COUNT,
            wrap_limit: QUEUE_BUFFER_BYTE_COUNT,
            wrap_count: 1,
            read_wrap_limit: Some(43_690),
            secondary_wrap_limit: Some(3),
            status_bits: 32,
            ..PresentationQueueState::default()
        }
    }

    fn build_stream(
        bytes: Vec<u8>,
        absolute_origin: usize,
        flags: u16,
    ) -> PresentationResourceStreamState {
        PresentationResourceStreamState {
            requested: Some(PresentationResourceId::new(5)),
            active: Some(PresentationResourceId::new(5)),
            flags,
            ready: true,
            absolute_origin,
            lease: PresentationSourceLease::Owned,
            source: Some(super::super::PresentationByteSource::new(bytes)),
            ..PresentationResourceStreamState::default()
        }
    }

    fn write_extent(buffer: &mut [u8], position: usize, extent: usize) {
        buffer[position..position + size_of::<u16>()]
            .copy_from_slice(&(extent as u16).to_le_bytes());
    }

    fn oracle_vectors() -> Vec<RefillOracle> {
        let vectors: Vec<RefillOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a2ab_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), REFILL_VECTOR_COUNT);
        vectors
    }

    #[test]
    fn refill_accounts_for_every_original_control_flow_vector() {
        let vectors = oracle_vectors();
        for vector in &vectors[..4] {
            let source_len = vector.initial_pending.max(1);
            let mut stream = build_stream(
                vec![0x5A; source_len],
                vector.initial_source_offset,
                vector.initial_flags,
            );
            let mut queue = build_queue(vector.initial_pending);
            if vector.name == "pending_capacity_failure" {
                queue.wrap_limit = queue.queued_bytes + vector.initial_pending;
            }
            let initial_queue = queue.clone();
            let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
            let mut links = PresentationQueueLinkCursor::new(vector.link_target_offset);
            let result =
                refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links)
                    .unwrap();

            if vector.name == "pending_capacity_failure" {
                assert_eq!(
                    result,
                    PresentationQueueRefillOutcome::WaitingForRoom {
                        byte_count: vector.initial_pending,
                    }
                );
                assert_eq!(queue, initial_queue);
                continue;
            }
            let expected = if vector.name == "pending_capped_at_next_window" {
                3_532
            } else {
                vector.initial_pending
            };
            assert_eq!(
                result,
                PresentationQueueRefillOutcome::Transferred {
                    byte_count: expected,
                },
                "{}",
                vector.name
            );
            assert_eq!(queue.pending_entry_bytes, vector.initial_pending - expected);
            assert_eq!(links.position(), vector.result_link_target_offset);
        }

        assert_eq!(vectors[4].name, "next_extent_then_body");
        let mut stream = build_stream(
            [32u16.to_le_bytes().as_slice(), &[0xA5; 30]].concat(),
            vectors[4].initial_source_offset,
            vectors[4].initial_flags,
        );
        let mut queue = build_queue(0);
        let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
        let mut links = PresentationQueueLinkCursor::new(INITIAL_LINK_POSITION);
        assert_eq!(
            refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links,)
                .unwrap(),
            PresentationQueueRefillOutcome::Transferred { byte_count: 30 }
        );
        assert_eq!(queue.wrap_count, 2);
        assert_eq!(queue.pending_entry_bytes, 0);

        assert_eq!(vectors[5].name, "next_extent_read_failure");
        let mut stream = build_stream(
            vec![0x20],
            vectors[5].initial_source_offset,
            vectors[5].initial_flags,
        );
        let mut queue = build_queue(0);
        let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
        assert!(
            refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links,)
                .is_err()
        );

        for (index, queued_bytes) in [(6, INITIAL_QUEUED_BYTE_COUNT), (7, 0)] {
            let vector = &vectors[index];
            let mut stream = build_stream(
                Vec::new(),
                vector.initial_source_offset,
                vector.initial_flags,
            );
            let mut queue = build_queue(0);
            queue.wrap_count = if index == 6 { 3 } else { 1 };
            queue.queued_bytes = queued_bytes;
            let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
            let result =
                refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links)
                    .unwrap();
            assert_eq!(
                result,
                PresentationQueueRefillOutcome::Finished {
                    queue_empty: queued_bytes == 0,
                    source_released: queued_bytes == 0,
                },
                "{}",
                vector.name
            );
        }

        assert_eq!(vectors[8].name, "rollover_capacity_failure");
        let mut stream = build_stream(Vec::new(), 0, vectors[8].initial_flags);
        stream.range = Some(PresentationSourceRange {
            position: 0,
            remaining: 1,
        });
        let mut queue = build_queue(0);
        queue.wrap_count = 3;
        queue.wrap_limit = queue.queued_bytes + ROLLOVER_RESERVATION_BYTE_COUNT;
        let initial = queue.clone();
        let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
        let result =
            refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links)
                .unwrap();
        assert_eq!(
            result,
            PresentationQueueRefillOutcome::WaitingForRoom {
                byte_count: ROLLOVER_RESERVATION_BYTE_COUNT,
            }
        );
        assert_eq!(queue, initial);

        for index in [9, 10] {
            let vector = &vectors[index];
            let mut stream = build_stream(vec![0x20], 0, vector.initial_flags);
            stream.range = Some(PresentationSourceRange {
                position: 0,
                remaining: 1,
            });
            let mut descriptors = vec![descriptor(0, None); 8];
            if index == 10 {
                stream.requested = Some(PresentationResourceId::new(2));
                stream.active = Some(PresentationResourceId::new(7));
                descriptors[7] = descriptor(
                    CACHED_RANGE_VALID_FLAG | SOURCE_ROLLOVER_ENABLED_FLAG,
                    stream.range,
                );
            }
            let mut queue = build_queue(0);
            queue.wrap_count = 3;
            let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
            let result = refill_presentation_queue(
                &mut queue,
                &mut buffer,
                &mut stream,
                &descriptors,
                &mut links,
            );
            assert!(result.is_err(), "{}", vector.name);
            assert_eq!(queue.wrap_count, 0, "{}", vector.name);
            assert_eq!(queue.read_wrap_limit, Some(3), "{}", vector.name);
            if index == 10 {
                assert_eq!(stream.requested, stream.active);
                assert_eq!(stream.flags.to_le_bytes()[0], 9);
            }
        }

        assert_eq!(vectors[11].name, "rollover_synthesizes_four_links");
        let mut stream = build_stream(vec![0x20], 0, vectors[11].initial_flags);
        stream.range = Some(PresentationSourceRange {
            position: 0,
            remaining: 1,
        });
        let mut queue = build_queue(0);
        queue.wrap_count = 3;
        let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
        let mut target = vectors[11].link_target_offset;
        for extent in RETAINED_LINK_EXTENTS {
            write_extent(&mut buffer, target, extent);
            target += extent;
        }
        let first_link = queue.head;
        let mut links = PresentationQueueLinkCursor::new(vectors[11].link_target_offset);
        assert!(
            refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links,)
                .is_err()
        );
        assert_eq!(links.position(), vectors[11].result_link_target_offset);
        assert_eq!(queue.wrap_count, SYNTHETIC_LINK_COUNT as u16);
        assert_eq!(queue.pending_entry_bytes, SYNTHETIC_LINK_BODY_BYTE_COUNT);
        assert!(queue.rollover_latched);
        for (index, expected_extent) in RETAINED_LINK_EXTENTS.into_iter().enumerate() {
            let link_position = first_link + index * SYNTHETIC_LINK_EXTENT;
            assert_eq!(
                u16::from_le_bytes(buffer[link_position..link_position + 2].try_into().unwrap()),
                SYNTHETIC_LINK_EXTENT as u16
            );
            let body = link_position + 2;
            assert_eq!(
                u16::from_le_bytes(buffer[body..body + 2].try_into().unwrap()),
                SYNTHETIC_LINK_MARKER
            );
            let link = PresentationLinkId::new(u32::from_le_bytes(
                buffer[body + 2..body + 6].try_into().unwrap(),
            ));
            let linked = resolve_presentation_queue_link(&buffer, link).unwrap();
            assert_eq!(linked.len(), expected_extent);
            assert_eq!(
                u16::from_le_bytes(buffer[body + 6..body + 8].try_into().unwrap()),
                expected_extent as u16
            );
        }

        assert_eq!(
            vectors[12].name,
            "rollover_invalid_cache_hits_malformed_suffix"
        );
        let mut stream = build_stream(Vec::new(), 0, vectors[12].initial_flags);
        stream.requested = Some(PresentationResourceId::new(2));
        stream.active = Some(PresentationResourceId::new(7));
        let mut descriptors = vec![descriptor(0, None); 8];
        descriptors[7] = descriptor(SOURCE_ROLLOVER_ENABLED_FLAG, None);
        let mut queue = build_queue(0);
        queue.wrap_count = 3;
        let mut buffer = vec![0u8; QUEUE_BUFFER_BYTE_COUNT];
        assert_eq!(
            refill_presentation_queue(
                &mut queue,
                &mut buffer,
                &mut stream,
                &descriptors,
                &mut links,
            ),
            Err(PresentationQueueRefillError::CachedRangeUnavailable {
                resource: PresentationResourceId::new(7),
            })
        );
        assert_eq!(stream.requested, stream.active);
    }

    #[test]
    fn refill_rejects_queue_capacity_outside_owned_storage() {
        let mut queue = build_queue(4);
        let initial_queue = queue.clone();
        let mut buffer = vec![0u8; queue.buffer_capacity - 1];
        let mut stream = build_stream(vec![0u8; 4], 0, 0);
        let mut links = PresentationQueueLinkCursor::default();

        assert_eq!(
            refill_presentation_queue(&mut queue, &mut buffer, &mut stream, &[], &mut links,),
            Err(PresentationQueueRefillError::Source(
                PresentationSourceError::QueueBufferTooShort {
                    queue_capacity: QUEUE_BUFFER_BYTE_COUNT,
                    buffer_len: QUEUE_BUFFER_BYTE_COUNT - 1,
                }
            ))
        );
        assert_eq!(queue, initial_queue);
    }
}
