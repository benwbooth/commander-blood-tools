//! Flat streamed-video playback over the translated presentation queue.

use anyhow::{Context, Result};
use commander_blood_formats::archive::BloodResourceName;

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    FlatPresentationEntryPresenter, IndexedGamePalette, InputCancellationBackend,
    PresentationActiveEntryState, PresentationEntryPolicy, PresentationPaletteState,
    PresentationPresentPolicy, PresentationQueueClock, PresentationQueueClockGates,
    PresentationQueueLinkCursor, PresentationQueueRefillOutcome, PresentationQueueServiceContext,
    PresentationQueueServiceOutcome, PresentationQueueState, PresentationResourceCursor,
    PresentationResourceDescriptor, PresentationResourceId, PresentationResourceSequenceContext,
    PresentationResourceSequenceOutcome, PresentationResourceStreamState, PresentationSourceRange,
    load_presentation_resource_sequence, service_presentation_queue,
};

use super::OriginalGameRuntime;

const RUNTIME_PRESENTATION_RESOURCE_ID: PresentationResourceId =
    PresentationResourceId::new(u16::MIN);
const PRESENTATION_BUFFER_BYTE_COUNT: usize = u16::MAX as usize + 1;
const PRESENTED_FRAME_INCREMENT: u64 = 1;

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
    /// Whether voice position or the software timer paces frame retirement.
    pub clock_gates: PresentationQueueClockGates,
}

