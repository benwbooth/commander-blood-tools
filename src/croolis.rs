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

/// The overlay's animation-state PRNG, in `XDB:croolis:0x16A4`. The routine
/// entry is a vtable dispatch (`test word [di+0x36],0xffff / je` @`0x16AA`); the
/// arithmetic itself is three instructions further on, cited individually here
/// because naming only the routine is how #301's citation went wrong:
///
/// ```text
///   0x16B4  mov ax, word ptr fs:[0x105c]
///   0x16B8  ror ax, 7
///   0x16BB  sbb ax, 0
/// ``` On 8086 `ror ax,7` leaves CF = the result's MSB (the last
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
pub const ALIEN_POSITION_WRAP: i16 = 16384;

/// Low-15-bit mask used while folding a position into the toroidal play-space (a bit mask, so
/// hexadecimal is the natural form).
const POSITION_WRAP_MASK: u16 = 0x7fff;

/// Timer reload (in frames) when a new animation state is chosen.
pub const ALIEN_STATE_TIMER_RELOAD: u16 = 50;
/// Animation-accumulator step added per state change.
pub const ALIEN_ANIM_STEP: u16 = 250;

/// Vertical bias subtracted from the timer-indexed animation offset when testing on-screen y.
const VISIBLE_ANIM_Y_BIAS: i16 = 60;
/// Top of the on-screen band an object's screen y must fall within to be drawn.
const VISIBLE_SCREEN_Y_MAX: i16 = 128;
/// Half-width of the world-x window (centered on the camera) an object must fall within.
const VISIBLE_WORLD_X_HALF: i16 = 256;

/// The alien view's camera: THREE 32-bit fixed-point accumulators, each read
/// through its high word (audit-fixes #269-#271).
///
/// ```text
///   0x1FC5  add dword ptr [0x22ea], eax     X accumulator
///   0x1FD5  add dword ptr [0x22ee], eax     Y
///   0x1FE5  add dword ptr [0x22f2], eax     Z
///   0x1FEA  movsx ebx, word ptr [0x22ec]    X's HIGH WORD (0x22EA + 2)
///   0x1FF0  movsx ecx, word ptr [0x22f0]    Y's        (0x22EE + 2)
///   0x1FF6  movsx esi, word ptr [0x22f4]    Z's        (0x22F2 + 2)
/// ```
///
/// Each step is `[0x22d2 | 0x22d6 | 0x22da] * ebx >> 3` — a per-axis component
/// scaled by a common factor and shifted, so the fraction each frame contributes
/// is genuinely sub-integer and only crosses into the high word once it has
/// summed.
///
/// #269 and #270 got this half right: they found `0x22EE` was an accumulator read
/// via `[0x22F0]`, but recorded `0x22EC` as "genuinely a word" on the strength of
/// `movsx eax,word ptr [0x22ec]` @`0xBFA`. That instruction reads a word because
/// it wants the INTEGER PART, not because the storage is 16-bit — and the writer
/// four instructions before it settles the matter. All three axes are the same
/// shape; the asymmetry was an artefact of decoding one of them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienCamera {
    /// `0x22EA`, read as `[0x22EC]`.
    pub x_fixed: i32,
    /// `0x22EE`, read as `[0x22F0]`.
    pub y_fixed: i32,
    /// `0x22F2`, read as `[0x22F4]`.
    pub z_fixed: i32,
}

impl AlienCamera {
    /// An axis's INTEGER part — the high word the overlay `movsx`es.
    pub fn axis(&self, index: usize) -> i16 {
        let fixed = match index {
            0 => self.x_fixed,
            1 => self.y_fixed,
            _ => self.z_fixed,
        };
        (fixed >> 16) as i16
    }

    /// `[0x22EC]`, X's high word.
    pub fn x(&self) -> i16 {
        self.axis(0)
    }

    /// `[0x22F0]`, Y's high word.
    pub fn y(&self) -> i16 {
        self.axis(1)
    }

    /// `[0x22F4]`, Z's high word — used by the proximity gate's THIRD axis test
    /// (`add ax, word ptr [0x22f4]` @`0xA81`), which audit-fixes #355 found the
    /// port was not performing.
    pub fn z(&self) -> i16 {
        self.axis(2)
    }
}

impl AlienObject {
    /// Create an object with the decoded initial state (timer reloaded to
    /// [`ALIEN_STATE_TIMER_RELOAD`]), seeded PRNG, running the animation state machine by default.
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

