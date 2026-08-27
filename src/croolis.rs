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

/// The overlay's SHARED behaviour cells — the two counters every object in a
/// colony draws from, rather than owning (audit-fixes #400, #401):
///
/// ```text
///   0x16B4  mov ax, word ptr fs:[0x105c]     the PRNG stream...
///   0x16BE  mov word ptr fs:[0x105c], ax     ...written back, so it is global
///   0x16C2  movsx ebx, word ptr cs:[0x16a2]  the anim counter...
///   0x16DC  add bx, 0xfa                     ...advanced by 0xFA per draw
///   0x16E0  mov word ptr cs:[0x16a2], bx     ...and stored in the CODE segment
/// ```
///
/// Both live outside the 0x5E-byte object record, which is why modelling either
/// as a per-object field produces the right sequence for ONE object and the
/// wrong interleaving for a colony.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienStreams {
    /// `fs:[0x105C]` — the animation-state PRNG stream.
    pub prng: u16,
    /// `cs:[0x16A2]` — the animation counter, advanced by [`ALIEN_ANIM_STEP`].
    pub anim: u16,
}

impl AlienStreams {
    /// Seed both shared cells.
    pub fn new(prng: u16, anim: u16) -> Self {
        Self { prng, anim }
    }
}

/// A `croolis` object's animation state (the 0x5E-byte record's behaviour fields):
/// `+0x36` state flag, `+0x38` timer (init 0x32), `+0x3C` animation accumulator, plus
/// its PRNG seed word (`fs:[0x105C]`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienObject {
    /// The PRNG seed this object last drew (`fs:[0x105C]` AFTER its own step).
    ///
    /// NOT the sequence itself (audit-fixes #400). `fs:0x105C` is a GLOBAL —
    /// `mov ax, word ptr fs:[0x105c]` @`XDB:croolis:0x16B4`, stepped by
    /// `ror ax,7 / sbb ax,0`, written back by `mov word ptr fs:[0x105c], ax`
    /// @`0x16BE` — so every object in the colony draws from ONE shared stream,
    /// in the order their timers expire. This field records what this object
    /// drew; [`AlienColony::prng`] holds the stream.
    pub prng: u16,
    /// `+0x36` state flag (1 = a new animation state was just chosen this frame).
    pub state_flag: u16,
    /// `+0x38` countdown timer (initialised to 0x32 = 50 when a state is chosen).
    pub timer: u16,
    /// `+0x3C` — the value the SHARED counter `cs:[0x16A2]` held when this object
    /// last changed state, sign-extended to 32 bits: `movsx ebx, word ptr
    /// cs:[0x16a2]` @`0x16C2` then `mov dword ptr [di+0x3c], ebx` @`0x16D8`.
    /// A DWORD store of a WORD counter, so the sign extension is observable.
    pub anim: i32,
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
/// `0x8000`, written by the initializer at `XDB:croolis:0x385`, `0x38D` and
/// `0x395` — three `mov dword ptr [si+0x12/0x22/0x32], 0x8000` stores, one per
/// component (audit-fixes #546).
///
/// It is 1.0 in the matrix format: `ship3d::SHIP_3D_MATRIX_FIXED_SHIFT` is 15
/// (`sar e_x,0xf`, #499), so `0x8000 >> 15 == 1`. The neutral transform and the
/// projection's fixed-point scale are the same fact in two subsystems.
pub const ALIEN_TRANSFORM_NEUTRAL: i32 = 0x8000;

/// The half-extent of the object-space toroidal wrap (`0x4000`); positions wrap into
/// `[-0x4000, 0x4000)` relative to the wrap origin (method `0x999`).
pub const ALIEN_POSITION_WRAP: i16 = 16384;

/// Low-15-bit mask used while folding a position into the toroidal play-space (a bit mask, so
/// hexadecimal is the natural form).
/// `mov bp,0x7fff` @`XDB:croolis:0x9A2`, loaded beside `mov di,0x4000` @`0x99F`
/// ([`ALIEN_POSITION_WRAP`]) at the head of the toroidal wrap — the mask and the
/// half-extent are set up together, two instructions apart (audit-fixes #546).
const POSITION_WRAP_MASK: u16 = 0x7fff;