impl RuntimePresentationRequest {
    /// Build an ordinary software-clocked request for one validated resource.
    pub const fn new(resource_name: BloodResourceName) -> Self {
        Self {
            resource_name,
            descriptor_flags: u8::MIN,
            variant: u8::MIN,
            entry_policy: PresentationEntryPolicy {
                sound_enabled: true,
                skip_back_buffer_present: false,
                draw_via_back_buffer: false,
            },
            present_policy: PresentationPresentPolicy {
                draw_via_back_buffer: false,
                skip_back_buffer_present: false,
                unclamped_rows: false,
                vertical_offset: usize::MIN,
            },
            clock_gates: PresentationQueueClockGates {
                primary_mode: false,
                secondary_mode: false,
                voice_playback: false,
            },
        }
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
    provider: OriginalResourceStore,
    stream: PresentationResourceStreamState,
    queue: PresentationQueueState,
    queue_buffer: Box<[u8]>,
    palette: PresentationPaletteState,
    active_entry: PresentationActiveEntryState,
    entry_policy: PresentationEntryPolicy,
    present_policy: PresentationPresentPolicy,
    clock: PresentationQueueClock,
    clock_gates: PresentationQueueClockGates,
    link_cursor: PresentationQueueLinkCursor,
    decode_staging: Box<[u8]>,
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
        let descriptor = PresentationResourceDescriptor {
            flags: request.descriptor_flags,
            filename: request.resource_name,
            cached_range: None,
        };
        let mut palette = PresentationPaletteState {
            live: *runtime.live_palette(),
            render_snapshot: *runtime.live_palette(),
            dirty: false,
        };
        let mut player = Self {
            descriptors: [descriptor],
            provider: runtime.data().resource_store().clone(),
            stream: PresentationResourceStreamState::default(),
            queue: PresentationQueueState::default(),
            queue_buffer: zeroed_presentation_buffer(),
            palette: palette.clone(),
            active_entry: PresentationActiveEntryState::default(),
            entry_policy: request.entry_policy,
            present_policy: request.present_policy,
            clock: PresentationQueueClock::default(),
            clock_gates: request.clock_gates,
            link_cursor: PresentationQueueLinkCursor::default(),
            decode_staging: zeroed_presentation_buffer(),
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
        *runtime.live_palette_mut() = player.palette.live;
        Ok((player, outcome))
    }

    /// Advance the translated queue once using host audio and timer positions.
    pub fn service_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        audio_position: u16,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        import_shared_live_palette(runtime.live_palette(), &mut self.palette);
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
                clock_gates: self.clock_gates,
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
        publish_shared_live_palette(
            queue_applied_palette(&queue),
            &self.palette,
            runtime.live_palette_mut(),
        );
        Ok(RuntimePresentationStepOutcome {
            queue,
            stream_finished: self.finished,
            queue_metrics: self.queue_metrics()?,
        })
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

    /// Palette snapshot mirrored by the recovered low-128-color copy gate.
    pub const fn render_palette_snapshot(&self) -> &IndexedGamePalette {
        &self.palette.render_snapshot
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

fn queue_applied_palette(outcome: &PresentationQueueServiceOutcome) -> bool {
    matches!(
        outcome,
        PresentationQueueServiceOutcome::Active {
            palette: Some(_),
            ..
        }
    )
}

fn import_shared_live_palette(shared: &IndexedGamePalette, stream: &mut PresentationPaletteState) {
    stream.live = *shared;
}

fn publish_shared_live_palette(
    palette_applied: bool,
    stream: &PresentationPaletteState,
    shared: &mut IndexedGamePalette,
) {
    if palette_applied {
        *shared = stream.live;
    }
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

    use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};
    use sha2::{Digest, Sha256};

    use super::super::{OriginalGameData, OriginalGameDataPaths};
    use super::*;

    const TEST_VIDEO_RESOURCE: &[u8] = b"SQ\\LOGO01.HNM";
    const OPENING_VIDEO_RESOURCE: &[u8] = b"SQ\\MIND.HNM";
    const CLIPTOOT_VIDEO_RESOURCE: &[u8] = b"SQ\\CLIPTOOT.HNM";
    const HNM_FILENAME_SUFFIX: &[u8] = b".HNM";
    const SHIPPED_HNM_RESOURCE_COUNT: usize = 701;
    const MAXIMUM_TEST_SERVICE_CALLS: usize = 10_000;
    const MINIMUM_EXPECTED_FRAME_COUNT: u64 = 2;
    const TEST_PALETTE_INDEX: usize = 17;
    const TEST_SEQUENCE_INDEX: u16 = 37;
    const EXTERNAL_PALETTE_COLOR: [u8; RGB_COMPONENT_COUNT] = [3, 5, 7];
    const VIDEO_PALETTE_COLOR: [u8; RGB_COMPONENT_COUNT] = [11, 13, 17];
    const MIND_ORACLE_FRAME_INDEX: u16 = 30;
    const MIND_ORACLE_READ_WRAP_INDEX: u16 = MIND_ORACLE_FRAME_INDEX + 1;
    const MIND_EXPECTED_FRAME_COUNT: u64 = 263;
    const MIND_ORACLE_LIVE_PALETTE_SHA256: &str =
        "37278e27614ceaf4300b9e16d6704cba054fe4cd63b675630e8a6bd4d3186df2";
    const MIND_ORACLE_FRONT_BUFFER_SHA256: &str =
        "d147b06b531c26aa2e57565f57c286df48658b2116472855a0067fd0fe786d97";
    const MIND_ORACLE_BACK_BUFFER_SHA256: &str =
        "4f7988030a00d082fe445e00a2ac5dab502300ff1b80e8592dd569867b60ef74";
    const PRESENTATION_PALETTE_COLOR_COUNT: usize = 128;
    const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
    const CONTENT_PANEL_TOP: usize = 10;
    const CONTENT_PANEL_ROW_COUNT: usize = 130;
    const CLIPTOOT_BOTTOM_ROW_ORACLE_FRAME: u16 = 856;
    const CLIPTOOT_BOTTOM_ROW_ORACLE_PIXELS: &[(usize, u8)] = &[(25, 18), (26, 6)];

    #[test]
    fn stream_palette_only_publishes_when_a_palette_record_was_applied() {
        let mut shared = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        shared[TEST_PALETTE_INDEX] = EXTERNAL_PALETTE_COLOR;
        let mut stream = PresentationPaletteState::default();

        import_shared_live_palette(&shared, &mut stream);
        assert_eq!(stream.live[TEST_PALETTE_INDEX], EXTERNAL_PALETTE_COLOR);

        stream.live[TEST_PALETTE_INDEX] = VIDEO_PALETTE_COLOR;
        publish_shared_live_palette(false, &stream, &mut shared);
        assert_eq!(shared[TEST_PALETTE_INDEX], EXTERNAL_PALETTE_COLOR);

        publish_shared_live_palette(true, &stream, &mut shared);
        assert_eq!(shared[TEST_PALETTE_INDEX], VIDEO_PALETTE_COLOR);
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
                .service_frame(&mut runtime, u16::MIN, timer_tick, false)
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
                .service_frame(&mut runtime, clock, clock, false)
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
                Sha256::digest(runtime.live_palette().as_flattened())
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
            runtime.live_palette()[PRESENTATION_PALETTE_COLOR_COUNT..]
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
                .service_frame(&mut runtime, u16::MIN, timer_tick as u16, false)
                .unwrap();
            if stream.is_finished() {
                break;
            }
        }

        assert!(stream.is_finished());
        assert_eq!(stream.presented_frame_count(), MIND_EXPECTED_FRAME_COUNT);
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
                .service_frame(&mut runtime, u16::MIN, timer_tick, false)
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
