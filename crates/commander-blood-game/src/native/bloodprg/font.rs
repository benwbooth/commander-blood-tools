//! Measurement and rasterization support for executable-embedded game fonts.

use std::error::Error;
use std::fmt;

use commander_blood_formats::bloodprg::BloodprgFontResources;

use super::framebuffer_copy::{LOGICAL_FRAMEBUFFER_HEIGHT, LOGICAL_FRAMEBUFFER_WIDTH};

const TRAILING_INTERCHARACTER_GAP: u16 = 2;
const BIOS_FONT_CHARACTER_COUNT: usize = 256;
const BIOS_FONT_GLYPH_WIDTH: i32 = 8;
const BIOS_FONT_GLYPH_HEIGHT: usize = 8;
const SQUARE_CAPS_GLYPH_WIDTH: usize = 16;
const SQUARE_CAPS_GLYPH_HEIGHT: i32 = 10;
const SQUARE_CAPS_GLYPH_BYTE_COUNT: usize = 20;
const MAIN_FONT_GLYPH_WIDTH: usize = 8;
const MAIN_FONT_GLYPH_HEIGHT: i32 = 8;
const MAIN_FONT_SPACE_ADVANCE: i32 = 6;
const HIGHEST_BYTE_BIT: u8 = 128;
const HIGHEST_WORD_BIT: u16 = 32_768;
const BITS_PER_BYTE: u32 = 8;
const UNLIMITED_BIOS_CHARACTER_COUNT: usize = 256;
const COUNT_INCREMENT: usize = 1;
const SQUARE_CAPS_ROW_BYTE_COUNT: usize = 2;
const SQUARE_CAPS_ROW_LOW_BYTE_INDEX: usize = 1;
const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;

/// Complete 256-character ROM font captured by the DOS startup routine.
pub type BiosFont8x8 = [[u8; BIOS_FONT_GLYPH_HEIGHT]; BIOS_FONT_CHARACTER_COUNT];

/// Proportional font selected by the original dual-font width routine.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameFontFace {
    /// Ten-row square-capital UI font.
    SquareCaps,
    /// Eight-row main dialogue font.
    Main,
}

/// Invalid text byte or decoded font lookup supplied to a font operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameFontError {
    /// A byte falls outside the recovered character-map domain.
    UnsupportedCharacter {
        /// Position in the supplied text.
        position: usize,
        /// Original text byte.
        character: u8,
    },
    /// A decoded glyph index falls outside the measurement table.
    InvalidGlyphIndex {
        /// Position in the supplied text.
        position: usize,
        /// Original text byte.
        character: u8,
        /// Index produced by the executable character map.
        glyph_index: u8,
    },
}

/// Logical text origin in the original 320 by 200 display.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FontPoint {
    /// Horizontal pen coordinate.
    pub x: i32,
    /// Top glyph-row coordinate.
    pub y: i32,
}

/// Inclusive vertical activation band used by the proportional draw routines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FontVerticalBand {
    /// First row of the active content band.
    pub top: i32,
    /// Last accepted glyph-origin row.
    pub bottom: i32,
}

/// Observable result of one flat text draw call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GameFontDrawOutcome {
    /// The vertical activation gate rejected the complete string.
    pub clipped: bool,
    /// Number of non-NUL source bytes inspected.
    pub consumed_characters: usize,
    /// Number of glyph bitmaps submitted.
    pub drawn_glyphs: usize,
    /// Main-font spaces that moved the pen without changing draw width.
    pub spaces: usize,
    /// Main-font mapped bytes skipped through the high-bit sentinel.
    pub skipped_glyphs: usize,
    /// Original wrapping draw-width accumulator.
    pub draw_width: u16,
    /// Logical pen position after the final processed character.
    pub final_pen: FontPoint,
}

/// Invalid flat destination or font resource supplied to a text draw operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameFontDrawError {
    /// The destination does not contain a complete logical framebuffer.
    FramebufferTooShort {
        /// Number of available indexed pixels.
        actual: usize,
    },
    /// The vertical activation band is inverted or outside the display.
    InvalidVerticalBand(FontVerticalBand),
    /// A byte falls outside the decoded character-map domain.
    UnsupportedCharacter {
        /// Position in the supplied text.
        position: usize,
        /// Original text byte.
        character: u8,
    },
    /// A decoded glyph index falls outside the owned font tables.
    InvalidGlyphIndex {
        /// Position in the supplied text.
        position: usize,
        /// Original text byte.
        character: u8,
        /// Index produced by the executable character map.
        glyph_index: u8,
    },
    /// A lit glyph pixel falls outside the flat logical display.
    PixelOutsideDisplay {
        /// Position in the supplied text.
        position: usize,
        /// Horizontal logical coordinate.
        x: i32,
        /// Vertical logical coordinate.
        y: i32,
    },
}

