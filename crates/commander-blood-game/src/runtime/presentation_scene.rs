//! Concrete original-data and framebuffer host for presentation scene dispatch.

use std::ops::Range;

use anyhow::{Context, Result};
use commander_blood_formats::bloodprg::BLOODPRG_PRESENTATION_LINE_COUNT;
use commander_blood_formats::descript::DescriptBackgroundSlot;
use commander_blood_formats::lbm::RGB_COMPONENT_COUNT;
use commander_blood_formats::script::ScriptObjectId;

use crate::native::bloodprg::{
    IndexedGamePalette, PaletteRemapTable, PbmDecodeOptions, PbmPaletteUpdate, PbmTransparency,
    PresentationPresentPolicy, PresentationQueueServiceOutcome, PresentationResourceId,
    PresentationSceneDescriptor, PresentationSceneDispatchContext, PresentationSceneDispatchHost,
    PresentationSceneDispatchOutcome, PresentationSceneDispatchState,
    PresentationSceneQueueService, PresentationSceneSource, ShipHudPaletteSnapshot,
    build_palette_blend_remap_table, decode_pbm_image, dispatch_presentation_scene,
    fill_back_buffer_band, fill_display_band,
};

use super::{ModernGameServices, RuntimePresentationBackground};

const TIMER_TICK_INCREMENT: u16 = 1;
const PRESENTATION_PALETTE_FIRST_COLOR: usize = 128;

/// Palette and timing state retained by every presentation-scene caller.
pub struct RuntimePresentationScene {
    scene_palette: IndexedGamePalette,
    presentation_palette: ShipHudPaletteSnapshot,
    blend_remap: PaletteRemapTable,
    timer_tick: u16,
}

impl RuntimePresentationScene {
    /// Initialize scene colors from the executable-authored startup palette.
    pub fn new(initial_palette: IndexedGamePalette) -> Self {
        Self {
            scene_palette: initial_palette,
            presentation_palette: [[u8::MIN; RGB_COMPONENT_COUNT];
                crate::native::bloodprg::SHIP_HUD_PALETTE_COLOR_COUNT],
            blend_remap: std::array::from_fn(|index| {
                u8::try_from(index).expect("the palette has exactly 256 entries")
            }),
            timer_tick: u16::MIN,
        }
    }

    /// Dispatch one recovered scene update through real resources and flat buffers.
    pub fn dispatch<'window>(
        &mut self,
        services: &mut ModernGameServices<'window>,
        state: &mut PresentationSceneDispatchState<DescriptBackgroundSlot>,
        active_record_related: Option<ScriptObjectId>,
        scruter_jo_record: Option<ScriptObjectId>,
        render_snapshot_suppressed: bool,
    ) -> Result<PresentationSceneDispatchOutcome> {
        let scenes = resolved_scene_descriptors(services);
        let unclamped_line_ids = *services.presentation_catalog().unclamped_line_ids();
        let shared_cache_available = services
            .script_backend()
            .assets()
            .encoded_idle_video()
            .is_some();
        let mut remap_request = None;
        let outcome = {
            let mut context = PresentationSceneDispatchContext {
                scenes: &scenes,
                active_record_related: active_record_related.as_ref(),
                scruter_jo_record: scruter_jo_record.as_ref(),
                unclamped_line_ids: &unclamped_line_ids,
                shared_cache_available,
                scene_palette: &mut self.scene_palette,
                presentation_palette: &mut self.presentation_palette,
            };
            let mut host = RuntimePresentationSceneHost {
                services,
                timer_tick: &mut self.timer_tick,
                remap_request: &mut remap_request,
                render_snapshot_suppressed,
            };
            dispatch_presentation_scene(state, &mut context, &mut host)
                .map_err(|error| anyhow::anyhow!("{error}"))?
        };
        if let Some((blend_percent, target)) = remap_request {
            build_palette_blend_remap_table(
                &self.scene_palette,
                &mut self.blend_remap,
                blend_percent,
                target,
            )
            .context("building the presentation scene palette remap")?;
        }
        Ok(outcome)
    }

    /// Current nearest-color table produced by scene transitions.
    pub const fn blend_remap(&self) -> &PaletteRemapTable {
        &self.blend_remap
    }

    /// Captured scene colors 128 through 191.
    pub const fn presentation_palette(&self) -> &ShipHudPaletteSnapshot {
        &self.presentation_palette
    }

    /// Synchronize the scene colors that alias the native live palette.
    pub fn set_scene_palette(&mut self, palette: IndexedGamePalette) {
        self.scene_palette = palette;
    }

    /// Synchronize the scene palette and its 128-through-191 presentation window.
    pub fn stage_navigation_palette(&mut self, palette: &IndexedGamePalette) {
        self.scene_palette = *palette;
        let end = PRESENTATION_PALETTE_FIRST_COLOR + self.presentation_palette.len();
        self.presentation_palette
            .copy_from_slice(&palette[PRESENTATION_PALETTE_FIRST_COLOR..end]);
    }
}

