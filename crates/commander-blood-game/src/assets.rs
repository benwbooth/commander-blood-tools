//! Typed loading of original Commander Blood artwork.

use std::ops::RangeInclusive;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};

const RGBA_COMPONENT_COUNT: usize = 4;
const OPAQUE_ALPHA: u8 = 255;
const TITLE_FILENAME: &str = "BLOOD.LBM";
const EXECUTABLE_FILENAME: &str = "BLOODPRG.EXE";

/// One decoded original frame ready for upload to a modern GPU texture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OriginalFrame {
    /// Source width in pixels.
    pub width: u32,
    /// Source height in pixels.
    pub height: u32,
    /// Row-major red, green, blue, alpha pixels.
    pub rgba: Vec<u8>,
    /// Original row-major palette indices retained for palette animation.
    pub indexed_pixels: Vec<u8>,
    /// Source palette expanded to packed red, green, blue, alpha entries.
    pub palette_rgba: Vec<u8>,
}

impl OriginalFrame {
    /// Decode one original ILBM or PBM image without changing its palette colors.
    pub fn load_lbm(path: &Path) -> Result<Self> {
        let bytes = std::fs::read(path)
            .with_context(|| format!("reading original image {}", path.display()))?;
        let image = commander_blood_formats::lbm::decode_lbm(&bytes)
            .with_context(|| format!("decoding original LBM image {}", path.display()))?;

        let mut palette_rgba = Vec::with_capacity(image.palette.len() * RGBA_COMPONENT_COUNT);
        for [red, green, blue] in image.palette {
            palette_rgba.extend_from_slice(&[red, green, blue, OPAQUE_ALPHA]);
        }
        let mut frame = Self {
            width: image.width as u32,
            height: image.height as u32,
            rgba: Vec::new(),
            indexed_pixels: image.pixels,
            palette_rgba,
        };
        frame.rebuild_rgba();
        Ok(frame)
    }

    /// Install a recovered palette range and rebuild the current RGBA frame.
    pub fn install_palette_range(
        &mut self,
        palette: &[[u8; 3]; commander_blood_formats::lbm::PALETTE_ENTRY_COUNT],
        range: RangeInclusive<usize>,
    ) {
        for index in range {
            let destination = index * RGBA_COMPONENT_COUNT;
            self.palette_rgba[destination..destination + 3].copy_from_slice(&palette[index]);
            self.palette_rgba[destination + 3] = OPAQUE_ALPHA;
        }
        self.rebuild_rgba();
    }

    fn rebuild_rgba(&mut self) {
        self.rgba.clear();
        self.rgba
            .reserve(self.indexed_pixels.len() * RGBA_COMPONENT_COUNT);
        for palette_index in &self.indexed_pixels {
            let start = usize::from(*palette_index) * RGBA_COMPONENT_COUNT;
            self.rgba
                .extend_from_slice(&self.palette_rgba[start..start + RGBA_COMPONENT_COUNT]);
        }
    }
}

/// Find the original title image from an explicit path or known game-data roots.
pub fn find_title_image(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_owned());
        }
        bail!("title image does not exist: {}", path.display());
    }

    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_DATA") {
        candidates.push(PathBuf::from(root).join(TITLE_FILENAME));
    }
    candidates.extend([
        PathBuf::from("commander-blood-audio/_tmp_iso").join(TITLE_FILENAME),
        PathBuf::from("output/_tmp_iso").join(TITLE_FILENAME),
        PathBuf::from("accuracy/cblood_install/cblood").join(TITLE_FILENAME),
    ]);

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("BLOOD.LBM not found; pass --asset PATH or set CBLOOD_DATA")
}

/// Find the original executable used as the authoritative default-palette source.
pub fn find_bloodprg_executable(explicit: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_owned());
        }
        bail!("game executable does not exist: {}", path.display());
    }

    let mut candidates = Vec::new();
    if let Some(root) = std::env::var_os("CBLOOD_DATA") {
        candidates.push(PathBuf::from(root).join(EXECUTABLE_FILENAME));
    }
    candidates.extend([
        PathBuf::from("commander-blood-audio/_tmp_iso").join(EXECUTABLE_FILENAME),
        PathBuf::from("output/_tmp_iso").join(EXECUTABLE_FILENAME),
        PathBuf::from("accuracy/cblood_install/cblood").join(EXECUTABLE_FILENAME),
    ]);

    candidates
        .into_iter()
        .find(|path| path.is_file())
        .context("BLOODPRG.EXE not found; pass --bloodprg PATH or set CBLOOD_DATA")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ORIGINAL_TITLE_WIDTH: u32 = 640;
    const ORIGINAL_TITLE_HEIGHT: u32 = 480;
    const MINIMUM_DISTINCT_TITLE_COLORS: usize = 8;
    const ALPHA_COMPONENT_INDEX: usize = RGBA_COMPONENT_COUNT - 1;

    #[test]
    fn converts_the_original_indexed_title_to_rgba() {
        let Ok(path) = find_title_image(None) else {
            return;
        };
        let frame = OriginalFrame::load_lbm(&path).unwrap();
        assert_eq!(
            (frame.width, frame.height),
            (ORIGINAL_TITLE_WIDTH, ORIGINAL_TITLE_HEIGHT)
        );
        assert_eq!(
            frame.rgba.len(),
            ORIGINAL_TITLE_WIDTH as usize * ORIGINAL_TITLE_HEIGHT as usize * RGBA_COMPONENT_COUNT
        );
        assert!(
            frame
                .rgba
                .chunks_exact(RGBA_COMPONENT_COUNT)
                .all(|pixel| pixel[ALPHA_COMPONENT_INDEX] == OPAQUE_ALPHA)
        );
        assert_eq!(
            frame.palette_rgba.len(),
            commander_blood_formats::lbm::PALETTE_ENTRY_COUNT * RGBA_COMPONENT_COUNT
        );
        let distinct_colors: std::collections::BTreeSet<&[u8]> =
            frame.rgba.chunks_exact(RGBA_COMPONENT_COUNT).collect();
        assert!(distinct_colors.len() >= MINIMUM_DISTINCT_TITLE_COLORS);
    }
}
