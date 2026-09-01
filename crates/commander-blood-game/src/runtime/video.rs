//! Flat streamed-video playback over the translated presentation queue.

use anyhow::{Context, Result};
use commander_blood_formats::archive::BloodResourceName;

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    FlatPresentationEntryPresenter, IndexedGamePalette, InputCancellationBackend,
    OpenedPresentationResource, PresentationActiveEntryState, PresentationEntryPolicy,
    PresentationPaletteState, PresentationPresentPolicy, PresentationQueueClock,
    PresentationQueueClockGates, PresentationQueueLinkCursor, PresentationQueueRefillOutcome,
    PresentationQueueServiceContext, PresentationQueueServiceOutcome, PresentationQueueState,
    PresentationResourceCursor, PresentationResourceDescriptor, PresentationResourceId,
    PresentationResourceOpenError, PresentationResourceProvider,
    PresentationResourceSequenceContext, PresentationResourceSequenceOutcome,
    PresentationResourceStreamState, PresentationSourceLease, PresentationSourceRange,
    clear_scene_palette_entries, load_presentation_resource_sequence,
    presentation_resource_enabled, service_presentation_queue,
};
use crate::render::indexed_frame_rgba;

use super::OriginalGameRuntime;

const RUNTIME_PRESENTATION_RESOURCE_ID: PresentationResourceId =
    PresentationResourceId::new(u16::MIN);
const PRESENTATION_BUFFER_BYTE_COUNT: usize = u16::MAX as usize + 1;
const PRESENTED_FRAME_INCREMENT: u64 = 1;
// GS:0x0B17 has no recovered writer and remains zero in guarded native runs.
const SHIPPED_PRESENTATION_SOUND_STATE: u8 = u8::MIN;

/// Authored descriptor and runtime gates for one HNM presentation stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePresentationRequest {
    /// Exact archive or loose-file resource name, including its authored directory.
    pub resource_name: BloodResourceName,
    /// Low-byte descriptor flags recovered from the calling presentation scene.
    pub descriptor_flags: u8,
    /// Runtime variant stored in the descriptor flag word's high byte.
    pub variant: u8,
    /// Sound and immediate/deferred decode gates for queue-entry activation.
    pub entry_policy: PresentationEntryPolicy,
    /// Destination, vertical placement, and row-clamping policy.
    pub present_policy: PresentationPresentPolicy,
    /// Persistent DESCRIPT idle-video bytes selected instead of reopening a file.
    pub shared_source: Option<Box<[u8]>>,
}

impl RuntimePresentationRequest {
    /// Build an ordinary software-clocked request for one validated resource.
    pub const fn new(resource_name: BloodResourceName) -> Self {
        Self {
            resource_name,
            descriptor_flags: u8::MIN,
            variant: u8::MIN,
            entry_policy: PresentationEntryPolicy {
                sound_enabled: presentation_resource_enabled(SHIPPED_PRESENTATION_SOUND_STATE),
                skip_back_buffer_present: false,
                draw_via_back_buffer: false,
            },
            present_policy: PresentationPresentPolicy {
                draw_via_back_buffer: false,
                skip_back_buffer_present: false,
                unclamped_rows: false,
                vertical_offset: usize::MIN,
            },
            shared_source: None,
        }
    }
}

#[derive(Clone)]
struct RuntimePresentationProvider {
    store: OriginalResourceStore,
    shared_source: Option<Box<[u8]>>,
}

impl PresentationResourceProvider for RuntimePresentationProvider {
    fn open_presentation_resource(
        &mut self,
        descriptor: &PresentationResourceDescriptor,
    ) -> Result<OpenedPresentationResource, PresentationResourceOpenError> {
        if let Some(bytes) = self.shared_source.as_ref() {
            return Ok(OpenedPresentationResource::new(
                bytes.clone(),
                usize::MIN,
                PresentationSourceLease::SharedCache,
            ));
        }
        self.store.open_presentation_resource(descriptor)
    }
}

