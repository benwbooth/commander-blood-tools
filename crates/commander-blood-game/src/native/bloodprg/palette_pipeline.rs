//! Palette interpolation and indexed-color remap construction.

use std::error::Error;
use std::fmt;
use std::ops::RangeInclusive;

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

use super::IndexedGamePalette;

/// Complete index-to-index palette remap table.
pub type PaletteRemapTable = [u8; PALETTE_ENTRY_COUNT];

/// Number of colors in one authored tint bank.
pub const TINT_PALETTE_BANK_SIZE: usize = 16;
/// Number of leading scene-palette colors cleared between presentations.
pub const SCENE_PALETTE_CLEAR_COLOR_COUNT: usize = 192;

const PERCENT_SCALE: u16 = 100;
const PALETTE_BLEND_DISTANCE_LIMIT: u16 = 3_000;
const TINT_RED_WEIGHT: u16 = 3;
const TINT_GREEN_WEIGHT: u16 = 6;
const TINT_BLUE_WEIGHT: u16 = 1;
const TINT_WEIGHT_DIVISOR: u16 = 28;
const TINT_MAXIMUM_SHADE: u16 = TINT_PALETTE_BANK_SIZE as u16 - 1;

/// Invalid palette operation request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PalettePipelineError {
    /// Blend percentages are constrained to the original zero-to-100 domain.
    InvalidBlendPercent(u8),
    /// An inclusive interpolation range was empty or outside the palette.
    InvalidPaletteRange {
        /// Requested first color.
        first: usize,
        /// Requested last color.
        last: usize,
    },
    /// A complete tint bank would extend beyond the palette.
    InvalidTintBank(u8),
}

impl fmt::Display for PalettePipelineError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid palette pipeline request: {self:?}")
    }
}

impl Error for PalettePipelineError {}

/// Build the nearest-color remap for one RGB blend target.
///
/// This translates `palette_blend_remap_table_build` at BLOODPRG routine
/// offset `0x0022E0`. The original 16-bit wrapped distance calculation,
/// 3,000-distance acceptance threshold, and later-index tie selection are
/// preserved over typed palette arrays. Entries with no accepted color retain
/// their prior value.
pub fn build_palette_blend_remap_table(
    palette: &IndexedGamePalette,
    table: &mut PaletteRemapTable,
    blend_percent: u8,
    target: [u8; RGB_COMPONENT_COUNT],
) -> Result<(), PalettePipelineError> {
    if u16::from(blend_percent) > PERCENT_SCALE {
        return Err(PalettePipelineError::InvalidBlendPercent(blend_percent));
    }

    let percent = u16::from(blend_percent);
    let source_weight = PERCENT_SCALE - percent;
    let scaled_target = target.map(|component| percent * u16::from(component) / PERCENT_SCALE);

    for (source_index, source_color) in palette.iter().enumerate() {
        let blended: [u16; RGB_COMPONENT_COUNT] = std::array::from_fn(|component| {
            u16::from(source_color[component]) * source_weight / PERCENT_SCALE
                + scaled_target[component]
        });
        let mut best_index = None;
        let mut best_distance = PALETTE_BLEND_DISTANCE_LIMIT;

        for (candidate_index, candidate_color) in palette.iter().enumerate() {
            let mut distance = u16::MIN;
            for component in 0..RGB_COMPONENT_COUNT {
                let difference =
                    blended[component].wrapping_sub(u16::from(candidate_color[component]));
                let magnitude = if (difference as i16).is_negative() {
                    difference.wrapping_neg()
                } else {
                    difference
                };
                distance = distance.wrapping_add(magnitude.wrapping_mul(magnitude));
            }

            if distance <= best_distance {
                best_distance = distance;
                best_index = Some(candidate_index as u8);
            }
        }

        if let Some(best_index) = best_index {
            table[source_index] = best_index;
        }
    }
    Ok(())
}

/// Interpolate an inclusive palette range from target toward source.
///
/// This translates `palette_range_interpolate` at BLOODPRG routine offset
/// `0x0023C5`. Signed-byte component differences and signed division toward
/// zero are retained, including negative percentages, while checked array
/// ranges replace unchecked byte cursors.
pub fn interpolate_palette_range(
    source: &IndexedGamePalette,
    target: &IndexedGamePalette,
    destination: &mut IndexedGamePalette,
    percent: i8,
    colors: RangeInclusive<usize>,
) -> Result<(), PalettePipelineError> {
    let first = *colors.start();
    let last = *colors.end();
    if first > last || last >= PALETTE_ENTRY_COUNT {
        return Err(PalettePipelineError::InvalidPaletteRange { first, last });
    }

    for color_index in colors {
        for component in 0..RGB_COMPONENT_COUNT {
            let source_value = source[color_index][component];
            let target_value = target[color_index][component];
            let delta = source_value.wrapping_sub(target_value) as i8;
            let adjustment = i16::from(delta) * i16::from(percent) / PERCENT_SCALE as i16;
            destination[color_index][component] = target_value.wrapping_add(adjustment as u8);
        }
    }
    Ok(())
}

