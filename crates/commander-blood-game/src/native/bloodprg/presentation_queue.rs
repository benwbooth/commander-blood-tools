//! Flat owned state for streamed presentation-resource queues.

use std::error::Error;
use std::fmt;

const ENTRY_HEADER_BYTE_COUNT: usize = 2;
const QUEUE_GUARD_BYTE_COUNT: usize = 18;
const QUEUE_ACCOUNTING_BYTE_COUNT: usize = 10;
const SOURCE_ROLLOVER_FLAG: u16 = 128;
const SOURCE_FINISHED_FLAG: u8 = 1;
const QUEUE_CLOSED_FLAG: u8 = 2;
const AUDIO_CLOCK_PERIOD: u16 = 16_384;
const AUDIO_ADVANCE_THRESHOLD: u16 = 920;

/// Checked state of the original circular presentation-resource queue.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationQueueState {
    /// Next owned-buffer position receiving source bytes.
    pub head: usize,
    /// Header position of the next queued entry.
    pub tail: usize,
    /// Number of source bytes currently retained by the queue.
    pub queued_bytes: usize,
    /// Payload bytes still expected for the entry being refilled.
    pub pending_entry_bytes: usize,
    /// Exclusive upper bound of the owned circular buffer.
    pub buffer_capacity: usize,
    /// Maximum queue occupancy accepted before the next circular wrap.
    pub wrap_limit: usize,
    /// Number of entries observed in the current source range.
    pub wrap_count: u16,
    /// One-based index of the entry currently being consumed.
    pub read_wrap_index: u16,
    /// Optional terminal entry index for the current range.
    pub read_wrap_limit: Option<u16>,
    /// Optional terminal entry index for the secondary range.
    pub secondary_wrap_limit: Option<u16>,
    /// Monotonic authored entry sequence number.
    pub sequence_index: u16,
    /// Native status bits retained because unrelated high bits are observable.
    pub status_bits: u8,
    /// Whether the queue currently owns a decoded entry.
    pub active_entry: bool,
    /// Source rollover state visible only during a refill call.
    pub rollover_latched: bool,
}

/// Invalid queue geometry that the flat port refuses to alias or wrap.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationQueueError {
    /// An entry cannot be shorter than its two-byte extent header.
    EntryExtentTooSmall {
        /// Supplied extent including the header.
        extent: usize,
    },
    /// A consumer requested more bytes than the queue owns.
    EntryExceedsQueuedBytes {
        /// Complete entry extent requested by the consumer.
        entry_bytes: usize,
        /// Bytes currently owned by the queue.
        queued_bytes: usize,
    },
    /// Appending bytes would leave the owned circular buffer.
    HeadOutsideBuffer {
        /// Current append position.
        head: usize,
        /// Requested append size.
        byte_count: usize,
        /// Exclusive owned-buffer bound.
        capacity: usize,
    },
    /// Queue byte accounting overflowed the host integer domain.
    ByteCountOverflow {
        /// Current queue occupancy.
        queued_bytes: usize,
        /// Requested occupancy increase.
        byte_count: usize,
    },
}

impl fmt::Display for PresentationQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation queue state: {self:?}")
    }
}

impl Error for PresentationQueueError {}

/// Result of consuming one complete queue entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationQueueConsumeOutcome {
    /// The tail moved from the end of the circular buffer to its start.
    pub wrapped: bool,
    /// Tail position selected for the next entry.
    pub next_tail: usize,
}

impl PresentationQueueState {
    /// Reset queue ownership and positions for one flat owned buffer.
    ///
    /// This translates `list_d8c_init` at BLOODPRG offset `0x00A757`.
    /// Ordinary buffer indices replace the two equal far-pointer segments.
    pub fn reset(&mut self, buffer_capacity: usize) {
        self.head = usize::MIN;
        self.tail = usize::MIN;
        self.queued_bytes = usize::MIN;
        self.pending_entry_bytes = usize::MIN;
        self.buffer_capacity = buffer_capacity;
        self.wrap_limit = buffer_capacity;
        self.active_entry = false;
    }

