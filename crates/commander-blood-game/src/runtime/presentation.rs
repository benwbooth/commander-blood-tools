//! Indexed-frame and modern 3D composition for the translated game lifecycle.

use anyhow::{Context, Result, bail};
use commander_blood_formats::alien::AlienAsset;
use sdl3::video::Window;

use crate::assets::OriginalFrame;
use crate::native::alien::AlienSceneFrame;
use crate::native::bloodprg::{BridgeSceneFrame, IndexedGamePalette};
use crate::native::manu3::raster::RenderTriangle;
use crate::render::{Renderer, indexed_frame_rgba, indexed_palette_rgba};

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT, LOGICAL_FRAMEBUFFER_WIDTH,
    OriginalGameRuntime,
};

/// Layer selection for one modern bridge presentation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum RuntimeBridgeComposition {
    /// Present the complete indexed framebuffer without the independently rendered bridge.
    IndexedFramebuffer,
    /// Present only the current wgpu bridge and MANU3 layers.
    BridgeScene,
    /// Present the wgpu bridge followed by indexed UI pixels and MANU3.
    BridgeSceneWithIndexedOverlay,
}

/// Color inputs for one complete indexed or already-resolved display page.
pub(super) struct RuntimeDisplayFrame<'frame> {
    pub indexed_palette: &'frame IndexedGamePalette,
    pub presentation_rgba: Option<&'frame [u8]>,
}

const RGBA_COMPONENT_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = u8::MAX;

/// SDL/wgpu presentation state for the original logical framebuffer and bridge.
pub struct RuntimePresentationHost<'window> {
    window: &'window Window,
    renderer: Option<Renderer<'window>>,
    presented_frame_count: u64,
    last_manu3_triangle_count: usize,
}

impl<'window> RuntimePresentationHost<'window> {
    /// Create the artwork-only renderer used by the loading screen.
    pub fn new_startup(window: &'window Window, runtime: &OriginalGameRuntime) -> Result<Self> {
        let initial_frame =
            runtime_original_frame(runtime.front_buffer().pixels(), runtime.live_palette())?;
        let renderer = Renderer::new(window, &initial_frame, None, None, None, None)
            .context("initializing startup wgpu presentation")?;
        Ok(Self {
            window,
            renderer: Some(renderer),
            presented_frame_count: u64::MIN,
            last_manu3_triangle_count: usize::MIN,
        })
    }

    /// Create the main-game renderer with bridge and optional MANU3 resources.
    pub fn new_main_game(window: &'window Window, runtime: &OriginalGameRuntime) -> Result<Self> {
        let renderer = main_game_renderer(window, runtime)?;
        Ok(Self {
            window,
            renderer: Some(renderer),
            presented_frame_count: u64::MIN,
            last_manu3_triangle_count: usize::MIN,
        })
    }

    /// Replace the startup renderer after MANU3 and bridge data are available.
    pub fn configure_main_game(&mut self, runtime: &OriginalGameRuntime) -> Result<()> {
        self.renderer = None;
        self.renderer = Some(main_game_renderer(self.window, runtime)?);
        Ok(())
    }

    /// Reconfigure the wgpu surface after an SDL pixel-size change.
    pub fn resize(&mut self, width: u32, height: u32) {
        if let Some(renderer) = &mut self.renderer {
            renderer.resize(width, height);
        }
    }

    /// Resolve and upload the runtime's complete indexed frame as true-color RGBA.
    pub fn submit_indexed_frame(
        &mut self,
        runtime: &OriginalGameRuntime,
        display_palette: &IndexedGamePalette,
    ) -> Result<()> {
        self.submit_frame(runtime.front_buffer(), display_palette)
    }

    /// Upload one already-resolved true-color logical display page.
    pub fn submit_rgba_frame(&mut self, rgba: &[u8]) -> Result<()> {
        self.renderer_ref()?
            .upload_rgba_frame(rgba)
            .context("uploading translated true-color game frame")
    }

    /// Upload one complete logical frame supplied by startup or presentation code.
    pub fn submit_frame(
        &mut self,
        frame: &super::IndexedFramebuffer,
        palette: &IndexedGamePalette,
    ) -> Result<()> {
        self.renderer_ref()?
            .upload_indexed_frame(frame.pixels(), palette)
            .context("uploading translated indexed game frame")
    }

