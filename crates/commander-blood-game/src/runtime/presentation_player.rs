//! Runtime ownership for presentation-line resolution and HNM queue playback.

use anyhow::{Context, Result};
use commander_blood_formats::bloodprg::BloodprgPresentationCatalog;

use crate::native::bloodprg::{
    DescriptPresentationAssets, IndexedGamePalette, InputCancellationBackend,
    PresentationPresentPolicy, PresentationQueueClockGates, PresentationResourceCursor,
    PresentationResourceId, PresentationResourceSequenceOutcome, PresentationSceneSource,
};
use crate::render::indexed_frame_rgba;

use super::{
    OriginalGameRuntime, RuntimePresentationCatalog, RuntimePresentationStepOutcome,
    RuntimePresentationStream,
};

/// Flat catalog and active stream for one presentation-line consumer.
pub struct RuntimePresentationPlayer {
    catalog: RuntimePresentationCatalog,
    shared_idle_video: Option<Box<[u8]>>,
    active_stream: Option<RuntimePresentationStream>,
    retained_display: Option<RetainedPresentationFrame>,
    next_stream_source_colors: Option<IndexedGamePalette>,
}

struct RetainedPresentationFrame {
    indexed_pixels: Box<[u8]>,
    source_colors: IndexedGamePalette,
    rgba: Box<[u8]>,
}

impl RetainedPresentationFrame {
    fn from_stream(stream: &RuntimePresentationStream) -> Self {
        Self {
            indexed_pixels: Box::from(stream.display_indices()),
            source_colors: *stream.display_palette(),
            rgba: Box::from(stream.display_rgba()),
        }
    }

    fn resolve_rgba(&mut self) -> Result<()> {
        self.rgba = indexed_frame_rgba(&self.indexed_pixels, &self.source_colors)
            .context("resolving the retained HNM page to true-color RGBA")?
            .into_boxed_slice();
        Ok(())
    }
}

impl RuntimePresentationPlayer {
    /// Clone executable-authored line templates into ordinary owned runtime state.
    pub fn new(initial: &BloodprgPresentationCatalog) -> Self {
        Self {
            catalog: RuntimePresentationCatalog::new(initial),
            shared_idle_video: None,
            active_stream: None,
            retained_display: None,
            next_stream_source_colors: None,
        }
    }

    /// Apply mutable location, object, and character names from one DESCRIPT record.
    pub fn apply_descript_assets(&mut self, assets: &DescriptPresentationAssets) -> Result<()> {
        self.catalog.apply_descript_assets(assets)?;
        if let Some(video) = assets.encoded_idle_video() {
            self.shared_idle_video = Some(Box::from(video));
        }
        Ok(())
    }

    /// Select the current line-2 video from a DESCRIPT sequence record.
    pub fn select_descript_sequence_video(&mut self, basename: &[u8]) -> Result<()> {
        self.catalog.select_sequence_video(basename)
    }

    /// Select the current line-6 hyperspace clip.
    pub fn select_hyperspace_video(&mut self, basename: &[u8]) -> Result<()> {
        self.catalog.select_hyperspace_video(basename)
    }

    /// Select the current line-7 video requested by BloodScript A8.
    pub fn select_script_sequence_video(&mut self, basename: &[u8]) -> Result<()> {
        self.catalog.select_script_sequence_video(basename)
    }

