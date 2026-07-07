//! `manu3.xdb` overlay logic — the ship's 3D pyramid-menu interface.
//!
//! Distinct from the alien overlays (no shared PRNG); its entry (`0x0000`) takes the
//! caller's input params (mouse coords: `[bp+6]>>4; +0xA0` → screen row, `[bp+4]&0x1F`
//! → column) and dispatches through the menu handlers. Ported here: the input-coord
//! decode + item-selection dispatch (`0x181`, `base + table[item]`), the tween setup
//! that links selection to animation (`0x1DF`, `delta = (end-current)<<16 / count`),
//! the menu animation/tween list (`0x19B`) — the full menu animation pipeline
//! (select → build tweens → advance) — and the 3D-menu camera pan (`0x34..0x51`,
//! centre-delta steering). Remaining: the per-item action handlers the dispatch jumps
//! to, and the pyramid vertex blit; the draw's angle/matrix setup (`0x270`) is ported and the projection reuses the shared ship-3D compositor.

/// The menu-item column index from the caller's input word (`[bp+4] & 0x1F`, method
/// entry `0x000`/`0x181`) — 0..31 selects one of up to 32 menu items.
pub fn menu_item_index(input: u16) -> usize {
    (input & 0x1F) as usize
}

/// The screen row derived from the caller's input word (`[bp+6] >> 4`, then the high
/// byte offset by `0xA0` = row+160 in the entry at `0x000`).
pub fn menu_screen_row(input: u16) -> u16 {
    let shifted = input >> 4;
    // add ah, 0xA0  →  add 0xA0 to the high byte only.
    shifted.wrapping_add(0xA000u16 as u16)
}

/// Resolve a selected menu item to its handler offset (method `0x181`): the item index
/// (word-scaled) reads an entry from the offset table located at `base`, and the
/// handler is `base + table[item]` (`di = [0x2306]; di += [item*2 + di]`). `table` is
/// the overlay's word table read at `base`. Out-of-range items resolve to `base`.
pub fn menu_item_handler(base: u16, table: &[u16], item: usize) -> u16 {
    base.wrapping_add(table.get(item).copied().unwrap_or(0))
}

/// The menu-view centre the camera pans around (`0xA0`/`0x64` = screen 160,100).
pub const MENU_CAMERA_CENTRE: (i16, i16) = (0xA0, 0x64);

/// The rotation angle-index mask the pyramid draw applies (`0xFFC` = a 10-bit angle
/// scaled ×4 into the shared trig table).
pub const MENU_ANGLE_MASK: u16 = 0x0FFC;

/// The pyramid draw's per-axis rotation angle indices (method `0x270` setup): the three
/// object angle fields (`+0x4E`/`+0x50`/`+0x52`), each masked to `0xFFC`, form the
/// trig-table offsets that build the rotation matrix — after which the menu reuses the
/// **shared ship-3D projection** (`build_ship_3d_projection_matrix` + `project_ship_3d_point`)
/// to draw the pyramid. Objects use the same `0x5E`-byte stride as the alien engine.
pub fn menu_pyramid_angles(angle_x: u16, angle_y: u16, angle_z: u16) -> [u16; 3] {
    [
        angle_x & MENU_ANGLE_MASK,
        angle_y & MENU_ANGLE_MASK,
        angle_z & MENU_ANGLE_MASK,
    ]
}

/// The 3D-menu camera pan from the cursor position (entry `0x34..0x51`): the cursor's
/// delta from the view centre, doubled, is added to the view offset `[0x23E4]` (x from
/// `[0x1A]`) / `[0x23E2]` (y from `[0x1C]`) each frame before the pyramid draw (`0x270`)
/// — the same centre-delta steering as the ship-3D / alien views. Returns the
/// `(dx, dy)` added to the view offset.
pub fn menu_camera_pan(cursor_x: i16, cursor_y: i16) -> (i16, i16) {
    let dx = cursor_x.wrapping_sub(MENU_CAMERA_CENTRE.0).wrapping_mul(2);
    let dy = cursor_y.wrapping_sub(MENU_CAMERA_CENTRE.1).wrapping_mul(2);
    (dx, dy)
}

/// One entry in the menu's active-animation list (method `0x19B`): a fixed-point tween
/// that each frame writes its accumulator's high word to a target field, then advances
/// the accumulator by a delta, decrementing a frame counter until it expires.
///
/// Record layout at `di`: `+0x00` frame counter, `+0x06` 32-bit accumulator, `+0x08`
/// its high word (the value written to the target `[di+4]`), `+0x0A` 32-bit per-frame
/// delta.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuTween {
    /// `+0x00` frames remaining; the tween is removed once this goes negative.
    pub counter: i16,
    /// `+0x06` 32-bit fixed-point accumulator; its high word is the output value.
    pub accumulator: i32,
    /// `+0x0A` 32-bit per-frame increment added to the accumulator.
    pub delta: i32,
}

impl MenuTween {
    pub fn new(counter: i16, start: i32, delta: i32) -> Self {
        Self {
            counter,
            accumulator: start,
            delta,
        }
    }

    /// Build a tween from an animation descriptor (method `0x1DF`, the setup that links
    /// item-selection to the tween list): animate a field from its `current` value to
    /// the descriptor's `end` value over `count` frames. The accumulator starts at
    /// `current << 16` and the per-frame delta is `((end - current) << 16) / count`
    /// (16.16 fixed point: `shl eax,0x10; cdq; idiv ecx`), so the output high word
    /// walks `current → end`.
    pub fn to_target(current: i16, end: i16, count: i16) -> Self {
        let n = (count as i32).max(1);
        let delta = ((end as i32 - current as i32) << 16) / n;
        Self::new(count, (current as i32) << 16, delta)
    }

