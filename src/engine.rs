//! Runnable engine main loop — the integration layer that ties the decoded
//! subsystems (VM/script, renderer, audio, ship-3D) into a single stepped game
//! loop faithful to `BLOODPRG.EXE`.
//!
//! The engine's top-level dispatch loop is at `0x0FFB` (REVERSE.md "MAIN GAME LOOP
//! HEAD"); each iteration polls the mouse via the shared dispatcher `0:0x70E`
//! ("MOUSE INPUT POLL"), resets the sprite dirty-rect list, calls the render/present
//! subsystems, gates on the on-ship flag `[0x2793] & 8`, advances a countdown, and
//! checks for a pending `D2` script/scene handoff at `0x108E`.
//!
//! This module reimplements that loop as a headless-steppable state machine so the
//! decoded components can be driven frame-by-frame (the interactive real-time driver
//! + graphics/input backend layers on top of this). It starts with the faithfully-
//! decoded input + frame bookkeeping; rendering and VM stepping wire in on top.

/// Live mouse input for one frame. Mirrors the engine globals written by the poll
/// at `0:0x70E`: `gs:[0xA2A]`=x, `gs:[0xA2C]`=y, `gs:[0xA2E]`=buttons.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseInput {
    pub x: u16,
    pub y: u16,
    /// Button bitmask as returned by `int 33h AX=3` in BX (bit0=left, bit1=right).
    pub buttons: u16,
}

impl MouseInput {
    pub fn left_down(&self) -> bool {
        self.buttons & 1 != 0
    }
    pub fn right_down(&self) -> bool {
        self.buttons & 2 != 0
    }
}

use crate::ship3d::{
    BloodPrng, Ship3dMatrixAngles, Ship3dProjectionOrigin, Ship3dProjectionViewport,
    render_ship_3d_starfield,
};
use crate::sprite::{SpriteFrameImage, blit_sprite_frame_centered, decode_sprite_bank_indices};

/// Screen dimensions of the engine framebuffer (VGA mode 13h / mode-X, 320x200).
pub const ENGINE_SCREEN_WIDTH: usize = 320;
pub const ENGINE_SCREEN_HEIGHT: usize = 200;

/// Per-frame engine state — the subset of the `DS`/`gs` globals the main loop
/// (`0x0FFB`) touches, plus the indexed framebuffer the render subsystems fill.
pub struct EngineState {
    /// Frame counter (increments once per [`EngineState::step`]).
    pub frame: u64,
    /// Current mouse input (poll result this frame).
    pub mouse: MouseInput,
    /// Previous mouse position, for movement detection (`gs:[0xA38]/[0xA3A]`).
    prev_pos: (u16, u16),
    /// Idle timer zeroed on mouse movement (`gs:[0xB3B]`).
    pub idle_ticks: u32,
    /// On-ship-nav render flag (`gs:[0x2793] & 8`) — selects on-ship HUD vs
    /// letterboxed-planet rendering, exactly as the main loop's mouse-path gate.
    pub on_ship: bool,
    /// Frame countdown at `[0x0A40]` advanced each iteration.
    pub countdown: u16,
    /// Ship-nav compass rotation angle (`[0x2795]`, 0..179), steered by the mouse.
    pub compass_angle: u16,
    /// Deterministic PRNG seed for the starfield point cloud (the engine seeds
    /// from CMOS RTC seconds at runtime; fixed here for reproducibility).
    pub starfield_seed: u8,
    /// Decoded ship-nav HUD sprite banks: BCARTE perspective grid frames.
    hud_grid: Vec<SpriteFrameImage>,
    /// Decoded ship-nav HUD orb sprite frames (BORXX).
    hud_orb: Vec<SpriteFrameImage>,
    /// Indexed (palette) framebuffer the render subsystems draw into.
    pub framebuffer: Vec<u8>,
}

impl Default for EngineState {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineState {
    pub fn new() -> Self {
        Self {
            frame: 0,
            mouse: MouseInput::default(),
            prev_pos: (0, 0),
            idle_ticks: 0,
            on_ship: false,
            countdown: 0,
            compass_angle: 0,
            starfield_seed: 17,
            hud_grid: Vec::new(),
            hud_orb: Vec::new(),
            framebuffer: vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT],
        }
    }

    /// Load the ship-nav HUD sprite banks (BCARTE grid frames + BORXX orb) from
    /// their raw `.spr` bytes so [`EngineState::render_ship_view`] composites the
    /// accurate sprite HUD over the starfield.
    pub fn load_hud_sprites(&mut self, bcarte_spr: &[u8], borxx_spr: &[u8]) {
        self.hud_grid = decode_sprite_bank_indices(bcarte_spr).unwrap_or_default();
        self.hud_orb = decode_sprite_bank_indices(borxx_spr).unwrap_or_default();
    }