/// One serviced presentation frame and whether it exhausted the stream.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimePresentationStepOutcome {
    /// Exact translated queue-service result, including an embedded sound record.
    pub queue: PresentationQueueServiceOutcome,
    /// Whether no frame remains in the selected source or queue.
    pub stream_finished: bool,
    /// Native-width queue counters sampled after this service pass.
    pub queue_metrics: RuntimePresentationQueueMetrics,
}

/// Queue counters consumed by recovered scene-transition thresholds.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RuntimePresentationQueueMetrics {
    /// Current source-entry metric.
    pub entry_metric: u16,
    /// One-based index of the entry currently being consumed.
    pub read_wrap_index: u16,
    /// Monotonic authored entry sequence consumed by DESCRIPT subtitle cues.
    pub sequence_index: u16,
}

/// Persistent flat ownership for one streamed HNM resource.
pub struct RuntimePresentationStream {
    descriptors: [PresentationResourceDescriptor; 1],
    provider: RuntimePresentationProvider,
    stream: PresentationResourceStreamState,
    queue: PresentationQueueState,
    queue_buffer: Box<[u8]>,
    palette: PresentationPaletteState,
    active_entry: PresentationActiveEntryState,
    entry_policy: PresentationEntryPolicy,
    present_policy: PresentationPresentPolicy,
    clock: PresentationQueueClock,
    link_cursor: PresentationQueueLinkCursor,
    decode_staging: Box<[u8]>,
    display_indices: Box<[u8]>,
    display_rgba: Box<[u8]>,
    finished: bool,
    presented_frame_count: u64,
}

