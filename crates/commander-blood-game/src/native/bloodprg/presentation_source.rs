//! Owned byte transport for streamed presentation resources.

use std::error::Error;
use std::fmt;

use super::{PresentationQueueError, PresentationQueueState};

const ENTRY_HEADER_BYTE_COUNT: usize = 2;

/// One fully owned presentation byte stream and its flat read position.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationByteSource {
    bytes: Box<[u8]>,
    position: usize,
}

impl PresentationByteSource {
    /// Own a complete resource stream positioned at its first byte.
    pub fn new(bytes: impl Into<Box<[u8]>>) -> Self {
        Self {
            bytes: bytes.into(),
            position: usize::MIN,
        }
    }

    /// Return the complete immutable resource bytes.
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Return the next unread byte position.
    pub const fn position(&self) -> usize {
        self.position
    }

    /// Return the byte count still available to the queue.
    pub fn remaining(&self) -> usize {
        self.bytes.len() - self.position
    }

    /// Move to a validated flat byte position within this owned stream.
    pub fn seek(&mut self, position: usize) -> Result<(), PresentationSourceError> {
        if position > self.bytes.len() {
            return Err(PresentationSourceError::PositionOutOfRange {
                position,
                source_len: self.bytes.len(),
            });
        }
        self.position = position;
        Ok(())
    }
}

/// Invalid owned-source or queue transfer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationSourceError {
    /// No presentation source is currently active.
    SourceUnavailable,
    /// The owned source ends before the requested exact transfer.
    SourceTruncated {
        /// Next unread source position.
        position: usize,
        /// Exact byte count requested by game logic.
        requested: usize,
        /// Available bytes from the current position.
        remaining: usize,
    },
    /// A flat seek requested a position beyond the owned source.
    PositionOutOfRange {
        /// Requested absolute byte position within the owned source.
        position: usize,
        /// Complete source byte count.
        source_len: usize,
    },
    /// A selected source range extends beyond the owned resource.
    SourceRangeOutOfBounds {
        /// First byte selected by the range.
        position: usize,
        /// Number of bytes selected by the range.
        remaining: usize,
        /// Complete source byte count.
        source_len: usize,
    },
    /// Queue accounting or destination geometry rejected the transfer.
    Queue(PresentationQueueError),
    /// Queue capacity names bytes not owned by the supplied flat buffer.
    QueueBufferTooShort {
        /// Capacity recorded by queue state.
        queue_capacity: usize,
        /// Actual owned-buffer length.
        buffer_len: usize,
    },
    /// An extent-prefixed entry is shorter than its header.
    EntryExtentTooSmall {
        /// Serialized extent value.
        extent: usize,
    },
    /// The initial entry and its retained guard do not fit the queue buffer.
    InitialEntryTooLarge {
        /// Serialized extent including the entry header.
        extent: usize,
        /// Available queue-buffer length.
        buffer_len: usize,
    },
}

impl fmt::Display for PresentationSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation source operation: {self:?}")
    }
}

impl Error for PresentationSourceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Queue(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PresentationQueueError> for PresentationSourceError {
    fn from(source: PresentationQueueError) -> Self {
        Self::Queue(source)
    }
}

/// Decoded extent header and queue position immediately after that header.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationEntryExtent {
    /// Complete serialized entry extent.
    pub extent: usize,
    /// Flat queue-buffer position immediately after the header.
    pub payload_cursor: usize,
}

fn checked_source_end(
    source: &PresentationByteSource,
    byte_count: usize,
) -> Result<usize, PresentationSourceError> {
    source
        .position
        .checked_add(byte_count)
        .filter(|end| *end <= source.bytes.len())
        .ok_or(PresentationSourceError::SourceTruncated {
            position: source.position,
            requested: byte_count,
            remaining: source.remaining(),
        })
}