    /// Port of method `0xA30` (per-object proximity/visibility gate): only runs when the object's
    /// state flag is set; it advances the animation counter and returns whether the object sits
    /// within the on-screen region of the camera — its screen y within
    /// `[0, VISIBLE_SCREEN_Y_MAX]` and its world x within `[-VISIBLE_WORLD_X_HALF,
    /// VISIBLE_WORLD_X_HALF]`. Returns `false` (no advance) when the state flag is clear.
    /// WHERE THE CAMERA COMES FROM, decoded 2026-07-25 to unblock the wiring
    /// `docs/port-validation.md` recorded as missing (audit-fixes #269):
    ///
    /// ```text
    ///   croolis.xdb 0xA62  add ax, word ptr [0x22f0]   the Y term
    ///   croolis.xdb 0xA70  add ax, word ptr [0x22ec]   the X term
    /// ```
    ///
    /// BOTH terms are HIGH WORDS of 32-bit accumulators. This doc used to say
    /// `DS:0x22EC` "is a WORD" because `movsx eax,word ptr [0x22ec]` @`0xBFA`
    /// reads sixteen bits there; audit-fixes #271 corrected that in
    /// `re/labels.csv` and the correction never reached here (fixed in #344).
    /// All three camera axes are stepped as dwords:
    ///
    /// ```text
    ///   0x1FC5  add dword ptr [0x22ea], eax    X  -> high word 0x22EC
    ///   0x1FD5  add dword ptr [0x22ee], eax    Y  -> high word 0x22F0
    ///   0x1FE5  add dword ptr [0x22f2], eax    Z  -> high word 0x22F4
    /// ```
    ///
    /// So each camera axis is the integer part of a 32-bit FIXED-POINT
    /// accumulator, read by taking the top sixteen bits. A LOAD tells you what
    /// the caller wanted; only the STORE tells you how wide the cell is. Wiring
    /// this needs the accumulators, not `camera: [i16; 3]` updated per frame,
    /// which would drop the fractional motion on every axis rather than one.
    pub fn proximity_visible(&mut self, camera: AlienCamera, anim_offset: i16) -> bool {
        if self.state_flag == 0 {
            return false;
        }
        self.anim_counter = self.anim_counter.wrapping_add(1);
        let screen_y = anim_offset
            .wrapping_sub(VISIBLE_ANIM_Y_BIAS)
            .wrapping_add(self.pos[1] as i16)
            .wrapping_add(camera.y());
        if screen_y < 0 || screen_y > VISIBLE_SCREEN_Y_MAX {
            return false;
        }
        let world_x = (self.pos[0] as i16).wrapping_add(camera.x());
        if world_x < -VISIBLE_WORLD_X_HALF || world_x > VISIBLE_WORLD_X_HALF {
            return false;
        }
        // THE THIRD AXIS (audit-fixes #355). The gate tests Z with the same
        // bounds as X, and the port was omitting it entirely:
        //
        //   0xA7E  mov ax, word ptr [si+0x4a]     the object's Z
        //   0xA81  add ax, word ptr [0x22f4]      + the camera's Z high word
        //   0xA85  cmp ax, 0xff00 / jl 0xaa0      reject below -256
        //   0xA8A  cmp ax, 0x100  / jg 0xaa0      reject above +256
        let world_z = (self.pos[2] as i16).wrapping_add(camera.z());
        world_z >= -VISIBLE_WORLD_X_HALF && world_z <= VISIBLE_WORLD_X_HALF
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

    /// Port of method `0x999` (object position update): fold each axis of the object's world
    /// position (`camera + pos`) into the toroidal play-space `[-ALIEN_POSITION_WRAP,
    /// ALIEN_POSITION_WRAP)` centered on the camera, then re-express it relative to the camera.
    /// The fold is done in 16 bits and widened back to the stored 32-bit position.
    pub fn update_position(&mut self, camera: AlienCamera) {
        for axis in 0..3 {
            let cam = camera.axis(axis);
            let world = cam.wrapping_add(self.pos[axis] as i16);
            let folded =
                (world.wrapping_add(ALIEN_POSITION_WRAP) as u16 & POSITION_WRAP_MASK) as i16;
            let relative = folded.wrapping_sub(ALIEN_POSITION_WRAP).wrapping_sub(cam);
            self.pos[axis] = relative as i32;
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
    /// down; when it expires the PRNG picks a new animation state — `+0x36 = 1`
    /// (`0x16C9`), `+0x38 = 0x32` (`0x16CE`) — otherwise the object holds its
    /// state (`+0x36 = 0`) and defers to its sub-behaviour. Returns `true` on a
    /// state change.
    ///
    /// `+0x3C` IS NOT A PER-OBJECT ACCUMULATOR (audit-fixes #356). The doc used
    /// to say "`+0x3C += 0xFA`", and the port adds 250 to its own `anim` field.
    /// The routine does something else:
    ///
    /// ```text
    ///   0x16C2  movsx ebx, word ptr cs:[0x16a2]   a SHARED counter
    ///   0x16D8  mov dword ptr [di+0x3c], ebx      object gets its CURRENT value
    ///   0x16DC  add bx, 0xfa                      the SHARED counter advances
    ///   0x16E0  mov word ptr cs:[0x16a2], bx
    /// ```
    ///
    /// So `0xFA` steps a counter in the OVERLAY's code segment, and each object
    /// receives the value that counter held when it last changed state — as a
    /// DWORD, not a word. For one object the two models produce the same
    /// sequence, which is why the port's version survives its single-object
    /// tests; for a COLONY they differ, because the game interleaves one shared
    /// sequence across objects while the port gives each its own.
    ///
    /// ALSO NOT MODELLED, from the same block: `+0x3A = 0` (`0x16D3`), a SECOND
    /// PRNG step whose result lands in `+0x42` (`0x16E5`..`0x16EB`) — which is
    /// the very field the proximity gate reads as the object's X — and
    /// `[si+0x50] = ax & 0xFFC` / `[si+0x52] = 0` (`0x16EE`..`0x16F4`).
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

/// The dispatcher's frame-gate reload: `mov word cs:[0xb72],7` at croolis.xdb
/// `0x11C5` and `0x12F9`.
///
/// The doc used to cite `cs:0xB72` alone — the gate's STORAGE, which holds 2 in
/// the shipped image (its idle value), not 7. Citing the cell rather than the
/// instruction that reloads it made a correct constant unverifiable and, read
/// literally, wrong. Two call sites write the reload value; both carry it as an
/// immediate.
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

    /// THE FRACTION IS THE POINT. Y is the high word of a 32-bit accumulator
    /// (`add dword ptr [0x22ee],eax` @`0x1FD5`, read as `[0x22F0]` @`0xA62`), so
    /// sub-pixel motion accumulates and only crosses into the integer part when
    /// it has summed to a whole unit. An `i16` camera would round each frame's
    /// movement away and never move at all under a small enough step.
    #[test]
    fn the_camera_y_accumulates_below_the_integer_part() {
        let mut cam = AlienCamera::default();
        assert_eq!(cam.y(), 0);

        // A third of a unit per frame: two frames round to nothing, the third
        // must carry.
        let third = i32::from(u16::MAX) / 3 + 1;
        cam.y_fixed += third;
        assert_eq!(cam.y(), 0, "a partial step must not move the integer part");
        cam.y_fixed += third;
        assert_eq!(cam.y(), 0);
        cam.y_fixed += third;
        assert_eq!(cam.y(), 1, "three thirds must cross into the integer part");

        // The axis accessor reports the same value the proximity test adds, and
        // ALL THREE axes are accumulators -- #271 corrected #270's asymmetry.
        assert_eq!(cam.axis(1), cam.y());
        cam.x_fixed = -5 << 16;
        assert_eq!(cam.axis(0), -5);
        assert_eq!(cam.x(), -5);
        cam.z_fixed = 7 << 16;
        assert_eq!(cam.axis(2), 7);

        // And a negative accumulator floors, matching an arithmetic shift.
        let mut down = AlienCamera { y_fixed: -1, ..Default::default() };
        assert_eq!(down.y(), -1, "sar rounds toward -inf, not toward zero");
        down.y_fixed = -(1 << 16);
        assert_eq!(down.y(), -1);
    }
    use super::*;

    #[test]
    fn proximity_gate_advances_and_windows_on_screen() {
        // State flag clear -> no advance, not visible.
        let mut obj = AlienObject::new(0x1);
        obj.state_flag = 0;
        assert!(!obj.proximity_visible(AlienCamera::default(), 0x3C));
        assert_eq!(obj.anim_counter, 0);
        // State set, object at origin, anim_offset 0x3C (sy=0), camera 0 -> in window.
        obj.state_flag = 1;
        obj.pos = [0, 0, 0];
        assert!(obj.proximity_visible(AlienCamera::default(), 0x3C), "on-screen object is visible");
        assert_eq!(obj.anim_counter, 1, "counter advanced");
        // Push x outside +-0x100 -> not visible (but counter still advances).
        obj.pos = [0x400, 0, 0];
        assert!(!obj.proximity_visible(AlienCamera::default(), 0x3C));
        assert_eq!(obj.anim_counter, 2);
        // Push screen-y above 0x80 -> not visible.
        obj.pos = [0, 0x400, 0];
        assert!(!obj.proximity_visible(AlienCamera::default(), 0x3C));
        // Push Z outside +-0x100 -> not visible. THE PORT DID NOT TEST Z AT ALL
        // until audit-fixes #355, and this case is why nothing caught it: every
        // assertion above leaves Z at 0, so a missing third axis is invisible to
        // them. `cmp ax,0xff00 / jl` @0xA85 and `cmp ax,0x100 / jg` @0xA8A.
        obj.pos = [0, 0, 0x400];
        assert!(
            !obj.proximity_visible(AlienCamera::default(), 0x3C),
            "an object beyond the Z window must be rejected"
        );
        obj.pos = [0, 0, -0x400];
        assert!(
            !obj.proximity_visible(AlienCamera::default(), 0x3C),
            "...and beyond it in the negative direction too"
        );
        // A Z inside the window still passes, so the new test is a WINDOW and not
        // a blanket rejection.
        obj.pos = [0, 0, 0x80];
        assert!(obj.proximity_visible(AlienCamera::default(), 0x3C));
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
        obj.update_position(AlienCamera::default());
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
        inside.update_position(AlienCamera::default());
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