impl RuntimePresentationStream {
    /// Open, bootstrap, present, and prefill one original HNM stream.
    pub fn load(
        runtime: &mut OriginalGameRuntime,
        request: RuntimePresentationRequest,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<(Self, PresentationResourceSequenceOutcome)> {
        let source_colors = *runtime.live_palette();
        Self::load_with_source_colors(
            runtime,
            request,
            source_colors,
            timer_tick,
            render_snapshot_suppressed,
        )
    }

    /// Open one HNM stream with colors inherited from the preceding display owner.
    ///
    /// The DOS DAC naturally persisted across chained HNM resources. The flat
    /// renderer keeps that continuity inside the video decoder instead of
    /// publishing video-local colors into game or bridge state.
    pub(super) fn load_with_source_colors(
        runtime: &mut OriginalGameRuntime,
        request: RuntimePresentationRequest,
        source_colors: IndexedGamePalette,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<(Self, PresentationResourceSequenceOutcome)> {
        let descriptor = PresentationResourceDescriptor {
            flags: request.descriptor_flags,
            filename: request.resource_name,
            cached_range: None,
        };
        let mut palette = PresentationPaletteState {
            live: source_colors,
            render_snapshot: source_colors,
            dirty: false,
        };
        let provider = RuntimePresentationProvider {
            store: runtime.data().resource_store().clone(),
            shared_source: request.shared_source,
        };
        let mut player = Self {
            descriptors: [descriptor],
            provider,
            stream: PresentationResourceStreamState::default(),
            queue: PresentationQueueState::default(),
            queue_buffer: zeroed_presentation_buffer(),
            palette: palette.clone(),
            active_entry: PresentationActiveEntryState::default(),
            entry_policy: request.entry_policy,
            present_policy: request.present_policy,
            clock: PresentationQueueClock::default(),
            link_cursor: PresentationQueueLinkCursor::default(),
            decode_staging: zeroed_presentation_buffer(),
            display_indices: Box::default(),
            display_rgba: Box::default(),
            finished: false,
            presented_frame_count: u64::MIN,
        };

        let (display_buffer, back_buffer) = runtime.presentation_buffers_mut();
        let mut presenter = FlatPresentationEntryPresenter {
            display_buffer,
            back_buffer,
            decode_staging: &mut player.decode_staging,
        };
        let outcome = load_presentation_resource_sequence(
            &mut player.stream,
            RUNTIME_PRESENTATION_RESOURCE_ID,
            &mut PresentationResourceSequenceContext {
                descriptors: &player.descriptors,
                variant: request.variant,
                queue: &mut player.queue,
                queue_buffer: &mut player.queue_buffer,
                palette: &mut palette,
                render_update_flags: u8::from(render_snapshot_suppressed),
                provider: &mut player.provider,
                entry_policy: player.entry_policy,
                active_entry: &mut player.active_entry,
                present_policy: player.present_policy,
                host: &mut presenter,
                clock: &mut player.clock,
                timer_tick,
            },
        )
        .with_context(|| {
            format!(
                "loading presentation stream {}",
                resource_name(&player.descriptors[usize::MIN].filename)
            )
        })?;
        player.link_cursor = outcome.link_cursor;
        player.palette = palette;
        if outcome.initial_present.frame_presented {
            player.presented_frame_count = player
                .presented_frame_count
                .wrapping_add(PRESENTED_FRAME_INCREMENT);
        }
        player.resolve_display_rgba(runtime.front_buffer().pixels())?;
        Ok((player, outcome))
    }

    /// Advance the translated queue once using host audio and timer positions.
    pub fn service_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        audio_position: u16,
        timer_tick: u16,
        clock_gates: PresentationQueueClockGates,
        render_snapshot_suppressed: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        self.service_frame_with_link_target(
            runtime,
            None,
            audio_position,
            timer_tick,
            clock_gates,
            render_snapshot_suppressed,
        )
    }

    /// Advance one queue frame from the caller's recovered `BP` link cursor.
    pub fn service_frame_from_link_target(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        link_target: u16,
        audio_position: u16,
        timer_tick: u16,
        clock_gates: PresentationQueueClockGates,
        render_snapshot_suppressed: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        self.service_frame_with_link_target(
            runtime,
            Some(link_target),
            audio_position,
            timer_tick,
            clock_gates,
            render_snapshot_suppressed,
        )
    }

    fn service_frame_with_link_target(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        link_target: Option<u16>,
        audio_position: u16,
        timer_tick: u16,
        clock_gates: PresentationQueueClockGates,
        render_snapshot_suppressed: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        if let Some(link_target) = link_target {
            self.import_service_link_target(link_target);
        }
        let (display_buffer, back_buffer) = runtime.presentation_buffers_mut();
        let mut presenter = FlatPresentationEntryPresenter {
            display_buffer,
            back_buffer,
            decode_staging: &mut self.decode_staging,
        };
        let mut read_audio_position = || audio_position;
        let mut read_timer_tick = || timer_tick;
        let queue = service_presentation_queue(
            &mut self.stream,
            &mut PresentationQueueServiceContext {
                descriptors: &self.descriptors,
                queue: &mut self.queue,
                queue_buffer: &mut self.queue_buffer,
                entry_policy: self.entry_policy,
                active_entry: &mut self.active_entry,
                present_policy: self.present_policy,
                host: &mut presenter,
                palette: &mut self.palette,
                render_update_flags: u8::from(render_snapshot_suppressed),
                clock: &mut self.clock,
                clock_gates,
                audio_position: &mut read_audio_position,
                timer_tick: &mut read_timer_tick,
                link_cursor: &mut self.link_cursor,
            },
        )
        .with_context(|| {
            format!(
                "servicing presentation stream {}",
                resource_name(&self.descriptors[usize::MIN].filename)
            )
        })?;
        if queue_presented_frame(&queue) {
            self.presented_frame_count = self
                .presented_frame_count
                .wrapping_add(PRESENTED_FRAME_INCREMENT);
        }
        self.finished |= queue_finished(&queue);
        self.resolve_display_rgba(runtime.front_buffer().pixels())?;
        Ok(RuntimePresentationStepOutcome {
            queue,
            stream_finished: self.finished,
            queue_metrics: self.queue_metrics()?,
        })
    }

    fn import_service_link_target(&mut self, link_target: u16) {
        self.link_cursor = PresentationQueueLinkCursor::new(usize::from(link_target));
    }

    /// Return whether source and queue exhaustion have completed playback.
    pub const fn is_finished(&self) -> bool {
        self.finished
    }

    /// Return whether the recovered queue status still owns or drains a source.
    pub const fn source_open_or_draining(&self) -> bool {
        self.queue.source_open_or_draining()
    }

    /// Number of decoded frames retired into the logical framebuffer.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presented_frame_count
    }