impl fmt::Display for GameFontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Commander Blood font operation: {self:?}"
        )
    }
}

impl Error for GameFontError {}

impl fmt::Display for GameFontDrawError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid Commander Blood font draw: {self:?}")
    }
}

impl Error for GameFontDrawError {}

/// Measure text using the original dual-font lookup and trailing-gap rule.
///
/// This translates `text_width_dual_font` at BLOODPRG offset `0x0030CD`.
/// Width addition and the final two-pixel subtraction retain 16-bit wrapping.
/// A zero byte terminates the input; ordinary Rust slices replace the far text
/// pointer, and decoded owned tables replace runtime game-segment addressing.
pub fn measure_game_text_width(
    text: &[u8],
    face: GameFontFace,
    fonts: &BloodprgFontResources,
) -> Result<u16, GameFontError> {
    let (character_map, measurement_advances) = match face {
        GameFontFace::SquareCaps => (
            fonts.square_caps_character_map.as_slice(),
            fonts.square_caps_measurement_advances.as_slice(),
        ),
        GameFontFace::Main => (
            fonts.main_character_map.as_slice(),
            fonts.main_measurement_advances.as_slice(),
        ),
    };

    let mut width = u16::MIN;
    for (position, character) in text.iter().copied().enumerate() {
        if character == u8::MIN {
            break;
        }
        let glyph_index = *character_map.get(usize::from(character)).ok_or(
            GameFontError::UnsupportedCharacter {
                position,
                character,
            },
        )?;
        let advance = *measurement_advances.get(usize::from(glyph_index)).ok_or(
            GameFontError::InvalidGlyphIndex {
                position,
                character,
                glyph_index,
            },
        )?;
        width = width.wrapping_add(u16::from(advance));
    }
    Ok(width.wrapping_sub(TRAILING_INTERCHARACTER_GAP))
}

/// Draw a bounded string through the host-supplied BIOS 8 by 8 font.
///
/// This translates `font8x8_text_draw_display` at BLOODPRG offset `0x003066`.
/// It retains NUL termination, the packed zero-means-256 character limit, fixed
/// eight-pixel advance, and transparent glyph bits. An owned ROM-font array and
/// checked logical coordinates replace the captured far BIOS pointer, display
/// segment offset, and 16-bit pointer wrapping.
pub fn draw_bios_font_text(
    framebuffer: &mut [u8],
    font: &BiosFont8x8,
    text: &[u8],
    origin: FontPoint,
    color: u8,
    character_limit: u8,
) -> Result<GameFontDrawOutcome, GameFontDrawError> {
    validate_framebuffer(framebuffer)?;
    let budget = if character_limit == u8::MIN {
        UNLIMITED_BIOS_CHARACTER_COUNT
    } else {
        usize::from(character_limit)
    };
    let mut writes = Vec::new();
    let mut drawn_glyphs = usize::MIN;

    for (position, character) in text.iter().copied().take(budget).enumerate() {
        if character == u8::MIN {
            break;
        }
        let glyph = &font[usize::from(character)];
        let pen_x = origin.x.saturating_add(
            i32::try_from(position)
                .unwrap_or(i32::MAX)
                .saturating_mul(BIOS_FONT_GLYPH_WIDTH),
        );
        collect_byte_glyph_writes(
            &mut writes,
            glyph,
            BIOS_FONT_GLYPH_WIDTH as usize,
            FontPoint {
                x: pen_x,
                y: origin.y,
            },
            position,
            color,
        )?;
        drawn_glyphs += COUNT_INCREMENT;
    }
    apply_writes(framebuffer, &writes);
    let advance = i32::try_from(drawn_glyphs)
        .unwrap_or(i32::MAX)
        .saturating_mul(BIOS_FONT_GLYPH_WIDTH);
    Ok(GameFontDrawOutcome {
        clipped: false,
        consumed_characters: drawn_glyphs,
        drawn_glyphs,
        spaces: usize::MIN,
        skipped_glyphs: usize::MIN,
        draw_width: u16::try_from(drawn_glyphs)
            .unwrap_or(u16::MAX)
            .wrapping_mul(BIOS_FONT_GLYPH_WIDTH as u16),
        final_pen: FontPoint {
            x: origin.x.saturating_add(advance),
            y: origin.y,
        },
    })
}