struct RuntimePresentationSceneHost<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    timer_tick: &'services mut u16,
    remap_request: &'services mut Option<(u8, [u8; RGB_COMPONENT_COUNT])>,
    render_snapshot_suppressed: bool,
}

impl PresentationSceneDispatchHost<DescriptBackgroundSlot>
    for RuntimePresentationSceneHost<'_, '_>
{
    type Error = anyhow::Error;

    fn load_scene_image(
        &mut self,
        image: &DescriptBackgroundSlot,
        scene_palette: &mut IndexedGamePalette,
    ) -> Result<()> {
        let encoded = self
            .services
            .script_backend()
            .backgrounds()
            .get(*image)
            .with_context(|| format!("DESCRIPT background slot {image:?} is not loaded"))?
            .encoded_image()
            .to_vec();
        let (_front, back) = self.services.runtime_mut().presentation_buffers_mut();
        decode_pbm_image(
            &encoded,
            back,
            scene_palette,
            PbmDecodeOptions {
                palette_update: PbmPaletteUpdate::SceneColors,
                transparency: PbmTransparency::TransparentZero,
            },
        )
        .context("decoding a DESCRIPT presentation background")?;
        Ok(())
    }

    fn clear_back_buffer_band(&mut self, rows: Range<usize>, color: u8) -> Result<()> {
        let (_front, back) = self.services.runtime_mut().presentation_buffers_mut();
        fill_back_buffer_band(back, rows.start, rows.end, color)
            .context("clearing the presentation background band")
    }

    fn load_presentation_sequence(
        &mut self,
        resource: PresentationResourceId,
        _source: PresentationSceneSource,
        policy: PresentationPresentPolicy,
    ) -> Result<bool> {
        let outcome = self.services.load_presentation_sequence(
            resource,
            policy,
            *self.timer_tick,
            self.render_snapshot_suppressed,
        )?;
        Ok(outcome.initial_present.frame_presented)
    }

    fn build_black_remap(
        &mut self,
        blend_percent: u8,
        target: [u8; RGB_COMPONENT_COUNT],
    ) -> Result<()> {
        *self.remap_request = Some((blend_percent, target));
        Ok(())
    }

    fn service_presentation_queue(
        &mut self,
        _policy: PresentationPresentPolicy,
    ) -> Result<PresentationSceneQueueService> {
        *self.timer_tick = self.timer_tick.wrapping_add(TIMER_TICK_INCREMENT);
        let audio_position = self
            .services
            .foreground_audio_position()?
            .unwrap_or(u64::MIN) as u16;
        let outcome = self.services.service_presentation_sequence(
            audio_position,
            *self.timer_tick,
            self.render_snapshot_suppressed,
        )?;
        Ok(PresentationSceneQueueService {
            frame_presented: queue_presented_frame(&outcome.queue),
            entry_metric: outcome.queue_metrics.entry_metric,
            read_wrap_index: outcome.queue_metrics.read_wrap_index,
        })
    }

    fn presentation_source_open_or_draining(&mut self) -> bool {
        let active = self.services.presentation_stream_active();
        if !active {
            self.services.finish_presentation_sequence();
        }
        active
    }

    fn clear_display_band(&mut self, rows: Range<usize>, color: u8) -> Result<()> {
        fill_display_band(
            self.services.runtime_mut().front_buffer_mut().pixels_mut(),
            rows.start,
            rows.end,
            color,
        )
        .context("clearing the active presentation band")
    }
}

fn resolved_scene_descriptors(
    services: &ModernGameServices<'_>,
) -> [PresentationSceneDescriptor<DescriptBackgroundSlot>; BLOODPRG_PRESENTATION_LINE_COUNT] {
    std::array::from_fn(|line| {
        let image = match services
            .presentation_catalog()
            .background(PresentationResourceId::new(line as u16))
        {
            Some(RuntimePresentationBackground::Cached(slot)) => Some(slot),
            Some(RuntimePresentationBackground::None) | None => None,
        };
        PresentationSceneDescriptor { image }
    })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_palette_remap_is_the_identity_table() {
        let palette = [[u8::MIN; RGB_COMPONENT_COUNT]; 256];
        let scene = RuntimePresentationScene::new(palette);
        for (index, mapped) in scene.blend_remap().iter().copied().enumerate() {
            assert_eq!(usize::from(mapped), index);
        }
    }
}
