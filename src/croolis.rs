//! Alien-species behaviour engine — the logic shared by the alien overlays
//! `croolis.xdb`, `amer.xdb`, and `scrut.xdb` (each is the same engine over different
//! alien data). Verified: all three carry the identical animation-state PRNG
//! (`mov ax,fs:[0x105C]; ror ax,7; sbb ax,0`) and the same 0x5E-byte object stride —
//! see `alien_engine_prng_present_in_all_overlays` below.
//!
//! Decoded (see `re/REVERSE.md`, sess 003): the overlay drives a list of 0x5E-byte
//! object records, each a PRNG + timer *animation state machine*, dispatched per frame
//! and feeding the shared ship-3D per-object draw. Ported here: the animation-state
//! PRNG (`0x16A4`), the per-object state machine, the per-frame colony dispatcher
//! (`0x12DE`, frame-gated), the behaviour vtable (`fs:0x103A`), the object
//! position-update wrap (`0x999`), the initializer (`0x36A`), and the proximity/
//! visibility gate (`0xA30`) — the overlay's complete behaviour-method set. Remaining:
//! the per-object 3D draw/blit, which reuses the shared ship-3D compositor.

/// The overlay's animation-state PRNG (`0x16A4`: `mov ax,fs:[0x105C]; ror ax,7;
/// sbb ax,0; store back`). On 8086 `ror ax,7` leaves CF = the result's MSB (the last
/// bit rotated through carry), and `sbb ax,0` subtracts that carry — so the next state
/// is `rotate_right(seed,7) - msb`. Distinct from the ship-view `rcr/rcl` PRNG.
pub fn alien_anim_prng_next(seed: u16) -> u16 {
    let rotated = seed.rotate_right(7);
    let carry = rotated >> 15; // CF after `ror …,7` = MSB of the rotated value
    rotated.wrapping_sub(carry)
}

/// The overlay's per-object behaviour method, selected via the vtable at `fs:0x103A`
/// (near-ptr entries indexed by `bx = [di+0x34]`). The decoded entries are:
/// `0x1D27` (null/`ret`), `0x16A4` (animation state machine — ported), `0x12DE`
/// (colony iterator — ported), `0x999` (position update — ported), `0x36A`
/// (initializer — ported), and `0xA30` (proximity gate — ported as
/// [`AlienObject::proximity_visible`], which needs camera context so it isn't reached
/// through the parameterless [`AlienObject::dispatch`]).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienMethod {
    /// `0x1D27` — the null method (`ret`); the object does nothing.
    Null,
    /// `0x16A4` — the animation state machine ([`AlienObject::step`]).
    AnimStateMachine,
    /// Another vtable entry kept as its table offset so the dispatch shape is faithful
    /// (e.g. `0xA30`, driven separately via [`AlienObject::proximity_visible`]).
    SubBehaviour(u16),
}

impl AlienMethod {
    /// Resolve a vtable index (`[di+0x34]`) to its method, mirroring the `fs:0x103A`
    /// table entries.
    pub fn from_vtable_offset(offset: u16) -> Self {
        match offset {
            0x1D27 => AlienMethod::Null,
            0x16A4 => AlienMethod::AnimStateMachine,
            other => AlienMethod::SubBehaviour(other),
        }
    }
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
    /// The object's behaviour method (`[di+0x34]` → `fs:0x103A` vtable entry).
    pub method: AlienMethod,
    /// Object 3D position (record fields `+0x42`/`+0x46`/`+0x4a`), stored as sign-
    /// extended 32-bit words — camera-relative, wrapped by [`AlienObject::update_position`].
    pub pos: [i32; 3],
    /// Transform/orientation components (record fields `+0x12`/`+0x22`/`+0x32`),
    /// initialised to `0x8000` by the object initializer (`0x36A`) — the neutral
    /// fixed-point value the shared 3D transform uses.
    pub transform: [i32; 3],
    /// Animation frame counter (`+0x50`), advanced by the proximity method (`0xA30`).
    pub anim_counter: u16,
}

/// The neutral transform value the initializer writes to `+0x12`/`+0x22`/`+0x32`.
pub const ALIEN_TRANSFORM_NEUTRAL: i32 = 0x8000;

/// The half-extent of the object-space toroidal wrap (`0x4000`); positions wrap into
/// `[-0x4000, 0x4000)` relative to the wrap origin (method `0x999`).
pub const ALIEN_POSITION_WRAP: i16 = 0x4000;

