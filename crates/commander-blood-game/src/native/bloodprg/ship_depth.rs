//! Ship-view depth transition and band-composition rules.

const ACTIVE_FLAG: u16 = 1;
const FULLY_OPEN_DEPTH: u16 = 65;
const TRANSITION_INCREMENT_HOLD: u16 = 10;
const MAXIMUM_TRANSITION_PERCENT: u16 = 100;
const BAND_DEPTH_BIAS: u16 = 35;
const BAND_ROW_BYTES: u16 = 80;
const BAND_SOURCE_SPLIT: u16 = 57_152;
const BAND_DESTINATION_SPLIT: u16 = 16_000;
const LOGICAL_FRAMEBUFFER_HALF_HEIGHT: u16 = 100;
const LOGICAL_FRAMEBUFFER_HEIGHT: u16 = 200;

/// Mutable depth-door state retained by the bridge presentation coordinator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ShipDepthTransition {
    /// Current vertical depth offset.
    pub depth: u16,
    /// Native opening flags; bit zero owns the transition.
    pub opening_flags: u16,
    /// Native closing flags; bit zero owns the transition.
    pub closing_flags: u16,
    /// Low-byte depth movement applied per frame.
    pub step: u8,
}

/// Observable path selected by one depth transition step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShipDepthTransitionOutcome {
    /// Neither transition bit was active.
    Inactive,
    /// The opening flag was cleared from an already complete transition.
    OpeningCompleted,
    /// The opening low byte advanced and the result was stored or clamped.
    OpeningAdvanced,
    /// The closing flag was cleared from an already complete transition.
    ClosingCompleted,
    /// The closing low byte retreated and the result was stored or clamped.
    ClosingAdvanced,
}

/// Advance the ship-view depth door exactly as native routine `0x00B75C`.
///
/// Only the low byte participates in movement, while signed tests inspect the
/// reconstructed word or byte. Wrapping operations are therefore retained as
/// gameplay behavior without retaining a packed memory representation.
pub fn advance_ship_depth(state: &mut ShipDepthTransition) -> ShipDepthTransitionOutcome {
    if state.opening_flags & ACTIVE_FLAG != u16::MIN {
        if state.depth == FULLY_OPEN_DEPTH {
            state.opening_flags = u16::MIN;
            return ShipDepthTransitionOutcome::OpeningCompleted;
        }

        let [low, high] = state.depth.to_le_bytes();
        let candidate = u16::from_le_bytes([low.wrapping_add(state.step), high]);
        state.depth = if (candidate as i16) < FULLY_OPEN_DEPTH as i16 {
            candidate
        } else {
            FULLY_OPEN_DEPTH
        };
        return ShipDepthTransitionOutcome::OpeningAdvanced;
    }

    if state.closing_flags & ACTIVE_FLAG == u16::MIN {
        return ShipDepthTransitionOutcome::Inactive;
    }
    if state.depth == u16::MIN {
        state.closing_flags = u16::MIN;
        return ShipDepthTransitionOutcome::ClosingCompleted;
    }

    let [low, high] = state.depth.to_le_bytes();
    let next_low = low.wrapping_sub(state.step);
    state.depth = if (next_low as i8).is_negative() {
        u16::MIN
    } else {
        u16::from_le_bytes([next_low, high])
    };
    ShipDepthTransitionOutcome::ClosingAdvanced
}

/// Flat render intent produced by the native two-band Mode X copy.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ShipDepthBandLayout {
    /// Number of bytes in each original planar band.
    pub byte_count: u16,
    /// Start of the first band in the captured source page.
    pub first_source: u16,
    /// Start of the second band in the captured source page.
    pub second_source: u16,
    /// Start of the first band in the destination page.
    pub first_destination: u16,
    /// Start of the second band in the destination page.
    pub second_destination: u16,
}