    /// Reset all primary and secondary source-range counters.
    ///
    /// This translates `list_d8c_bounds_init` at `0x00A73E`.
    pub fn initialize_bounds(&mut self) {
        self.read_wrap_index = u16::MIN;
        self.wrap_count = u16::MIN;
        self.read_wrap_limit = None;
        self.secondary_wrap_limit = None;
    }

    /// Reset source-range limits while retaining the current read index.
    ///
    /// This translates `list_d8c_wrap_bounds_reset` at `0x00A744`.
    pub fn reset_wrap_bounds(&mut self) {
        self.wrap_count = u16::MIN;
        self.read_wrap_limit = None;
        self.secondary_wrap_limit = None;
    }

    /// Begin receiving one extent-prefixed entry and update circular bounds.
    ///
    /// This translates `queue_d8c_wrap` at `0x00A38E`. The original offset
    /// overflow becomes an explicit end-of-buffer comparison.
    pub fn begin_entry(
        &mut self,
        entry_extent: usize,
        payload_cursor: usize,
    ) -> Result<bool, PresentationQueueError> {
        if entry_extent < ENTRY_HEADER_BYTE_COUNT {
            return Err(PresentationQueueError::EntryExtentTooSmall {
                extent: entry_extent,
            });
        }
        let wrapped = payload_cursor
            .checked_add(entry_extent)
            .is_none_or(|next| next > self.buffer_capacity);
        if wrapped {
            let previous_head = self.head;
            self.head = usize::MIN;
            self.wrap_limit = previous_head;
        }
        self.pending_entry_bytes = entry_extent - ENTRY_HEADER_BYTE_COUNT;
        self.wrap_count = self.wrap_count.wrapping_add(1);
        Ok(wrapped)
    }

    /// Return whether the queue can accept another byte range.
    ///
    /// This translates `queue_d8c_has_room` at `0x00A3AD`. Checked host
    /// arithmetic replaces the native false-positive cases caused by wrapping.
    pub fn has_room(&self, byte_count: usize) -> bool {
        if self.head < self.tail {
            let Some(gap_end) = self
                .head
                .checked_add(byte_count)
                .and_then(|value| value.checked_add(QUEUE_GUARD_BYTE_COUNT))
            else {
                return false;
            };
            if self.tail < gap_end {
                return false;
            }
        }

        self.queued_bytes
            .checked_add(QUEUE_ACCOUNTING_BYTE_COUNT)
            .and_then(|value| value.checked_add(byte_count))
            .is_some_and(|needed| needed <= self.wrap_limit)
    }

    /// Account for bytes appended to the owned queue buffer.
    ///
    /// This translates `queue_d8c_enqueue` at `0x00A734` without allowing
    /// 16-bit position or byte-count overflow.
    pub fn enqueue(&mut self, byte_count: usize) -> Result<(), PresentationQueueError> {
        let next_head =
            self.head
                .checked_add(byte_count)
                .ok_or(PresentationQueueError::HeadOutsideBuffer {
                    head: self.head,
                    byte_count,
                    capacity: self.buffer_capacity,
                })?;
        if next_head > self.buffer_capacity {
            return Err(PresentationQueueError::HeadOutsideBuffer {
                head: self.head,
                byte_count,
                capacity: self.buffer_capacity,
            });
        }
        let next_count = self.queued_bytes.checked_add(byte_count).ok_or(
            PresentationQueueError::ByteCountOverflow {
                queued_bytes: self.queued_bytes,
                byte_count,
            },
        )?;
        if next_count > self.buffer_capacity {
            return Err(PresentationQueueError::ByteCountOverflow {
                queued_bytes: self.queued_bytes,
                byte_count,
            });
        }
        self.head = next_head;
        self.queued_bytes = next_count;
        Ok(())
    }