/// Append an exact byte range from the active owned source into the queue.
///
/// This translates `ems_paged_read` at BLOODPRG offset `0x00A664`. Complete
/// owned bytes replace EMS pages, XMS move records, DOS handles, seeks, and
/// short-read retries. Checked flat positions preserve exact logical byte
/// accounting and make malformed transfers transactional.
pub fn append_presentation_source_bytes(
    source: Option<&mut PresentationByteSource>,
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
    byte_count: usize,
) -> Result<(), PresentationSourceError> {
    let source = source.ok_or(PresentationSourceError::SourceUnavailable)?;
    if queue.buffer_capacity > queue_buffer.len() {
        return Err(PresentationSourceError::QueueBufferTooShort {
            queue_capacity: queue.buffer_capacity,
            buffer_len: queue_buffer.len(),
        });
    }
    let source_end = checked_source_end(source, byte_count)?;
    let mut staged_queue = queue.clone();
    staged_queue.enqueue(byte_count)?;
    let buffer_len = queue_buffer.len();
    let destination = queue_buffer.get_mut(queue.head..staged_queue.head).ok_or(
        PresentationSourceError::QueueBufferTooShort {
            queue_capacity: staged_queue.head,
            buffer_len,
        },
    )?;
    destination.copy_from_slice(&source.bytes[source.position..source_end]);
    source.position = source_end;
    *queue = staged_queue;
    Ok(())
}

/// Read and decode one two-byte extent at the queue head.
///
/// This translates `list_d8c_read` at `0x00A622`. The returned flat cursor
/// replaces the original far pointer result, and an unavailable source leaves
/// both outputs and queue state unchanged.
pub fn read_presentation_entry_extent(
    source: Option<&mut PresentationByteSource>,
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
) -> Result<PresentationEntryExtent, PresentationSourceError> {
    append_presentation_source_bytes(source, queue, queue_buffer, ENTRY_HEADER_BYTE_COUNT)?;
    let payload_cursor = queue.head;
    let header_start = payload_cursor - ENTRY_HEADER_BYTE_COUNT;
    let extent = usize::from(u16::from_le_bytes(
        queue_buffer[header_start..payload_cursor]
            .try_into()
            .expect("the exact two-byte append validated this header"),
    ));
    Ok(PresentationEntryExtent {
        extent,
        payload_cursor,
    })
}

