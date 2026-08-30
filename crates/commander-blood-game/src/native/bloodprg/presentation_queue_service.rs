//! One-frame service of the active streamed presentation queue.

use std::error::Error;
use std::fmt;

use super::{
    PresentationActiveEntryError, PresentationActiveEntryOutcome, PresentationActiveEntryState,
    PresentationEntryDisposition, PresentationEntryError, PresentationEntryPolicy,
    PresentationEntryPresenter, PresentationEntryReadiness, PresentationPaletteError,
    PresentationPaletteOutcome, PresentationPaletteState, PresentationPresentPolicy,
    PresentationQueueAdvance, PresentationQueueClock, PresentationQueueClockGates,
    PresentationQueueConsumeOutcome, PresentationQueueError, PresentationQueueLinkCursor,
    PresentationQueueRefillError, PresentationQueueRefillOutcome, PresentationQueueState,
    PresentationResourceDescriptor, PresentationResourceStreamState, PresentationSourceLease,
    activate_presentation_entry, apply_presentation_palette_blocks, present_active_entry,
    presentation_entry_activation_request, presentation_queue_advance_due,
    refill_presentation_queue, resolve_presentation_queue_link,
};

const HIGH_PRIORITY_REFILL_FLAG: u8 = 128;

/// Mutable dependencies used while servicing one queue frame.
pub struct PresentationQueueServiceContext<'a, Host> {
    /// Authored descriptors used by cached-range rollover.
    pub descriptors: &'a [PresentationResourceDescriptor],
    /// Queue state shared by activation, consumption, and refill.
    pub queue: &'a mut PresentationQueueState,
    /// Flat owned circular queue allocation.
    pub queue_buffer: &'a mut [u8],
    /// Parsing and decode gates for a newly ready entry.
    pub entry_policy: PresentationEntryPolicy,
    /// Active frame, retained palette payload, and owning queue extent.
    pub active_entry: &'a mut PresentationActiveEntryState,
    /// Destination and row policies for a due frame.
    pub present_policy: PresentationPresentPolicy,
    /// Concrete flat-frame presenter.
    pub host: &'a mut Host,
    /// Palette receiving blocks immediately before a due frame is presented.
    pub palette: &'a mut PresentationPaletteState,
    /// Palette snapshot suppression flags.
    pub render_update_flags: u8,
    /// Queue pacing state.
    pub clock: &'a mut PresentationQueueClock,
    /// Runtime gates selecting audio-phase or software-tick pacing.
    pub clock_gates: PresentationQueueClockGates,
    /// Current voice-playback position source.
    pub audio_position: &'a mut dyn FnMut() -> u16,
    /// Current software timer source; a due result reads it twice.
    pub timer_tick: &'a mut dyn FnMut() -> u16,
    /// Cursor advanced by synthetic rollover links.
    pub link_cursor: &'a mut PresentationQueueLinkCursor,
}

/// Terminal result from one presentation queue service call.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationQueueServiceOutcome {
    /// A non-archive source has no active owned byte stream.
    SourceUnavailable,
    /// The non-archive high-priority path performed only its latched refill.
    HighPriorityRefill {
        /// Refill result observed while the high flag was published.
        refill: PresentationQueueRefillOutcome,
    },
    /// Refill made no source progress, so this frame returns without spinning.
    WaitingForEntry {
        /// Successful source transfers made before the stalled result.
        retry_refills: usize,
        /// Backpressure or source-completion result ending the retry loop.
        refill: PresentationQueueRefillOutcome,
    },
    /// A stale synthetic link was consumed and the queue was refilled once.
    RejectedStaleLink {
        /// Successful source transfers made before the stale entry activated.
        retry_refills: usize,
        /// Result of the final latched refill.
        refill: PresentationQueueRefillOutcome,
    },
    /// An active entry was paced and the queue received its final refill.
    Active {
        /// Successful source transfers required before activation.
        retry_refills: usize,
        /// Audio-phase or software-tick pacing result.
        advance: PresentationQueueAdvance,
        /// Palette update applied immediately before presentation when due.
        palette: Option<PresentationPaletteOutcome>,
        /// Rendering work completed when the frame was due.
        present: Option<PresentationActiveEntryOutcome>,
        /// Complete embedded sound side-record retired with the due frame.
        sound_record: Option<Box<[u8]>>,
        /// Queue retirement completed when the frame was due.
        consumed: Option<PresentationQueueConsumeOutcome>,
        /// Result of the final latched refill.
        refill: PresentationQueueRefillOutcome,
    },
}

