//! Runtime ownership for presentation-line resolution and HNM queue playback.

use anyhow::{Context, Result, bail};
use commander_blood_formats::bloodprg::BloodprgPresentationCatalog;

use crate::native::bloodprg::{
    DescriptPresentationAssets, IndexedGamePalette, PresentationPresentPolicy,
    PresentationResourceId, PresentationResourceSequenceOutcome,
};

use super::{
    OriginalGameRuntime, RuntimePresentationCatalog, RuntimePresentationStepOutcome,
    RuntimePresentationStream,
};

/// Flat catalog and active stream for one presentation-line consumer.
pub struct RuntimePresentationPlayer {
    catalog: RuntimePresentationCatalog,
    active_stream: Option<RuntimePresentationStream>,
}

impl RuntimePresentationPlayer {
    /// Clone executable-authored line templates into ordinary owned runtime state.
    pub fn new(initial: &BloodprgPresentationCatalog) -> Self {
        Self {
            catalog: RuntimePresentationCatalog::new(initial),
            active_stream: None,
        }
    }

    /// Apply mutable location, object, and character names from one DESCRIPT record.
    pub fn apply_descript_assets(&mut self, assets: &DescriptPresentationAssets) -> Result<()> {
        self.catalog.apply_descript_assets(assets)
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
        policy: PresentationPresentPolicy,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<PresentationResourceSequenceOutcome> {
        if self.active_stream.is_some() {
            bail!("a presentation stream is already active");
        }
        let mut request = self
            .catalog
            .request(line)
            .with_context(|| format!("resolving presentation line {}", line.get()))?;
        request.present_policy = policy;
        request.entry_policy.draw_via_back_buffer = policy.draw_via_back_buffer;
        request.entry_policy.skip_back_buffer_present = policy.skip_back_buffer_present;
        let (stream, outcome) = RuntimePresentationStream::load(
            runtime,
            request,
            timer_tick,
            render_snapshot_suppressed,
        )?;
        self.active_stream = Some(stream);
        Ok(outcome)
    }

    /// Advance the active HNM queue using explicit host audio and timer positions.
    pub fn service_frame(
        &mut self,
        runtime: &mut OriginalGameRuntime,
        audio_position: u16,
        timer_tick: u16,
        render_snapshot_suppressed: bool,
    ) -> Result<RuntimePresentationStepOutcome> {
        self.active_stream
            .as_mut()
            .context("no presentation stream is active")?
            .service_frame(
                runtime,
                audio_position,
                timer_tick,
                render_snapshot_suppressed,
            )
    }

    /// Return whether a stream remains owned, including its final drained state.
    pub const fn has_stream(&self) -> bool {
        self.active_stream.is_some()
    }

    /// Return whether the retained source and queue have both finished.
    pub fn is_finished(&self) -> bool {
        self.active_stream
            .as_ref()
            .is_none_or(RuntimePresentationStream::is_finished)
    }

    /// Number of decoded frames retired by the active stream.
    pub fn decoded_frame_count(&self) -> u64 {
        self.active_stream
            .as_ref()
            .map_or(u64::MIN, RuntimePresentationStream::presented_frame_count)
    }

    /// Copy the transition-source snapshot retained by the active stream.
    pub fn render_palette_snapshot(&self) -> Option<IndexedGamePalette> {
        self.active_stream
            .as_ref()
            .map(|stream| *stream.render_palette_snapshot())
    }

    /// Release the active stream after completion or explicit cancellation.
    pub fn finish(&mut self) -> Option<RuntimePresentationStream> {
        self.active_stream.take()
    }

    /// Borrow the mutable presentation-line catalog for recovered coordinators.
    pub const fn catalog(&self) -> &RuntimePresentationCatalog {
        &self.catalog
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use crate::runtime::{OriginalGameData, OriginalGameDataPaths};

    use super::*;

    const OPENING_PRESENTATION_LINE: PresentationResourceId = PresentationResourceId::new(u16::MIN);
    const INITIAL_DECODED_FRAME_COUNT: u64 = 1;

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
                policy,
                u16::MIN,
                false,
            )
            .unwrap();
        assert!(initial.initial_present.frame_presented);
        assert!(player.has_stream());
        assert_eq!(player.decoded_frame_count(), INITIAL_DECODED_FRAME_COUNT);
        assert_eq!(
            player
                .catalog()
                .resource_name(OPENING_PRESENTATION_LINE)
                .unwrap()
                .as_bytes(),
            b"sq\\mind.HNM"
        );
        assert!(player.finish().is_some());
        assert!(!player.has_stream());
    }

    #[test]
    fn active_stream_must_be_finished_before_another_line_is_loaded() {
        let Some(data) = original_data() else {
            return;
        };
        let mut player = RuntimePresentationPlayer::new(data.presentation_catalog());
        let mut runtime = OriginalGameRuntime::new(data);
        player
            .load(
                &mut runtime,
                OPENING_PRESENTATION_LINE,
                PresentationPresentPolicy::default(),
                u16::MIN,
                false,
            )
            .unwrap();

        assert!(
            player
                .load(
                    &mut runtime,
                    OPENING_PRESENTATION_LINE,
                    PresentationPresentPolicy::default(),
                    u16::MIN,
                    false,
                )
                .is_err()
        );
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