    /// Resolve, open, bootstrap, and retain one presentation resource stream.
    pub fn load(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        line: PresentationResourceId,
        source: PresentationSceneSource,
        policy: PresentationPresentPolicy,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<Option<PresentationResourceSequenceOutcome>> {
        // The DOS resource switch closes the current queue file before opening
        // every newly requested line, even when the prior line has not drained.
        // Its displayed page remains visible if the replacement cannot open.
        let source_colors = self
            .next_stream_source_colors
            .or_else(|| self.display_palette().copied())
            .unwrap_or(*runtime.live_palette());
        self.finish();
        let mut request = self
            .catalog
            .request(line)
            .with_context(|| format!("resolving presentation line {}", line.get()))?;
        if source == PresentationSceneSource::SharedCache {
            request.shared_source = Some(
                self.shared_idle_video
                    .clone()
                    .context("presentation line 8 selected an unavailable DESCRIPT idle cache")?,
            );
        } else if !runtime
            .data()
            .resource_store()
            .resource_exists(&request.resource_name)?
        {
            return Ok(None);
        }
        request.present_policy = policy;
        request.entry_policy.draw_via_back_buffer = policy.draw_via_back_buffer;
        request.entry_policy.skip_back_buffer_present = policy.skip_back_buffer_present;
        let (stream, outcome) = RuntimePresentationStream::load_with_source_colors(
            runtime,
            request,
            source_colors,
            timer_tick,
            render_snapshot_suppressed,
        )?;
        self.next_stream_source_colors = None;
        self.retained_display = None;
        self.active_stream = Some(stream);
        Ok(Some(outcome))
    }

    /// Advance the active HNM queue using explicit host audio and timer positions.
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

    /// Service one queue frame from the caller's recovered link cursor.
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
        let Some(stream) = self.active_stream.as_mut() else {
            return Ok(RuntimePresentationStepOutcome {
                queue: crate::native::bloodprg::PresentationQueueServiceOutcome::SourceUnavailable,
                stream_finished: true,
                queue_metrics: super::RuntimePresentationQueueMetrics::default(),
            });
        };
        match link_target {
            Some(link_target) => stream.service_frame_from_link_target(
                runtime,
                link_target,
                audio_position,
                timer_tick,
                clock_gates,
                render_snapshot_suppressed,
            ),
            None => stream.service_frame(
                runtime,
                audio_position,
                timer_tick,
                clock_gates,
                render_snapshot_suppressed,
            ),
        }
    }

    /// Return whether a stream remains owned, including its final drained state.
    pub const fn has_stream(&self) -> bool {
        self.active_stream.is_some()
    }

    /// Return whether an HNM page still owns the visible indexed surface.
    ///
    /// The native display keeps the final decoded page after the resource queue
    /// closes. Modern rendering must retain that page's already-resolved colors
    /// until bridge, scene, or panel drawing explicitly replaces it.
    pub const fn owns_display_frame(&self) -> bool {
        self.active_stream.is_some() || self.retained_display.is_some()
    }

    /// Return legacy source colors used only to resolve the current HNM page.
    pub fn display_palette(&self) -> Option<&IndexedGamePalette> {
        self.active_stream
            .as_ref()
            .map(RuntimePresentationStream::display_palette)
            .or_else(|| {
                self.retained_display
                    .as_ref()
                    .map(|frame| &frame.source_colors)
            })
    }

    /// Mutably borrow decoder-local colors for a recovered video fade.
    pub fn display_palette_mut(&mut self) -> Option<&mut IndexedGamePalette> {
        self.active_stream
            .as_mut()
            .map(RuntimePresentationStream::display_palette_mut)
            .or_else(|| {
                self.retained_display
                    .as_mut()
                    .map(|frame| &mut frame.source_colors)
            })
    }

    /// Return the true-color HNM page that currently owns the display.
    pub fn display_rgba(&self) -> Option<&[u8]> {
        self.active_stream
            .as_ref()
            .map(RuntimePresentationStream::display_rgba)
            .or_else(|| {
                self.retained_display
                    .as_ref()
                    .map(|frame| frame.rgba.as_ref())
            })
    }

    /// Re-resolve the current video page after subtitles or a local color fade.
    pub fn refresh_display_rgba(&mut self, active_indexed_pixels: &[u8]) -> Result<()> {
        if let Some(stream) = self.active_stream.as_mut() {
            stream.resolve_display_rgba(active_indexed_pixels)
        } else if let Some(frame) = self.retained_display.as_mut() {
            frame.resolve_rgba()
        } else {
            Ok(())
        }
    }

    /// Stage the color mapping established by a new scene background.
    ///
    /// The background is decoded into the back page while the prior RGBA video
    /// remains visible. Its colors nevertheless become the initial decoder
    /// mapping when the next HNM presents that page.
    pub fn stage_next_stream_source_colors(&mut self, colors: IndexedGamePalette) {
        self.next_stream_source_colors = Some(colors);
    }

