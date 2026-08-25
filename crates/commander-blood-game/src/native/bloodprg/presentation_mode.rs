//! Semantic bridge presentation mode selection.

const FIRST_BAND_FRAMES: std::ops::RangeInclusive<i16> = 23..=67;
const SECOND_BAND_FRAMES: std::ops::RangeInclusive<i16> = 68..=112;
const THIRD_BAND_FRAMES: std::ops::RangeInclusive<i16> = 113..=157;

/// Panorama band controlling bridge presentation placement.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationBridgeMode {
    /// Default placement outside the three central panorama bands.
    Outer,
    /// Panorama frames 23 through 67.
    FirstBand,
    /// Panorama frames 68 through 112.
    SecondBand,
    /// Panorama frames 113 through 157.
    ThirdBand,
}

/// Replace the active bridge presentation mode for the current panorama frame.
///
/// This translates `presentation_mode_bits_update` at BLOODPRG routine offset
/// `0x009510`. An optional enum replaces the mode nibble embedded in the native
/// shared UI word; `blocked` represents its independent bit-one gate.
pub fn update_presentation_bridge_mode(
    frame: i16,
    blocked: bool,
    mode: &mut Option<PresentationBridgeMode>,
) {
    *mode = if blocked {
        None
    } else if FIRST_BAND_FRAMES.contains(&frame) {
        Some(PresentationBridgeMode::FirstBand)
    } else if SECOND_BAND_FRAMES.contains(&frame) {
        Some(PresentationBridgeMode::SecondBand)
    } else if THIRD_BAND_FRAMES.contains(&frame) {
        Some(PresentationBridgeMode::ThirdBand)
    } else {
        Some(PresentationBridgeMode::Outer)
    };
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 15;

    #[derive(Deserialize)]
    struct PresentationModeOracle {
        name: String,
        signed_frame: i16,
        bit_one_gate_set: bool,
        selected_mode: u16,
    }

    #[test]
    fn semantic_modes_match_every_original_state_word_vector() {
        let vectors: Vec<PresentationModeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_9510_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut mode = Some(PresentationBridgeMode::ThirdBand);
            update_presentation_bridge_mode(
                vector.signed_frame,
                vector.bit_one_gate_set,
                &mut mode,
            );
            assert_eq!(mode, expected_mode(vector.selected_mode), "{}", vector.name);
        }
    }

    fn expected_mode(native_mode: u16) -> Option<PresentationBridgeMode> {
        match native_mode {
            0 => None,
            16 => Some(PresentationBridgeMode::Outer),
            32 => Some(PresentationBridgeMode::FirstBand),
            64 => Some(PresentationBridgeMode::SecondBand),
            128 => Some(PresentationBridgeMode::ThirdBand),
            _ => panic!("unknown recovered mode {native_mode}"),
        }
    }
}
