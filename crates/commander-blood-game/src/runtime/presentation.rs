//! Indexed-frame and modern 3D composition for the translated game lifecycle.

use anyhow::{Context, Result, bail};
use sdl3::video::Window;

use crate::assets::OriginalFrame;
use crate::native::bloodprg::{BridgeSceneFrame, IndexedGamePalette};
use crate::render::{Renderer, indexed_frame_rgba, indexed_palette_rgba};

use super::{
    LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_PIXEL_COUNT, LOGICAL_FRAMEBUFFER_WIDTH,
    OriginalGameRuntime,
};

/// SDL/wgpu presentation state for the original logical framebuffer and bridge.
pub struct RuntimePresentationHost<'window> {
    renderer: Renderer<'window>,
    presented_frame_count: u64,
}

impl<'window> RuntimePresentationHost<'window> {
    /// Create the main-game renderer with bridge and optional MANU3 resources.
    pub fn new(window: &'window Window, runtime: &OriginalGameRuntime) -> Result<Self> {
        let initial_frame =
            runtime_original_frame(runtime.front_buffer().pixels(), runtime.live_palette())?;
        let renderer = Renderer::new(
            window,
            &initial_frame,
            runtime.manu3(),
            None,
            Some(runtime.live_palette()),
        )
        .context("initializing main-game wgpu presentation")?;
        Ok(Self {
            renderer,
            presented_frame_count: u64::MIN,
        })
    }

    /// Reconfigure the wgpu surface after an SDL pixel-size change.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.renderer.resize(width, height);
    }

    /// Upload the runtime's complete indexed frame and current VGA palette.
    pub fn submit_indexed_frame(&mut self, runtime: &OriginalGameRuntime) -> Result<()> {
        self.renderer
            .upload_indexed_frame(runtime.front_buffer().pixels(), runtime.live_palette())
            .context("uploading translated indexed game frame")
    }

    /// Present indexed artwork or one bridge frame, then composite MANU3.
    pub fn present_frame(
        &mut self,
        runtime: &OriginalGameRuntime,
        bridge_frame: Option<&BridgeSceneFrame>,
    ) -> Result<()> {
        let manu3_triangles = runtime
            .manu3()
            .map(|model| model.render_triangles())
            .unwrap_or(&[]);
        self.renderer
            .render(manu3_triangles, None, bridge_frame)
            .context("presenting translated game frame")?;
        self.presented_frame_count = self.presented_frame_count.wrapping_add(1);
        Ok(())
    }

    /// Number of frames submitted to the window surface.
    pub const fn presented_frame_count(&self) -> u64 {
        self.presented_frame_count
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PALETTE_INDEX: u8 = 19;
    const TEST_COLOR: [u8; 3] = [63, 32, 0];
    const EXPANDED_TEST_COLOR: [u8; 4] = [255, 129, 0, 255];

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
}
