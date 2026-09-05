//! RGB UI assets imported once, and a palette-free logical overlay.

use crate::native::bloodprg::{
    PresentationChoiceNumber, SubtitleRevealLine, draw_presentation_choice_number,
};
use anyhow::{Context, Result, bail};
use commander_blood_formats::bloodprg::BloodprgFontResources;

const GLYPH_WIDTH: usize = 16;
const GLYPH_HEIGHT: usize = 10;
const GLYPH_ROW_BYTES: usize = 2;
const GLYPH_BYTES: usize = GLYPH_HEIGHT * GLYPH_ROW_BYTES;
const RGBA_COMPONENTS: usize = 4;
const OPAQUE: u8 = 255;
const TRANSPARENT: u8 = 0;
const DARKEN_ALPHA: u8 = 128;
const FIRST_ROW_BIT: u16 = 1 << (GLYPH_WIDTH - 1);
const CAPTION_GLYPH_SIZE: usize = 8;
const CAPTION_FIRST_ROW_BIT: u8 = 128;
/// DESCRIPT's centered subtitle style, resolved only while importing assets.
pub(crate) const SEQUENCE_CAPTION_COLOR: usize = 239;

const SUBTITLE_GLYPH_SIZE: usize = 8;
const SUBTITLE_COLOR_INDICES: [usize; 3] = [255, 254, 253];
const SKIPPED_GLYPH_BIT: u8 = 128;
const CHANNEL_COLOR_INDEX: usize = 254;
const CHANNEL_CANVAS: [usize; 2] = [320, 200];
const INLINE_DIALOGUE_COLOR: usize = 239;
const INLINE_DIALOGUE_CLIP_HEIGHT: i32 = 10;

/// Precolored progressive-dialogue glyphs and channel masks, imported once.
pub(crate) struct DialogueUiAssets {
    character_map: Box<[u8]>,
    glyphs: Vec<[Box<[u8]>; 3]>,
    colors: [[u8; 4]; 3],
    channels: Vec<Box<[u8]>>,
    inline_font: SequenceCaptionFont,
    inline_character_map: Box<[u8]>,
    inline_advances: Box<[u8]>,
    inline_color: [u8; 4],
}

