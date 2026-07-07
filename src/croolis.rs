//! `croolis.xdb` overlay logic — the alien-species interaction/behaviour subsystem.
//!
//! Decoded (see `re/REVERSE.md`, sess 003): the overlay drives a list of 0x5E-byte
//! object records, each a PRNG + timer *animation state machine*, dispatched per frame
//! and feeding the shared ship-3D per-object draw. This module ports the decoded pieces
//! (the animation-state PRNG and the per-object state machine at method `0x16A4`); the
//! full per-object draw/vtable dispatch is future work.

/// The overlay's animation-state PRNG (`0x16A4`: `mov ax,fs:[0x105C]; ror ax,7;
/// sbb ax,0; store back`). On 8086 `ror ax,7` leaves CF = the result's MSB (the last
/// bit rotated through carry), and `sbb ax,0` subtracts that carry — so the next state
/// is `rotate_right(seed,7) - msb`. Distinct from the ship-view `rcr/rcl` PRNG.
pub fn alien_anim_prng_next(seed: u16) -> u16 {
    let rotated = seed.rotate_right(7);
    let carry = rotated >> 15; // CF after `ror …,7` = MSB of the rotated value
    rotated.wrapping_sub(carry)
}

/// A `croolis` object's animation state (the 0x5E-byte record's behaviour fields):
/// `+0x36` state flag, `+0x38` timer (init 0x32), `+0x3C` animation accumulator, plus
/// its PRNG seed word (`fs:[0x105C]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienObject {
    /// PRNG seed word (`fs:[0x105C]`).
    pub prng: u16,
    /// `+0x36` state flag (1 = a new animation state was just chosen this frame).
    pub state_flag: u16,
    /// `+0x38` countdown timer (initialised to 0x32 = 50 when a state is chosen).
    pub timer: u16,
    /// `+0x3C` animation accumulator (`cs:[0x16A2]` advanced by 0xFA per state change).
    pub anim: u16,
}

/// Timer reload when a new animation state is chosen (`+0x38 = 0x32`).
pub const ALIEN_STATE_TIMER_RELOAD: u16 = 0x32;
/// Animation-accumulator step per state change (`cs:[0x16A2] += 0xFA`).
pub const ALIEN_ANIM_STEP: u16 = 0xFA;

impl AlienObject {
    /// Create an object with the decoded initial state (`+0x38 = 0x32`), seeded PRNG.
    pub fn new(seed: u16) -> Self {
        Self {
            prng: seed,
            state_flag: 0,
            timer: ALIEN_STATE_TIMER_RELOAD,
            anim: 0,
        }
    }

    /// Advance one frame of the decoded state machine (`0x16A4`): the timer counts
    /// down; when it expires the PRNG picks a new animation state — `+0x36 = 1`,
    /// `+0x38 = 0x32`, `+0x3C += 0xFA` — otherwise the object holds its state
    /// (`+0x36 = 0`) and defers to its sub-behaviour. Returns `true` on a state change.
    pub fn step(&mut self) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
            self.state_flag = 0;
            return false;
        }
        self.prng = alien_anim_prng_next(self.prng);
        self.state_flag = 1;
        self.timer = ALIEN_STATE_TIMER_RELOAD;
        self.anim = self.anim.wrapping_add(ALIEN_ANIM_STEP);
        true
    }
}

/// The overlay's per-frame object-list dispatcher (method `0x12DE`): each `0x12DE`
/// call iterates `cx = [di+0x1A]` sub-objects, calling each object's sub-method
/// (`call [si+0xE]`, `si += 0x5E`), but only when the frame timer `cs:0xB72` has
/// elapsed (it resets to 7) — so the colony advances every 7th frame. This ports the
/// dispatch cadence + object iteration; each object runs its [`AlienObject`] state
/// machine.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AlienColony {
    /// The overlay's list of behaviour objects (`0x5E`-byte records).
    pub objects: Vec<AlienObject>,
    /// Frame-gate countdown (`cs:0xB72`); the colony steps when it reaches 0, then
    /// reloads to [`ALIEN_COLONY_FRAME_GATE`].
    pub frame_timer: u8,
}

/// The dispatcher's frame-gate reload (`cs:0xB72` reset value = 7).
pub const ALIEN_COLONY_FRAME_GATE: u8 = 7;

impl AlienColony {
    /// A colony of `count` objects, PRNG-seeded distinctly (the overlay seeds each
    /// object from `fs:[0x105C]`; here we vary the seed per index so they de-sync).
    pub fn new(count: usize, base_seed: u16) -> Self {
        Self {
            objects: (0..count)
                .map(|i| AlienObject::new(base_seed.wrapping_add((i as u16).wrapping_mul(0x9E3B))))
                .collect(),
            frame_timer: ALIEN_COLONY_FRAME_GATE,
        }
    }

    /// Advance one frame: gated by `cs:0xB72`, step every object's state machine on the
    /// 7th frame (decrement, and when it hits 0 update + reload to 7). Returns `true`
    /// on the frames the colony actually updated.
    pub fn step(&mut self) -> bool {
        self.frame_timer = self.frame_timer.saturating_sub(1);
        if self.frame_timer != 0 {
            return false;
        }
        self.frame_timer = ALIEN_COLONY_FRAME_GATE;
        for object in &mut self.objects {
            object.step();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colony_advances_on_the_frame_gate_cadence() {
        let mut colony = AlienColony::new(3, 0x1234);
        assert_eq!(colony.objects.len(), 3);
        // No update until the gate elapses (7 frames), then one update.
        let mut updates = 0;
        for _ in 0..(ALIEN_COLONY_FRAME_GATE as u32 * 3 + 1) {
            if colony.step() {
                updates += 1;
            }
        }
        assert_eq!(updates, 3, "colony updates once per 7-frame gate");
        // Objects are seeded distinctly so they don't all change state in lockstep.
        assert_ne!(colony.objects[0].prng, colony.objects[1].prng);
    }

    #[test]
    fn anim_prng_matches_ror7_sbb() {
        // Reference: rotate_right(seed,7) then subtract its MSB (the 8086 carry).
        for seed in [0x0001u16, 0x8000, 0x1234, 0xFFFF, 0x0080] {
            let rotated = seed.rotate_right(7);
            let expected = rotated.wrapping_sub(rotated >> 15);
            assert_eq!(alien_anim_prng_next(seed), expected);
        }
        // 0x8000 ror 7 = 0x0100 (MSB 0 → no borrow) = 0x0100.
        assert_eq!(alien_anim_prng_next(0x8000), 0x0100);
        // 0x0040 ror 7 = 0x8000 (MSB 1 → borrow 1) = 0x7FFF.
        assert_eq!(alien_anim_prng_next(0x0040), 0x7FFF);
    }

    #[test]
    fn object_holds_then_changes_state_on_timer_expiry() {
        let mut obj = AlienObject::new(0x1357);
        assert_eq!(obj.timer, ALIEN_STATE_TIMER_RELOAD);
        // Holds for the timer window (no state change, flag stays 0).
        for _ in 0..ALIEN_STATE_TIMER_RELOAD {
            assert!(!obj.step());
            assert_eq!(obj.state_flag, 0);
        }
        // Timer now 0 → next step chooses a new state.
        let anim_before = obj.anim;
        assert!(obj.step(), "state change on timer expiry");
        assert_eq!(obj.state_flag, 1);
        assert_eq!(obj.timer, ALIEN_STATE_TIMER_RELOAD);
        assert_eq!(obj.anim, anim_before.wrapping_add(ALIEN_ANIM_STEP));
    }
}