    /// The output value written to the target this frame — the accumulator's high word
    /// (`[di+8]`, i.e. `accumulator >> 16`).
    pub fn output(&self) -> u16 {
        (self.accumulator >> 16) as u16
    }

    /// Advance one frame exactly as `0x19B` does per entry: the caller first takes
    /// [`output`](Self::output) and writes it to the target, then this decrements the
    /// counter — returning `false` (remove me) when it goes negative — and otherwise
    /// advances the accumulator by the delta.
    pub fn step(&mut self) -> bool {
        self.counter -= 1;
        if self.counter < 0 {
            return false;
        }
        self.accumulator = self.accumulator.wrapping_add(self.delta);
        true
    }
}

/// The menu's active-animation list (`0x19B`): processes every tween each frame,
/// writing each output to its target via a caller-supplied sink, and swap-removes the
/// tweens that have expired (mirroring the binary's `sub bx,2; xchg [bx],di` compaction).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct MenuTweenList {
    /// `(target-id, tween)` pairs; the `target-id` stands in for the `[di+4]` write
    /// address so a caller can route the value to the right menu field.
    pub tweens: Vec<(u16, MenuTween)>,
}

impl MenuTweenList {
    /// Process all tweens for one frame: for each, emit `(target_id, output_value)` via
    /// `sink`, then advance it; expired tweens are removed. Returns the number still
    /// active.
    pub fn step(&mut self, mut sink: impl FnMut(u16, u16)) -> usize {
        let mut i = 0;
        while i < self.tweens.len() {
            let (target, tween) = &mut self.tweens[i];
            sink(*target, tween.output());
            if tween.step() {
                i += 1;
            } else {
                // swap-remove the finished tween (compaction, as the binary does).
                self.tweens.swap_remove(i);
            }
        }
        self.tweens.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pyramid_angles_mask_to_trig_offsets() {
        // Each angle field masked to 0xFFC (clears low 2 bits + bits above 0x0FFC).
        assert_eq!(menu_pyramid_angles(0x1234, 0x0FFF, 0x0003), [0x0234, 0x0FFC, 0x0000]);
        // Already-aligned angles pass through.
        assert_eq!(menu_pyramid_angles(0x0400, 0x0800, 0x0FFC), [0x0400, 0x0800, 0x0FFC]);
    }

    #[test]
    fn camera_pans_by_doubled_centre_delta() {
        // Cursor at the centre -> no pan.
        assert_eq!(menu_camera_pan(0xA0, 0x64), (0, 0));
        // Right/down of centre -> positive doubled deltas.
        assert_eq!(menu_camera_pan(0xA0 + 10, 0x64 + 5), (20, 10));
        // Left/up -> negative.
        assert_eq!(menu_camera_pan(0xA0 - 8, 0x64 - 4), (-16, -8));
    }

    #[test]
    fn item_index_and_handler_dispatch() {
        // Item index = input & 0x1F.
        assert_eq!(menu_item_index(0x0000), 0);
        assert_eq!(menu_item_index(0x0007), 7);
        assert_eq!(menu_item_index(0x1F3F), 0x1F); // high bits ignored
        // Handler = base + table[item].
        let table = [0x0010u16, 0x0040, 0x0080];
        assert_eq!(menu_item_handler(0x2000, &table, 0), 0x2010);
        assert_eq!(menu_item_handler(0x2000, &table, 2), 0x2080);
        // Out-of-range item resolves to base (offset 0).
        assert_eq!(menu_item_handler(0x2000, &table, 9), 0x2000);
    }

    #[test]
    fn tween_to_target_walks_current_to_end() {
        // Animate 10 -> 50 over 8 frames: output starts at 10, and after stepping the
        // full count the high word reaches (about) 50.
        let mut t = MenuTween::to_target(10, 50, 8);
        assert_eq!(t.output(), 10, "starts at current");
        for _ in 0..8 {
            t.step();
        }
        assert_eq!(t.output(), 50, "reaches end after count frames");
        // Descending target too.
        let mut d = MenuTween::to_target(100, 20, 4);
        assert_eq!(d.output(), 100);
        for _ in 0..4 {
            d.step();
        }
        assert_eq!(d.output(), 20);
    }

    #[test]
    fn tween_outputs_high_word_and_advances_by_delta() {
        // Accumulator 0x0002_8000, delta 0x0000_8000: output = high word = 2, then the
        // accumulator advances so the next high word is 3.
        let mut t = MenuTween::new(4, 0x0002_8000, 0x0000_8000);
        assert_eq!(t.output(), 2);
        assert!(t.step());
        assert_eq!(t.output(), 3);
    }

    #[test]
    fn tween_removed_when_counter_expires() {
        let mut t = MenuTween::new(0, 0, 0x10000);
        // counter 0 -> step decrements to -1 -> remove.
        assert!(!t.step());
    }

    #[test]
    fn list_writes_targets_and_drops_expired() {
        let mut list = MenuTweenList {
            tweens: vec![
                (0xAA, MenuTween::new(1, 0x0005_0000, 0x0001_0000)),
                (0xBB, MenuTween::new(0, 0x0009_0000, 0)), // expires this frame
            ],
        };
        let mut writes = Vec::new();
        let active = list.step(|target, value| writes.push((target, value)));
        // Both wrote their current high word this frame (5 and 9)...
        assert_eq!(writes, vec![(0xAA, 5), (0xBB, 9)]);
        // ...but the second expired, leaving one active.
        assert_eq!(active, 1);
        assert_eq!(list.tweens.len(), 1);
        assert_eq!(list.tweens[0].0, 0xAA);
    }
}
