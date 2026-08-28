//! Selection and bootstrap parsing for streamed presentation resources.

use std::error::Error;
use std::fmt;

use commander_blood_formats::archive::BloodResourceName;

use crate::assets::{OriginalResourceSource, OriginalResourceStore};

use super::{
    PresentationByteSource, PresentationPaletteError, PresentationPaletteOutcome,
    PresentationPaletteState, PresentationQueueState, PresentationResourceId,
    PresentationSourceError, PresentationSourceLease, apply_presentation_palette_blocks,
    close_owned_presentation_source, presentation_resource_descriptor,
};

const ENTRY_HEADER_BYTE_COUNT: usize = size_of::<u16>();
const RANGE_OFFSET_BYTE_COUNT: usize = size_of::<u32>();
const ALTERNATE_RANGE_TABLE_OFFSET: usize = 16;
const RANGE_TABLE_STRIDE: usize = size_of::<u32>();
const ALTERNATE_RANGE_FLAG: u8 = 4;
const METADATA_PADDING_BYTE: u8 = u8::MAX;

/// Authored descriptor selected by a presentation line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationResourceDescriptor {
    /// Low-byte stream behavior flags.
    pub flags: u8,
    /// Exact original resource filename.
    pub filename: BloodResourceName,
    /// Optional previously discovered range used by rollover recovery.
    pub cached_range: Option<PresentationSourceRange>,
}

/// One validated range within a flat presentation source.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationSourceRange {
    /// Byte position within the owned source allocation.
    pub position: usize,
    /// Bytes remaining from that position.
    pub remaining: usize,
}

/// Owned resource bytes returned by a modern resource provider.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenedPresentationResource {
    /// Complete bytes visible to the streamed queue.
    pub source: PresentationByteSource,
    /// Absolute source offset corresponding to flat byte position zero.
    pub absolute_origin: usize,
    /// Ownership used when switching or closing resources.
    pub lease: PresentationSourceLease,
}

impl OpenedPresentationResource {
    /// Construct an owned presentation stream at its first byte.
    pub fn new(
        bytes: impl Into<Box<[u8]>>,
        absolute_origin: usize,
        lease: PresentationSourceLease,
    ) -> Self {
        Self {
            source: PresentationByteSource::new(bytes),
            absolute_origin,
            lease,
        }
    }
}

/// Host failure while resolving one presentation descriptor.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentationResourceOpenError {
    message: String,
}

impl PresentationResourceOpenError {
    /// Retain a host-facing resource-open failure without a DOS error code.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for PresentationResourceOpenError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for PresentationResourceOpenError {}

/// Provider used to resolve presentation descriptors into owned bytes.
pub trait PresentationResourceProvider {
    /// Open one authored presentation resource.
    fn open_presentation_resource(
        &mut self,
        descriptor: &PresentationResourceDescriptor,
    ) -> Result<OpenedPresentationResource, PresentationResourceOpenError>;
}

impl PresentationResourceProvider for OriginalResourceStore {
    fn open_presentation_resource(
        &mut self,
        descriptor: &PresentationResourceDescriptor,
    ) -> Result<OpenedPresentationResource, PresentationResourceOpenError> {
        let lease = match self.source(&descriptor.filename) {
            OriginalResourceSource::EmbeddedArchive => PresentationSourceLease::SharedArchive,
            OriginalResourceSource::LooseFile => PresentationSourceLease::Owned,
        };
        let bytes = self
            .load(&descriptor.filename)
            .map_err(|error| PresentationResourceOpenError::new(error.to_string()))?;
        Ok(OpenedPresentationResource::new(bytes, usize::MIN, lease))
    }
}

/// Persistent stream state selected by the latest resource switch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PresentationResourceStreamState {
    /// Most recently requested resource.
    pub requested: Option<PresentationResourceId>,
    /// Descriptor currently supplying bytes.
    pub active: Option<PresentationResourceId>,
    /// Descriptor flags in the low byte and selected variant in the high byte.
    pub flags: u16,
    /// Whether bootstrap palette and range metadata are valid.
    pub ready: bool,
    /// Metric derived from the bootstrap palette stream.
    pub entry_metric: usize,
    /// Primary frame-data range selected for refill.
    pub range: Option<PresentationSourceRange>,
    /// Secondary index-data range selected by the entry metric.
    pub index_range: Option<PresentationSourceRange>,
    /// Absolute origin represented by source byte position zero.
    pub absolute_origin: usize,
    /// Current source ownership class.
    pub lease: PresentationSourceLease,
    /// Complete active source and its current read position.
    pub source: Option<PresentationByteSource>,
    /// Exclusive end of the currently selected range in the owned source.
    pub(crate) active_range_end: Option<usize>,
}

