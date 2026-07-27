//! `manu3.xdb` overlay logic — the ship's 3D pyramid-menu interface.
//!
//! Distinct from the alien overlays (no shared PRNG); its entry (`0x0000`) takes the
//! caller's input params (mouse coords: `[bp+6]>>4; +0xA0` → screen row, `[bp+4]&0x1F`
//! → column) and dispatches through the menu handlers. Ported here: the input-coord
//! decode + item-selection dispatch (`0x181`, `base + table[item]`), the tween setup
//! that links selection to animation (`0x1DF`, `delta = (end-current)<<16 / count`),
//! the menu animation/tween list (`0x19B`) — the full menu animation pipeline
//! (select → build tweens → advance) — the 3D-menu camera pan (`0x34..0x51`,
//! centre-delta steering), the pyramid angle/matrix setup (`0x270`, projection via the
//! shared ship-3D compositor), and the per-item animation descriptors (the DATA the
//! dispatch selects — phase|count/target/end — not code routines). So manu3's logic is
//! ported end-to-end; the remaining pieces are overlay DATA (the descriptor tables) and
//! the final pyramid vertex blit.

/// The menu-item column index from the caller's input word (`[bp+4] & 0x1F`, method
/// entry `XDB:manu3:0x000`/`XDB:manu3:0x181`) — 0..31 selects one of up to 32 menu items.
pub fn menu_item_index(input: u16) -> usize {
    (input & 0x1F) as usize
}

/// The amount added to a menu row (placed in the high byte of the derived coordinate word).
const MENU_ROW_BIAS: u16 = 160;

/// The screen row derived from the caller's input word: shift right by 4, then bias the row
/// (the high byte) by [`MENU_ROW_BIAS`].
pub fn menu_screen_row(input: u16) -> u16 {
    let shifted = input >> 4;
    shifted.wrapping_add(MENU_ROW_BIAS << 8)
}

/// Resolve a selected menu item to its handler offset (method `XDB:manu3:0x181`): the item index
/// (word-scaled) reads an entry from the offset table located at `base`, and the
/// handler is `base + table[item]` (`di = [0x2306]; di += [item*2 + di]`). `table` is
/// the overlay's word table read at `base`. Out-of-range items resolve to `base`.
pub fn menu_item_handler(base: u16, table: &[u16], item: usize) -> u16 {
    base.wrapping_add(table.get(item).copied().unwrap_or(0))
}

/// The menu-view centre the camera pans around (screen 160,100).
pub const MENU_CAMERA_CENTRE: (i16, i16) = (160, 100);

/// The rotation angle-index mask, `mov bx,0xffc` at manu3.xdb `0x283`.
///
/// The mask is loaded once and applied to THREE per-node angle fields:
///
/// ```text
///   0x0283  mov bx,0xffc
///   0x0280  mov ax,[di+0x52]   / 0x0289  and ax,bx
///   0x0286  mov si,[di+0x4e]   / 0x028E  and si,bx
///   0x028B  mov di,[di+0x50]   / 0x0290  and di,bx
/// ```
///
/// `0xFFC` keeps 10 bits with the low two clear, i.e. a multiple of 4 — an angle
/// index scaled ×4 into the shared trig table. The doc used to give the value and
/// no instruction, which is a value restated rather than a provenance; found with
/// `re/tools/find_imm.py 0xFFC output/_tmp_dat/manu3.xdb`.
pub const MENU_ANGLE_MASK: u16 = 0x0FFC;

