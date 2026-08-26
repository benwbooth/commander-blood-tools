//! Main-loop pause label refresh in logical screen coordinates.

const MODE_X_BYTES_PER_ROW: u16 = 80;
const MODE_X_PIXELS_PER_ADDRESS: u16 = 4;
const CLEAR_START_ADDRESS: u16 = 7_470;
const CLEAR_ADDRESS_WIDTH: u16 = 20;
const CLEAR_ROW_COUNT: u16 = 14;
const PAUSE_TEXT: &[u8] = b"PAUSE";

/// Palette index used by the pause label's ten-row UI font.
pub const PAUSE_HUD_PALETTE_INDEX: u8 = 232;

/// Logical rectangle cleared behind the pause label.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PauseHudRectangle {
    /// Horizontal origin.
    pub x: u16,
    /// Vertical origin.
    pub y: u16,
    /// Width in logical pixels.
    pub width: u16,
    /// Height in logical pixels.
    pub height: u16,
}

/// One renderer-independent refresh of the main-loop pause indicator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PauseHudRefresh {
    /// Logical region cleared before rendering text.
    pub clear_region: PauseHudRectangle,
    /// Original game-font bytes.
    pub text: &'static [u8],
    /// Logical text origin.
    pub text_position: [u16; 2],
    /// Palette index used for the text.
    pub text_palette_index: u8,
}

/// Build the main-loop pause refresh when its low-bit gate is enabled.
///
/// This translates `main_loop_hud_refresh` at BLOODPRG routine offset
/// `0x001A93`. The original clear covers 20 Mode-X addresses on each of 14
/// rows with all four planes selected, which is an 80 by 14 logical-pixel
/// rectangle. The authored text, placement, color, and low-bit gate remain;
/// wgpu frame submission replaces VGA map-mask writes and retrace polling.
pub const fn build_pause_hud_refresh(refresh_gate: u8) -> Option<PauseHudRefresh> {
    if refresh_gate & 1 == u8::MIN {
        return None;
    }

    Some(PauseHudRefresh {
        clear_region: PauseHudRectangle {
            x: (CLEAR_START_ADDRESS % MODE_X_BYTES_PER_ROW) * MODE_X_PIXELS_PER_ADDRESS,
            y: CLEAR_START_ADDRESS / MODE_X_BYTES_PER_ROW,
            width: CLEAR_ADDRESS_WIDTH * MODE_X_PIXELS_PER_ADDRESS,
            height: CLEAR_ROW_COUNT,
        },
        text: PAUSE_TEXT,
        text_position: [135, 96],
        text_palette_index: PAUSE_HUD_PALETTE_INDEX,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 7;

    #[derive(Deserialize)]
    struct HudOracle {
        name: String,
        gate: u8,
        enabled: bool,
        calls: Vec<serde_json::Value>,
    }

    #[test]
    fn refresh_gate_matches_every_original_vector() {
        let vectors: Vec<HudOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1a93_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let refresh = build_pause_hud_refresh(vector.gate);
            assert_eq!(refresh.is_some(), vector.enabled, "{}", vector.name);
            assert_eq!(
                refresh.is_some(),
                !vector.calls.is_empty(),
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn refresh_converts_the_mode_x_clear_to_exact_logical_geometry() {
        let refresh = build_pause_hud_refresh(1).unwrap();
        assert_eq!(
            refresh.clear_region,
            PauseHudRectangle {
                x: 120,
                y: 93,
                width: 80,
                height: 14,
            }
        );
        assert_eq!(refresh.text, b"PAUSE");
        assert_eq!(refresh.text_position, [135, 96]);
        assert_eq!(refresh.text_palette_index, 232);
    }
}
