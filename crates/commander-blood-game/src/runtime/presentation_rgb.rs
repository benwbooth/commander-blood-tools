//! RGB page ownership at the legacy video decoder boundary.

use anyhow::Result;

use crate::native::bloodprg::{
    IndexedGamePalette, LOGICAL_FRAMEBUFFER_WIDTH, PresentationActiveEntryError,
    PresentationEntryPresenter, PresentationEntryRenderTarget, PresentationRectBlitOutcome,
    PresentationRectDecodeOutcome, blit_presentation_rect, copy_full_frame_to_display,
    decode_presentation_rect,
};
use crate::render::indexed_palette_rgba;

/// Pixels inherited from artwork or another clip remain RGB, not palette references.
#[derive(Clone)]
pub(super) struct RgbVideoPage {
    pub pixels: Box<[u8]>,
    video_coverage: Box<[bool]>,
    artwork_pixels: Box<[u8]>,
    artwork_coverage: Box<[bool]>,
    artwork_darken: u8,
}

impl RgbVideoPage {
    #[cfg(test)]
    pub fn coverage(&self) -> &[bool] {
        &self.video_coverage
    }

    pub fn new(pixels: Box<[u8]>) -> Self {
        Self {
            video_coverage: vec![false; pixels.len() / 4].into_boxed_slice(),
            artwork_coverage: vec![false; pixels.len() / 4].into_boxed_slice(),
            artwork_pixels: pixels.clone(),
            artwork_darken: 0,
            pixels,
        }
    }

    pub fn inherited(&self) -> Self {
        let mut inherited = self.clone();
        inherited.video_coverage.fill(false);
        inherited
    }

    pub fn import_artwork(&mut self, rgba: &[u8], indices: &[u8], transparent_zero: bool) {
        for (index, source) in indices.iter().copied().enumerate() {
            if !transparent_zero || source != 0 {
                let pixel = index * 4..index * 4 + 4;
                self.pixels[pixel.clone()].copy_from_slice(&rgba[pixel.clone()]);
                self.artwork_pixels[pixel.clone()].copy_from_slice(&rgba[pixel]);
                // Convert the authored background-color bank to layer coverage
                // once, at import. Later effects operate on RGB artwork only.
                self.artwork_coverage[index] = (128..192).contains(&source);
                self.video_coverage[index] = false;
            }
        }
        self.artwork_darken = 0;
    }

    pub fn darken_artwork(&mut self, amount: u8) {
        self.artwork_darken = amount;
        for ((pixel, original), covered) in self
            .pixels
            .chunks_exact_mut(4)
            .zip(self.artwork_pixels.chunks_exact(4))
            .zip(self.artwork_coverage.iter())
        {
            if *covered {
                for component in 0..3 {
                    pixel[component] = original[component].saturating_sub(amount);
                }
            }
        }
    }

    pub fn clear_rows(&mut self, rows: std::ops::Range<usize>, rgb: [u8; 4]) {
        let range = rows.start * LOGICAL_FRAMEBUFFER_WIDTH..rows.end * LOGICAL_FRAMEBUFFER_WIDTH;
        for index in range {
            self.pixels[index * 4..index * 4 + 4].copy_from_slice(&rgb);
            self.video_coverage[index] = false;
            self.artwork_coverage[index] = false;
        }
    }

    pub fn resolve_video(&mut self, indices: &[u8], colors: &IndexedGamePalette) -> Result<()> {
        let decoded_colors = indexed_palette_rgba(colors)?;
        for ((pixel, index), covered) in self
            .pixels
            .chunks_exact_mut(4)
            .zip(indices)
            .zip(self.video_coverage.iter())
        {
            if *covered {
                let start = usize::from(*index) * 4;
                pixel.copy_from_slice(&decoded_colors[start..start + 4]);
            }
        }
        self.darken_artwork(self.artwork_darken);
        Ok(())
    }
}

pub(super) struct RgbPresentationEntryPresenter<'a> {
    pub display_buffer: &'a mut [u8],
    pub back_buffer: &'a mut [u8],
    pub decode_staging: &'a mut [u8],
    pub display: &'a mut RgbVideoPage,
    pub back: &'a mut RgbVideoPage,
}