    /// Render the on-ship nav view's starfield background at the current compass
    /// angle into the framebuffer (the background layer of the ship-3D view; the
    /// sprite HUD + scene band compose over it). Uses the recovered PRNG point
    /// cloud + projection. No-op if the angle is outside the trig table.
    pub fn render_ship_view(&mut self) {
        let mut prng = BloodPrng::seeded_from_rtc_seconds(self.starfield_seed);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: self.compass_angle % 180,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin {
            x: 0x8000,
            y: 0x8000,
            z: 0x8000,
        };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: ENGINE_SCREEN_WIDTH as u16,
            top: 0,
            bottom: ENGINE_SCREEN_HEIGHT as u16,
        };
        if let Some(render) = render_ship_3d_starfield(&mut prng, angles, origin, viewport) {
            self.framebuffer.copy_from_slice(&render.buffer);
        }
        // Composite the sprite HUD over the starfield: the BCARTE perspective grid
        // frame selected by the compass angle, then the BORXX orb, into the HUD band.
        let grid_idx = {
            let grid: Vec<usize> = self
                .hud_grid
                .iter()
                .enumerate()
                .filter(|(_, f)| f.height >= 64)
                .map(|(i, _)| i)
                .collect();
            (!grid.is_empty()).then(|| {
                grid[(self.compass_angle as usize * grid.len() / 180).min(grid.len() - 1)]
            })
        };
        if let Some(gi) = grid_idx {
            let frame = self.hud_grid[gi].clone();
            blit_sprite_frame_centered(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &frame,
                160,
                172,
            );
        }
        if let Some(orb) = self.hud_orb.first().cloned() {
            blit_sprite_frame_centered(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &orb,
                160,
                172,
            );
        }
    }

    /// The mouse input poll (`0:0x70E`): store the frame's cursor state and, if the
    /// cursor moved since last frame, reset the idle timer; otherwise advance it.
    fn poll_input(&mut self, input: MouseInput) {
        self.mouse = input;
        if (input.x, input.y) != self.prev_pos {
            self.prev_pos = (input.x, input.y);
            self.idle_ticks = 0;
        } else {
            self.idle_ticks = self.idle_ticks.saturating_add(1);
        }
    }

    /// One iteration of the top-level dispatch loop (`0x0FFB`). Ordered to match the
    /// decoded engine: poll input → (reset render state) → (render subsystems) →
    /// on-ship gate → countdown. Rendering and VM/script stepping wire in on top of
    /// this faithful control-flow skeleton; for now it advances input + bookkeeping
    /// so the loop is drivable and testable headlessly.
    pub fn step(&mut self, input: MouseInput) {
        self.poll_input(input);
        // On-ship gate ([0x2793] & 8): steer the compass from the mouse and render
        // the nav view's starfield background (the render subsystems the main loop
        // calls). Mouse x across the screen maps to the 0..179 compass rotation.
        if self.on_ship {
            self.compass_angle =
                ((self.mouse.x as u32 * 180) / ENGINE_SCREEN_WIDTH as u32).min(179) as u16;
            self.render_ship_view();
        }
        // Countdown at [0x0A40]: advanced each iteration, saturating at 0.
        self.countdown = self.countdown.saturating_sub(1);
        self.frame += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn step_advances_frame_and_polls_input() {
        let mut e = EngineState::new();
        assert_eq!(e.frame, 0);
        let m = MouseInput {
            x: 100,
            y: 50,
            buttons: 1,
        };
        e.step(m);
        assert_eq!(e.frame, 1);
        assert_eq!(e.mouse, m);
        assert!(e.mouse.left_down());
        assert_eq!(e.idle_ticks, 0, "movement resets idle timer");
    }

    #[test]
    fn idle_timer_counts_stationary_frames_and_resets_on_move() {
        let mut e = EngineState::new();
        let still = MouseInput {
            x: 10,
            y: 10,
            buttons: 0,
        };
        e.step(still); // first frame: moved from (0,0) -> reset
        e.step(still); // stationary -> +1
        e.step(still); // stationary -> +2
        assert_eq!(e.idle_ticks, 2);
        e.step(MouseInput {
            x: 11,
            y: 10,
            buttons: 0,
        });
        assert_eq!(e.idle_ticks, 0, "movement zeroes the idle timer");
    }

    #[test]
    fn on_ship_step_renders_starfield_steered_by_mouse() {
        let mut e = EngineState::new();
        e.on_ship = true;
        // Step with the mouse at the left, then far right: the compass angle should
        // track the mouse and the rendered starfield should differ.
        e.step(MouseInput { x: 0, y: 100, buttons: 0 });
        let angle_left = e.compass_angle;
        let frame_left = e.framebuffer.clone();
        e.step(MouseInput { x: 319, y: 100, buttons: 0 });
        assert_eq!(angle_left, 0);
        assert!(e.compass_angle > 150, "mouse right steers the compass high");
        assert!(
            frame_left.iter().any(|&p| p != 0),
            "the starfield renders some points"
        );
        assert_ne!(frame_left, e.framebuffer, "different angle -> different view");
    }

    #[test]
    fn on_ship_render_composites_sprite_hud_when_loaded() {
        let read = |names: &[&str]| -> Option<Vec<u8>> {
            names.iter().find_map(|p| std::fs::read(p).ok())
        };
        let (Some(bc), Some(bo)) = (
            read(&["output/_tmp_iso/BCARTE.SPR", "../output/_tmp_iso/BCARTE.SPR"]),
            read(&["output/_tmp_iso/BORXX.SPR", "../output/_tmp_iso/BORXX.SPR"]),
        ) else {
            eprintln!("skipping: HUD sprites not available");
            return;
        };
        let mut e = EngineState::new();
        e.on_ship = true;
        e.load_hud_sprites(&bc, &bo);
        assert!(!e.hud_grid.is_empty() && !e.hud_orb.is_empty());
        // Render without HUD (empty) vs with HUD -> the HUD band gains sprite pixels.
        e.step(MouseInput { x: 90, y: 100, buttons: 0 });
        // Count non-zero pixels in the HUD band (rows 150..195, where the HUD sits).
        let band: usize = (150..195)
            .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] != 0)
            .count();
        assert!(band > 200, "sprite HUD composites into the band (got {band})");
    }

    #[test]
    fn framebuffer_is_full_screen_indexed() {
        let e = EngineState::new();
        assert_eq!(
            e.framebuffer.len(),
            ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT
        );
    }
}
