//! Embedded application identity, independent of the original game data.

use anyhow::{Context, Result};
use sdl3::{pixels::PixelFormat, surface::Surface, video::Window};

pub(crate) const APPLICATION_ID: &str = "commander-blood";
const ICON_SIZE: u32 = 256;
const RGBA_COMPONENTS: u32 = 4;
const ICON_PIXELS: &[u8; (ICON_SIZE * ICON_SIZE * RGBA_COMPONENTS) as usize] =
    include_bytes!("../assets/commander-blood.rgba");

pub(crate) fn install(window: &mut Window) -> Result<()> {
    let mut pixels = ICON_PIXELS.to_vec();
    let surface = Surface::from_data(
        &mut pixels,
        ICON_SIZE,
        ICON_SIZE,
        ICON_SIZE * RGBA_COMPONENTS,
        PixelFormat::RGBA32,
    )
    .map_err(anyhow::Error::msg)
    .context("loading the embedded application icon")?;
    // Some Wayland compositors only use the matching desktop-entry icon.
    if !window.set_icon(surface) {
        eprintln!(
            "Window manager did not accept the icon; desktop application identity remains available"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_icon_has_visible_artwork_and_transparent_margins() {
        assert!(ICON_PIXELS.chunks_exact(4).any(|pixel| pixel[3] == 0));
        let visible_blue_pixels = ICON_PIXELS
            .chunks_exact(4)
            .filter(|pixel| pixel[3] >= 240 && pixel[2] > 128)
            .count();
        assert!(visible_blue_pixels > (ICON_SIZE * ICON_SIZE / 4) as usize);
    }
}