    /// Release a completed HNM page after another recovered display owner writes.
    pub fn release_retained_display_frame(&mut self) -> bool {
        self.retained_display.take().is_some()
    }

    /// Return whether the recovered queue status still owns or drains a source.
    pub fn source_open_or_draining(&self) -> bool {
        self.active_stream
            .as_ref()
            .is_some_and(RuntimePresentationStream::source_open_or_draining)
    }

    /// Number of decoded frames retired by the active stream.
    pub fn decoded_frame_count(&self) -> u64 {
        self.active_stream
            .as_ref()
            .map_or(u64::MIN, RuntimePresentationStream::presented_frame_count)
    }

    /// Borrow the authored DOS resource identity owned by the active stream.
    pub(crate) fn active_resource_name(
        &self,
    ) -> Option<&commander_blood_formats::archive::BloodResourceName> {
        self.active_stream
            .as_ref()
            .map(RuntimePresentationStream::resource_name)
    }

    #[cfg(test)]
    fn active_source_lease(&self) -> Option<crate::native::bloodprg::PresentationSourceLease> {
        self.active_stream
            .as_ref()
            .map(RuntimePresentationStream::source_lease)
    }

    /// Copy the transition-source snapshot retained by the active stream.
    pub fn render_palette_snapshot(&self) -> Option<IndexedGamePalette> {
        self.active_stream
            .as_ref()
            .map(|stream| *stream.render_palette_snapshot())
    }

    /// Snapshot the queue counters shared with scene and subtitle dispatch.
    pub fn queue_metrics(&self) -> Result<Option<super::RuntimePresentationQueueMetrics>> {
        self.active_stream
            .as_ref()
            .map(RuntimePresentationStream::queue_metrics)
            .transpose()
    }

    /// Release the active stream after completion or explicit cancellation.
    pub fn finish(&mut self) -> bool {
        let Some(stream) = self.active_stream.take() else {
            return false;
        };
        if stream.presented_frame_count() != u64::MIN {
            self.retained_display = Some(RetainedPresentationFrame::from_stream(&stream));
        }
        true
    }

    /// Snapshot the active stream cursor used by the native Escape handler.
    pub(crate) fn cancellation_cursor(&self) -> Option<PresentationResourceCursor> {
        self.active_stream
            .as_ref()
            .and_then(RuntimePresentationStream::cancellation_cursor)
    }

    /// Apply the cursor rewritten by the native Escape handler.
    pub(crate) fn apply_cancellation_cursor(
        &mut self,
        cursor: PresentationResourceCursor,
    ) -> Result<()> {
        self.active_stream
            .as_mut()
            .context("presentation cancellation has no active stream")?
            .apply_cancellation_cursor(cursor)
    }

    /// Borrow the mutable presentation-line catalog for recovered coordinators.
    pub const fn catalog(&self) -> &RuntimePresentationCatalog {
        &self.catalog
    }
}