/// Row-major form of the two-band copy used by the flat modern framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct ShipDepthBandRows {
    /// Number of rows copied by each band.
    pub(crate) row_count: u16,
    /// First source row copied into the top of the display.
    pub(crate) upper_source_start: u16,
    /// First source row copied into the bottom of the display.
    pub(crate) lower_source_start: u16,
    /// First destination row of the upper band.
    pub(crate) upper_destination_start: u16,
    /// First destination row of the lower band.
    pub(crate) lower_destination_start: u16,
}

impl ShipDepthBandLayout {
    /// Convert the native planar-byte result into logical 320-pixel rows.
    ///
    /// Only layouts targeting the start of one complete framebuffer are
    /// accepted. The original write-mode-one copy moved all four VGA planes,
    /// making each 80-byte planar row exactly one 320-pixel logical row.
    pub(crate) fn logical_rows(self) -> Option<ShipDepthBandRows> {
        if self.first_destination != u16::MIN
            || !self.byte_count.is_multiple_of(BAND_ROW_BYTES)
            || self.second_source != BAND_SOURCE_SPLIT
            || self.first_source != BAND_SOURCE_SPLIT.wrapping_sub(self.byte_count)
            || self.second_destination != BAND_DESTINATION_SPLIT.wrapping_sub(self.byte_count)
        {
            return None;
        }

        let row_count = self.byte_count / BAND_ROW_BYTES;
        if row_count > LOGICAL_FRAMEBUFFER_HALF_HEIGHT {
            return None;
        }
        Some(ShipDepthBandRows {
            row_count,
            upper_source_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT - row_count,
            lower_source_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT,
            upper_destination_start: u16::MIN,
            lower_destination_start: LOGICAL_FRAMEBUFFER_HEIGHT - row_count,
        })
    }
}