/// Timer reload (in frames) when a new animation state is chosen.
/// `mov word ptr [di+0x38],0x32` @`XDB:croolis:0x16CE` — 50, stored in the same
/// run as the state flag `[di+0x36] = 1` @`0x16C9` and the cleared `[di+0x3a]`
/// @`0x16D3`. Choosing a state, arming the timer and clearing the accumulator are
/// ONE sequence (audit-fixes #546).
pub const ALIEN_STATE_TIMER_RELOAD: u16 = 50;
/// Animation-accumulator step added per state change.
/// `add bx,0xfa` @`XDB:croolis:0x16DC` — 250 added to the SHARED counter
/// `cs:[0x16A2]` (stored back @`0x16E0`), not to anything per-object. See
/// [`AlienStreams`] (audit-fixes #400, #546).
pub const ALIEN_ANIM_STEP: u16 = 250;

/// Vertical bias subtracted from the timer-indexed animation offset when testing
/// on-screen y — `sub ax, 0x3c` at croolis.xdb `0xA5C` (audit-fixes #488).
///
/// The value it biases is read `fs:[(timer & 0xFFC) + 0x36]` @`0xA47` and shifted
/// `sar ax,8` @`0xA4C`, so the gate tests the HIGH BYTE of a timer-indexed table
/// entry, minus 60, plus the object's `+0x46` and the camera's `[0x22F0]`.
const VISIBLE_ANIM_Y_BIAS: i16 = 60;
/// Top of the on-screen band an object's screen y must fall within to be drawn —
/// `cmp ax,0x80 / jg` at croolis.xdb `0xA68`, inside the `0xA30` gate
/// (audit-fixes #488).
///
/// The lower bound is NOT a second compare: `js 0xAA0` @`0xA66` rejects any
/// negative y outright, so the window is `[0, 128]` — asymmetric, unlike the two
/// world axes below.
const VISIBLE_SCREEN_Y_MAX: i16 = 128;
/// Half-width of the world-x window (centered on the camera) an object must fall
/// within — croolis.xdb `0xA74` `cmp ax,0xff00 / jl` and `0xA79` `cmp ax,0x100 /
/// jg` (audit-fixes #488).
///
/// Both jumps are SIGNED (`jl`/`jg`), which is what makes `0xFF00` the literal
/// -256 rather than 65280, so the window really is symmetric `[-256, +256]`. The
/// Z axis @`0xA85`/`0xA8A` is tested against the same pair of immediates.
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
    ///
    /// All three, consecutively, at `XDB:croolis:0x0BFA`:
    ///
    /// ```text
    /// 0x0bfa  660fbf06ec22  movsx eax, word ptr [0x22ec]
    /// 0x0c00  660fbf1ef022  movsx ebx, word ptr [0x22f0]
    /// 0x0c06  660fbf0ef422  movsx ecx, word ptr [0x22f4]
    /// ```
    ///
    /// Each address is the accumulator's base + 2 — the HIGH half of the 32-bit
    /// cell — so `fixed >> 16` is the same read. The three run back to back and
    /// are identical in shape, which is what settles #269/#270's asymmetry: it
    /// came from decoding one axis, not from the axes differing.
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
            anim: 0i32,
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
    ///
    /// Verified at `XDB:croolis:0x036A` (audit-fixes #468). It writes through
    /// `mov si, word ptr [di + 0x16]` — the CHILD record, like every other method
    /// in this overlay (#402) — and the neutral value is explicit:
    /// `mov dword ptr [si + 0x12], 0x8000` @`0x0385`, `[si + 0x22]` @`0x038D`,
    /// `[si + 0x32]` @`0x0395`.
    ///
    /// AN ODDITY, recorded because a rewrite would silently "fix" it:
    /// `mov dword ptr [si + 0x3a], 0` appears TWICE, at `0x0375` and `0x037D`,
    /// byte-for-byte identical (`66 c7 44 3a 00 00 00 00`). The natural reading is
    /// that the second was meant to be `+0x3e` and the field one slot along is
    /// never initialised. Neither offset is modelled here, so the port is not
    /// currently affected either way — but if `+0x3a`/`+0x3e` are ever added, the
    /// game zeroes the first twice and the second never.
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
    pub fn dispatch(&mut self, shared: &mut AlienStreams) -> bool {
        match self.method {
            AlienMethod::AnimStateMachine => self.step(shared),
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
    /// `+0x42` IS NOW MODELLED (audit-fixes #401) as the second, non-written-back
    /// PRNG step. Still not modelled: `+0x3A = 0` (`0x16D3`), an unidentified
    /// field.
    ///
    /// AND THE `[si+…]` WRITES ARE ON A DIFFERENT RECORD (audit-fixes #401). The
    /// routine opens:
    ///
    /// ```text
    ///   0x16A4  mov si, word ptr [di + 0x16]   a pointer out of THIS object
    ///   0x16A7  add si, 0x5e                   ...advanced by ONE OBJECT STRIDE
    ///   0x16AA  test word ptr [di + 0x36], 0xffff
    ///   0x16AF  je 0x16B4                      state_flag == 0 -> the machine
    ///   0x16B1  jmp word ptr [si + 0xe]        else -> that record's sub-method
    /// ```
    ///
    /// So `si` is a RELATED record at `[di+0x16] + 0x5E`, not this one, and the
    /// tail writes (`[si+0x50] = ax & 0xFFC`, `[si+0x52] = 0` @`0x16EE`..`0x16F4`,
    /// and `mov word ptr [si+0xe], 0x1727` @`0x16FE`, which INSTALLS a sub-method
    /// on it) all target that neighbour.
    ///
    /// WHAT `+0x16` IS, decoded in #403: objects are a TREE, not a flat list.
    /// The colony dispatcher opens with the same pointer plus its count —
    ///
    /// ```text
    ///   0x12DE  mov si, word ptr [di + 0x16]   the CHILD ARRAY base
    ///   0x12E1  mov cx, word ptr [di + 0x1a]   the CHILD COUNT
    ///   0x12E4  add si, 0x5e                   iteration starts at element 1
    ///   0x1301  call word ptr [si + 0xe]       each child's method
    /// ```
    ///
    /// — and every one of the overlay's sixteen `[reg+0x16]` accesses is a READ
    /// (`0x36A`, `0x966`, `0x999`, `0xA01`, `0xA37`, `0xB50`, `0xB60`, `0x12DE`,
    /// `0x16A4`, `0x1A86`, `0x1B85`, `0x1BCD`, `0x1C1C`, `0x207C`, `0x2291`,
    /// `0x23D0`): the field is set up outside this overlay and only ever
    /// followed. Both the dispatcher and this state machine skip element 0.
    ///
    /// So the port's `AlienColony { objects: Vec<AlienObject> }` is the wrong
    /// shape — an object owns a child array (`+0x16`, count `+0x1A`, stride
    /// `0x5E`), and `step` reaches into child 1. Building that is the next task;
    /// what it must satisfy is written above rather than guessed at later.
    pub fn step(&mut self, shared: &mut AlienStreams) -> bool {
        if self.timer > 0 {
            self.timer -= 1;
            self.state_flag = 0;
            return false;
        }
        // Draw from the SHARED stream (`fs:[0x105C]`, 0x16B4..0x16BE), then keep
        // what we drew. Objects de-sync because their timers expire on different
        // frames, not because they were seeded differently.
        shared.prng = alien_anim_prng_next(shared.prng);
        self.prng = shared.prng;
        self.state_flag = 1;
        self.timer = ALIEN_STATE_TIMER_RELOAD;
        // `+0x3C` takes the counter's CURRENT value (sign-extended), and only
        // THEN does the counter advance -- `mov dword [di+0x3c], ebx` @0x16D8
        // precedes `add bx,0xfa` @0x16DC. The port used to add 0xFA to its own
        // field, which is the same sequence for one object and the wrong
        // interleaving for a colony (audit-fixes #356, #401).
        self.anim = shared.anim as i16 as i32;
        shared.anim = shared.anim.wrapping_add(ALIEN_ANIM_STEP);
        // A SECOND PRNG step, NOT written back to the global, landing in `+0x42`
        // -- the field the proximity gate reads as this object's X:
        // `ror ax,7 / sbb ax,0` @0x16E5 then `mov word [di+0x42], ax` @0x16EB.
        let derived = alien_anim_prng_next(shared.prng);
        self.pos[0] = derived as i16 as i32;
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
    /// THE SHARED CELLS — `fs:[0x105C]` and `cs:[0x16A2]` (audit-fixes #400,
    /// #401). Both live outside the object record, so all objects draw from one
    /// sequence each, in the order their timers expire. The port used to give
    /// every object its own seed and its own accumulator, with a test asserting
    /// the seeds differed — which made an invention look like a decode.
    pub shared: AlienStreams,
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
    /// A colony of `count` objects sharing ONE PRNG stream — which is what
    /// `fs:[0x105C]` is. `base_seed` seeds the STREAM, not the objects.
    ///
    /// This used to seed each object as `base_seed + i * 0x9E3B` "so they
    /// de-sync". Nothing in the overlay does that; objects de-sync because their
    /// `+0x38` timers expire on different frames, and each then draws the next
    /// value from the shared stream (audit-fixes #400).
    pub fn new(count: usize, base_seed: u16) -> Self {
        Self {
            objects: (0..count).map(|_| AlienObject::new(0)).collect(),
            frame_timer: ALIEN_COLONY_FRAME_GATE,
            shared: AlienStreams::new(base_seed, 0),
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
        let shared = &mut self.shared;
        for object in &mut self.objects {
            object.dispatch(shared);
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
        let mut down = AlienCamera {
            y_fixed: -1,
            ..Default::default()
        };
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
        assert!(
            obj.proximity_visible(AlienCamera::default(), 0x3C),
            "on-screen object is visible"
        );
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
        assert_eq!(
            AlienMethod::from_vtable_offset(0x16A4),
            AlienMethod::AnimStateMachine
        );
        assert_eq!(
            AlienMethod::from_vtable_offset(0x0A30),
            AlienMethod::SubBehaviour(0x0A30)
        );
        // The null method never changes state; the anim method eventually does.
        let mut stream = AlienStreams::new(0x1, 0);
        let mut null = AlienObject::new(0x1);
        null.method = AlienMethod::Null;
        for _ in 0..100 {
            assert!(!null.dispatch(&mut stream));
        }
        let mut anim = AlienObject::new(0x1);
        let changed = (0..100).any(|_| anim.dispatch(&mut stream));
        assert!(
            changed,
            "anim-state object changes state within its timer window"
        );
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
        // ONE SHARED STREAM (`fs:[0x105C]`, audit-fixes #400). This used to assert
        // objects[0].prng != objects[1].prng, justified as "seeded distinctly so
        // they don't all change state in lockstep" -- an invention the overlay
        // does not make.
        //
        // Nothing has drawn yet: every object starts with the same 50-frame
        // timer, and three gate updates only tick it to 47.
        assert!(colony.objects.iter().all(|o| o.prng == 0), "no draws yet");
        assert_eq!(colony.shared.prng, 0x1234, "stream untouched");

        // Run until the timers expire. All three then draw on the SAME update,
        // in object order, taking CONSECUTIVE values from the one stream.
        for _ in 0..(ALIEN_COLONY_FRAME_GATE as u32 * 48) {
            colony.step();
        }
        let mut expected = 0x1234u16;
        for (i, object) in colony.objects.iter().enumerate() {
            expected = alien_anim_prng_next(expected);
            assert_eq!(
                object.prng, expected,
                "object {i} draws the next value from the shared stream"
            );
        }
        assert_eq!(
            colony.shared.prng, expected,
            "the stream ends where the last object left it"
        );
    }

    /// `+0x3C` is a DWORD store of a sign-extended WORD (`movsx ebx, word ptr
    /// cs:[0x16a2]` @0x16C2, `mov dword ptr [di+0x3c], ebx` @0x16D8), so once the
    /// 16-bit counter passes 0x7FFF the object's value is NEGATIVE. Consumers
    /// that read it as unsigned have to say so; engine.rs does.
    #[test]
    fn anim_counter_sign_extends_past_0x7fff() {
        // Seeded AT the boundary: the first draw records 0x7FFF (still the
        // largest positive i16), and only the second, at 0x7FFF + 0xFA, is
        // negative. Seeding one step lower made the second draw land exactly on
        // 0x7FFF and the test failed -- which is the boundary being off by one
        // step, not by one unit.
        let mut shared = AlienStreams::new(1, 0x7FFF);
        let mut obj = AlienObject::new(0);
        obj.timer = 0;
        assert!(obj.step(&mut shared));
        assert!(obj.anim > 0, "still positive just below the boundary");

        obj.timer = 0;
        assert!(obj.step(&mut shared));
        assert!(
            obj.anim < 0,
            "past 0x7FFF the sign-extended value is negative, not a big positive"
        );
        assert_eq!(obj.anim as u16 as u32, obj.anim as u32 & 0xFFFF);
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
        let mut stream = AlienStreams::new(0x1357, 0);
        let mut obj = AlienObject::new(0);
        assert_eq!(obj.timer, ALIEN_STATE_TIMER_RELOAD);
        // Holds for the timer window (no state change, flag stays 0).
        for _ in 0..ALIEN_STATE_TIMER_RELOAD {
            assert!(!obj.step(&mut stream));
            assert_eq!(obj.state_flag, 0);
        }
        // Timer now 0 → next step chooses a new state.
        let anim_before = obj.anim;
        assert!(obj.step(&mut stream), "state change on timer expiry");
        assert_eq!(obj.state_flag, 1);
        assert_eq!(obj.timer, ALIEN_STATE_TIMER_RELOAD);
        // `+0x3C` takes the counter's value BEFORE it advances, so the first
        // draw records 0 and the shared counter moves to 0xFA.
        assert_eq!(obj.anim, anim_before);
        assert_eq!(stream.anim, ALIEN_ANIM_STEP);
        // `+0x42` is a SECOND PRNG step that is NOT written back to the global
        // (`ror ax,7 / sbb ax,0` @0x16E5, `mov word [di+0x42], ax` @0x16EB), so
        // it is derived from the stream's current value and leaves it alone.
        assert_eq!(
            obj.pos[0],
            alien_anim_prng_next(stream.prng) as i16 as i32,
            "+0x42 is the derived second step"
        );
        assert_eq!(
            stream.prng, obj.prng,
            "the second step does NOT advance the shared stream"
        );
    }

    /// The visibility gate's bounds are IMMEDIATES in croolis.xdb's `0xA30`
    /// method, so pin them to the overlay's bytes rather than restating them
    /// (audit-fixes #488). A changed constant that no longer matches the image
    /// fails here.
    #[test]
    fn visibility_bounds_are_croolis_xdb_immediates() {
        let xdb = [
            "export_check/_tmp_dat/croolis.xdb",
            "output/_tmp_dat/croolis.xdb",
            "../export_check/_tmp_dat/croolis.xdb",
        ]
        .iter()
        .find_map(|p| std::fs::read(p).ok());
        let Some(xdb) = xdb else { return };

        // `cmp ax, 0x80` @0xA68 -- the screen-y ceiling, `3d 80 00`.
        assert_eq!(&xdb[0xA68..0xA6B], &[0x3D, 0x80, 0x00]);
        assert_eq!(
            i16::from_le_bytes([xdb[0xA69], xdb[0xA6A]]),
            super::VISIBLE_SCREEN_Y_MAX
        );
        // The y floor is `js` @0xA66 (`78 38`), NOT a compare -- which is why the
        // y window is [0, 128] while x and z are symmetric.
        assert_eq!(xdb[0xA66], 0x78, "js: negative y is rejected outright");

        // `cmp ax, 0xff00` @0xA74 then `cmp ax, 0x100` @0xA79 -- the world-x pair.
        assert_eq!(&xdb[0xA74..0xA77], &[0x3D, 0x00, 0xFF]);
        assert_eq!(&xdb[0xA79..0xA7C], &[0x3D, 0x00, 0x01]);
        assert_eq!(
            i16::from_le_bytes([xdb[0xA75], xdb[0xA76]]),
            -super::VISIBLE_WORLD_X_HALF,
            "0xFF00 read SIGNED is -256, which is what jl/jg make it"
        );
        assert_eq!(
            i16::from_le_bytes([xdb[0xA7A], xdb[0xA7B]]),
            super::VISIBLE_WORLD_X_HALF
        );
        // The Z axis @0xA85/0xA8A reuses the same two immediates.
        assert_eq!(&xdb[0xA85..0xA88], &xdb[0xA74..0xA77]);
        assert_eq!(&xdb[0xA8A..0xA8D], &xdb[0xA79..0xA7C]);
    }
}
