//! Runtime ownership for presentation-line resolution and HNM queue playback.

use anyhow::{Context, Result};
use commander_blood_formats::bloodprg::BloodprgPresentationCatalog;

use super::presentation_rgb::RgbVideoPage;
use crate::native::bloodprg::{
    DescriptPresentationAssets, IndexedGamePalette, InputCancellationBackend,
    PresentationPresentPolicy, PresentationQueueClockGates, PresentationResourceCursor,
    PresentationResourceId, PresentationResourceSequenceOutcome, PresentationSceneSource,
    clear_scene_palette_entries,
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
    background_rgba: Option<RgbVideoPage>,
    contact_background_rgb: Option<RgbVideoPage>,
    background_indices: Option<Box<[u8]>>,
    background_prepared: bool,
    display_occludes_manu3: bool,
}

struct RetainedPresentationFrame {
    indexed_pixels: Box<[u8]>,
    source_colors: IndexedGamePalette,
    rgb: RgbVideoPage,
}

impl RetainedPresentationFrame {
    fn from_stream(stream: &RuntimePresentationStream) -> Self {
        Self {
            indexed_pixels: Box::from(stream.display_indices()),
            source_colors: *stream.display_palette(),
            rgb: stream.rgb_display().clone(),
        }
    }

    fn resolve_rgba(&mut self) -> Result<()> {
        self.rgb
            .resolve_video(&self.indexed_pixels, &self.source_colors)
    }