impl PresentationResourceStreamState {
    /// Return the current source position in the provider's absolute domain.
    pub fn absolute_source_offset(&self) -> Option<usize> {
        self.source
            .as_ref()
            .and_then(|source| self.absolute_origin.checked_add(source.position()))
    }

    /// Return bytes remaining in the active owned source.
    pub fn source_remaining(&self) -> Option<usize> {
        let source = self.source.as_ref()?;
        Some(
            self.active_range_end
                .unwrap_or(source.bytes().len())
                .saturating_sub(source.position()),
        )
    }

    /// Select one validated range as the source consumed by queue refill.
    pub(crate) fn select_range(
        &mut self,
        range: PresentationSourceRange,
    ) -> Result<(), PresentationSourceError> {
        let source = self
            .source
            .as_mut()
            .ok_or(PresentationSourceError::SourceUnavailable)?;
        let source_len = source.bytes().len();
        let range_end = range.position.checked_add(range.remaining).ok_or(
            PresentationSourceError::SourceRangeOutOfBounds {
                position: range.position,
                remaining: range.remaining,
                source_len,
            },
        )?;
        if range_end > source_len {
            return Err(PresentationSourceError::SourceRangeOutOfBounds {
                position: range.position,
                remaining: range.remaining,
                source_len,
            });
        }
        source.seek(range.position)?;
        self.active_range_end = Some(range_end);
        Ok(())
    }

    /// Release a source owned only by this stream and reset queue range bounds.
    pub(crate) fn close_owned_source(&mut self, queue: &mut PresentationQueueState) -> bool {
        let closed = close_owned_presentation_source(&mut self.lease, queue);
        if closed {
            self.source = None;
            self.active_range_end = None;
            self.ready = false;
        }
        closed
    }
}

/// Mutable dependencies used while switching a presentation stream.
pub struct PresentationResourceSwitchContext<'a, Provider> {
    /// Authored descriptor table addressed by presentation resource ID.
    pub descriptors: &'a [PresentationResourceDescriptor],
    /// Runtime variant copied into the descriptor flag word.
    pub variant: u8,
    /// Queue reset before opening the new stream.
    pub queue: &'a mut PresentationQueueState,
    /// Capacity of the owned queue allocation.
    pub queue_capacity: usize,
    /// Indexed palette updated by bootstrap palette blocks.
    pub palette: &'a mut PresentationPaletteState,
    /// Palette snapshot suppression flags.
    pub render_update_flags: u8,
    /// Host resource provider.
    pub provider: &'a mut Provider,
}

/// Successful bootstrap values retained for diagnostics and tests.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PresentationResourceSwitchOutcome {
    /// Complete initial serialized entry extent.
    pub entry_extent: usize,
    /// First byte of selected range metadata in the owned source.
    pub metadata_position: usize,
    /// Bootstrap palette work completed before range selection.
    pub palette: PresentationPaletteOutcome,
}

