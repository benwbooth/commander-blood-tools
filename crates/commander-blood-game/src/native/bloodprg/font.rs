//! Measurement and rasterization support for executable-embedded game fonts.

use std::error::Error;
use std::fmt;

use commander_blood_formats::bloodprg::BloodprgFontResources;

const TRAILING_INTERCHARACTER_GAP: u16 = 2;

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

impl fmt::Display for GameFontError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "invalid Commander Blood font operation: {self:?}"
        )
    }
}

impl Error for GameFontError {}

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

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;
    use commander_blood_formats::bloodprg::decode_bloodprg_font_resources;

    const TEXT_WIDTH_ORACLE_COUNT: usize = 21;

    #[derive(Deserialize)]
    struct TextWidthOracle {
        selector: u16,
        text: Vec<u8>,
        width_minus_trailing_gap: u16,
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
}