    /// Exact authored resource currently supplying the stream.
    pub fn resource_name(&self) -> &BloodResourceName {
        &self.descriptors[usize::MIN].filename
    }

    #[cfg(test)]
    pub(crate) const fn source_lease(&self) -> PresentationSourceLease {
        self.stream.lease
    }

    /// Palette snapshot mirrored by the recovered low-128-color copy gate.
    pub const fn render_palette_snapshot(&self) -> &IndexedGamePalette {
        &self.palette.render_snapshot
    }

    /// Palette that resolves the currently retained indexed HNM page to RGB.
    pub const fn display_palette(&self) -> &IndexedGamePalette {
        &self.palette.live
    }

    /// Mutably borrow HNM-local source colors for recovered presentation fades.
    pub fn display_palette_mut(&mut self) -> &mut IndexedGamePalette {
        &mut self.palette.live
    }

    /// Indexed source page retained only to resolve legacy HNM colors to RGBA.
    pub fn display_indices(&self) -> &[u8] {
        &self.display_indices
    }

    /// True-color page produced from the current indexed HNM frame and its local palette.
    pub fn display_rgba(&self) -> &[u8] {
        &self.display_rgba
    }

    /// Resolve the latest legacy indexed page into the renderer-owned RGBA surface.
    pub fn resolve_display_rgba(&mut self, indexed_pixels: &[u8]) -> Result<()> {
        self.display_indices = Box::from(indexed_pixels);
        self.resolve_retained_display_rgba()
    }

    /// Apply the native low scene-color clear to this stream's private mapping.
    pub fn clear_scene_source_colors(&mut self) -> Result<()> {
        clear_scene_palette_entries(&mut self.palette.live);
        self.resolve_retained_display_rgba()
    }

    fn resolve_retained_display_rgba(&mut self) -> Result<()> {
        self.display_rgba = indexed_frame_rgba(&self.display_indices, &self.palette.live)
            .context("resolving the current HNM display page to true-color RGBA")?
            .into_boxed_slice();
        Ok(())
    }

    pub(crate) fn queue_metrics(&self) -> Result<RuntimePresentationQueueMetrics> {
        Ok(RuntimePresentationQueueMetrics {
            entry_metric: u16::try_from(self.stream.entry_metric)
                .context("presentation entry metric exceeds the native word range")?,
            read_wrap_index: self.queue.read_wrap_index,
            sequence_index: self.queue.sequence_index,
        })
    }

    /// Snapshot the native resource cursor and its authored cancellation rewind point.
    pub(crate) fn cancellation_cursor(&self) -> Option<PresentationResourceCursor> {
        let source = self.stream.source.as_ref()?;
        let rewind = self.stream.index_range?;
        Some(PresentationResourceCursor {
            read_position: source.position(),
            remaining: self.stream.source_remaining()?,
            rewind_position: rewind.position,
            rewind_remaining: rewind.remaining,
        })
    }

    /// Restore the validated range selected by the recovered cancellation routine.
    pub(crate) fn apply_cancellation_cursor(
        &mut self,
        cursor: PresentationResourceCursor,
    ) -> Result<()> {
        self.stream
            .select_range(PresentationSourceRange {
                position: cursor.read_position,
                remaining: cursor.remaining,
            })
            .context("rewinding the cancelled presentation resource")?;
        self.active_entry = PresentationActiveEntryState::default();
        self.finished = false;
        Ok(())
    }
}

impl InputCancellationBackend for RuntimePresentationStream {
    fn reset_presentation_queue(&mut self) {
        self.queue.reset(self.queue_buffer.len());
    }
}

fn zeroed_presentation_buffer() -> Box<[u8]> {
    vec![u8::MIN; PRESENTATION_BUFFER_BYTE_COUNT].into_boxed_slice()
}

fn queue_presented_frame(outcome: &PresentationQueueServiceOutcome) -> bool {
    matches!(
        outcome,
        PresentationQueueServiceOutcome::Active {
            present: Some(present),
            ..
        } if present.frame_presented
    )
}

fn refill_finished(refill: &PresentationQueueRefillOutcome) -> bool {
    matches!(
        refill,
        PresentationQueueRefillOutcome::Finished {
            queue_empty: true,
            ..
        }
    )
}

