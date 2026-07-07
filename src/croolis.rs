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

#[cfg(test)]
mod tests {
    use super::*;

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