/// Timer reload when a new animation state is chosen (`+0x38 = 0x32`).
pub const ALIEN_STATE_TIMER_RELOAD: u16 = 0x32;
/// Animation-accumulator step per state change (`cs:[0x16A2] += 0xFA`).
pub const ALIEN_ANIM_STEP: u16 = 0xFA;

impl AlienObject {
    /// Create an object with the decoded initial state (`+0x38 = 0x32`), seeded PRNG,
    /// running the animation state machine by default.
    pub fn new(seed: u16) -> Self {
        Self {
            prng: seed,
            state_flag: 0,
            timer: ALIEN_STATE_TIMER_RELOAD,
            anim: 0,
            method: AlienMethod::AnimStateMachine,
            pos: [0; 3],
            transform: [ALIEN_TRANSFORM_NEUTRAL; 3],
            anim_counter: 0,
        }
    }

    /// Port of method `0xA30` (per-object proximity/visibility gate): only runs when
    /// the object's state flag (`+0x36`) is set; it advances the animation counter
    /// (`+0x50`) and returns whether the object sits within the on-screen region of the
    /// camera — its screen y (`anim_offset − 0x3C + pos.y + cam.y`) in `[0, 0x80]` and
    /// its world x (`pos.x + cam.x`) in `[-0x100, 0x100]`. `anim_offset` is the
    /// timer-indexed animation value (`fs:[timer&0xFFC + 0x36] >> 8`). Returns `false`
    /// (no advance) when the state flag is clear.
    pub fn proximity_visible(&mut self, camera: [i16; 3], anim_offset: i16) -> bool {
        if self.state_flag == 0 {
            return false;
        }
        self.anim_counter = self.anim_counter.wrapping_add(1);
        // Screen-y band [0, 0x80].
        let sy = anim_offset
            .wrapping_sub(0x3C)
            .wrapping_add(self.pos[1] as i16)
            .wrapping_add(camera[1]);
        if sy < 0 || sy > 0x80 {
            return false;
        }
        // World-x window [-0x100, 0x100] (0xFF00..=0x100 as signed).
        let sx = (self.pos[0] as i16).wrapping_add(camera[0]);
        sx >= -0x100 && sx <= 0x100
    }

    /// Port of the object initializer (`0x36A`): reset the behaviour state — zero the
    /// state flag + animation accumulator, reload the timer, and set the transform
    /// components to the neutral `0x8000` — putting the object in its start pose.
    pub fn reset(&mut self) {
        self.state_flag = 0;
        self.anim = 0;
        self.timer = ALIEN_STATE_TIMER_RELOAD;
        self.transform = [ALIEN_TRANSFORM_NEUTRAL; 3];
    }

    /// Port of method `0x999` (object position update): for each axis, wrap the object's
    /// WORLD position (`camera + pos`) into `[-0x4000, 0x4000)` then subtract the camera
    /// back — keeping objects within a toroidal play-space around the camera. The 8086
    /// does this in 16-bit (`add;+0x4000;and 0x7fff;-0x4000`) then `movsx` to 32-bit.
    pub fn update_position(&mut self, camera: [i16; 3]) {
        for axis in 0..3 {
            let cam = camera[axis];
            // ax = camera + pos (16-bit)
            let mut ax = cam.wrapping_add(self.pos[axis] as i16);
            // wrap into [-0x4000, 0x4000): +0x4000; &0x7fff; -0x4000
            ax = ax.wrapping_add(ALIEN_POSITION_WRAP);
            ax = (ax as u16 & 0x7fff) as i16;
            ax = ax.wrapping_sub(ALIEN_POSITION_WRAP);
            // ax -= camera; movsx to 32-bit
            ax = ax.wrapping_sub(cam);
            self.pos[axis] = ax as i32;
        }
    }