/// Draw text through the executable-embedded ten-row square-capital font.
///
/// This translates `square_caps_text_draw_display` at BLOODPRG offset
/// `0x003106`. It retains the inclusive bottom-origin gate, top-minus-height
/// rule, signed advances, transparent 16-bit rows, and wrapping draw-width
/// accumulator while replacing aliased game-segment tables with decoded owned
/// font resources.
pub fn draw_square_caps_text(
    framebuffer: &mut [u8],
    fonts: &BloodprgFontResources,
    text: &[u8],
    origin: FontPoint,
    band: FontVerticalBand,
    color: u8,
) -> Result<GameFontDrawOutcome, GameFontDrawError> {
    validate_framebuffer(framebuffer)?;
    validate_vertical_band(band)?;
    if text_origin_is_clipped(origin.y, band, SQUARE_CAPS_GLYPH_HEIGHT) {
        return Ok(clipped_outcome(origin));
    }

    let mut writes = Vec::new();
    let mut pen_x = origin.x;
    let mut draw_width = u16::MIN;
    let mut drawn_glyphs = usize::MIN;
    for (position, character) in text.iter().copied().enumerate() {
        if character == u8::MIN {
            break;
        }
        let glyph_index = mapped_glyph(&fonts.square_caps_character_map, position, character)?;
        let advance = *fonts
            .square_caps_advances
            .get(usize::from(glyph_index))
            .ok_or(GameFontDrawError::InvalidGlyphIndex {
                position,
                character,
                glyph_index,
            })? as i8 as i32;
        let glyph_start = usize::from(glyph_index) * SQUARE_CAPS_GLYPH_BYTE_COUNT;
        let glyph = fonts
            .square_caps_glyphs
            .get(glyph_start..glyph_start + SQUARE_CAPS_GLYPH_BYTE_COUNT)
            .ok_or(GameFontDrawError::InvalidGlyphIndex {
                position,
                character,
                glyph_index,
            })?;
        collect_word_glyph_writes(
            &mut writes,
            glyph,
            FontPoint {
                x: pen_x,
                y: origin.y,
            },
            position,
            color,
        )?;
        pen_x = pen_x.saturating_add(advance);
        draw_width = draw_width.wrapping_add(advance as i16 as u16);
        drawn_glyphs += COUNT_INCREMENT;
    }
    apply_writes(framebuffer, &writes);
    Ok(GameFontDrawOutcome {
        clipped: false,
        consumed_characters: drawn_glyphs,
        drawn_glyphs,
        spaces: usize::MIN,
        skipped_glyphs: usize::MIN,
        draw_width,
        final_pen: FontPoint {
            x: pen_x,
            y: origin.y,
        },
    })
}

/// Draw text through the executable-embedded eight-row main dialogue font.
///
/// This translates `main_font_text_draw_display` at BLOODPRG offset
/// `0x003192`. It retains the vertical gate, six-pixel space movement without
/// width accumulation, high-bit map sentinel, signed advances, transparent
/// rows, and wrapping draw width. Checked slices replace adjacent-table aliasing
/// and all segmented pointers.
pub fn draw_main_font_text(
    framebuffer: &mut [u8],
    fonts: &BloodprgFontResources,
    text: &[u8],
    origin: FontPoint,
    band: FontVerticalBand,
    color: u8,
) -> Result<GameFontDrawOutcome, GameFontDrawError> {
    validate_framebuffer(framebuffer)?;
    validate_vertical_band(band)?;
    if text_origin_is_clipped(origin.y, band, MAIN_FONT_GLYPH_HEIGHT) {
        return Ok(clipped_outcome(origin));
    }

    let mut writes = Vec::new();
    let mut pen_x = origin.x;
    let mut draw_width = u16::MIN;
    let mut consumed_characters = usize::MIN;
    let mut drawn_glyphs = usize::MIN;
    let mut spaces = usize::MIN;
    let mut skipped_glyphs = usize::MIN;
    for (position, character) in text.iter().copied().enumerate() {
        if character == u8::MIN {
            break;
        }
        consumed_characters += COUNT_INCREMENT;
        if character == b' ' {
            pen_x = pen_x.saturating_add(MAIN_FONT_SPACE_ADVANCE);
            spaces += COUNT_INCREMENT;
            continue;
        }
        let glyph_index = mapped_glyph(&fonts.main_character_map, position, character)?;
        if glyph_index & HIGHEST_BYTE_BIT != u8::MIN {
            skipped_glyphs += COUNT_INCREMENT;
            continue;
        }
        let advance = *fonts.main_advances.get(usize::from(glyph_index)).ok_or(
            GameFontDrawError::InvalidGlyphIndex {
                position,
                character,
                glyph_index,
            },
        )? as i8 as i32;
        let glyph_start = usize::from(glyph_index) * MAIN_FONT_GLYPH_HEIGHT as usize;
        let glyph = fonts
            .main_glyphs
            .get(glyph_start..glyph_start + MAIN_FONT_GLYPH_HEIGHT as usize)
            .ok_or(GameFontDrawError::InvalidGlyphIndex {
                position,
                character,
                glyph_index,
            })?;
        collect_byte_glyph_writes(
            &mut writes,
            glyph,
            MAIN_FONT_GLYPH_WIDTH,
            FontPoint {
                x: pen_x,
                y: origin.y,
            },
            position,
            color,
        )?;
        pen_x = pen_x.saturating_add(advance);
        draw_width = draw_width.wrapping_add(advance as i16 as u16);
        drawn_glyphs += COUNT_INCREMENT;
    }
    apply_writes(framebuffer, &writes);
    Ok(GameFontDrawOutcome {
        clipped: false,
        consumed_characters,
        drawn_glyphs,
        spaces,
        skipped_glyphs,
        draw_width,
        final_pen: FontPoint {
            x: pen_x,
            y: origin.y,
        },
    })
}