/// Prepare the visible band transition from native routine `0x00B6DD`.
///
/// The returned ranges preserve the original low-byte row count and wrapping
/// arithmetic. A modern renderer may express the two copies as texture regions;
/// no VGA write mode, plane mask, aperture, or segment identity is represented.
pub fn prepare_ship_depth_band(
    crop_flags: u16,
    depth: u16,
    transition_increment: u16,
    transition_percent: &mut u16,
    destination_base: u16,
) -> Option<ShipDepthBandLayout> {
    if crop_flags & ACTIVE_FLAG == u16::MIN {
        return None;
    }

    if transition_increment != TRANSITION_INCREMENT_HOLD {
        let doubled_depth = depth.wrapping_add(depth);
        let clamped_depth = if (doubled_depth as i16) > MAXIMUM_TRANSITION_PERCENT as i16 {
            MAXIMUM_TRANSITION_PERCENT
        } else {
            doubled_depth
        };
        *transition_percent = MAXIMUM_TRANSITION_PERCENT.wrapping_sub(clamped_depth);
    }

    let band_rows = depth.wrapping_add(BAND_DEPTH_BIAS) as u8;
    let byte_count = u16::from(band_rows).wrapping_mul(BAND_ROW_BYTES);
    Some(ShipDepthBandLayout {
        byte_count,
        first_source: BAND_SOURCE_SPLIT.wrapping_sub(byte_count),
        second_source: BAND_SOURCE_SPLIT,
        first_destination: destination_base,
        second_destination: destination_base
            .wrapping_add(BAND_DESTINATION_SPLIT.wrapping_sub(byte_count)),
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct DepthVector {
        name: String,
        depth_before: u16,
        opening_before: u16,
        closing_before: u16,
        step: u8,
        depth_after: u16,
        opening_after: u16,
        closing_after: u16,
        path: String,
    }

    #[derive(Deserialize)]
    struct BandVector {
        name: String,
        gate: u16,
        depth: u16,
        transition_increment: u16,
        percent_before: u16,
        percent_after: u16,
        byte_count: u16,
        first_source_offset: Option<u16>,
        second_source_offset: Option<u16>,
        destination_offset: u16,
        second_destination_offset: Option<u16>,
    }

    #[test]
    fn depth_transition_matches_every_original_vector() {
        let vectors: Vec<DepthVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b75c_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 17);
        for vector in vectors {
            let mut state = ShipDepthTransition {
                depth: vector.depth_before,
                opening_flags: vector.opening_before,
                closing_flags: vector.closing_before,
                step: vector.step,
            };
            let outcome = advance_ship_depth(&mut state);
            assert_eq!(state.depth, vector.depth_after, "{}", vector.name);
            assert_eq!(state.opening_flags, vector.opening_after, "{}", vector.name);
            assert_eq!(state.closing_flags, vector.closing_after, "{}", vector.name);
            assert_eq!(outcome, outcome_for_path(&vector.path), "{}", vector.name);
        }
    }

    #[test]
    fn band_layout_matches_every_original_vector() {
        let vectors: Vec<BandVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_b6dd_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 12);
        for vector in vectors {
            let mut percent = vector.percent_before;
            let layout = prepare_ship_depth_band(
                vector.gate,
                vector.depth,
                vector.transition_increment,
                &mut percent,
                vector.destination_offset,
            );
            assert_eq!(percent, vector.percent_after, "{}", vector.name);
            let Some(layout) = layout else {
                assert_eq!(vector.byte_count, u16::MIN, "{}", vector.name);
                assert!(vector.first_source_offset.is_none(), "{}", vector.name);
                assert!(vector.second_source_offset.is_none(), "{}", vector.name);
                assert!(
                    vector.second_destination_offset.is_none(),
                    "{}",
                    vector.name
                );
                continue;
            };
            assert_eq!(layout.byte_count, vector.byte_count, "{}", vector.name);
            assert_eq!(
                Some(layout.first_source),
                vector.first_source_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                Some(layout.second_source),
                vector.second_source_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                layout.first_destination, vector.destination_offset,
                "{}",
                vector.name
            );
            assert_eq!(
                Some(layout.second_destination),
                vector.second_destination_offset,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn shipped_depth_layouts_convert_to_flat_logical_rows() {
        let mut percent = u16::MIN;
        let closed = prepare_ship_depth_band(
            ACTIVE_FLAG,
            u16::MIN,
            TRANSITION_INCREMENT_HOLD,
            &mut percent,
            u16::MIN,
        )
        .unwrap()
        .logical_rows()
        .unwrap();
        assert_eq!(
            closed,
            ShipDepthBandRows {
                row_count: BAND_DEPTH_BIAS,
                upper_source_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT - BAND_DEPTH_BIAS,
                lower_source_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT,
                upper_destination_start: u16::MIN,
                lower_destination_start: LOGICAL_FRAMEBUFFER_HEIGHT - BAND_DEPTH_BIAS,
            }
        );

        let open = prepare_ship_depth_band(
            ACTIVE_FLAG,
            FULLY_OPEN_DEPTH,
            TRANSITION_INCREMENT_HOLD,
            &mut percent,
            u16::MIN,
        )
        .unwrap()
        .logical_rows()
        .unwrap();
        assert_eq!(
            open,
            ShipDepthBandRows {
                row_count: LOGICAL_FRAMEBUFFER_HALF_HEIGHT,
                upper_source_start: u16::MIN,
                lower_source_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT,
                upper_destination_start: u16::MIN,
                lower_destination_start: LOGICAL_FRAMEBUFFER_HALF_HEIGHT,
            }
        );
    }

    fn outcome_for_path(path: &str) -> ShipDepthTransitionOutcome {
        match path {
            "inactive" => ShipDepthTransitionOutcome::Inactive,
            "open_clear" => ShipDepthTransitionOutcome::OpeningCompleted,
            "open_store" => ShipDepthTransitionOutcome::OpeningAdvanced,
            "close_clear" => ShipDepthTransitionOutcome::ClosingCompleted,
            "close_store" | "close_store_zero" => ShipDepthTransitionOutcome::ClosingAdvanced,
            _ => panic!("unknown oracle path {path}"),
        }
    }
}