impl InputCancellationBackend for RuntimePresentationPlayer {
    fn reset_presentation_queue(&mut self) {
        if let Some(stream) = &mut self.active_stream {
            stream.reset_presentation_queue();
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::native::bloodprg::{
        InputCancellationOutcome, InputCancellationState, InputDispatchState, ScriptClock,
        TextPresentationState, cancel_input_action,
    };
    use crate::runtime::{OriginalGameData, OriginalGameDataPaths, RuntimeScriptBackend};

    use super::*;

    const OPENING_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(u16::MIN);
    const SCENE_DESCRIPTION_PRESENTATION_LINE: PresentationResourceId =
        PresentationResourceId::new(41);
    const SCENE_FADE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(39);
    const DESCRIPT_SEQUENCE_PRESENTATION_LINE: PresentationResourceId =
        PresentationResourceId::new(2);
    const CHARACTER_IDLE_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(8);
    const INITIAL_DECODED_FRAME_COUNT: u64 = 1;
    const INHERITED_COLOR_INDEX: usize = 250;
    const INHERITED_VIDEO_COLOR: [u8; 3] = [5, 7, 11];
    const STAGED_SCENE_COLOR: [u8; 3] = [17, 19, 23];

    #[test]
    fn opening_line_runs_through_the_catalog_and_flat_stream() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        let policy = PresentationPresentPolicy {
            skip_back_buffer_present: true,
            unclamped_rows: true,
            ..PresentationPresentPolicy::default()
        };

        let initial = player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                policy,
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();
        assert!(initial.initial_present.frame_presented);
        assert!(player.has_stream());
        assert!(player.owns_display_frame());
        assert!(player.source_open_or_draining());
        assert_eq!(player.decoded_frame_count(), INITIAL_DECODED_FRAME_COUNT);
        assert_eq!(
            player
                .catalog()
                .resource_name(OPENING_PRESENTATION_LINE)
                .unwrap()
                .as_bytes(),
            b"sq\\mind.HNM"
        );
        let retained_rgba = player.display_rgba().unwrap().to_vec();
        let runtime_colors = *runtime.live_palette();
        assert!(player.finish());
        assert!(!player.has_stream());
        assert!(!player.source_open_or_draining());
        assert!(player.owns_display_frame());
        assert_eq!(player.display_rgba(), Some(retained_rgba.as_slice()));
        player.display_palette_mut().unwrap().fill([63, 0, 0]);
        player.refresh_display_rgba(&[]).unwrap();
        assert_ne!(player.display_rgba(), Some(retained_rgba.as_slice()));
        assert_eq!(
            runtime.live_palette(),
            &runtime_colors,
            "a retained HNM color transition contaminated flat game colors"
        );
        assert!(player.release_retained_display_frame());
        assert!(!player.owns_display_frame());
        assert!(player.display_rgba().is_none());
    }

    #[test]
    fn loading_another_line_replaces_the_active_stream() {
        let Some(data) = original_data() else {
            return;
        };
        let mut backend = RuntimeScriptBackend::new(
            &data,
            ScriptClock {
                hour: 12,
                day: 1,
                month: 1,
            },
        );
        backend
            .apply_description(b"Bob_Morlock", true, &mut TextPresentationState::default())
            .unwrap()
            .unwrap();
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        player.apply_descript_assets(backend.assets()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        player
            .load(
                &mut runtime,
                SCENE_DESCRIPTION_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();
        let description_resource = player.active_resource_name().unwrap().clone();
        assert_eq!(player.decoded_frame_count(), INITIAL_DECODED_FRAME_COUNT);

        player
            .load(
                &mut runtime,
                SCENE_FADE_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();

        assert!(player.has_stream());
        assert_eq!(player.decoded_frame_count(), INITIAL_DECODED_FRAME_COUNT);
        assert_ne!(player.active_resource_name(), Some(&description_resource));
    }

    #[test]
    fn replacement_stream_prefers_new_scene_colors_over_preceding_video_colors() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        let game_colors = *runtime.live_palette();
        let mut preceding_colors = game_colors;
        preceding_colors[INHERITED_COLOR_INDEX] = INHERITED_VIDEO_COLOR;
        let request = super::super::RuntimePresentationRequest::new(
            commander_blood_formats::archive::BloodResourceName::new(b"PL\\PTERRA10.HNM").unwrap(),
        );
        let (preceding_stream, _) = RuntimePresentationStream::load_with_source_colors(
            &mut runtime,
            request,
            preceding_colors,
            u16::MIN,
            false,
        )
        .unwrap();
        assert_eq!(
            preceding_stream.display_palette()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR
        );
        player.active_stream = Some(preceding_stream);

        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            player.display_palette().unwrap()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR,
            "a chained HNM stream lost the color state owned by its predecessor"
        );
        assert_eq!(
            runtime.live_palette(),
            &game_colors,
            "chained HNM colors escaped into flat game rendering"
        );

        let mut scene_colors = game_colors;
        scene_colors[INHERITED_COLOR_INDEX] = STAGED_SCENE_COLOR;
        player.stage_next_stream_source_colors(scene_colors);
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            player.display_palette().unwrap()[INHERITED_COLOR_INDEX],
            STAGED_SCENE_COLOR,
            "a new scene inherited the preceding HNM's colors instead of its decoded background"
        );
        assert!(player.next_stream_source_colors.is_none());
        assert_eq!(
            runtime.live_palette(),
            &game_colors,
            "staged HNM source colors contaminated flat game rendering"
        );
    }

    #[test]
    fn line_eight_uses_the_persistent_descript_idle_cache() {
        let Some(data) = original_data() else {
            return;
        };
        let mut backend = RuntimeScriptBackend::new(
            &data,
            ScriptClock {
                hour: 12,
                day: 1,
                month: 1,
            },
        );
        backend
            .apply_description(b"Izwalito", false, &mut TextPresentationState::default())
            .unwrap()
            .unwrap();
        assert!(backend.assets().encoded_idle_video().is_some());

        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        player.apply_descript_assets(backend.assets()).unwrap();
        let mut runtime = OriginalGameRuntime::new(data);
        player
            .load(
                &mut runtime,
                CHARACTER_IDLE_PRESENTATION_LINE,
                PresentationSceneSource::SharedCache,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            player.active_source_lease(),
            Some(crate::native::bloodprg::PresentationSourceLease::SharedCache)
        );
        assert!(player.source_open_or_draining());
    }

    #[test]
    fn missing_authored_hnm_uses_the_native_unavailable_source_path() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        player
            .select_descript_sequence_video(b"puven1.hnm")
            .unwrap();
        let mut runtime = OriginalGameRuntime::new(data);

        let load = player
            .load(
                &mut runtime,
                DESCRIPT_SEQUENCE_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap();
        assert!(load.is_none());
        assert!(!player.has_stream());

        let service = player
            .service_frame(
                &mut runtime,
                u16::MIN,
                u16::MIN,
                PresentationQueueClockGates::default(),
                false,
            )
            .unwrap();
        assert_eq!(
            service.queue,
            crate::native::bloodprg::PresentationQueueServiceOutcome::SourceUnavailable
        );
        assert!(service.stream_finished);
    }

    #[test]
    fn unresolved_dynamic_line_uses_the_native_unavailable_source_path() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);