/// Build a luminance tint table in one 16-color palette bank.
///
/// This translates `tint_table_build_banked` at BLOODPRG routine offset
/// `0x00242D`. Its weighted `(3R + 6G + B) / 28` shade and 15 clamp are exact;
/// colors already inside the selected bank map to themselves.
pub fn build_banked_tint_table(
    palette: &IndexedGamePalette,
    table: &mut PaletteRemapTable,
    bank_first: u8,
) -> Result<(), PalettePipelineError> {
    let bank_first_index = usize::from(bank_first);
    let Some(bank_end) = bank_first_index.checked_add(TINT_PALETTE_BANK_SIZE) else {
        return Err(PalettePipelineError::InvalidTintBank(bank_first));
    };
    if bank_end > PALETTE_ENTRY_COUNT {
        return Err(PalettePipelineError::InvalidTintBank(bank_first));
    }

    for (index, color) in palette.iter().enumerate() {
        let weighted = u16::from(color[0]) * TINT_RED_WEIGHT
            + u16::from(color[1]) * TINT_GREEN_WEIGHT
            + u16::from(color[2]) * TINT_BLUE_WEIGHT;
        let shade = (weighted / TINT_WEIGHT_DIVISOR).min(TINT_MAXIMUM_SHADE);
        let mapped = if (bank_first_index..bank_end).contains(&index) {
            index
        } else {
            bank_first_index + usize::from(shade)
        };
        table[index] = mapped as u8;
    }
    Ok(())
}