/// Invalid queue, entry, palette, pacing, presentation, or refill state.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationQueueServiceError {
    /// Queue geometry rejected readiness or entry retirement.
    Queue(PresentationQueueError),
    /// Entry grammar or payload decoding failed.
    Activation(PresentationEntryError),
    /// Palette blocks retained by the active entry were malformed.
    Palette(PresentationPaletteError),
    /// Active frame presentation failed.
    Present(PresentationActiveEntryError),
    /// Source refill found malformed queue, descriptor, or source state.
    Refill(PresentationQueueRefillError),
    /// Queue state says an entry is active but no owned frame is retained.
    ActiveEntryUnavailable,
    /// A due active frame has no retained queue extent to consume.
    ActiveEntryExtentUnavailable,
}

impl fmt::Display for PresentationQueueServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation queue service: {self:?}")
    }
}

impl Error for PresentationQueueServiceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Queue(source) => Some(source),
            Self::Activation(source) => Some(source),
            Self::Palette(source) => Some(source),
            Self::Present(source) => Some(source),
            Self::Refill(source) => Some(source),
            Self::ActiveEntryUnavailable | Self::ActiveEntryExtentUnavailable => None,
        }
    }
}

impl From<PresentationQueueError> for PresentationQueueServiceError {
    fn from(error: PresentationQueueError) -> Self {
        Self::Queue(error)
    }
}

impl From<PresentationEntryError> for PresentationQueueServiceError {
    fn from(error: PresentationEntryError) -> Self {
        Self::Activation(error)
    }
}

impl From<PresentationPaletteError> for PresentationQueueServiceError {
    fn from(error: PresentationPaletteError) -> Self {
        Self::Palette(error)
    }
}

impl From<PresentationActiveEntryError> for PresentationQueueServiceError {
    fn from(error: PresentationActiveEntryError) -> Self {
        Self::Present(error)
    }
}

impl From<PresentationQueueRefillError> for PresentationQueueServiceError {
    fn from(error: PresentationQueueRefillError) -> Self {
        Self::Refill(error)
    }
}

fn latched_refill<Host>(
    stream: &mut PresentationResourceStreamState,
    context: &mut PresentationQueueServiceContext<'_, Host>,
) -> Result<PresentationQueueRefillOutcome, PresentationQueueServiceError> {
    let flags = stream.flags;
    Ok(context.queue.refill_with_rollover_latch(
        flags,
        context.link_cursor.position(),
        |queue, _| {
            refill_presentation_queue(
                queue,
                context.queue_buffer,
                stream,
                context.descriptors,
                context.link_cursor,
            )
        },
    )?)
}