impl DialogueUiAssets {
    pub(crate) fn import(fonts: &BloodprgFontResources, source: &[[u8; 3]; 256]) -> Result<Self> {
        let mut colors = [[0; RGBA_COMPONENTS]; 3];
        for (slot, index) in SUBTITLE_COLOR_INDICES.into_iter().enumerate() {
            let color = source[index];
            if color.iter().any(|&component| component > 63) {
                bail!("dialogue UI source contains an invalid DAC component");
            }
            let rgb = color.map(|component| (component << 2) | (component >> 4));
            colors[slot] = [rgb[0], rgb[1], rgb[2], OPAQUE];
        }
        if !fonts
            .subtitle_glyphs
            .len()
            .is_multiple_of(SUBTITLE_GLYPH_SIZE)
        {
            bail!("subtitle font contains an incomplete glyph");
        }
        let glyphs = fonts
            .subtitle_glyphs
            .chunks_exact(SUBTITLE_GLYPH_SIZE)
            .map(|rows| {
                colors.map(|color| {
                    let mut rgba =
                        vec![0; SUBTITLE_GLYPH_SIZE * SUBTITLE_GLYPH_SIZE * RGBA_COMPONENTS];
                    for (y, &bits) in rows.iter().enumerate() {
                        for x in 0..SUBTITLE_GLYPH_SIZE {
                            if bits & (CAPTION_FIRST_ROW_BIT >> x) != 0 {
                                let offset = (y * SUBTITLE_GLYPH_SIZE + x) * RGBA_COMPONENTS;
                                rgba[offset..offset + RGBA_COMPONENTS].copy_from_slice(&color);
                            }
                        }
                    }
                    rgba.into_boxed_slice()
                })
            })
            .collect();
        let mut channels = Vec::new();
        for index in 0..PresentationChoiceNumber::COUNT {
            let choice = PresentationChoiceNumber::from_index(index as u8).unwrap();
            let mut mask = vec![0; CHANNEL_CANVAS[0] * CHANNEL_CANVAS[1]];
            draw_presentation_choice_number(choice, &mut mask)
                .map_err(|error| anyhow::anyhow!("importing channel mask: {error:?}"))?;
            channels.push(
                mask.into_iter()
                    .flat_map(|pixel| {
                        if usize::from(pixel) == CHANNEL_COLOR_INDEX {
                            colors[1]
                        } else {
                            [0; RGBA_COMPONENTS]
                        }
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            );
        }
        let mut inline_rows = [[0; 8]; 256];
        if !fonts.main_glyphs.len().is_multiple_of(SUBTITLE_GLYPH_SIZE)
            || fonts.main_glyphs.len() > inline_rows.len() * SUBTITLE_GLYPH_SIZE
        {
            bail!("invalid main dialogue font size");
        }
        for (target, rows) in inline_rows
            .iter_mut()
            .zip(fonts.main_glyphs.chunks_exact(SUBTITLE_GLYPH_SIZE))
        {
            target.copy_from_slice(rows);
        }
        let inline_font = SequenceCaptionFont::import(&inline_rows, source[INLINE_DIALOGUE_COLOR])?;
        let rgb =
            source[INLINE_DIALOGUE_COLOR].map(|component| (component << 2) | (component >> 4));
        Ok(Self {
            character_map: Box::from(fonts.subtitle_character_map.as_slice()),
            glyphs,
            colors,
            channels,
            inline_font,
            inline_character_map: Box::from(fonts.main_character_map.as_slice()),
            inline_advances: Box::from(fonts.main_advances.as_slice()),
            inline_color: [rgb[0], rgb[1], rgb[2], OPAQUE],
        })
    }

    pub(crate) fn color(&self, authored: u8) -> Result<[u8; 4]> {
        if usize::from(authored) == INLINE_DIALOGUE_COLOR {
            return Ok(self.inline_color);
        }
        SUBTITLE_COLOR_INDICES
            .iter()
            .position(|&index| index == usize::from(authored))
            .map(|slot| self.colors[slot])
            .context("unknown dialogue UI color")
    }

    pub(crate) fn draw_channel(
        &self,
        overlay: &mut RgbaUiOverlay,
        choice: PresentationChoiceNumber,
    ) {
        overlay.blit_image(&self.channels[choice.index()], CHANNEL_CANVAS, [0, 0]);
    }

    pub(crate) fn draw_word(
        &self,
        overlay: &mut RgbaUiOverlay,
        text: &[u8],
        origin: [i32; 2],
        color: u8,
    ) -> Result<()> {
        if usize::from(color) != INLINE_DIALOGUE_COLOR {
            bail!("unknown inline dialogue style {color}");
        }
        if origin[1] <= -INLINE_DIALOGUE_CLIP_HEIGHT || origin[1] >= CHANNEL_CANVAS[1] as i32 {
            return Ok(());
        }
        let mut x = origin[0];
        for &character in text.iter().take_while(|&&byte| byte != 0) {
            let index = *self
                .inline_character_map
                .get(usize::from(character))
                .context("inline dialogue byte outside font map")?;
            let advance = *self
                .inline_advances
                .get(usize::from(index))
                .context("inline dialogue references missing glyph")?
                as i8 as i32;
            overlay.blit_image(
                &self.inline_font.glyphs[usize::from(index)],
                [SUBTITLE_GLYPH_SIZE; 2],
                [x, origin[1]],
            );
            x = x.saturating_add(advance);
        }
        Ok(())
    }

    pub(crate) fn draw_line(
        &self,
        overlay: &mut RgbaUiOverlay,
        line: SubtitleRevealLine<'_>,
    ) -> Result<()> {
        for (position, &character) in line.text.iter().enumerate() {
            let Some(distance) = line
                .reveal_cursor
                .checked_sub(line.byte_offset.saturating_add(position))
            else {
                break;
            };
            // C tests the low byte of the reveal distance, including its wrap.
            let style = match distance as u8 {
                0 => 0,
                1 => 1,
                _ => 2,
            };
            let index = *self
                .character_map
                .get(usize::from(character))
                .context("subtitle byte outside font map")?;
            if index & SKIPPED_GLYPH_BIT != 0 {
                continue;
            }
            let glyph = self
                .glyphs
                .get(usize::from(index))
                .context("subtitle references missing glyph")?;
            let x = i32::from(line.position[0]).saturating_add(
                i32::try_from(position.saturating_mul(SUBTITLE_GLYPH_SIZE)).unwrap_or(i32::MAX),
            );
            overlay.blit_image(
                &glyph[style],
                [SUBTITLE_GLYPH_SIZE; 2],
                [x, i32::from(line.position[1])],
            );
        }
        Ok(())
    }
}

/// Precolored BIOS glyphs used by DESCRIPT captions, independent of video colors.
pub(crate) struct SequenceCaptionFont {
    glyphs: Vec<Box<[u8]>>,
}

impl SequenceCaptionFont {
    pub(crate) fn import(font: &[[u8; 8]; 256], source_color: [u8; 3]) -> Result<Self> {
        if source_color.iter().any(|&component| component > 63) {
            bail!("sequence caption source contains an invalid DAC component");
        }
        let rgb = source_color.map(|component| (component << 2) | (component >> 4));
        let color = [rgb[0], rgb[1], rgb[2], OPAQUE];
        let glyphs = font
            .iter()
            .map(|rows| {
                let mut rgba =
                    vec![TRANSPARENT; CAPTION_GLYPH_SIZE * CAPTION_GLYPH_SIZE * RGBA_COMPONENTS];
                for (y, bits) in rows.iter().enumerate() {
                    for x in 0..CAPTION_GLYPH_SIZE {
                        if bits & (CAPTION_FIRST_ROW_BIT >> x) != 0 {
                            let offset = (y * CAPTION_GLYPH_SIZE + x) * RGBA_COMPONENTS;
                            rgba[offset..offset + RGBA_COMPONENTS].copy_from_slice(&color);
                        }
                    }
                }
                rgba.into_boxed_slice()
            })
            .collect();
        Ok(Self { glyphs })
    }

    pub(crate) fn draw_text(&self, overlay: &mut RgbaUiOverlay, text: &[u8], origin: [i32; 2]) {
        let mut pen_x = origin[0];
        for &character in text.iter().take_while(|&&byte| byte != 0) {
            overlay.blit_image(
                &self.glyphs[usize::from(character)],
                [CAPTION_GLYPH_SIZE; 2],
                [pen_x, origin[1]],
            );
            pen_x = pen_x.saturating_add(CAPTION_GLYPH_SIZE as i32);
        }
    }
}

/// Semantic styles emitted by the recovered choice-list planner.
#[derive(Clone, Copy, Debug)]
pub(crate) enum ChoiceTextStyle {
    Normal,
    Hovered,
    Pressed,
}

impl TryFrom<u8> for ChoiceTextStyle {
    type Error = anyhow::Error;

    fn try_from(authored_style: u8) -> Result<Self> {
        match authored_style {
            232 => Ok(Self::Normal),
            239 => Ok(Self::Hovered),
            254 => Ok(Self::Pressed),
            _ => bail!("unrecognized choice text style {authored_style}"),
        }
    }
}

impl ChoiceTextStyle {
    const fn slot(self) -> usize {
        match self {
            Self::Normal => 0,
            Self::Hovered => 1,
            Self::Pressed => 2,
        }
    }
}

struct RgbaGlyph {
    advance: i32,
    images: [Box<[u8]>; 3],
}

/// Immutable, precolored glyph images. No live color table is retained.
pub(crate) struct ChoiceUiAssets {
    character_map: Box<[u8]>,
    glyphs: Vec<RgbaGlyph>,
    text_colors: [[u8; 4]; 3],
}

impl ChoiceUiAssets {
    /// Import the executable font and its three authored text colors at startup.
    pub(crate) fn import(
        fonts: &BloodprgFontResources,
        source_colors: &[[u8; 3]; 256],
    ) -> Result<Self> {
        let mut colors = [[TRANSPARENT; RGBA_COMPONENTS]; 3];
        for (slot, source_index) in [232, 239, 254].into_iter().enumerate() {
            let source = source_colors[source_index];
            if source.iter().any(|&component| component > 63) {
                bail!("choice font source contains an invalid DAC component");
            }
            colors[slot] = [
                (source[0] << 2) | (source[0] >> 4),
                (source[1] << 2) | (source[1] >> 4),
                (source[2] << 2) | (source[2] >> 4),
                OPAQUE,
            ];
        }
        if !fonts.square_caps_glyphs.len().is_multiple_of(GLYPH_BYTES) {
            bail!("square-cap font contains an incomplete glyph");
        }
        let glyphs = fonts
            .square_caps_glyphs
            .chunks_exact(GLYPH_BYTES)
            .enumerate()
            .map(|(index, encoded)| {
                let advance = *fonts
                    .square_caps_advances
                    .get(index)
                    .context("square-cap glyph has no pen advance")?
                    as i8 as i32;
                let images = colors.map(|color| {
                    let mut rgba = vec![TRANSPARENT; GLYPH_WIDTH * GLYPH_HEIGHT * RGBA_COMPONENTS];
                    for (y, row) in encoded.chunks_exact(GLYPH_ROW_BYTES).enumerate() {
                        let bits = u16::from_be_bytes([row[0], row[1]]);
                        for x in 0..GLYPH_WIDTH {
                            if bits & (FIRST_ROW_BIT >> x) != 0 {
                                let offset = (y * GLYPH_WIDTH + x) * RGBA_COMPONENTS;
                                rgba[offset..offset + RGBA_COMPONENTS].copy_from_slice(&color);
                            }
                        }
                    }
                    rgba.into_boxed_slice()
                });
                Ok(RgbaGlyph { advance, images })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self {
            character_map: Box::from(fonts.square_caps_character_map.as_slice()),
            glyphs,
            text_colors: colors,
        })
    }

    pub(crate) fn text_color(&self, style: ChoiceTextStyle) -> [u8; 4] {
        self.text_colors[style.slot()]
    }

    pub(crate) fn draw_text(
        &self,
        overlay: &mut RgbaUiOverlay,
        text: &[u8],
        origin: [i32; 2],
        style: ChoiceTextStyle,
    ) -> Result<()> {
        let mut pen_x = origin[0];
        for &character in text.iter().take_while(|&&byte| byte != 0) {
            let index = *self
                .character_map
                .get(usize::from(character))
                .context("choice text byte is outside the imported font")?;
            let glyph = self
                .glyphs
                .get(usize::from(index))
                .context("choice text refers to a missing imported glyph")?;
            overlay.blit_image(
                &glyph.images[style.slot()],
                [GLYPH_WIDTH, GLYPH_HEIGHT],
                [pen_x, origin[1]],
            );
            pen_x = pen_x.saturating_add(glyph.advance);
        }
        Ok(())
    }
}

/// Independent UI pixels retained across visual refreshes, reset once per game frame.
pub(crate) struct RgbaUiOverlay {
    width: usize,
    height: usize,
    pixels: Box<[u8]>,
}

impl RgbaUiOverlay {
    pub(crate) fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            pixels: vec![TRANSPARENT; width * height * RGBA_COMPONENTS].into_boxed_slice(),
        }
    }

    pub(crate) fn pixels(&self) -> &[u8] {
        &self.pixels
    }

    pub(crate) fn clear(&mut self) {
        self.pixels.fill(TRANSPARENT);
    }

    /// A translucent black rectangle replaces C's nearest-color darkening map.
    /// Repeated preparation in one game frame does not accumulate extra dimming.
    pub(crate) fn darken_rect(&mut self, origin: [i32; 2], size: [u16; 2]) {
        self.fill_rect(origin, size, [0, 0, 0, DARKEN_ALPHA]);
    }

    pub(crate) fn fill_rect(&mut self, origin: [i32; 2], size: [u16; 2], color: [u8; 4]) {
        let left = origin[0].clamp(0, self.width as i32) as usize;
        let top = origin[1].clamp(0, self.height as i32) as usize;
        let right = origin[0]
            .saturating_add(i32::from(size[0]))
            .clamp(0, self.width as i32) as usize;
        let bottom = origin[1]
            .saturating_add(i32::from(size[1]))
            .clamp(0, self.height as i32) as usize;
        for y in top..bottom {
            for x in left..right {
                let offset = (y * self.width + x) * RGBA_COMPONENTS;
                self.pixels[offset..offset + RGBA_COMPONENTS].copy_from_slice(&color);
            }
        }
    }

    /// Stamp retained opaque caption pixels, leaving transparent pixels untouched.
    pub(crate) fn blit_overlay(&mut self, overlay: &Self) {
        self.blit_image(overlay.pixels(), [overlay.width, overlay.height], [0, 0]);
    }

    fn blit_image(&mut self, rgba: &[u8], size: [usize; 2], origin: [i32; 2]) {
        for y in 0..size[1] {
            for x in 0..size[0] {
                let source = (y * size[0] + x) * RGBA_COMPONENTS;
                if rgba[source + RGBA_COMPONENTS - 1] == TRANSPARENT {
                    continue;
                }
                let dx = origin[0].saturating_add(x as i32);
                let dy = origin[1].saturating_add(y as i32);
                if dx < 0 || dy < 0 || dx >= self.width as i32 || dy >= self.height as i32 {
                    continue;
                }
                let destination = (dy as usize * self.width + dx as usize) * RGBA_COMPONENTS;
                self.pixels[destination..destination + RGBA_COMPONENTS]
                    .copy_from_slice(&rgba[source..source + RGBA_COMPONENTS]);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgb_dialogue_glyphs_and_channel_masks_match_native_rasters() {
        use crate::native::bloodprg::{
            FontPoint, FontVerticalBand, draw_planar_dialogue_text, draw_subtitle_reveal_line,
        };
        let fonts = commander_blood_formats::bloodprg::decode_bloodprg_font_resources(
            include_bytes!("../../../re/bin/BLOODPRG.EXE"),
        )
        .unwrap();
        let mut colors = [[17, 31, 47]; 256];
        colors[255] = [63, 63, 63];
        colors[254] = [40, 30, 20];
        colors[253] = [10, 20, 30];
        let assets = DialogueUiAssets::import(&fonts, &colors).unwrap();
        let mut ui = RgbaUiOverlay::new(320, 200);
        let mut reference = vec![0; 320 * 200];
        let assert_raster = |ui: &RgbaUiOverlay, reference: &[u8]| {
            for (&index, pixel) in reference.iter().zip(ui.pixels().chunks_exact(4)) {
                if index == 0 {
                    assert_eq!(pixel, [0; 4]);
                } else {
                    assert_eq!(pixel, assets.color(index).unwrap());
                }
            }
        };
        for byte in 1..fonts.subtitle_character_map.len() {
            if byte == usize::from(b'\r') {
                continue;
            }
            for cursor in [0, 1, 2, 256, 257] {
                reference.fill(0);
                ui.clear();
                let text = [byte as u8];
                let original = draw_subtitle_reveal_line(
                    &mut reference,
                    &fonts,
                    &[byte as u8, b'\r'],
                    FontPoint { x: 20, y: 20 },
                    cursor as i32,
                );
                let modern = assets.draw_line(
                    &mut ui,
                    SubtitleRevealLine {
                        text: &text,
                        byte_offset: 7,
                        reveal_cursor: cursor + 7,
                        position: [20, 20],
                    },
                );
                assert_eq!(modern.is_ok(), original.is_ok(), "subtitle byte {byte}");
                if modern.is_ok() {
                    assert_raster(&ui, &reference);
                }
            }
        }
        for byte in 1..fonts.main_character_map.len() {
            reference.fill(0);
            ui.clear();
            let text = [byte as u8, byte as u8, 0];
            let original = draw_planar_dialogue_text(
                &mut reference,
                &fonts,
                &text,
                FontPoint { x: 20, y: 20 },
                FontVerticalBand {
                    top: 0,
                    bottom: 199,
                },
                INLINE_DIALOGUE_COLOR as u8,
            );
            let modern = assets.draw_word(&mut ui, &text, [20, 20], INLINE_DIALOGUE_COLOR as u8);
            assert_eq!(modern.is_ok(), original.is_ok(), "inline byte {byte}");
            if modern.is_ok() {
                assert_raster(&ui, &reference);
            }
        }
        for index in 0..PresentationChoiceNumber::COUNT {
            reference.fill(0);
            ui.clear();
            let choice = PresentationChoiceNumber::from_index(index as u8).unwrap();
            draw_presentation_choice_number(choice, &mut reference).unwrap();
            assets.draw_channel(&mut ui, choice);
            assert_raster(&ui, &reference);
        }
    }

    #[test]
    fn imported_caption_font_matches_every_original_bios_glyph() {
        use crate::native::bloodprg::{FontPoint, draw_bios_font_text};
        use crate::runtime::VGA_BIOS_FONT_8X8;
        let font = SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [17, 31, 47]).unwrap();
        let mut overlay = RgbaUiOverlay::new(320, 200);
        let mut reference = vec![0; 320 * 200];
        for character in u8::MIN..=u8::MAX {
            overlay.clear();
            reference.fill(0);
            let text = [character, 0, b'X'];
            font.draw_text(&mut overlay, &text, [20, 20]);
            draw_bios_font_text(
                &mut reference,
                &VGA_BIOS_FONT_8X8,
                &text,
                FontPoint { x: 20, y: 20 },
                1,
                text.len() as u8,
            )
            .unwrap();
            for (&coverage, rgba) in reference.iter().zip(overlay.pixels().chunks_exact(4)) {
                assert_eq!(rgba[3] != 0, coverage != 0, "glyph for byte {character}");
                if coverage != 0 {
                    assert_eq!(rgba, [69, 125, 190, 255]);
                }
            }
        }
        assert!(SequenceCaptionFont::import(&VGA_BIOS_FONT_8X8, [64, 0, 0]).is_err());
    }

    #[test]
    #[ignore = "requires the original BLOODPRG.EXE font data"]
    fn every_imported_square_cap_glyph_matches_the_c_raster_coverage() {
        use crate::native::bloodprg::{FontPoint, FontVerticalBand, draw_square_caps_text};
        use commander_blood_formats::bloodprg::decode_bloodprg_font_resources;
        let executable =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../re/bin/BLOODPRG.EXE");
        let bytes =
            std::fs::read(executable).expect("original executable font fixture is required");
        let fonts = decode_bloodprg_font_resources(&bytes).unwrap();
        let colors = [[17, 31, 47]; 256];
        let assets = ChoiceUiAssets::import(&fonts, &colors).unwrap();
        let mut overlay = RgbaUiOverlay::new(320, 200);
        let mut reference = vec![0; 320 * 200];
        let mut checked = 0;
        for character in 1..fonts.square_caps_character_map.len() {
            let glyph = usize::from(fonts.square_caps_character_map[character]);
            if glyph >= assets.glyphs.len() {
                continue;
            }
            let text = [character as u8];
            overlay.clear();
            reference.fill(0);
            assets
                .draw_text(&mut overlay, &text, [20, 20], ChoiceTextStyle::Hovered)
                .unwrap();
            draw_square_caps_text(
                &mut reference,
                &fonts,
                &text,
                FontPoint { x: 20, y: 20 },
                FontVerticalBand {
                    top: 0,
                    bottom: 199,
                },
                1,
            )
            .unwrap();
            for (&coverage, rgba) in reference.iter().zip(overlay.pixels().chunks_exact(4)) {
                assert_eq!(rgba[3] != 0, coverage != 0, "glyph for byte {character}");
                if coverage != 0 {
                    assert_eq!(rgba, [69, 125, 190, 255]);
                }
            }
            checked += 1;
        }
        const SUPPORTED_SQUARE_CAP_CHARACTERS: usize = 86;
        assert_eq!(checked, SUPPORTED_SQUARE_CAP_CHARACTERS);
    }

    #[test]
    fn darkening_is_clipped_idempotent_and_cleared_without_touching_scene_pixels() {
        let mut overlay = RgbaUiOverlay::new(4, 3);
        overlay.darken_rect([-1, 1], [3, 4]);
        let once = overlay.pixels().to_vec();
        overlay.darken_rect([-1, 1], [3, 4]);
        assert_eq!(overlay.pixels(), once);
        for (index, pixel) in overlay.pixels().chunks_exact(4).enumerate() {
            let expected = if index / 4 >= 1 && index % 4 < 2 {
                [0, 0, 0, 128]
            } else {
                [0; 4]
            };
            assert_eq!(pixel, expected);
        }
        overlay.clear();
        assert!(overlay.pixels().iter().all(|&byte| byte == 0));
    }
}