/// Clear the first 192 scene colors while preserving the upper palette bank.
///
/// This translates `palette_scene_entries_clear` at BLOODPRG routine offset
/// `0x00248B`. Forward typed array filling is the complete modern behavior;
/// inherited processor direction state has no runtime representation.
pub fn clear_scene_palette_entries(palette: &mut IndexedGamePalette) {
    palette[..SCENE_PALETTE_CLEAR_COLOR_COUNT].fill([u8::MIN; RGB_COMPONENT_COUNT]);
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const BLEND_VECTOR_COUNT: usize = 4;
    const INTERPOLATION_VECTOR_COUNT: usize = 5;
    const TINT_VECTOR_COUNT: usize = 4;
    const CLEAR_VECTOR_COUNT: usize = 4;
    const FLAT_PALETTE_BYTE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
    const PALETTE_COMPONENT_MASK: usize = 63;
    const BLEND_TABLE_SEED: usize = 49;
    const BLEND_TABLE_CASE_STEP: usize = 23;
    const BLEND_TABLE_ENTRY_STEP: usize = 37;
    const INTERPOLATION_DESTINATION_SEED: usize = 113;
    const INTERPOLATION_CASE_STEP: usize = 13;
    const INTERPOLATION_ENTRY_STEP: usize = 17;
    const CLEAR_WINDOW_SIZE: usize = 1_792;
    const CLEAR_PALETTE_WINDOW_INDEX: usize = 849;
    const CLEAR_PATTERN_ENTRY_STEP: usize = 45;
    const CLEAR_PATTERN_GROUP_SHIFT: usize = 3;
    const ASCENDING_CLEAR_VECTOR_COUNT: usize = 3;
    const DESCENDING_CLEAR_VECTOR_COUNT: usize = 1;

    #[derive(Deserialize)]
    struct BlendOracle {
        name: String,
        percent: u8,
        target: [u8; RGB_COMPONENT_COUNT],
        palette_sha256: String,
        initial_table_sha256: String,
        result_table_sha256: String,
        changed_entries: usize,
        distinct_results: usize,
    }

    #[derive(Deserialize)]
    struct InterpolationOracle {
        name: String,
        percent: i8,
        first: usize,
        last: usize,
        updated_components: usize,
        result_palette_sha256: String,
    }

    #[derive(Deserialize)]
    struct TintOracle {
        name: String,
        bank_base: u8,
        palette_sha256: String,
        result_table_sha256: String,
        result_min: u8,
        result_max: u8,
        distinct_results: usize,
        identity_entries: usize,
    }

    #[derive(Deserialize)]
    struct ClearOracle {
        name: String,
        direction: String,
        seed: u8,
        palette_entries_cleared: Option<usize>,
        upper_palette_entries_preserved: Option<usize>,
        palette_before_sha256: String,
        palette_after_sha256: String,
    }

    #[test]
    fn blend_remap_matches_every_original_palette_vector() {
        let vectors: Vec<BlendOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_22e0_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), BLEND_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let palette = blend_palette(&vector.name);
            let mut table = std::array::from_fn(|index| {
                (BLEND_TABLE_SEED
                    + case_index * BLEND_TABLE_CASE_STEP
                    + index * BLEND_TABLE_ENTRY_STEP) as u8
            });
            let before = table;
            assert_eq!(
                palette_hash(&palette),
                vector.palette_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                byte_hash(&table),
                vector.initial_table_sha256,
                "{}",
                vector.name
            );

            build_palette_blend_remap_table(&palette, &mut table, vector.percent, vector.target)
                .unwrap();

            assert_eq!(
                byte_hash(&table),
                vector.result_table_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                table
                    .iter()
                    .zip(before)
                    .filter(|(left, right)| **left != *right)
                    .count(),
                vector.changed_entries,
                "{}",
                vector.name
            );
            let distinct = table
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(distinct.len(), vector.distinct_results, "{}", vector.name);
        }
    }

    #[test]
    fn range_interpolation_matches_every_original_palette_vector() {
        let vectors: Vec<InterpolationOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_23c5_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), INTERPOLATION_VECTOR_COUNT);
        let source = interpolation_source_palette();
        let target = interpolation_target_palette();

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let mut destination = patterned_palette(
                INTERPOLATION_DESTINATION_SEED + case_index * INTERPOLATION_CASE_STEP,
                INTERPOLATION_ENTRY_STEP,
            );

            interpolate_palette_range(
                &source,
                &target,
                &mut destination,
                vector.percent,
                vector.first..=vector.last,
            )
            .unwrap();

            assert_eq!(
                (vector.last - vector.first + 1) * RGB_COMPONENT_COUNT,
                vector.updated_components,
                "{}",
                vector.name
            );
            assert_eq!(
                palette_hash(&destination),
                vector.result_palette_sha256,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn banked_tint_matches_every_original_palette_vector() {
        let vectors: Vec<TintOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_242d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), TINT_VECTOR_COUNT);
        let palette_biases = [0, 7, 19, 31];

        for (vector, palette_bias) in vectors.into_iter().zip(palette_biases) {
            let palette = tint_palette(palette_bias);
            let mut table = [u8::MIN; PALETTE_ENTRY_COUNT];
            assert_eq!(
                palette_hash(&palette),
                vector.palette_sha256,
                "{}",
                vector.name
            );

            build_banked_tint_table(&palette, &mut table, vector.bank_base).unwrap();

            assert_eq!(
                byte_hash(&table),
                vector.result_table_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                table.iter().copied().min(),
                Some(vector.result_min),
                "{}",
                vector.name
            );
            assert_eq!(
                table.iter().copied().max(),
                Some(vector.result_max),
                "{}",
                vector.name
            );
            let distinct = table
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>();
            assert_eq!(distinct.len(), vector.distinct_results, "{}", vector.name);
            assert_eq!(
                table
                    .iter()
                    .enumerate()
                    .filter(|(index, value)| **value as usize == *index)
                    .count(),
                vector.identity_entries,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn scene_clear_matches_forward_vectors_and_discards_direction_state() {
        let vectors: Vec<ClearOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_248b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), CLEAR_VECTOR_COUNT);
        let mut ascending_count = usize::MIN;
        let mut descending_count = usize::MIN;

        for vector in vectors {
            if vector.direction == "descending" {
                descending_count += 1;
                continue;
            }
            ascending_count += 1;
            let mut palette = clear_palette(vector.seed);
            let upper_before = palette[SCENE_PALETTE_CLEAR_COLOR_COUNT..].to_vec();
            assert_eq!(
                palette_hash(&palette),
                vector.palette_before_sha256,
                "{}",
                vector.name
            );

            clear_scene_palette_entries(&mut palette);

            assert_eq!(
                palette_hash(&palette),
                vector.palette_after_sha256,
                "{}",
                vector.name
            );
            assert_eq!(
                vector.palette_entries_cleared,
                Some(SCENE_PALETTE_CLEAR_COLOR_COUNT),
                "{}",
                vector.name
            );
            assert_eq!(
                vector.upper_palette_entries_preserved,
                Some(PALETTE_ENTRY_COUNT - SCENE_PALETTE_CLEAR_COLOR_COUNT),
                "{}",
                vector.name
            );
            assert_eq!(
                palette[SCENE_PALETTE_CLEAR_COLOR_COUNT..],
                upper_before,
                "{}",
                vector.name
            );
        }

        assert_eq!(ascending_count, ASCENDING_CLEAR_VECTOR_COUNT);
        assert_eq!(descending_count, DESCENDING_CLEAR_VECTOR_COUNT);
    }

    #[test]
    fn invalid_flat_palette_requests_fail_before_mutation() {
        let palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        let mut table = [91; PALETTE_ENTRY_COUNT];
        let table_before = table;
        assert!(matches!(
            build_palette_blend_remap_table(
                &palette,
                &mut table,
                101,
                [u8::MIN; RGB_COMPONENT_COUNT]
            ),
            Err(PalettePipelineError::InvalidBlendPercent(101))
        ));
        assert_eq!(table, table_before);
        assert!(matches!(
            build_banked_tint_table(&palette, &mut table, 241),
            Err(PalettePipelineError::InvalidTintBank(241))
        ));

        let mut destination = palette;
        assert!(matches!(
            interpolate_palette_range(
                &palette,
                &palette,
                &mut destination,
                50,
                invalid_palette_range(),
            ),
            Err(PalettePipelineError::InvalidPaletteRange { .. })
        ));
        assert_eq!(destination, palette);
    }

    fn blend_palette(name: &str) -> IndexedGamePalette {
        match name {
            "zero_percent_later_ties" => std::array::from_fn(|index| {
                [
                    (index & PALETTE_COMPONENT_MASK) as u8,
                    ((index * TINT_RED_WEIGHT as usize) & PALETTE_COMPONENT_MASK) as u8,
                    ((index * 5) & PALETTE_COMPONENT_MASK) as u8,
                ]
            }),
            "half_darkening" | "full_target" => std::array::from_fn(|index| {
                [
                    (index & PALETTE_COMPONENT_MASK) as u8,
                    (index >> TINT_GREEN_WEIGHT) as u8,
                    ((index * INTERPOLATION_ENTRY_STEP + 9) & PALETTE_COMPONENT_MASK) as u8,
                ]
            }),
            "no_match_preserves" => [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT],
            other => panic!("unknown blend oracle {other}"),
        }
    }

    fn interpolation_source_palette() -> IndexedGamePalette {
        std::array::from_fn(|index| {
            [
                ((index * 7 + 3) & PALETTE_COMPONENT_MASK) as u8,
                ((index * 11 + 19) & PALETTE_COMPONENT_MASK) as u8,
                ((index * 13 + 37) & PALETTE_COMPONENT_MASK) as u8,
            ]
        })
    }

    fn interpolation_target_palette() -> IndexedGamePalette {
        std::array::from_fn(|index| {
            [
                (63usize.wrapping_sub(index * 5) & PALETTE_COMPONENT_MASK) as u8,
                (41usize.wrapping_sub(index * 9) & PALETTE_COMPONENT_MASK) as u8,
                (23usize.wrapping_sub(index * 15) & PALETTE_COMPONENT_MASK) as u8,
            ]
        })
    }

    fn tint_palette(bias: usize) -> IndexedGamePalette {
        std::array::from_fn(|index| {
            [
                ((index + bias) & PALETTE_COMPONENT_MASK) as u8,
                ((index * 3 + bias * 2) & PALETTE_COMPONENT_MASK) as u8,
                ((index * 5 + bias * 3) & PALETTE_COMPONENT_MASK) as u8,
            ]
        })
    }

    fn patterned_palette(seed: usize, step: usize) -> IndexedGamePalette {
        palette_from_bytes(std::array::from_fn(|index| (seed + index * step) as u8))
    }

    fn clear_palette(seed: u8) -> IndexedGamePalette {
        let window: [u8; CLEAR_WINDOW_SIZE] = if seed == u8::MIN {
            [u8::MIN; CLEAR_WINDOW_SIZE]
        } else if seed == u8::MAX {
            [u8::MAX; CLEAR_WINDOW_SIZE]
        } else {
            std::array::from_fn(|index| {
                seed.wrapping_add((index * CLEAR_PATTERN_ENTRY_STEP) as u8)
                    .wrapping_add((index >> CLEAR_PATTERN_GROUP_SHIFT) as u8)
            })
        };
        palette_from_bytes(
            window
                [CLEAR_PALETTE_WINDOW_INDEX..CLEAR_PALETTE_WINDOW_INDEX + FLAT_PALETTE_BYTE_COUNT]
                .try_into()
                .unwrap(),
        )
    }

    fn palette_from_bytes(bytes: [u8; FLAT_PALETTE_BYTE_COUNT]) -> IndexedGamePalette {
        std::array::from_fn(|index| {
            bytes[index * RGB_COMPONENT_COUNT..(index + 1) * RGB_COMPONENT_COUNT]
                .try_into()
                .unwrap()
        })
    }

    fn palette_hash(palette: &IndexedGamePalette) -> String {
        byte_hash(
            &palette
                .iter()
                .flat_map(|color| color.iter().copied())
                .collect::<Vec<_>>(),
        )
    }

    fn byte_hash(bytes: &[u8]) -> String {
        format!("{:x}", Sha256::digest(bytes))
    }

    fn invalid_palette_range() -> RangeInclusive<usize> {
        let first = 20;
        let last = 19;
        first..=last
    }
}