/// Service one ready or partially buffered presentation entry.
///
/// This translates `ems_resource_flush` at BLOODPRG offset `0x00A1B4`.
/// Source availability and the non-archive high-priority path are checked
/// first. Ordinary operation refills until an entry activates, retains its
/// side records, paces it through the recovered queue clock, applies its
/// final palette record only when due, presents and consumes it, and performs
/// one final refill while the authored high flag is latched. A refill that
/// cannot make progress returns `WaitingForEntry` instead of reproducing the
/// original routine's unbounded retry hang.
pub fn service_presentation_queue<Host>(
    stream: &mut PresentationResourceStreamState,
    context: &mut PresentationQueueServiceContext<'_, Host>,
) -> Result<PresentationQueueServiceOutcome, PresentationQueueServiceError>
where
    Host: PresentationEntryPresenter,
{
    let shared_source = matches!(
        stream.lease,
        PresentationSourceLease::SharedArchive | PresentationSourceLease::SharedCache
    );
    if !shared_source {
        if stream.source.is_none() {
            context.queue.rollover_latched = false;
            return Ok(PresentationQueueServiceOutcome::SourceUnavailable);
        }
        if stream.flags.to_le_bytes()[0] & HIGH_PRIORITY_REFILL_FLAG != 0 {
            let refill = latched_refill(stream, context)?;
            return Ok(PresentationQueueServiceOutcome::HighPriorityRefill { refill });
        }
    }

    let mut retry_refills = usize::MIN;
    loop {
        match presentation_entry_activation_request(
            context.queue,
            context.queue_buffer,
            stream.flags,
        )? {
            PresentationEntryReadiness::NotReady => {
                let refill = refill_presentation_queue(
                    context.queue,
                    context.queue_buffer,
                    stream,
                    context.descriptors,
                    context.link_cursor,
                )?;
                if matches!(refill, PresentationQueueRefillOutcome::Transferred { .. }) {
                    retry_refills += 1;
                    continue;
                }
                return Ok(PresentationQueueServiceOutcome::WaitingForEntry {
                    retry_refills,
                    refill,
                });
            }
            PresentationEntryReadiness::AlreadyActive => {
                if context.active_entry.active.is_none() {
                    return Err(PresentationQueueServiceError::ActiveEntryUnavailable);
                }
            }
            PresentationEntryReadiness::Activate(request) => {
                let activation = activate_presentation_entry(
                    context.queue_buffer,
                    request,
                    context.entry_policy,
                    |link| resolve_presentation_queue_link(context.queue_buffer, link),
                )?;
                context.active_entry.active_sound_record = activation.side_data.sound_record;
                context.active_entry.pending_palette_payload = activation.side_data.palette_payload;
                match activation.disposition {
                    PresentationEntryDisposition::Active(entry) => {
                        context.active_entry.active = Some(entry);
                        context.active_entry.active_queue_extent = Some(request.entry_extent);
                        context.queue.active_entry = true;
                    }
                    PresentationEntryDisposition::RejectedLink { .. } => {
                        context.queue.consume_entry(request.entry_extent)?;
                        context.queue.active_entry = false;
                        context.active_entry.active = None;
                        context.active_entry.active_queue_extent = None;
                        context.active_entry.active_sound_record = None;
                        context.active_entry.pending_palette_payload = None;
                        let refill = latched_refill(stream, context)?;
                        return Ok(PresentationQueueServiceOutcome::RejectedStaleLink {
                            retry_refills,
                            refill,
                        });
                    }
                }
            }
        }

        let advance = presentation_queue_advance_due(
            context.clock,
            context.clock_gates,
            || (context.audio_position)(),
            || (context.timer_tick)(),
        );
        let mut palette = None;
        let mut present = None;
        let mut sound_record = None;
        let mut consumed = None;
        if advance.due {
            if let Some(payload) = context.active_entry.pending_palette_payload.take() {
                palette = Some(apply_presentation_palette_blocks(
                    &payload,
                    context.palette,
                    context.render_update_flags,
                    context.queue.read_wrap_index,
                    &mut stream.entry_metric,
                )?);
            }
            let entry_extent = context
                .active_entry
                .active_queue_extent
                .ok_or(PresentationQueueServiceError::ActiveEntryExtentUnavailable)?;
            present = Some(present_active_entry(
                context.active_entry,
                context.present_policy,
                context.host,
            )?);
            sound_record = context.active_entry.active_sound_record.take();
            consumed = Some(context.queue.consume_entry(entry_extent)?);
            context.queue.active_entry = false;
            context.active_entry.active_queue_extent = None;
        }
        let refill = latched_refill(stream, context)?;
        return Ok(PresentationQueueServiceOutcome::Active {
            retry_refills,
            advance,
            palette,
            present,
            sound_record,
            consumed,
            refill,
        });
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{
        PresentationByteSource, PresentationEntryRenderTarget, PresentationRectBlitOutcome,
        PresentationRectDecodeOutcome,
    };

    const SERVICE_VECTOR_COUNT: usize = 5;
    const QUEUE_BUFFER_BYTE_COUNT: usize = 65_536;
    const EMPTY_FRAME_LAYOUT: u16 = 1_024;
    const EMPTY_FRAME_EXTENT: usize = 6;
    const PALETTE_FRAME_EXTENT: usize = 12;
    const SOUND_FRAME_EXTENT: usize = 12;
    const SOUND_RECORD_EXTENT: usize = 6;
    const SOUND_RECORD_PAYLOAD: [u8; 2] = [0x12, 0x34];
    const RETRY_FRAME_EXTENT: usize = 3_002;

    #[derive(Deserialize)]
    struct ServiceOracle {
        name: String,
        resource_flags: u16,
        link_target_offset: usize,
        calls: Vec<serde_json::Value>,
    }

    #[derive(Default)]
    struct RecordingHost {
        present_calls: usize,
    }

    impl PresentationEntryPresenter for RecordingHost {
        fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError> {
            self.present_calls += 1;
            Ok(())
        }

        fn blit_rectangle(
            &mut self,
            _source: &[u8],
            _target: PresentationEntryRenderTarget,
            _x: usize,
            _y: usize,
            _width: usize,
            _row_mode: u16,
        ) -> Result<PresentationRectBlitOutcome, PresentationActiveEntryError> {
            self.present_calls += 1;
            Ok(PresentationRectBlitOutcome {
                consumed_bytes: usize::MIN,
                changed_pixels: usize::MIN,
            })
        }

        fn decode_rectangle(
            &mut self,
            _source: &[u8],
            _vertical_offset: usize,
            _layout: u16,
            _row_mode: u16,
        ) -> Result<PresentationRectDecodeOutcome, PresentationActiveEntryError> {
            self.present_calls += 1;
            Ok(PresentationRectDecodeOutcome {
                consumed_bytes: usize::MIN,
                staged_values_consumed: usize::MIN,
                changed_pixels: usize::MIN,
                x: usize::MIN,
                y: usize::MIN,
                width: usize::MIN,
                rows: usize::MIN,
                final_row_offset: usize::MIN,
                final_destination_offset: usize::MIN,
            })
        }
    }

    fn write_word(buffer: &mut [u8], position: usize, value: usize) {
        buffer[position..position + size_of::<u16>()]
            .copy_from_slice(&(value as u16).to_le_bytes());
    }

    fn ready_queue(with_palette: bool) -> (PresentationQueueState, Vec<u8>) {
        let extent = if with_palette {
            PALETTE_FRAME_EXTENT
        } else {
            EMPTY_FRAME_EXTENT
        };
        let mut buffer = vec![u8::MIN; QUEUE_BUFFER_BYTE_COUNT];
        write_word(&mut buffer, 0, extent);
        let frame_position = if with_palette {
            write_word(&mut buffer, 2, u16::from_le_bytes(*b"pl") as usize);
            write_word(&mut buffer, 4, 6);
            buffer[6..8].fill(u8::MAX);
            8
        } else {
            2
        };
        write_word(&mut buffer, frame_position, EMPTY_FRAME_LAYOUT as usize);
        let queue = PresentationQueueState {
            head: extent,
            tail: usize::MIN,
            queued_bytes: extent,
            buffer_capacity: QUEUE_BUFFER_BYTE_COUNT,
            wrap_limit: QUEUE_BUFFER_BYTE_COUNT,
            read_wrap_index: 1,
            ..PresentationQueueState::default()
        };
        (queue, buffer)
    }

    fn retry_queue() -> (PresentationQueueState, Vec<u8>, Vec<u8>) {
        let mut buffer = vec![u8::MIN; QUEUE_BUFFER_BYTE_COUNT];
        write_word(&mut buffer, usize::MIN, RETRY_FRAME_EXTENT);
        let mut source = vec![u8::MIN; RETRY_FRAME_EXTENT - size_of::<u16>()];
        write_word(&mut source, usize::MIN, EMPTY_FRAME_LAYOUT as usize);
        let queue = PresentationQueueState {
            head: size_of::<u16>(),
            tail: usize::MIN,
            queued_bytes: size_of::<u16>(),
            pending_entry_bytes: source.len(),
            buffer_capacity: QUEUE_BUFFER_BYTE_COUNT,
            wrap_limit: QUEUE_BUFFER_BYTE_COUNT,
            read_wrap_index: 1,
            ..PresentationQueueState::default()
        };
        (queue, buffer, source)
    }

    #[test]
    fn queue_service_accounts_for_every_original_coordinator_vector() {
        let vectors: Vec<ServiceOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a1b4_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SERVICE_VECTOR_COUNT);

        for vector in vectors {
            let with_palette = vector.name == "file_due_with_palette";
            let (mut queue, mut queue_buffer, source_bytes) =
                if vector.name == "retry_twice_then_not_due" {
                    let (queue, buffer, source) = retry_queue();
                    (queue, buffer, source)
                } else {
                    let (queue, buffer) = ready_queue(with_palette);
                    (queue, buffer, Vec::new())
                };
            let mut stream = PresentationResourceStreamState {
                flags: if vector.name == "retry_twice_then_not_due" {
                    vector.resource_flags & !u16::from(HIGH_PRIORITY_REFILL_FLAG)
                } else {
                    vector.resource_flags
                },
                ready: true,
                lease: if matches!(
                    vector.name.as_str(),
                    "retry_twice_then_not_due" | "banked_due_without_palette"
                ) {
                    PresentationSourceLease::SharedArchive
                } else {
                    PresentationSourceLease::Owned
                },
                source: (vector.name != "no_file_handle")
                    .then(|| PresentationByteSource::new(source_bytes)),
                ..PresentationResourceStreamState::default()
            };
            let mut active_entry = PresentationActiveEntryState::default();
            let mut host = RecordingHost::default();
            let mut palette = PresentationPaletteState::default();
            let due = matches!(
                vector.name.as_str(),
                "banked_due_without_palette" | "file_due_with_palette"
            );
            let mut clock = PresentationQueueClock {
                tick_threshold: if due { 1 } else { 10 },
                ..PresentationQueueClock::default()
            };
            let mut audio_position = || u16::MIN;
            let mut timer_values = if due {
                VecDeque::from([2, 3])
            } else {
                VecDeque::from([u16::MIN])
            };
            let mut timer_tick = || timer_values.pop_front().unwrap_or(u16::MIN);
            let mut link_cursor = PresentationQueueLinkCursor::new(vector.link_target_offset);
            let mut context = PresentationQueueServiceContext {
                descriptors: &[],
                queue: &mut queue,
                queue_buffer: &mut queue_buffer,
                entry_policy: PresentationEntryPolicy::default(),
                active_entry: &mut active_entry,
                present_policy: PresentationPresentPolicy {
                    skip_back_buffer_present: true,
                    ..PresentationPresentPolicy::default()
                },
                host: &mut host,
                palette: &mut palette,
                render_update_flags: u8::MIN,
                clock: &mut clock,
                clock_gates: PresentationQueueClockGates::default(),
                audio_position: &mut audio_position,
                timer_tick: &mut timer_tick,
                link_cursor: &mut link_cursor,
            };
            let result = service_presentation_queue(&mut stream, &mut context).unwrap();

            match vector.name.as_str() {
                "no_file_handle" => {
                    assert_eq!(result, PresentationQueueServiceOutcome::SourceUnavailable);
                }
                "malformed_nonbanked_high_bit_call" => {
                    assert!(matches!(
                        result,
                        PresentationQueueServiceOutcome::HighPriorityRefill { .. }
                    ));
                }
                "retry_twice_then_not_due" => match result {
                    PresentationQueueServiceOutcome::Active {
                        retry_refills,
                        advance,
                        present,
                        consumed,
                        ..
                    } => {
                        assert_eq!(retry_refills, 2);
                        assert!(!advance.due);
                        assert!(present.is_none());
                        assert!(consumed.is_none());
                    }
                    other => panic!("unexpected retry vector outcome {other:?}"),
                },
                "banked_due_without_palette" | "file_due_with_palette" => match result {
                    PresentationQueueServiceOutcome::Active {
                        advance,
                        palette: palette_outcome,
                        present,
                        consumed,
                        ..
                    } => {
                        assert!(advance.due);
                        assert_eq!(palette_outcome.is_some(), with_palette);
                        assert!(present.is_some_and(|outcome| outcome.frame_presented));
                        assert!(consumed.is_some());
                        assert_eq!(palette.dirty, with_palette);
                    }
                    other => panic!("unexpected due vector outcome {other:?}"),
                },
                name => panic!("unknown service oracle vector {name}"),
            }
            assert!(!queue.rollover_latched, "{}", vector.name);
            assert!(!vector.calls.is_empty() || vector.name == "no_file_handle");
        }
    }

    #[test]
    fn due_frame_returns_its_embedded_sound_record_to_the_runtime() {
        let mut queue_buffer = vec![u8::MIN; QUEUE_BUFFER_BYTE_COUNT];
        write_word(&mut queue_buffer, usize::MIN, SOUND_FRAME_EXTENT);
        write_word(&mut queue_buffer, 2, u16::from_le_bytes(*b"sd") as usize);
        write_word(&mut queue_buffer, 4, SOUND_RECORD_EXTENT);
        queue_buffer[6..8].copy_from_slice(&SOUND_RECORD_PAYLOAD);
        write_word(&mut queue_buffer, 8, EMPTY_FRAME_LAYOUT as usize);
        let mut queue = PresentationQueueState {
            head: SOUND_FRAME_EXTENT,
            tail: usize::MIN,
            queued_bytes: SOUND_FRAME_EXTENT,
            buffer_capacity: QUEUE_BUFFER_BYTE_COUNT,
            wrap_limit: QUEUE_BUFFER_BYTE_COUNT,
            read_wrap_index: 1,
            ..PresentationQueueState::default()
        };
        let mut stream = PresentationResourceStreamState {
            ready: true,
            lease: PresentationSourceLease::SharedArchive,
            source: Some(PresentationByteSource::new(Box::<[u8]>::default())),
            ..PresentationResourceStreamState::default()
        };
        let mut active_entry = PresentationActiveEntryState::default();
        let mut host = RecordingHost::default();
        let mut palette = PresentationPaletteState::default();
        let mut clock = PresentationQueueClock::default();
        let mut audio_position = || u16::MIN;
        let mut timer_tick = || u16::MIN;
        let mut link_cursor = PresentationQueueLinkCursor::default();
        let outcome = service_presentation_queue(
            &mut stream,
            &mut PresentationQueueServiceContext {
                descriptors: &[],
                queue: &mut queue,
                queue_buffer: &mut queue_buffer,
                entry_policy: PresentationEntryPolicy {
                    sound_enabled: true,
                    ..PresentationEntryPolicy::default()
                },
                active_entry: &mut active_entry,
                present_policy: PresentationPresentPolicy {
                    skip_back_buffer_present: true,
                    ..PresentationPresentPolicy::default()
                },
                host: &mut host,
                palette: &mut palette,
                render_update_flags: u8::MIN,
                clock: &mut clock,
                clock_gates: PresentationQueueClockGates::default(),
                audio_position: &mut audio_position,
                timer_tick: &mut timer_tick,
                link_cursor: &mut link_cursor,
            },
        )
        .unwrap();

        let PresentationQueueServiceOutcome::Active { sound_record, .. } = outcome else {
            panic!("sound-bearing frame did not become active");
        };
        let mut expected = (SOUND_RECORD_EXTENT as u16).to_le_bytes().to_vec();
        expected.extend_from_slice(&SOUND_RECORD_PAYLOAD);
        assert_eq!(sound_record.as_deref(), Some(expected.as_slice()));
        assert!(active_entry.active_sound_record.is_none());
    }
}