    /// Upload only indexed pixels written after the recovered bridge base pass.
    fn submit_bridge_indexed_overlay(
        &mut self,
        runtime: &OriginalGameRuntime,
        bridge_frame: &BridgeSceneFrame,
        bridge_palette: &IndexedGamePalette,
    ) -> Result<()> {
        let rgba = bridge_indexed_overlay_rgba(
            runtime.front_buffer().pixels(),
            bridge_palette,
            bridge_frame,
        )?;
        self.renderer_ref()?
            .upload_rgba_frame(&rgba)
            .context("uploading sparse indexed bridge UI overlay")
    }

    /// Present indexed artwork without a 3D base scene.
    pub fn present_artwork(&mut self, manu3_triangles: &[RenderTriangle]) -> Result<()> {
        self.renderer_mut()?
            .render(manu3_triangles, None, None)
            .context("presenting translated artwork frame")?;
        self.last_manu3_triangle_count = manu3_triangles.len();
        self.presented_frame_count = self.presented_frame_count.wrapping_add(1);
        Ok(())
    }

    /// Present indexed artwork or one bridge frame, then composite MANU3.
    pub(super) fn present_frame(
        &mut self,
        runtime: &OriginalGameRuntime,
        bridge_frame: &BridgeSceneFrame,
        bridge_palette: &IndexedGamePalette,
        display: RuntimeDisplayFrame<'_>,
        composition: RuntimeBridgeComposition,
        manu3_visible: bool,
    ) -> Result<()> {
        // Frame-tail text and palette work occurs after the native chunky-copy
        // boundary, so refresh the modern texture immediately before drawing.
        match composition {
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay => {
                self.submit_bridge_indexed_overlay(runtime, bridge_frame, bridge_palette)?;
            }
            RuntimeBridgeComposition::IndexedFramebuffer
            | RuntimeBridgeComposition::BridgeScene => {
                if let Some(rgba) = display.presentation_rgba {
                    self.submit_rgba_frame(rgba)?;
                } else {
                    self.submit_indexed_frame(runtime, display.indexed_palette)?;
                }
            }
        }
        let all_manu3_triangles = runtime
            .manu3()
            .map(|model| model.render_triangles())
            .unwrap_or(&[]);
        let manu3_triangles = select_manu3_triangles(all_manu3_triangles, manu3_visible);
        let renderer = self.renderer_mut()?;
        renderer
            .update_bridge_palette(bridge_palette)
            .context("refreshing bridge colors from the retained bridge palette")?;
        match composition {
            RuntimeBridgeComposition::IndexedFramebuffer => {
                renderer.render(manu3_triangles, None, None)
            }
            RuntimeBridgeComposition::BridgeScene => {
                renderer.render(manu3_triangles, None, Some(bridge_frame))
            }
            RuntimeBridgeComposition::BridgeSceneWithIndexedOverlay => {
                renderer.render_with_indexed_overlay(manu3_triangles, None, Some(bridge_frame))
            }
        }
        .context("presenting translated game frame")?;
        self.last_manu3_triangle_count = manu3_triangles.len();
        self.presented_frame_count = self.presented_frame_count.wrapping_add(1);
        Ok(())
    }

    /// Upload immutable GPU resources for one decoded interactive alien scene.
    pub fn begin_alien_overlay(&mut self, asset: &AlienAsset) -> Result<()> {
        self.renderer_mut()?.install_alien_scene(asset);
        Ok(())
    }

    /// Present one full-screen alien frame without indexed UI or MANU3 layers.
    pub fn present_alien_overlay_frame(&mut self, frame: &AlienSceneFrame) -> Result<()> {
        self.renderer_mut()?
            .render(&[], Some(frame), None)
            .context("presenting translated alien-overlay frame")?;
        self.last_manu3_triangle_count = usize::MIN;
        self.presented_frame_count = self.presented_frame_count.wrapping_add(1);
        Ok(())
    }

    /// Release temporary alien GPU resources while retaining the bridge renderer.
    pub fn finish_alien_overlay(&mut self) -> bool {
        self.renderer
            .as_mut()
            .is_some_and(Renderer::remove_alien_scene)
    }