fn queue_finished(outcome: &PresentationQueueServiceOutcome) -> bool {
    match outcome {
        PresentationQueueServiceOutcome::SourceUnavailable => true,
        PresentationQueueServiceOutcome::HighPriorityRefill { refill }
        | PresentationQueueServiceOutcome::WaitingForEntry { refill, .. }
        | PresentationQueueServiceOutcome::RejectedStaleLink { refill, .. }
        | PresentationQueueServiceOutcome::Active { refill, .. } => refill_finished(refill),
    }
}

fn resource_name(name: &BloodResourceName) -> String {
    String::from_utf8_lossy(name.as_bytes()).into_owned()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;
    use std::path::{Path, PathBuf};

    use commander_blood_formats::lbm::RGB_COMPONENT_COUNT;
    use sha2::{Digest, Sha256};

    use super::super::{OriginalGameData, OriginalGameDataPaths};
    use super::*;

    const TEST_VIDEO_RESOURCE: &[u8] = b"SQ\\LOGO01.HNM";
    const OPENING_VIDEO_RESOURCE: &[u8] = b"SQ\\MIND.HNM";
    const CLIPTOOT_VIDEO_RESOURCE: &[u8] = b"SQ\\CLIPTOOT.HNM";
    const PTERRA_VIDEO_RESOURCE: &[u8] = b"PL\\PTERRA10.HNM";
    const HNM_FILENAME_SUFFIX: &[u8] = b".HNM";
    const SHIPPED_HNM_RESOURCE_COUNT: usize = 701;
    const MAXIMUM_TEST_SERVICE_CALLS: usize = 10_000;
    const MINIMUM_EXPECTED_FRAME_COUNT: u64 = 2;
    const TEST_SEQUENCE_INDEX: u16 = 37;
    const MIND_ORACLE_FRAME_INDEX: u16 = 30;
    const MIND_ORACLE_READ_WRAP_INDEX: u16 = MIND_ORACLE_FRAME_INDEX + 1;
    const MIND_EXPECTED_FRAME_COUNT: u64 = 263;
    const MIND_ORACLE_LIVE_PALETTE_SHA256: &str =
        "37278e27614ceaf4300b9e16d6704cba054fe4cd63b675630e8a6bd4d3186df2";
    const MIND_ORACLE_FRONT_BUFFER_SHA256: &str =
        "d147b06b531c26aa2e57565f57c286df48658b2116472855a0067fd0fe786d97";
    const MIND_ORACLE_BACK_BUFFER_SHA256: &str =
        "4f7988030a00d082fe445e00a2ac5dab502300ff1b80e8592dd569867b60ef74";
    const MIND_FIRST_ACTION_DISPLAY_FRAME: u64 = 122;
    const MIND_FIRST_ACTION_DISPLAY_HASH: &str = "f60b6847044b71de";
    const MIND_FIRST_ACTION_DISPLAY_RGB_HASH: &str = "6894f0c2f97a5caf";
    const MIND_FIRST_ACTION_LOGICAL_FRAME: u64 = 123;
    const MIND_FIRST_ACTION_LOGICAL_HASH: &str = "f7f6a4473cfe6f67";
    const MIND_FIRST_ACTION_LOGICAL_RGB_HASH: &str = "7baca932bf4c149e";
    const MIND_SECOND_ACTION_FRAME: u64 = 242;
    const MIND_SECOND_ACTION_HASH: &str = "f825996090b5cb13";
    const MIND_SECOND_ACTION_RGB_HASH: &str = "58e5176c3944a09b";
    const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
    const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;
    const PRESENTATION_PALETTE_COLOR_COUNT: usize = 128;
    const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
    const CONTENT_PANEL_TOP: usize = 10;
    const CONTENT_PANEL_ROW_COUNT: usize = 130;
    const CLIPTOOT_BOTTOM_ROW_ORACLE_FRAME: u16 = 856;
    const CLIPTOOT_BOTTOM_ROW_ORACLE_PIXELS: &[(usize, u8)] = &[(25, 18), (26, 6)];

    #[test]
    fn ordinary_request_uses_the_shipped_embedded_sound_gate() {
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(TEST_VIDEO_RESOURCE).unwrap());

        assert!(!request.entry_policy.sound_enabled);
    }

    #[test]
    fn queue_service_imports_the_callers_native_link_cursor() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(TEST_VIDEO_RESOURCE).unwrap());
        let (mut stream, _) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();

        stream.import_service_link_target(312);

        assert_eq!(stream.link_cursor.position(), 312);
    }

    #[test]
    fn hnm_color_records_remain_local_to_the_true_color_video_surface() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let runtime_colors = *runtime.live_palette();
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(TEST_VIDEO_RESOURCE).unwrap());
        let (mut stream, _) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();

        assert_eq!(runtime.live_palette(), &runtime_colors);
        stream
            .service_frame(
                &mut runtime,
                u16::MIN,
                1,
                PresentationQueueClockGates::default(),
                false,
            )
            .unwrap();
        assert_eq!(runtime.live_palette(), &runtime_colors);
    }

    #[test]
    fn pterra_initial_palette_record_preserves_the_reserved_high_bank() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let initial = *data.default_vga_palette();
        let mut runtime = OriginalGameRuntime::new(data);
        *runtime.live_palette_mut() = initial;
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(PTERRA_VIDEO_RESOURCE).unwrap());

        let (stream, _) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();

        let changed = initial[192..]
            .iter()
            .zip(&stream.display_palette()[192..])
            .enumerate()
            .filter_map(|(relative, (before, after))| {
                (before != after).then_some((relative + 192, *before, *after))
            })
            .collect::<Vec<_>>();
        assert!(
            changed.is_empty(),
            "Pterra's initial HNM palette record changed reserved colors: {changed:?}"
        );
        assert_eq!(
            runtime.live_palette(),
            &initial,
            "Pterra's decoder-local colors escaped into the flat game surface"
        );
    }

    #[test]
    fn pterra_true_color_frame_is_independent_of_later_global_palette_changes() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(PTERRA_VIDEO_RESOURCE).unwrap());
        let (stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        assert!(initial.initial_present.frame_presented);

        let expected_rgba = stream.display_rgba().to_vec();
        runtime
            .live_palette_mut()
            .fill([u8::MIN; RGB_COMPONENT_COUNT]);

        let incorrectly_recolored =
            indexed_frame_rgba(runtime.front_buffer().pixels(), runtime.live_palette()).unwrap();
        assert_eq!(stream.display_rgba(), expected_rgba);
        assert_ne!(incorrectly_recolored, expected_rgba);
    }

    #[test]
    fn real_hnm_stream_decodes_to_completion_through_the_flat_queue() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let request =
            RuntimePresentationRequest::new(BloodResourceName::new(TEST_VIDEO_RESOURCE).unwrap());
        let (mut stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        stream.queue.sequence_index = TEST_SEQUENCE_INDEX;
        assert_eq!(
            stream.queue_metrics().unwrap().sequence_index,
            TEST_SEQUENCE_INDEX
        );

        assert!(initial.initial_entry_accepted);
        assert!(initial.initial_present.frame_presented);
        assert!(stream.source_open_or_draining());
        let mut saw_visible_pixels = runtime
            .front_buffer()
            .pixels()
            .iter()
            .any(|pixel| *pixel != u8::MIN);
        for timer_tick in 1..=MAXIMUM_TEST_SERVICE_CALLS {
            let timer_tick = timer_tick as u16;
            stream
                .service_frame(
                    &mut runtime,
                    u16::MIN,
                    timer_tick,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap_or_else(|error| panic!("frame {timer_tick} failed: {error:#}"));
            assert_eq!(stream.source_open_or_draining(), !stream.is_finished());
            saw_visible_pixels |= runtime
                .front_buffer()
                .pixels()
                .iter()
                .any(|pixel| *pixel != u8::MIN);
            if stream.is_finished() {
                break;
            }
        }

        assert!(stream.is_finished());
        assert!(!stream.source_open_or_draining());
        assert!(stream.presented_frame_count() >= MINIMUM_EXPECTED_FRAME_COUNT);
        assert!(saw_visible_pixels);
    }

    #[test]
    fn opening_frame_30_matches_the_original_live_palette_boundary() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut request = RuntimePresentationRequest::new(
            BloodResourceName::new(OPENING_VIDEO_RESOURCE).unwrap(),
        );
        request.entry_policy.skip_back_buffer_present = true;
        request.present_policy.skip_back_buffer_present = true;
        request.present_policy.unclamped_rows = true;
        let (mut stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        assert!(initial.initial_present.frame_presented);

        for clock in 1..=MIND_ORACLE_FRAME_INDEX {
            let step = stream
                .service_frame(
                    &mut runtime,
                    clock,
                    clock,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap();
            assert!(queue_presented_frame(&step.queue), "clock {clock}");
        }

        assert_eq!(
            stream.queue_metrics().unwrap().read_wrap_index,
            MIND_ORACLE_READ_WRAP_INDEX
        );
        assert_eq!(
            format!(
                "{:x}",
                Sha256::digest(stream.display_palette().as_flattened())
            ),
            MIND_ORACLE_LIVE_PALETTE_SHA256
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(runtime.front_buffer().pixels())),
            MIND_ORACLE_FRONT_BUFFER_SHA256
        );
        assert_eq!(
            format!("{:x}", Sha256::digest(runtime.back_buffer().pixels())),
            MIND_ORACLE_BACK_BUFFER_SHA256
        );
        assert!(
            stream.display_palette()[PRESENTATION_PALETTE_COLOR_COUNT..]
                .iter()
                .flatten()
                .all(|component| *component == u8::MIN)
        );
    }

    #[test]
    fn opening_stream_reaches_its_authored_terminal_frame() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let request = RuntimePresentationRequest::new(
            BloodResourceName::new(OPENING_VIDEO_RESOURCE).unwrap(),
        );
        let (mut stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        assert!(initial.initial_present.frame_presented);

        for timer_tick in 1..=MAXIMUM_TEST_SERVICE_CALLS {
            stream
                .service_frame(
                    &mut runtime,
                    u16::MIN,
                    timer_tick as u16,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap();
            if stream.is_finished() {
                break;
            }
        }

        assert!(stream.is_finished());
        assert_eq!(stream.presented_frame_count(), MIND_EXPECTED_FRAME_COUNT);
    }

    #[test]
    fn opening_frames_match_dos_logical_and_display_action_boundaries() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut request = RuntimePresentationRequest::new(
            BloodResourceName::new(OPENING_VIDEO_RESOURCE).unwrap(),
        );
        request.entry_policy.skip_back_buffer_present = true;
        request.present_policy.skip_back_buffer_present = true;
        request.present_policy.unclamped_rows = true;
        let (mut stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        assert!(initial.initial_present.frame_presented);

        let hash = |bytes: &[u8]| {
            let mut hash = 0xcbf29ce484222325u64;
            for byte in bytes {
                hash ^= u64::from(*byte);
                hash = hash.wrapping_mul(0x100000001b3);
            }
            format!("{hash:016x}")
        };
        let rgb_hash = |pixels: &[u8], palette: &IndexedGamePalette| {
            let mut hash = 0xcbf29ce484222325u64;
            for palette_index in pixels {
                for component in palette[usize::from(*palette_index)] {
                    let expanded = (u16::from(component) * EIGHT_BIT_CHANNEL_MAXIMUM
                        / VGA_DAC_CHANNEL_MAXIMUM) as u8;
                    hash ^= u64::from(expanded);
                    hash = hash.wrapping_mul(0x100000001b3);
                }
            }
            format!("{hash:016x}")
        };
        for timer_tick in 1..=242 {
            stream
                .service_frame(
                    &mut runtime,
                    timer_tick,
                    timer_tick,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap();
            let expected = match stream.presented_frame_count() {
                MIND_FIRST_ACTION_DISPLAY_FRAME => Some((
                    MIND_FIRST_ACTION_DISPLAY_HASH,
                    MIND_FIRST_ACTION_DISPLAY_RGB_HASH,
                )),
                MIND_FIRST_ACTION_LOGICAL_FRAME => Some((
                    MIND_FIRST_ACTION_LOGICAL_HASH,
                    MIND_FIRST_ACTION_LOGICAL_RGB_HASH,
                )),
                MIND_SECOND_ACTION_FRAME => {
                    Some((MIND_SECOND_ACTION_HASH, MIND_SECOND_ACTION_RGB_HASH))
                }
                _ => None,
            };
            if let Some((expected_indices, expected_rgb)) = expected {
                assert_eq!(hash(runtime.front_buffer().pixels()), expected_indices);
                assert_eq!(
                    rgb_hash(runtime.front_buffer().pixels(), stream.display_palette()),
                    expected_rgb
                );
            }
        }
    }

    #[test]
    fn cliptoot_bottom_row_matches_authored_image_payload() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        let mut request = RuntimePresentationRequest::new(
            BloodResourceName::new(CLIPTOOT_VIDEO_RESOURCE).unwrap(),
        );
        request.entry_policy.draw_via_back_buffer = true;
        request.present_policy.draw_via_back_buffer = true;
        request.present_policy.vertical_offset = CONTENT_PANEL_TOP;
        let (mut stream, initial) =
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false).unwrap();
        assert!(initial.initial_present.frame_presented);

        let bottom_row = CONTENT_PANEL_TOP + CONTENT_PANEL_ROW_COUNT - 1;
        let row_start = bottom_row * LOGICAL_FRAMEBUFFER_WIDTH;
        let row_end = row_start + LOGICAL_FRAMEBUFFER_WIDTH;
        for timer_tick in 1..=CLIPTOOT_BOTTOM_ROW_ORACLE_FRAME {
            stream
                .service_frame(
                    &mut runtime,
                    u16::MIN,
                    timer_tick,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap();
        }

        let nonzero_pixels = runtime.front_buffer().pixels()[row_start..row_end]
            .iter()
            .enumerate()
            .filter_map(|(x, pixel)| (*pixel != u8::MIN).then_some((x, *pixel)))
            .collect::<Vec<_>>();
        assert_eq!(
            nonzero_pixels, CLIPTOOT_BOTTOM_ROW_ORACLE_PIXELS,
            "CLIPTOOT row 129 is decoded image data, not a reserved metadata row"
        );
    }

    #[test]
    fn every_shipped_hnm_stream_bootstraps_under_the_recovered_back_buffer_policy() {
        let Some(paths) = original_data_paths() else {
            return;
        };
        let data = OriginalGameData::load_with_writable_root(paths, std::env::temp_dir()).unwrap();
        let resource_names: BTreeSet<_> = data
            .resource_store()
            .resource_names()
            .into_iter()
            .filter(|name| {
                name.as_bytes()
                    .get(
                        name.as_bytes()
                            .len()
                            .saturating_sub(HNM_FILENAME_SUFFIX.len())..,
                    )
                    .is_some_and(|suffix| suffix.eq_ignore_ascii_case(HNM_FILENAME_SUFFIX))
            })
            .collect();
        assert_eq!(resource_names.len(), SHIPPED_HNM_RESOURCE_COUNT);
        let mut runtime = OriginalGameRuntime::new(data);

        for resource_name in resource_names {
            let display_name = super::resource_name(&resource_name);
            let mut request = RuntimePresentationRequest::new(resource_name);
            // Lines 2 and 7 select this exact policy in scene dispatch. It
            // forces immediate codec dispatch, providing one uniform corpus
            // check without guessing each resource's eventual DESCRIPT slot.
            request.entry_policy.draw_via_back_buffer = true;
            request.present_policy.draw_via_back_buffer = true;
            let (_stream, outcome) =
                RuntimePresentationStream::load(&mut runtime, request, u16::MIN, false)
                    .unwrap_or_else(|error| panic!("{display_name}: {error:#}"));
            assert!(outcome.initial_entry_accepted, "{display_name}");
            assert!(outcome.initial_present.frame_presented, "{display_name}");
        }
    }

    fn original_data_paths() -> Option<OriginalGameDataPaths> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("commander-blood-audio/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ]
        .into_iter()
        .find_map(|root: PathBuf| OriginalGameDataPaths::from_root(root).ok())
    }
}