/// Reset the queue and place its first complete extent-prefixed entry.
///
/// This translates `banked_list_load` at `0x00A642`. The retained entry stays
/// at the original end-relative position, but the complete source is validated
/// before publication. Extent underflow and disappearing DOS handles become
/// explicit malformed-source errors instead of wrapped body reads.
pub fn load_initial_presentation_entry(
    source: Option<&mut PresentationByteSource>,
    queue: &mut PresentationQueueState,
    queue_buffer: &mut [u8],
) -> Result<PresentationEntryExtent, PresentationSourceError> {
    queue.reset(queue_buffer.len());
    let source = source.ok_or(PresentationSourceError::SourceUnavailable)?;
    let header_end = checked_source_end(source, ENTRY_HEADER_BYTE_COUNT)?;
    let extent = usize::from(u16::from_le_bytes(
        source.bytes[source.position..header_end]
            .try_into()
            .expect("validated two-byte entry header"),
    ));
    if extent < ENTRY_HEADER_BYTE_COUNT {
        return Err(PresentationSourceError::EntryExtentTooSmall { extent });
    }
    let source_end = checked_source_end(source, extent)?;
    let retained_bytes = extent.checked_add(ENTRY_HEADER_BYTE_COUNT).ok_or(
        PresentationSourceError::InitialEntryTooLarge {
            extent,
            buffer_len: queue_buffer.len(),
        },
    )?;
    let entry_start = queue_buffer.len().checked_sub(retained_bytes).ok_or(
        PresentationSourceError::InitialEntryTooLarge {
            extent,
            buffer_len: queue_buffer.len(),
        },
    )?;
    let entry_end = entry_start + extent;
    queue_buffer[entry_start..entry_end]
        .copy_from_slice(&source.bytes[source.position..source_end]);

    source.position = source_end;
    queue.tail = entry_start;
    queue.head = entry_end;
    queue.queued_bytes = extent;
    Ok(PresentationEntryExtent {
        extent,
        payload_cursor: entry_start + ENTRY_HEADER_BYTE_COUNT,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const SOURCE_APPEND_VECTOR_COUNT: usize = 9;
    const ENTRY_READ_VECTOR_COUNT: usize = 6;
    const INITIAL_ENTRY_VECTOR_COUNT: usize = 6;
    const FLAT_QUEUE_BUFFER_BYTE_COUNT: usize = u16::MAX as usize + 1;
    const SOURCE_PATTERN_STEP: usize = 37;
    const SOURCE_PATTERN_BIAS: usize = 11;
    const NATIVE_WORD_CAPACITY: usize = u16::MAX as usize;

    #[derive(Deserialize)]
    struct SourceAppendOracle {
        name: String,
        success: bool,
        requested: usize,
        transferred: Option<usize>,
        result: SourceAppendResult,
    }

    #[derive(Deserialize)]
    struct SourceAppendResult {
        head: u16,
        queued: u16,
    }

    #[derive(Deserialize)]
    struct EntryReadOracle {
        name: String,
        success: bool,
        handle: u16,
        initial_head: u16,
        initial_byte_count: u16,
        extent: Option<u16>,
    }

    #[derive(Deserialize)]
    struct InitialEntryOracle {
        name: String,
        success: bool,
        extent: u16,
        body_count: Option<u16>,
        entry_start: Option<u16>,
        result: InitialEntryResult,
    }

    #[derive(Deserialize)]
    struct InitialEntryResult {
        head: u16,
        tail: u16,
        wrap_limit: u16,
        byte_count: u16,
    }

    fn patterned_bytes(byte_count: usize) -> Box<[u8]> {
        (usize::MIN..byte_count)
            .map(|index| (index * SOURCE_PATTERN_STEP + SOURCE_PATTERN_BIAS) as u8)
            .collect()
    }

    fn queue_state(head: usize, queued_bytes: usize) -> PresentationQueueState {
        PresentationQueueState {
            head,
            tail: usize::MIN,
            queued_bytes,
            buffer_capacity: FLAT_QUEUE_BUFFER_BYTE_COUNT,
            wrap_limit: FLAT_QUEUE_BUFFER_BYTE_COUNT,
            ..PresentationQueueState::default()
        }
    }

    #[test]
    fn owned_source_append_accounts_for_every_transport_vector() {
        let vectors: Vec<SourceAppendOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a664_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SOURCE_APPEND_VECTOR_COUNT);

        for vector in vectors {
            let native_transfer = vector.transferred.unwrap_or(usize::MIN);
            let initial_head = usize::from(vector.result.head.wrapping_sub(native_transfer as u16));
            let initial_queued =
                usize::from(vector.result.queued.wrapping_sub(native_transfer as u16));
            let mut queue = queue_state(initial_head, initial_queued);
            let initial_queue = queue.clone();
            let mut queue_buffer = vec![u8::MIN; FLAT_QUEUE_BUFFER_BYTE_COUNT];
            let before = queue_buffer.clone();
            let mut source = PresentationByteSource::new(patterned_bytes(vector.requested));
            let result = append_presentation_source_bytes(
                vector.success.then_some(&mut source),
                &mut queue,
                &mut queue_buffer,
                vector.requested,
            );

            if !vector.success {
                assert_eq!(result, Err(PresentationSourceError::SourceUnavailable));
                assert_eq!(queue, initial_queue, "{}", vector.name);
                assert_eq!(queue_buffer, before, "{}", vector.name);
                continue;
            }
            let flat_transfer_fits = initial_head + vector.requested
                <= FLAT_QUEUE_BUFFER_BYTE_COUNT
                && initial_queued + vector.requested <= FLAT_QUEUE_BUFFER_BYTE_COUNT;
            if !flat_transfer_fits {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(queue, initial_queue, "{}", vector.name);
                assert_eq!(queue_buffer, before, "{}", vector.name);
                continue;
            }

            result.unwrap();
            assert_eq!(
                queue.head,
                initial_head + vector.requested,
                "{}",
                vector.name
            );
            assert_eq!(
                queue.queued_bytes,
                initial_queued + vector.requested,
                "{}",
                vector.name
            );
            assert_eq!(source.position(), vector.requested, "{}", vector.name);
            assert_eq!(source.remaining(), usize::MIN, "{}", vector.name);
            assert_eq!(
                &queue_buffer[initial_head..queue.head],
                source.bytes(),
                "{}",
                vector.name
            );
            assert_eq!(native_transfer, vector.requested, "{}", vector.name);
        }
    }

    #[test]
    fn extent_read_accounts_for_every_original_transport_case() {
        let vectors: Vec<EntryReadOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a622_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ENTRY_READ_VECTOR_COUNT);

        for vector in vectors {
            let mut queue = queue_state(
                usize::from(vector.initial_head),
                usize::from(vector.initial_byte_count),
            );
            let initial_queue = queue.clone();
            let mut queue_buffer = vec![u8::MIN; FLAT_QUEUE_BUFFER_BYTE_COUNT];
            let mut source = vector
                .extent
                .map(|extent| PresentationByteSource::new(Box::<[u8]>::from(extent.to_le_bytes())));
            let active_source = if vector.handle == u16::MIN {
                None
            } else {
                Some(source.as_mut().expect("available oracle source"))
            };
            let result =
                read_presentation_entry_extent(active_source, &mut queue, &mut queue_buffer);

            if !vector.success {
                assert_eq!(result, Err(PresentationSourceError::SourceUnavailable));
                assert_eq!(queue, initial_queue, "{}", vector.name);
                continue;
            }
            if usize::from(vector.initial_head) + ENTRY_HEADER_BYTE_COUNT
                > FLAT_QUEUE_BUFFER_BYTE_COUNT
            {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(queue, initial_queue, "{}", vector.name);
                continue;
            }

            let extent = usize::from(vector.extent.unwrap());
            assert_eq!(
                result.unwrap(),
                PresentationEntryExtent {
                    extent,
                    payload_cursor: usize::from(vector.initial_head) + ENTRY_HEADER_BYTE_COUNT,
                },
                "{}",
                vector.name
            );
            assert_eq!(
                queue.queued_bytes,
                usize::from(vector.initial_byte_count) + ENTRY_HEADER_BYTE_COUNT,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn initial_entry_load_matches_valid_vectors_and_rejects_native_underflow() {
        let vectors: Vec<InitialEntryOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a642_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), INITIAL_ENTRY_VECTOR_COUNT);

        for vector in vectors {
            let buffer_len = usize::from(vector.result.wrap_limit);
            let mut queue_buffer = vec![u8::MIN; buffer_len];
            let mut queue = PresentationQueueState {
                head: NATIVE_WORD_CAPACITY,
                tail: NATIVE_WORD_CAPACITY,
                queued_bytes: NATIVE_WORD_CAPACITY,
                active_entry: true,
                ..PresentationQueueState::default()
            };
            let source_len = if vector.name == "body_handle_removed" {
                ENTRY_HEADER_BYTE_COUNT
            } else {
                usize::from(vector.extent).max(ENTRY_HEADER_BYTE_COUNT)
            };
            let mut source_bytes = patterned_bytes(source_len);
            source_bytes[..ENTRY_HEADER_BYTE_COUNT].copy_from_slice(&vector.extent.to_le_bytes());
            let mut source = PresentationByteSource::new(source_bytes);
            let result = load_initial_presentation_entry(
                (vector.name != "initial_no_handle").then_some(&mut source),
                &mut queue,
                &mut queue_buffer,
            );

            let flat_success = vector.success
                && usize::from(vector.extent) >= ENTRY_HEADER_BYTE_COUNT
                && vector.body_count != Some(u16::MAX);
            if !flat_success {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(queue.head, usize::MIN, "{}", vector.name);
                assert_eq!(queue.tail, usize::MIN, "{}", vector.name);
                assert_eq!(queue.queued_bytes, usize::MIN, "{}", vector.name);
                assert_eq!(queue.buffer_capacity, buffer_len, "{}", vector.name);
                continue;
            }

            let extent = usize::from(vector.extent);
            let entry_start = usize::from(vector.entry_start.unwrap());
            assert_eq!(
                result.unwrap(),
                PresentationEntryExtent {
                    extent,
                    payload_cursor: entry_start + ENTRY_HEADER_BYTE_COUNT,
                },
                "{}",
                vector.name
            );
            assert_eq!(
                queue.head,
                usize::from(vector.result.head),
                "{}",
                vector.name
            );
            assert_eq!(
                queue.tail,
                usize::from(vector.result.tail),
                "{}",
                vector.name
            );
            assert_eq!(
                queue.queued_bytes,
                usize::from(vector.result.byte_count),
                "{}",
                vector.name
            );
            assert_eq!(source.position(), extent, "{}", vector.name);
            assert_eq!(
                &queue_buffer[entry_start..entry_start + extent],
                source.bytes(),
                "{}",
                vector.name
            );
        }
    }
}