    fn clear_scene_source_colors(&mut self) -> Result<()> {
        clear_scene_palette_entries(&mut self.source_colors);
        self.resolve_rgba()
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
            background_rgba: None,
            contact_background_rgb: None,
            background_indices: None,
            background_prepared: false,
            display_occludes_manu3: false,
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
        occludes_manu3: bool,
    ) -> Result<Option<PresentationResourceSequenceOutcome>> {
        // The DOS resource switch closes the current queue file before opening
        // every newly requested line, even when the prior line has not drained.
        // Its displayed page remains visible if the replacement cannot open.
        let source_colors = self
            .next_stream_source_colors
            .or_else(|| self.display_palette().copied())
            .unwrap_or(*runtime.live_palette());
        let display_rgb = self
            .active_stream
            .as_ref()
            .map(|stream| stream.rgb_display().inherited())
            .or_else(|| {
                self.retained_display
                    .as_ref()
                    .map(|frame| frame.rgb.inherited())
            });
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
        if !self.background_prepared {
            let (_, back) = runtime.presentation_buffers_mut();
            // Unmigrated bridge/navigation producers can replace the shared
            // back page. Their writes must not reuse an unrelated RGB cache.
            if self.background_indices.as_deref() != Some(&*back) {
                self.background_rgba = None;
                self.background_indices = None;
            }
        }
        let back_rgb = self.background_rgba.as_ref().map(RgbVideoPage::inherited);
        let (stream, outcome) = RuntimePresentationStream::load_with_rgb_pages(
            runtime,
            request,
            source_colors,
            display_rgb,
            back_rgb,
            timer_tick,
            render_snapshot_suppressed,
        )?;
        self.next_stream_source_colors = None;
        self.background_prepared = false;
        self.retained_display = None;
        self.display_occludes_manu3 = occludes_manu3;
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

    /// Return whether the retained true-color page replaces MANU3's display page.
    pub const fn display_occludes_manu3(&self) -> bool {
        self.owns_display_frame() && self.display_occludes_manu3
    }

    /// Retain full-screen ownership discovered after the stream was opened.
    pub fn latch_display_occludes_manu3(&mut self) {
        if self.owns_display_frame() {
            self.display_occludes_manu3 = true;
        }
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
                    .map(|frame| frame.rgb.pixels.as_ref())
            })
    }

    /// Re-resolve owned video pixels after a local color fade.
    pub fn refresh_display_rgba(&mut self) -> Result<()> {
        if let Some(stream) = self.active_stream.as_mut() {
            stream.resolve_retained_display_rgba()
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

    /// Import PBM artwork with its own complete color map, preserving zero coverage.
    pub(super) fn stage_background_rgb(
        &mut self,
        encoded: &[u8],
        fallback: &[u8],
        transparent_zero: bool,
    ) -> Result<()> {
        use crate::native::bloodprg::{
            PbmDecodeOptions, PbmPaletteUpdate, PbmTransparency, decode_pbm_image,
        };
        let mut indices = vec![0; 320 * 200];
        let mut colors = [[0; 3]; 256];
        decode_pbm_image(
            encoded,
            &mut indices,
            &mut colors,
            PbmDecodeOptions {
                palette_update: PbmPaletteUpdate::AllColors,
                transparency: PbmTransparency::Opaque,
            },
        )?;
        let imported = indexed_frame_rgba(&indices, &colors)?;
        let mut background = self
            .background_rgba
            .take()
            .unwrap_or_else(|| RgbVideoPage::new(Box::from(fallback)));
        background.import_artwork(&imported, &indices, transparent_zero);
        self.contact_background_rgb = None;
        self.background_rgba = Some(background);
        self.background_prepared = true;
        Ok(())
    }

    pub(super) fn stage_contact_background_rgb(
        &mut self,
        encoded: &[u8],
        fallback: &[u8],
        transparent_zero: bool,
        new_contact: bool,
    ) -> Result<()> {
        if new_contact || self.contact_background_rgb.is_none() {
            self.stage_background_rgb(encoded, fallback, transparent_zero)?;
            self.contact_background_rgb = self.background_rgba.clone();
        } else {
            // The native interlude return reloads FRIGO while preserving the
            // current DAC. Modern rendering restores the already resolved art.
            self.background_rgba = self.contact_background_rgb.clone();
            self.background_prepared = true;
        }
        Ok(())
    }

    pub(super) fn clear_background_rgb(&mut self, rows: std::ops::Range<usize>, rgb: [u8; 4]) {
        if let Some(background) = self.background_rgba.as_mut() {
            background.clear_rows(rows.clone(), rgb);
            self.background_prepared = true;
        }
        if let Some(background) = self.contact_background_rgb.as_mut() {
            background.clear_rows(rows, rgb);
        }
    }

    pub(super) fn present_background_rgb(&mut self, indices: &[u8], colors: IndexedGamePalette) {
        self.finish();
        if let Some(background) = &self.background_rgba {
            self.retained_display = Some(RetainedPresentationFrame {
                indexed_pixels: Box::from(indices),
                source_colors: self.next_stream_source_colors.unwrap_or(colors),
                rgb: background.clone(),
            });
            self.display_occludes_manu3 = false;
        }
    }

    pub(super) fn darken_background_rgb(&mut self, amount: u8) {
        if let Some(background) = self.contact_background_rgb.as_mut() {
            background.darken_artwork(amount);
        }
        if let Some(background) = self.background_rgba.as_mut() {
            background.darken_artwork(amount);
        }
        if let Some(stream) = self.active_stream.as_mut() {
            stream.darken_background_rgb(amount);
        }
        if let Some(frame) = self.retained_display.as_mut() {
            frame.rgb.darken_artwork(amount);
        }
    }

    /// Broadcast the native shared low-color clear to every video color owner.
    pub fn clear_scene_source_colors(&mut self) -> Result<()> {
        if let Some(stream) = self.active_stream.as_mut() {
            stream.clear_scene_source_colors()?;
        }
        if let Some(frame) = self.retained_display.as_mut() {
            frame.clear_scene_source_colors()?;
        }
        if let Some(colors) = self.next_stream_source_colors.as_mut() {
            clear_scene_palette_entries(colors);
        }
        self.darken_background_rgb(u8::MAX);
        Ok(())
    }

    /// Release a completed HNM page after another recovered display owner writes.
    pub fn release_retained_display_frame(&mut self) -> bool {
        let released = self.retained_display.take().is_some();
        if released {
            self.display_occludes_manu3 = false;
        }
        released
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
        // A prepared PBM may already own the next back page. Do not replace it
        // with the preceding stream's older snapshot during a scene switch.
        if !self.background_prepared {
            self.background_rgba = Some(stream.rgb_back().inherited());
            self.background_indices = Some(Box::from(stream.back_indices()));
        }
        if stream.presented_frame_count() != u64::MIN {
            self.retained_display = Some(RetainedPresentationFrame::from_stream(&stream));
        } else {
            self.display_occludes_manu3 = false;
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
    const SCENE_COLOR_INDEX: usize = 100;
    const FIRST_PRESERVED_COLOR_INDEX: usize =
        crate::native::bloodprg::SCENE_PALETTE_CLEAR_COLOR_COUNT;
    const INHERITED_COLOR_INDEX: usize = 250;
    const INHERITED_VIDEO_COLOR: [u8; 3] = [5, 7, 11];
    const STAGED_SCENE_COLOR: [u8; 3] = [17, 19, 23];

    #[test]
    #[ignore = "requires the original Big Bug Bang assets"]
    fn tempest_landing_colors_do_not_depend_on_the_preceding_scene() {
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/imported-assets");
        let mut reference = Vec::new();
        for (pass, previous_colors) in [[[63, 0, 0]; 256], [[0, 0, 63]; 256]]
            .into_iter()
            .enumerate()
        {
            let data = OriginalGameData::load_with_writable_root(
                OriginalGameDataPaths::from_root(&root).unwrap(),
                temporary_root(),
            )
            .unwrap();
            let mut backend = RuntimeScriptBackend::new(
                &data,
                ScriptClock {
                    hour: 12,
                    day: 1,
                    month: 1,
                },
            );
            backend
                .apply_description(b"Templand", true, &mut TextPresentationState::default())
                .unwrap()
                .unwrap();
            let slot =
                commander_blood_formats::descript::DescriptBackgroundSlot::decode(1).unwrap();
            let encoded = backend
                .backgrounds()
                .get(slot)
                .unwrap()
                .encoded_image()
                .to_vec();
            let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
            player.apply_descript_assets(backend.assets()).unwrap();
            let mut runtime = OriginalGameRuntime::new(data);
            *runtime.live_palette_mut() = previous_colors;
            let mut scene_colors = previous_colors;
            crate::native::bloodprg::decode_pbm_image(
                &encoded,
                runtime.presentation_buffers_mut().1,
                &mut scene_colors,
                crate::native::bloodprg::PbmDecodeOptions {
                    palette_update: crate::native::bloodprg::PbmPaletteUpdate::SceneColors,
                    transparency: crate::native::bloodprg::PbmTransparency::TransparentZero,
                },
            )
            .unwrap();
            player
                .stage_background_rgb(&encoded, &vec![0; 320 * 200 * 4], true)
                .unwrap();
            player.stage_next_stream_source_colors(scene_colors);
            let policy = PresentationPresentPolicy::for_presentation_line(
                3,
                player.catalog.unclamped_line_ids(),
                35,
            )
            .0;
            player
                .load(
                    &mut runtime,
                    PresentationResourceId::new(3),
                    PresentationSceneSource::Owned,
                    policy,
                    0,
                    false,
                    true,
                )
                .unwrap()
                .unwrap();
            for tick in 1..2000 {
                let outcome = player
                    .service_frame(
                        &mut runtime,
                        tick,
                        tick,
                        PresentationQueueClockGates::default(),
                        false,
                    )
                    .unwrap();
                let pixels = &player.active_stream.as_ref().unwrap().rgb_display().pixels
                    [35 * 320 * 4..165 * 320 * 4];
                if pass == 0 {
                    reference.push(pixels.to_vec());
                } else {
                    assert!(
                        pixels == reference[usize::from(tick) - 1],
                        "Tempest RGB differs at tick {tick}"
                    );
                }
                if outcome.stream_finished {
                    assert!(tick > 1);
                    break;
                }
                assert!(tick < 1999, "Tempest clip did not finish");
            }
        }
    }

    #[test]
    #[ignore = "requires the original Big Bug Bang assets"]
    fn returning_to_daddy_preserves_background_through_player_lifecycle() {
        use crate::native::bloodprg::{
            PbmDecodeOptions, PbmPaletteUpdate, PbmTransparency, decode_pbm_image,
        };
        let root =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../output/big-bug-bang/imported-assets");
        let paths = OriginalGameDataPaths::from_root(root).unwrap();
        let data = OriginalGameData::load_with_writable_root(paths, temporary_root()).unwrap();
        let encoded = data.load_named_resource(b"FRIGO.FD").unwrap();
        let mut backend = RuntimeScriptBackend::new(
            &data,
            ScriptClock {
                hour: 12,
                day: 1,
                month: 1,
            },
        );
        backend
            .apply_description(b"Daddy_Gluxx", true, &mut TextPresentationState::default())
            .unwrap()
            .unwrap();
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        player.apply_descript_assets(backend.assets()).unwrap();
        let unclamped = *player.catalog.unclamped_line_ids();
        let mut runtime = OriginalGameRuntime::new(data);
        player.select_script_sequence_video(b"ppit07.hnm").unwrap();
        player
            .load(
                &mut runtime,
                PresentationResourceId::new(7),
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                0,
                false,
                false,
            )
            .unwrap()
            .unwrap();
        let mut context = *player.display_palette().unwrap();
        player.finish();
        let (_, back) = runtime.presentation_buffers_mut();
        decode_pbm_image(
            &encoded,
            back,
            &mut context,
            PbmDecodeOptions {
                palette_update: PbmPaletteUpdate::AllColors,
                transparency: PbmTransparency::Opaque,
            },
        )
        .unwrap();
        let imported = indexed_frame_rgba(back, &context).unwrap();
        player
            .stage_contact_background_rgb(&encoded, &imported, false, true)
            .unwrap();
        player.darken_background_rgb(162);
        let mut checked_pixels = 0;
        for (line, sequence) in [
            (39, None),
            (8, None),
            (9, None),
            (8, None),
            (10, None),
            (7, Some(b"ppit07.hnm".as_slice())),
            (8, None),
            (7, Some(b"flitutr.hnm".as_slice())),
            (7, Some(b"ppit07.hnm".as_slice())),
            (8, None),
        ] {
            if let Some(sequence) = sequence {
                player.select_script_sequence_video(sequence).unwrap();
            }
            let policy = PresentationPresentPolicy::for_presentation_line(
                line,
                &unclamped,
                if line == 39 { 0 } else { 35 },
            )
            .0;
            player
                .load(
                    &mut runtime,
                    PresentationResourceId::new(line),
                    PresentationSceneSource::Owned,
                    policy,
                    0,
                    false,
                    false,
                )
                .unwrap()
                .unwrap();
            for tick in 1..10000 {
                let outcome = player
                    .service_frame(
                        &mut runtime,
                        tick,
                        tick,
                        PresentationQueueClockGates::default(),
                        false,
                    )
                    .unwrap();
                if line == 8 {
                    let stream = player.active_stream.as_ref().unwrap();
                    let mut expected =
                        indexed_frame_rgba(stream.display_indices(), &context).unwrap();
                    for pixel in expected.chunks_exact_mut(4) {
                        for component in &mut pixel[..3] {
                            *component = component.saturating_sub(162);
                        }
                    }
                    for (index, color) in stream.display_indices().iter().enumerate() {
                        if (128..192).contains(color) {
                            assert_eq!(
                                &stream.display_rgba()[index * 4..index * 4 + 4],
                                &expected[index * 4..index * 4 + 4],
                                "background pixel {index} tick {tick}"
                            );
                            checked_pixels += 1;
                        }
                    }
                }
                if outcome.stream_finished {
                    break;
                }
            }
            assert!(player.active_stream.as_ref().unwrap().is_finished());
            player.finish();
            if line == 7 {
                let mut changed_context = *player.display_palette().unwrap();
                let (_, back) = runtime.presentation_buffers_mut();
                decode_pbm_image(
                    &encoded,
                    back,
                    &mut changed_context,
                    PbmDecodeOptions {
                        palette_update: PbmPaletteUpdate::Preserve,
                        transparency: PbmTransparency::Opaque,
                    },
                )
                .unwrap();
                player
                    .stage_contact_background_rgb(&encoded, &imported, false, false)
                    .unwrap();
            }
        }
        assert!(
            checked_pixels > 1000,
            "did not exercise retained background pixels"
        );
    }

    #[test]
    fn an_external_back_page_replacement_invalidates_the_previous_rgb_cache() {
        let Some(data) = original_data() else {
            assert!(std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none());
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
                0,
                false,
                false,
            )
            .unwrap()
            .unwrap();
        player.finish();
        player.display_palette_mut().unwrap()[150] = [1, 23, 45];
        let expected = indexed_frame_rgba(&[150], player.display_palette().unwrap()).unwrap();
        let (_, back) = runtime.presentation_buffers_mut();
        back.fill(150);
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                0,
                false,
                false,
            )
            .unwrap()
            .unwrap();
        assert!(
            player
                .active_stream
                .as_ref()
                .unwrap()
                .back_rgba()
                .chunks_exact(4)
                .all(|pixel| pixel == expected)
        );
    }

    #[test]
    fn preparing_rgb_artwork_does_not_replace_the_display_or_get_lost_at_clip_close() {
        let Some(data) = original_data() else {
            assert!(std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none());
            return;
        };
        let encoded = data.load_named_resource(b"FRIGO.FD").unwrap();
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                0,
                false,
                false,
            )
            .unwrap()
            .unwrap();
        let displayed = player.display_rgba().unwrap().to_vec();
        player
            .stage_background_rgb(&encoded, &displayed, false)
            .unwrap();
        let prepared = player.background_rgba.as_ref().unwrap().pixels.clone();
        assert!(
            prepared
                .chunks_exact(4)
                .filter(|pixel| pixel[..3] != [0, 0, 0])
                .count()
                > 1000
        );
        assert!(player.display_rgba().unwrap() == displayed);
        assert!(player.finish());
        assert!(player.background_rgba.as_ref().unwrap().pixels == prepared);
        player.display_palette_mut().unwrap().fill([63, 0, 0]);
        player.refresh_display_rgba().unwrap();
        assert!(player.background_rgba.as_ref().unwrap().pixels == prepared);
        player.present_background_rgb(runtime.front_buffer().pixels(), *runtime.live_palette());
        assert!(player.display_rgba().unwrap() == prepared.as_ref());
        player.clear_background_rgb(35..165, [0, 0, 0, 255]);
        assert!(
            player.background_rgba.as_ref().unwrap().pixels[35 * 320 * 4..165 * 320 * 4]
                .chunks_exact(4)
                .all(|pixel| pixel == [0, 0, 0, 255])
        );
    }

    #[test]
    fn scruter_talking_clip_close_preserves_the_authored_final_frame() {
        let Some(data) = original_data() else {
            assert!(std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none());
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        let request = super::super::RuntimePresentationRequest::new(
            commander_blood_formats::archive::BloodResourceName::new(b"PE\\SCR02.HNM").unwrap(),
        );
        let (stream, _) = RuntimePresentationStream::load(&mut runtime, request, 0, false).unwrap();
        player.active_stream = Some(stream);
        let mut penultimate = None;
        for tick in 1..=1000 {
            if player.decoded_frame_count() == 30 {
                penultimate = Some(player.display_rgba().unwrap().to_vec());
            }
            let outcome = player
                .service_frame(
                    &mut runtime,
                    0,
                    tick,
                    PresentationQueueClockGates::default(),
                    false,
                )
                .unwrap();
            if outcome.stream_finished {
                break;
            }
        }
        assert!(player.active_stream.as_ref().unwrap().is_finished());
        assert_eq!(player.decoded_frame_count(), 31);
        let final_frame = player.display_rgba().unwrap().to_vec();
        assert_ne!(penultimate.unwrap(), final_frame);
        let final_palette = *player.display_palette().unwrap();
        assert!(player.finish());
        runtime.front_buffer_mut().clear(255);
        player.refresh_display_rgba().unwrap();
        assert_eq!(player.display_rgba().unwrap(), final_frame);
        assert_eq!(*player.display_palette().unwrap(), final_palette);
    }

    #[test]
    fn palette_refresh_does_not_recapture_the_shared_work_page() {
        let Some(data) = original_data() else {
            assert!(
                std::env::var_os("CBLOOD_REQUIRE_ACCURACY_TESTS").is_none(),
                "original assets required"
            );
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
                0,
                false,
                false,
            )
            .unwrap()
            .unwrap();
        let before = player.display_rgba().unwrap().to_vec();
        runtime.front_buffer_mut().clear(255);
        player.refresh_display_rgba().unwrap();
        assert!(
            player.display_rgba().unwrap() == before,
            "a color refresh replaced video pixels"
        );
    }

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
                true,
            )
            .unwrap()
            .unwrap();
        assert!(initial.initial_present.frame_presented);
        assert!(player.has_stream());
        assert!(player.owns_display_frame());
        assert!(player.display_occludes_manu3());
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
        assert!(player.display_occludes_manu3());
        assert_eq!(player.display_rgba(), Some(retained_rgba.as_slice()));
        player.display_palette_mut().unwrap().fill([63, 0, 0]);
        player.refresh_display_rgba().unwrap();
        assert_ne!(player.display_rgba(), Some(retained_rgba.as_slice()));
        assert_eq!(
            runtime.live_palette(),
            &runtime_colors,
            "a retained HNM color transition contaminated flat game colors"
        );
        assert!(player.release_retained_display_frame());
        assert!(!player.owns_display_frame());
        assert!(!player.display_occludes_manu3());
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
                false,
            )
            .unwrap()
            .unwrap();

        assert!(player.has_stream());
        assert_eq!(player.decoded_frame_count(), INITIAL_DECODED_FRAME_COUNT);
        assert_ne!(player.active_resource_name(), Some(&description_resource));
    }

    #[test]
    fn replacement_stream_merges_new_scene_colors_with_the_reserved_video_bank() {
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
        scene_colors[SCENE_COLOR_INDEX] = STAGED_SCENE_COLOR;
        let mut merged_colors = *player.display_palette().unwrap();
        merged_colors[..crate::native::bloodprg::SCENE_PALETTE_CLEAR_COLOR_COUNT].copy_from_slice(
            &scene_colors[..crate::native::bloodprg::SCENE_PALETTE_CLEAR_COLOR_COUNT],
        );
        player.stage_next_stream_source_colors(merged_colors);
        assert_eq!(
            player.next_stream_source_colors.unwrap()[SCENE_COLOR_INDEX],
            STAGED_SCENE_COLOR
        );
        assert_eq!(
            player.next_stream_source_colors.unwrap()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR
        );
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
                false,
            )
            .unwrap()
            .unwrap();

        assert_eq!(
            player.display_palette().unwrap()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR,
            "a limited PBM update discarded the preceding HNM's reserved color bank"
        );
        assert!(player.next_stream_source_colors.is_none());
        assert_eq!(
            runtime.live_palette(),
            &game_colors,
            "staged HNM source colors contaminated flat game rendering"
        );
    }

    #[test]
    fn native_scene_clear_updates_all_video_color_owners_without_global_state() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        let game_colors = *runtime.live_palette();
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationSceneSource::Owned,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
                false,
            )
            .unwrap()
            .unwrap();

        let indexed_pixels = player
            .active_stream
            .as_ref()
            .unwrap()
            .display_indices()
            .to_vec();
        let visible_scene_color_index = indexed_pixels
            .iter()
            .copied()
            .map(usize::from)
            .find(|index| (1..FIRST_PRESERVED_COLOR_INDEX).contains(index))
            .expect("the opening frame has no visible low scene color");
        let mut source_colors = *player.display_palette().unwrap();
        source_colors[visible_scene_color_index] = STAGED_SCENE_COLOR;
        source_colors[INHERITED_COLOR_INDEX] = INHERITED_VIDEO_COLOR;
        *player.display_palette_mut().unwrap() = source_colors;
        player.refresh_display_rgba().unwrap();
        let rgba_before_clear = player.display_rgba().unwrap().to_vec();
        player.stage_next_stream_source_colors(source_colors);

        player.clear_scene_source_colors().unwrap();

        assert!(
            player.display_palette().unwrap()[..FIRST_PRESERVED_COLOR_INDEX]
                .iter()
                .all(|color| *color == [u8::MIN; 3])
        );
        assert_eq!(
            player.display_palette().unwrap()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR
        );
        let staged_colors = player.next_stream_source_colors.unwrap();
        assert!(
            staged_colors[..FIRST_PRESERVED_COLOR_INDEX]
                .iter()
                .all(|color| *color == [u8::MIN; 3])
        );
        assert_eq!(staged_colors[INHERITED_COLOR_INDEX], INHERITED_VIDEO_COLOR);
        assert_ne!(player.display_rgba().unwrap(), rgba_before_clear);

        *player.display_palette_mut().unwrap() = source_colors;
        player.refresh_display_rgba().unwrap();
        assert!(player.finish());
        let retained_rgba_before_clear = player.display_rgba().unwrap().to_vec();
        player.stage_next_stream_source_colors(source_colors);
        player.clear_scene_source_colors().unwrap();
        assert!(
            player.display_palette().unwrap()[..FIRST_PRESERVED_COLOR_INDEX]
                .iter()
                .all(|color| *color == [u8::MIN; 3])
        );
        assert_eq!(
            player.display_palette().unwrap()[INHERITED_COLOR_INDEX],
            INHERITED_VIDEO_COLOR
        );
        assert_ne!(player.display_rgba().unwrap(), retained_rgba_before_clear);
        assert_eq!(
            runtime.live_palette(),
            &game_colors,
            "decoder-local scene clearing contaminated renderer-owned game colors"
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
        static NEXT_ROOT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        std::env::temp_dir().join(format!(
            "commander-blood-presentation-player-test-{}-{}",
            std::process::id(),
            NEXT_ROOT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    }
}