/// The pyramid draw's per-axis rotation angle indices (method `XDB:manu3:0x270` setup): the three
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
/// `[0x1A]`) / `[0x23E2]` (y from `[0x1C]`) each frame before the pyramid draw (`XDB:manu3:0x270`)
/// — the same centre-delta steering as the ship-3D / alien views. Returns the
/// `(dx, dy)` added to the view offset.
// Verified at `XDB:manu3:0x0034..0x0058` (audit-fixes #472) — the whole law,
// including its non-destructive push/pop:
//
//   0x0034  push word ptr [0x23e2]   save pitch
//   0x0038  push word ptr [0x23e4]   save yaw
//   0x003C  mov ax, word ptr [0x1a]  cursor X
//   0x0043  sub ax, 0xa0             X - 160
//   0x0046  add ax, ax               ...doubled
//   0x0048  add word ptr [0x23e4], ax
//   0x004C  sub bx, 0x64             Y - 100
//   0x004F  add bx, bx               ...doubled
//   0x0051  add word ptr [0x23e2], bx
//   0x0055  call 0x270               compose
//   0x0058  pop word ptr [0x23e4]    restore
//
// So MENU_CAMERA_CENTRE (160, 100) is `0xA0`/`0x64` as immediates, the doubling
// is `add reg,reg` rather than a shift, and the angles are ADDED to the stored
// values then restored — the hand aims by DISPLACEMENT and does not accumulate.
pub fn menu_camera_pan(cursor_x: i16, cursor_y: i16) -> (i16, i16) {
    let dx = cursor_x.wrapping_sub(MENU_CAMERA_CENTRE.0).wrapping_mul(2);
    let dy = cursor_y.wrapping_sub(MENU_CAMERA_CENTRE.1).wrapping_mul(2);
    (dx, dy)
}

/// A menu item's animation descriptor — the DATA the item-dispatch (`XDB:manu3:0x181`) selects
/// and the tween setup (`XDB:manu3:0x1DF`) consumes. NOT a code routine; the field layout is
/// read straight off `XDB:manu3:0x01DF..0x01FE`:
///
/// ```text
///   0x01DF  mov si, word ptr [0x102e]     the descriptor pointer
///   0x01E3  movzx ecx, word ptr [si]      the packed word...
///   0x01E7  or cl, cl / je                ...low byte = frame COUNT, 0 ends the list
///   0x01EB  cmp ch, byte ptr [0x102c]     ...high byte = PHASE, gated on 0x102C
///   0x01EF  jne                           a phase mismatch skips the item
///   0x01F8  mov bp, word ptr [si + 4]     the TARGET field address
///   0x01FE  mov ax, word ptr [si + 6]     the END value
/// ```
///
/// It then builds a [`MenuTween`] animating the target's current value to `end`
/// over `count` frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MenuAnimDescriptor {
    /// High byte of `[si]` (`ch`) — the phase group, gated against `[0x102C]`.
    pub phase: u8,
    /// Low byte of `[si]` (`cl`) — the tween frame count.
    pub count: u8,
    /// `[si+4]` — the target menu field the tween writes.
    pub target: u16,
    /// `[si+6]` — the end value the field animates to.
    pub end: i16,
}

impl MenuAnimDescriptor {
    /// Parse the descriptor at `off` in the overlay data (`[off]`=phase|count,
    /// `[off+4]`=target, `[off+6]`=end).
    pub fn parse(data: &[u8], off: usize) -> Option<Self> {
        let w = |o: usize| -> Option<u16> {
            Some(u16::from_le_bytes([*data.get(o)?, *data.get(o + 1)?]))
        };
        let packed = w(off)?;
        Some(Self {
            phase: (packed >> 8) as u8,
            count: (packed & 0xFF) as u8,
            target: w(off + 4)?,
            end: w(off + 6)? as i16,
        })
    }

    /// Build the tween this descriptor drives (via `XDB:manu3:0x1DF`): animate the target field
    /// from its `current` value to `end` over `count` frames.
    pub fn tween(&self, current: i16) -> MenuTween {
        MenuTween::to_target(current, self.end, self.count as i16)
    }
}

/// One entry in the menu's active-animation list (method `XDB:manu3:0x19B`): a fixed-point tween
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

    /// Build a tween from an animation descriptor (method `XDB:manu3:0x1DF`, the setup that links
    /// item-selection to the tween list): animate a field from its `current` value to
    /// the descriptor's `end` value over `count` frames. The accumulator starts at
    /// `current << 16` and the per-frame delta is `((end - current) << 16) / count`
    /// (16.16 fixed point: `shl eax,0x10; cdq; idiv ecx`), so the output high word
    /// walks `current → end`.
    pub fn to_target(current: i16, end: i16, count: i16) -> Self {
        let n = (count as i32).max(1);
        let delta = ((end as i32 - current as i32) << 16) / n;
        // THE SETUP PRE-ADVANCES BY ONE FRAME, and both halves of that are easy to
        // miss (audit-fixes #486). `dec cx` @0x214 stores `count - 1`, and
        // `add ebp,eax` @0x219 stores `(current << 16) + delta` — so the FIRST
        // value the step loop writes is already `current + delta`, and the tween
        // lands on `end` after exactly `count` writes. Storing `count` and a bare
        // `current << 16` (as this did) writes the unmoved start value for one
        // extra frame, making every menu animation one frame long and one step
        // behind. `count == 0` never reaches here in the binary — `or cl,cl / je`
        // @0x1E7 skips the descriptor entirely — so the saturating floor is the
        // port's own guard, not a decoded behaviour.
        Self::new(
            count.saturating_sub(1),
            ((current as i32) << 16).wrapping_add(delta),
            delta,
        )
    }

    /// The output value written to the target this frame — the accumulator's high word
    /// (`[di+8]`, i.e. `accumulator >> 16`).
    pub fn output(&self) -> u16 {
        (self.accumulator >> 16) as u16
    }

    /// Advance one frame exactly as `XDB:manu3:0x19B` does per entry: the caller first takes
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