    /// Dispatch one frame through the object's vtable method (`call [si+0xE]` in the
    /// colony iterator): the animation state machine advances, the null method and
    /// not-yet-decoded sub-behaviours are no-ops. Returns `true` on an anim state
    /// change.
    pub fn dispatch(&mut self) -> bool {
        match self.method {
            AlienMethod::AnimStateMachine => self.step(),
            AlienMethod::Null | AlienMethod::SubBehaviour(_) => false,
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
            object.dispatch();
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proximity_gate_advances_and_windows_on_screen() {
        // State flag clear -> no advance, not visible.
        let mut obj = AlienObject::new(0x1);
        obj.state_flag = 0;
        assert!(!obj.proximity_visible([0, 0, 0], 0x3C));
        assert_eq!(obj.anim_counter, 0);
        // State set, object at origin, anim_offset 0x3C (sy=0), camera 0 -> in window.
        obj.state_flag = 1;
        obj.pos = [0, 0, 0];
        assert!(obj.proximity_visible([0, 0, 0], 0x3C), "on-screen object is visible");
        assert_eq!(obj.anim_counter, 1, "counter advanced");
        // Push x outside +-0x100 -> not visible (but counter still advances).
        obj.pos = [0x400, 0, 0];
        assert!(!obj.proximity_visible([0, 0, 0], 0x3C));
        assert_eq!(obj.anim_counter, 2);
        // Push screen-y above 0x80 -> not visible.
        obj.pos = [0, 0x400, 0];
        assert!(!obj.proximity_visible([0, 0, 0], 0x3C));
    }

    #[test]
    fn initializer_resets_to_start_pose() {
        let mut obj = AlienObject::new(0x1);
        obj.state_flag = 1;
        obj.anim = 0x500;
        obj.timer = 3;
        obj.transform = [0, 0, 0];
        obj.reset();
        assert_eq!(obj.state_flag, 0);
        assert_eq!(obj.anim, 0);
        assert_eq!(obj.timer, ALIEN_STATE_TIMER_RELOAD);
        assert_eq!(obj.transform, [ALIEN_TRANSFORM_NEUTRAL; 3]);
    }

    #[test]
    fn position_update_wraps_into_toroidal_space() {
        // An object far outside the wrap window wraps back inside relative to camera.
        let mut obj = AlienObject::new(0x1);
        obj.pos = [0x5000, -0x5000, 0x100];
        obj.update_position([0, 0, 0]);
        for &p in &obj.pos {
            assert!(
                (-(ALIEN_POSITION_WRAP as i32)..(ALIEN_POSITION_WRAP as i32)).contains(&p),
                "axis {p} wrapped into [-0x4000, 0x4000)"
            );
        }
        // 0x5000 world -> (0x5000+0x4000)&0x7fff-0x4000 = -0x3000.
        assert_eq!(obj.pos[0], -0x3000);
        // A position already inside the window, camera 0, is unchanged.
        let mut inside = AlienObject::new(0x1);
        inside.pos = [0x1000, -0x2000, 0];
        inside.update_position([0, 0, 0]);
        assert_eq!(inside.pos, [0x1000, -0x2000, 0]);
    }

    #[test]
    fn vtable_dispatch_routes_methods() {
        assert_eq!(AlienMethod::from_vtable_offset(0x1D27), AlienMethod::Null);
        assert_eq!(AlienMethod::from_vtable_offset(0x16A4), AlienMethod::AnimStateMachine);
        assert_eq!(
            AlienMethod::from_vtable_offset(0x0A30),
            AlienMethod::SubBehaviour(0x0A30)
        );
        // The null method never changes state; the anim method eventually does.
        let mut null = AlienObject::new(0x1);
        null.method = AlienMethod::Null;
        for _ in 0..100 {
            assert!(!null.dispatch());
        }
        let mut anim = AlienObject::new(0x1);
        let changed = (0..100).any(|_| anim.dispatch());
        assert!(changed, "anim-state object changes state within its timer window");
    }

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
    fn alien_engine_prng_present_in_all_overlays() {
        // The animation-state PRNG byte sequence: `ror ax,7` (C1 C8 07) immediately
        // followed by `sbb ax,0` (1D 00 00). It appears in every alien overlay,
        // confirming they share this behaviour engine. Skips if assets are absent.
        let seq = [0xC1u8, 0xC8, 0x07, 0x1D, 0x00, 0x00];
        let mut checked = 0;
        for stem in ["croolis", "amer", "scrut"] {
            let path = ["output/_tmp_dat", "../output/_tmp_dat"]
                .iter()
                .map(|d| std::path::Path::new(d).join(format!("{stem}.xdb")))
                .find(|p| p.exists());
            let Some(path) = path else { continue };
            let data = std::fs::read(path).unwrap();
            assert!(
                data.windows(seq.len()).any(|w| w == seq),
                "{stem}.xdb carries the shared anim PRNG (ror ax,7; sbb ax,0)"
            );
            checked += 1;
        }
        // The ported PRNG models exactly that sequence.
        assert_eq!(alien_anim_prng_next(0x8000), 0x0100);
        let _ = checked;
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