    /// Number of frames submitted to the window surface.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presented_frame_count
    }

    /// Number of MANU3 triangles submitted by the most recent GPU frame.
    pub const fn last_manu3_triangle_count(&self) -> usize {
        self.last_manu3_triangle_count
    }

    fn renderer_ref(&self) -> Result<&Renderer<'window>> {
        self.renderer
            .as_ref()
            .context("wgpu renderer is being reconfigured")
    }

    fn renderer_mut(&mut self) -> Result<&mut Renderer<'window>> {
        self.renderer
            .as_mut()
            .context("wgpu renderer is being reconfigured")
    }
}

fn select_manu3_triangles(triangles: &[RenderTriangle], visible: bool) -> &[RenderTriangle] {
    if visible { triangles } else { &[] }
}

fn main_game_renderer<'window>(
    window: &'window Window,
    runtime: &OriginalGameRuntime,
) -> Result<Renderer<'window>> {
    let initial_frame =
        runtime_original_frame(runtime.front_buffer().pixels(), runtime.live_palette())?;
    Renderer::new(
        window,
        &initial_frame,
        runtime.manu3(),
        Some(runtime.data().default_vga_palette()),
        None,
        Some(runtime.data().default_vga_palette()),
    )
    .context("initializing main-game wgpu presentation")
}

fn runtime_original_frame(
    indexed_pixels: &[u8],
    palette: &IndexedGamePalette,
) -> Result<OriginalFrame> {
    if indexed_pixels.len() != LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        bail!(
            "runtime frame has {} pixels; expected {LOGICAL_FRAMEBUFFER_PIXEL_COUNT}",
            indexed_pixels.len()
        );
    }
    Ok(OriginalFrame {
        width: u32::try_from(LOGICAL_FRAMEBUFFER_WIDTH)
            .context("logical framebuffer width exceeds u32")?,
        height: u32::try_from(LOGICAL_FRAMEBUFFER_HEIGHT)
            .context("logical framebuffer height exceeds u32")?,
        rgba: indexed_frame_rgba(indexed_pixels, palette)?,
        indexed_pixels: indexed_pixels.to_vec(),
        palette_rgba: indexed_palette_rgba(palette)?,
    })
}

fn bridge_indexed_overlay_rgba(
    indexed_pixels: &[u8],
    palette: &IndexedGamePalette,
    bridge_frame: &BridgeSceneFrame,
) -> Result<Vec<u8>> {
    if indexed_pixels.len() != LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        bail!(
            "runtime frame has {} pixels; expected {LOGICAL_FRAMEBUFFER_PIXEL_COUNT}",
            indexed_pixels.len()
        );
    }
    for (name, layer) in [
        (
            "projected-object",
            bridge_frame.object_sprite_pixels.as_ref(),
        ),
        ("panorama", bridge_frame.panorama_pixels.as_ref()),
        ("actor", bridge_frame.actor_sprite_pixels.as_ref()),
    ] {
        if layer.len() != LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
            bail!(
                "bridge {name} layer has {} pixels; expected {LOGICAL_FRAMEBUFFER_PIXEL_COUNT}",
                layer.len()
            );
        }
    }

    let mut bridge_base = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
    for star in &bridge_frame.starfield.plotted {
        bridge_base[star.framebuffer_index] = star.palette_index;
    }
    overlay_nonzero_indices(&mut bridge_base, &bridge_frame.object_sprite_pixels);
    overlay_nonzero_indices(&mut bridge_base, &bridge_frame.panorama_pixels);
    overlay_nonzero_indices(&mut bridge_base, &bridge_frame.actor_sprite_pixels);

    sparse_indexed_overlay_rgba(indexed_pixels, palette, &bridge_base)
}