fn validate_framebuffer(framebuffer: &[u8]) -> Result<(), GameFontDrawError> {
    if framebuffer.len() < LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        return Err(GameFontDrawError::FramebufferTooShort {
            actual: framebuffer.len(),
        });
    }
    Ok(())
}

fn validate_vertical_band(band: FontVerticalBand) -> Result<(), GameFontDrawError> {
    let valid = band.top >= i32::from(u16::MIN)
        && band.top <= band.bottom
        && band.bottom < LOGICAL_FRAMEBUFFER_HEIGHT as i32;
    if !valid {
        return Err(GameFontDrawError::InvalidVerticalBand(band));
    }
    Ok(())
}

fn text_origin_is_clipped(y: i32, band: FontVerticalBand, glyph_height: i32) -> bool {
    y > band.bottom || y <= band.top.saturating_sub(glyph_height)
}

fn clipped_outcome(origin: FontPoint) -> GameFontDrawOutcome {
    GameFontDrawOutcome {
        clipped: true,
        consumed_characters: usize::MIN,
        drawn_glyphs: usize::MIN,
        spaces: usize::MIN,
        skipped_glyphs: usize::MIN,
        draw_width: u16::MIN,
        final_pen: origin,
    }
}

fn mapped_glyph(
    character_map: &[u8],
    position: usize,
    character: u8,
) -> Result<u8, GameFontDrawError> {
    character_map.get(usize::from(character)).copied().ok_or(
        GameFontDrawError::UnsupportedCharacter {
            position,
            character,
        },
    )
}

fn collect_byte_glyph_writes(
    writes: &mut Vec<(usize, u8)>,
    glyph: &[u8],
    width: usize,
    origin: FontPoint,
    position: usize,
    color: u8,
) -> Result<(), GameFontDrawError> {
    for (row, bits) in glyph.iter().copied().enumerate() {
        for column in usize::MIN..width {
            if bits & (HIGHEST_BYTE_BIT >> column as u32) != u8::MIN {
                let x = origin.x.saturating_add(column as i32);
                let y = origin.y.saturating_add(row as i32);
                writes.push((checked_pixel_index(position, x, y)?, color));
            }
        }
    }
    Ok(())
}

fn collect_word_glyph_writes(
    writes: &mut Vec<(usize, u8)>,
    glyph: &[u8],
    origin: FontPoint,
    position: usize,
    color: u8,
) -> Result<(), GameFontDrawError> {
    for (row, bytes) in glyph.chunks_exact(SQUARE_CAPS_ROW_BYTE_COUNT).enumerate() {
        let bits = u16::from(bytes[usize::MIN]) << BITS_PER_BYTE
            | u16::from(bytes[SQUARE_CAPS_ROW_LOW_BYTE_INDEX]);
        for column in usize::MIN..SQUARE_CAPS_GLYPH_WIDTH {
            if bits & (HIGHEST_WORD_BIT >> column as u32) != u16::MIN {
                let x = origin.x.saturating_add(column as i32);
                let y = origin.y.saturating_add(row as i32);
                writes.push((checked_pixel_index(position, x, y)?, color));
            }
        }
    }
    Ok(())
}

fn checked_pixel_index(position: usize, x: i32, y: i32) -> Result<usize, GameFontDrawError> {
    if x < i32::from(u16::MIN)
        || x >= LOGICAL_FRAMEBUFFER_WIDTH as i32
        || y < i32::from(u16::MIN)
        || y >= LOGICAL_FRAMEBUFFER_HEIGHT as i32
    {
        return Err(GameFontDrawError::PixelOutsideDisplay { position, x, y });
    }
    Ok(y as usize * LOGICAL_FRAMEBUFFER_WIDTH + x as usize)
}

