//! Flat streamed-video playback over the translated presentation queue.

use anyhow::{Context, Result};
use commander_blood_formats::archive::BloodResourceName;

use crate::assets::OriginalResourceStore;
use crate::native::bloodprg::{
    FlatPresentationEntryPresenter, PresentationActiveEntryState, PresentationEntryPolicy,
    PresentationPaletteState, PresentationPresentPolicy, PresentationQueueClock,
    PresentationQueueClockGates, PresentationQueueLinkCursor, PresentationQueueRefillOutcome,
    PresentationQueueServiceContext, PresentationQueueServiceOutcome, PresentationQueueState,
    PresentationResourceDescriptor, PresentationResourceId, PresentationResourceSequenceContext,
    PresentationResourceSequenceOutcome, PresentationResourceStreamState,
    load_presentation_resource_sequence, service_presentation_queue,
};

use super::OriginalGameRuntime;

const RUNTIME_PRESENTATION_RESOURCE_ID: PresentationResourceId =
    PresentationResourceId::new(u16::MIN);
const PRESENTATION_BUFFER_BYTE_COUNT: usize = u16::MAX as usize + 1;
const PRESENTATION_RENDER_UPDATE_FLAGS: u8 = u8::MIN;
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
                render_update_flags: PRESENTATION_RENDER_UPDATE_FLAGS,
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
    ) -> Result<RuntimePresentationStepOutcome> {
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
                render_update_flags: PRESENTATION_RENDER_UPDATE_FLAGS,
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
        *runtime.live_palette_mut() = self.palette.live;
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

    /// Number of decoded frames retired into the logical framebuffer.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presented_frame_count
    }

    /// Exact authored resource currently supplying the stream.
    pub fn resource_name(&self) -> &BloodResourceName {
        &self.descriptors[usize::MIN].filename
    }

    pub(crate) fn queue_metrics(&self) -> Result<RuntimePresentationQueueMetrics> {
        Ok(RuntimePresentationQueueMetrics {
            entry_metric: u16::try_from(self.stream.entry_metric)
                .context("presentation entry metric exceeds the native word range")?,
            read_wrap_index: self.queue.read_wrap_index,
        })
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

    use super::super::{OriginalGameData, OriginalGameDataPaths};
    use super::*;

    const TEST_VIDEO_RESOURCE: &[u8] = b"SQ\\LOGO01.HNM";
    const HNM_FILENAME_SUFFIX: &[u8] = b".HNM";
    const SHIPPED_HNM_RESOURCE_COUNT: usize = 701;
    const MAXIMUM_TEST_SERVICE_CALLS: usize = 10_000;
    const MINIMUM_EXPECTED_FRAME_COUNT: u64 = 2;

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
            RuntimePresentationStream::load(&mut runtime, request, u16::MIN).unwrap();

        assert!(initial.initial_entry_accepted);
        assert!(initial.initial_present.frame_presented);
        let mut saw_visible_pixels = runtime
            .front_buffer()
            .pixels()
            .iter()
            .any(|pixel| *pixel != u8::MIN);
        for timer_tick in 1..=MAXIMUM_TEST_SERVICE_CALLS {
            let timer_tick = timer_tick as u16;
            stream
                .service_frame(&mut runtime, u16::MIN, timer_tick)
                .unwrap_or_else(|error| panic!("frame {timer_tick} failed: {error:#}"));
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
        assert!(stream.presented_frame_count() >= MINIMUM_EXPECTED_FRAME_COUNT);
        assert!(saw_visible_pixels);
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
                RuntimePresentationStream::load(&mut runtime, request, u16::MIN)
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