/// Invalid descriptor, source, palette, or range metadata.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PresentationResourceSwitchError {
    /// The requested resource has no authored descriptor.
    DescriptorUnavailable {
        /// Requested presentation resource.
        resource: PresentationResourceId,
    },
    /// The host could not open the descriptor's bytes.
    Open(PresentationResourceOpenError),
    /// Initial extent transport failed.
    Source(PresentationSourceError),
    /// Bootstrap palette blocks were malformed.
    Palette(PresentationPaletteError),
    /// The metadata stream consists only of padding bytes.
    MetadataMissing {
        /// First searched source position.
        position: usize,
    },
    /// A four-byte range offset lies outside the owned source.
    MetadataTruncated {
        /// Position of the missing range field.
        position: usize,
        /// Available source bytes.
        source_len: usize,
    },
    /// A metadata-relative range begins beyond the unread source.
    RelativeRangeOutOfBounds {
        /// Authored relative byte offset.
        relative: usize,
        /// Bytes remaining after the bootstrap entry.
        remaining: usize,
    },
    /// Absolute source-offset arithmetic overflowed the host domain.
    AbsoluteOffsetOverflow {
        /// Base absolute offset.
        base: usize,
        /// Relative source position.
        relative: usize,
    },
}

impl fmt::Display for PresentationResourceSwitchError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid presentation resource switch: {self:?}")
    }
}

impl Error for PresentationResourceSwitchError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Open(source) => Some(source),
            Self::Source(source) => Some(source),
            Self::Palette(source) => Some(source),
            _ => None,
        }
    }
}

impl From<PresentationResourceOpenError> for PresentationResourceSwitchError {
    fn from(error: PresentationResourceOpenError) -> Self {
        Self::Open(error)
    }
}

impl From<PresentationSourceError> for PresentationResourceSwitchError {
    fn from(error: PresentationSourceError) -> Self {
        Self::Source(error)
    }
}

impl From<PresentationPaletteError> for PresentationResourceSwitchError {
    fn from(error: PresentationPaletteError) -> Self {
        Self::Palette(error)
    }
}

fn read_range_offset(
    source: &[u8],
    position: usize,
) -> Result<usize, PresentationResourceSwitchError> {
    let end = position.checked_add(RANGE_OFFSET_BYTE_COUNT).ok_or(
        PresentationResourceSwitchError::MetadataTruncated {
            position,
            source_len: source.len(),
        },
    )?;
    let bytes =
        source
            .get(position..end)
            .ok_or(PresentationResourceSwitchError::MetadataTruncated {
                position,
                source_len: source.len(),
            })?;
    Ok(u32::from_le_bytes(bytes.try_into().expect("validated four-byte range offset")) as usize)
}

fn source_range(
    source_position: usize,
    source_remaining: usize,
    relative: usize,
) -> Result<PresentationSourceRange, PresentationResourceSwitchError> {
    if relative > source_remaining {
        return Err(PresentationResourceSwitchError::RelativeRangeOutOfBounds {
            relative,
            remaining: source_remaining,
        });
    }
    let position = source_position.checked_add(relative).ok_or(
        PresentationResourceSwitchError::AbsoluteOffsetOverflow {
            base: source_position,
            relative,
        },
    )?;
    Ok(PresentationSourceRange {
        position,
        remaining: source_remaining - relative,
    })
}

