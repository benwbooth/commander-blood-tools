//! `manu3.xdb` overlay logic — the ship's 3D pyramid-menu interface.
//!
//! Distinct from the alien overlays (no shared PRNG); its entry (`0x0000`) takes the
//! caller's input params (mouse coords: `[bp+6]>>4; +0xA0` → screen row, `[bp+4]&0x1F`
//! → column) and dispatches through the menu handlers. Ported here: the input-coord
//! decode + item-selection dispatch (`0x181`, `base + table[item]`) and the menu
//! animation/tween list (`0x19B`). Remaining: the per-item action handlers the
//! dispatch jumps to, and the 3D pyramid draw.

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
