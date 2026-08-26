//! Typed ship-HUD palette capture and camera reset.

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

/// First palette entry reserved for the 3D ship HUD.
pub const SHIP_HUD_PALETTE_FIRST: usize = 128;
/// Number of palette entries captured for the 3D ship HUD.
pub const SHIP_HUD_PALETTE_COLOR_COUNT: usize = 64;

/// Camera origin installed before the bridge's procedural 3D projection pass.
pub const SHIP_CAMERA_RESET: [i16; 3] = [10_000, 12_000, 0];

/// Complete indexed palette used by original artwork and bridge rendering.
pub type IndexedGamePalette = [[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
/// Captured palette window reserved for ship HUD rendering.
pub type ShipHudPaletteSnapshot = [[u8; RGB_COMPONENT_COUNT]; SHIP_HUD_PALETTE_COLOR_COUNT];

/// Mutable ship HUD state captured before entering 3D camera work.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShipHudState {
    /// Palette colors 128 through 191 after HUD drawing has completed.
    pub palette_snapshot: ShipHudPaletteSnapshot,
    /// Flat signed camera coordinates.
    pub camera: [i16; 3],
}

impl Default for ShipHudState {
    fn default() -> Self {
        Self {
            palette_snapshot: [[u8::MIN; RGB_COMPONENT_COUNT]; SHIP_HUD_PALETTE_COLOR_COUNT],
            camera: [i16::MIN; 3],
        }
    }
}

impl ShipHudState {
    /// Capture the ship-HUD palette window and restore the authored camera origin.
    ///
    /// The caller performs the translated HUD artwork selection first. Keeping
    /// the capture as a state operation lets modern hosts invoke the recovered
    /// drawing logic directly instead of supplying an artificial callback.
    pub fn capture_palette_and_reset_camera(&mut self, live_palette: &IndexedGamePalette) {
        self.palette_snapshot.copy_from_slice(
            &live_palette
                [SHIP_HUD_PALETTE_FIRST..SHIP_HUD_PALETTE_FIRST + SHIP_HUD_PALETTE_COLOR_COUNT],
        );
        self.camera = SHIP_CAMERA_RESET;
    }
}

/// Host HUD drawing performed immediately before the palette capture.
pub trait ShipHudBackend {
    /// Draw the HUD and apply any resulting changes to the live palette.
    fn draw_ship_hud(&mut self, live_palette: &mut IndexedGamePalette);
}

/// Draw the ship HUD, capture its palette window, and reset the camera.
///
/// This translates `ship_3d_hud_palette_snapshot_and_camera_reset` at
/// BLOODPRG routine offset `0x008C96`. Typed palette arrays and camera
/// coordinates replace far copies, segment direction state, and fixed globals.
pub fn snapshot_ship_hud_palette_and_reset_camera<Backend: ShipHudBackend>(
    live_palette: &mut IndexedGamePalette,
    state: &mut ShipHudState,
    backend: &mut Backend,
) {
    backend.draw_ship_hud(live_palette);
    state.capture_palette_and_reset_camera(live_palette);
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 4;
    const SNAPSHOT_BYTE_COUNT: usize = SHIP_HUD_PALETTE_COLOR_COUNT * RGB_COMPONENT_COUNT;

    #[derive(Deserialize)]
    struct HudOracle {
        name: String,
        copied_bytes: usize,
        source_mutations: Vec<PaletteMutation>,
        camera: CameraOracle,
    }

    #[derive(Deserialize)]
    struct PaletteMutation {
        relative_offset: usize,
        value: u8,
    }

    #[derive(Deserialize)]
    struct CameraOracle {
        x: i16,
        y: i16,
        z: i16,
    }

    struct OracleBackend {
        mutations: Vec<PaletteMutation>,
        called: bool,
    }

    impl ShipHudBackend for OracleBackend {
        fn draw_ship_hud(&mut self, live_palette: &mut IndexedGamePalette) {
            self.called = true;
            for mutation in &self.mutations {
                let absolute =
                    SHIP_HUD_PALETTE_FIRST * RGB_COMPONENT_COUNT + mutation.relative_offset;
                live_palette[absolute / RGB_COMPONENT_COUNT][absolute % RGB_COMPONENT_COUNT] =
                    mutation.value;
            }
        }
    }

    #[test]
    fn capture_matches_every_original_palette_and_camera_vector() {
        let vectors: Vec<HudOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8c96_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut palette = indexed_palette();
            let mut expected = palette;
            for mutation in &vector.source_mutations {
                let absolute =
                    SHIP_HUD_PALETTE_FIRST * RGB_COMPONENT_COUNT + mutation.relative_offset;
                expected[absolute / RGB_COMPONENT_COUNT][absolute % RGB_COMPONENT_COUNT] =
                    mutation.value;
            }
            let mut backend = OracleBackend {
                mutations: vector.source_mutations,
                called: false,
            };
            let mut state = ShipHudState::default();
            snapshot_ship_hud_palette_and_reset_camera(&mut palette, &mut state, &mut backend);

            assert!(backend.called, "{}", vector.name);
            assert_eq!(vector.copied_bytes, SNAPSHOT_BYTE_COUNT, "{}", vector.name);
            assert_eq!(
                state.palette_snapshot.as_slice(),
                &expected
                    [SHIP_HUD_PALETTE_FIRST..SHIP_HUD_PALETTE_FIRST + SHIP_HUD_PALETTE_COLOR_COUNT],
                "{}",
                vector.name
            );
            assert_eq!(
                state.camera,
                [vector.camera.x, vector.camera.y, vector.camera.z],
                "{}",
                vector.name
            );
        }
    }

    fn indexed_palette() -> IndexedGamePalette {
        let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
        for (index, color) in palette.iter_mut().enumerate() {
            *color = [
                index as u8,
                index.wrapping_mul(3) as u8,
                index.wrapping_mul(5) as u8,
            ];
        }
        palette
    }
}