fn apply_writes(framebuffer: &mut [u8], writes: &[(usize, u8)]) {
    for (pixel, color) in writes.iter().copied() {
        framebuffer[pixel] = color;
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;
    use commander_blood_formats::bloodprg::decode_bloodprg_font_resources;

    const TEXT_WIDTH_ORACLE_COUNT: usize = 21;
    const BIOS_DRAW_ORACLE_COUNT: usize = 8;
    const BIOS_DRAW_EXACT_HASH_COUNT: usize = 5;
    const SQUARE_CAPS_DRAW_ORACLE_COUNT: usize = 9;
    const SQUARE_CAPS_DRAW_EXACT_HASH_COUNT: usize = 7;
    const MAIN_FONT_DRAW_ORACLE_COUNT: usize = 11;
    const MAIN_FONT_DRAW_EXACT_HASH_COUNT: usize = 8;
    const DOS_SEGMENT_BYTE_COUNT: usize = u16::MAX as usize + 1;
    const FONT_OUTPUT_INDEX_MULTIPLIER: usize = 37;
    const FONT_OUTPUT_CASE_MULTIPLIER: usize = 41;
    const FONT_OUTPUT_COLOR_OFFSET: usize = 43;
    const BIOS_OUTPUT_INDEX_MULTIPLIER: usize = 31;
    const BIOS_OUTPUT_CASE_MULTIPLIER: usize = 43;
    const BIOS_OUTPUT_COLOR_OFFSET: usize = 39;

    #[derive(Deserialize)]
    struct TextWidthOracle {
        selector: u16,
        text: Vec<u8>,
        width_minus_trailing_gap: u16,
    }

    #[derive(Deserialize)]
    struct BiosDrawOracle {
        name: String,
        x: u16,
        y: u16,
        color: u8,
        character_limit: u8,
        characters_drawn: usize,
        display_offset: u16,
        font_offset: u16,
        output_segment_sha256: String,
    }

    #[derive(Deserialize)]
    struct ProportionalDrawOracle {
        name: String,
        x: u16,
        y: u16,
        color: u8,
        clip_top: u16,
        clip_bottom: u16,
        clipped: bool,
        characters_drawn: usize,
        #[serde(default)]
        spaces: usize,
        #[serde(default)]
        skipped: usize,
        display_offset: u16,
        draw_width: u16,
        output_segment_sha256: String,
    }

    #[test]
    fn dual_font_width_matches_every_original_vector() {
        let fonts =
            decode_bloodprg_font_resources(include_bytes!("../../../../../re/bin/BLOODPRG.EXE"))
                .unwrap();
        let vectors: Vec<TextWidthOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_30cd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TEXT_WIDTH_ORACLE_COUNT);

        for vector in vectors {
            let face = if vector.selector == u16::MIN {
                GameFontFace::SquareCaps
            } else {
                GameFontFace::Main
            };
            assert_eq!(
                measure_game_text_width(&vector.text, face, &fonts).unwrap(),
                vector.width_minus_trailing_gap
            );
        }
    }

    #[test]
    fn unsupported_bytes_are_rejected_without_table_aliasing() {
        let fonts =
            decode_bloodprg_font_resources(include_bytes!("../../../../../re/bin/BLOODPRG.EXE"))
                .unwrap();
        let unsupported_character = fonts.square_caps_character_map.len() as u8;

        assert_eq!(
            measure_game_text_width(
                &[b'A', unsupported_character],
                GameFontFace::SquareCaps,
                &fonts,
            ),
            Err(GameFontError::UnsupportedCharacter {
                position: 1,
                character: unsupported_character,
            })
        );
    }

    #[test]
    fn bios_font_draw_matches_every_flat_original_vector() {
        let vectors: Vec<BiosDrawOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3066_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), BIOS_DRAW_ORACLE_COUNT);
        let mut exact_hashes = usize::MIN;

        for (case_index, vector) in vectors.iter().enumerate() {
            let text = bios_oracle_text(&vector.name);
            let font = bios_oracle_font(case_index, vector.font_offset);
            let mut framebuffer = bios_framebuffer(case_index, vector.display_offset);
            let before = framebuffer.clone();
            let result = draw_bios_font_text(
                &mut framebuffer,
                &font,
                &text,
                FontPoint {
                    x: i32::from(vector.x as i16),
                    y: i32::from(vector.y as i16),
                },
                vector.color,
                vector.character_limit,
            );

            if bios_oracle_is_flat(&vector.name) {
                let outcome = result.unwrap();
                assert_eq!(
                    outcome.drawn_glyphs, vector.characters_drawn,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    bios_output_hash(&framebuffer, case_index, vector.display_offset),
                    vector.output_segment_sha256,
                    "{}",
                    vector.name
                );
                exact_hashes += 1;
            } else {
                assert!(
                    matches!(result, Err(GameFontDrawError::PixelOutsideDisplay { .. })),
                    "{}: {result:?}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
            }
        }
        assert_eq!(exact_hashes, BIOS_DRAW_EXACT_HASH_COUNT);
    }

    #[test]
    fn square_caps_draw_matches_every_flat_original_vector() {
        let vectors: Vec<ProportionalDrawOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3106_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), SQUARE_CAPS_DRAW_ORACLE_COUNT);
        let mut exact_hashes = usize::MIN;

        for (case_index, vector) in vectors.iter().enumerate() {
            let text = square_caps_oracle_text(&vector.name);
            let fonts = square_caps_oracle_fonts(&vector.name, case_index);
            let mut framebuffer = font_framebuffer(case_index, vector.display_offset);
            let before = framebuffer.clone();
            let band = oracle_band(vector);
            let result = draw_square_caps_text(
                &mut framebuffer,
                &fonts,
                &text,
                oracle_origin(vector),
                band,
                vector.color,
            );

            if square_caps_oracle_is_flat(&vector.name) {
                let outcome = result.unwrap();
                assert_eq!(outcome.clipped, vector.clipped, "{}", vector.name);
                assert_eq!(
                    outcome.drawn_glyphs, vector.characters_drawn,
                    "{}",
                    vector.name
                );
                assert_eq!(outcome.draw_width, vector.draw_width, "{}", vector.name);
                assert_eq!(
                    font_output_hash(&framebuffer, case_index, vector.display_offset),
                    vector.output_segment_sha256,
                    "{}",
                    vector.name
                );
                exact_hashes += 1;
            } else {
                assert!(
                    matches!(
                        result,
                        Err(GameFontDrawError::InvalidVerticalBand(_))
                            | Err(GameFontDrawError::PixelOutsideDisplay { .. })
                    ),
                    "{}: {result:?}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
            }
        }
        assert_eq!(exact_hashes, SQUARE_CAPS_DRAW_EXACT_HASH_COUNT);
    }

    #[test]
    fn main_font_draw_matches_every_flat_original_vector() {
        let vectors: Vec<ProportionalDrawOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_3192_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), MAIN_FONT_DRAW_ORACLE_COUNT);
        let mut exact_hashes = usize::MIN;

        for (case_index, vector) in vectors.iter().enumerate() {
            let text = main_font_oracle_text(&vector.name);
            let fonts = main_font_oracle_fonts(&vector.name, case_index);
            let mut framebuffer = font_framebuffer(case_index, vector.display_offset);
            let before = framebuffer.clone();
            let result = draw_main_font_text(
                &mut framebuffer,
                &fonts,
                &text,
                oracle_origin(vector),
                oracle_band(vector),
                vector.color,
            );

            if main_font_oracle_is_flat(&vector.name) {
                let outcome = result.unwrap();
                assert_eq!(outcome.clipped, vector.clipped, "{}", vector.name);
                assert_eq!(
                    outcome.drawn_glyphs, vector.characters_drawn,
                    "{}",
                    vector.name
                );
                assert_eq!(outcome.spaces, vector.spaces, "{}", vector.name);
                assert_eq!(outcome.skipped_glyphs, vector.skipped, "{}", vector.name);
                assert_eq!(outcome.draw_width, vector.draw_width, "{}", vector.name);
                assert_eq!(
                    font_output_hash(&framebuffer, case_index, vector.display_offset),
                    vector.output_segment_sha256,
                    "{}",
                    vector.name
                );
                exact_hashes += 1;
            } else {
                assert!(
                    matches!(
                        result,
                        Err(GameFontDrawError::InvalidVerticalBand(_))
                            | Err(GameFontDrawError::PixelOutsideDisplay { .. })
                            | Err(GameFontDrawError::UnsupportedCharacter { .. })
                    ),
                    "{}: {result:?}",
                    vector.name
                );
                assert_eq!(framebuffer, before, "{}", vector.name);
            }
        }
        assert_eq!(exact_hashes, MAIN_FONT_DRAW_EXACT_HASH_COUNT);
    }

    fn bios_oracle_text(name: &str) -> Vec<u8> {
        match name {
            "immediate_nul" => vec![u8::MIN],
            "single_character_count_limit" => b"AB\0".to_vec(),
            "nul_after_two_font_offset_wrap" => vec![1, u8::MAX, u8::MIN],
            "zero_limit_means_256_characters" => (usize::MIN..256)
                .map(|index| (index % 255 + 1) as u8)
                .chain([127, u8::MIN])
                .collect(),
            "text_offset_wrap" => b"CD\0".to_vec(),
            "display_offset_wrap" => b"E\0".to_vec(),
            "zero_color_and_inherited_direction" => b"F\0".to_vec(),
            "full_word_y_uses_byte_swap_row_formula" => b"G\0".to_vec(),
            _ => panic!("unknown BIOS draw oracle {name}"),
        }
    }

    fn bios_oracle_font(case_index: usize, font_offset: u16) -> BiosFont8x8 {
        std::array::from_fn(|character| {
            std::array::from_fn(|row| {
                let address =
                    font_offset.wrapping_add((character * BIOS_FONT_GLYPH_HEIGHT + row) as u16);
                (usize::from(address) * 29 + case_index * 37 + 97) as u8
            })
        })
    }

    fn bios_oracle_is_flat(name: &str) -> bool {
        !matches!(
            name,
            "zero_limit_means_256_characters"
                | "display_offset_wrap"
                | "full_word_y_uses_byte_swap_row_formula"
        )
    }

    fn square_caps_oracle_text(name: &str) -> Vec<u8> {
        match name {
            "above_bottom_clipped_before_text"
            | "top_minus_height_edge_clipped"
            | "single_glyph_bottom_inclusive_and_top_wrap" => b"A\0".to_vec(),
            "empty_string_in_visible_band" => vec![u8::MIN],
            "two_glyphs_variable_advances"
            | "source_offset_wrap"
            | "inherited_backward_source_direction" => b"AB\0".to_vec(),
            "full_word_row_formula_and_output_wrap" => b"C\0".to_vec(),
            "width_accumulator_wrap" => std::iter::repeat_n(b'Z', 300).chain([u8::MIN]).collect(),
            _ => panic!("unknown square-caps oracle {name}"),
        }
    }

    fn square_caps_oracle_fonts(name: &str, case_index: usize) -> BloodprgFontResources {
        let mut fonts = decoded_fonts();
        let entries: &[(u8, u8, u8)] = match name {
            "single_glyph_bottom_inclusive_and_top_wrap" => &[(b'A', 2, 7)],
            "two_glyphs_variable_advances" => &[(b'A', 2, 7), (b'B', 3, 11)],
            "source_offset_wrap" => &[(b'A', 4, 9), (b'B', 6, 13)],
            "inherited_backward_source_direction" => &[(b'A', 7, 5), (b'B', 8, 14)],
            "full_word_row_formula_and_output_wrap" => &[(b'C', 9, 15)],
            "width_accumulator_wrap" => &[(b'Z', 5, u8::MAX)],
            _ => &[],
        };
        for &(character, glyph_index, advance) in entries {
            fonts.square_caps_character_map[usize::from(character)] = glyph_index;
            fonts.square_caps_advances[usize::from(glyph_index)] = advance;
            install_square_caps_oracle_glyph(&mut fonts, glyph_index, case_index);
        }
        fonts
    }

    fn install_square_caps_oracle_glyph(
        fonts: &mut BloodprgFontResources,
        glyph_index: u8,
        case_index: usize,
    ) {
        let patterns = [
            0_u16,
            32_768,
            16_385,
            1,
            u16::MAX,
            240,
            33_025,
            32_766,
            42_405,
            u16::from(glyph_index) * 4_369 + case_index as u16 * 257,
        ];
        let start = usize::from(glyph_index) * SQUARE_CAPS_GLYPH_BYTE_COUNT;
        for (row, bits) in patterns.into_iter().enumerate() {
            let bytes = bits.to_be_bytes();
            fonts.square_caps_glyphs[start + row * 2..start + row * 2 + 2].copy_from_slice(&bytes);
        }
    }

    fn square_caps_oracle_is_flat(name: &str) -> bool {
        !matches!(
            name,
            "full_word_row_formula_and_output_wrap" | "width_accumulator_wrap"
        )
    }

    fn main_font_oracle_text(name: &str) -> Vec<u8> {
        match name {
            "above_bottom_clipped_before_text" | "top_minus_height_edge_clipped" => b"A\0".to_vec(),
            "empty_string_in_visible_band" => vec![u8::MIN],
            "space_moves_pen_without_width" => b" A\0".to_vec(),
            "unmapped_byte_skips_without_moving_pen" => b"XA\0".to_vec(),
            "two_glyphs_signed_advances_and_zero_color"
            | "source_offset_wrap"
            | "inherited_backward_source_direction" => b"AB\0".to_vec(),
            "full_word_row_formula_and_output_wrap" => b"C\0".to_vec(),
            "width_accumulator_wrap" => std::iter::repeat_n(b'Z', 300).chain([u8::MIN]).collect(),
            "high_byte_indexes_past_nominal_map_extent" => vec![254, u8::MIN],
            _ => panic!("unknown main-font oracle {name}"),
        }
    }

    fn main_font_oracle_fonts(name: &str, case_index: usize) -> BloodprgFontResources {
        let mut fonts = decoded_fonts();
        let entries: &[(u8, u8, u8)] = match name {
            "space_moves_pen_without_width" => &[(b'A', 2, 7)],
            "unmapped_byte_skips_without_moving_pen" => &[(b'X', u8::MAX, 0), (b'A', 3, 11)],
            "two_glyphs_signed_advances_and_zero_color" => &[(b'A', 4, 7), (b'B', 6, 251)],
            "source_offset_wrap" => &[(b'A', 7, 9), (b'B', 8, 13)],
            "inherited_backward_source_direction" => &[(b'A', 9, 5), (b'B', 10, 14)],
            "full_word_row_formula_and_output_wrap" => &[(b'C', 11, 15)],
            "width_accumulator_wrap" => &[(b'Z', 5, u8::MAX)],
            _ => &[],
        };
        for &(character, glyph_index, advance) in entries {
            fonts.main_character_map[usize::from(character)] = glyph_index;
            if glyph_index & HIGHEST_BYTE_BIT == u8::MIN {
                fonts.main_advances[usize::from(glyph_index)] = advance;
                install_main_font_oracle_glyph(&mut fonts, glyph_index, case_index);
            }
        }
        fonts
    }

    fn install_main_font_oracle_glyph(
        fonts: &mut BloodprgFontResources,
        glyph_index: u8,
        case_index: usize,
    ) {
        let patterns = [
            u8::MIN,
            128,
            65,
            1,
            u8::MAX,
            15,
            129,
            (usize::from(glyph_index) * 17 + case_index * 7) as u8,
        ];
        let start = usize::from(glyph_index) * MAIN_FONT_GLYPH_HEIGHT as usize;
        fonts.main_glyphs[start..start + patterns.len()].copy_from_slice(&patterns);
    }

    fn main_font_oracle_is_flat(name: &str) -> bool {
        !matches!(
            name,
            "full_word_row_formula_and_output_wrap"
                | "width_accumulator_wrap"
                | "high_byte_indexes_past_nominal_map_extent"
        )
    }

    fn decoded_fonts() -> BloodprgFontResources {
        decode_bloodprg_font_resources(include_bytes!("../../../../../re/bin/BLOODPRG.EXE"))
            .unwrap()
    }

    fn oracle_origin(vector: &ProportionalDrawOracle) -> FontPoint {
        FontPoint {
            x: i32::from(vector.x as i16),
            y: i32::from(vector.y as i16),
        }
    }

    fn oracle_band(vector: &ProportionalDrawOracle) -> FontVerticalBand {
        FontVerticalBand {
            top: i32::from(vector.clip_top as i16),
            bottom: i32::from(vector.clip_bottom as i16),
        }
    }

    fn font_output_seed(case_index: usize) -> Vec<u8> {
        (usize::MIN..DOS_SEGMENT_BYTE_COUNT)
            .map(|index| {
                (index * FONT_OUTPUT_INDEX_MULTIPLIER
                    + case_index * FONT_OUTPUT_CASE_MULTIPLIER
                    + FONT_OUTPUT_COLOR_OFFSET) as u8
            })
            .collect()
    }

    fn font_framebuffer(case_index: usize, display_offset: u16) -> Vec<u8> {
        mapped_framebuffer(font_output_seed(case_index), display_offset)
    }

    fn font_output_hash(framebuffer: &[u8], case_index: usize, display_offset: u16) -> String {
        mapped_output_hash(framebuffer, font_output_seed(case_index), display_offset)
    }

    fn bios_output_seed(case_index: usize) -> Vec<u8> {
        (usize::MIN..DOS_SEGMENT_BYTE_COUNT)
            .map(|index| {
                (index * BIOS_OUTPUT_INDEX_MULTIPLIER
                    + case_index * BIOS_OUTPUT_CASE_MULTIPLIER
                    + BIOS_OUTPUT_COLOR_OFFSET) as u8
            })
            .collect()
    }

    fn bios_framebuffer(case_index: usize, display_offset: u16) -> Vec<u8> {
        mapped_framebuffer(bios_output_seed(case_index), display_offset)
    }

    fn bios_output_hash(framebuffer: &[u8], case_index: usize, display_offset: u16) -> String {
        mapped_output_hash(framebuffer, bios_output_seed(case_index), display_offset)
    }

    fn mapped_framebuffer(output: Vec<u8>, display_offset: u16) -> Vec<u8> {
        (usize::MIN..LOGICAL_FRAMEBUFFER_PIXEL_COUNT)
            .map(|index| output[(usize::from(display_offset) + index) % DOS_SEGMENT_BYTE_COUNT])
            .collect()
    }

    fn mapped_output_hash(framebuffer: &[u8], mut output: Vec<u8>, display_offset: u16) -> String {
        for (index, pixel) in framebuffer.iter().copied().enumerate() {
            output[(usize::from(display_offset) + index) % DOS_SEGMENT_BYTE_COUNT] = pixel;
        }
        format!("{:x}", Sha256::digest(output))
    }
}