    /// Retire one extent-prefixed entry and advance range counters.
    ///
    /// This translates `queue_d8c_consume` at `0x00A3D0`. Entry underflow is
    /// rejected transactionally; circular end crossing remains semantic.
    pub fn consume_entry(
        &mut self,
        entry_bytes: usize,
    ) -> Result<PresentationQueueConsumeOutcome, PresentationQueueError> {
        if entry_bytes < ENTRY_HEADER_BYTE_COUNT {
            return Err(PresentationQueueError::EntryExtentTooSmall {
                extent: entry_bytes,
            });
        }
        if entry_bytes > self.queued_bytes {
            return Err(PresentationQueueError::EntryExceedsQueuedBytes {
                entry_bytes,
                queued_bytes: self.queued_bytes,
            });
        }

        let after_header = self.tail.checked_add(ENTRY_HEADER_BYTE_COUNT);
        let candidate = after_header.and_then(|offset| offset.checked_add(entry_bytes));
        let wrapped = candidate.is_none_or(|offset| offset > self.buffer_capacity);
        let next_tail = if wrapped {
            entry_bytes - ENTRY_HEADER_BYTE_COUNT
        } else {
            self.tail + entry_bytes
        };
        let next_read_index = self.read_wrap_index.wrapping_add(1);
        let (read_wrap_index, read_wrap_limit) = match self.read_wrap_limit {
            Some(limit) if next_read_index > limit => (1, None),
            limit => (next_read_index, limit),
        };

        self.queued_bytes -= entry_bytes;
        self.tail = next_tail;
        self.sequence_index = self.sequence_index.wrapping_add(1);
        self.read_wrap_index = read_wrap_index;
        self.read_wrap_limit = read_wrap_limit;
        Ok(PresentationQueueConsumeOutcome { wrapped, next_tail })
    }

    /// Mark source completion and report whether the backing source can close.
    ///
    /// This translates `presentation_queue_finish` at `0x00A2DD` while leaving
    /// actual file ownership to the host resource store.
    pub fn finish_source(&mut self) -> bool {
        self.status_bits |= SOURCE_FINISHED_FLAG;
        if self.queued_bytes == usize::MIN {
            self.status_bits |= QUEUE_CLOSED_FLAG;
            true
        } else {
            false
        }
    }

    /// Return whether the native status byte is in state zero or one.
    ///
    /// This translates `list_d8c_state_le_one` at `0x00A40B`.
    pub const fn source_open_or_draining(&self) -> bool {
        self.status_bits <= SOURCE_FINISHED_FLAG
    }

    /// Run one refill with the source rollover flag exposed to the callback.
    ///
    /// This translates `list_d8c_refill_with_rollover_latch` at `0x00A1F3`.
    pub fn refill_with_rollover_latch<ResultValue>(
        &mut self,
        resource_flags: u16,
        link_target: usize,
        refill: impl FnOnce(&mut Self, usize) -> ResultValue,
    ) -> ResultValue {
        self.rollover_latched = resource_flags & SOURCE_ROLLOVER_FLAG != u16::MIN;
        let result = refill(self, link_target);
        self.rollover_latched = false;
        result
    }
}

/// Return the semantic low-bit result of `flag_test_b17` at `0x00A634`.
pub const fn presentation_resource_enabled(state: u8) -> bool {
    state & SOURCE_FINISHED_FLAG != u8::MIN
}

/// Clock state used to pace streamed presentation entries.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationQueueClock {
    /// Last normalized voice-playback phase accepted by the queue.
    pub audio_phase: u16,
    /// Last software timer sample accepted by the queue.
    pub previous_tick: u16,
    /// Minimum low-byte software tick delta.
    pub tick_threshold: u8,
}

/// Low-bit gates selecting voice-position pacing over software ticks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PresentationQueueClockGates {
    /// Primary presentation mode is active.
    pub primary_mode: bool,
    /// Secondary presentation mode is active.
    pub secondary_mode: bool,
    /// Voice playback is active.
    pub voice_playback: bool,
}

impl PresentationQueueClockGates {
    const fn uses_audio_clock(self) -> bool {
        self.primary_mode && self.secondary_mode && self.voice_playback
    }
}

/// Observable pacing decision for one queue update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationQueueAdvance {
    /// Whether the caller should activate the next entry.
    pub due: bool,
    /// Normalized elapsed phase or tick count used by the decision.
    pub elapsed: u16,
}