fn sparse_indexed_overlay_rgba(
    indexed_pixels: &[u8],
    palette: &IndexedGamePalette,
    bridge_base: &[u8],
) -> Result<Vec<u8>> {
    if indexed_pixels.len() != bridge_base.len() {
        bail!(
            "runtime frame has {} pixels but its reconstructed bridge base has {}",
            indexed_pixels.len(),
            bridge_base.len()
        );
    }
    let colors = indexed_palette_rgba(palette)?;
    let mut rgba = vec![u8::MIN; indexed_pixels.len() * RGBA_COMPONENT_COUNT];
    for (index, (&presented, &base)) in indexed_pixels.iter().zip(bridge_base).enumerate() {
        if presented == base {
            continue;
        }
        let source = usize::from(presented) * RGBA_COMPONENT_COUNT;
        let destination = index * RGBA_COMPONENT_COUNT;
        rgba[destination..destination + RGBA_COMPONENT_COUNT]
            .copy_from_slice(&colors[source..source + RGBA_COMPONENT_COUNT]);
        rgba[destination + RGBA_COMPONENT_COUNT - 1] = OPAQUE_ALPHA;
    }
    Ok(rgba)
}

fn overlay_nonzero_indices(destination: &mut [u8], source: &[u8]) {
    for (destination, source) in destination.iter_mut().zip(source.iter().copied()) {
        if source != u8::MIN {
            *destination = source;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native::manu3::raster::RenderVertex;

    const TEST_PALETTE_INDEX: u8 = 19;
    const TEST_COLOR: [u8; 3] = [63, 32, 0];
    const EXPANDED_TEST_COLOR: [u8; 4] = [255, 129, 0, 255];
    const TEST_MANU3_TRIANGLE: RenderTriangle = RenderTriangle {
        source_face: 0,
        vertices: [RenderVertex {
            screen: [0, 0],
            texture: [0, 0],
            depth: 1,
        }; 3],
    };

    #[test]
    fn runtime_frame_preserves_indices_and_expands_the_native_palette() {
        let mut palette = [[u8::MIN; 3]; 256];
        palette[usize::from(TEST_PALETTE_INDEX)] = TEST_COLOR;
        let pixels = vec![TEST_PALETTE_INDEX; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        let frame = runtime_original_frame(&pixels, &palette).unwrap();

        assert_eq!(frame.width, LOGICAL_FRAMEBUFFER_WIDTH as u32);
        assert_eq!(frame.height, LOGICAL_FRAMEBUFFER_HEIGHT as u32);
        assert_eq!(frame.indexed_pixels, pixels);
        assert_eq!(&frame.rgba[..4], &EXPANDED_TEST_COLOR);
        let palette_offset = usize::from(TEST_PALETTE_INDEX) * 4;
        assert_eq!(
            &frame.palette_rgba[palette_offset..palette_offset + 4],
            &EXPANDED_TEST_COLOR
        );
    }

    #[test]
    fn runtime_frame_rejects_wrong_dimensions_and_invalid_dac_values() {
        let palette = [[u8::MIN; 3]; 256];
        assert!(runtime_original_frame(&[], &palette).is_err());

        let mut invalid_palette = palette;
        invalid_palette[255][2] = 64;
        let pixels = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        assert!(runtime_original_frame(&pixels, &invalid_palette).is_err());
    }

    #[test]
    fn bridge_overlay_uploads_only_pixels_authored_after_scene_composition() {
        let mut palette = [[u8::MIN; 3]; 256];
        palette[usize::from(TEST_PALETTE_INDEX)] = TEST_COLOR;
        let bridge_base = [4, 5, 6];
        let presented = [4, TEST_PALETTE_INDEX, 6];

        let rgba = sparse_indexed_overlay_rgba(&presented, &palette, &bridge_base).unwrap();

        assert_eq!(&rgba[..RGBA_COMPONENT_COUNT], &[0, 0, 0, 0]);
        assert_eq!(
            &rgba[RGBA_COMPONENT_COUNT..RGBA_COMPONENT_COUNT * 2],
            &EXPANDED_TEST_COLOR
        );
        assert_eq!(
            &rgba[RGBA_COMPONENT_COUNT * 2..RGBA_COMPONENT_COUNT * 3],
            &[0, 0, 0, 0]
        );
    }

    #[test]
    fn suppressed_manu3_dispatch_cannot_reuse_stale_triangles() {
        let triangles = [TEST_MANU3_TRIANGLE];

        assert_eq!(select_manu3_triangles(&triangles, true), triangles);
        assert!(select_manu3_triangles(&triangles, false).is_empty());
    }
}
