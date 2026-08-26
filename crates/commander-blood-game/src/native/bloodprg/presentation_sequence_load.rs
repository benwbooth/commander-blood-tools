//! Initial loading and bounded prefill of one presentation resource sequence.

use std::error::Error;
use std::fmt;

use super::{
    PresentationActiveEntryError, PresentationActiveEntryOutcome, PresentationActiveEntryState,
    PresentationEntryActivationRequest, PresentationEntryDisposition, PresentationEntryError,
    PresentationEntryPolicy, PresentationEntryPresenter, PresentationEntrySideData,
    PresentationEntryStorage, PresentationPaletteState, PresentationPresentPolicy,
    PresentationQueueClock, PresentationQueueLinkCursor, PresentationQueueRefillError,
    PresentationQueueRefillOutcome, PresentationQueueState, PresentationResourceDescriptor,
    PresentationResourceId, PresentationResourceProvider, PresentationResourceStreamState,
    PresentationResourceSwitchContext, PresentationResourceSwitchError,
    PresentationResourceSwitchOutcome, PresentationSourceError, activate_presentation_entry,
    load_initial_presentation_entry, present_active_entry, refill_presentation_queue,
    resolve_presentation_queue_link, switch_presentation_resource,
};

const INITIAL_REFILL_ATTEMPT_COUNT: usize = 50;
const SKIP_INITIAL_REFILL_FLAG: u8 = 64;

/// Host failure while publishing an initial sound or palette side record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationSequenceSideDataError {
    message: String,
}

impl PresentationSequenceSideDataError {
    /// Retain a host-facing side-record failure without a DOS error code.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PresentationSequenceSideDataError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PresentationSequenceSideDataError {}

/// Host boundary for sound and palette records parsed before the first frame.
pub trait PresentationSequenceSideDataSink {
    /// Apply side records before the activated frame reaches the presenter.
    fn apply_presentation_side_data(
        &mut self,
        side_data: &PresentationEntrySideData,
    ) -> Result<(), PresentationSequenceSideDataError>;
}

/// Mutable dependencies used to load and prefill one presentation sequence.
pub struct PresentationResourceSequenceContext<'a, Provider, Host> {
    /// Authored descriptors selected by presentation resource ID.
    pub descriptors: &'a [PresentationResourceDescriptor],
    /// Runtime variant copied into the selected descriptor flags.
    pub variant: u8,
    /// Queue state shared by initial loading and bounded refill.
    pub queue: &'a mut PresentationQueueState,
    /// Flat owned circular queue allocation.
    pub queue_buffer: &'a mut [u8],
    /// Indexed palette updated by bootstrap resource metadata.
    pub palette: &'a mut PresentationPaletteState,
    /// Palette snapshot suppression flags.
    pub render_update_flags: u8,
    /// Host resource provider.
    pub provider: &'a mut Provider,
    /// Parsing and decode gates for the initial queue entry.
    pub entry_policy: PresentationEntryPolicy,
    /// Owned active and retired frame state.
    pub active_entry: &'a mut PresentationActiveEntryState,
    /// Destination and row policies for first-frame presentation.
    pub present_policy: PresentationPresentPolicy,
    /// Concrete presenter and required sound/palette side-record publisher.
    pub host: &'a mut Host,
    /// Queue pacing state receiving the current timer sample.
    pub clock: &'a mut PresentationQueueClock,
    /// Current low-word game timer sample.
    pub timer_tick: u16,
}

/// Invalid stage encountered while loading the initial presentation sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationResourceSequenceError {
    /// Descriptor selection or bootstrap parsing failed.
    Switch(PresentationResourceSwitchError),
    /// The source did not contain a complete initial entry.
    InitialEntry(PresentationSourceError),
    /// The initial entry grammar or payload was malformed.
    Activation(PresentationEntryError),
    /// A sound or palette side record could not be published.
    SideData(PresentationSequenceSideDataError),
    /// Initial frame presentation failed.
    Present(PresentationActiveEntryError),
    /// A bounded prefill attempt found malformed queue or source state.
    Refill(PresentationQueueRefillError),
}

impl fmt::Display for PresentationResourceSequenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid presentation resource sequence: {self:?}"
        )
    }
}

impl Error for PresentationResourceSequenceError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Switch(source) => Some(source),
            Self::InitialEntry(source) => Some(source),
            Self::Activation(source) => Some(source),
            Self::SideData(source) => Some(source),
            Self::Present(source) => Some(source),
            Self::Refill(source) => Some(source),
        }
    }
}