impl PresentationEntryPresenter for RgbPresentationEntryPresenter<'_> {
    fn present_back_buffer(&mut self) -> Result<(), PresentationActiveEntryError> {
        copy_full_frame_to_display(self.back_buffer, self.display_buffer)?;
        self.display.clone_from(self.back);
        Ok(())
    }

    fn blit_rectangle(
        &mut self,
        source: &[u8],
        target: PresentationEntryRenderTarget,
        x: usize,
        y: usize,
        width: usize,
        row_mode: u16,
    ) -> Result<PresentationRectBlitOutcome, PresentationActiveEntryError> {
        let (indices, page) = match target {
            PresentationEntryRenderTarget::Display => {
                (&mut *self.display_buffer, &mut *self.display)
            }
            PresentationEntryRenderTarget::BackBuffer => (&mut *self.back_buffer, &mut *self.back),
        };
        let outcome = blit_presentation_rect(source, indices, x, y, width, row_mode)?;
        if width != 0 {
            for (row, pixels) in source[..outcome.consumed_bytes]
                .chunks_exact(width)
                .enumerate()
            {
                for (column, pixel) in pixels.iter().enumerate() {
                    if row_mode >> 8 != 255 || *pixel != 0 {
                        let index = (y + row) * LOGICAL_FRAMEBUFFER_WIDTH + x + column;
                        page.video_coverage[index] = true;
                        page.artwork_coverage[index] = false;
                    }
                }
            }
        }
        Ok(outcome)
    }

    fn decode_rectangle(
        &mut self,
        source: &[u8],
        vertical_offset: usize,
        layout: u16,
        row_mode: u16,
    ) -> Result<PresentationRectDecodeOutcome, PresentationActiveEntryError> {
        // AD zero is transparent. Decode against zero to retain exact write
        // coverage, including writes equal to the old destination index.
        let mut decoded = vec![0; self.display_buffer.len()];
        let mut outcome = decode_presentation_rect(
            source,
            self.decode_staging,
            &mut decoded,
            vertical_offset,
            layout,
            row_mode,
        )?;
        outcome.changed_pixels = 0;
        for (index, (destination, pixel)) in self.display_buffer.iter_mut().zip(decoded).enumerate()
        {
            if pixel != 0 {
                outcome.changed_pixels += usize::from(*destination != pixel);
                *destination = pixel;
                self.display.video_coverage[index] = true;
                self.display.artwork_coverage[index] = false;
            }
        }
        Ok(outcome)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn imported_artwork_darkens_in_rgb_without_accumulating_across_clip_switches() {
        let pixels = 320 * 200;
        let original = [210, 170, 80, 255].repeat(pixels).into_boxed_slice();
        let mut page = RgbVideoPage::new(original.clone());
        page.import_artwork(&original, &vec![150; pixels], false);
        page.darken_artwork(81);
        assert_eq!(&page.pixels[..4], &[129, 89, 0, 255]);
        let mut inherited = page.inherited();
        inherited.darken_artwork(162);
        assert_eq!(&inherited.pixels[..4], &[48, 8, 0, 255]);
        inherited
            .resolve_video(&vec![150; pixels], &[[63, 0, 0]; 256])
            .unwrap();
        inherited.darken_artwork(162);
        assert_eq!(&inherited.pixels[..4], &[48, 8, 0, 255]);
        inherited.darken_artwork(0);
        assert_eq!(inherited.pixels, original);
    }

    #[test]
    fn equal_index_writes_and_transparent_pixels_have_distinct_rgb_owners() {
        let pixels = 320 * 200;
        let original_rgb = [17, 31, 49, 255];
        let mut display = RgbVideoPage::new(original_rgb.repeat(pixels).into_boxed_slice());
        let mut back = display.clone();
        let mut display_indices = vec![150; pixels];
        let mut back_indices = display_indices.clone();
        let mut staging = vec![0; 65536];
        let mut presenter = RgbPresentationEntryPresenter {
            display_buffer: &mut display_indices,
            back_buffer: &mut back_indices,
            decode_staging: &mut staging,
            display: &mut display,
            back: &mut back,
        };
        presenter
            .blit_rectangle(
                &[0, 150, 0],
                PresentationEntryRenderTarget::Display,
                0,
                0,
                3,
                0xff01,
            )
            .unwrap();
        let mut palette = [[63, 0, 0]; 256];
        display.resolve_video(&display_indices, &palette).unwrap();
        assert_eq!(&display.pixels[..4], &original_rgb);
        assert_eq!(&display.pixels[4..8], &[255, 0, 0, 255]);
        assert_eq!(&display.pixels[8..12], &original_rgb);
        palette.fill([0, 63, 0]);
        display.resolve_video(&display_indices, &palette).unwrap();
        assert_eq!(&display.pixels[..4], &original_rgb);
        assert_eq!(&display.pixels[4..8], &[0, 255, 0, 255]);
        assert_eq!(&display.pixels[8..12], &original_rgb);
    }

    #[test]
    fn back_page_copy_preserves_rgb_and_opaque_zero_is_a_video_write() {
        let pixels = 320 * 200;
        let mut display = RgbVideoPage::new([1, 2, 3, 255].repeat(pixels).into_boxed_slice());
        let mut back = RgbVideoPage::new([11, 22, 33, 255].repeat(pixels).into_boxed_slice());
        let mut display_indices = vec![0; pixels];
        let mut back_indices = vec![0; pixels];
        let mut staging = vec![0; 65536];
        let mut presenter = RgbPresentationEntryPresenter {
            display_buffer: &mut display_indices,
            back_buffer: &mut back_indices,
            decode_staging: &mut staging,
            display: &mut display,
            back: &mut back,
        };
        presenter
            .blit_rectangle(&[0], PresentationEntryRenderTarget::BackBuffer, 0, 0, 1, 1)
            .unwrap();
        presenter.present_back_buffer().unwrap();
        display
            .resolve_video(&display_indices, &[[63; 3]; 256])
            .unwrap();
        assert_eq!(&display.pixels[..4], &[255; 4]);
        assert_eq!(&display.pixels[4..8], &[11, 22, 33, 255]);
    }
}