        let load = player
            .load(
                &mut runtime,
                DESCRIPT_SEQUENCE_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap();

        assert!(load.is_none());
        assert!(!player.has_stream());
        assert_eq!(
            player
                .catalog()
                .resource_name(DESCRIPT_SEQUENCE_PRESENTATION_LINE)
                .unwrap()
                .as_bytes(),
            b"sq\\xxxxxxxxxxxx"
        );
    }

    #[test]
    fn cancelled_real_stream_rewinds_and_resumes_without_stalling() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap()
            .unwrap();
        player
            .service_frame(
                &mut runtime,
                u16::MIN,
                1,
                PresentationQueueClockGates::default(),
                false,
            )
            .unwrap();

        let mut cancellation = InputCancellationState {
            presentation_active: true,
            dialogue_ready: false,
            ship_active: false,
            active_line: 2,
            resources: player.cancellation_cursor().unwrap(),
            scene_palette: *runtime.live_palette(),
            palette_dirty: false,
        };
        let rewind = cancellation.resources.rewind_position;
        let mut dispatch = InputDispatchState::default();

        assert_eq!(
            cancel_input_action(&mut dispatch, &mut cancellation, &mut player, 27),
            InputCancellationOutcome::CancelledPresentation
        );
        player
            .apply_cancellation_cursor(cancellation.resources)
            .unwrap();
        assert_eq!(player.cancellation_cursor().unwrap().read_position, rewind);
        player
            .service_frame(
                &mut runtime,
                u16::MIN,
                2,
                PresentationQueueClockGates::default(),
                false,
            )
            .unwrap();
        assert!(player.has_stream());
    }

    fn original_data() -> Option<OriginalGameData> {
        let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let roots = [
            workspace_root.join("output/_tmp_iso"),
            workspace_root.join("accuracy/cblood_install/cblood"),
        ];
        roots.into_iter().find_map(|root| {
            OriginalGameDataPaths::from_root(root)
                .ok()
                .and_then(|paths| {
                    OriginalGameData::load_with_writable_root(paths, temporary_root()).ok()
                })
        })
    }

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!(
            "commander-blood-presentation-player-test-{}",
            std::process::id()
        ))
    }
}