/// The menu's active-animation list (`XDB:manu3:0x19B`): processes every tween each frame,
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
    fn item0_descriptor_decodes_and_builds_tween() {
        // Item 0's descriptor in the real overlay: phase 1, count 124, target 0x11B0,
        // end 300 (0x012C). Skips if the asset isn't present.
        let path = ["output/_tmp_dat/manu3.xdb", "../output/_tmp_dat/manu3.xdb"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        let d = MenuAnimDescriptor::parse(&data, 0x6254).unwrap();
        assert_eq!(d.phase, 1);
        assert_eq!(d.count, 124);
        assert_eq!(d.target, 0x11B0);
        assert_eq!(d.end, 300);
        // The descriptor builds a tween that walks a field to (near) its end over the
        // count; fixed-point truncation ((300<<16)/124) lands one short, as the game's.
        let mut t = d.tween(0);
        for _ in 0..d.count {
            t.step();
        }
        assert!((299..=300).contains(&t.output()), "reaches ~300, got {}", t.output());
    }

    #[test]
    fn item_dispatch_matches_real_manu3_table() {
        // Verify the ported dispatch against the real overlay: base = [0x2306], and
        // handler = base + table[item] must land inside the overlay's code. Skips if
        // the asset isn't present.
        let path = ["output/_tmp_dat/manu3.xdb", "../output/_tmp_dat/manu3.xdb"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(path) = path else { return };
        let data = std::fs::read(path).unwrap();
        let base = u16::from_le_bytes([data[0x2306], data[0x2307]]);
        assert_eq!(base, 0x3E72, "the [0x2306] table base");
        let table: Vec<u16> = (0..12)
            .map(|i| {
                let o = base as usize + i * 2;
                u16::from_le_bytes([data[o], data[o + 1]])
            })
            .collect();
        // Item 0's handler resolves to a real code offset inside the file.
        let h0 = menu_item_handler(base, &table, 0);
        assert_eq!(h0, 0x6254);
        assert!((h0 as usize) < data.len(), "handler is within the overlay");
        // The dispatch equals base + table[item] for every item.
        for (i, &entry) in table.iter().enumerate() {
            assert_eq!(menu_item_handler(base, &table, i), base.wrapping_add(entry));
        }
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
        // Animate 10 -> 50 over 8 frames. The setup PRE-ADVANCES one delta
        // (`add ebp,eax` @XDB:manu3:0x219) and PRE-DECREMENTS the counter
        // (`dec cx` @0x214), so the first value written is current + delta = 15,
        // NOT the unmoved 10 (audit-fixes #486). Eight writes then land on 50.
        let mut t = MenuTween::to_target(10, 50, 8);
        assert_eq!(t.output(), 15, "first write is current + delta, pre-advanced");
        for _ in 0..8 {
            t.step();
        }
        assert_eq!(t.output(), 50, "reaches end after count frames");
        // The eighth step is the one that expires it: seven advance, then the
        // counter goes negative and `step` reports remove-me without advancing.
        let mut t = MenuTween::to_target(10, 50, 8);
        for i in 0..7 {
            assert!(t.step(), "step {i} still active");
        }
        assert_eq!(t.output(), 50);
        assert!(!t.step(), "expires on the count-th step");
        assert_eq!(t.output(), 50, "an expired tween does not advance past end");
        // Descending target too: 100 -> 20 over 4, first write 100 - 20 = 80.
        let mut d = MenuTween::to_target(100, 20, 4);
        assert_eq!(d.output(), 80);
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