/// Select a resource, parse its bootstrap palette, and publish refill ranges.
///
/// This translates `resource_switch` at BLOODPRG offset `0x009F8E`. The
/// original EMS/XMS/DOS source selection becomes one owned provider result;
/// queue reset, variant flags, palette parsing, padding skip, and both relative
/// range calculations remain explicit game logic over checked flat positions.
pub fn switch_presentation_resource<Provider: PresentationResourceProvider>(
    state: &mut PresentationResourceStreamState,
    resource: PresentationResourceId,
    context: &mut PresentationResourceSwitchContext<'_, Provider>,
) -> Result<PresentationResourceSwitchOutcome, PresentationResourceSwitchError> {
    state.requested = Some(resource);
    state.active = None;
    state.flags = u16::MIN;
    state.ready = false;
    state.entry_metric = usize::MIN;
    state.range = None;
    state.index_range = None;
    state.source = None;
    state.active_range_end = None;
    state.lease = PresentationSourceLease::Closed;
    context.queue.reset(context.queue_capacity);
    context.queue.status_bits = u8::MIN;
    context.queue.initialize_bounds();

    let descriptor = presentation_resource_descriptor(context.descriptors, resource.get())
        .ok_or(PresentationResourceSwitchError::DescriptorUnavailable { resource })?;
    state.active = Some(resource);
    state.flags = u16::from_le_bytes([descriptor.flags, context.variant]);

    let opened = context.provider.open_presentation_resource(descriptor)?;
    state.absolute_origin = opened.absolute_origin;
    state.lease = opened.lease;
    state.source = Some(opened.source);
    state.active_range_end = state.source.as_ref().map(|source| source.bytes().len());
    let source = state
        .source
        .as_mut()
        .expect("opened source was retained before bootstrap parsing");

    let entry_start = source.position();
    let header_end = entry_start.checked_add(ENTRY_HEADER_BYTE_COUNT).ok_or(
        PresentationSourceError::SourceTruncated {
            position: entry_start,
            requested: ENTRY_HEADER_BYTE_COUNT,
            remaining: source.remaining(),
        },
    )?;
    let header = source.bytes().get(entry_start..header_end).ok_or(
        PresentationSourceError::SourceTruncated {
            position: entry_start,
            requested: ENTRY_HEADER_BYTE_COUNT,
            remaining: source.remaining(),
        },
    )?;
    let entry_extent = usize::from(u16::from_le_bytes(
        header
            .try_into()
            .expect("validated two-byte bootstrap extent"),
    ));
    source.seek(header_end)?;
    state.entry_metric = entry_extent;
    if entry_extent < ENTRY_HEADER_BYTE_COUNT {
        return Err(PresentationSourceError::EntryExtentTooSmall {
            extent: entry_extent,
        }
        .into());
    }
    let body_byte_count = entry_extent - ENTRY_HEADER_BYTE_COUNT;
    let entry_end = header_end.checked_add(body_byte_count).ok_or(
        PresentationSourceError::SourceTruncated {
            position: header_end,
            requested: body_byte_count,
            remaining: source.remaining(),
        },
    )?;
    if entry_end > source.bytes().len() {
        return Err(PresentationSourceError::SourceTruncated {
            position: header_end,
            requested: body_byte_count,
            remaining: source.remaining(),
        }
        .into());
    }
    source.seek(entry_end)?;

    let mut staged_palette = context.palette.clone();
    let palette_outcome = apply_presentation_palette_blocks(
        &source.bytes()[header_end..],
        &mut staged_palette,
        context.render_update_flags,
        u16::MIN,
        &mut state.entry_metric,
    )?;
    let padding_start = header_end + palette_outcome.consumed_bytes;
    let metadata_position = source.bytes()[padding_start..]
        .iter()
        .position(|byte| *byte != METADATA_PADDING_BYTE)
        .map(|relative| padding_start + relative)
        .ok_or(PresentationResourceSwitchError::MetadataMissing {
            position: padding_start,
        })?;

    let range_table_position = if descriptor.flags & ALTERNATE_RANGE_FLAG == u8::MIN {
        metadata_position
    } else {
        metadata_position + ALTERNATE_RANGE_TABLE_OFFSET
    };
    let range_relative = read_range_offset(source.bytes(), range_table_position)?;
    let index_position = metadata_position
        .checked_add(state.entry_metric * RANGE_TABLE_STRIDE)
        .ok_or(PresentationResourceSwitchError::MetadataTruncated {
            position: metadata_position,
            source_len: source.bytes().len(),
        })?;
    let index_relative = read_range_offset(source.bytes(), index_position)?;
    let source_position = source.position();
    let source_remaining = source.remaining();
    state.range = Some(source_range(
        source_position,
        source_remaining,
        range_relative,
    )?);
    state.index_range = Some(source_range(
        source_position,
        source_remaining,
        index_relative,
    )?);
    state.ready = true;
    *context.palette = staged_palette;
    Ok(PresentationResourceSwitchOutcome {
        entry_extent,
        metadata_position,
        palette: palette_outcome,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const SWITCH_VECTOR_COUNT: usize = 7;
    const FINAL_ENTRY_METRIC: usize = 6;
    const BOOTSTRAP_TRAILER_BYTE_COUNT: usize = 32;
    const PRIMARY_RANGE_TABLE_OFFSET: usize = 0;

    #[derive(Deserialize)]
    struct SwitchOracle {
        name: String,
        mode: String,
        resource_id: u16,
        record_flags: u16,
        success: bool,
        calls: Vec<SwitchCall>,
        source_offset: usize,
        source_remaining: usize,
        entry_metric: usize,
        metadata_offset: usize,
        range_start: usize,
        range_remaining: usize,
        index_start: usize,
        index_remaining: usize,
    }

    #[derive(Deserialize)]
    struct SwitchCall {
        call: String,
        bytes: Option<usize>,
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
                .expect("each switch vector opens at most one resource")
        }
    }

    fn write_dword(bytes: &mut [u8], position: usize, value: usize) {
        bytes[position..position + RANGE_OFFSET_BYTE_COUNT]
            .copy_from_slice(&(value as u32).to_le_bytes());
    }

    fn palette_stream(consumed_bytes: usize) -> Vec<u8> {
        match consumed_bytes {
            2 => vec![u8::MAX, u8::MAX],
            7 => vec![3, 1, 1, 2, 3, u8::MAX, u8::MAX],
            12 => vec![1, 2, 1, 2, 3, 4, 5, 6, 9, 0, u8::MAX, u8::MAX],
            count => panic!("unsupported oracle palette extent {count}"),
        }
    }

    fn opened_resource(vector: &SwitchOracle) -> OpenedPresentationResource {
        if vector.name == "initial_read_failure" {
            return OpenedPresentationResource::new(
                Box::<[u8]>::default(),
                usize::MIN,
                PresentationSourceLease::SharedArchive,
            );
        }
        let body_read = vector
            .calls
            .iter()
            .find(|call| call.call == "body_read")
            .and_then(|call| call.bytes)
            .unwrap_or(BOOTSTRAP_TRAILER_BYTE_COUNT);
        let entry_extent = body_read + ENTRY_HEADER_BYTE_COUNT;
        if vector.name == "body_read_failure" {
            return OpenedPresentationResource::new(
                (entry_extent as u16).to_le_bytes().to_vec(),
                vector.source_offset.saturating_sub(ENTRY_HEADER_BYTE_COUNT),
                PresentationSourceLease::SharedArchive,
            );
        }

        let absolute_origin = vector.source_offset - entry_extent;
        let source_len = vector.source_remaining + entry_extent;
        let mut bytes = vec![u8::MIN; source_len];
        bytes[..ENTRY_HEADER_BYTE_COUNT].copy_from_slice(&(entry_extent as u16).to_le_bytes());
        let palette_bytes = palette_stream(entry_extent - BOOTSTRAP_TRAILER_BYTE_COUNT);
        bytes[ENTRY_HEADER_BYTE_COUNT..ENTRY_HEADER_BYTE_COUNT + palette_bytes.len()]
            .copy_from_slice(&palette_bytes);
        bytes[ENTRY_HEADER_BYTE_COUNT + palette_bytes.len()..vector.metadata_offset]
            .fill(METADATA_PADDING_BYTE);

        let range_relative = vector.range_start - vector.source_offset;
        let index_relative = vector.index_start - vector.source_offset;
        let selected_offset = if vector.record_flags as u8 & ALTERNATE_RANGE_FLAG == u8::MIN {
            PRIMARY_RANGE_TABLE_OFFSET
        } else {
            ALTERNATE_RANGE_TABLE_OFFSET
        };
        write_dword(
            &mut bytes,
            vector.metadata_offset + selected_offset,
            range_relative,
        );
        write_dword(
            &mut bytes,
            vector.metadata_offset + FINAL_ENTRY_METRIC * RANGE_TABLE_STRIDE,
            index_relative,
        );
        OpenedPresentationResource::new(
            bytes,
            absolute_origin,
            match vector.mode.as_str() {
                "external" => PresentationSourceLease::Owned,
                "banked" | "embedded" => PresentationSourceLease::SharedArchive,
                mode => panic!("unknown oracle source mode {mode}"),
            },
        )
    }

    #[test]
    fn switch_semantics_account_for_every_original_vector() {
        let vectors: Vec<SwitchOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9f8e_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SWITCH_VECTOR_COUNT);

        for vector in vectors {
            let descriptor = PresentationResourceDescriptor {
                flags: vector.record_flags as u8,
                filename: BloodResourceName::new(b"RESOURCE.DAT").unwrap(),
                cached_range: None,
            };
            let mut descriptors = vec![descriptor.clone(); usize::from(vector.resource_id) + 1];
            descriptors[usize::from(vector.resource_id)] = descriptor;
            let provider_result = if vector.name == "external_open_failure" {
                Err(PresentationResourceOpenError::new("oracle open failure"))
            } else {
                Ok(opened_resource(&vector))
            };
            let mut provider = VectorProvider {
                result: Some(provider_result),
            };
            let mut queue = PresentationQueueState {
                head: 9,
                tail: 8,
                status_bits: u8::MAX,
                read_wrap_index: 7,
                ..PresentationQueueState::default()
            };
            let mut palette = PresentationPaletteState::default();
            let mut state = PresentationResourceStreamState::default();
            let result = switch_presentation_resource(
                &mut state,
                PresentationResourceId::new(vector.resource_id),
                &mut PresentationResourceSwitchContext {
                    descriptors: &descriptors,
                    variant: vector.record_flags.to_le_bytes()[1],
                    queue: &mut queue,
                    queue_capacity: usize::from(u16::MAX) + 1,
                    palette: &mut palette,
                    render_update_flags: if vector.mode == "embedded" {
                        u8::MIN
                    } else {
                        1
                    },
                    provider: &mut provider,
                },
            );

            assert_eq!(
                state.requested,
                Some(PresentationResourceId::new(vector.resource_id))
            );
            assert_eq!(
                state.active,
                Some(PresentationResourceId::new(vector.resource_id))
            );
            assert_eq!(state.flags, vector.record_flags, "{}", vector.name);
            assert_eq!(queue.head, usize::MIN, "{}", vector.name);
            assert_eq!(queue.tail, usize::MIN, "{}", vector.name);
            assert_eq!(queue.status_bits, u8::MIN, "{}", vector.name);

            if vector.success {
                let outcome = result.unwrap();
                assert_eq!(
                    outcome.metadata_position, vector.metadata_offset,
                    "{}",
                    vector.name
                );
                assert_eq!(state.entry_metric, vector.entry_metric, "{}", vector.name);
                assert_eq!(state.absolute_source_offset(), Some(vector.source_offset));
                assert_eq!(state.source_remaining(), Some(vector.source_remaining));
                let range = state.range.unwrap();
                let index = state.index_range.unwrap();
                assert_eq!(
                    state.absolute_origin + range.position,
                    vector.range_start,
                    "{}",
                    vector.name
                );
                assert_eq!(range.remaining, vector.range_remaining, "{}", vector.name);
                assert_eq!(
                    state.absolute_origin + index.position,
                    vector.index_start,
                    "{}",
                    vector.name
                );
                assert_eq!(index.remaining, vector.index_remaining, "{}", vector.name);
                assert!(state.ready, "{}", vector.name);
            } else {
                assert!(result.is_err(), "{}", vector.name);
                assert!(!state.ready, "{}", vector.name);
            }
        }
    }

    #[test]
    fn descriptor_and_relative_range_failures_are_explicit() {
        let mut provider = VectorProvider { result: None };
        let mut queue = PresentationQueueState::default();
        let mut palette = PresentationPaletteState::default();
        let mut state = PresentationResourceStreamState::default();
        let resource = PresentationResourceId::new(3);
        let result = switch_presentation_resource(
            &mut state,
            resource,
            &mut PresentationResourceSwitchContext {
                descriptors: &[],
                variant: u8::MIN,
                queue: &mut queue,
                queue_capacity: 1024,
                palette: &mut palette,
                render_update_flags: u8::MIN,
                provider: &mut provider,
            },
        );
        assert_eq!(
            result,
            Err(PresentationResourceSwitchError::DescriptorUnavailable { resource })
        );
    }
}