impl From<PresentationResourceSwitchError> for PresentationResourceSequenceError {
    fn from(error: PresentationResourceSwitchError) -> Self {
        Self::Switch(error)
    }
}

impl From<PresentationSourceError> for PresentationResourceSequenceError {
    fn from(error: PresentationSourceError) -> Self {
        Self::InitialEntry(error)
    }
}

impl From<PresentationEntryError> for PresentationResourceSequenceError {
    fn from(error: PresentationEntryError) -> Self {
        Self::Activation(error)
    }
}

impl From<PresentationSequenceSideDataError> for PresentationResourceSequenceError {
    fn from(error: PresentationSequenceSideDataError) -> Self {
        Self::SideData(error)
    }
}

impl From<PresentationActiveEntryError> for PresentationResourceSequenceError {
    fn from(error: PresentationActiveEntryError) -> Self {
        Self::Present(error)
    }
}

impl From<PresentationQueueRefillError> for PresentationResourceSequenceError {
    fn from(error: PresentationQueueRefillError) -> Self {
        Self::Refill(error)
    }
}

/// Work completed by one successful initial sequence load.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationResourceSequenceOutcome {
    /// Bootstrap resource metadata selected by the switch.
    pub resource_switch: PresentationResourceSwitchOutcome,
    /// Complete extent of the first presented entry.
    pub initial_entry_extent: usize,
    /// Whether the first entry survived linked-generation validation.
    pub initial_entry_accepted: bool,
    /// Rendering work completed for the first entry.
    pub initial_present: PresentationActiveEntryOutcome,
    /// Number of bounded refill calls made after queue reset.
    pub refill_attempts: usize,
    /// Total source bytes appended by those refill calls.
    pub refill_transferred_bytes: usize,
    /// Retained queue-entry cursor after all synthetic links.
    pub link_cursor: PresentationQueueLinkCursor,
}

fn advance_initial_sequence_counters(queue: &mut PresentationQueueState) {
    queue.read_wrap_index = queue.read_wrap_index.wrapping_add(1);
    queue.sequence_index = queue.sequence_index.wrapping_add(1);
    queue.wrap_count = queue.wrap_count.wrapping_add(1);
}