/// Decide whether enough audio phase or software time elapsed.
///
/// This translates `list_d8c_advance_due` at `0x00A240`. Closures retain the
/// native read ordering, including the second timer read after a due result.
pub fn presentation_queue_advance_due(
    clock: &mut PresentationQueueClock,
    gates: PresentationQueueClockGates,
    mut audio_position: impl FnMut() -> u16,
    mut timer_tick: impl FnMut() -> u16,
) -> PresentationQueueAdvance {
    if gates.uses_audio_clock() {
        let current = AUDIO_CLOCK_PERIOD.wrapping_sub(audio_position());
        let mut elapsed = current.wrapping_sub(clock.audio_phase);
        if (elapsed as i16).is_negative() {
            elapsed = elapsed.wrapping_add(AUDIO_CLOCK_PERIOD);
        }
        let due = elapsed >= AUDIO_ADVANCE_THRESHOLD;
        if due {
            clock.audio_phase = current;
        }
        return PresentationQueueAdvance { due, elapsed };
    }

    let current = timer_tick();
    let mut elapsed = current.wrapping_sub(clock.previous_tick);
    if (elapsed as i16).is_negative() {
        elapsed = u16::MIN.wrapping_sub(elapsed);
    }
    let due = elapsed >= u16::from(clock.tick_threshold);
    if due {
        clock.previous_tick = timer_tick();
    }
    PresentationQueueAdvance { due, elapsed }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use serde::Deserialize;

    use super::*;

    const FLAT_BUFFER_CAPACITY: usize = u16::MAX as usize;
    const ROLLOVER_VECTOR_COUNT: usize = 4;
    const CLOCK_VECTOR_COUNT: usize = 12;
    const FINISH_VECTOR_COUNT: usize = 6;
    const BEGIN_ENTRY_VECTOR_COUNT: usize = 6;
    const BEGIN_ENTRY_FLAT_VECTOR_COUNT: usize = 4;
    const ROOM_VECTOR_COUNT: usize = 8;
    const ROOM_FLAT_VECTOR_COUNT: usize = 5;
    const CONSUME_VECTOR_COUNT: usize = 8;
    const CONSUME_FLAT_VECTOR_COUNT: usize = 6;
    const ENQUEUE_VECTOR_COUNT: usize = 8;
    const ENQUEUE_FLAT_VECTOR_COUNT: usize = 2;
    const RESET_VECTOR_COUNT: usize = 5;
    const BOUNDS_VECTOR_COUNT: usize = 20;

    #[derive(Deserialize)]
    struct RolloverOracle {
        name: String,
        resource_flags: u16,
        link_target_offset: u16,
        latch_during_call: u8,
        latch_after_return: u8,
    }

    #[derive(Deserialize)]
    struct ClockOracle {
        name: String,
        audio_clock: bool,
        due: bool,
        mode_27e0: u8,
        mode_27e1: u8,
        audio_enabled: u8,
        callback_value: u16,
        previous_phase: u16,
        tick: u16,
        previous_tick: u16,
        threshold: u8,
        reread_tick: u16,
        normalized_delta: u16,
    }

    #[derive(Deserialize)]
    struct FinishOracle {
        name: String,
        initial_state: u8,
        byte_count: u16,
        result_state: u8,
        close_called: bool,
    }

    #[derive(Deserialize)]
    struct BeginEntryOracle {
        name: String,
        cursor: u16,
        byte_count: u16,
        buffer_end: u16,
        head: u16,
        wrap_limit: u16,
        wrap_count: u16,
        wrapped: bool,
        result_head: u16,
        result_wrap_limit: u16,
        result_iteration_count: u16,
        result_wrap_count: u16,
    }

    #[derive(Deserialize)]
    struct RoomOracle {
        name: String,
        head: u16,
        tail: u16,
        byte_count: u16,
        wrap_limit: u16,
        request: u16,
        has_room: bool,
    }

    #[derive(Deserialize)]
    struct ConsumeOracle {
        name: String,
        tail: u16,
        entry_bytes: u16,
        byte_count: u16,
        buffer_end: u16,
        sequence: u16,
        read_index: u16,
        read_limit: u16,
        wrapped: bool,
        result_tail: u16,
        result_byte_count: u16,
        result_sequence: u16,
        result_read_index: u16,
        result_read_limit: u16,
    }

    #[derive(Deserialize)]
    struct EnqueueOracle {
        name: String,
        head: u16,
        byte_count: u16,
        increment: u16,
        result_head: u16,
        result_byte_count: u16,
    }

    #[derive(Deserialize)]
    struct ResetOracle {
        name: String,
        buffer_end: u16,
        result_wrap_limit: u16,
    }

    #[test]
    fn refill_rollover_latch_matches_every_original_vector() {
        let vectors: Vec<RolloverOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a1f3_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ROLLOVER_VECTOR_COUNT);

        for vector in vectors {
            let mut state = PresentationQueueState::default();
            let result = state.refill_with_rollover_latch(
                vector.resource_flags,
                usize::from(vector.link_target_offset),
                |state, link_target| {
                    assert_eq!(
                        u8::from(state.rollover_latched) * SOURCE_ROLLOVER_FLAG as u8,
                        vector.latch_during_call,
                        "{}",
                        vector.name
                    );
                    link_target
                },
            );
            assert_eq!(
                result,
                usize::from(vector.link_target_offset),
                "{}",
                vector.name
            );
            assert_eq!(
                u8::from(state.rollover_latched),
                vector.latch_after_return,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn advance_timing_matches_every_original_vector() {
        let vectors: Vec<ClockOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a240_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), CLOCK_VECTOR_COUNT);

        for vector in vectors {
            let mut clock = PresentationQueueClock {
                audio_phase: vector.previous_phase,
                previous_tick: vector.previous_tick,
                tick_threshold: vector.threshold,
            };
            let mut audio_samples = VecDeque::from([vector.callback_value]);
            let mut timer_samples = VecDeque::from([vector.tick, vector.reread_tick]);
            let result = presentation_queue_advance_due(
                &mut clock,
                PresentationQueueClockGates {
                    primary_mode: vector.mode_27e0 & 1 != u8::MIN,
                    secondary_mode: vector.mode_27e1 & 1 != u8::MIN,
                    voice_playback: vector.audio_enabled & 1 != u8::MIN,
                },
                || audio_samples.pop_front().unwrap(),
                || timer_samples.pop_front().unwrap(),
            );

            assert_eq!(
                vector.audio_clock,
                vector.mode_27e0 & 1 != 0
                    && vector.mode_27e1 & 1 != 0
                    && vector.audio_enabled & 1 != 0,
                "{}",
                vector.name
            );
            assert_eq!(result.due, vector.due, "{}", vector.name);
            assert_eq!(result.elapsed, vector.normalized_delta, "{}", vector.name);
            if vector.audio_clock {
                let expected_phase = if vector.due {
                    AUDIO_CLOCK_PERIOD.wrapping_sub(vector.callback_value)
                } else {
                    vector.previous_phase
                };
                assert_eq!(clock.audio_phase, expected_phase, "{}", vector.name);
                assert_eq!(clock.previous_tick, vector.previous_tick, "{}", vector.name);
                assert!(audio_samples.is_empty(), "{}", vector.name);
                assert_eq!(timer_samples.len(), 2, "{}", vector.name);
            } else {
                let expected_tick = if vector.due {
                    vector.reread_tick
                } else {
                    vector.previous_tick
                };
                assert_eq!(clock.previous_tick, expected_tick, "{}", vector.name);
                assert_eq!(clock.audio_phase, vector.previous_phase, "{}", vector.name);
                assert_eq!(audio_samples.len(), 1, "{}", vector.name);
                assert_eq!(
                    timer_samples.len(),
                    usize::from(!vector.due),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn source_finish_matches_every_original_vector() {
        let vectors: Vec<FinishOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a2dd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), FINISH_VECTOR_COUNT);

        for vector in vectors {
            let mut state = PresentationQueueState {
                queued_bytes: usize::from(vector.byte_count),
                status_bits: vector.initial_state,
                ..PresentationQueueState::default()
            };
            assert_eq!(
                state.finish_source(),
                vector.close_called,
                "{}",
                vector.name
            );
            assert_eq!(state.status_bits, vector.result_state, "{}", vector.name);
        }
    }

    #[test]
    fn entry_begin_matches_flat_original_vectors() {
        let vectors: Vec<BeginEntryOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a38e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), BEGIN_ENTRY_VECTOR_COUNT);
        let mut exact = usize::MIN;

        for vector in vectors {
            let mut state = PresentationQueueState {
                head: usize::from(vector.head),
                buffer_capacity: usize::from(vector.buffer_end),
                wrap_limit: usize::from(vector.wrap_limit),
                wrap_count: vector.wrap_count,
                ..PresentationQueueState::default()
            };
            let before = state.clone();
            let result =
                state.begin_entry(usize::from(vector.byte_count), usize::from(vector.cursor));
            if vector.byte_count >= ENTRY_HEADER_BYTE_COUNT as u16 {
                assert_eq!(result.unwrap(), vector.wrapped, "{}", vector.name);
                assert_eq!(
                    state.head,
                    usize::from(vector.result_head),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.wrap_limit,
                    usize::from(vector.result_wrap_limit),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.pending_entry_bytes,
                    usize::from(vector.result_iteration_count),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.wrap_count, vector.result_wrap_count,
                    "{}",
                    vector.name
                );
                exact += 1;
            } else {
                assert!(matches!(
                    result,
                    Err(PresentationQueueError::EntryExtentTooSmall { .. })
                ));
                assert_eq!(state, before, "{}", vector.name);
            }
        }
        assert_eq!(exact, BEGIN_ENTRY_FLAT_VECTOR_COUNT);
    }

    #[test]
    fn room_check_matches_flat_original_vectors() {
        let vectors: Vec<RoomOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a3ad_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ROOM_VECTOR_COUNT);
        let mut exact = usize::MIN;

        for vector in vectors {
            let state = PresentationQueueState {
                head: usize::from(vector.head),
                tail: usize::from(vector.tail),
                queued_bytes: usize::from(vector.byte_count),
                wrap_limit: usize::from(vector.wrap_limit),
                ..PresentationQueueState::default()
            };
            let result = state.has_room(usize::from(vector.request));
            if matches!(
                vector.name.as_str(),
                "count_plus_ten_carry_is_discarded"
                    | "head_plus_request_carry_is_discarded"
                    | "head_plus_padding_carry_is_discarded"
            ) {
                assert!(!result, "{}", vector.name);
                assert!(vector.has_room, "{}", vector.name);
            } else {
                assert_eq!(result, vector.has_room, "{}", vector.name);
                exact += 1;
            }
        }
        assert_eq!(exact, ROOM_FLAT_VECTOR_COUNT);
    }

    #[test]
    fn consume_matches_flat_original_vectors() {
        let vectors: Vec<ConsumeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a3d0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), CONSUME_VECTOR_COUNT);
        let mut exact = usize::MIN;

        for vector in vectors {
            let mut state = PresentationQueueState {
                tail: usize::from(vector.tail),
                queued_bytes: usize::from(vector.byte_count),
                buffer_capacity: usize::from(vector.buffer_end),
                sequence_index: vector.sequence,
                read_wrap_index: vector.read_index,
                read_wrap_limit: (vector.read_limit != u16::MAX).then_some(vector.read_limit),
                ..PresentationQueueState::default()
            };
            let before = state.clone();
            let result = state.consume_entry(usize::from(vector.entry_bytes));
            if vector.entry_bytes < ENTRY_HEADER_BYTE_COUNT as u16
                || vector.entry_bytes > vector.byte_count
            {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(state, before, "{}", vector.name);
                continue;
            }

            let outcome = result.unwrap();
            assert_eq!(outcome.wrapped, vector.wrapped, "{}", vector.name);
            assert_eq!(
                state.tail,
                usize::from(vector.result_tail),
                "{}",
                vector.name
            );
            assert_eq!(
                state.queued_bytes,
                usize::from(vector.result_byte_count),
                "{}",
                vector.name
            );
            assert_eq!(
                state.sequence_index, vector.result_sequence,
                "{}",
                vector.name
            );
            assert_eq!(
                state.read_wrap_index, vector.result_read_index,
                "{}",
                vector.name
            );
            assert_eq!(
                state.read_wrap_limit.unwrap_or(u16::MAX),
                vector.result_read_limit,
                "{}",
                vector.name
            );
            exact += 1;
        }
        assert_eq!(exact, CONSUME_FLAT_VECTOR_COUNT);
    }

    #[test]
    fn low_bit_and_status_queries_cover_every_byte_value() {
        let status_summaries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a40b_natural.json"
        ))
        .unwrap();
        let flag_summaries: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a634_natural.json"
        ))
        .unwrap();
        let status_summary = &status_summaries[usize::MIN];
        let flag_summary = &flag_summaries[usize::MIN];
        assert_eq!(status_summary["tested_state_count"], 256);
        assert_eq!(flag_summary["tested_state_count"], 256);

        for value in u8::MIN..=u8::MAX {
            let state = PresentationQueueState {
                status_bits: value,
                ..PresentationQueueState::default()
            };
            assert_eq!(state.source_open_or_draining(), value <= 1);
            assert_eq!(presentation_resource_enabled(value), value & 1 != 0);
        }
    }

    #[test]
    fn enqueue_rejects_native_counter_wrap_transactionally() {
        let vectors: Vec<EnqueueOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a734_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ENQUEUE_VECTOR_COUNT);
        let mut exact = usize::MIN;

        for vector in vectors {
            let mut state = PresentationQueueState {
                head: usize::from(vector.head),
                queued_bytes: usize::from(vector.byte_count),
                buffer_capacity: FLAT_BUFFER_CAPACITY,
                ..PresentationQueueState::default()
            };
            let before = state.clone();
            let result = state.enqueue(usize::from(vector.increment));
            if matches!(vector.name.as_str(), "zero" | "ordinary") {
                result.unwrap();
                assert_eq!(
                    state.head,
                    usize::from(vector.result_head),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.queued_bytes,
                    usize::from(vector.result_byte_count),
                    "{}",
                    vector.name
                );
                exact += 1;
            } else {
                assert!(result.is_err(), "{}", vector.name);
                assert_eq!(state, before, "{}", vector.name);
            }
        }
        assert_eq!(exact, ENQUEUE_FLAT_VECTOR_COUNT);
    }

    #[test]
    fn bounds_initializers_replace_native_sentinel_words() {
        let initialize_vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a73e.json"
        ))
        .unwrap();
        let reset_vectors: Vec<serde_json::Value> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a744.json"
        ))
        .unwrap();
        assert_eq!(initialize_vectors.len(), BOUNDS_VECTOR_COUNT);
        assert_eq!(reset_vectors.len(), BOUNDS_VECTOR_COUNT);

        let mut state = PresentationQueueState {
            read_wrap_index: 17,
            wrap_count: 19,
            read_wrap_limit: Some(23),
            secondary_wrap_limit: Some(29),
            ..PresentationQueueState::default()
        };
        state.initialize_bounds();
        assert_eq!(state.read_wrap_index, u16::MIN);
        assert_eq!(state.wrap_count, u16::MIN);
        assert_eq!(state.read_wrap_limit, None);
        assert_eq!(state.secondary_wrap_limit, None);

        state.read_wrap_index = 31;
        state.wrap_count = 37;
        state.read_wrap_limit = Some(41);
        state.secondary_wrap_limit = Some(43);
        state.reset_wrap_bounds();
        assert_eq!(state.read_wrap_index, 31);
        assert_eq!(state.wrap_count, u16::MIN);
        assert_eq!(state.read_wrap_limit, None);
        assert_eq!(state.secondary_wrap_limit, None);
    }

    #[test]
    fn queue_reset_matches_every_flat_original_vector() {
        let vectors: Vec<ResetOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a757_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), RESET_VECTOR_COUNT);

        for vector in vectors {
            let mut state = PresentationQueueState {
                head: 2,
                tail: 3,
                queued_bytes: 5,
                pending_entry_bytes: 7,
                active_entry: true,
                read_wrap_index: 11,
                status_bits: 13,
                ..PresentationQueueState::default()
            };
            state.reset(usize::from(vector.buffer_end));
            assert_eq!(state.head, usize::MIN, "{}", vector.name);
            assert_eq!(state.tail, usize::MIN, "{}", vector.name);
            assert_eq!(state.queued_bytes, usize::MIN, "{}", vector.name);
            assert_eq!(state.pending_entry_bytes, usize::MIN, "{}", vector.name);
            assert!(!state.active_entry, "{}", vector.name);
            assert_eq!(
                state.wrap_limit,
                usize::from(vector.result_wrap_limit),
                "{}",
                vector.name
            );
            assert_eq!(state.read_wrap_index, 11, "{}", vector.name);
            assert_eq!(state.status_bits, 13, "{}", vector.name);
        }
    }
}