/// Load, present, and prefill one streamed presentation resource.
///
/// This translates `resource_load_sequence` at BLOODPRG offset `0x00A15F`.
/// It composes the concrete flat resource switch, first-entry load and
/// activation, side-record publication, frame presentation, queue reset,
/// counter updates, and exactly 50 refill attempts unless flag 64 suppresses
/// prefill. The retained first-entry position replaces the native routine's
/// accidental reuse of a storage segment as its `mm` link cursor.
pub fn load_presentation_resource_sequence<Provider, Host>(
    stream: &mut PresentationResourceStreamState,
    resource: PresentationResourceId,
    context: &mut PresentationResourceSequenceContext<'_, Provider, Host>,
) -> Result<PresentationResourceSequenceOutcome, PresentationResourceSequenceError>
where
    Provider: PresentationResourceProvider,
    Host: PresentationEntryPresenter + PresentationSequenceSideDataSink,
{
    let resource_switch = switch_presentation_resource(
        stream,
        resource,
        &mut PresentationResourceSwitchContext {
            descriptors: context.descriptors,
            variant: context.variant,
            queue: context.queue,
            queue_capacity: context.queue_buffer.len(),
            palette: context.palette,
            render_update_flags: context.render_update_flags,
            provider: context.provider,
        },
    )?;
    let initial_entry = load_initial_presentation_entry(
        stream.source.as_mut(),
        context.queue,
        context.queue_buffer,
    )?;
    let mut link_cursor = PresentationQueueLinkCursor::new(context.queue.tail);
    let activation = activate_presentation_entry(
        context.queue_buffer,
        PresentationEntryActivationRequest {
            entry_extent: initial_entry.extent,
            payload_offset: initial_entry.payload_cursor,
            storage: PresentationEntryStorage::Default,
        },
        context.entry_policy,
        |link| resolve_presentation_queue_link(context.queue_buffer, link),
    )?;
    if activation.side_data.sound_record.is_some() || activation.side_data.palette_payload.is_some()
    {
        context
            .host
            .apply_presentation_side_data(&activation.side_data)?;
    }

    let initial_entry_accepted = match activation.disposition {
        PresentationEntryDisposition::Active(entry) => {
            context.active_entry.active = Some(entry);
            context.queue.active_entry = true;
            true
        }
        PresentationEntryDisposition::RejectedLink { .. } => {
            context.active_entry.active = None;
            context.queue.active_entry = false;
            false
        }
    };
    let initial_present =
        present_active_entry(context.active_entry, context.present_policy, context.host)?;
    context.queue.reset(context.queue_buffer.len());
    advance_initial_sequence_counters(context.queue);

    let mut refill_attempts = usize::MIN;
    let mut refill_transferred_bytes = usize::MIN;
    if stream.flags.to_le_bytes()[0] & SKIP_INITIAL_REFILL_FLAG == 0 {
        for _ in usize::MIN..INITIAL_REFILL_ATTEMPT_COUNT {
            let outcome = refill_presentation_queue(
                context.queue,
                context.queue_buffer,
                stream,
                context.descriptors,
                &mut link_cursor,
            )?;
            refill_attempts += 1;
            if let PresentationQueueRefillOutcome::Transferred { byte_count } = outcome {
                refill_transferred_bytes += byte_count;
            }
        }
    }
    context.clock.previous_tick = context.timer_tick;

    Ok(PresentationResourceSequenceOutcome {
        resource_switch,
        initial_entry_extent: initial_entry.extent,
        initial_entry_accepted,
        initial_present,
        refill_attempts,
        refill_transferred_bytes,
        link_cursor,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use crate::native::bloodprg::{
        OpenedPresentationResource, PresentationEntryRenderTarget, PresentationRectBlitOutcome,
        PresentationRectDecodeOutcome, PresentationResourceOpenError, PresentationSourceLease,
    };
    use commander_blood_formats::archive::BloodResourceName;

    const LOAD_VECTOR_COUNT: usize = 4;
    const QUEUE_BUFFER_BYTE_COUNT: usize = 65_536;
    const BOOTSTRAP_ENTRY_EXTENT: usize = 34;
    const FIRST_FRAME_EXTENT: usize = 6;
    const EMPTY_FRAME_LAYOUT: u16 = 1_024;

    #[derive(Deserialize)]
    struct LoadOracle {
        name: String,
        resource_id: u16,
        resource_flags: u16,
        refill_count: usize,
        result: LoadResult,
    }

    #[derive(Deserialize)]
    struct LoadResult {
        sequence: u16,
        previous_tick: u16,
    }

    struct VectorProvider {
        result: Option<Result<OpenedPresentationResource, PresentationResourceOpenError>>,
    }

    impl PresentationResourceProvider for VectorProvider {
        fn open_presentation_resource(
            &mut self,
            _descriptor: &PresentationResourceDescriptor,
        ) -> Result<OpenedPresentationResource, PresentationResourceOpenError> {
            self.result
                .take()
                .expect("each load vector opens one resource")
        }
    }

    #[derive(Default)]
    struct RecordingHost {
        side_data_calls: usize,
        presentation_calls: usize,
    }

    impl PresentationSequenceSideDataSink for RecordingHost {
        fn apply_presentation_side_data(
            &mut self,
            _side_data: &PresentationEntrySideData,
        ) -> Result<(), PresentationSequenceSideDataError> {
            self.side_data_calls += 1;
            Ok(())
        }
    }

    impl PresentationEntryPresenter for RecordingHost {
        fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError> {
            self.presentation_calls += 1;
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
            self.presentation_calls += 1;
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
            self.presentation_calls += 1;
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

    fn write_word(bytes: &mut [u8], position: usize, value: usize) {
        bytes[position..position + size_of::<u16>()].copy_from_slice(&(value as u16).to_le_bytes());
    }

    fn resource_bytes(include_initial_entry: bool, include_refill_entry: bool) -> Box<[u8]> {
        let mut bytes = vec![u8::MIN; BOOTSTRAP_ENTRY_EXTENT];
        write_word(&mut bytes, usize::MIN, BOOTSTRAP_ENTRY_EXTENT);
        bytes[2..4].fill(u8::MAX);
        if include_initial_entry {
            let start = bytes.len();
            bytes.resize(start + FIRST_FRAME_EXTENT, u8::MIN);
            write_word(&mut bytes, start, FIRST_FRAME_EXTENT);
            write_word(&mut bytes, start + 2, EMPTY_FRAME_LAYOUT as usize);
            if include_refill_entry {
                let refill_start = bytes.len();
                bytes.resize(refill_start + FIRST_FRAME_EXTENT, u8::MIN);
                write_word(&mut bytes, refill_start, FIRST_FRAME_EXTENT);
                write_word(&mut bytes, refill_start + 2, EMPTY_FRAME_LAYOUT as usize);
            }
        }
        bytes.into_boxed_slice()
    }

    fn descriptors(vector: &LoadOracle) -> Vec<PresentationResourceDescriptor> {
        vec![
            PresentationResourceDescriptor {
                flags: vector.resource_flags.to_le_bytes()[0],
                filename: BloodResourceName::new(b"RESOURCE.DAT").unwrap(),
                cached_range: None,
            };
            usize::from(vector.resource_id) + 1
        ]
    }

    #[test]
    fn sequence_loading_accounts_for_every_original_coordinator_vector() {
        let vectors: Vec<LoadOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_a15f_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), LOAD_VECTOR_COUNT);

        for vector in vectors {
            let provider_result = match vector.name.as_str() {
                "resource_switch_failure" => {
                    Err(PresentationResourceOpenError::new("oracle switch failure"))
                }
                "banked_list_load_failure" => Ok(OpenedPresentationResource::new(
                    resource_bytes(false, false),
                    usize::MIN,
                    PresentationSourceLease::Owned,
                )),
                "success_prefills_fifty" | "success_flag_40_skips_prefill" => {
                    Ok(OpenedPresentationResource::new(
                        resource_bytes(true, true),
                        usize::MIN,
                        PresentationSourceLease::Owned,
                    ))
                }
                name => panic!("unknown load oracle vector {name}"),
            };
            let mut provider = VectorProvider {
                result: Some(provider_result),
            };
            let descriptors = descriptors(&vector);
            let mut queue = PresentationQueueState {
                read_wrap_index: u16::MAX - 1,
                wrap_count: u16::MAX,
                sequence_index: vector.result.sequence.wrapping_sub(1),
                ..PresentationQueueState::default()
            };
            let mut queue_buffer = vec![u8::MIN; QUEUE_BUFFER_BYTE_COUNT];
            let mut palette = PresentationPaletteState::default();
            let mut active_entry = PresentationActiveEntryState::default();
            let mut host = RecordingHost::default();
            let mut clock = PresentationQueueClock {
                previous_tick: 4_369,
                ..PresentationQueueClock::default()
            };
            let mut stream = PresentationResourceStreamState::default();
            let mut context = PresentationResourceSequenceContext {
                descriptors: &descriptors,
                variant: vector.resource_flags.to_le_bytes()[1],
                queue: &mut queue,
                queue_buffer: &mut queue_buffer,
                palette: &mut palette,
                render_update_flags: u8::MIN,
                provider: &mut provider,
                entry_policy: PresentationEntryPolicy::default(),
                active_entry: &mut active_entry,
                present_policy: PresentationPresentPolicy {
                    skip_back_buffer_present: true,
                    ..PresentationPresentPolicy::default()
                },
                host: &mut host,
                clock: &mut clock,
                timer_tick: vector.result.previous_tick,
            };
            let result = load_presentation_resource_sequence(
                &mut stream,
                PresentationResourceId::new(vector.resource_id),
                &mut context,
            );

            match vector.name.as_str() {
                "resource_switch_failure" => {
                    assert!(matches!(
                        result,
                        Err(PresentationResourceSequenceError::Switch(_))
                    ));
                    assert_eq!(clock.previous_tick, 4_369);
                }
                "banked_list_load_failure" => {
                    assert!(matches!(
                        result,
                        Err(PresentationResourceSequenceError::InitialEntry(_))
                    ));
                    assert_eq!(clock.previous_tick, 4_369);
                }
                _ => {
                    let outcome = result.unwrap();
                    assert_eq!(outcome.initial_entry_extent, FIRST_FRAME_EXTENT);
                    assert!(outcome.initial_entry_accepted);
                    assert!(outcome.initial_present.frame_presented);
                    assert_eq!(outcome.refill_attempts, vector.refill_count);
                    assert_eq!(clock.previous_tick, vector.result.previous_tick);
                    assert_eq!(queue.sequence_index, vector.result.sequence);
                    assert_eq!(host.side_data_calls, usize::MIN);
                    assert_eq!(host.presentation_calls, usize::MIN);
                }
            }
        }
    }

    #[test]
    fn direct_counter_updates_retain_wrapping_arithmetic() {
        let mut queue = PresentationQueueState {
            read_wrap_index: u16::MAX - 1,
            sequence_index: 13_398,
            wrap_count: u16::MAX,
            ..PresentationQueueState::default()
        };
        advance_initial_sequence_counters(&mut queue);
        assert_eq!(queue.read_wrap_index, u16::MAX);
        assert_eq!(queue.sequence_index, 13_399);
        assert_eq!(queue.wrap_count, u16::MIN);
    }
}
