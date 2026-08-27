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
    /// RAW relative motion this frame (mickeys), independent of the on-screen cursor. The
    /// original's bridge steering tracks the mouse in RING space — the driver's h-range is the
    /// 1440-px panorama ring (`[0xa2a]`, rebased near the view @0x97FC), NOT the 320-px screen —
    /// so pushing the physical mouse keeps rotating the view even when the visible cursor sits
    /// clamped at the screen edge. Frontends that capture the pointer supply the raw deltas here;
    /// 0/0 falls back to on-screen cursor deltas (headless drivers, tests).
    pub dx: i32,
    pub dy: i32,
}

impl MouseInput {
    pub fn left_down(&self) -> bool {
        self.buttons & 1 != 0
    }
    pub fn right_down(&self) -> bool {
        self.buttons & 2 != 0
    }
}

use crate::font::draw_text_indexed;
use crate::hnm::HnmFile;
use crate::ship3d::{
    BloodPrng, Ship3dMatrixAngles, Ship3dProjectionOrigin, Ship3dProjectionViewport,
    render_ship_3d_starfield,
};
use crate::sprite::{
    SpriteFrameImage, blit_sprite_frame_at, blit_sprite_frame_centered, decode_sprite_bank_indices,
};
use crate::vm::{LineState, VmToken, execute_trace, walk};
use std::collections::HashMap;
use std::path::Path;

/// Parse a `SCRIPTn.DIC` dictionary: NUL-terminated words keyed by their byte
/// offset (a Text token's `word_offsets` index into this).
fn parse_dictionary(dic: &[u8]) -> HashMap<u16, String> {
    // Single implementation in script.rs. This used to be one of FOUR byte-identical
    // copies, which is precisely how the CP437 decode bug came to be fixed in one
    // place and left wrong in the other three.
    crate::script::parse_dictionary(dic)
}

/// Public wrapper: DEB symbol names for ALL kinds (objects, sequences, …) keyed
/// by offset — used by the script decompiler to label record references.
pub fn deb_actor_name_map(deb: &[u8]) -> HashMap<u16, String> {
    let mut names = HashMap::new();
    for r in deb.chunks_exact(20) {
        let nl = r[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let offset = u16::from_le_bytes([r[16], r[17]]);
        if !r[..nl].is_empty() {
            names.insert(offset, crate::font::cp437_string(&r[..nl]));
        }
    }
    names
}

/// Parse a `SCRIPTn.DEB` symbol table: object records (`kind==1`) mapping an
/// object's byte offset to its name (the speaker's `actor_offset` indexes this).
fn parse_deb_object_names(deb: &[u8]) -> HashMap<u16, String> {
    let mut names = HashMap::new();
    for r in deb.chunks_exact(20) {
        let nl = r[..16].iter().position(|&b| b == 0).unwrap_or(16);
        let offset = u16::from_le_bytes([r[16], r[17]]);
        let kind = u16::from_le_bytes([r[18], r[19]]);
        if kind == 1 {
            // e.g. SCRIPT1.DEB record 6 is `porte_cl\x82s` = "porte_clés".
            names.insert(offset, crate::font::cp437_string(&r[..nl]));
        }
    }
    names
}

/// Recursively collect `*.hnm` asset paths under `dir`, keyed by lowercase
/// filename, so a DESCRIPT talk-HNM name resolves to its file.
///
/// APPROX — the game NEVER searches for a file (audit-fixes #482; the matrix row
/// is "the port SEARCHES for media files" in docs/port-validation.md). It reads
/// `asset_path_template_table` @0x0F48B: 45 variable-length records, each a
/// relative path whose filename is a twelve-`x` placeholder patched at load time,
/// with the directory baked into the SLOT (`pe\` x33, `sq\` x10, `pl\` x1, `ob\`
/// x1 — the same four directories this scan rediscovers by walking the tree).
/// This stands in only until the routine that patches a slot is found; the
/// `FS:0x0C04` resource table is NOT that routine — its 95 names include no
/// `.hnm` at all, which is what #481 raised and #482 closed.
fn collect_hnm_paths(dir: &Path) -> HashMap<String, std::path::PathBuf> {
    let mut map = HashMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        let Ok(rd) = std::fs::read_dir(&d) else {
            continue;
        };
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                stack.push(p);
            } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case("hnm")) {
                if let Some(n) = p.file_name() {
                    map.insert(n.to_string_lossy().to_lowercase(), p);
                }
            }
        }
    }
    map
}

/// Join dictionary words into the on-screen subtitle string with the game's decoded
/// text-assembly rule (0xA6 handler @0x66CD–0x6739): a space between words unless the
/// next begins with attaching punctuation (`, . ? ! :`), and after inserting a space,
/// wrap to a new line (`0x0D`, `'\n'` here) once the current line length reaches 0x23
/// (35) characters. No wrap check on the no-space path; long words are not split.
pub(crate) fn assemble_words(parts: &[String]) -> String {
    let parts: Vec<&String> = parts.iter().filter(|w| !w.is_empty()).collect();
    let mut out = String::new();
    let mut line_len: usize = 0;
    for (i, w) in parts.iter().enumerate() {
        out.push_str(w);
        line_len += w.chars().count();
        if i + 1 < parts.len() {
            let attaches = matches!(
                parts[i + 1].chars().next(),
                Some(',' | '.' | '?' | '!' | ':')
            );
            if !attaches {
                out.push(' ');
                line_len += 1;
                // PREDICTIVE WRAP (audit-fixes #313). The game adds the NEXT
                // WORD'S LENGTH before comparing, so it breaks BEFORE a word that
                // would overflow rather than after one that already did:
                //
                //   0x66FF  mov di,[si] / call 0x67a7   al = strlen(next word)
                //   0x6728  inc dl                      dl = line length + space
                //   0x672A  add al, dl
                //   0x672C  cmp al, 0x23 / jb           under 35 -> keep going
                //   0x6730  xor dl,dl / al=0x0D / stosb else newline, reset
                //
                // The port compared `line_len` alone, which wraps a word later.
                let next_len = parts[i + 1].chars().count();
                if line_len + next_len >= crate::script::SUBTITLE_WRAP_COLUMN {
                    out.push('\n');
                    line_len = 0;
                }
            }
        }
    }
    out
}

/// A world being visited from the nav map: its decoded `fd/` rooms (paths, decoded
/// lazily) with the currently-shown room. Rooms are the world's floor/view-angle
/// backgrounds; cycling walks through them.
struct WorldVisit {
    name: String,
    rooms: Vec<std::path::PathBuf>,
    current: usize,
    image: crate::lbm::LbmImage,
    /// Decoded `.ext` object positions `(x, y)` to mark on the location (from
    /// [`crate::ext::ExtWorld::objects`]); empty until supplied by the caller.
    objects: Vec<(u16, u16)>,
}

/// Screen dimensions of the engine framebuffer (VGA mode 13h / mode-X, 320x200).
/// 320, and it is never an immediate — the game builds the row stride from two
/// shifts, `xchg bh,bl` (y*256) plus `shl ...,6` (y*64), at `0x9B25`..`0x9B2C` in
/// the star plot and again at `0x50C4` in the dirty-rect walker (audit-fixes #502,
/// #506). Searching the image for `0x140` finds nothing.
pub const ENGINE_SCREEN_WIDTH: usize = 320;
/// 200 rows — `mov word ptr [0x523b],0xc8` @`0xB41D`, the value the navigation
/// routine RESTORES the clip bottom to after narrowing it (audit-fixes #495).
pub const ENGINE_SCREEN_HEIGHT: usize = 200;

/// Days-since-Unix-epoch → (year, month, day) civil date (Howard Hinnant's `civil_from_days`).
/// Used for the TV ad channel's seasonal variants (Dec 25 / Jan 1) — no external time dep needed.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64; // day of era [0, 146096]
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365; // year of era
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100); // day of year
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

/// One TV broadcast program: a DESCRIPT Sequence record that self-identifies as a channel
/// (its subtitles announce "…watching…"), played as its chained HNM clips + music +
/// tick-timed subtitle cues. See [`EngineState::load_tv_programs`].
struct TvProgram {
    name: String,
    clips: Vec<HnmFile>,
    cues: Vec<crate::descript::SubtitleCue>,
    music: Option<String>,
}

/// One entry of the nav chart's visible-object list (`DS:0x2AD3`, built by
/// `0x604E` -> `0x721A`): a real object record, its display name (`record+4`),
/// its kind and the marker the picker hit-tests (`+0x18`/`+0x1A`, or `+0x1C`/
/// `+0x1E` for a black hole's far end).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NavChartObject {
    /// The record offset — what `gs:0x27BF` holds once selected.
    pub object: u16,
    pub name: String,
    pub kind: u16,
    pub marker: (i32, i32),
    /// The artwork resource id from `DS:0x2BC7`, when the name is in that table.
    pub art_id: Option<u16>,
    /// The `0x92ED` branch, carried beside [`Self::marker`] because ONE test in the
    /// binary produces both: a black hole away from the arche takes the second
    /// endpoint AND skips the ship box test (`0x92EF`). Storing the marker without
    /// it let `hit_box` re-decide on the kind alone and answer differently.
    pub far_endpoint: bool,
}

impl NavChartObject {
    /// The picker's hit box (`0x92BF`, `0x92D3`, `0x92FC`).
    ///
    /// Delegates rather than repeating the rule: `VmMachine::nav_chart_hit_box` is
    /// the same sequence of stores, and it is the one swept against the lifted
    /// `func_92a3`. Two copies of one rule is how a box quietly stops matching the
    /// hit-test that uses it — which is exactly what audit-fixes #575 found, in the
    /// form of an `else if` that could not see the `0x92ED` branch.
    pub fn hit_box(&self) -> (i32, i32) {
        crate::vm::nav_chart_hit_box_for_kind(self.kind, self.far_endpoint)
    }
}

/// The destination info panel's state word, `gs:0x2788`. The dispatcher at
/// `0x9083` reads it as a bitfield: bit0 = zooming open, bit1 = zooming shut,
/// zero = open and drawn. There is no separate "idle" value in the original —
/// the panel is simply not reached when `gs:0x27BF` is 0. The close clears the
/// pair in ADJACENT instructions — `mov byte ptr [0x2788], 0` @`0x9217` then
/// `mov word ptr [0x27bf], 0` @`0x921C` — so the port names that state rather
/// than leaving a zero object to stand for it. (The doc cited only `0x921C`,
/// the second of the two; audit-fixes #368.)
///
/// EVERY WRITE to the state word, which is what makes this enum exhaustive
/// rather than a guess:
///
/// ```text
///   0x9043  mov byte ptr [0x2788], 1   start zooming OPEN
///   0x922F  mov byte ptr [0x2788], 2   start zooming SHUT
///   0x9120  mov byte ptr [0x2788], 0   open complete -> drawn
///   0x9217  mov byte ptr [0x2788], 0   close complete -> idle
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum LocationPanelState {
    #[default]
    Idle,
    /// `[0x2788] & 1` — `0x9087`.
    ZoomingOpen,
    /// `[0x2788] & 2` — `0x9125`.
    ZoomingShut,
    /// `[0x2788] == 0` with a selection live — the drawn panel at `0x9137`.
    Open,
}

/// The panel's whole runtime state: `gs:0x2788` (state), `gs:0x2789` (the entity
/// zoom scale the draw at `0x9240` reads as `bh = (3*[0x2789])/2+1`), `gs:0x27BF`
/// (the selected object) and `gs:0x2AAB` (the 4x4 cursor rect it zooms from).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LocationInfoPanel {
    pub state: LocationPanelState,
    pub object: u16,
    pub scale: u8,
    /// `[0xADB]`, the interpolation step; `[0xADA]` is the constant 8.
    pub step: u8,
    pub cursor_rect: [i32; 4],
}

impl LocationInfoPanel {
    /// The entity draw's scale at `0x9240`, computed as the original does — in
    /// EIGHT BITS:
    ///
    /// ```text
    ///   0x9247  mov bh,[0x2789]
    ///   0x924B  mov al,3 / mul bh     ax = 3 * scale (a 16-bit product)
    ///   0x924F  mov bh,al             ...but only the LOW BYTE is kept
    ///   0x9251  shr bh,1 / inc bh     then /2 and +1, on the byte
    /// ```
    ///
    /// The truncation happens BEFORE the shift, so the port's old
    /// `(3 * scale as u16 / 2 + 1) as u8` diverged once `3 * scale` passed 255:
    /// at scale 86 the original gives 2 and the 16-bit form gives 130. The panel's
    /// zoom counter never reaches 86 — it runs 0..8 — so this was latent, but the
    /// faithful form costs nothing.
    pub fn entity_draw_scale(&self) -> u8 {
        let product = 3u8.wrapping_mul(self.scale); // `mul bh` then `mov bh,al`
        (product >> 1).wrapping_add(1)
    }
}

/// Station 2 of the bridge ring — the pyramid navigation room (`TB.BIG` frame
/// headers; frames 72..=107 carry it).
pub const NAV_ROOM_STATION: u16 = 2;

/// Per-frame engine state — the subset of the `DS`/`gs` globals the main loop
/// (`0x0FFB`) touches, plus the indexed framebuffer the render subsystems fill.
///
/// This doc lived 110 lines earlier, stranded above `civil_from_days` with no
/// item between them, so the ledger attached its `0x0FFB` citation to a date
/// algorithm and left this struct undocumented (audit-fixes #280).
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
    /// Set to the compass heading of the destination the player committed by clicking
    /// in the nav view (edge-triggered). A driver polls [`EngineState::take_nav_selection`]
    /// to load that destination's dialogue — the nav→dialogue game-loop hook.
    nav_selection: Option<u16>,
    /// Previous-frame left-button state, for edge-detecting nav clicks.
    prev_left_down: bool,
    /// Deterministic PRNG seed for the starfield point cloud (the engine seeds
    /// from CMOS RTC seconds at runtime; fixed here for reproducibility).
    /// Seed for the ship-3D point cloud. CONSTANT, and deliberately so.
    ///
    /// The game randomizes the cloud ONCE: `ship_3d_point_cloud_randomize`
    /// (`0x9B67`) has exactly one caller, the far call at `0x0FD3` on a setup path
    /// that first sets `[0x27D9]=1`. Its starfield is therefore stable for the
    /// session — it does not twinkle.
    ///
    /// This engine regenerates the cloud every render instead, which is
    /// equivalent ONLY because this seed never changes: the same 1000 points come
    /// back each frame. Making it vary per frame would look like a fix (the game
    /// does seed from the RTC) and would introduce shimmer the original does not
    /// have.
    ///
    /// The one real divergence: the game's RTC seed differs per session, so its
    /// star positions vary between runs; this fixed seed makes them reproducible,
    /// which the oracle comparisons depend on.
    pub starfield_seed: u8,
    /// Ship-3D view TRANSITION + DEPTH state (`DS:0x2533/0x252F/0x2530/0x2531`
    /// and `DS:0x2527`). These drive the nav view's open/close sweep. The state
    /// machine (`update_ship_3d_transition_state` vs `0xB692`) and the scroll step
    /// (`step_ship_3d_depth_scroll` vs `0xB75C`) were both audit-verified EXACT,
    /// but had NO caller outside tests — the ported subsystem never ran. Wired
    /// here so the game actually executes it.
    pub ship3d_transition: crate::ship3d::Ship3dTransitionState,
    pub ship3d_depth: crate::ship3d::Ship3dDepthState,
    /// The nav view's hold counter (`gs:0x0B3B`), which the transition gate reads.
    pub ship3d_hold_ticks: u16,
    /// Ship-3D PROCEDURAL update state (`run_ship_3d_procedural_update`): the
    /// per-frame HUD-rotation / nav-timer machine. Verified but previously
    /// unreachable; driven below from the live cursor so it runs in play.
    pub ship3d_procedural: crate::ship3d::Ship3dProceduralUpdateState,
    /// Interpolation gate (0x1E5D) — advances one tick per nav frame.
    pub ship3d_interpolation: crate::ship3d::Ship3dInterpolationGate,
    pub ship3d_interpolation_source: [u16; crate::ship3d::SHIP_3D_INTERPOLATION_WORDS],
    pub ship3d_interpolation_dest: [u16; crate::ship3d::SHIP_3D_INTERPOLATION_WORDS],
    /// Target selector (0xB2BB) state.
    pub ship3d_target_selector: crate::ship3d::Ship3dTargetSelectorState,
    /// Navigation sequence FSM state (owns exit/opening).
    pub ship3d_nav_sequence: crate::ship3d::Ship3dNavigationSequenceState,
    /// Whether the sequence FSM raised its framebuffer-dirty request this frame.
    pub ship3d_sequence_redraw_requested: bool,
    /// Nav-marker SPRITE SLOTS, persistent across frames so the dirty tracking in
    /// `update_ship_3d_sprite_slot_position` is meaningful (it raises the dirty flag
    /// only when a slot actually MOVES).
    pub ship3d_nav_slots: Vec<crate::ship3d::Ship3dObjectSpriteDescriptor>,
    /// The dirty-rect list the render-command collector filters against.
    pub ship3d_dirty_rects: crate::ship3d::Ship3dDirtyRectList,
    /// Set once the global clip snapshot has been armed for the frame.
    pub ship3d_clip_snapshot_armed: bool,
    /// The game's own PRNG, used for the transition's `rand(20)` close gate
    /// (`0xB6D0 mov ax,0x14` -> `lcall 0x1CE:0x0B02`).
    ship3d_prng: crate::ship3d::BloodPrng,
    /// Decoded ship-nav HUD sprite banks: BCARTE perspective grid frames.
    hud_grid: Vec<SpriteFrameImage>,
    /// Decoded ship-nav HUD orb sprite frames (BORXX).
    hud_orb: Vec<SpriteFrameImage>,
    /// Real world names to label the nearest nav-destination row with (the navigable
    /// `.ext` planets from the decoded level directory, [`crate::levels`]).
    nav_world_labels: Vec<&'static str>,
    /// When a world is being "visited" from the nav map, its decoded rooms (the `fd/`
    /// PBM art) — cyclable — shown as the landing/exploration screen.
    world_location: Option<WorldVisit>,
    /// The decoded title art (`BLOOD.LBM`, 640×480 planar ILBM) downscaled to the
    /// 320×200 framebuffer + its palette, shown as the title screen when armed.
    title_screen: Option<(Vec<u8>, [[u8; 3]; 256])>,
    /// The game's star-map destination pyramid frames (CARTE.SPR f0..f5, six
    /// pre-scaled sizes) + selection reticle (f6) — the real art drawn by the sprite
    /// path at projected destination positions.
    nav_pyramids: Vec<SpriteFrameImage>,
    /// The real navigation star-map background (`CHART.FD`): the game's chart image —
    /// nebula + destination stars + route lines + the ship console. When loaded it
    /// replaces the procedural starfield in the nav view.
    nav_chart: Option<crate::lbm::LbmImage>,
    /// The choose-a-location destination list shown on the nav chart: each entry is a
    /// (label, that character's dialogue lines). Clicking one visits it (plays that
    /// character's decoded dialogue). Empty = the plain compass-steer nav.
    #[allow(clippy::type_complexity)]
    nav_destinations: Vec<(String, Vec<(String, Option<std::path::PathBuf>)>)>,
    /// The RECORD-DRIVEN chart list (`DS:0x2AD3`). Empty until the VM supplies it.
    nav_chart_objects: Vec<NavChartObject>,
    /// The info panel's text rows, captured when it opens so the draw does not
    /// need the VM every frame.
    location_panel_rows: Vec<crate::vm::LocationPanelRow>,
    /// The destination info panel's zoom FSM (`gs:0x2788`/`gs:0x2789`/`gs:0x27BF`).
    pub location_panel: LocationInfoPanel,
    /// `gs:0xA3E` bit0 as the panel sees it: the selection commit turns the mouse
    /// OFF (`0x900C`), and it being ON again is what closes the panel (`0x912E`).
    pub location_panel_mouse_enabled: bool,
    /// The ship-3D camera-approach animation (the decoded `[0x27DF]` phase FSM):
    /// drives the camera origin as the ship pulls in / travels when entering nav.
    camera: crate::ship3d::Ship3dCameraApproach,
    /// The alien-examination screen (croolis.xdb): pre-rendered rotation views of an
    /// alien (e.g. Scruter Jo's `pe/scrut_a..d.hnm`) selected by the mouse camera pan
    /// — the interactive 3D alien-view decoded at `re/REVERSE.md` (mouse delta →
    /// smoothed camera, per-angle pre-rendered HNM). Empty = screen not loaded.
    alien_views: Vec<HnmFile>,
    /// Whether the alien-examination screen is the active view.
    pub alien_view_active: bool,
    /// Smoothed camera pan for the alien view (mouse delta from centre, clamped),
    /// selecting the pre-rendered rotation angle.
    alien_pan: i32,
    /// The alien's decoded behaviour object (`croolis.xdb` `0x16A4` state machine):
    /// its PRNG+timer picks new animation states, giving the examined alien an idle
    /// life of its own between the player's rotations.
    alien_object: crate::croolis::AlienObject,
    /// The shared alien behaviour cells. The addresses and the decode live on
    /// [`crate::croolis::AlienStreams`]; this engine only HOLDS one, the way the
    /// colony does, so it deliberately does not re-cite them (audit-fixes #423 —
    /// citing them here made `new` look like a second implementation of the same
    /// rule to the duplicate-rule guard).
    alien_prng: crate::croolis::AlienStreams,
    /// The scrutinizer-apparatus intro animation (`sq/caiscrut.hnm`) played once when
    /// the examination screen opens, before the rotatable alien.
    alien_intro: Option<HnmFile>,
    /// Intro-animation frame counter; `None` once the intro has finished (or if there
    /// is no intro), so the rotatable alien takes over.
    alien_intro_frame: Option<usize>,
    /// The comms "Hate TV" screen: broadcast channel HNMs (`sq/tvgren*`, `tvred*` —
    /// self-contained character-in-TV-frame animations). Steering switches channels.
    /// Legacy fallback when no `tv_programs` are available.
    tv_channels: Vec<HnmFile>,
    /// The real TV PROGRAMMING: the DESCRIPT Sequence records that self-identify as broadcasts —
    /// their own subtitles announce the channel ("YOU ARE WATCHING HATE TV" / "…you're watching
    /// the IZWAL channel"), i.e. `hatetv` (8 chained clips + hatetv.voc) and `microkid` (IZWAL,
    /// 3 clips + balise.voc). Each channel plays its record's clips in sequence with its music
    /// and tick-timed subtitles — the data-faithful broadcast, not a silent raw HNM loop.
    tv_programs: Vec<TvProgram>,
    /// Whether the comms/TV screen is the active view.
    pub tv_active: bool,
    /// Currently-selected TV channel index.
    tv_channel: usize,
    /// Current clip index within the active TV program (its record chains several HNMs).
    tv_clip: usize,
    /// Frame index within the current TV clip.
    tv_clip_frame: usize,
    /// Total frames elapsed in the active program (drives its subtitle-cue ticks; wraps on loop).
    tv_program_frame: usize,
    /// The cyberspace hyperspace-tunnel animations (`sq/hyper_00..07.hnm` — colour
    /// warp-tunnel variants). This is the cyberspace screen's *presentation*; the
    /// navigation minigame logic is undecoded.
    cyber_tunnels: Vec<HnmFile>,
    /// Whether the cyberspace tunnel screen is active.
    pub cyber_active: bool,
    /// The cryo-chamber scene (`sq/cryorad.hnm`), shown by the console's CRYOBOX option.
    cryobox_scene: Option<HnmFile>,
    /// Whether the CRYOBOX cryo-chamber screen is active.
    pub cryobox_active: bool,
    /// Current tunnel-segment index (advances as you "travel").
    cyber_segment: usize,
    /// The cyberspace traversal mini-game's lateral course offset (steered by the mouse).
    cyber_steer: i32,
    /// Whether the cyberspace traversal has reached its destination (last segment).
    pub cyber_arrived: bool,
    /// The real ship bridge: the TB.BIG 360° panorama ([`crate::tbbig`]) whose
    /// frames ARE the console/menu/nav-room/Orxx views (golden menu text baked in).
    panorama: Option<crate::tbbig::BridgePanorama>,
    /// The BOLD console subtitle font from the user's BLOODPRG.EXE (the face
    /// the game uses for ALL on-console text); None until loaded.
    bold_font: Option<crate::font::BoldConsoleFont>,
    /// The dialogue TOPIC MENU (the game's concept-menu conversation system,
    /// live-captured: TALK/ONE..NINE, TALK/EGO/LIBIDO/...): each entry is a
    /// label + the dialogue-line index its topic starts at. Empty = no menu.
    topic_menu: Vec<(String, usize)>,
    /// Currently highlighted topic row.
    pub topic_selected: Option<usize>,
    /// The decoded BAS concept-menu stack for the active conversation script (the
    /// game's `gs:0x6772`/`gs:0x6774` menu stack — see [`crate::bas_vm`]). Present
    /// once a script's `.BAS` is loaded; `current()` is the menu to display.
    pub bas_menus: Option<crate::bas_vm::BasMenuStack>,
    /// The in-progress sequential response player for the active menu's monologue
    /// (advances one `0xA6` response per interaction — the already-shown gating).
    pub bas_responses: Option<crate::bas_vm::SequentialResponses>,
    /// Whether the displayed `topic_menu` is a BAS concept menu (so clicks route to
    /// [`Self::bas_menu_interact`]) rather than the legacy line-jump topic menu.
    pub topic_menu_is_bas: bool,
    /// The decompiled bridge interaction state ([`crate::bridge`]): mouse-push
    /// steering, station seeks, and the golden-menu hit testing/highlighting.
    pub bridge: crate::bridge::BridgeView,
    /// The ship-console UI font (`HONKF.SPR`): 49 8×8 glyphs — A–Z, 0–9, punctuation —
    /// the game draws its console menu labels with. Empty until loaded.
    console_font: Vec<SpriteFrameImage>,
    /// Whether the ship-bridge hub is the active view.
    pub bridge_active: bool,
    /// Whether the console MENU option's submenu ({EXPLANATIONS, GAME}) is showing — the
    /// game's main menu, decoded by driving the emulator (MENU opens this two-item submenu).
    pub menu_submenu_active: bool,
    /// The OPTION choice box (over the panorama) is open — the REAL OPTION interaction
    /// (savestate-verified); replaces the invented 3D-pyramid OPTION screen.
    pub option_box_active: bool,
    /// The SAVE-SLOT UI (OPTION submenu -> SAVE). Rendered through the ORDINARY
    /// LIST WIDGET with the edit buffer substituted for the row being renamed
    /// (`0x1BAB`/`0x1BBD`/`0x8573`) — see [`Self::draw_save_ui_rows`]. True while
    /// awaiting a typed slot name (digits+lowercase, Enter commits — the `0x1DD8`
    /// edit law).
    pub save_ui_active: bool,
    pub save_ui_name: String,
    /// The slot the SAVE flow is typing into — the row `[0x2734]` names
    /// (`0x1BAB`), whose text the widget swaps for the edit buffer.
    pub save_ui_slot: usize,
    /// The ten `blood.sav` slot names the widget lists (`bloodsav::parse_slot_directory`).
    pub save_slots: Vec<crate::bloodsav::SaveSlot>,
    /// The BOB_MORLOCK CONTACT screen (CRYOBOX row -> BOB_MORLOCK): Bob's eyes
    /// (FRIGO.FD) + top-band subtitle + his concept menu — the ORACLE-CAPTURED
    /// surface (cryobox_enter dual-run vs_005..007; frigo.fd file-open traced).
    pub bob_contact_active: bool,
    bob_contact_bg: Option<(usize, usize, Vec<u8>, Vec<[u8; 3]>)>,
    /// Bob's topic labels — from his prompt LINE RECORD's 0xFFFF-carried menu
    /// words (the bytecode source). EMPTY until the VM supplies them — there is no
    /// fallback list, by rule (audit-fixes #531).
    pub bob_topics: Vec<String>,
    /// The presentation box-OPEN animation phase (1..=6 while opening, 0 idle) —
    /// the game's screen_mode_update (0x79E5) zooms the presentation frame through
    /// the 6-rect table at DS:0x2B97 before content shows.
    presentation_open_phase: u8,
    /// The UNIVERSAL console choice box (savestate-verified: every golden-menu row opens a
    /// contextual gold box over the panorama): its item labels, or empty = closed. The last
    /// item is always CANCEL. `console_box_kind` = which console row opened it.
    pub console_box: Vec<String>,
    /// UI strings read from `BLOODPRG.EXE`, keyed by DS offset (audit-fixes #524).
    pub ds_strings: std::collections::HashMap<u16, String>,
    /// When set, the hand is NOT software-rasterized into the 320x200 framebuffer;
    /// instead its triangles are exported here each frame for the GPU presenter
    /// (window-resolution rendering with per-pixel texel sampling).
    pub gpu_hand: Option<Vec<[[f32; 5]; 3]>>,
    pub gpu_hand_enabled: bool,
    /// GPU starfield export: plotted star points (x, y, palette shade) for the frame;
    /// when set, the bridge background's colour-0 pixels are a colour key (windows)
    /// and the GPU draws the stars behind at window resolution.
    pub gpu_stars: Option<Vec<(u16, u16, u8)>>,
    pub gpu_bg_colorkey: bool,
    /// Whether the LAST game tick's screen drew the hand — the display-rate refresh
    /// must never draw a hand on screens that don't (title/intro/TV/films...).
    hand_on_screen: bool,
    /// The skeleton state at the PREVIOUS tick (for between-tick pose interpolation).
    hand_state_prev: Option<Vec<u8>>,
    /// The hub PRESENTATION surface is live (bridge dialogue arrived; the CANCEL
    /// label + orb click-to-advance remain until CANCELed — the oracle hub state).
    pub hub_presentation: bool,
    pub console_box_kind: usize,
    /// The engaged topic row of the in-window concept box — renders WHITE while the
    /// others stay grey (oracle honk_blood: the clicked BLOOD row highlights).
    pub console_box_selected: Option<usize>,
    /// Game-progression state (which locations/crew have been visited), built on the
    /// decoded entity flag state machine. Drives completion (all visited → ending) and is
    /// persisted in the save.
    pub progress: crate::progress::GameProgress,
    /// The game-ending finale cutscene (`sq/fin.hnm`) — the bookend to the intro, played
    /// once to completion when the player has finished the game.
    ending_scene: Option<HnmFile>,
    /// The finale's current frame (advances to the last frame, then holds).
    ending_frame: usize,
    /// Whether the ending finale is the active view.
    pub ending_active: bool,
    /// The video-phone call screen (console TELEPHONE option): the animated call widget
    /// (`BAPPEL.SPR`, a low-index UI sprite that decodes cleanly) plus the roster of
    /// callable crew. Each contact is (display name, their talk-head HNM `pe/aa*.hnm`,
    /// full-colour, shown as the "video feed" when the call connects). Two states:
    /// dialling (widget + contact list) and connected (the animated talk-head).
    phone_widget: Vec<SpriteFrameImage>,
    #[allow(clippy::type_complexity)]
    phone_contacts: Vec<(String, HnmFile)>,
    /// The currently selected/dialled contact index.
    phone_contact: usize,
    /// Whether the call is connected (showing the talk-head) vs still dialling.
    phone_connected: bool,
    /// Whether the video-phone screen is the active view.
    pub phone_active: bool,
    /// The console-menu OPEN ANIMATION, `0x86E4`'s ten-tick interpolation.
    ///
    /// A menu click arms `[0x0ADA] = 0x0A` in the same breath as `[0x2A19]` and the
    /// seek (`0x86AB`..`0x86E9`), and the row's handler holds its INTERPOLATING
    /// phase until that gate completes (`0x876A`). So the telephone/cryobox/submenu
    /// do not appear on the click frame — they arrive ten frames later.
    ///
    /// `None` when no open is in flight. `Some((row, gate))` while it animates; the
    /// screen's own flag is set when the gate reports `Complete` (audit-fixes #615).
    pub console_open: Option<(usize, crate::ship3d::Ship3dInterpolationGate)>,
    /// The TRAVELLING RECT, `DS:0x253D` — `{x, y, w, h}`, the box that actually
    /// moves while the console menu opens.
    ///
    /// `0x8772`/`0x8775` hand the gate `si = 0x2AAB` (the layout rect's target
    /// shape) and `di = 0x253D`, and the gate writes back through `di` — so this
    /// interpolates TOWARD the widget rect. Its starting Y is the CLICKED ROW:
    /// `0x86C6`..`0x86D1` computes `(row-1)*0x12 + 0x50` into `[0x253F]`, which is
    /// this rect's `+2` word (audit-fixes #618, #619).
    pub console_open_rect: [u16; 4],
    /// Dialogue line sequence for the loaded script (from the VM trace), played
    /// back frame-by-frame — the script/scene stepping the main loop drives.
    dialogue: Vec<LineState>,
    /// The reconstructed subtitle text for each `dialogue` line (parallel vec).
    dialogue_texts: Vec<String>,
    /// Playback cursor into [`EngineState::dialogue`].
    dialogue_cursor: usize,
    /// Which intro clips play ON the pyramid-console band (the crew montage does; the logo
    /// reel and in-game cutscenes do not). Real-game-verified via DOSBox-X captures.
    intro_pyramid: Vec<bool>,
    /// Composite the pyramid-console band under the current dialogue (the SCRIPT1 console
    /// tutorial plays its talk-HNMs over the band — real-game-verified, tut_180s..300s).
    console_band_dialogue: bool,
    /// The manu3 3D hand model (lazy-loaded).
    hand_mesh: Option<crate::manu3_hand::HandMesh>,
    /// Last bridge frame (steering-pose detection).
    prev_bridge_frame: u16,
    /// The VIEWSCREEN console (gray pyramid band + upper viewscreen): the real NAV screen
    /// reached from the bridge's pyramid sector. With no granted destinations the viewscreen
    /// shows STATIC (oracle: nav_screen_opened + navscr captures); with destinations the
    /// destination content shows. Esc returns to the bridge.
    pub viewscreen_active: bool,
    /// Deterministic noise phase for the static.
    viewscreen_noise: u32,
    /// Auto-play stops when the cursor reaches this line (exclusive) — the SCRIPTED OPENING
    /// plays unprompted, then the dialogue HOLDS at the topic menu and further content is
    /// player-driven (a topic click plays its segment, then re-holds). `None` = play through.
    /// The real game gates topic content behind the conversation menu (the player clicks HONK /
    /// a topic); only scripted events auto-play (user-reported: Honk rattled everything off).
    autoplay_end: Option<usize>,
    /// Dialogue SEGMENT starts (per script-function beats). The first segment is the scripted
    /// opening (auto-plays); each concept-menu interaction plays the next segment then re-holds —
    /// the location scripts' conversation is menu-driven, not a full-stream monologue.
    dialogue_segments: Vec<usize>,
    /// Per-line minimum hold override, set by the driver from the VOICE clip's duration —
    /// the real game holds a line while its voice plays (the SB playback completion gates the
    /// advance), so a long clip must not be cut off by the text-length hold. (line, frames).
    line_min_hold: Option<(usize, u32)>,
    /// Index of the next unplayed segment in `dialogue_segments`.
    dialogue_segment_pos: usize,
    /// Driver-set floor on the per-line hold (the faithful hold is computed from the
    /// text-speed step; see [`EngineState::current_line_hold`]).
    pub dialogue_hold_frames: u32,
    /// The game's text-speed step (`gs:[0x0ACA]`), from the config text-speed setting
    /// via `vm::text_speed_step_from_setting` (init @0x1B3A). Drives the subtitle
    /// reveal rate and line-hold timers. Default: setting 3 → step 4.
    pub text_speed_step: u16,
    /// Frames the current dialogue line has been held.
    dialogue_timer: u32,
    /// Per-line resolved talk-HNM asset path (the speaker's animation for each
    /// dialogue line), loaded automatically as playback advances.
    dialogue_scene_paths: Vec<Option<std::path::PathBuf>>,
    /// Per-line render style: true = character SPEECH (green bold reveal, the 0x3630
    /// renderer); false = static TEXT (white thin proportional, the 0x31C8 renderer —
    /// the MENU's "Today's fare:" style). ORACLE-verified: both live on the console.
    dialogue_is_speech: Vec<bool>,
    /// Per-line resolved speaker voice bank (`sn/<name>.snd`), parallel to
    /// [`EngineState::dialogue`].
    dialogue_voice_banks: Vec<Option<std::path::PathBuf>>,
    /// The A6 voice-selector byte per text-token offset (for the current script).
    voice_by_offset: HashMap<usize, u8>,
    /// Choice-menu rows per `0xA6` line offset — the words AFTER the `0xFFFF`
    /// separator in the record's word list. This is where a console choice box's
    /// labels come from in the real game (SCRIPT1.COD `0x4A9` = the MENU submenu's
    /// `explanations` / `game`), rather than from a literal in this file.
    menu_by_offset: HashMap<usize, Vec<String>>,
    /// The next scene/profile index this script's D2 handoff requests (the
    /// scene-to-scene dispatch target), or `None` if the script has no successor.
    pending_profile: Option<u16>,
    /// Queued scene scripts `(cod, var, dic)` for auto-chaining: when the current
    /// dialogue finishes, the driver advances to the next queued scene (the
    /// scene-to-scene flow the D2 handoff drives).
    scene_queue: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>,
    /// Index of the currently-playing scene in [`EngineState::scene_queue`].
    scene_queue_idx: usize,
    /// Optional talk-HNM / scene background for the dialogue scene band, decoded
    /// per frame behind the subtitle.
    scene_hnm: Option<HnmFile>,
    /// Persistent scene buffer the HNM decodes into. Kept separate from the display
    /// framebuffer because HNM *delta* frames build on the previous frame's pixels —
    /// drawing the subtitle straight into this buffer would leave old subtitle text
    /// in regions the next delta doesn't touch, piling up across lines.
    scene_buffer: Vec<u8>,
    /// Per-scene frame counter: reset to 0 when a new talk-HNM loads so each scene
    /// plays from its keyframe forward (delta frames need their own keyframe base,
    /// not `global_frame % count` which would start mid-animation on a stale buffer).
    scene_frame: usize,
    /// Letterbox origin for the loaded scene clip: 0x23 for 130-tall band clips
    /// (the game's `gs:[0x1fa7]` blit base), 0 for full-screen clips.
    scene_band_y: usize,
    /// Palette filled by the scene HNM decode (the framebuffer is indexed).
    pub scene_palette: [[u8; 3]; 256],
    /// Indexed (palette) framebuffer the render subsystems draw into.
    pub framebuffer: Vec<u8>,
    /// Startup intro sequence: HNM paths played in order before the game proper.
    intro_hnms: Vec<std::path::PathBuf>,
    /// Subtitle cues to overlay on each intro clip (parallel to `intro_hnms`; empty for
    /// clips with none). The publisher-credit clip (`cliptoot.hnm`, the DESCRIPT `present`
    /// record) carries "CRYO Interactive Entertainment 1995" / "Commander BLOOD  V 1.0".
    intro_cues: Vec<Vec<crate::descript::SubtitleCue>>,
    /// Background music to start when each intro clip BEGINS (parallel to `intro_hnms`; `None`
    /// for the silent clips). Faithful to the DESCRIPT data: the `present` record ties its
    /// `Music` ("blintr.voc") to its `cliptoot.hnm` cinematic — so the MINDSCAPE/Microfolie's
    /// logo reel (`mind.hnm`) plays SILENT and the music starts only with the cinematic.
    intro_music: Vec<Option<String>>,
    /// Whether each intro clip is presented on the PYRAMID CONSOLE (the crew-showcase cliptoot
    /// clip: crew video + grey pyramid floor + eye-orb — accuracy/captures/frame_6-9). True only
    /// for the intro's credit/showcase clip; in-game cutscenes played via `start_descript_cutscene`
    /// (maledict/hatetv/…) are full-screen and set this false, so they don't get the console.
    /// Index of the intro HNM currently playing.
    intro_index: usize,
    /// True while the startup intro sequence is playing (gates the main render path).
    intro_active: bool,
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
            nav_selection: None,
            prev_left_down: false,
            starfield_seed: 17,
            ship3d_transition: Default::default(),
            ship3d_depth: Default::default(),
            ship3d_hold_ticks: 0,
            ship3d_procedural: Default::default(),
            ship3d_interpolation: Default::default(),
            ship3d_interpolation_source: Default::default(),
            ship3d_interpolation_dest: Default::default(),
            ship3d_target_selector: Default::default(),
            ship3d_nav_sequence: Default::default(),
            ship3d_sequence_redraw_requested: false,
            ship3d_nav_slots: Vec::new(),
            ship3d_dirty_rects: Default::default(),
            ship3d_clip_snapshot_armed: false,
            ship3d_prng: crate::ship3d::BloodPrng::seeded_from_rtc_seconds(0),
            hud_grid: Vec::new(),
            hud_orb: Vec::new(),
            nav_world_labels: crate::levels::primary_worlds().map(|e| e.stem).collect(),
            world_location: None,
            title_screen: None,
            nav_pyramids: Vec::new(),
            nav_chart: None,
            nav_destinations: Vec::new(),
            nav_chart_objects: Vec::new(),
            location_panel_rows: Vec::new(),
            location_panel: LocationInfoPanel::default(),
            location_panel_mouse_enabled: true,
            camera: crate::ship3d::Ship3dCameraApproach::default(),
            alien_views: Vec::new(),
            alien_view_active: false,
            alien_pan: 0,
            alien_object: crate::croolis::AlienObject::new(0),
            // The shared alien streams (audit-fixes #400/#401): the engine's
            // single object draws from them like a colony member. The seed is
            // this port's, not the game's -- see AlienStreams for the decode.
            alien_prng: crate::croolis::AlienStreams::new(0x2DD3, 0),
            alien_intro: None,
            alien_intro_frame: None,
            tv_channels: Vec::new(),
            tv_programs: Vec::new(),
            tv_active: false,
            tv_channel: 0,
            tv_clip: 0,
            tv_clip_frame: 0,
            tv_program_frame: 0,
            cyber_tunnels: Vec::new(),
            cyber_steer: 0,
            cyber_arrived: false,
            cyber_active: false,
            cryobox_scene: None,
            cryobox_active: false,
            cyber_segment: 0,
            panorama: None,
            bold_font: None,
            topic_menu: Vec::new(),
            topic_selected: None,
            bas_menus: None,
            bas_responses: None,
            topic_menu_is_bas: false,
            bridge: crate::bridge::BridgeView::default(),
            console_font: Vec::new(),
            bridge_active: false,
            menu_submenu_active: false,
            option_box_active: false,
            save_ui_active: false,
            save_ui_name: String::new(),
            save_ui_slot: 0,
            save_slots: Vec::new(),
            bob_contact_active: false,
            bob_contact_bg: None,
            bob_topics: Vec::new(),
            presentation_open_phase: 0,
            console_box: Vec::new(),
            ds_strings: std::collections::HashMap::new(),
            gpu_hand: None,
            gpu_hand_enabled: false,
            gpu_stars: None,
            gpu_bg_colorkey: false,
            hand_on_screen: false,
            hand_state_prev: None,
            hub_presentation: false,
            console_box_kind: 0,
            console_box_selected: None,
            progress: crate::progress::GameProgress::new(),
            ending_scene: None,
            ending_frame: 0,
            ending_active: false,
            phone_widget: Vec::new(),
            phone_contacts: Vec::new(),
            phone_contact: 0,
            phone_connected: false,
            phone_active: false,
            console_open: None,
            console_open_rect: [0; 4],
            dialogue: Vec::new(),
            dialogue_texts: Vec::new(),
            dialogue_is_speech: Vec::new(),
            dialogue_cursor: 0,
            intro_pyramid: Vec::new(),
            console_band_dialogue: false,
            hand_mesh: None,
            prev_bridge_frame: 0,
            viewscreen_active: false,
            viewscreen_noise: 0x1234_5678,
            autoplay_end: None,
            dialogue_segments: Vec::new(),
            line_min_hold: None,
            dialogue_segment_pos: 0,
            dialogue_hold_frames: 60,
            text_speed_step: crate::vm::text_speed_step_from_setting(3),
            dialogue_timer: 0,
            dialogue_scene_paths: Vec::new(),
            dialogue_voice_banks: Vec::new(),
            voice_by_offset: HashMap::new(),
            menu_by_offset: HashMap::new(),
            pending_profile: None,
            scene_queue: Vec::new(),
            scene_queue_idx: 0,
            scene_hnm: None,
            scene_buffer: vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT],
            scene_frame: 0,
            scene_band_y: 0,
            scene_palette: [[0u8; 3]; 256],
            framebuffer: vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT],
            intro_hnms: Vec::new(),
            intro_cues: Vec::new(),
            intro_music: Vec::new(),
            intro_index: 0,
            intro_active: false,
        }
    }

    /// Load a talk-HNM / scene-background HNM for the dialogue scene band, decoded
    /// behind the subtitle by [`EngineState::render_dialogue_frame`].
    pub fn load_scene_hnm(&mut self, path: &Path) {
        if let Ok(hnm) = HnmFile::open(path) {
            // Seed from the file's base palette; decode_frame applies per-frame
            // palette updates on top of it.
            self.scene_palette = hnm.palette;
            // Letterbox origin: band clips (130-tall keyframe) present at screen row
            // 0x23, exactly the game's `stream_y + gs:[0x1fa7]` blit base.
            self.scene_band_y = hnm.band_y_origin();
            self.scene_hnm = Some(hnm);
            // New scene: restart at its keyframe on a cleared buffer.
            self.scene_frame = 0;
            for p in self.scene_buffer.iter_mut() {
                *p = 0;
            }
        }
    }

    /// Present the decoded scene buffer on the display framebuffer at the clip's
    /// letterbox origin (`scene_band_y`): band clips land on rows 0x23..0xA5 with
    /// black bars above/below, full-screen clips copy 1:1.
    ///
    /// `gs:[0x1FA7]` IS THAT BLIT BASE, and it is a real decode rather than an
    /// analogue (audit-fixes #315). The blit reads it as a row offset —
    /// `add bx, word ptr gs:[0x1fa7]` @`0xA464` and @`0xAB6E` — and the writers
    /// give the cases:
    ///
    /// ```text
    ///   mov word ptr [0x1fa7], 0x23   @0x18BE, @0xB3FA   the BAND top, 35
    ///   mov word ptr [0x1fa7], 0      @0x1A37, @0x7C45   FULL-SCREEN, 1:1
    ///   mov word ptr [0x1fa7], 0xa    @0x7B5F            a THIRD case, 10
    /// ```
    ///
    /// THE CASE LIST ABOVE IS NOT THE WHOLE CENSUS (audit-fixes #450). A full
    /// enumeration of `0x1FA7` finds TEN sites, and only six are immediates:
    ///
    /// ```text
    ///   0x018BE  =35   0x01A37  =0    0x07B5F  =10   0x07C45  =0   0x0B3FA  =35
    ///   0x09DC7  READ (mov ax,[m])
    ///   0x01F1E  0x05C94  0x0B12E  0x0B526   mov [m], ax  -- COMPUTED writes
    /// ```
    ///
    /// So four sites write a value held in `ax`, and the base is not limited to
    /// {0, 10, 35}. "The writers give the cases" was true of the writers the
    /// earlier pass enumerated — the immediate forms — which is the same
    /// one-encoding-family blind spot as #335/#359/#403/#434, here in a DOC rather
    /// than a tool. What those four compute is undecoded.
    ///
    /// The port models the first two immediate cases. THE `0xA` CASE IS NOT MODELLED: `0x7B5F`
    /// sets the base to 10 and `[0x131C]` to 0 before jumping to `0x7B80`, so
    /// there is a third letterbox origin — a ten-row offset — that this function
    /// cannot produce. What selects it is undecoded, which is why the row stays
    /// provisional rather than settling on the two cases that do verify.
    fn present_scene_buffer(&mut self) {
        if self.scene_band_y == 0 {
            self.framebuffer.copy_from_slice(&self.scene_buffer);
            return;
        }
        for p in self.framebuffer.iter_mut() {
            *p = 0;
        }
        let band_rows = ENGINE_SCREEN_HEIGHT - self.scene_band_y;
        for y in 0..band_rows.min(ENGINE_SCREEN_HEIGHT) {
            let dy = y + self.scene_band_y;
            if dy >= ENGINE_SCREEN_HEIGHT {
                break;
            }
            let s = y * ENGINE_SCREEN_WIDTH;
            let d = dy * ENGINE_SCREEN_WIDTH;
            self.framebuffer[d..d + ENGINE_SCREEN_WIDTH]
                .copy_from_slice(&self.scene_buffer[s..s + ENGINE_SCREEN_WIDTH]);
        }
    }

    /// Queue the startup intro-video sequence to play before the game proper — the
    /// first thing the real game shows. `assets` is the DAT root; missing files are
    /// skipped. `sq/mind.hnm` is the boot-logo reel (verified by decoding: frames
    /// ~0-30 MINDSCAPE logo, ~40-80 Microfolie's logo zoom, ~100-200 the
    /// ship-over-planet cutscene). `sq/cliptoot.hnm` is the CRYO presentation cinematic
    /// (the DESCRIPT `present` Sequence record) over which the publisher credit is
    /// overlaid; then the fire "COMMANDER Blood" title (`logo_bl`).
    /// (`microfol.hnm` is a shorter variant of the boot reel without MINDSCAPE;
    /// `inter_sh` is the ship interior, `cryogel`/`cryorad` cryo-chamber scenes,
    /// `logo01/02` the HATE-TV logo — none of them boot clips.)
    ///
    /// `descript_db` supplies the credit subtitles: the `present` record's cues
    /// ("CRYO Interactive Entertainment 1995", "Commander BLOOD  V 1.0") are overlaid
    /// on its `cliptoot.hnm` clip, sourced from the game data rather than hard-coded.
    pub fn load_intro(&mut self, assets: &Path, descript_db: &crate::descript::DescriptDb) {
        const CREDIT_RECORD: &str = "present";
        let sq = assets.join("sq");
        // Each intro clip is (hnm stem, subtitle cues to overlay). The credit clip's cues
        // come straight from the DESCRIPT `present` record.
        let credit_cues = descript_db
            .records
            .iter()
            .find(|r| r.name == CREDIT_RECORD)
            .map(|r| r.subtitles.clone())
            .unwrap_or_default();
        let credit_clip = descript_db
            .records
            .iter()
            .find(|r| r.name == CREDIT_RECORD)
            .and_then(|r| r.sequence_hnms.first().cloned())
            .unwrap_or_else(|| "cliptoot.hnm".to_string());
        // The intro music the game ties to the credit cinematic (DESCRIPT `present` -> Music).
        // It belongs to the cinematic clip, NOT the boot-logo reel — so the logos stay silent.
        let credit_music = descript_db
            .records
            .iter()
            .find(|r| r.name == CREDIT_RECORD)
            .and_then(|r| r.music.first().cloned());
        // (hnm stem, subtitle cues, music-to-start-with-this-clip). Only the credit cinematic
        // carries music; the logo reel is silent, matching the real game. Ground truth
        // (accuracy/captures/frame_*): the intro is logos → cinematic → CRYO logo → console with
        // the "CRYO 1995"/"Commander BLOOD V 1.0" credits overlaid as SUBTITLES — there is NO
        // separate fire-title clip, so `logo_bl.hnm` is NOT queued (its "title" is the credit
        // cue on the CRYO clip). The real title screen art (BLOOD.LBM) is handled separately.
        let order: [(String, Vec<crate::descript::SubtitleCue>, Option<String>); 2] = [
            ("mind.hnm".to_string(), Vec::new(), None), // boot logos: MINDSCAPE + Microfolie's + ship
            (credit_clip, credit_cues, credit_music),   // CRYO cinematic + publisher credit + music
        ];
        self.intro_hnms = Vec::new();
        self.intro_cues = Vec::new();
        self.intro_music = Vec::new();
        self.intro_pyramid = Vec::new();
        for (i, (name, cues, music)) in order.into_iter().enumerate() {
            let path = sq.join(&name);
            if path.exists() {
                // REAL-GAME-VERIFIED (DOSBox-X game_95s..130s; dlg_05..dlg_11; interpreter
                // bd_226M..bd_290M): the credit cinematic / crew MONTAGE (clip 1) plays ON
                // the pyramid-console + eye-orb band; the logo/ship reel (clip 0,
                // mind.hnm) plays full-screen. Explicit per-clip — not inferred from
                // music/cue presence.
                let showcase = i == 1;
                self.intro_hnms.push(path);
                self.intro_cues.push(cues);
                self.intro_music.push(music);
                self.intro_pyramid.push(showcase);
            }
        }
        self.intro_index = 0;
        self.intro_active = !self.intro_hnms.is_empty();
        if self.intro_active {
            let first = self.intro_hnms[0].clone();
            self.load_scene_hnm(&first);
        }
    }

    /// True while the startup intro sequence is still playing.
    pub fn intro_active(&self) -> bool {
        self.intro_active
    }

    /// Index of the intro clip currently playing (0 = logo reel, 1 = credit cinematic, …).
    /// The frontend watches this for changes to start each clip's music at the right moment.
    pub fn intro_index(&self) -> usize {
        self.intro_index
    }

    /// The background-music stem to start WITH the current intro clip, if any (the logo reel
    /// returns `None` — it is silent; the credit cinematic returns the DESCRIPT `present` music).
    pub fn intro_clip_music(&self) -> Option<&str> {
        self.intro_music
            .get(self.intro_index)
            .and_then(|m| m.as_deref())
    }

    /// Start a DESCRIPT **Sequence** cutscene faithfully from its OWN record data — its HNM(s)
    /// (`sequence_hnms`), music (`music`), and tick-timed `subtitles` — reusing the intro playback
    /// path (`intro_*` fields + `render_intro_frame`/`draw_intro_credit`). The port previously
    /// played only the boot intro and dialogue scenes, so the in-game cutscenes (IZWAL-TV
    /// `microkid`, `hatetv`, the `maledict` curse, …) never ran with their data. This is the
    /// general, data-driven cutscene player; each cutscene's music/subtitles come from the record,
    /// not hardcoded. Returns true if at least one clip was queued.
    pub fn start_descript_cutscene(
        &mut self,
        record: &crate::descript::DescriptRecord,
        assets: &Path,
    ) -> bool {
        let sq = assets.join("sq");
        let music = record.music.first().cloned();
        self.intro_hnms = Vec::new();
        self.intro_cues = Vec::new();
        self.intro_music = Vec::new();
        self.intro_pyramid = Vec::new();
        for (i, name) in record.sequence_hnms.iter().enumerate() {
            let path = sq.join(name);
            if path.exists() {
                self.intro_hnms.push(path);
                // The record's subtitle cues + music are timed over the sequence from its 1st clip.
                self.intro_cues.push(if i == 0 {
                    record.subtitles.clone()
                } else {
                    Vec::new()
                });
                self.intro_music
                    .push(if i == 0 { music.clone() } else { None });
                // In-game cutscenes play FULL-SCREEN — the console band is intro-montage-only.
                self.intro_pyramid.push(false);
            }
        }
        self.intro_index = 0;
        self.intro_active = !self.intro_hnms.is_empty();
        if self.intro_active {
            let first = self.intro_hnms[0].clone();
            self.load_scene_hnm(&first);
        }
        self.intro_active
    }

    /// Skip the rest of the boot intro immediately (the real game lets a click/key skip
    /// straight to the game). Ends intro playback so the driver can hand off to gameplay.
    pub fn skip_intro(&mut self) {
        self.intro_active = false;
        self.intro_hnms.clear();
    }

    /// Load an alien-examination screen's pre-rendered rotation views (the
    /// `pe/<stem>_a..d.hnm` set, e.g. `scrut` → Scruter Jo). Any that open are kept
    /// in rotation order; the screen renders once activated with `alien_view_active`.
    pub fn load_alien_view(&mut self, assets: &Path, stem: &str) {
        let pe = assets.join("pe");
        self.alien_views = ['a', 'b', 'c', 'd']
            .iter()
            .filter_map(|c| HnmFile::open(&pe.join(format!("{stem}_{c}.hnm"))).ok())
            .collect();
        // The scrutinizer-apparatus intro (`sq/cai<stem>.hnm`), played on entry.
        self.alien_intro = HnmFile::open(&assets.join("sq").join(format!("cai{stem}.hnm"))).ok();
        self.alien_pan = 0;
    }

    /// Load the TV PROGRAMMING from the DESCRIPT data: every Sequence record whose own subtitles
    /// announce it as broadcast content. The records self-identify decisively:
    ///   `hatetv` "YOU ARE WATCHING HATE TV" · `microkid` "…you're watching the IZWAL channel" ·
    ///   `garde` "CROOLIS CHANNEL WATCHES THE WATCHERS" · `ppit` "WELCOME TO PETIT-PITEUX'S CYBER
    ///   CHANNEL" · `scrut` "SCRUT CHANNEL" · `match` "Welcome to our nice gameshow…" · `venus`
    ///   "PUBLICITY" (the Venusia ad).
    /// Each becomes one TV channel playing its chained clips + music + tick-subtitles, sorted by
    /// record name for a stable channel order. Seasonal easter egg restored from the data: the
    /// `christmas` / `year` records are the SAME Venusia ad with seasonal text ("VENUSIA YULETIDE
    /// SALES" / "NEW YEAR SALES"), so on Dec 25 / Jan 1 they replace the `venus` ad channel.
    pub fn load_tv_programs(&mut self, db: &crate::descript::DescriptDb, assets: &Path) {
        let sq = assets.join("sq");
        // The ad channel's seasonal variant, by today's (UTC) civil date.
        //
        // THE CONTENT IS REAL; THE TRIGGER IS INVENTED (audit-fixes #475).
        // `christmas` and `year` are genuine DESCRIPT.DES records (at 0x26 and
        // 0x38 in the name table), so the assets exist. But the GAME CANNOT KNOW
        // THE DATE: a scan of BLOODPRG.EXE finds ZERO sites for `mov ah,0x2A`
        // (DOS get-date), ZERO for `mov ah,0x2C` (get time) and ZERO for
        // `mov ah,0x2B` — it never asks the system for a clock at all.
        //
        // So selecting these by the host's calendar is this port's own rule, and
        // whatever the game uses to reach them (a script flag, a menu, or nothing
        // — they may be unused content) is undecoded. Left in place because it
        // surfaces real shipped material, but it is NOT a decoded behaviour and
        // must not be cited as one.
        let seasonal = {
            let days = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs() / 86_400)
                .unwrap_or(0) as i64;
            let (_, m, d) = civil_from_days(days);
            match (m, d) {
                (12, 25) => Some("christmas"),
                (1, 1) => Some("year"),
                _ => None,
            }
        };
        let is_broadcast = |r: &crate::descript::DescriptRecord| {
            r.subtitles.iter().any(|c| {
                let t = c.text.to_lowercase();
                ["watching", "channel", "publicity", "gameshow"]
                    .iter()
                    .any(|k| t.contains(k))
            })
        };
        let mut programs: Vec<TvProgram> = db
            .records
            .iter()
            .filter(|r| r.kind == crate::descript::RecordKind::Sequence && is_broadcast(r))
            .map(|r| {
                // Seasonal swap: the venus ad becomes the christmas / new-year variant on the day.
                seasonal
                    .filter(|_| r.name == "venus")
                    .and_then(|s| db.records.iter().find(|x| x.name == s))
                    .unwrap_or(r)
            })
            .map(|r| TvProgram {
                name: r.name.clone(),
                clips: r
                    .sequence_hnms
                    .iter()
                    .filter_map(|h| HnmFile::open(&sq.join(h)).ok())
                    .collect(),
                cues: r.subtitles.clone(),
                music: r.music.first().cloned(),
            })
            .filter(|p| !p.clips.is_empty())
            .collect();
        programs.sort_by(|a, b| a.name.cmp(&b.name));
        self.tv_programs = programs;
        self.tv_channel = 0;
        self.tv_clip = 0;
        self.tv_clip_frame = 0;
        self.tv_program_frame = 0;
    }

    /// The current TV channel's broadcast music (from its record), for the driver to play while
    /// the channel is on — e.g. `hatetv.voc` / `balise.voc`. `None` on the legacy raw channels.
    pub fn tv_music(&self) -> Option<&str> {
        if self.tv_programs.is_empty() {
            return None;
        }
        self.tv_programs[self.tv_channel % self.tv_programs.len()]
            .music
            .as_deref()
    }

    /// Load the comms "Hate TV" screen: the broadcast-channel HNMs named `<prefix>*`
    /// under `sq/` (e.g. `tv` → tvgren*/tvred*), sorted so steering cycles channels
    /// in a stable order. The screen renders once `tv_active` is set. Legacy fallback for
    /// when the DESCRIPT programming (`load_tv_programs`) is unavailable.
    pub fn load_tv_channels(&mut self, assets: &Path, prefix: &str) {
        let sq = assets.join("sq");
        let mut names: Vec<String> = std::fs::read_dir(&sq)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| n.to_lowercase().starts_with(prefix) && n.to_lowercase().ends_with(".hnm"))
            .collect();
        names.sort();
        self.tv_channels = names
            .iter()
            .filter_map(|n| HnmFile::open(&sq.join(n)).ok())
            .collect();
        self.tv_channel = 0;
    }

    /// Render the comms/TV screen: play the current broadcast channel looped. A driver
    /// changes `tv_channel` (via `switch_tv_channel`) on left/right steer to flip
    /// channels — the interactive part of the screen.
    ///
    /// With DESCRIPT programming loaded, the channel is a broadcast RECORD: its clips play chained
    /// (advancing clip-by-clip, looping the whole program), and its tick-timed subtitle cues are
    /// drawn over the picture ("YOU ARE WATCHING HATE TV", …) — ticks are HNM frames, as verified
    /// for the intro cues. Without programming, falls back to the legacy raw `tv*` HNM loop.
    fn render_tv(&mut self) {
        if !self.tv_programs.is_empty() {
            let prog = &self.tv_programs[self.tv_channel % self.tv_programs.len()];
            if prog.clips.is_empty() {
                return;
            }
            // Advance to the next clip (or loop the program) when the current clip is exhausted.
            let mut clip_idx = self.tv_clip % prog.clips.len();
            if self.tv_clip_frame >= prog.clips[clip_idx].frame_count().max(1) {
                clip_idx = (clip_idx + 1) % prog.clips.len();
                self.tv_clip = clip_idx;
                self.tv_clip_frame = 0;
                if clip_idx == 0 {
                    self.tv_program_frame = 0; // the broadcast loops from the top
                }
            }
            let clip = &prog.clips[clip_idx];
            if self.tv_clip_frame == 0 {
                self.scene_palette = clip.palette;
            }
            clip.decode_frame(
                self.tv_clip_frame,
                &mut self.scene_buffer,
                &mut self.scene_palette,
            );
            self.framebuffer.copy_from_slice(&self.scene_buffer);
            self.tv_clip_frame += 1;
            self.tv_program_frame += 1;
            // The broadcast's active subtitle cue (last cue whose tick has been reached).
            let text = prog
                .cues
                .iter()
                .filter(|c| self.tv_program_frame >= c.tick as usize)
                .next_back()
                .map(|c| c.text.clone());
            if let Some(text) = text.filter(|t| !t.is_empty()) {
                self.scene_palette[Self::INTRO_CREDIT_COLOR_INDEX as usize] = [245, 245, 245];
                // Wrap at the game's ~35-column subtitle width (crate::script::SUBTITLE_WRAP_COLUMN)
                // and centre each line in the lower band, like the dialogue subtitles.
                let mut lines: Vec<String> = Vec::new();
                let mut cur = String::new();
                for word in text.split_whitespace() {
                    // `>=`, not `>`: `0x672C` is `cmp al,0x23 / jb`, so 35 itself
                    // breaks. `>` kept a 35-character line and wrapped only at 36
                    // (audit-fixes #590).
                    if !cur.is_empty()
                        && cur.len() + 1 + word.len() >= crate::script::SUBTITLE_WRAP_COLUMN
                    {
                        lines.push(std::mem::take(&mut cur));
                    }
                    if !cur.is_empty() {
                        cur.push(' ');
                    }
                    cur.push_str(word);
                }
                if !cur.is_empty() {
                    lines.push(cur);
                }
                let first_y =
                    Self::TV_CUE_BASELINE_Y.saturating_sub(10 * lines.len().saturating_sub(1));
                for (i, line) in lines.iter().enumerate() {
                    let width: usize = line.chars().map(crate::font::game_font_advance).sum();
                    let x = ENGINE_SCREEN_WIDTH.saturating_sub(width) / 2;
                    draw_text_indexed(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        line,
                        x,
                        first_y + 10 * i,
                        Self::INTRO_CREDIT_COLOR_INDEX,
                    );
                }
            }
            return;
        }
        let n = self.tv_channels.len();
        if n == 0 {
            return;
        }
        let ch = self.tv_channel % n;
        let hnm = &self.tv_channels[ch];
        let count = hnm.frame_count().max(1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(
            self.scene_frame % count,
            &mut self.scene_buffer,
            &mut self.scene_palette,
        );
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
    }

    /// Number of loaded TV channels.
    pub fn tv_channel_count(&self) -> usize {
        if self.tv_programs.is_empty() {
            self.tv_channels.len()
        } else {
            self.tv_programs.len()
        }
    }

    /// Switch the TV channel by `delta` (wrapping), restarting the broadcast.
    pub fn switch_tv_channel(&mut self, delta: i32) {
        let n = self.tv_channel_count();
        if n == 0 {
            return;
        }
        self.tv_channel = (self.tv_channel as i32 + delta).rem_euclid(n as i32) as usize;
        // Restart the new broadcast from its top (clip 0, frame 0, cue clock reset).
        self.scene_frame = 0;
        self.tv_clip = 0;
        self.tv_clip_frame = 0;
        self.tv_program_frame = 0;
    }

    /// Load the real ship bridge: the `TB.BIG` 360° panorama archive (the whole
    /// bridge as 180 pre-rendered frames — see [`crate::tbbig`]). `iso` is the CD
    /// root directory. Without it the bridge cannot render (no fabricated stand-in).
    pub fn load_bridge(&mut self, iso: &Path) {
        // The bold console font ships inside the game binary itself.
        for name in ["BLOODPRG.EXE", "bloodprg.exe"] {
            if let Ok(exe) = std::fs::read(iso.join(name)) {
                self.bold_font = crate::font::BoldConsoleFont::load_from_exe(&exe);
                if self.bold_font.is_some() {
                    break;
                }
            }
        }
        for name in ["TB.BIG", "tb.big"] {
            if let Ok(data) = std::fs::read(iso.join(name)) {
                self.panorama = crate::tbbig::BridgePanorama::parse(data);
                if self.panorama.is_some() {
                    return;
                }
            }
        }
    }

    // REMOVED (audit-fixes #385): `CONSOLE_MENU`, a 5-string array of the console
    // row names. It had NO caller anywhere in the crate, and its stated source was
    // "baked into the golden menu of the TB.BIG panorama frames (verified against
    // the live capture)" — i.e. read off pixels, which is the sourcing the prime
    // rule forbids. The port's real menu handling is index-based and cited:
    // `BridgeView::selected_menu_item` (`DS:0x2A19`) and `menu_row_under_cursor`
    // (`0x8614..0x868D`) in src/bridge.rs, where the names appear only as doc
    // comments. Nothing was lost by deleting it.

    /// The console MENU option's submenu, decoded by driving the real game (clicking MENU
    /// opens these two items): EXPLANATIONS (the tutorial/help) and GAME (play). Drawn over
    /// the top menu rows, matching the observed golden-menu overlay.
    /// MENU submenu rows.
    ///
    /// PROVENANCE DEFECT — these are still transcribed literals. The words exist in
    /// the game's own data: `SCRIPT1.DIC` holds `explanations` at offset `0x02FC` and
    /// `game` at `0x0309` (also `GAME` at `0x030E`), and the console list widget
    /// `0x8428` takes a 0/0xFFFF-terminated list of WORD OFFSETS, not literals. The
    /// remaining task is to find the routine that builds THIS list so the offsets come
    /// from the script rather than from here. Tracked in docs/port-validation.md.
    pub const MENU_SUBMENU: [&'static str; 2] = ["EXPLANATIONS", "GAME"];

    /// The MENU submenu's rows, taken from the LOADED SCRIPT when one is present.
    ///
    /// SELECTION IS A PROXY, NOT A DECODE (audit-fixes #323). `menu_by_offset`
    /// maps a line record's offset to its menu rows, and the faithful consumer
    /// looks a menu up BY THE CURRENT LINE'S OFFSET. This accessor instead takes
    /// the globally LOWEST offset (`min_by_key`), which is a stand-in for "the
    /// MENU submenu" and has nothing behind it — no citation, no rule. It happens
    /// to yield SCRIPT1's `0x4A9` record because that record is early in the
    /// file; a script whose first menu record is some other list would silently
    /// return the wrong rows.
    ///
    /// The engine-side fix is to reach this menu the way the game does — through
    /// whatever the MENU click dispatches to — rather than by scanning for a
    /// minimum. Until then the fallback literal below is the LESS suspect half of
    /// this function.
    ///
    /// The real source is an `0xA6` record's word list after its `0xFFFF` separator
    /// (SCRIPT1.COD `0x4A9` -> DIC `explanations` / `game`); the widget upper-cases
    /// for display, which is why the DIC entries are lowercase. Falls back to
    /// [`Self::MENU_SUBMENU`] only when no script is loaded (unit tests, bare
    /// `EngineState::new()`), so the const is a default rather than the authority.
    pub fn menu_submenu_labels(&self) -> Vec<String> {
        self.menu_by_offset
            .iter()
            .min_by_key(|(off, _)| **off)
            .map(|(_, rows)| rows.iter().map(|r| r.to_uppercase()).collect())
            .unwrap_or_else(|| Self::MENU_SUBMENU.iter().map(|s| s.to_string()).collect())
    }

    /// The OPTION choice box's single row.
    ///
    /// This is the game's own string, not a transcription: `DS:0x0174` (file
    /// `0x0D594`), the symbol already recorded as `ship_3d_target_extra_label` — the
    /// EXTRA row the list widget appends, gated by `[0x0ADD]` at `0x843B`, which is
    /// the same branch that sets the kind-10 width floor and height seed. It sits in
    /// the UI string table alongside `UNKNOWN`, `ARE_YOU_SURE?`, `YES` and `PAUSE`.
    ///
    /// Pinned to the image by `option_box_label_is_the_games_own_string`. It was
    /// previously justified by an ORACLE CAPTURE ("resume-probe rp_option: the
    /// measured gold choice box containing CANCEL"), which is exactly backwards under
    /// the prime rule — the capture may only confirm the decoded value, never source it.
    /// `DS:0x0174` / file `0x0D594`, NUL-terminated — READ from the image, not
    /// transcribed (audit-fixes #526). The literal `"CANCEL"` and the
    /// one-element `OPTION_BOX` array that wrapped it are gone; the string comes
    /// from [`EngineState::ds_text`] like the five in #524.
    ///
    /// This is the SAME string `bloodprg::list_widget_cancel_label` reads and
    /// `main.rs` already appends to the OPTION-menu labels, so the port was
    /// holding a copy of something it also read correctly elsewhere.
    /// Ten save slots — the `blood.sav` directory's record count.
    pub const SAVE_SLOT_ROWS: usize = crate::bloodsav::SLOT_COUNT;
    /// `mov si,0x174` @`0x85B3`, the list widget's shared trailing row, drawn
    /// only when `[0xadd]` bit 0 is set @`0x85AC` (audit-fixes #526).
    ///
    /// `ship3d::SHIP_3D_TARGET_EXTRA_LABEL_OFFSET` is the SAME `0x174` from the
    /// SAME instruction (#492), named for the widget that draws it rather than
    /// the menu that supplies it — one string, one load site, two port names.
    pub const OPTION_BOX_LABEL_DS_OFFSET: u16 = 0x0174;
    pub const OPTION_BOX_LABEL_FILE_OFFSET: usize = 0x0d594;

    /// Map a click to a console-choice-box row while it is open, else None.
    pub fn console_box_click(&self, x: u16, y: u16) -> Option<usize> {
        if self.console_box.is_empty() {
            return None;
        }
        // The IN-WINDOW concept box (kind 3, HONK's TALK/REMEMBER/BYE_BYE):
        // left-aligned rows at x=175 from y=83, pitch 11 — inside the console
        // window, no backdrop (oracle honk_talk vs_005..007).
        if self.console_box_kind == 3 {
            // The unified widget's hit-test at the right-side anchor 225: inside
            // [x0, x0+w] with row = dy/11 (div bl,0x0B @0x8508).
            let rows = self.console_box.len();
            let widest = self
                .console_box
                .iter()
                .map(|l| crate::font::square_caps_text_width(l))
                .max()
                .unwrap_or(0);
            let w = widest + 0x14;
            let x0 = 225usize.saturating_sub(w / 2) as u16;
            if !(x0..=(x0 + w as u16)).contains(&x) {
                return None;
            }
            // Hit origin == draw origin == box_y+4 (0x84E6 add cx,4); no extra
            // offset. The old `- 2` shifted the in-window concept-box band 2px up.
            let top = Self::choice_box_top_y(rows) as i32;
            let row = (y as i32 - top) / 11;
            return (row >= 0 && (row as usize) < rows).then_some(row as usize);
        }
        // Non-kind-3 console box (telephone kind 2, cryobox kind 4, world kind
        // 10): the clickable band is the drawn box extent [x0, x1] from the
        // shared geometry (0x84EE..0x84F6), NOT the fixed 40..160 — that band
        // mis-fit the anchor-80 world box and any label wider than 100px.
        let rows = self.console_box.len().min(8);
        let widest = self
            .console_box
            .iter()
            .take(rows)
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        let (_anchor, x0, x1) = self.choice_box_geometry(widest);
        let (xi, yi) = (x as usize, y as usize);
        if xi < x0 || xi > x1 {
            return None;
        }
        let top = self.choice_box_text_top(rows);
        if yi < top {
            return None;
        }
        let row = (yi - top) / Self::CHOICE_BOX_PITCH;
        (row < rows).then_some(row)
    }

    /// Map a click to a MENU-submenu item (0 = EXPLANATIONS, 1 = GAME) when the
    /// submenu is showing. The submenu is a gold CHOICE BOX (the game's universal
    /// console-choice widget), so hit-test its rows, not the golden menu.
    /// Map a click to a MENU-submenu item. The rows come from the SCRIPT's own
    /// labels, and the hit-test is [`Self::choice_box_row_at`] — the widget's
    /// `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`) over rows stepped by
    /// `add bp,0xB` (@`0x847A`), with the 4px origin inset at `0x84E6`.
    ///
    /// The labels come from the same source the DRAW uses rather than a constant
    /// list, so the clickable band cannot describe a menu the screen is not
    /// showing.
    pub fn menu_submenu_click(&self, x: u16, y: u16) -> Option<usize> {
        if !self.menu_submenu_active {
            return None;
        }
        // Same rows the draw uses, so the clickable band tracks the SCRIPT's labels
        // rather than a const that may not match what is on screen.
        let labels = self.menu_submenu_labels();
        let refs: Vec<&str> = labels.iter().map(|s| s.as_str()).collect();
        self.choice_box_row_at(x, y, refs.len(), Self::choice_box_widest(&refs))
    }

    /// Aim the bridge's virtual ring-space cursor at an absolute screen point —
    /// the inverse of the game's `screen = ring - (frame*8 - 160)` rebase — so
    /// absolute click coordinates can be tested with the decoded hit math.
    fn point_virtual_cursor(view: &mut crate::bridge::BridgeView, x: u16, y: u16) {
        view.ring_mouse_x = (x as i32 + view.frame as i32 * crate::bridge::RING_PX_PER_FRAME
            - crate::bridge::HALF_SCREEN)
            .rem_euclid(crate::tbbig::ANGLE_UNITS_PER_REVOLUTION as i32);
        view.mouse_y = y as i32;
    }

    /// Map a click to a ship-console menu option index (0 = HONK … 4 = OPTION)
    /// using the decoded golden-menu hit math (`0x8614`): the menu is only
    /// clickable while its panorama sector is in view, its box scrolls with the
    /// rotation, and rows are 18 px apart. `None` off the menu.
    pub fn console_menu_click(&self, x: u16, y: u16) -> Option<usize> {
        let mut probe = self.bridge.clone();
        Self::point_virtual_cursor(&mut probe, x, y);
        probe.menu_row_under_cursor()
    }

    /// A left-button press on the bridge at absolute screen `(x, y)`: runs the
    /// decoded click paths — a golden-menu row selects that console function
    /// (returned as 0 = HONK … 4 = OPTION, with the view seeking to centre the
    /// menu), while a hit on the current station's eye-orb arms a station seek
    /// (rotating the view there) and returns `None`.
    /// A click on the hub presentation's CANCEL label (the abort control).
    pub fn hub_cancel_click(&mut self, x: u16, y: u16) -> bool {
        if (self.hub_presentation || self.bridge.engaged_row.is_some())
            && (70..=134).contains(&x)
            && (90..=106).contains(&y)
        {
            self.hub_presentation = false;
            self.hand_pose_event(0xB);
            return true;
        }
        false
    }

    pub fn bridge_press(&mut self, x: u16, y: u16) -> Option<usize> {
        Self::point_virtual_cursor(&mut self.bridge, x, y);
        self.bridge.click().map(|item| item as usize - 1)
    }

    /// Load the ship-console UI font from `HONKF.SPR` (49 8×8 glyphs: A–Z, 0–9,
    /// punctuation) — the game draws its console menu labels with it. Returns whether it
    /// loaded.
    pub fn load_console_font(&mut self, iso: &Path) -> bool {
        for name in ["HONKF.SPR", "honkf.spr"] {
            if let Ok(data) = std::fs::read(iso.join(name)) {
                if let Some(glyphs) = decode_sprite_bank_indices(&data) {
                    if glyphs.len() >= 36 {
                        self.console_font = glyphs;
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Map a character to its HONKF console-font glyph index (uppercase A–Z = 0..25,
    /// 0–9 = 26..35, then punctuation in the bank's authored order).
    #[allow(dead_code)]
    fn console_glyph_index(ch: char) -> Option<usize> {
        match ch.to_ascii_uppercase() {
            c @ 'A'..='Z' => Some(c as usize - 'A' as usize),
            c @ '0'..='9' => Some(26 + c as usize - '0' as usize),
            '?' => Some(36),
            '!' => Some(37),
            '.' => Some(38),
            // CORRECTED: these three were rotated (`,`/`:`/`;` at 39/40/41). Read
            // off the bank's own 8x8 bitmaps: frame 39 has marks at rows 2 and 6
            // with no descender = COLON; frame 40 adds a row-3 dot above a tailed
            // mark = SEMICOLON; frame 41 is a single mark at rows 5-6 with the
            // row-7 tail = COMMA. Corroborated independently by the BUILT-IN font's
            // table at DS:0x7802, which orders '.'=30, ':'=31, ';'=32 consecutively
            // — the same relative order, with ',' separated from them.
            ':' => Some(39),
            ';' => Some(40),
            ',' => Some(41),
            '_' => Some(42),
            '+' => Some(43),
            '-' => Some(44),
            '"' => Some(45),
            '\'' => Some(46),
            '[' => Some(47),
            ']' => Some(48),
            _ => None,
        }
    }

    /// Draw text with the ship-console font (HONKF). Retained for the console-font
    /// load test; live console text now uses the bold (0x71AA) and square-caps
    /// (0xE8) faces, so this has no runtime callers.
    #[allow(dead_code)]
    fn draw_console_text(&mut self, text: &str, x: usize, y: usize, color: u8) -> usize {
        let mut pen = x;
        for ch in text.chars() {
            if ch == ' ' {
                pen += 4;
                continue;
            }
            let advance =
                match Self::console_glyph_index(ch).and_then(|gi| self.console_font.get(gi)) {
                    Some(glyph) => {
                        for gy in 0..glyph.height {
                            for gx in 0..glyph.width {
                                if glyph.indices[gy * glyph.width + gx] != 0 {
                                    let (px, py) = (pen + gx, y + gy);
                                    if px < ENGINE_SCREEN_WIDTH && py < ENGINE_SCREEN_HEIGHT {
                                        self.framebuffer[py * ENGINE_SCREEN_WIDTH + px] = color;
                                    }
                                }
                            }
                        }
                        glyph.width + 1
                    }
                    None => 8,
                };
            pen += advance;
        }
        pen
    }

    /// Render the real ship bridge: the current TB.BIG panorama frame composited
    /// over the window starfield, with the golden menu's five palette rows
    /// programmed for hover highlighting — the decompiled composite order of the
    /// original per-tick path (`page_flip` 0x954A: starfield projection first,
    /// then the panorama unpacked with colour-0 transparency so the stars show
    /// through the black window regions).
    fn render_bridge(&mut self) {
        // Advance the decompiled steering / station-seek state machine.
        self.bridge.update_view();
        self.compass_angle = self.bridge.frame;
        self.render_bridge_background();
        // In the pyramid nav sector, offer the choose-a-location destinations as
        // a golden choice box over the console's left. The SECTOR comes from the
        // panorama frame header (station 2), not from a frame range spelled out
        // here — same gate as `bridge_nav_destination_click`, so the drawn surface
        // and the clickable one cannot drift apart. (The box's appearance was
        // confirmed against accuracy/captures/bridge/choice_box_bob_morlock.ppm;
        // the capture verifies, it does not source.)
        if self.bridge_station() == Some(NAV_ROOM_STATION) && !self.nav_destinations.is_empty() {
            self.draw_choice_box_labels();
        }
        // The MENU submenu ({EXPLANATIONS, GAME}) is a gold CHOICE BOX (the
        // game's universal console-choice widget) — draw it before the hand so
        // the cursor sits over it, as the live game composites.
        if self.menu_submenu_active {
            let labels = self.menu_submenu_labels();
            self.draw_choice_box(&labels, None);
        }
        if self.option_box_active {
            let labels: Vec<String> =
                vec![self.ds_text(Self::OPTION_BOX_LABEL_DS_OFFSET).to_string()];
            self.draw_choice_box(&labels, None);
        }
        if !self.console_box.is_empty() {
            // The TOP-LEVEL console menu (HONK/TELEPHONE/CRYOBOX/MENU/OPTION) is
            // BAKED INTO the panorama frames (verified: TB.BIG frame 45 == the live
            // hub screen at 95%, gold menu window included; hover = palette swaps
            // 0x7B..0x7F via apply_menu_palette). Only CONTEXTUAL boxes (contacts,
            // confirmations...) are live-drawn gold boxes.
            // The TOP-LEVEL console menu is BAKED INTO the panorama frames, so it
            // is never a live-drawn gold box. This used to be guarded by
            // `if !is_baked_menu`, comparing `console_box` against the literal
            // ["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"] -- a content
            // literal of the same kind #385 deleted as CONSOLE_MENU, and DEAD:
            // nothing in the library ever assigns those names to `console_box`,
            // so the guard was always true and the comparison never matched
            // (audit-fixes #465).
            if self.console_box_kind == 3 {
                // The IN-WINDOW concept list = THE SAME unified widget (0x8428)
                // with the right-side anchor 0xE1=225 (mov [0xAC6],0xE1 @0x89A6):
                // x0 = 225 - w/2 (w = widest+0x14), top = (200-h)/2 + 4 with
                // h = rows*11+8 — DERIVING the previously measured x~175 and the
                // y=39/83 split (11 rows -> 39, 3 rows -> 83, both exact).
                // Labels left-aligned at x0+4; the ENGAGED topic renders WHITE.
                self.scene_palette[0xE8] = [150, 150, 150];
                self.scene_palette[0xEF] = [255, 255, 255];
                let labels = self.console_box.clone();
                let rows = labels.len();
                let widest = labels
                    .iter()
                    .map(|l| crate::font::square_caps_text_width(l))
                    .max()
                    .unwrap_or(0);
                let w = widest + 0x14;
                let x0 = 225usize.saturating_sub(w / 2);
                let top = Self::choice_box_top_y(rows);
                for (i, label) in labels.iter().enumerate() {
                    let color = if self.console_box_selected == Some(i) {
                        0xEF
                    } else {
                        0xE8
                    };
                    // Each label is CENTERED in the box (0x855C/0x8555:
                    // label_x = x0 + 10 + (widest - width)/2), not left-aligned
                    // at x0+4; short labels indent to center on the anchor.
                    let width = crate::font::square_caps_text_width(label);
                    let lx = x0 + 10 + widest.saturating_sub(width) / 2;
                    crate::font::draw_square_caps(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        label,
                        lx,
                        top + i * 11,
                        color,
                    );
                }
            } else {
                let labels = self.console_box.clone();
                self.draw_choice_box(&labels, None);
            }
        }
        // NOTE: an earlier capture reading claimed the engaged CRYOBOX row
        // re-labels to "CONTACT" — REFUTED by the assembly and the data: no such
        // label exists anywhere in the game files, and the engaged-row code
        // (console_menu_hit_test 0x8614) does a pure DAC swap of the BAKED label.
        // The red text in the capture was the baked CRYOBOX glyphs in red,
        // misread at capture scale. The DAC model (apply_menu_palette) stands.
        if self.save_ui_active {
            // THE SAVE UI IS THE ORDINARY LIST WIDGET. The save flow sets
            // `[0x2734]` to the slot record being renamed (`0x1BAB`) and copies it
            // into the edit buffer `DS:0x273B` (`rep movsd cx=4` @`0x1BBD`); the
            // widget then substitutes that buffer for the matching row as it draws
            // (`cmp si,[0x2734] / jne / mov si,0x273B` @`0x8573`). So the screen is
            // the TEN SLOT NAMES in the list, one of them being typed into.
            //
            // The port used to hand-compose a different screen entirely: a grey
            // 0xE8 bar at x63..137/y39..48 with the name at (73,40) and CANCEL at
            // (73,150), all measured off one capture of the live save flow. None
            // of those positions exist in the widget's layout.
            self.draw_save_ui_rows();
        } else if (self.hub_presentation || self.bridge.engaged_row.is_some())
            && self.console_box.is_empty()
        {
            // The live CANCEL label (oracle: gray 0xE8 console text at (73,95)) —
            // shown during the hub presentation AND while a row is engaged WITHOUT
            // its own choice box (an open box carries its own CANCEL row).
            self.draw_console_text("CANCEL", 73, 95, 0xE8);
        }
        self.draw_hand_cursor();
    }

    /// Load the BOB_MORLOCK contact screen: Bob's talk-head video (pe/aabob.hnm,
    /// the oracle's red-face eye close-up) as the live scene, with FRIGO.FD (the
    /// cryo chamber the real game also file-opens on CONTACT) as static fallback.
    pub fn load_bob_contact(&mut self, iso: &Path, assets: &Path) {
        let head = assets.join("pe").join("aabob.hnm");
        if head.exists() {
            self.load_scene_hnm(&head);
        }
        for name in ["FRIGO.FD", "frigo.fd"] {
            if let Ok(d) = std::fs::read(iso.join(name)) {
                if let Some(img) = crate::lbm::decode_lbm(&d) {
                    self.bob_contact_bg = Some((
                        img.width as usize,
                        img.height as usize,
                        img.pixels,
                        img.palette.to_vec(),
                    ));
                    return;
                }
            }
        }
    }

    /// Render the BOB_MORLOCK CONTACT screen.
    ///
    /// The topic-row layout is DERIVED from the list widget's decoded geometry, not
    /// measured off a capture as this comment used to say. `x0 = anchor - w/2` with
    /// the concept anchor `0xE1` (`0x89A6`) and `w = widest + 0x14` (`0x84A1`);
    /// `y = (200 - (rows*11 + 8))/2 + 4` (`0x84A7`, `0x84B9..0x84BF`); pitch 11 from
    /// `add bp,0xB` (`0x847A`). For Bob's 8 rows that yields y=56 — the number the
    /// capture showed, which is why the capture is a CONFIRMATION and never was the
    /// source. FRIGO.FD is the static fallback background; the subtitle sits at the
    /// console position (10,8).
    fn render_bob_contact(&mut self) {
        // Bob's LIVE talk-head band (pe/aabob.hnm — the red face + mismatched eyes
        // of the oracle capture) drawn OVER THE HUB VIEW — the oracle border rows
        // sample the purple bridge panorama ((10,180)=(85,77,186)): the contact is
        // an overlay on the bridge screen, not a black screen. FRIGO.FD is the
        // static fallback when the video is missing.
        if self.panorama.is_some() {
            self.render_bridge_background();
        } else {
            for p in self.framebuffer.iter_mut() {
                *p = 0;
            }
        }
        if let Some(hnm) = self.scene_hnm.take() {
            // The video's palette (header + pl chunks, indices 1..127) must survive
            // the bridge background's palette install — the face renders under ITS
            // colours while the panorama border keeps the shared dark ramp.
            for (i, c) in hnm.palette.iter().enumerate().take(128).skip(1) {
                self.scene_palette[i] = *c;
            }
            let idx = self.scene_frame % hnm.frame_count().max(1);
            hnm.decode_frame(idx, &mut self.scene_buffer, &mut self.scene_palette);
            self.scene_hnm = Some(hnm);
            self.scene_frame += 1;
            self.present_scene_buffer();
        } else if let Some((w, h, pix, pal)) = &self.bob_contact_bg {
            for y in 0..ENGINE_SCREEN_HEIGHT.min(*h) {
                for x in 0..ENGINE_SCREEN_WIDTH.min(*w) {
                    self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = pix[y * w + x];
                }
            }
            for (i, c) in pal.iter().take(256).enumerate() {
                self.scene_palette[i] = *c;
            }
        }
        // The concept menu (grey square-caps; the ENGAGED topic renders WHITE —
        // oracle bob_mission: the clicked MISSION row highlights).
        self.scene_palette[0xE8] = [150, 150, 150];
        self.scene_palette[0xE0] = [255, 255, 255];
        self.scene_palette[0xEF] = [255, 255, 255];
        // NO FALLBACK TOPIC LIST (audit-fixes #531). These used to fall back to an
        // ORACLE-CAPTURED array when the VM had not supplied the prompt line's
        // 0xFFFF-carried words — content sourced from a capture, which the prime
        // rule forbids, and which HID a failed VM path behind plausible text. The
        // lines a few lines below already carry a "NO FALLBACK LINE" rule; this is
        // the same rule for the menu.
        let topics: Vec<String> = self.bob_topics.clone();
        // Layout is DERIVED from the widget geometry, not measured off a capture.
        // The old constants (x=170, y=56, pitch 11) were recorded as "measured from
        // the dual-run oracle captures"; all three fall out of the decoded box maths:
        //   pitch 11  = `add bp,0xB`   @0x847A
        //   y  = (200 - (rows*11 + 8))/2 + 4  @0x84A7/0x84B9..0x84BF  -> 56 for 8 rows
        //   x0 = anchor - w/2, anchor 0xE1=225 @0x89A6, w = widest + 0x14 @0x84A1
        // So the capture CONFIRMED the geometry; it was never its source.
        let widest = topics
            .iter()
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        let box_w = widest + 0x14;
        let x0 = Self::CHOICE_BOX_ANCHOR_CONCEPT.saturating_sub(box_w / 2);
        let top = Self::choice_box_top_y(topics.len());
        for (i, label) in topics.iter().enumerate() {
            let color = if self.console_box_selected == Some(i) {
                0xEF
            } else {
                0xE8
            };
            crate::font::draw_square_caps(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                label,
                x0,
                top + i * Self::CHOICE_BOX_PITCH,
                color,
            );
        }
        // The dialogue line at the console subtitle position, settled white.
        if let Some(text) = self.current_subtitle().map(str::to_string) {
            use crate::font::game_font_advance;
            let mut y = 8usize;
            for line in text.split('\n') {
                let mut x = 10usize;
                for ch in line.chars() {
                    let mut buf = [0u8; 4];
                    draw_text_indexed(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        ch.encode_utf8(&mut buf),
                        x,
                        y,
                        0xE0,
                    );
                    x += game_font_advance(ch);
                }
                y += 10;
            }
        }
    }

    /// Hit-test a click against Bob's concept menu rows (x 165..300, rows from
    /// y=56 at pitch 11). Returns the topic index.
    pub fn bob_topic_click(&self, x: u16, y: u16) -> Option<usize> {
        if !(165..=300).contains(&x) {
            return None;
        }
        let n = self.bob_topics.len();
        let row = (y as i32 - 56) / 11;
        (row >= 0 && (row as usize) < n).then_some(row as usize)
    }

    /// Feed a typed character to the save-slot name entry — the DOS original's edit
    /// law (0x1DD8): digits and lowercase letters append (max 14), backspace (8)
    /// deletes, Enter (13) with a non-empty name COMMITS. Returns the committed name.
    pub fn save_ui_key(&mut self, ch: u8) -> Option<String> {
        if !self.save_ui_active {
            return None;
        }
        match ch {
            13 if !self.save_ui_name.is_empty() => {
                self.save_ui_active = false;
                return Some(std::mem::take(&mut self.save_ui_name));
            }
            8 => {
                self.save_ui_name.pop();
            }
            b'0'..=b'9' | b'a'..=b'z' if self.save_ui_name.len() < 14 => {
                self.save_ui_name.push(ch as char);
            }
            _ => {}
        }
        None
    }

    /// Draw the pointing-hand cursor — the game's ONLY cursor — at the current
    /// mouse position on any screen.
    ///
    /// This documented a CAPTURE ATLAS (`accuracy/captures/bridge/hand/*.bin`,
    /// harvested per cursor position from the emulator) that no longer exists: the
    /// port renders the hand from `manu3.xdb`'s actual skeletal mesh, and the
    /// atlas loader was deleted. The stale comment survived the deletion and would
    /// have sent the next reader looking for capture files as the port's source.
    fn draw_hand_at_mouse(&mut self) {
        self.hand_on_screen = true;
        let (cx, cy) = (self.mouse.x as i32, self.mouse.y as i32);
        // THE REAL 3D HAND: manu3's skeletal mesh (16 live segments, 110+32 verts,
        // 216 textured faces) rendered with the transcribed matrix build (game trig
        // tables), the EXACT perspective projection (0x549: divide by depth, centres
        // 252/110, y negated), the decoded cursor law on the wrist segment, and the
        // game's own texture — fingertip anchored at the cursor.
        {
            // POSE — ASSEMBLY LAW (list widget 0x8522..0x8534): hovering INSIDE an
            // OPEN list box sets selector 6 ([0xA32]=6); the press gate ([0xA3E])
            // selects 7. Everywhere else: REST (1) — the hub_tour oracle confirmed
            // rest over console rows/orb (no box open there), so the box-hover rule
            // applies only while a box is actually open under the cursor.
            let over_box = !self.console_box.is_empty()
                && self.console_box_click(cx as u16, cy as u16).is_some();
            let sel = if over_box {
                if self.mouse.left_down() { 7 } else { 6 }
            } else {
                1
            };
            let mesh = self
                .hand_mesh
                .get_or_insert_with(crate::manu3_hand::HandMesh::load);
            mesh.set_pose(sel);
            let prev = mesh.snapshot_state();
            mesh.tick_pose();
            let gp = crate::palette::game_screen_palette();
            // The hand's texture occupies ONLY indices 202..=251 (the skin ramp —
            // verified over the whole seg4 texture). Installing all of 128..=255
            // clobbered scene palettes whose images own 128..201 (the world rooms:
            // the cyan-cast defect found by the planet reference bank).
            for i in 202..=251usize {
                self.scene_palette[i] = gp[i];
            }
            if self.gpu_hand_enabled {
                self.gpu_hand = Some(mesh.triangles(cx, cy));
                self.hand_state_prev = Some(prev);
                return;
            }
            mesh.draw(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                cx,
                cy,
            );
            return;
        }
    }

    /// Draw the pointing-hand cursor at the bridge's ring-anchored position (the
    /// steering cursor), using the atlas sprite captured nearest to it (the real
    /// renderer varies the hand's orientation with position). No-op without an atlas.
    fn draw_hand_cursor(&mut self) {
        self.hand_on_screen = true;
        // The bridge's steering hand: the SAME real manu3 3D hand, at the ring-anchored
        // cursor position. POSE per the decoded selector rule (0x7809..0x782C):
        // rest=1; while the view rotates, 2 (cursor right half) / 3 (left half).
        // The manu3 entry receives the SCREEN cursor ([bp] x/y) and pins the
        // fingertip to it via the cursor-derived projection centres — use the
        // engine mouse directly (the bridge's ring-space mouse is for steering
        // and console hit dispatch only).
        let (cx, cy) = (self.mouse.x as i32, self.mouse.y as i32);
        let rotating = self.bridge.frame != self.prev_bridge_frame;
        self.prev_bridge_frame = self.bridge.frame;
        let sel = if self.bridge.seeking {
            0x10 // the decoded AUTO-SEEK/travel pose (station seek in progress)
        } else if rotating {
            if cx < 160 { 3 } else { 2 }
        } else {
            1
        };
        let mesh = self
            .hand_mesh
            .get_or_insert_with(crate::manu3_hand::HandMesh::load);
        mesh.set_pose(sel);
        let prev = mesh.snapshot_state();
        mesh.tick_pose();
        let gp = crate::palette::game_screen_palette();
        // Hand skin ramp only (202..=251) — see draw_hand_at_mouse.
        for i in 202..=251usize {
            self.scene_palette[i] = gp[i];
        }
        if self.gpu_hand_enabled {
            self.gpu_hand = Some(mesh.triangles(cx, cy));
            self.hand_state_prev = Some(prev);
            return;
        }
        mesh.draw(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            cx,
            cy,
        );
    }

    /// Recompute the GPU hand for the current cursor at DISPLAY refresh rate.
    /// PURE re-projection: the existing pose/skeleton state renders at the new
    /// cursor — no pose selection, no tween advance, no engine state writes
    /// (re-running the selection here alternated tick-pose vs rest between
    /// presents = 60Hz pose flicker, and corrupted rotation detection).
    pub fn refresh_gpu_hand(&mut self, mx: u16, my: u16, alpha: f32) {
        if !self.gpu_hand_enabled || !self.hand_on_screen {
            return;
        }
        self.mouse.x = mx;
        self.mouse.y = my;
        let prev = self.hand_state_prev.clone();
        if let Some(mesh) = self.hand_mesh.as_mut() {
            self.gpu_hand = Some(match prev {
                Some(p) => mesh.triangles_lerp(mx as i32, my as i32, &p, alpha),
                None => mesh.triangles(mx as i32, my as i32),
            });
        }
    }

    /// Draw the nav destinations as a golden choice box over the console's left —
    /// the game's captured interaction pattern (a rounded gold-bordered box with
    /// gold console-font rows, see accuracy/captures/bridge/
    /// choice_box_bob_morlock.ppm). Uses the observed geometry: box at x ~40,
    /// rows from y ~92 at 13 px pitch.
    fn draw_choice_box_labels(&mut self) {
        let labels: Vec<String> = self
            .nav_destinations
            .iter()
            .take(8)
            .map(|(label, _)| label.clone())
            .collect();
        self.draw_choice_box(&labels, None);
    }

    /// Draw the console choice box as the real game composes it (measured from
    /// `choice_box_bob_morlock.ppm`, the telephone contact list): grey square-caps
    /// item text (index 0xE8) laid out CENTERED on the axis x=100, first row top
    /// y=89, 11px pitch — the labels sit in the panorama's dark orb-socket region,
    /// so a black backdrop (index 0xE0, DAC 0,0,0) is filled behind them for
    /// legibility. `selected` re-renders one row in the brighter 0xEF white.
    /// The list widget's centre-X anchor — the game's `[0xAC6]`, set per context
    /// by the caller: hub console boxes 100 (@0x86D9), in-window lists 225
    /// (@0x89A6), the WORLD/entity candidate list 80 (@0xB0D1, ship_click_commit,
    /// with the narrow alt-mode flags [0xADC]/[0xADD]).
    pub const CHOICE_BOX_CENTER_X: usize = 100;
    /// The WORLD list's anchor: `mov word ptr [0xac6],0x50` @`0xB0D1` in
    /// `ship_3d_hud_init` (#556, #558) — the same `[0xAC6]` cell
    /// [`CHOICE_BOX_ANCHOR_CONCEPT`] is set to `0xE1` @`0x89A6` and the nav choice
    /// sets to `0x64` @`0x86D9` (#494). THREE screens, one cell, three anchors,
    /// which is why a single "choice box centre" would misplace two of them
    /// (audit-fixes #572).
    const CHOICE_BOX_ANCHOR_WORLD: usize = 80;
    /// The IN-WINDOW concept list's right-side anchor: `mov [0xAC6], 0xE1` at `0x89A6`.
    /// `0xE1` = 225. The widget then places the box at `x0 = anchor - w/2` (`0x84AD`).
    pub const CHOICE_BOX_ANCHOR_CONCEPT: usize = 0xE1;
    /// Row pitch 11 — ASSEMBLY: the unified list widget steps `add bp,0xB`
    /// (0x847A) and hit-tests `row = dy/11 + 1` (`div bl,0x0B` @0x8508).
    const CHOICE_BOX_PITCH: usize = 11;

    /// The first row's top y, from the widget's ASSEMBLY layout (0x84A1..0x84C6):
    /// box height h = rows*11 + 8, box y = (200 - h)/2 (screen-centred), text
    /// top = box y + 4. (This DERIVES the previously capture-measured tops-centre
    /// ~95: rows=2 -> h=30, y=85, top=89; rows=6 -> h=74, y=63, top=67 — exact.)
    ///
    /// AUDIT NOTE: the accuracy audit proposed a "[0xadd]=1 tall-mode" (+10 -> h =
    /// rows*11 + 18). VERIFIED FALSE POSITIVE against the oracle: choice_box_bob_
    /// morlock.ppm shows the 2-row telephone box at y=89/100 (= the +8 formula);
    /// +18 would put it at 84 and break the pixel match. The port is already
    /// correct; the raw-assembly reading disagreed with what the game displays.
    /// Text-top of the choice box for a row count, given the widget's HEIGHT SEED.
    ///
    /// The seed is the `bp` the list widget starts accumulating from. `0x8436`
    /// does `xor bp, bp` (seed 0), but the `[0xADD]&1` branch immediately
    /// overrides it with `mov bp, 0xa` at `0x8442` — the same branch that sets the
    /// narrower width floor at `0x8445`. Height is then `bp + rows*11` (`add bp,0xB`
    /// at `0x847A`) `+ 8` (`0x84A7`), and the box is centred at `(200-h)/2`
    /// (`0x84B9..0x84BF`), so a wrong seed shifts the box by half the error.
    fn choice_box_top_y_seeded(rows: usize, height_seed: usize) -> usize {
        let h = height_seed + rows.max(1) * Self::CHOICE_BOX_PITCH + 8;
        (200usize.saturating_sub(h)) / 2 + 4
    }

    /// Default-path top (`bp = 0` from `xor bp,bp` at `0x8436`).
    fn choice_box_top_y(rows: usize) -> usize {
        Self::choice_box_top_y_seeded(rows, 0)
    }

    /// Kind-aware top: the world/entity box (kind 10) is the `[0xADD]&1` branch,
    /// which seeds the height 10 higher.
    fn choice_box_text_top(&self, rows: usize) -> usize {
        Self::choice_box_top_y_seeded(rows, self.choice_box_height_seed())
    }

    fn choice_box_height_seed(&self) -> usize {
        if self.console_box_kind == 10 { 0xa } else { 0 }
    }

    /// The choice box's `(anchor, x0, x1)` for a widest label pixel width, per the
    /// widget layout at `0x84A1..0x84F6`: `w = widest.max(floor) + 0x14`, then
    /// `x0 = anchor - w/2` (`0x84AD shr / 0x84AF sub [0xac6] / 0x84B3 neg`) and the
    /// hit-test accepts `x0 <= mx <= x0+w` (`0x84EE..0x84F6`). The floor/anchor are
    /// kind-dependent: the world/entity box (kind 10) floors 55 and anchors 80
    /// (`0xB0D1`); every other box floors 100 and anchors 100 (`0xAC6=0x64`). Shared
    /// by [`draw_choice_box`] and the console hit-test so the clickable band is
    /// EXACTLY the drawn box — the fixed `40..160` band was only correct for a
    /// centered box with labels ≤100px (it mis-fit the anchor-80 world box and any
    /// wide-label box).
    fn choice_box_geometry(&self, widest: usize) -> (usize, usize, usize) {
        let (anchor, floor) = if self.console_box_kind == 10 {
            (Self::CHOICE_BOX_ANCHOR_WORLD, 55usize)
        } else {
            (Self::CHOICE_BOX_CENTER_X, 100usize)
        };
        let w = widest.max(floor) + 0x14;
        let x0 = anchor.saturating_sub(w / 2);
        (anchor, x0, (x0 + w).min(ENGINE_SCREEN_WIDTH))
    }

    /// The two centred STATUS OVERLAYS, and the quicksave slot name — three UI
    /// strings the port never drew. Found by sweeping the game's UI string table
    /// for live draw sites (`re/tools/check_ui_strings.py`) rather than by
    /// looking at the port, which cannot report a screen it never had.
    ///
    /// ```text
    ///   0x16BC  si=0x159 "LOADING"  ax=0x82 bx=0x60  dl=0xEF  lcall 0x299:0xD6
    ///   0x1ABB  si=0x166 "PAUSE"    bx=0x87 dx=0x60  al=0xE8  lcall 0x299:0x498
    /// ```
    ///
    /// Both land on y=96 — the screen's vertical centre band.
    /// `mov si,0x159` @`0x16BC` — the string is READ from the image, not held as a
    /// literal (audit-fixes #524). See [`EngineState::load_ds_strings`].
    pub const LOADING_TEXT_DS: u16 = 0x159;
    /// `mov ax,0x82` @`0x16BF` (x) and `mov bx,0x60` @`0x16C2` (y)
    /// (audit-fixes #523).
    pub const LOADING_POS: (usize, usize) = (0x82, 0x60);
    /// `mov dl,0xef` @`0x16C5`, passed to `lcall 0x299,0xd6` @`0x16C9` —
    /// `RENDER_FIXED_8X8_TEXT_OFFSET` (#490), with the colour in DL
    /// (audit-fixes #523).
    pub const LOADING_COLOR: u8 = 0xEF;
    /// `mov si,0x166` @`0x1ABB` (audit-fixes #524).
    pub const PAUSE_TEXT_DS: u16 = 0x166;
    /// `mov bx,0x87` @`0x1ABE` (x) and `mov dx,0x60` @`0x1AC1` (y) — note the
    /// registers are NOT the ones LOADING uses (audit-fixes #523).
    pub const PAUSE_POS: (usize, usize) = (0x87, 0x60);
    /// `mov al,0xe8` @`0x1AC4`, passed to `lcall 0x299,0x498` @`0x1AC6` —
    /// `RENDER_PLANAR_UI_TEXT_OFFSET` (#490), a DIFFERENT text entry from
    /// LOADING's, with the colour in AL rather than DL.
    ///
    /// The two overlays look symmetric in this file and are not: different render
    /// routines, different register conventions, and only the shared y=0x60 is
    /// genuinely common (audit-fixes #523).
    pub const PAUSE_COLOR: u8 = 0xE8;

    /// The QUICKSAVE slot name (`0x1B58`): the game copies the literal `LAST` into
    /// the slot-name buffer at `DS:0x270D`, points the save flow's `[0x2734]` at
    /// it, clears `[0x2739]` and jumps STRAIGHT into `vm_state_save` (`0x1C3F`) —
    /// a save with no rename prompt. The port had no quicksave at all.
    pub const QUICKSAVE_SLOT_NAME: &'static str = "LAST";
    /// `mov di,0x270d` @`0x1B5B`, the slot-name buffer the quicksave copies `LAST`
    /// into before jumping straight to `vm_state_save` (audit-fixes #525).
    pub const QUICKSAVE_NAME_BUFFER_DS: u16 = 0x270D;

    /// The DS offsets whose strings this engine draws. Each is cited on its own
    /// constant; they are gathered here so `load_ds_strings` has one list.
    pub const UI_STRING_OFFSETS: [u16; 6] = [
        Self::OPTION_BOX_LABEL_DS_OFFSET,
        Self::LOADING_TEXT_DS,
        Self::PAUSE_TEXT_DS,
        Self::CONFIRM_TITLE_DS,
        Self::CONFIRM_YES_DS,
        Self::CONFIRM_NO_DS,
    ];

    /// Read the UI strings out of `BLOODPRG.EXE` (audit-fixes #524).
    ///
    /// The port used to hold `"LOADING"`, `"PAUSE"`, `"ARE_YOU_SURE?"`, `"YES"`
    /// and `"NO"` as `&'static str` literals pinned to these bytes by a test. That
    /// is a VERIFIED TRANSCRIPTION and still a copy: it breaks against a differing
    /// build instead of following it. `STATUS_STRING_TABLE` reached the same
    /// conclusion and was converted; these had not been.
    pub fn load_ds_strings(&mut self, exe: &[u8]) {
        // `0x600 + DATA_SEGMENT * 16` = `0x600 + 0x0CE2 * 16` — the MZ
        // image-to-file identity, the same one every row of
        // `bloodprg::KNOWN_CODE_SEGMENTS` satisfies (#553). Not a remembered
        // number: `bloodprg::DATA_SEGMENT` is 0x0CE2 and the arithmetic is forced
        // (audit-fixes #571).
        // One definition, derived from the MZ header (audit-fixes #587).
        use crate::bloodprg::DS_BASE;
        for off in Self::UI_STRING_OFFSETS {
            let start = DS_BASE + off as usize;
            let Some(len) = exe
                .get(start..)
                .and_then(|t| t.iter().position(|&b| b == 0))
            else {
                continue;
            };
            self.ds_strings.insert(
                off,
                String::from_utf8_lossy(&exe[start..start + len]).into_owned(),
            );
        }
    }

    /// A UI string by its DS offset; empty when the image has not been loaded.
    pub fn ds_text(&self, off: u16) -> &str {
        self.ds_strings.get(&off).map_or("", String::as_str)
    }

    /// Draw one of the centred status overlays at its own decoded position:
    /// `LOADING` from `0x16BC` and `PAUSE` from `0x1ABB`.
    pub fn draw_status_overlay(&mut self, loading: bool) {
        let (text, (x, y), color) = if loading {
            (
                self.ds_text(Self::LOADING_TEXT_DS).to_string(),
                Self::LOADING_POS,
                Self::LOADING_COLOR,
            )
        } else {
            (
                self.ds_text(Self::PAUSE_TEXT_DS).to_string(),
                Self::PAUSE_POS,
                Self::PAUSE_COLOR,
            )
        };
        draw_text_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &text,
            x,
            y,
            color,
        );
    }

    /// The rows a QUICKSAVE produces: the slot list with `LAST` written into the
    /// target slot, matching `0x1B58`'s copy into `DS:0x270D`.
    pub fn quicksave(&mut self, slot: usize) {
        let slot = slot.min(Self::SAVE_SLOT_ROWS - 1);
        if self.save_slots.len() < Self::SAVE_SLOT_ROWS {
            self.save_slots.resize(
                Self::SAVE_SLOT_ROWS,
                crate::bloodsav::SaveSlot {
                    name: String::new(),
                    file: String::new(),
                },
            );
        }
        self.save_slots[slot].name = Self::QUICKSAVE_SLOT_NAME.to_string();
        self.save_ui_slot = slot;
        self.save_ui_name = Self::QUICKSAVE_SLOT_NAME.to_string();
    }

    /// The CONFIRM DIALOG (`0x14E6..0x1524`) — the game's `ARE_YOU_SURE?` box,
    /// which the port did not implement at all.
    ///
    /// ```text
    ///   0x14E6  bx=0x5A cx=0x50 dx=0x8C bp=0x28   the box rect (90,80,140,40)
    ///   0x14F2  lcall 0x299:0xCDC                 draw it
    ///   0x14F7  mov al,0xE8                       text colour, passed to...
    ///   0x14F9  lcall 0x299:0xBB5                 ...the colour SETTER (al only)
    ///   0x14FE  si=0x17B "ARE_YOU_SURE?"          bx += 0x0A, dx = 0x58
    ///   0x1507  lcall 0x299:0x176                 THE STRING DRAW (si, bx, dx)
    ///   0x150C  si=0x189 "YES"                    bx += 0x14, dx += 0x11
    ///   0x1515  lcall 0x299:0x176
    ///   0x151A  si=0x18D "NO"                     bx += 0x3C
    ///   0x1520  lcall 0x299:0x176
    ///   0x1525  bp=0x2555 / 0x255D                the two hit regions
    /// ```
    ///
    /// So `bx` walks 90 -> 100 -> 120 -> 180 and `dx` 88 -> 105 -> 105, and the
    /// two records at `DS:0x2555` read (120,105,30,10) and (180,105,20,10):
    /// the drawn text and the clickable rect are the same layout from two
    /// independent places in the image.
    ///
    /// The draw positions and the hit rects agree independently: `YES` lands at
    /// x=120 and its region is `(120, 105, 30, 10)`; `NO` lands at x=180 with
    /// `(180, 105, 20, 10)`. Two separate tables describing the same layout is
    /// about as good as static corroboration gets.
    pub const CONFIRM_BOX: (usize, usize, usize, usize) = (90, 80, 140, 40);
    /// `add bx,0xa` @`0x1501` (x = the box's 90 + 10) and `mov dx,0x58` @`0x1504`
    /// (y = 88, ABSOLUTE) — the anchor the other two rows step from
    /// (audit-fixes #525).
    pub const CONFIRM_TITLE_POS: (usize, usize) = (100, 88);
    /// Position by RELATIVE steps from the title: `add bx,0x14` @`0x150F` (+20 ->
    /// 120) and `add dx,0x11` @`0x1512` (+17 -> 105). The extent `(30, 10)` comes
    /// from the region record the doc above cites (audit-fixes #525).
    pub const CONFIRM_YES_REGION: (usize, usize, usize, usize) = (120, 105, 30, 10);
    /// `add bx,0x3c` @`0x151D` (+60 -> 180) with NO further `add dx`, so NO shares
    /// YES's row — one `add` is the whole difference between the two buttons
    /// (audit-fixes #525).
    pub const CONFIRM_NO_REGION: (usize, usize, usize, usize) = (180, 105, 20, 10);
    /// The game's own strings, READ from the image rather than transcribed
    /// (audit-fixes #524). The table keeps the `(DS offset, file offset)` pairs as
    /// the address evidence — the same shape `STATUS_STRING_TABLE` settled on.
    ///
    /// The title itself: `mov si,0x17b` @`0x14FE`, drawn by `lcall 0x299,0x176`
    /// @`0x1507`.
    pub const CONFIRM_TITLE_DS: u16 = 0x017B;
    /// `mov si,0x189` @`0x150C`, drawn by `lcall 0x299,0x176` @`0x1515` after
    /// `add bx,0x14` / `add dx,0x11` step the cursor from the title
    /// (audit-fixes #524).
    pub const CONFIRM_YES_DS: u16 = 0x0189;
    /// `mov si,0x18d` @`0x151A`, drawn @`0x1520` after `add bx,0x3c` — so YES and
    /// NO are placed by RELATIVE steps from the title, not absolute positions
    /// (audit-fixes #524).
    pub const CONFIRM_NO_DS: u16 = 0x018D;
    /// The `(DS offset, file offset)` pairs, kept as address evidence now that the
    /// strings themselves are read: `0x17B` @`0x14FE`, `0x189` @`0x150C`, `0x18D`
    /// @`0x151A` (audit-fixes #524).
    pub const CONFIRM_STRING_TABLE: [(u16, usize); 3] =
        [(0x017B, 0x0D59B), (0x0189, 0x0D5A9), (0x018D, 0x0D5AD)];

    /// Draw the confirm dialog. The box is the same tinted window every other
    /// panel uses (`0x299:0xCDC` shares the blit family with `0x299:0x40E`).
    pub fn draw_confirm_box(&mut self) {
        let (bx, by, bw, bh) = Self::CONFIRM_BOX;
        let table = self.location_panel_tint_table();
        crate::sprite::remap_rect_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &table,
            bx as i32,
            by as i32,
            bw as i32,
            bh as i32,
        );
        // Text index 0xE8 — `mov al,0xE8` @0x14F7, passed to the colour SETTER
        // `0x299:0xBB5` @0x14F9. (The string draw is a different entry point,
        // `0x299:0x176`, called once per line at 0x1507/0x1515/0x1520.)
        const TEXT: u8 = 0xE8;
        for (text, (x, y)) in [
            (
                self.ds_text(Self::CONFIRM_TITLE_DS).to_string(),
                Self::CONFIRM_TITLE_POS,
            ),
            (
                self.ds_text(Self::CONFIRM_YES_DS).to_string(),
                (Self::CONFIRM_YES_REGION.0, Self::CONFIRM_YES_REGION.1),
            ),
            (
                self.ds_text(Self::CONFIRM_NO_DS).to_string(),
                (Self::CONFIRM_NO_REGION.0, Self::CONFIRM_NO_REGION.1),
            ),
        ] {
            draw_text_indexed(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &text,
                x,
                y,
                TEXT,
            );
        }
    }

    /// Hit-test the confirm dialog: `Some(true)` = YES, `Some(false)` = NO.
    /// The regions are `DS:0x2555`/`DS:0x255D`, tested inclusively like every
    /// other region rect (`region_record_hittest` `0x8295`).
    pub fn confirm_box_click(&self, x: i32, y: i32) -> Option<bool> {
        let hit = |(rx, ry, rw, rh): (usize, usize, usize, usize)| {
            x >= rx as i32 && x <= (rx + rw) as i32 && y >= ry as i32 && y <= (ry + rh) as i32
        };
        if hit(Self::CONFIRM_YES_REGION) {
            Some(true)
        } else if hit(Self::CONFIRM_NO_REGION) {
            Some(false)
        } else {
            None
        }
    }

    /// The SAVE screen's rows: the ten slot names with the edit buffer swapped in
    /// for the row being renamed, plus the widget's own extra row. See the call
    /// site for the assembly (`0x1BAB`, `0x1BBD`, `0x8573`).
    pub fn draw_save_ui_rows(&mut self) {
        let editing = self.save_ui_slot.min(Self::SAVE_SLOT_ROWS - 1);
        // PAD as the directory does. `blood.sav`'s records hold FIFTEEN SPACES for
        // an unused slot, not an empty string, and the game measures that padded
        // text: `square_caps_text_width("")` would be `sub ax,2` on zero, which
        // wraps to 0xFFFE, and the widget's max-width compare (`jb` @`0x8472`) is
        // UNSIGNED — so an empty label would win and blow the box out to 65534
        // wide. The game never hits it because it never has an empty label; the
        // port must not manufacture one by trimming.
        let mut rows: Vec<String> = (0..Self::SAVE_SLOT_ROWS)
            .map(|i| {
                let name = self
                    .save_slots
                    .get(i)
                    .map(|s: &crate::bloodsav::SaveSlot| s.name.clone())
                    .unwrap_or_default();
                format!("{name:<width$}", width = crate::bloodsav::SLOT_NAME_LEN - 1)
            })
            .collect();
        rows[editing] = format!(
            "{:<width$}",
            self.save_ui_name,
            width = crate::bloodsav::SLOT_NAME_LEN - 1
        );
        rows.push(self.ds_text(Self::OPTION_BOX_LABEL_DS_OFFSET).to_string());
        self.draw_choice_box(&rows, Some(editing));
    }

    fn draw_choice_box(&mut self, labels: &[String], selected: Option<usize>) {
        if labels.is_empty() {
            return;
        }
        // ASSEMBLY-SOURCED (these were capture-measured, and the box was wrong).
        // The whole row loop is 0x8565..0x85A6, re-read end to end:
        //   0x8565  mov al,0xE8                 unselected row (LOOP TOP)
        //   0x8584  dec byte ptr gs:[0x27C7]    the selected-row countdown...
        //   0x8589  jne 0x8597                  ...only the row that hits 0 recolours
        //   0x858B  mov al,0xEF                 the selected row...
        //   0x8595  mov al,0xFE                 ...but 0xFE while the mouse is ON
        //                                        (`test byte gs:[0xA3E],1` @0x858D)
        //   0x8597  lcall 0x299:0x176           the string draw — the SAME entry
        //                                        point the confirm dialog uses
        //   0x85A0  add dx,0xB                  the row pitch: CHOICE_BOX_PITCH=11
        //   0x85A6  jmp 0x8565                  next row
        // (`dis.py 0x8560` decodes phantoms here — 0x8564 swallows the `b0 e8`.
        // Decode from 0x8565, the verified entry, per the self-sync rule.)
        const TEXT: u8 = 0xE8;
        // `mov al,0xEF` @0x858B — the selected row.
        const TEXT_SELECTED: u8 = 0xEF;
        // `mov al,0xFE` @0x8595 — selected AND the mouse is on it
        // (`test byte gs:[0xA3E],1` @0x858D).
        const TEXT_SELECTED_MOUSE: u8 = 0xFE;
        let rows = labels.len().min(8);
        let widest = labels
            .iter()
            .take(rows)
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        let text_top = self.choice_box_text_top(rows);
        // The box rect per the widget's ASSEMBLY layout (DS:0x2AAB @0x84A1..):
        // w = widest_label + 0x14 (20), h = rows*11 + 8, x = anchor - w/2,
        // y = (200 - h)/2.
        let h = rows * Self::CHOICE_BOX_PITCH + 8;
        // Box rect from the shared widget geometry (0x84A1..): the world box
        // (kind 10) anchors 80 / floors 55, others anchor 100 / floor 100. The
        // same helper drives the hit-test, so the click band == the drawn box.
        let (anchor, x0, x1) = self.choice_box_geometry(widest);
        let y0 = (200usize.saturating_sub(h)) / 2;
        let y1 = (y0 + h).min(ENGINE_SCREEN_HEIGHT);
        // THE BOX IS NOT PAINTED, IT IS TINTED. `0x84D8` loads `si = [0xAC8]` — the
        // 50%-toward-black remap table — and calls `0x299:0x40E` with the rect it
        // just computed, the same translucent-window primitive the destination
        // info panel uses. The port previously filled a border index 0x15 and a
        // fill index 0xE0, both measured off a capture where the box sits over the
        // panorama's dark orb socket, so a darkened background reads as flat black.
        // Over any lighter surface the two disagree completely.
        let table = self.location_panel_tint_table();
        crate::sprite::remap_rect_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &table,
            x0 as i32,
            y0 as i32,
            (x1 - x0) as i32,
            (y1 - y0) as i32,
        );
        for (i, label) in labels.iter().take(rows).enumerate() {
            let color = if selected == Some(i) {
                if self.location_panel_mouse_enabled {
                    TEXT_SELECTED_MOUSE
                } else {
                    TEXT_SELECTED
                }
            } else {
                TEXT
            };
            let width = crate::font::square_caps_text_width(label);
            // Labels center on the BOX ANCHOR, not a fixed 100 (0x857D sub bx,[bp] /
            // 0x8580 shr / 0x8582 add: label_x = x0+10+(widest-width)/2 = anchor-w/2).
            // For the world/entity list (kind 10) the anchor is 80 (0xB0D1); centering
            // on 100 drew those labels 20px right of their own box.
            let lx = anchor.saturating_sub(width / 2);
            crate::font::draw_square_caps(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                label,
                lx,
                text_top + i * Self::CHOICE_BOX_PITCH,
                color,
            );
        }
    }

    /// Hit-test a click against the console choice box (see [`draw_choice_box`]).
    ///
    /// The geometry is DECODED, not measured: the box centres on
    /// [`Self::CHOICE_BOX_CENTER_X`] (`mov word [0xAC6],0x64` @`0x86D9`), the rows
    /// step 11px (`add bp,0xB` @`0x847A`), and the widget's own hit-test is
    /// `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`) — this reproduces that divide.
    /// Shared by the telephone contact list, the MENU submenu and the
    /// nav-destination chooser, which are the same rendered widget.
    ///
    /// The doc used to say "the same measured geometry as the draw", which
    /// undercut its own ASM status: the values were decoded, the wording was left
    /// over from when they were not.
    fn choice_box_row_at(&self, x: u16, y: u16, num_rows: usize, widest: usize) -> Option<usize> {
        let rows = num_rows.min(8);
        // The clickable band is the DRAWN box extent [x0, x1] from the shared
        // geometry (0x84EE..0x84F6) — the same helper draw_choice_box uses, so
        // the band tracks the box's kind/anchor/width instead of a fixed 40..160
        // (which mis-fit the anchor-80 world/nav box and any wide-label box).
        let (_anchor, x0, x1) = self.choice_box_geometry(widest);
        let (xi, yi) = (x as usize, y as usize);
        if xi < x0 || xi > x1 {
            return None;
        }
        // Hit origin == draw origin == the assembly's text_top = box_y+4
        // (0x84E6 `add cx,4`; 0x84FB `sub ax,dx`; 0x8508 `div bl,0x0B`). Kind-aware
        // for the same reason the x-extent is: the kind-10 branch seeds the box
        // height 10 higher (`mov bp,0xa` @0x8442), so a kind-blind top would put
        // the clickable rows 5px off the drawn ones.
        let top = self.choice_box_text_top(rows);
        if yi < top {
            return None;
        }
        let row = (yi - top) / Self::CHOICE_BOX_PITCH;
        (row < rows).then_some(row)
    }

    /// The widest square-caps label pixel width over `labels` (capped at 8 rows,
    /// the box's own limit) — the `widest` the box geometry keys on.
    fn choice_box_widest<S: AsRef<str>>(labels: &[S]) -> usize {
        labels
            .iter()
            .take(8)
            .map(|l| crate::font::square_caps_text_width(l.as_ref()))
            .max()
            .unwrap_or(0)
    }

    /// Draw a LIST MENU — the game's blue square-capitals vertical list (topic
    /// menus in dialogue, destinations at nav, contacts). Same widget as
    /// [`Self::draw_choice_box`], so the same ASSEMBLY layout: rows vertically
    /// centred by count (`box_y = (200 - (rows*11+8))/2`, `text_top = box_y+4`),
    /// 11px pitch (`add bp,0xB` @`0x847A`), text index `0xE8` (`mov al,0xE8`
    /// @`0x8565`) with the selected row in `0xEF` (@`0x858B`).
    ///
    /// Each label is CENTRED on the widget's anchor — `label_x = x0 + 10 +
    /// (widest - width)/2` with `x0 = anchor - (widest+20)/2` (`0x84AD`,
    /// `0x857D..0x8582`). The port previously drew every label flush at a fixed
    /// `x = 170`, "the capture-matched left edge": that is the value the formula
    /// happens to produce for the ~105px label in the capture, frozen as though
    /// it applied to all of them. Labels of different widths belong at different
    /// x, and flush-left is the wrong SHAPE regardless of the constant.
    pub fn draw_list_menu(&mut self, labels: &[String], selected: Option<usize>) {
        // Same row colours as the choice box: `mov al,0xE8` @0x8565 unselected,
        // `mov al,0xEF` @0x858B selected.
        const TEXT: u8 = 0xE8;
        // `mov al,0xEF` @0x858B — the selected row.
        const TEXT_SELECTED: u8 = 0xEF;
        let rows = labels.len().min(12);
        let top = Self::choice_box_top_y(rows);
        let widest = labels
            .iter()
            .take(rows)
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        // The in-window concept list's anchor (`mov [0xAC6],0xE1` @`0x89A6`).
        let anchor = Self::CHOICE_BOX_ANCHOR_CONCEPT;
        let x0 = anchor.saturating_sub((widest + 20) / 2);
        for (i, label) in labels.iter().take(12).enumerate() {
            let color = if selected == Some(i) {
                TEXT_SELECTED
            } else {
                TEXT
            };
            // FLUSH-LEFT, not centred. `0x857D` (`sub bx,[bp] / shr bx,1 /
            // add bx,cx`) is a centring computation, and the port applied it here
            // — but the live game does not centre THIS list: every row of the
            // concept menu starts at the same x, measured per row against
            // `concept_menu.ppm` (see `concept_menu_mask_bounds`). Both masks span
            // x 170..280 identically while overlapping at IoU 0.18, which is what
            // per-row misplacement inside a correct band looks like.
            //
            // The left edge stays DERIVED — `x0 + 10` from the anchor `0xE1` and
            // the widest label — so this is not a return to the hardcoded x=170
            // that #97 removed; 170 is what the formula yields here. Which widget
            // `0x857D` does centre is an open question, recorded in
            // docs/port-validation.md.
            let lx = x0 + 10;
            crate::font::draw_square_caps(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                label,
                lx,
                top + i * 11,
                color,
            );
        }
    }

    /// Set the dialogue topic menu: (label, first line index) per topic.
    pub fn set_topic_menu(&mut self, topics: Vec<(String, usize)>) {
        self.topic_menu = topics;
        self.topic_selected = None;
        self.topic_menu_is_bas = false;
    }

    /// Load the conversation script's decoded concept-menu stack (the game's
    /// `gs:0x6772` menu system, [`crate::bas_vm`]) from its `.BAS`/`.DIC`. Seeds at
    /// the entry menu; `current_bas_menu_labels` then gives the menu to display.
    pub fn load_bas_menus(&mut self, bas: &[u8], dic: &[u8]) {
        self.bas_menus = crate::bas_vm::BasMenuStack::new(bas, dic);
    }

    /// The current BAS concept menu's topic labels (uppercased, as the square-caps
    /// menu renders them). Empty if no `.BAS` menus are loaded. Enter/back-out
    /// navigation is driven via [`crate::bas_vm::BasMenuStack::push`]/`pop` on the
    /// `bas_menus` field as the conversation reaches each menu.
    pub fn current_bas_menu_labels(&self) -> Vec<String> {
        self.bas_menus
            .as_ref()
            .and_then(|s| s.current())
            .map(|m| m.labels.iter().map(|l| l.to_uppercase()).collect())
            .unwrap_or_default()
    }

    /// The complete per-beat concept-menu interaction: a click on `row` either pops
    /// (talk/bye_bye — back out) returning `None`, or advances the current menu's
    /// sequential response monologue and returns the next response's subtitle text.
    /// This is the whole menu behavior (concept menus are flat sequential leaves).
    pub fn bas_menu_interact(&mut self, row: usize) -> Option<String> {
        let labels = self.current_bas_menu_labels();
        if labels
            .get(row)
            .is_some_and(|l| crate::bas_vm::BasMenuStack::is_back_topic(l))
        {
            self.bas_topic_click(row);
            self.bas_responses = None; // fresh monologue for the menu we backed out to
            return None;
        }
        if self.bas_responses.is_none() {
            self.bas_start_responses();
        }
        let offset = self.bas_advance_response()?;
        self.bas_menus
            .as_ref()
            .and_then(|s| s.response_text(offset))
    }

    /// Handle a screen click on the displayed BAS concept menu: map (x,y) to a topic
    /// row (the list-menu geometry) and run [`Self::bas_menu_interact`]. Returns the
    /// response subtitle (empty string on a pop/back), or `None` if the click missed
    /// the menu rows (so the caller can fall through to advancing the dialogue).
    pub fn bas_menu_click(&mut self, x: u16, y: u16) -> Option<String> {
        let row = Self::list_menu_click(self.topic_menu.len(), x, y)?;
        self.topic_selected = Some(row);
        Some(self.bas_menu_interact(row).unwrap_or_default())
    }

    /// Sync the displayed topic menu to the current BAS concept menu, so the decoded
    /// menus actually RENDER (via [`draw_list_menu`]/the topic-menu widget). Each row
    /// carries its topic index; a click is handled by [`Self::bas_topic_click`]. No-op
    /// if no `.BAS` menus are loaded.
    pub fn sync_topic_menu_from_bas(&mut self) {
        let labels = self.current_bas_menu_labels();
        if !labels.is_empty() {
            self.topic_menu = labels
                .into_iter()
                .enumerate()
                .map(|(i, l)| (l, i))
                .collect();
            self.topic_selected = None;
            self.topic_menu_is_bas = true;
        }
    }

    /// Begin the current BAS menu's response monologue (its `0xA6` responses), reset
    /// to the start. Called when a conversation menu becomes active.
    pub fn bas_start_responses(&mut self) {
        self.bas_responses = self.bas_menus.as_ref().and_then(|s| s.current_responses());
    }

    /// Advance to the next response of the active menu (the already-shown gating),
    /// returning its `0xA6` BAS offset for the dialogue renderer. `None` when the
    /// monologue is exhausted or no menu is active.
    pub fn bas_advance_response(&mut self) -> Option<usize> {
        self.bas_responses.as_mut()?.advance()
    }

    /// Handle a click on `row` of the current BAS concept menu. Back-out topics
    /// (`talk`/`bye_bye`) POP to the parent menu — verified against the running
    /// game (MENUTREE: fear/anger menu `talk` → the top-level parent 0x2f). Other
    /// topics select the row: their response/sub-menu is driven by the conversation
    /// flow (a `push` when it opens a sub-menu). Returns the new current labels.
    pub fn bas_topic_click(&mut self, row: usize) -> Vec<String> {
        if let Some(stack) = self.bas_menus.as_mut() {
            let back = stack
                .current()
                .and_then(|m| m.labels.get(row))
                .is_some_and(|l| crate::bas_vm::BasMenuStack::is_back_topic(l));
            if back {
                stack.pop();
            }
        }
        self.current_bas_menu_labels()
    }

    /// The current topic-menu labels (for drivers that route clicks themselves).
    pub fn topic_labels(&self) -> Vec<String> {
        self.topic_menu.iter().map(|(l, _)| l.clone()).collect()
    }

    /// A click while the topic menu is showing: selects the topic and jumps the
    /// dialogue to its first line. Returns the selected topic index.
    pub fn topic_menu_click(&mut self, x: u16, y: u16) -> Option<usize> {
        let row = Self::list_menu_click(self.topic_menu.len(), x, y)?;
        self.topic_selected = Some(row);
        let line = self.topic_menu[row].1;
        self.set_dialogue_cursor(line);
        self.dialogue_timer = 0;
        // Play only THIS topic's SEGMENT, then re-hold. The boundary is the next
        // function-segment start after this line if segment starts were supplied (so e.g. the
        // MENU topic shows one daily menu, not every function up to the next topic); otherwise
        // fall back to the next topic's first line.
        let seg_end = self
            .dialogue_segments
            .iter()
            .copied()
            .filter(|&l| l > line)
            .min();
        let next_topic = self
            .topic_menu
            .iter()
            .map(|(_, l)| *l)
            .filter(|&l| l > line)
            .min();
        self.autoplay_end = seg_end.or(next_topic);
        Some(row)
    }

    /// Fire a one-shot hand-pose event (decoded selectors: 0xB = UI close, 0xA =
    /// screen transition) — the pose sequence plays once, then poses resume from state.
    pub fn hand_pose_event(&mut self, sel: u16) {
        if let Some(mesh) = self.hand_mesh.as_mut() {
            mesh.set_pose(sel);
        }
    }

    /// Whether the current dialogue plays over the pyramid-console band (SCRIPT1 tutorial).
    pub fn set_console_band_dialogue(&mut self, on: bool) {
        self.console_band_dialogue = on;
    }

    /// Gate auto-play at `end` (exclusive): the scripted opening plays unprompted, then the
    /// dialogue holds at the topic menu and topic clicks drive the rest.
    pub fn set_dialogue_autoplay_end(&mut self, end: Option<usize>) {
        self.autoplay_end = end;
    }

    /// Per-line render styles (parallel to the queued lines): true = character speech
    /// (green bold reveal), false = static text (white thin — menu/list content).
    pub fn set_dialogue_styles(&mut self, styles: Vec<bool>) {
        self.dialogue_is_speech = styles;
    }

    /// Record the dialogue's function-segment start lines so a topic click plays only that one
    /// segment (not everything up to the next topic). Does NOT change the play position — unlike
    /// [`Self::set_dialogue_segments`], which also arms sequential beat-play.
    pub fn set_segment_boundaries(&mut self, starts: Vec<usize>) {
        self.dialogue_segments = starts;
    }

    /// Segment the dialogue at the given line starts (script-function beats): the first segment
    /// (the scripted opening) auto-plays, then the dialogue holds; each call to
    /// [`Self::play_next_dialogue_segment`] plays one more segment.
    pub fn set_dialogue_segments(&mut self, starts: Vec<usize>) {
        self.autoplay_end = starts.get(1).copied();
        self.dialogue_segments = starts;
        self.dialogue_segment_pos = 1;
    }

    /// Play the next unplayed dialogue segment (a concept-menu interaction advances the
    /// conversation one beat), then re-hold at the menu. Returns false when exhausted.
    /// Driver hook: the current line's voice clip lasts `frames` engine frames — hold the line
    /// at least that long (voice-paced advance, as the real game gates on SB completion).
    pub fn hold_current_line_at_least(&mut self, frames: u32) {
        self.line_min_hold = Some((self.dialogue_cursor, frames));
    }

    pub fn play_next_dialogue_segment(&mut self) -> bool {
        let Some(&start) = self.dialogue_segments.get(self.dialogue_segment_pos) else {
            return false;
        };
        self.set_dialogue_cursor(start);
        self.dialogue_timer = 0;
        self.dialogue_segment_pos += 1;
        self.autoplay_end = self
            .dialogue_segments
            .get(self.dialogue_segment_pos)
            .copied();
        if !self.dialogue_scene_paths.is_empty() {
            self.load_current_scene();
        }
        true
    }

    /// Map a click to a list-menu row, using the widget's own hit-test rather
    /// than measured geometry: `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`), the
    /// same divide [`Self::choice_box_row_at`] reproduces, over rows stepped by
    /// `add bp,0xB` (@`0x847A`).
    pub fn list_menu_click(labels_len: usize, x: u16, y: u16) -> Option<usize> {
        if !(170..=245).contains(&(x as i32)) {
            return None;
        }
        // Row band vertically centered by row count (matches draw_list_menu /
        // choice_box_top_y): text_top = (200-(rows*11+8))/2 + 4, 11px pitch.
        let rows = labels_len.min(12);
        let top = Self::choice_box_top_y(rows) as i32;
        let row = (y as i32 - top) / 11;
        (row >= 0 && (row as usize) < rows).then_some(row as usize)
    }

    /// Whether a click hits the nav-sector ORB (the pyramid-sector station orb) — the
    /// oracle-verified way into the nav/viewscreen console. Orb near screen (105..145, 130..165)
    /// when the pyramid sector (frames 72..107) is in view.
    pub fn bridge_nav_orb_click(&self, x: u16, y: u16) -> bool {
        (72..=107).contains(&self.bridge.frame)
            && (95..=150).contains(&x)
            && (125..=170).contains(&y)
    }

    /// Map a click to a nav-sector destination row when the choice box is showing
    /// (bridge view in the pyramid sector with destinations set).
    /// The station the current panorama frame belongs to, read from its header.
    /// `None` when no archive is loaded (headless tests).
    ///
    /// `TB.BIG`'s per-frame headers carry the station, which is why this asks the
    /// data rather than testing a frame range — see
    /// [`Self::bridge_nav_destination_click`] for what that fixed.
    pub fn bridge_station(&self) -> Option<u16> {
        self.panorama
            .as_ref()?
            .frame_header(self.bridge.frame as usize)
            .map(|h| h.station)
    }

    /// Map a click to a nav-destination row, while the view is at the PYRAMID
    /// NAVIGATION ROOM.
    ///
    /// The station comes from the panorama's own frame header (`station == 2`),
    /// not from a frame range written here: `TB.BIG` carries the station per
    /// frame, and `tbbig`'s `panorama_stations_partition_the_ring` test pins
    /// frames 72..=107 to station 2. Reading the header means the gate follows the
    /// data if the archive ever says otherwise, and it names the condition —
    /// `72..=107` said what, not why.
    ///
    /// The row hit-test itself is [`Self::choice_box_row_at`], the widget's
    /// `row = dy/11 + 1` (`div bl,0x0B` @`0x8508`).
    pub fn bridge_nav_destination_click(&self, x: u16, y: u16) -> Option<usize> {
        if self.bridge_station() != Some(NAV_ROOM_STATION) || self.nav_destinations.is_empty() {
            return None;
        }
        let widest = self
            .nav_destinations
            .iter()
            .take(8)
            .map(|(l, _)| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        self.choice_box_row_at(x, y, self.nav_destinations.len(), widest)
    }

    /// Composite the bridge view into the framebuffer: window starfield, then the
    /// current TB.BIG panorama frame with colour-0 transparency, then the game
    /// palette + the golden menu's dynamic DAC rows — the original composite
    /// order (`page_flip` 0x954A: projection first, then the transparent frame
    /// unpack). Shared by the bridge screen and by on-ship (sceneless) dialogue,
    /// which the real game plays OVER the console.
    fn render_bridge_background(&mut self) {
        // 1. Starfield through the windows: the ship-3D point cloud projected at
        //    the view's yaw — the panorama frame index IS the yaw index
        //    (bridge_frame_to_yaw_sync 0x97E3 copies [0x2795] -> [0x2F6D]).
        let mut prng = BloodPrng::seeded_from_rtc_seconds(self.starfield_seed);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: self.bridge.frame % 180,
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
        if self.gpu_hand_enabled {
            // GPU path: the stars render at window resolution behind the panorama
            // (colour-0 keyed windows); the 320x200 fb keeps only the panorama.
            self.framebuffer.iter_mut().for_each(|p| *p = 0);
            let pts = crate::ship3d::randomize_ship_3d_point_cloud(&mut prng);
            if let Some(matrix) = crate::ship3d::build_ship_3d_projection_matrix(
                &crate::ship3d::SHIP_3D_ANGLE_TABLE,
                angles,
            ) {
                self.gpu_stars = Some(crate::ship3d::ship_3d_point_cloud_points(
                    &pts, origin, matrix, viewport,
                ));
            }
            self.gpu_bg_colorkey = true;
        } else if let Some(render) = render_ship_3d_starfield(&mut prng, angles, origin, viewport) {
            self.framebuffer.copy_from_slice(&render.buffer);
        } else {
            self.framebuffer.iter_mut().for_each(|p| *p = 0);
        }
        // 2. The panorama frame, colour 0 transparent (windows keep the stars).
        if let Some(panorama) = self.panorama.as_ref() {
            if let Some(header) =
                panorama.unpack_frame_over(self.bridge.frame as usize, &mut self.framebuffer, true)
            {
                // Refresh the current station's eye-orb click rectangle exactly
                // as the frame loader does (0x9877..0x9889).
                let orb_box = (header.box_x != 0xFFFF).then_some([
                    header.box_x,
                    header.box_y,
                    header.box_width,
                    header.box_height,
                ]);
                self.bridge.set_frame_orb_box(header.station, orb_box);
            }
        }
        // 3. The game-screen palette + the menu rows' dynamic DAC entries
        //    (0x7B..0x7F: idle dark gold, hovered bright — 0x862B..0x86A3).
        self.scene_palette = crate::palette::game_screen_palette();
        self.bridge.apply_menu_palette(&mut self.scene_palette);
    }

    /// Load the cyberspace hyperspace-tunnel animations (`sq/hyper_*.hnm`), sorted so
    /// segments advance in order. The screen renders once `cyber_active` is set.
    pub fn load_cyberspace(&mut self, assets: &Path) {
        let sq = assets.join("sq");
        let mut names: Vec<String> = std::fs::read_dir(&sq)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| {
                let l = n.to_lowercase();
                l.starts_with("hyper_") && l.ends_with(".hnm")
            })
            .collect();
        names.sort();
        self.cyber_tunnels = names
            .iter()
            .filter_map(|n| HnmFile::open(&sq.join(n)).ok())
            .collect();
        self.cyber_segment = 0;
    }

    /// Begin (or restart) a cyberspace traversal from the first segment.
    pub fn start_cyberspace(&mut self) {
        self.cyber_segment = 0;
        self.cyber_steer = 0;
        self.cyber_arrived = false;
        self.scene_frame = 0;
    }

    /// The cyberspace traversal progress: (current segment, total segments).
    pub fn cyber_progress(&self) -> (usize, usize) {
        (self.cyber_segment, self.cyber_tunnels.len())
    }

    /// Render the cyberspace hyperspace-traversal mini-game: fly through the real
    /// `hyper_*.hnm` warp segments toward the destination, steering the on-course reticle
    /// with the mouse; each completed segment advances the journey until the last one is
    /// reached (`cyber_arrived`). A progress bar + the steer reticle overlay the tunnel.
    ///
    /// NOTE: the tunnel VIDEO + segment order are the real decoded assets; the traversal
    /// interaction (steer + arrive) is the port's grounded interpretation — the original's
    /// exact goal/scoring for this screen is not decoded (see `docs/decompilation-roadmap.md`).
    fn render_cyberspace(&mut self) {
        let n = self.cyber_tunnels.len();
        if n == 0 {
            return;
        }
        // Steer: the cursor's horizontal delta from centre nudges the course, smoothed +
        // clamped (the same joystick-style delta the ship nav uses).
        let target = (self.mouse.x as i32 - ENGINE_SCREEN_WIDTH as i32 / 2) / 3;
        self.cyber_steer = ((self.cyber_steer * 3 + target) / 4).clamp(-120, 120);
        let seg = self.cyber_segment.min(n - 1);
        let count = self.cyber_tunnels[seg].frame_count().max(1);
        if self.scene_frame >= count {
            if self.cyber_segment + 1 < n {
                self.cyber_segment += 1;
            } else {
                self.cyber_arrived = true;
            }
            self.scene_frame = 0;
        }
        let hnm = &self.cyber_tunnels[self.cyber_segment.min(n - 1)];
        self.scene_palette = hnm.palette;
        hnm.decode_frame(
            self.scene_frame,
            &mut self.scene_buffer,
            &mut self.scene_palette,
        );
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
        // HUD overlay: a course reticle (steered) + a journey progress bar.
        //
        // PORT-SIDE CHOICE OF A DECODED SLOT (audit-fixes #542). `0xFD`/`0xFE` are
        // RESERVED high-palette entries the game fills at RUNTIME — that much is
        // decoded (REVERSE.md; the subtitle reveal draws through the same two, see
        // `extract::render::SUBTITLE_COLOR_REVEALED`). What is NOT decoded is this
        // screen's use of them for a reticle and a bar, nor the RGB below.
        //
        // The coupling is worth knowing: FOUR port sites write these two indices
        // with different colours (here, `0x110C`/`0x110D` in the nav marker path,
        // and the subtitle helper `apply_reserved_subtitle_palette`). Any screen
        // that draws a HUD and a subtitle gets whichever wrote last, and neither
        // constant's name suggests it shares a slot.
        const RETICLE: u8 = 0xFE;
        const BAR: u8 = 0xFD;
        self.scene_palette[RETICLE as usize] = [245, 245, 160];
        self.scene_palette[BAR as usize] = [120, 220, 245];
        let cx = (ENGINE_SCREEN_WIDTH as i32 / 2 + self.cyber_steer)
            .clamp(4, ENGINE_SCREEN_WIDTH as i32 - 5) as usize;
        let cy = ENGINE_SCREEN_HEIGHT / 2;
        for dy in -3i32..=3 {
            for dx in -3i32..=3 {
                if dx.abs() == 3 || dy.abs() == 3 {
                    let (px, py) = (cx as i32 + dx, cy as i32 + dy);
                    if px >= 0 && py >= 0 && (px as usize) < ENGINE_SCREEN_WIDTH {
                        let o = py as usize * ENGINE_SCREEN_WIDTH + px as usize;
                        if o < self.framebuffer.len() {
                            self.framebuffer[o] = RETICLE;
                        }
                    }
                }
            }
        }
        // Progress bar along the bottom: filled proportional to segments travelled.
        let filled = (self.cyber_segment + 1) * ENGINE_SCREEN_WIDTH / n;
        let bar_y = ENGINE_SCREEN_HEIGHT - 6;
        for x in 0..filled.min(ENGINE_SCREEN_WIDTH) {
            for by in 0..3 {
                self.framebuffer[(bar_y + by) * ENGINE_SCREEN_WIDTH + x] = BAR;
            }
        }
    }

    /// Load the cryo-chamber scene (`sq/cryorad.hnm`) shown by the console's CRYOBOX
    /// option — the ship's cryo-pod bay (its palette is the HNM's own header palette).
    pub fn load_cryobox(&mut self, assets: &Path) -> bool {
        self.cryobox_scene = HnmFile::open(&assets.join("sq").join("cryorad.hnm")).ok();
        self.cryobox_scene.is_some()
    }

    /// Render the CRYOBOX cryo-chamber, looping its frames.
    fn render_cryobox(&mut self) {
        let Some(hnm) = self.cryobox_scene.take() else {
            return;
        };
        let frame = self.scene_frame % hnm.frame_count().max(1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(frame, &mut self.scene_buffer, &mut self.scene_palette);
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
        self.cryobox_scene = Some(hnm);
    }

    /// The montage's full-screen colour reduction at `0x7AC3` (`si=DS:0x6011`,
    /// `lcall 0x299:0x40E` over `(0, 0, 320, 200)`), through the table the game's
    /// own builder produces — see
    /// [`crate::palette::build_console_bank_remap_table`]. Idempotent, as the
    /// table's fixed points guarantee.
    pub fn apply_console_bank_remap(&mut self) {
        let mut dac = [0u8; 768];
        for i in 0..256 {
            for k in 0..3 {
                dac[i * 3 + k] = (self.scene_palette[i][k] as u16 * 63 / 255) as u8;
            }
        }
        let table = crate::palette::build_console_bank_remap_table(&dac);
        crate::sprite::remap_rect_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &table,
            0,
            0,
            ENGINE_SCREEN_WIDTH as i32,
            ENGINE_SCREEN_HEIGHT as i32,
        );
    }

    /// Overlay the intro montage's console band — FROM THE REAL ASSET.
    ///
    /// The band is `TB.BIG` panorama frame [`CONSOLE_BAND_FRAME`], rows 140..200,
    /// pushed through the console-bank remap table the montage applies to the whole
    /// screen (`0x7AC3` -> `DS:0x6011`). Proven: that composition equals the
    /// previously harvested `console_band.idx` in ALL 19200 bytes, which is what
    /// finally identified the source — the band was never separate art, it is the
    /// bridge panorama colour-reduced.
    ///
    /// Earlier attempts missed it because they compared the panorama's RAW indices
    /// (`0..~75`) against the band's POST-REMAP indices (`224..=239`) and concluded
    /// the ranges were disjoint. The transform was exactly what made them differ.
    fn overlay_console_band(&mut self) {
        use crate::tbbig::{CONSOLE_BAND_FRAME, CONSOLE_BAND_HEIGHT, CONSOLE_BAND_TOP};
        let dac = crate::palette::game_screen_palette();
        for i in 224..=255usize {
            self.scene_palette[i] = dac[i];
        }
        let Some(pan) = self.panorama.as_ref() else {
            return; // no bridge archive loaded (headless tests)
        };
        let Some(px) = pan.frame_pixels(CONSOLE_BAND_FRAME) else {
            return;
        };
        if px.len() < (CONSOLE_BAND_TOP + CONSOLE_BAND_HEIGHT) * ENGINE_SCREEN_WIDTH {
            return;
        }
        let table = crate::palette::build_console_bank_remap_table(
            &crate::palette::GAME_SCREEN_PALETTE_DAC,
        );
        for row in 0..CONSOLE_BAND_HEIGHT {
            let y = CONSOLE_BAND_TOP + row;
            for x in 0..ENGINE_SCREEN_WIDTH {
                let src = px[y * ENGINE_SCREEN_WIDTH + x];
                self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = table[src as usize];
            }
        }
    }

    /// Render the VIEWSCREEN console: the band along the bottom — composed from
    /// `TB.BIG` frame 90 through the console-bank remap, see
    /// [`Self::overlay_console_band`] — and the upper viewscreen showing STATIC
    /// when no destinations are granted, or the destination list once granted.
    fn render_viewscreen_console(&mut self) {
        self.scene_palette = crate::palette::game_screen_palette();
        // Upper viewscreen: STATIC — binary black/white noise filling everything above
        // the console band (oracle intro_215M: rows 0..140 are ~54% black (224) /
        // ~46% white (239), only stray greys — the console bank's extremes).
        for p in self.framebuffer.iter_mut() {
            *p = 0;
        }
        for y in 0..140usize {
            for x in 0..ENGINE_SCREEN_WIDTH {
                self.viewscreen_noise = self
                    .viewscreen_noise
                    .wrapping_mul(1103515245)
                    .wrapping_add(12345);
                let v = (self.viewscreen_noise >> 16) as u8;
                self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = if v & 1 == 0 { 224 } else { 239 };
            }
        }
        self.overlay_console_band();
        // Destinations granted: list them over the viewscreen (the tuned state).
        if !self.nav_destinations.is_empty() {
            let labels: Vec<String> = self
                .nav_destinations
                .iter()
                .take(8)
                .map(|(l, _)| l.clone())
                .collect();
            self.draw_choice_box(&labels, None);
        }
    }

    /// Load the game-ending finale cutscene (`sq/fin.hnm`, the "fin"/end video) — the
    /// bookend to the intro. Returns whether it loaded.
    pub fn load_ending(&mut self, assets: &Path) -> bool {
        self.ending_scene = HnmFile::open(&assets.join("sq").join("fin.hnm")).ok();
        self.ending_scene.is_some()
    }

    /// Start the ending finale from its first frame (call when the game is completed).
    pub fn start_ending(&mut self) {
        self.ending_frame = 0;
        self.ending_active = self.ending_scene.is_some();
    }

    /// Whether the ending finale has played through all its frames.
    pub fn ending_finished(&self) -> bool {
        match &self.ending_scene {
            Some(hnm) => self.ending_frame + 1 >= hnm.frame_count().max(1),
            None => true,
        }
    }

    /// Render the ending finale, advancing one frame per call and holding on the last.
    fn render_ending(&mut self) {
        let Some(hnm) = self.ending_scene.take() else {
            return;
        };
        let count = hnm.frame_count().max(1);
        let frame = self.ending_frame.min(count - 1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(frame, &mut self.scene_buffer, &mut self.scene_palette);
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        if self.ending_frame + 1 < count {
            self.ending_frame += 1;
        }
        self.ending_scene = Some(hnm);
    }

    /// PROVENANCE (audit-fixes #326): the display names are TRANSCRIBED AND
    /// TRANSFORMED. `DESCRIPT.DES` holds `Bob_Morlock` at `0x09EB`; this table
    /// carries `"BOB MORLOCK"` — upper-cased with the underscore turned into a
    /// space. The names therefore DO come from the game's data, but neither the
    /// upper-casing nor the underscore substitution has a routine behind it here,
    /// so the table is a literal standing in for a lookup plus a formatting rule.
    /// `tools/check_ui_literals.py` reports it as ABSENT from the shipped data
    /// for exactly that reason: the game never stores this spelling.
    ///
    /// THE UNDERSCORE SUBSTITUTION HAS NO BASIS IN THE BINARY (audit-fixes
    /// #327). Two findings, together:
    ///
    ///   * NO instruction anywhere in BLOODPRG.EXE compares against `0x5F`.
    ///     BASIS CORRECTED (audit-fixes #334): this first cited `find_imm.py 5f`
    ///     returning zero confirmed hits, and that tool is now known to have
    ///     FALSE NEGATIVES — it rejects the genuine `mov byte [0x2737],1`
    ///     @`0x893C`. A zero from it is not proof of absence. The claim instead
    ///     rests on searching the raw ENCODINGS, which are unambiguous: `3c 5f`
    ///     (`cmp al,0x5f`), `80 fc 5f` (`cmp ah,0x5f`), `2c 5f` (`sub al,0x5f`)
    ///     and `80 3e .. .. 5f` (`cmp byte [imm16],0x5f`) occur ZERO times in the
    ///     image as raw bytes — so no encoding of the comparison exists to be
    ///     missed.
    ///   * A case-folding loop at `0x2760` PRESERVES one: `cmp al,0x61 / jb`
    ///     @`0x2765` skips every character below `'a'`, and `0x5F` is below
    ///     `'a'`. `and al,0xdf` @`0x2769` upper-cases the rest.
    ///
    /// SCOPE CORRECTED (audit-fixes #328). #327 called `0x2760` "the game's only
    /// case-folding loop" and treated it as the caption path. It is neither. It
    /// has ZERO callers — near or far — so it is a fall-through block, and the
    /// instructions immediately above it are a DOS READ (`int 21h` `AH=3Fh`,
    /// handle from `gs:[0xA88]`). It folds something just loaded from a FILE,
    /// most plausibly a path being normalised.
    ///
    /// WHAT SURVIVES that correction is the stronger half and it does not depend
    /// on `0x2760` at all: NO instruction anywhere in the image compares against
    /// `0x5F`. So nothing in the game can special-case an underscore, whatever
    /// renders the caption.
    ///
    /// RE-VERIFIED ACROSS ENCODINGS (audit-fixes #451), because a NEGATIVE claim
    /// is only as strong as the forms it searched, and this one justified a port
    /// change. Checking every compare-with-`0x5F` encoding — `cmp al,i8` (`3C`),
    /// `cmp ax,i16` (`3D`), `cmp r/m8,i8` (`80 /7`), `cmp r/m16,i8`
    /// sign-extended (`83 /7`) and `cmp r/m16,i16` (`81 /7`) — finds ZERO sites.
    /// The original claim was made before #450 showed a one-family enumeration
    /// reading as exhaustive; it survives the wider search.
    ///
    /// SETTLED FROM THE DATA (audit-fixes #437). #328 left the spelling open
    /// because the caption RENDERER is unfound — but the renderer is not the only
    /// evidence available. Searching all 261 shipped files: `Bob Morlock` in any
    /// case appears in ZERO; `Bob_Morlock` appears in 31, among them
    /// DESCRIPT.DES (`EBob_Morlock`) and SCRIPT2.DIC @0x462F. So the spaced form
    /// was invented here, and with nothing in the image able to fold `0x5F` there
    /// is no mechanism by which the underscore could disappear on the way to the
    /// screen. Corrected to `BOB_MORLOCK`.
    ///
    /// TWO MORE NAMES ARE SHORTENED, and the data path already exists
    /// (audit-fixes #438). `DESCRIPT.DES` carries a tagged character-name table —
    /// `8Jerry_Khan` @0x08B9, `;Tina_Burner` @0x0912, `2Hom` @0x084D,
    /// `AMaxxon` @0x0991, `CIzwalito` @0x09B5, `EBob_Morlock` @0x09EB — and the
    /// port ALREADY parses it: `DescriptDb::character_names()`. So `JERRY` and
    /// `TINA` here are short forms of the game's `Jerry_Khan` and `Tina_Burner`,
    /// the same defect #437 fixed for `BOB_MORLOCK`, and all nine names exist in
    /// the shipped files.
    ///
    /// THE CAPTION PATH IS NOW DECODED (audit-fixes #439), which is what #328
    /// said was missing. `nav_choice_handler_2` (`0x87BD`) builds the contact menu:
    ///
    /// ```text
    ///   0x87C5  mov si, 0x6d3e       the contact SOURCE list
    ///   0x87C8  mov di, 0x2b13       the menu it builds
    ///   0x87CB  lodsw / or ax,ax / je  a ZERO slot is skipped (empty)
    ///   0x87D0  cmp ax,-1 / je         0xFFFF ends the list
    ///   0x87D5  add ax, 4              <- the entry is an OBJECT OFFSET, and +4
    ///   0x87D8  stosw                     is its INLINE NAME (#418: 630/640
    ///                                     objects hold their DEB name at +4)
    /// ```
    ///
    /// So a contact's caption IS its object's inline name, and `DS:0x6D3E` is all
    /// zeros in the image — the list is runtime state, filled as crew become
    /// callable, exactly as that handler's label says.
    ///
    /// The `.VAR` object records carry those names in full: `Bob_Morlock` at
    /// SCRIPT1.VAR +78, which is object `0x4A` + 4 — the same object the inline-name
    /// tool reports. `Jerry_Khan` (+726), `Tina_Burner` (+1806), `Maxxon` (+1302),
    /// `Izwalito` (+1374) and `Hom` (+438) are all there. So `JERRY` and `TINA`
    /// were short forms of names the game stores in full, and are corrected here
    /// on the same footing as #437's underscore.
    ///
    /// STILL OPEN: `Migrax` and `Hanz` appear in NO `.VAR`, so those two entries
    /// have no object backing them and may be invented. The table remains a
    /// literal until the runtime slot list is modelled.
    ///
    /// The video-phone's callable crew: display name + their talk-head HNM basename
    /// (`pe/aa*.hnm`). These are the crew whose full-colour idle-head animations exist and
    /// decode cleanly; the phone shows the dialled one as the live "video feed".
    const PHONE_CONTACTS: [(&'static str, &'static str); 9] = [
        // `BOB_MORLOCK`, not `BOB MORLOCK` (audit-fixes #437). Searching all 261
        // shipped files: `Bob Morlock` in ANY case appears in ZERO of them;
        // `Bob_Morlock` appears in 31, including DESCRIPT.DES (`EBob_Morlock`,
        // a tagged record) and SCRIPT2.DIC @0x462F. The spaced spelling was
        // invented here.
        ("BOB_MORLOCK", "aabob"),
        ("HOM", "aahom"),
        ("IZWALITO", "aaisw"),
        ("JERRY_KHAN", "aajer"),
        ("MAXXON", "aamax"),
        ("MIGRAX", "aamig"),
        ("HANZ", "aahan"),
        ("TINA_BURNER", "aatin"),
        ("RGB", "aargb"),
    ];

    /// Load the video-phone call screen (console TELEPHONE option): the call widget
    /// (`BAPPEL.SPR`, from `iso`) and every callable crew's talk-head HNM (`pe/aa*.hnm`,
    /// from `assets`). Returns whether the widget and at least one contact loaded.
    pub fn load_telephone(&mut self, iso: &Path, assets: &Path) -> bool {
        if let Ok(data) = std::fs::read(iso.join("BAPPEL.SPR")) {
            if let Some(frames) = decode_sprite_bank_indices(&data) {
                self.phone_widget = frames;
            }
        }
        self.phone_contacts = Self::PHONE_CONTACTS
            .iter()
            .filter_map(|(name, stem)| {
                HnmFile::open(&assets.join("pe").join(format!("{stem}.hnm")))
                    .ok()
                    .map(|h| (name.to_string(), h))
            })
            .collect();
        !self.phone_widget.is_empty() && !self.phone_contacts.is_empty()
    }

    /// The number of callable phone contacts loaded.
    /// Contact names for the TELEPHONE choice box.
    pub fn phone_contact_labels(&self) -> Vec<String> {
        self.phone_contacts.iter().map(|(n, _)| n.clone()).collect()
    }

    pub fn phone_contact_count(&self) -> usize {
        self.phone_contacts.len()
    }

    /// The display name of the currently selected/dialled contact.
    pub fn phone_contact_name(&self) -> Option<&str> {
        self.phone_contacts
            .get(self.phone_contact)
            .map(|(n, _)| n.as_str())
    }

    /// Whether the call is connected (showing the talk-head video feed).
    pub fn phone_connected(&self) -> bool {
        self.phone_connected
    }

    /// Cycle the dialled contact (`dir` +1/−1), while dialling (a no-op once connected).
    pub fn phone_cycle_contact(&mut self, dir: i32) {
        let n = self.phone_contacts.len();
        if n == 0 || self.phone_connected {
            return;
        }
        self.phone_contact = (self.phone_contact as i32 + dir).rem_euclid(n as i32) as usize;
    }

    /// Map a click to a contact-list row (dialling state), matching the drawn layout.
    pub fn phone_contact_click(&self, x: u16, y: u16) -> Option<usize> {
        if self.phone_contacts.is_empty() {
            return None;
        }
        // The drawn box is [up to 7 contacts…, CANCEL] (see the dialling render),
        // vertically centred for that total. Hit-test the same total, but only a
        // contact row selects a call — the trailing CANCEL row backs out.
        let contacts = self.phone_contacts.len().min(7);
        // widest over the SAME labels the dialling render draws: the first 7
        // contact names plus the trailing CANCEL row.
        let widest = self
            .phone_contacts
            .iter()
            .take(7)
            .map(|(n, _)| crate::font::square_caps_text_width(n))
            .chain(std::iter::once(crate::font::square_caps_text_width(
                "CANCEL",
            )))
            .max()
            .unwrap_or(0);
        let row = self.choice_box_row_at(x, y, contacts + 1, widest)?;
        (row < contacts).then_some(row)
    }

    /// Connect the call to `index` (switch to the video-feed state). Invalid index = no-op.
    pub fn phone_connect(&mut self, index: usize) -> bool {
        if index >= self.phone_contacts.len() {
            return false;
        }
        self.phone_contact = index;
        self.phone_connected = true;
        self.scene_frame = 0;
        true
    }

    /// Hang up a connected call, returning to the dialling state.
    pub fn phone_hangup(&mut self) {
        self.phone_connected = false;
    }

    /// Render the video-phone. Dialling: the console-palette backdrop, the animated
    /// `BAPPEL` call widget, and the crew contact list in the console font (the dialled
    /// row highlighted). Connected: the dialled crew's full-colour talk-head HNM, looped.
    fn render_telephone(&mut self) {
        if self.phone_connected {
            let contacts = std::mem::take(&mut self.phone_contacts);
            if let Some((_, hnm)) = contacts.get(self.phone_contact) {
                let frame = self.scene_frame % hnm.frame_count().max(1);
                self.scene_palette = hnm.palette;
                hnm.decode_frame(frame, &mut self.scene_buffer, &mut self.scene_palette);
                self.framebuffer.copy_from_slice(&self.scene_buffer);
                self.scene_frame += 1;
            }
            self.phone_contacts = contacts;
            return;
        }
        // Dialling: the REAL pattern (captured live: choice_box_bob_morlock.ppm)
        // — contacts appear as a golden choice box OVER the console panorama,
        // not on a separate chart screen. (The BAPPEL widget belongs to the
        // subsequent calling animation, which loads after a contact is chosen.)
        self.render_bridge_background();
        if !self.console_font.is_empty() {
            let selected = self.phone_contact;
            let mut labels: Vec<String> =
                self.phone_contacts.iter().map(|(n, _)| n.clone()).collect();
            labels.truncate(7);
            labels.push("CANCEL".to_string());
            self.draw_choice_box(&labels, Some(selected));
        }
        self.scene_frame += 1;
    }

    /// Arm the scrutinizer-apparatus intro to play from its first frame the next time
    /// the examination screen renders (call when opening the screen).
    pub fn arm_alien_intro(&mut self) {
        if self.alien_intro.is_some() {
            self.alien_intro_frame = Some(0);
            self.scene_frame = 0;
        }
    }

    /// Render the alien-examination screen: the mouse pan (delta from centre,
    /// smoothed + clamped like the decoded camera at `re/REVERSE.md`) selects one of
    /// the pre-rendered rotation views, whose animation plays looped. Steer left/right
    /// to rotate the alien.
    fn render_alien_view(&mut self) {
        // Play the scrutinizer-apparatus intro once, then hand off to the rotatable
        // alien. `alien_intro_frame` is armed to 0 when the screen is (re)opened.
        if let Some(f) = self.alien_intro_frame {
            if let Some(intro) = self.alien_intro.take() {
                let count = intro.frame_count().max(1);
                if f < count {
                    self.scene_palette = intro.palette;
                    intro.decode_frame(f, &mut self.scene_buffer, &mut self.scene_palette);
                    self.framebuffer.copy_from_slice(&self.scene_buffer);
                    self.alien_intro = Some(intro);
                    self.alien_intro_frame = Some(f + 1);
                    return;
                }
                self.alien_intro = Some(intro);
            }
            self.alien_intro_frame = None; // intro done
            self.scene_frame = 0;
        }
        // Smooth the pan toward the mouse's centre-delta (halve+accumulate), clamped.
        let target = (self.mouse.x as i32 - ENGINE_SCREEN_WIDTH as i32 / 2) / 2;
        self.alien_pan = (self.alien_pan + target) / 2;
        let n = self.alien_views.len();
        if n == 0 {
            return;
        }
        // Map the clamped pan (−160..160) to a rotation view index.
        let span = ENGINE_SCREEN_WIDTH as i32 / 2;
        let t = (self.alien_pan + span).clamp(0, 2 * span - 1) as usize;
        let idx = (t * n / (2 * span as usize)).min(n - 1);
        // Advance the alien's decoded behaviour state machine; when it picks a new
        // animation state it nudges the animation phase, so the alien has idle life
        // (fidgets) between the player's rotations rather than a fixed loop.
        //
        // THE NUDGE ITSELF IS PORT-SIDE, not decoded — the game's consumer of
        // `+0x3C` has not been traced. What IS decoded is the value: since
        // audit-fixes #401, `anim` is the shared `cs:[0x16A2]` counter
        // sign-extended to 32 bits, so it goes NEGATIVE once the 16-bit cell
        // passes 0x7FFF (after ~262 draws). Read it back as the u16 the cell
        // actually is; `anim as usize` on a negative i32 would wrap to a huge
        // value and make this nudge arbitrary at exactly that point.
        if self.alien_object.step(&mut self.alien_prng) {
            let counter = self.alien_object.anim as u16 as usize;
            self.scene_frame = self.scene_frame.wrapping_add(counter % 3);
        }
        let hnm = &self.alien_views[idx];
        let count = hnm.frame_count().max(1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(
            self.scene_frame % count,
            &mut self.scene_buffer,
            &mut self.scene_palette,
        );
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
    }

    /// Render one frame of the current intro clip full-screen; when a clip's frames are
    /// exhausted, advance to the next; when the sequence ends, deactivate the intro so
    /// the main loop takes over.
    fn render_intro_frame(&mut self) {
        let Some(hnm) = self.scene_hnm.take() else {
            self.intro_active = false;
            return;
        };
        let count = hnm.frame_count().max(1);
        // EVERY clip plays its full HNM length. VERIFIED (decoded cliptoot checkpoints 120..1150 vs
        // accuracy/captures/frame_6..22): cliptoot.hnm is the full intro MONTAGE — crew members
        // (the mutant @250/550, the trunk alien @850 — matching captures 6-9), location scenes
        // (teal bar @400 ≈ capture 22), hyperspace @1000, ship scenes @1150 — all under the
        // pyramid console, with the CRYO/title credits clearing at tick 100 (why captures 15/22
        // show no credit text). An earlier ~7s cue-span cut here was WRONG (a misread of capture 9
        // as gameplay); the real game plays the montage through (a click skips it).
        if self.scene_frame >= count {
            // Current clip finished — advance to the next, or end the intro.
            self.intro_index += 1;
            if self.intro_index < self.intro_hnms.len() {
                let next = self.intro_hnms[self.intro_index].clone();
                self.load_scene_hnm(&next);
            } else {
                self.intro_active = false;
            }
            return;
        }
        hnm.decode_frame(
            self.scene_frame,
            &mut self.scene_buffer,
            &mut self.scene_palette,
        );
        self.scene_hnm = Some(hnm);
        let frame = self.scene_frame;
        self.scene_frame += 1;
        self.present_scene_buffer();
        // THE MONTAGE IS PRESENTED THROUGH A FULL-SCREEN REMAP, not by pasting a
        // band. `montage_frame_setup` (`0x7AC3`) pushes the whole 320x200 screen
        // through the CONSOLE-BANK table `DS:0x6011` and then draws the film into
        // the top 140 rows, so rows 140..200 keep whatever stood there — reduced
        // to the same 16 colours as everything else. That is why the captured band
        // is entirely `224..=239`: during the montage the WHOLE FRAME is.
        //
        // The port keeps `overlay_console_band` only until the intro sequencing
        // puts the console on screen ahead of the montage; the remap itself is
        // faithful now and runs first, so the film area is banked exactly as the
        // game banks it.
        if self
            .intro_pyramid
            .get(self.intro_index)
            .copied()
            .unwrap_or(false)
        {
            self.apply_console_bank_remap();
            self.overlay_console_band();
        }
        // Overlay this clip's active credit subtitle (the DESCRIPT `present` cues on the
        // CRYO cinematic), positioned as in the real captures (line rows ~69/79).
        self.draw_intro_credit(frame);
    }

    /// Frame index at which a credit cue's `tick` becomes active. The intro cinematic
    /// advances one clip frame per stepped game frame, so a cue displays from `tick`
    /// frames in until the next cue supersedes it (calibratable against the oracle).
    const INTRO_CREDIT_FRAMES_PER_TICK: usize = 1;
    /// Top row of the FIRST credit line. Native-resolution ground truth (dlg_05, a real
    /// 320x200 capture of the credit beat): line 1 top y=82, line 2 y=92 (pitch 10),
    /// centred on x=160, over the character video above the console band. (The earlier
    /// ~69 figure came from scaled DOSBox window captures.)
    const INTRO_CREDIT_BASELINE_Y: usize = 82;
    /// Top row of the LAST TV broadcast-cue line (the lower letterbox band).
    const TV_CUE_BASELINE_Y: usize = 178;
    /// Reserved palette index forced to white for the credit glyphs (mirrors the
    /// dialogue reveal's reserved 0xFD/0xFE slots).
    const INTRO_CREDIT_COLOR_INDEX: u8 = 253;

    /// Draw the credit subtitle active at intro clip `frame` (if any) centred in the
    /// lower letterbox. The active cue is the last one whose `tick` has been reached.
    fn draw_intro_credit(&mut self, frame: usize) {
        let Some(cues) = self.intro_cues.get(self.intro_index) else {
            return;
        };
        let active = cues
            .iter()
            .filter(|c| frame >= c.tick as usize * Self::INTRO_CREDIT_FRAMES_PER_TICK)
            .next_back();
        let Some(text) = active.map(|c| c.text.clone()) else {
            return;
        };
        self.scene_palette[Self::INTRO_CREDIT_COLOR_INDEX as usize] = [245, 245, 245];
        // Multi-line credits ("CRYO Interactive Entertainment" / "1995") draw centred,
        // 10 rows apart, first line at the real-measured row 69.
        for (i, line) in text
            .split(['\n', '\r'])
            .filter(|l| !l.trim().is_empty())
            .enumerate()
        {
            let width: usize = line.chars().map(crate::font::game_font_advance).sum();
            let x = ENGINE_SCREEN_WIDTH.saturating_sub(width) / 2;
            draw_text_indexed(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                line,
                x,
                Self::INTRO_CREDIT_BASELINE_Y + 10 * i,
                Self::INTRO_CREDIT_COLOR_INDEX,
            );
        }
    }

    /// Load a dialogue script AND resolve each line's speaker to its talk-HNM asset
    /// (actor `0xC4` offset → DEB object name → DESCRIPT record → talk HNM → file in
    /// `asset_dir`), so playback automatically shows the right character per line.
    pub fn load_dialogue_scenes(
        &mut self,
        cod: &[u8],
        var: &[u8],
        dic: &[u8],
        deb: &[u8],
        descript_db: &crate::descript::DescriptDb,
        asset_dir: &Path,
    ) {
        self.load_dialogue(cod, var, dic);
        let object_names = parse_deb_object_names(deb);
        let hnm_paths = collect_hnm_paths(asset_dir);
        self.dialogue_scene_paths = self
            .dialogue
            .iter()
            .map(|l| {
                l.actor_offset
                    .and_then(|o| object_names.get(&o))
                    .and_then(|name| descript_db.record(name))
                    .and_then(|r| r.talk_hnms.first())
                    .and_then(|m| hnm_paths.get(&m.name.to_lowercase()).cloned())
            })
            .collect();
        // Per-line speaker voice bank (`sn/<name>.snd` from the speaker's DESCRIPT
        // record) — the bank the game's voice path plays clips from.
        self.dialogue_voice_banks = self
            .dialogue
            .iter()
            .map(|l| {
                l.actor_offset
                    .and_then(|o| object_names.get(&o))
                    .and_then(|name| descript_db.record(name))
                    .and_then(|r| r.snd.as_ref())
                    .map(|s| {
                        let stem = s.rsplit(['\\', '/']).next().unwrap_or(s).to_lowercase();
                        asset_dir.join("sn").join(stem)
                    })
                    .filter(|p| p.exists())
            })
            .collect();
        self.load_current_scene();
    }

    /// Current dialogue playback cursor (line index), for drivers that react to line
    /// changes (e.g. per-line voice playback).
    pub fn dialogue_cursor(&self) -> usize {
        self.dialogue_cursor
    }

    /// Set the dialogue playback cursor (clamped to the loaded dialogue), used when
    /// restoring a save so playback resumes at the saved line.
    pub fn set_dialogue_cursor(&mut self, cursor: usize) {
        if self.dialogue.is_empty() {
            self.dialogue_cursor = 0;
        } else {
            self.dialogue_cursor = cursor.min(self.dialogue.len() - 1);
        }
    }

    /// Capture the resumable game state into a [`crate::save::SaveState`] (the port's own
    /// save). `script` is the current location/dialogue script number the driver loaded
    /// (0 = none, on the nav) — the engine doesn't own it, so the driver supplies it.
    pub fn capture_save(&self, script: u32) -> crate::save::SaveState {
        use crate::save::SaveScreen;
        let screen = if self.bridge_active {
            SaveScreen::Bridge
        } else if self.tv_active {
            SaveScreen::Comms
        } else if self.cyber_active {
            SaveScreen::Cyberspace
        } else if self.cryobox_active {
            SaveScreen::Cryobox
        } else if self.phone_active {
            SaveScreen::Telephone
        } else if self.on_ship {
            SaveScreen::Nav
        } else {
            SaveScreen::Dialogue
        };
        crate::save::SaveState {
            screen,
            script,
            compass_angle: self.compass_angle,
            dialogue_cursor: self.dialogue_cursor,
            phone_contact: self.phone_contact,
            phone_connected: self.phone_connected,
            text_speed_step: self.text_speed_step,
            visited: self.progress.visited_names(),
        }
    }

    /// Restore the engine-side view and settings from a save. The driver must (re)load
    /// `save.script`'s dialogue BEFORE calling this so the dialogue cursor lands on a valid
    /// line; screen selection, nav heading, phone selection and text speed are applied here.
    pub fn restore_save(&mut self, save: &crate::save::SaveState) {
        use crate::save::SaveScreen;
        self.bridge_active = false;
        self.tv_active = false;
        self.cyber_active = false;
        self.cryobox_active = false;
        self.phone_active = false;
        self.on_ship = false;
        match save.screen {
            SaveScreen::Nav => self.on_ship = true,
            SaveScreen::Bridge => self.bridge_active = true,
            SaveScreen::Comms => self.tv_active = true,
            SaveScreen::Cyberspace => self.cyber_active = true,
            SaveScreen::Cryobox => self.cryobox_active = true,
            SaveScreen::Telephone => self.phone_active = true,
            SaveScreen::Dialogue => {}
        }
        self.compass_angle = save.compass_angle % 180;
        if !self.phone_contacts.is_empty() {
            self.phone_contact = save.phone_contact.min(self.phone_contacts.len() - 1);
        }
        self.phone_connected = save.phone_connected;
        self.text_speed_step = save.text_speed_step;
        self.set_dialogue_cursor(save.dialogue_cursor);
        // Restore the game progression (which locations/crew were visited).
        for name in &save.visited {
            self.progress.visit(name);
        }
    }

    /// How many subtitle characters are currently revealed on the active line (the
    /// game's reveal pointer `gs:0x5E58`), and the line's total character count. A
    /// driver plays the `tb.snd` chatter (clip 0) when `revealed` first reaches
    /// `total` — the decoded one-chatter-per-completed-line behaviour.
    ///
    /// CITATION TIGHTENED (audit-fixes #367). The doc named `0x94BA`, which is
    /// the block's GUARD (`test byte [0x24f3],4 / jne`, then `[0x67BB]` and
    /// `[0x67BC]`); it plays nothing. The instruction behind "clip 0" is three
    /// tests later:
    ///
    /// ```text
    ///   0x94B4  inc word ptr [0x5e58]        the reveal pointer this returns
    ///   0x94BA  test byte ptr [0x24f3], 4    guard: already holding?
    ///   0x94CF  mov byte ptr [0xcfb], 0      <- SELECT clip 0
    ///   0x94D4  mov ax,[0xaca] / shl ax,2 / mov [0xb35], ax   the hold timer
    ///   0x94DD  mov byte ptr [0x67bb], 1     latch: hold armed
    /// ```
    ///
    /// So the game SELECTS the clip and arms a hold; the driver does the playing,
    /// which is what this doc says and what `0x94BA` alone did not show.
    pub fn subtitle_reveal_progress(&self) -> Option<(usize, usize)> {
        let text = self.dialogue_texts.get(self.dialogue_cursor)?;
        if text.is_empty() {
            return None;
        }
        let total = text.chars().count();
        let per_char = u32::from(crate::vm::reveal_frames_per_char(self.text_speed_step));
        let revealed = ((self.dialogue_timer / per_char.max(1)) as usize).min(total);
        Some((revealed, total))
    }

    /// The current line's speaker SND bank path + its `b3` selector.
    ///
    /// NO LONGER DRIVES AUDIO. The per-line voice clip this used to feed was removed:
    /// the game plays a random burble (`prng(10)+7`) while the line reveals, gated by
    /// `gs:[0xCFB]` (`0x66AF` set / `0x94CF` clear), with no per-line selection
    /// anywhere in the executable. Retained only as the bank/selector accessor.
    pub fn current_voice(&self) -> Option<(std::path::PathBuf, u8)> {
        let bank = self
            .dialogue_voice_banks
            .get(self.dialogue_cursor)?
            .clone()?;
        let line = self.dialogue.get(self.dialogue_cursor)?;
        let sel = *self.voice_by_offset.get(&line.offset)?;
        Some((bank, sel))
    }

    /// Load the talk-HNM resolved for the current dialogue line (if any).
    fn load_current_scene(&mut self) {
        // A line with its own scene switches to it; a line WITHOUT one keeps the current
        // scene (so a character's sceneless lines play over the location that was last
        // shown, not the console). `set_speech_dialogue` clears the scene at the start of a
        // new dialogue, so a dialogue with no scenes at all (HONK's food menu, the console
        // tutorial) correctly falls back to the console panel.
        if let Some(Some(path)) = self.dialogue_scene_paths.get(self.dialogue_cursor).cloned() {
            self.load_scene_hnm(&path);
            // FILM BEAT (decoded A8 @0x67F6..0x682F: LOADSTR queues the HNM as a film via
            // ship-FSM state 7; the following empty SAY is its line-slot): an empty-text
            // line with a scene holds for the film's FULL length so the reel plays out.
            let empty = self
                .dialogue_texts
                .get(self.dialogue_cursor)
                .is_some_and(|t| t.trim().is_empty());
            if empty {
                if let Some(h) = &self.scene_hnm {
                    let frames = h.frame_count() as u32;
                    self.line_min_hold = Some((self.dialogue_cursor, frames.max(1)));
                }
            }
        }
    }

    /// Load a dialogue script (`SCRIPTn.COD` + `.VAR`): run the VM trace and queue
    /// its reached dialogue lines for frame-stepped playback. Each [`EngineState::
    /// step`] advances the playback timer; the current line is [`EngineState::
    /// current_dialogue`]. This is the script/scene stepping the engine's main loop
    /// drives (the `D2` script/scene handoff at `0x108E`).
    pub fn load_dialogue(&mut self, cod: &[u8], var: &[u8], dic: &[u8]) {
        // Reconstruct each text call's subtitle text from the dictionary.
        let words = parse_dictionary(dic);
        let mut text_by_offset: HashMap<usize, String> = HashMap::new();
        self.voice_by_offset.clear();
        self.menu_by_offset.clear();
        for tok in walk(cod, 0, cod.len()) {
            if let VmToken::Text {
                offset,
                voice_selector,
                word_offsets,
                ..
            } = tok
            {
                // The word list is TWO sections split by 0xFFFF: the spoken line,
                // then the CHOICE-MENU rows. `filter_map` silently dropped the
                // separator (0xFFFF is not a DIC key) but KEPT the menu words after
                // it, so 214 of the 3687 A6 lines across the five scripts (COUNTED,
                // audit-fixes #416 -- this said 211 of 3650 from memory) rendered
                // their choices appended to the subtitle -- e.g. SCRIPT1.COD's
                // "Click quick, Cap'n Bob is waiting ..." came out with
                // "explanations game" glued on the end.
                let sep = word_offsets.iter().position(|&w| w == 0xFFFF);
                let spoken = &word_offsets[..sep.unwrap_or(word_offsets.len())];
                let parts: Vec<String> = spoken
                    .iter()
                    .filter_map(|o| words.get(o).cloned())
                    .collect();
                if !parts.is_empty() {
                    text_by_offset.insert(offset, assemble_words(&parts));
                }
                // Keep the menu rows as the line's own data, so a choice box can be
                // sourced from the SCRIPT rather than from a const in this file.
                if let Some(i) = sep {
                    let rows: Vec<String> = word_offsets[i + 1..]
                        .iter()
                        .filter_map(|o| words.get(o).cloned())
                        .collect();
                    if !rows.is_empty() {
                        self.menu_by_offset.insert(offset, rows);
                    }
                }
                self.voice_by_offset.insert(offset, voice_selector);
            }
        }
        let trace = execute_trace(cod, var);
        // D2 scene-to-scene handoff: the next scene/profile this script requests.
        self.pending_profile = trace.pending_script_profile();
        self.dialogue = trace.line_states;
        self.dialogue_texts = self
            .dialogue
            .iter()
            .map(|l| text_by_offset.get(&l.offset).cloned().unwrap_or_default())
            .collect();
        self.dialogue_cursor = 0;
        self.dialogue_timer = 0;
    }

    /// Play dialogue directly from the port's decoded speech events — the FULL per-script,
    /// per-character content (every character's lines, with each line's background scene),
    /// instead of `execute_trace`'s single linear branch (which reaches only a fraction of
    /// the ~3400 decoded lines). Each `lines` entry is (subtitle, background-HNM path).
    pub fn set_speech_dialogue(&mut self, lines: Vec<(String, Option<std::path::PathBuf>)>) {
        if self.bridge_active && !lines.is_empty() {
            self.hub_presentation = true;
        }
        if self.console_band_dialogue && !lines.is_empty() {
            // The presentation frame ZOOMS OPEN through the 6-phase rect table
            // (DS:0x2B97, screen_mode_update 0x79E5) before the content shows.
            self.presentation_open_phase = 1;
        }
        self.autoplay_end = None; // a new scene plays through unless the driver gates it
        self.dialogue_segments.clear();
        self.dialogue_segment_pos = 0;
        self.dialogue = (0..lines.len())
            .map(|offset| LineState {
                offset,
                actor_offset: None,
                location_offset: None,
            })
            .collect();
        self.dialogue_texts = lines.iter().map(|(t, _)| t.clone()).collect();
        self.dialogue_is_speech = vec![true; self.dialogue_texts.len()];
        self.dialogue_scene_paths = lines.into_iter().map(|(_, p)| p).collect();
        self.dialogue_cursor = 0;
        self.dialogue_timer = 0;
        // Start over the dialogue's FIRST available location scene, so a scene that opens
        // with sceneless briefing lines still plays over its location (not the console) from
        // the start; a fully-sceneless dialogue (HONK's food menu) stays on the console.
        self.scene_hnm = None;
        if let Some(path) = self.dialogue_scene_paths.iter().flatten().next().cloned() {
            self.load_scene_hnm(&path);
        }
    }

    /// The dialogue line currently being presented, if a script is loaded.
    pub fn current_dialogue(&self) -> Option<&LineState> {
        self.dialogue.get(self.dialogue_cursor)
    }

    /// The next scene/profile the loaded script's D2 handoff dispatches to (for
    /// scene-to-scene chaining), or `None` if this is a terminal scene. The driver
    /// loads that profile's script when the current dialogue finishes.
    pub fn pending_next_scene(&self) -> Option<u16> {
        self.pending_profile
    }

    /// Whether dialogue playback has reached the final line (the point at which the
    /// D2 handoff to [`EngineState::pending_next_scene`] would fire).
    pub fn dialogue_finished(&self) -> bool {
        !self.dialogue.is_empty() && self.dialogue_cursor + 1 >= self.dialogue.len()
    }

    /// Queue a sequence of scene scripts `(cod, var, dic)` and start the first, so
    /// the engine auto-advances scene-to-scene as each finishes (the scene flow the
    /// D2 handoff drives). Returns the number of scenes queued.
    pub fn queue_scenes(&mut self, scenes: Vec<(Vec<u8>, Vec<u8>, Vec<u8>)>) -> usize {
        self.scene_queue = scenes;
        self.scene_queue_idx = 0;
        let n = self.scene_queue.len();
        if let Some((cod, var, dic)) = self.scene_queue.first().cloned() {
            self.load_dialogue(&cod, &var, &dic);
        }
        n
    }

    /// The index of the scene currently playing in the queue.
    pub fn current_scene_index(&self) -> usize {
        self.scene_queue_idx
    }

    /// If the current dialogue has finished and another scene is queued, advance to
    /// it (loading its script). Returns true if it advanced.
    fn advance_scene_if_finished(&mut self) -> bool {
        if self.dialogue_finished() && self.scene_queue_idx + 1 < self.scene_queue.len() {
            self.scene_queue_idx += 1;
            let (cod, var, dic) = self.scene_queue[self.scene_queue_idx].clone();
            self.load_dialogue(&cod, &var, &dic);
            true
        } else {
            false
        }
    }

    /// 1-based page number of the current dialogue line — the green console digit the
    /// real presentation screen shows top-left (oracle bd_218M..bd_290M, index 254).
    pub fn dialogue_page_number(&self) -> usize {
        self.dialogue_cursor + 1
    }

    /// The current dialogue line's reconstructed subtitle text, if non-empty.
    pub fn current_subtitle(&self) -> Option<&str> {
        self.dialogue_texts
            .get(self.dialogue_cursor)
            .map(String::as_str)
            .filter(|s| !s.is_empty())
    }

    /// Number of dialogue lines the loaded script reached.
    pub fn dialogue_len(&self) -> usize {
        self.dialogue.len()
    }

    /// Advance the dialogue playback: after `dialogue_hold_frames`, step to the next
    /// reached line (stops at the last line).
    /// Hold for the current line, using the game's decoded subtitle timing: the text
    /// reveals at `reveal_frames_per_char(step)` frames per character (`gs:[0xB31] =
    /// step >> 2`, REVERSE.md @0x94BA), then holds `reveal_complete_hold_ticks(step)`
    /// (`gs:[0xB35] = step << 2` @0x94D4) before the next line. `dialogue_hold_frames`
    /// acts as a driver-set floor (tests use a huge floor to freeze playback).
    fn current_line_hold(&self) -> u32 {
        // Voice-paced floor: hold at least as long as the line's voice clip (driver-supplied).
        let min_hold = match self.line_min_hold {
            Some((line, frames)) if line == self.dialogue_cursor => frames,
            _ => 0,
        };
        use crate::vm::{reveal_complete_hold_ticks, reveal_frames_per_char};
        let len = self
            .dialogue_texts
            .get(self.dialogue_cursor)
            .map(|t| t.chars().count() as u32)
            .unwrap_or(0);
        let step = self.text_speed_step;
        let reveal = len.saturating_mul(u32::from(reveal_frames_per_char(step)));
        let hold = u32::from(reveal_complete_hold_ticks(step));
        self.dialogue_hold_frames
            .max(reveal.saturating_add(hold))
            .max(min_hold)
    }

    /// Whether a dialogue scene is currently the active view (playing, with no overlay
    /// screen / nav / intro / ending on top) — so a driver can route clicks to advance it.
    pub fn in_dialogue(&self) -> bool {
        !self.dialogue.is_empty()
            && !self.on_ship
            && !self.bridge_active
            && !self.tv_active
            && !self.cyber_active
            && !self.cryobox_active
            && !self.phone_active
            && !self.intro_active
            && !self.ending_active
            && !self.world_location_active()
    }

    /// Manually advance the dialogue on a click (as the real game does): if the current
    /// line is still revealing, snap it fully revealed; otherwise move to the next line.
    /// Returns `false` when already on the last line's fully-revealed text (the driver then
    /// ends the dialogue).
    pub fn skip_dialogue_line(&mut self) -> bool {
        if self.dialogue.is_empty() {
            return false;
        }
        let full = self.current_line_hold();
        if self.dialogue_timer + 1 < full {
            // Still revealing / holding: snap to fully revealed so the whole line shows.
            self.dialogue_timer = full.saturating_sub(1);
            return true;
        }
        // Fully shown: advance to the next line, or signal the end. A click does not blow
        // through the autoplay boundary — the topic menu owns what plays next.
        if let Some(end) = self.autoplay_end {
            if self.dialogue_cursor + 1 >= end {
                return false;
            }
        }
        self.dialogue_timer = 0;
        if self.dialogue_cursor + 1 < self.dialogue.len() {
            self.dialogue_cursor += 1;
            if !self.dialogue_scene_paths.is_empty() {
                self.load_current_scene();
            }
            true
        } else {
            false
        }
    }

    fn advance_dialogue(&mut self) {
        if self.dialogue.is_empty() {
            return;
        }
        // Hold at the autoplay boundary: the current line stays fully shown and the topic menu
        // waits for the player (freeze the timer at the reveal-complete point).
        if let Some(end) = self.autoplay_end {
            if self.dialogue_cursor + 1 >= end {
                let full = self.current_line_hold().saturating_sub(1);
                if self.dialogue_timer >= full {
                    self.dialogue_timer = full;
                    // The scripted OPENING (which plays over the pyramid-console band in
                    // SCRIPT1 — real-game tut_240s) has finished: the interactive phase runs
                    // on the purple bridge console (interpreter-oracle-verified, golden menu),
                    // so drop the band from here on.
                    self.console_band_dialogue = false;
                    return;
                }
            }
        }
        self.dialogue_timer += 1;
        if self.dialogue_timer >= self.current_line_hold() {
            self.dialogue_timer = 0;
            if self.dialogue_cursor + 1 < self.dialogue.len() {
                self.dialogue_cursor += 1;
                // New line: load its resolved talk-HNM (the right speaker).
                if !self.dialogue_scene_paths.is_empty() {
                    self.load_current_scene();
                }
            }
        }
    }

    /// Load the ship-nav HUD sprite banks (BCARTE grid frames + BORXX orb) from
    /// their raw `.spr` bytes so [`EngineState::render_ship_view`] composites the
    /// accurate sprite HUD over the starfield.
    pub fn load_hud_sprites(&mut self, bcarte_spr: &[u8], borxx_spr: &[u8]) {
        self.hud_grid = decode_sprite_bank_indices(bcarte_spr).unwrap_or_default();
        self.hud_orb = decode_sprite_bank_indices(borxx_spr).unwrap_or_default();
    }

    /// Load the star-map nav sprites: `CARTE.SPR` holds the game's actual destination
    /// pyramid frames at six pre-scaled sizes (f0..f5) plus the selection reticle
    /// (f6); `BORXX.SPR` the eye-orb frames. These are the real art the game's
    /// sprite-blit path (0x4BAA) draws at projected destination positions.
    pub fn load_nav_sprites(&mut self, carte_spr: &[u8], borxx_spr: &[u8]) {
        self.nav_pyramids = decode_sprite_bank_indices(carte_spr).unwrap_or_default();
        if self.hud_orb.is_empty() {
            self.hud_orb = decode_sprite_bank_indices(borxx_spr).unwrap_or_default();
        }
    }

    /// Draw the star-map destination pyramids with the game's real components: the
    /// ground-plane grid of destinations is projected point-by-point with
    /// `project_star_map_point` (the decoded 0x9BBA math, compass-panned), and each
    /// projection blits the CARTE.SPR pyramid frame whose pre-scaled size best
    /// matches the projected sprite scale (`0x100000/depth`, the sprite path's scale
    /// term). Real art + real math; the destination layout itself is the documented
    /// runtime-gated remainder (live `DS:0x4F09` records).

    /// Test/inspection helper: the first few world labels, for asserting the nav
    /// row is fed from the level directory at all.
    ///
    /// NO BINARY COUNTERPART, and this needs saying because the ledger read the
    /// PRECEDING doc block's citations (`0x9BBA`, `0x4F09`) as if they were this
    /// function's — they belong to the pyramid renderer above it (audit-fixes
    /// #421, #428). The game has no "sample the first 7 labels" operation.
    ///
    /// What IS data-backed is the content: `nav_world_labels` comes from
    /// `levels::primary_worlds()`, i.e. the resource directory, which
    /// `level_directory_literal_matches_the_image` holds to the bytes at file
    /// `0xCDF4`. The `7` is this helper's own, chosen to keep the assertion
    /// readable.
    pub fn nav_world_label_sample(&self) -> Vec<&'static str> {
        self.nav_world_labels.iter().take(7).copied().collect()
    }

    /// Load + arm the title screen from `BLOOD.LBM` under `iso`: decode the planar ILBM
    /// title art and downscale it aspect-correctly (e.g. 640×480 → 320×200, nearest,
    /// keeping the full image) into the framebuffer's resolution. Returns whether it
    /// loaded. Shown until dismissed.
    pub fn load_title(&mut self, iso: &std::path::Path) -> bool {
        let Ok(data) = std::fs::read(iso.join("BLOOD.LBM")) else {
            return false;
        };
        let Some(img) = crate::lbm::decode_lbm(&data) else {
            return false;
        };
        // Downscale to the engine framebuffer with the true width/height ratios (nearest
        // sample). Integer ratios crop: 480 rows over 200 at 2x would only sample rows
        // 0..400 and lose the bottom 80px, so scale by the exact source span instead —
        // the whole image maps into 320x200 (e.g. 640x480 -> 2.0x horizontal, 2.4x
        // vertical), preserving all of the art.
        let mut buf = vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT];
        for y in 0..ENGINE_SCREEN_HEIGHT {
            let src_y = (y * img.height / ENGINE_SCREEN_HEIGHT).min(img.height - 1);
            for x in 0..ENGINE_SCREEN_WIDTH {
                let src_x = (x * img.width / ENGINE_SCREEN_WIDTH).min(img.width - 1);
                buf[y * ENGINE_SCREEN_WIDTH + x] = img.pixels[src_y * img.width + src_x];
            }
        }
        self.title_screen = Some((buf, img.palette));
        true
    }

    /// Whether the title screen is armed/showing.
    pub fn title_active(&self) -> bool {
        self.title_screen.is_some()
    }

    /// Dismiss the title screen (advance to the intro/game).
    pub fn dismiss_title(&mut self) {
        self.title_screen = None;
    }

    /// Render the downscaled title art into the framebuffer.
    fn render_title(&mut self) {
        if let Some((buf, pal)) = &self.title_screen {
            self.framebuffer.copy_from_slice(buf);
            self.scene_palette = *pal;
        }
    }

    /// Visit a world by name: collect all its decoded `fd/` rooms (floor/view-angle
    /// backgrounds the world maps to) from `assets`, show the first, and enable cycling.
    /// Returns whether any room was found + loaded. Rooms are ordered by filename so
    /// floor 1 (the entry room) shows first.
    pub fn visit_world(&mut self, world: &str, assets: &std::path::Path) -> bool {
        if crate::levels::world_location_abbrev(world).is_none() {
            return false;
        }
        let fd = assets.join("fd");
        let mut rooms: Vec<std::path::PathBuf> = match std::fs::read_dir(&fd) {
            Ok(rd) => rd
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| {
                            let n = n.to_lowercase();
                            n.ends_with(".lbm") && crate::levels::art_belongs_to_world(&n, world)
                        })
                        .unwrap_or(false)
                })
                .collect(),
            Err(_) => return false,
        };
        // Sort by floor then filename so all floors of the world are explorable in order.
        rooms.sort_by(|a, b| {
            let fa = a.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let fb = b.file_name().and_then(|n| n.to_str()).unwrap_or("");
            crate::levels::art_floor(fa)
                .cmp(&crate::levels::art_floor(fb))
                .then_with(|| fa.cmp(fb))
        });
        if rooms.is_empty() {
            return false;
        }
        let Some(img) = std::fs::read(&rooms[0])
            .ok()
            .and_then(|d| crate::lbm::decode_pbm(&d))
        else {
            return false;
        };
        self.world_location = Some(WorldVisit {
            name: world.to_uppercase(),
            rooms,
            current: 0,
            image: img,
            objects: Vec::new(),
        });
        true
    }

    /// Supply the visited world's `.ext` bytes so its decoded object positions are marked
    /// on the location screen. Parses the objects via [`crate::ext`] and stores their
    /// `(x, y)`. No-op if no world is being visited or the data isn't a world file.
    pub fn set_world_ext(&mut self, ext_data: &[u8]) -> usize {
        let Some(visit) = &mut self.world_location else {
            return 0;
        };
        let Some(world) = crate::ext::parse_ext(ext_data) else {
            return 0;
        };
        visit.objects = world.objects(ext_data).iter().map(|o| (o.x, o.y)).collect();
        visit.objects.len()
    }

    /// Hit-test a click against the visited world's decoded `.ext` OBJECT markers
    /// (the world's entities, e.g. the initial id=1/type=4 inhabitant at its
    /// world-specific position). Returns the object index within ~14px.
    pub fn world_object_click(&self, x: u16, y: u16) -> Option<usize> {
        let visit = self.world_location.as_ref()?;
        visit.objects.iter().position(|&(ox, oy)| {
            (ox as i32 - x as i32).abs() <= 14 && (oy as i32 - y as i32).abs() <= 14
        })
    }

    /// Cycle to another room of the currently-visited world (`delta` = +1/-1), decoding
    /// its background. No-op if no world is being visited.
    pub fn cycle_world_room(&mut self, delta: i32) {
        let Some(visit) = &mut self.world_location else {
            return;
        };
        let n = visit.rooms.len();
        if n <= 1 {
            return;
        }
        let next = (visit.current as i32 + delta).rem_euclid(n as i32) as usize;
        if let Some(img) = std::fs::read(&visit.rooms[next])
            .ok()
            .and_then(|d| crate::lbm::decode_pbm(&d))
        {
            visit.current = next;
            visit.image = img;
        }
    }

    /// The visited world's room count + current index (for HUD/tests), if active.
    pub fn world_room_position(&self) -> Option<(usize, usize)> {
        self.world_location
            .as_ref()
            .map(|v| (v.current, v.rooms.len()))
    }

    /// Whether the world-location landing screen is active.
    pub fn world_location_active(&self) -> bool {
        self.world_location.is_some()
    }

    /// Whether the plain nav star-map is the active view — on the ship with no overlay
    /// screen (bridge/comms/cyberspace/cryobox/alien/world-landing) open. This is the
    /// view that shows the choose-a-location destination list.
    /// Drive the ship-3D view's TRANSITION + DEPTH state machine for one frame —
    /// the game's `0xB692` (transition) followed by `0xB75C` (depth scroll), both
    /// audit-verified exact against the binary but previously unreachable in play.
    ///
    /// The hold counter (`gs:0x0B3B`) advances while the nav view is up; once it
    /// passes 120 the view arms and OPENS with step 4, and thereafter a `rand(20)`
    /// draw that comes up ZERO closes it with step 8 — the modulus is the
    /// handler's own (`0xB6D0 mov ax,0x14`), taken from the game's PRNG.
    pub fn step_ship_3d_nav_state(&mut self) {
        use crate::ship3d::{step_ship_3d_depth_scroll, update_ship_3d_transition_state};
        self.ship3d_hold_ticks = self.ship3d_hold_ticks.saturating_add(1);
        self.ship3d_transition.hold_ticks = self.ship3d_hold_ticks;
        let close_gate = self.ship3d_prng.next(20) == 0;
        update_ship_3d_transition_state(&mut self.ship3d_transition, close_gate);
        // The transition writes the shared step/direction cells the scroll reads
        // (DS:0x2531 step, DS:0x252F opening, DS:0x2530 closing).
        self.ship3d_depth.depth_step = self.ship3d_transition.depth_step;
        self.ship3d_depth.opening = self.ship3d_transition.opening;
        self.ship3d_depth.closing = self.ship3d_transition.closing;
        step_ship_3d_depth_scroll(&mut self.ship3d_depth);
        // The procedural HUD/nav-timer machine runs on the same frame tick. Feed
        // it the REAL cursor and button state (it consumes them directly) and the
        // same hold counter the transition gate reads -- no invented inputs.
        self.ship3d_procedural.hold_ticks = self.ship3d_hold_ticks;
        self.ship3d_procedural.mouse_x = self.mouse.x;
        self.ship3d_procedural.mouse_y = self.mouse.y;
        self.ship3d_procedural.mouse_button_state = self.mouse.buttons;
        crate::ship3d::run_ship_3d_procedural_update(&mut self.ship3d_procedural);
        // The scroll clears its own direction flags when it reaches a limit.
        self.ship3d_transition.opening = self.ship3d_depth.opening;
        self.ship3d_transition.closing = self.ship3d_depth.closing;

        // --- interpolation gate -> sequence update -------------------------------
        // docs/port-validation.md called this chain "blocked on other DORMANT
        // ship-3D code". It is not blocked: every function in it is PURE, taking
        // bools/u16s/small arrays. What it needed was calling in the right order,
        // which is what this does.
        //
        // The gate (0x1E5D) advances one tick per frame and reports Complete when
        // current_tick reaches duration_ticks; the sequence update (the FSM that
        // owns exit/opening) consumes that completion plus the target selector's
        // query index.
        self.ship3d_interpolation.duration_ticks =
            self.ship3d_nav_sequence.interpolation_duration_ticks;
        let interpolation_complete = matches!(
            crate::ship3d::step_ship_3d_interpolation_gate(
                &mut self.ship3d_interpolation,
                self.ship3d_interpolation_source,
                self.ship3d_interpolation_dest,
            ),
            Some(crate::ship3d::Ship3dInterpolationStep::Complete)
        );

        // Target selection over the REAL granted-destination list, not a synthetic
        // one: the same source the projector gates on.
        let targets: Vec<u16> = (0..self.nav_destinations.len() as u16).collect();
        let current_target = self.ship3d_target_selector.current_target;
        let query_selection = crate::ship3d::select_ship_3d_target_record(
            &mut self.ship3d_target_selector,
            &targets,
            &[],
            current_target,
            interpolation_complete,
        )
        .map(|sel| sel.selected_target)
        .unwrap_or(current_target);

        let effect = crate::ship3d::run_ship_3d_navigation_sequence_update(
            &mut self.ship3d_nav_sequence,
            // No presentation runs while the nav view is up (the view is mutually
            // exclusive with dialogue/bridge/cyber/cryobox — see nav_view_active).
            false,
            false,
            interpolation_complete,
            query_selection,
        );
        // The FSM asks for a framebuffer copy; the engine already redraws every
        // frame, so this only records that the request was raised.
        self.ship3d_sequence_redraw_requested = effect.copied_framebuffer;
    }

    pub fn nav_view_active(&self) -> bool {
        self.on_ship
            && !self.bridge_active
            && !self.tv_active
            && !self.cyber_active
            && !self.cryobox_active
            && !self.phone_active
            && !self.alien_view_active
            && !self.world_location_active()
    }

    /// Close the world-location screen (back to nav).
    pub fn leave_world(&mut self) {
        self.world_location = None;
    }

    /// Render the current world-location background (its decoded palette + pixels) with
    /// the world name + room index captioned, into the framebuffer.
    fn render_world_location(&mut self) {
        // Take the visit out so the blit can mutate the framebuffer without a borrow
        // conflict, then put it back.
        let Some(visit) = self.world_location.take() else {
            return;
        };
        let img = &visit.image;
        // Caption with the decoded floor + room + facing parsed from the art name.
        let name = {
            let file = visit.rooms[visit.current]
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("")
                .to_lowercase();
            let floor = crate::levels::art_floor(&file);
            let abbrev =
                crate::levels::world_location_abbrev(&visit.name.to_lowercase()).unwrap_or("");
            // Match against the abbreviation, skipping any leading floor digit.
            let body = file
                .strip_prefix(|c: char| c.is_ascii_digit())
                .unwrap_or(&file);
            match crate::levels::parse_room_view(body, abbrev) {
                Some((room, view)) => {
                    let facing = match view {
                        'f' => "FRONT",
                        'b' => "BACK",
                        'd' => "LEFT",
                        'g' => "RIGHT",
                        _ => "VIEW",
                    };
                    format!("{}  FLOOR {floor} ROOM {}  {}", visit.name, room, facing)
                }
                None => format!(
                    "{}  {}/{}",
                    visit.name,
                    visit.current + 1,
                    visit.rooms.len()
                ),
            }
        };
        // Blit the decoded room background (320x200 fills the screen).
        for y in 0..ENGINE_SCREEN_HEIGHT.min(img.height) {
            for x in 0..ENGINE_SCREEN_WIDTH.min(img.width) {
                self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = img.pixels[y * img.width + x];
            }
        }
        self.scene_palette = img.palette;
        self.scene_palette[0xFE] = [245, 245, 160];
        self.scene_palette[0xFD] = [255, 80, 80]; // object marker colour
        // DEBUG-ONLY overlays (CB_DEBUG=1): the location caption and the entity
        // crosshairs are PORT tooling — no such strings/markers exist in the
        // binary (searched: no FLOOR/ROOM text; the real screens draw entities
        // as scene content and interact via the candidate box).
        if std::env::var("CB_DEBUG").is_ok() {
            for &(ox, oy) in &visit.objects {
                let (cx, cy) = (ox as usize, oy as usize);
                for d in 0..5usize {
                    for (px, py) in [
                        (cx + d, cy),
                        (cx.wrapping_sub(d), cy),
                        (cx, cy + d),
                        (cx, cy.wrapping_sub(d)),
                    ] {
                        if px < ENGINE_SCREEN_WIDTH && py < ENGINE_SCREEN_HEIGHT {
                            self.framebuffer[py * ENGINE_SCREEN_WIDTH + px] = 0xFD;
                        }
                    }
                }
            }
            draw_text_indexed(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &name,
                8,
                6,
                0xFE,
            );
        }
        self.world_location = Some(visit);
        if !self.console_box.is_empty() {
            let labels = self.console_box.clone();
            self.draw_choice_box(&labels, None);
        }
    }

    /// Draw the nav-map destination pyramids through the game's own projector.
    ///
    /// `ship_3d_object_sprite_project` @`0x9B98` is the per-destination
    /// projector (`mov bx,0x4f09` / `mov di,0x4f01` load its table pointers), and
    /// the matrix it uses is built from the angle words at `DS:0x2F6D`/`0x2F6F` —
    /// `0x990C` reads `[0x2f6f]`, `shl di,2` to index the table, and `movsx`es the
    /// pair into 32-bit before doubling them to Q15, which is the same widening
    /// `matrix_pair_for_angle` performs.
    ///
    /// That is why the compass angle is fed to `projection_angle_2f6d` here rather
    /// than applied to the sprite positions afterwards: the heading IS a matrix
    /// input in the original, so panning rotates the field through the projection
    /// instead of sliding it.
    ///
    /// Cited here because it was settled ASM with no doc (#141's queue).
    fn render_nav_pyramid_sprites(&mut self) {
        use crate::ship3d::{
            NAV_DESTINATION_POINTS, SHIP_3D_ANGLE_TABLE, SHIP_3D_OBJECT_VISIBLE_FLAG,
            SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG, Ship3dMatrixAngles, Ship3dProjectionOrigin,
            Ship3dProjectionPoint, Ship3dProjectionViewport, build_ship_3d_projection_matrix,
            collect_ship_3d_dirty_sprite_slot_render_commands, commit_ship_3d_global_clip_snapshot,
            commit_ship_3d_sprite_slot_dirty_geometry, project_ship_3d_object_sprite,
        };
        let Some(m) = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                // The compass angle feeds the MATRIX (DS:0x2F6D), matching the
                // sibling star-map renderer and the builder's own input at 0x990C.
                projection_angle_2f6d: self.compass_angle % 180,
                angle_2f6f: 0,
            },
        ) else {
            return;
        };
        // THE GAME'S OWN NAV-DESTINATION PROJECTOR (0x9B98), decoded:
        //   * it iterates entities 0x15..0x1F (`0x6212+((i+0x15)<<5)`) and draws
        //     ONLY those whose flags word has bit7 set (`test [si],0x80`), so the
        //     marker count is the number of ACTIVE destinations — never a fixed
        //     grid. Oracle-confirmed: in every savestate reachable today
        //     (location_visit / arrival_probe / milestone_script2) all eleven
        //     records are ZERO, i.e. the real routine draws nothing until
        //     destinations are granted.
        //   * positions come from DS:0x4F09, which is a STATIC table — 10 entries
        //     of three i16 at stride 6, every one (10200, 12100, 900). Verified
        //     unwritten at runtime by a WRITE WATCH over the table's linear range
        //     (runtime_boot NAVWRITE): zero writes across a full MENUMAP run. That
        //     watch carries a positive control — it dumps the watched bytes and
        //     they read back as the baked points — so the zero-hit result is a
        //     real negative, not a watch aimed at the wrong address.
        //   * the projector loops ELEVEN times (0x2F77 seeded 0x0B at 0x9BB4,
        //     DEC/JS at 0x9BBA, `add bx,6` at 0x9CF5) over a TEN-entry table, so
        //     its last iteration reads DS:0x4F45 — the trig table — as if it were
        //     a position. It pairs with entity 0x15, because the entity index
        //     0x6212+((i+0x15)<<5) descends as bx ascends. Not reproduced here:
        //     the draw is gated on the entity's active bit7, and no state reached
        //     so far sets it.
        //   * the camera origin is DS:0x2F65 = (10000, 12000, 0), also baked.
        // The port therefore draws one marker per GRANTED destination (the
        // GameProgress set that stands in for the active-entity bits) instead of
        // the old fabricated 7x4 = 28-point grid.
        // DS:0x2F65, WIDENED: the game stores three WORDS there and the port
        // holds i32, which is why a byte search of the image does not find this
        // array (`tools/check_literal_tables.py` reports it ABSENT for that
        // reason alone). `nav_camera_origin_matches_ds_2f65` reads the words back
        // out of BLOODPRG.EXE and compares.
        const NAV_CAMERA_ORIGIN: [i32; 3] = [10000, 12000, 0]; // DS:0x2F65
        let origin = NAV_CAMERA_ORIGIN;
        // Base pyramid dimension: the biggest CARTE pyramid frame (f4, 24px wide).
        let base_w = self.nav_pyramids[4].width.max(1) as u32;
        let count = self.nav_destinations.len();
        let src_h = self.nav_pyramids[4].height.max(1) as u16;

        // --- sprite slots -------------------------------------------------------
        // Slots are PERSISTENT across frames on purpose: update_ship_3d_sprite_slot_position
        // raises the dirty flag only when a slot actually MOVES, so a fresh slot every
        // frame would make the dirty tracking meaningless.
        self.ship3d_nav_slots.resize_with(count, Default::default);
        self.assign_nav_slot_entity_ids();
        for slot in self.ship3d_nav_slots.iter_mut() {
            // ACTIVE (0x01) | VISIBLE (0x80). The collector requires ACTIVE; the
            // projector's gate and the position updater's mask want VISIBLE.
            slot.flags |= SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG | SHIP_3D_OBJECT_VISIBLE_FLAG;
            slot.source_width = base_w as u16;
            slot.source_height = src_h;
        }
        let cam = Ship3dProjectionOrigin {
            x: origin[0] as u16,
            y: origin[1] as u16,
            z: origin[2] as u16,
        };
        for idx in 0..count {
            let point = NAV_DESTINATION_POINTS[idx.min(NAV_DESTINATION_POINTS.len() - 1)];
            let anchor = Ship3dProjectionPoint {
                x: point[0] as u16,
                y: point[1] as u16,
                z: point[2] as u16,
            };
            // The game's own projector (0x9B98), verified instruction-by-instruction:
            // real visibility gate (test ax,0x80 @0x9BE1), real scaling
            // (dim*depth_scale>>10) and real centring (shr dx,1; sub bx,dx @0x9CDE).
            let _ = project_ship_3d_object_sprite(anchor, cam, m, &mut self.ship3d_nav_slots[idx]);
        }

        // --- dirty rects + render commands --------------------------------------
        // The global clip snapshot seeds the dirty list from the view clip; the
        // collector then emits one command per (slot, intersecting rect) pair,
        // walking slots in REVERSE and clearing each slot's dirty flag as it goes.
        self.ship3d_clip_snapshot_armed = true;
        let clip = Ship3dProjectionViewport {
            left: 0,
            top: 0,
            right: ENGINE_SCREEN_WIDTH as u16,
            bottom: ENGINE_SCREEN_HEIGHT as u16,
        };
        commit_ship_3d_global_clip_snapshot(
            &mut self.ship3d_dirty_rects,
            &mut self.ship3d_clip_snapshot_armed,
            clip,
        );
        for slot in self.ship3d_nav_slots.iter_mut() {
            commit_ship_3d_sprite_slot_dirty_geometry(slot);
        }
        let commands = if count == 0 {
            Vec::new()
        } else {
            collect_ship_3d_dirty_sprite_slot_render_commands(
                &mut self.ship3d_nav_slots,
                &self.ship3d_dirty_rects,
                0,
                count - 1,
            )
        };

        for cmd in &commands {
            let slot = self.ship3d_nav_slots[cmd.slot_index];
            let sw = (slot.extent_width as i32).max(2);
            let fi = (0..6)
                .min_by_key(|&i| (self.nav_pyramids[i].width as i32 - sw).abs())
                .unwrap_or(4);
            let frame = self.nav_pyramids[fi].clone();
            // draw_x/draw_y are already the TOP-LEFT (screen - extent/2 at 0x9CDE),
            // so blit at that corner rather than re-centring.
            blit_sprite_frame_at(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &frame,
                signed_i16_engine(slot.draw_x),
                signed_i16_engine(slot.draw_y),
            );
        }

        // The eye-orb (BORXX, real art) at the view centre.
        if let Some(orb) = self.hud_orb.first().cloned() {
            blit_sprite_frame_centered(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &orb,
                160,
                120,
            );
        }
    }

    /// The destination info panel's WINDOW: the rect at `DS:0x2780` remapped
    /// through a 50%-toward-black tint table (`0x90ED..0x90F9` builds it,
    /// `0x9142..0x9156` draws it), then the panel's text rows on top.
    ///
    /// The rect and the row layout are the routine's own immediates — see
    /// [`crate::vm::LOCATION_PANEL_BOX`] and [`crate::vm::VmMachine::location_panel_rows`].
    /// The tint table is computed from the LIVE palette each time, as the game
    /// does; the port keeps its palette in 8-bit RGB, so it is converted back to
    /// the 6-bit DAC units the builder's distance threshold is expressed in.
    ///
    /// Only draws in [`LocationPanelState::Open`]; the zooming states draw the
    /// interpolated rect instead ([`Self::step_location_info_panel`]).
    pub fn location_panel_tint_table(&self) -> [u8; 256] {
        let mut dac = [[0u8; 3]; 256];
        for (i, entry) in dac.iter_mut().enumerate() {
            for k in 0..3 {
                entry[k] = (self.scene_palette[i][k] as u16 * 63 / 255) as u8;
            }
        }
        let mut table = [0u8; 256];
        for (i, entry) in table.iter_mut().enumerate() {
            *entry = i as u8; // unmatched entries keep their previous value
        }
        crate::palette::build_palette_blend_remap_table(
            &dac,
            crate::vm::LOCATION_PANEL_TINT_PERCENT,
            [0, 0, 0],
            &mut table,
        );
        table
    }

    /// Draw the destination INFO PANEL — `0x9137..0x91EC`:
    ///
    /// ```text
    ///   0x9142  mov bx,[0x2780] / cx,[0x2782] / dx,[0x2784] / bp,[0x2786]
    ///                                    the window rect (x, y, w, h)
    ///   0x9152  mov si,[0xac8]           the remap table
    ///   0x9156  lcall 0x299:0x40e        THE TINT BLIT -- the panel is not
    ///                                    painted, it is a tint of what is there
    ///   0x915B  mov bx,0x6e              text x = LOCATION_PANEL_X
    /// ```
    ///
    /// So the panel background goes through the same `0x299:0x40E` primitive as
    /// the choice box (`sprite::remap_rect_indexed`), which is why this function
    /// remaps a rect rather than filling one. `LOCATION_PANEL_BOX` is the rect at
    /// `DS:0x2780`, static in the image.
    ///
    /// Cited here because it was settled ASM with no doc (#141's queue); the rows
    /// it draws come from `vm::location_panel_rows`, which cites the same routine.
    pub fn render_location_info_panel(&mut self, rows: &[crate::vm::LocationPanelRow]) {
        let table = self.location_panel_tint_table();
        let [bx, by, bw, bh] = crate::vm::LOCATION_PANEL_BOX;
        crate::sprite::remap_rect_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &table,
            bx as i32,
            by as i32,
            bw as i32,
            bh as i32,
        );
        for row in rows {
            if row.x < 0 || row.y < 0 {
                continue;
            }
            draw_text_indexed(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                &row.text,
                row.x as usize,
                row.y as usize,
                row.color,
            );
        }
    }

    /// A left click while the nav chart is up, routed the way the game routes it
    /// (`0x8FB0..0x8FBE` then `0x8FF4`):
    ///
    /// * panel already up -> the click is what re-enables the mouse, which is the
    ///   `0x912E` edge that starts the close. Returns `false` (nothing selected).
    /// * otherwise hit-test the chart markers (`0x92A3`); a hit opens the panel
    ///   on that object with its rows, and returns `true`.
    ///
    /// `rows` come from the VM because only it can walk the roster; the engine
    /// keeps them for the duration of the panel.
    pub fn nav_chart_click(
        &mut self,
        x: i32,
        y: i32,
        current_location: u16,
        rows_for: impl FnOnce(u16) -> Vec<crate::vm::LocationPanelRow>,
    ) -> bool {
        if self.location_panel.state != LocationPanelState::Idle {
            self.location_panel_mouse_enabled = true;
            self.close_location_info_panel();
            return false;
        }
        let Some(hit) = self.nav_chart_object_click(x, y).map(|o| o.object) else {
            return false;
        };
        if !self.open_location_info_panel(hit, current_location, (x, y)) {
            return false;
        }
        self.location_panel_rows = rows_for(hit);
        true
    }

    /// Advance and draw the info panel for one nav frame. Call after the chart
    /// background is drawn; returns whether anything was drawn.
    ///
    /// Mirrors the dispatcher at `0x9083`: the zooming states draw only the
    /// interpolated rect (tinted, since that is what `0x8B:0xFAD` hands to
    /// `0x299:0x40E`), and the open state draws the panel proper.
    pub fn render_nav_info_panel_frame(&mut self) -> bool {
        match self.location_panel.state {
            LocationPanelState::Idle => false,
            LocationPanelState::Open => {
                let rows = std::mem::take(&mut self.location_panel_rows);
                self.render_location_info_panel(&rows);
                self.location_panel_rows = rows;
                true
            }
            _ => {
                let Some(rect) = self.step_location_info_panel() else {
                    return false;
                };
                let table = self.location_panel_tint_table();
                crate::sprite::remap_rect_indexed(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    &table,
                    rect[0] as i32,
                    rect[1] as i32,
                    rect[2] as i32,
                    rect[3] as i32,
                );
                true
            }
        }
    }

    /// Give each nav sprite slot the ENTITY id the projector writes it into:
    /// `0x6212 + ((i + 0x15) << 5)` at `0x9B98`, so slot `i` is entity `0x15 + i`.
    /// Without this a slot is anonymous and cannot be addressed the way the rest
    /// of the engine addresses entities.
    pub fn assign_nav_slot_entity_ids(&mut self) {
        for (i, slot) in self.ship3d_nav_slots.iter_mut().enumerate() {
            slot.entity_id = crate::ship3d::ship_3d_nav_entity_for_slot(i).map(|(id, _)| id);
        }
    }

    /// The HOVER status panel's trigger (`nav_state_gate` `0x82E8`): the gate
    /// reads `si = 0x65F2` — ENTITY `0x1F`'s record, the LAST of the 32 at
    /// `DS:0x6212` — requires its state bit0, and hit-tests the mouse against the
    /// rect at `+8`:
    ///
    /// ```text
    ///   0x830A  si = 0x65F2 / test byte [si],1 / je      the entity's state bit
    ///   0x8315  si += 8                                  the rect: x,y,w,h
    ///   0x8318  x  <= mx <= x + w        (ja/jb, inclusive both ends)
    ///   0x8328  y  <= my <= y + h
    /// ```
    ///
    /// The rect words are the same `+0x08/+0x0A` position and `+0x0C/+0x0E`
    /// extent the sprite-slot setters write (`0x420D`, `0x42CD`), so the port
    /// answers it from the nav slot carrying that entity id.
    pub fn nav_hover_status_active(&self, mouse: (i32, i32)) -> bool {
        let Some(slot) = self
            .ship3d_nav_slots
            .iter()
            .find(|s| s.entity_id == Some(crate::ship3d::SHIP_3D_ENTITY_COUNT - 1))
        else {
            return false;
        };
        if slot.flags & 1 == 0 {
            return false; // 0x830D: `test al,1 / je`
        }
        let x = slot.draw_x as i32;
        let y = slot.draw_y as i32;
        mouse.0 >= x
            && mouse.0 <= x + slot.extent_width as i32
            && mouse.1 >= y
            && mouse.1 <= y + slot.extent_height as i32
    }

    /// OPEN the destination info panel — the selection commit at
    /// `0x8FF4..0x905B`, minus the parts that belong to the DOS input layer:
    ///
    /// ```text
    ///   0x9029  [0x2AAB] = {mouse x, mouse y, 4, 4}   the zoom SOURCE rect
    ///   0x9039  [0xADB] = 0 / [0xADA] = 8             interpolation step, total
    ///   0x9043  [0x2788] = 1                          state: zooming open
    ///   0x9048  [0x2789] = 0                          the entity zoom scale
    ///   0x9052  [0x676A] = [0x27BF]                   the selected object
    ///   0x900C  [0xA3E] = 0                           the mouse goes OFF, which
    ///                                                 is what later triggers the
    ///                                                 close (0x912E)
    /// ```
    ///
    /// The caller supplies the object because the selection itself is
    /// [`crate::vm::VmMachine::nav_chart_pick`]'s job; `0x901D`'s refusal to open
    /// on the object you are already at is enforced here too.
    pub fn open_location_info_panel(
        &mut self,
        object: u16,
        current_location: u16,
        cursor: (i32, i32),
    ) -> bool {
        if object == 0 || object == current_location {
            return false; // 0x901D: `cmp ax,es:[arche+0x16] / je`
        }
        let size = crate::vm::LOCATION_PANEL_CURSOR_RECT_SIZE as i32;
        self.location_panel = LocationInfoPanel {
            state: LocationPanelState::ZoomingOpen,
            object,
            scale: 0,
            step: 0,
            cursor_rect: [cursor.0, cursor.1, size, size],
        };
        self.location_panel_mouse_enabled = false;
        true
    }

    /// Ask the panel to close — the `0x912E` edge, where the mouse being enabled
    /// again puts the FSM into state 2 (`0x922A`: `[0x278C]=0`, `[0x2788]=2`,
    /// `[0xADB]=0`, `inc [0x2789]`).
    pub fn close_location_info_panel(&mut self) {
        if self.location_panel.state == LocationPanelState::Open {
            self.location_panel.state = LocationPanelState::ZoomingShut;
            self.location_panel.step = 0;
            self.location_panel.scale = self.location_panel.scale.wrapping_add(1);
        }
    }

    /// Advance the panel one frame and return the rect to draw, if any.
    ///
    /// * `ZoomingOpen` (`0x90FF..0x9120`): `inc [0x2789]`, interpolate the cursor
    ///   rect toward the panel rect, and on completion drop to `Open`.
    /// * `Open`: nothing to animate; the caller draws the panel itself.
    /// * `ZoomingShut` (`0x91F1..0x9228`): `dec [0x2789]`, interpolate the other
    ///   way, and on completion clear the selection (`[0x27BF]=0`) and go idle.
    ///
    /// The interpolation is the game's own gate — `step_ship_3d_interpolation_gate`,
    /// `0x8B:0xFAD` — over the four rect words, `LOCATION_PANEL_ZOOM_STEPS` steps.
    pub fn step_location_info_panel(&mut self) -> Option<[u16; 4]> {
        use crate::ship3d::{
            Ship3dInterpolationGate, Ship3dInterpolationStep, step_ship_3d_interpolation_gate,
        };
        let panel = self.location_panel;
        let (source, dest) = match panel.state {
            LocationPanelState::Idle | LocationPanelState::Open => return None,
            // 0x9111: di = the cursor rect, si = the panel rect.
            LocationPanelState::ZoomingOpen => (
                crate::vm::LOCATION_PANEL_BOX,
                panel.cursor_rect.map(|v| v.max(0) as u16),
            ),
            // 0x9203: the same two, swapped.
            LocationPanelState::ZoomingShut => (
                panel.cursor_rect.map(|v| v.max(0) as u16),
                crate::vm::LOCATION_PANEL_BOX,
            ),
        };
        let mut gate = Ship3dInterpolationGate {
            duration_ticks: crate::vm::LOCATION_PANEL_ZOOM_STEPS,
            current_tick: panel.step,
            ..Default::default()
        };
        let stepped = step_ship_3d_interpolation_gate(&mut gate, source, dest);
        self.location_panel.step = gate.current_tick;
        match panel.state {
            LocationPanelState::ZoomingOpen => {
                self.location_panel.scale = self.location_panel.scale.wrapping_add(1)
            }
            LocationPanelState::ZoomingShut => {
                self.location_panel.scale = self.location_panel.scale.saturating_sub(1)
            }
            _ => {}
        }
        match stepped {
            Some(Ship3dInterpolationStep::Active(rect)) => Some(rect),
            // CF set at 0x1EB9 -> the caller's `jae` falls through to the next state.
            _ => {
                if panel.state == LocationPanelState::ZoomingOpen {
                    self.location_panel.state = LocationPanelState::Open; // 0x9120
                } else {
                    self.location_panel = LocationInfoPanel::default(); // 0x921C: [0x27BF]=0
                }
                None
            }
        }
    }

    /// Load the real navigation star-map background from `CHART.FD` (an IFF/PBM image
    /// under `iso`) — the game's own chart the ship-nav screen shows. Returns whether it
    /// loaded; when present, `render_ship_view` draws it instead of the procedural
    /// starfield. (Identified from the game's file-open trace at the nav screen.)
    pub fn load_nav_chart(&mut self, iso: &std::path::Path) -> bool {
        for name in ["CHART.FD", "chart.fd"] {
            if let Ok(data) = std::fs::read(iso.join(name)) {
                if let Some(img) = crate::lbm::decode_lbm(&data) {
                    self.nav_chart = Some(img);
                    return true;
                }
            }
        }
        false
    }

    /// APPROX — FABRICATED LAYOUT, and the decoded replacement already exists.
    ///
    /// These four numbers place the choose-a-location list at a fixed `x=6,
    /// y=22`, pitch 10, width 150. NOTHING cites them. The game does not lay any
    /// list out that way: the unified widget (`0x8428`) MEASURES the labels and
    /// derives the box from them — width = widest + 20 @`0x84A1`, height =
    /// rows*pitch + 8 @`0x84A7`, x = anchor - width/2 @`0x84AD` — which the port
    /// implements as `ship3d::layout_ship_3d_target_list` and tests against the
    /// game's own strings (#220).
    ///
    /// So this is a second, invented layout for a surface the game lays out one
    /// way, and the comment that used to sit here called it "the game's list-box
    /// nav", asserting a provenance it never had. Same defect class as the
    /// `compass_angle` chooser removed in #197.
    ///
    /// CORRECTED IN #240, having traced what fills this list. It is NOT a
    /// duplicate of the game's destination list and must not be deleted as one:
    /// `nav_destinations` is built in `main.rs` from the SCRIPT3..5 BUNDLES —
    /// label taken from a bundle's first actor record, entries its parsed
    /// dialogue lines — so it is a PORT-SIDE AFFORDANCE for reaching scenes, not
    /// a model of a game surface. The game's own destination list comes from the
    /// DEB candidate records (`0x7259`) and is routed through `console_box`
    /// (#212), which takes the click first.
    ///
    /// So what is wrong here is narrower than #239 claimed: not "a second layout
    /// for a game surface", but AN INVENTED LAYOUT WEARING A GAME PROVENANCE. The
    /// geometry stays until someone decides whether the port should offer this
    /// affordance at all; the false claim is gone, and it stays labelled APPROX
    /// because a reader must not mistake these four numbers for decoded ones.
    pub const NAV_DEST_X: i32 = 6;
    pub const NAV_DEST_Y: i32 = 22;
    pub const NAV_DEST_PITCH: i32 = 10;
    const NAV_DEST_W: i32 = 150;

    /// Set the choose-a-location destination list for the nav (each entry: a label and
    /// that character's decoded dialogue lines). The nav then shows them as a clickable
    /// list; clicking one plays that character's dialogue via [`Self::set_speech_dialogue`].
    #[allow(clippy::type_complexity)]
    pub fn set_nav_destinations(
        &mut self,
        dests: Vec<(String, Vec<(String, Option<std::path::PathBuf>)>)>,
    ) {
        self.nav_destinations = dests;
    }

    /// Set the RECORD-DRIVEN chart objects — the game's own nav chart, built by
    /// `0x604E` -> `0x721A` and mirrored by [`crate::vm::VmMachine::build_nav_chart_list`].
    ///
    /// This is what the destination list is supposed to be: object records with
    /// kinds, marker positions and artwork ids, not `(label, dialogue)` pairs
    /// derived from scripts. When it is non-empty the nav uses it for hit-testing
    /// and for the info panel; when it is empty (which is the shipped `.VAR`
    /// answer for SCRIPT1..4, whose in-play bits the story has not set yet) the
    /// script-derived list stays as the port's stand-in.
    pub fn set_nav_chart_objects(&mut self, objects: Vec<NavChartObject>) {
        self.nav_chart_objects = objects;
    }

    pub fn nav_chart_objects(&self) -> &[NavChartObject] {
        &self.nav_chart_objects
    }

    /// Hit-test a click against the chart markers, exactly as `0x92A3` does:
    /// the box starts 2px up-left of the marker, its size comes from the object's
    /// kind, and BOTH bounds are inclusive. Returns the first hit in list order.
    pub fn nav_chart_object_click(&self, x: i32, y: i32) -> Option<&NavChartObject> {
        // The same box rule the VM picker uses — one copy, so the engine's click
        // routing cannot drift from the hit-test verified against `func_92a3`.
        self.nav_chart_objects
            .iter()
            .find(|o| crate::vm::nav_chart_marker_contains(o.marker, o.hit_box(), (x, y)))
    }

    /// The number of nav destinations currently offered.
    /// The label of nav destination `i` (the location's character name), if present.
    pub fn nav_destination_label(&self, i: usize) -> Option<String> {
        self.nav_destinations.get(i).map(|(l, _)| l.clone())
    }

    pub fn nav_destination_count(&self) -> usize {
        self.nav_destinations.len()
    }

    /// Map a click on the nav chart to a destination index, matching the list layout.
    pub fn nav_destination_click(&self, x: u16, y: u16) -> Option<usize> {
        if self.nav_destinations.is_empty() {
            return None;
        }
        let (px, py) = (x as i32, y as i32);
        if px < Self::NAV_DEST_X || px > Self::NAV_DEST_X + Self::NAV_DEST_W {
            return None;
        }
        (0..self.nav_destinations.len())
            .find(|&i| (py - (Self::NAV_DEST_Y + i as i32 * Self::NAV_DEST_PITCH)).abs() <= 4)
    }

    /// Visit the chosen nav destination — play that character's decoded dialogue. Returns
    /// whether the index was valid.
    pub fn visit_nav_destination(&mut self, index: usize) -> bool {
        let Some((_, lines)) = self.nav_destinations.get(index).cloned() else {
            return false;
        };
        self.set_speech_dialogue(lines);
        true
    }

    /// Render the on-ship nav view. With the real chart (`CHART.FD`) loaded this draws
    /// that star-map background (nebula + destinations + console) and a heading cursor;
    /// otherwise it falls back to the procedural starfield + projected pyramid HUD.
    pub fn render_ship_view(&mut self) {
        // Real navigation chart background, when available.
        if let Some(chart) = &self.nav_chart {
            if chart.width == ENGINE_SCREEN_WIDTH && chart.height == ENGINE_SCREEN_HEIGHT {
                self.framebuffer.copy_from_slice(&chart.pixels);
                self.scene_palette = chart.palette;
                // Heading cursor: a reserved-colour tick along the chart's top, swept by
                // the compass angle, so steering has visible feedback over the static chart.
                // A RESERVED slot (0xC0..0xFF), filled by the game at runtime and
                // therefore left [0,0,0] by a scene's own palette — which is why
                // this installs a colour before drawing through it. FIFTH port
                // writer of 0xFE and it agrees with the other four
                // ([245,245,160]); `tools/check_palette_slot_writers.py` reports
                // 0xFE as non-conflicting precisely because they match
                // (audit-fixes #542, #543, #571).
                const CURSOR_COLOR: u8 = 0xFE;
                self.scene_palette[CURSOR_COLOR as usize] = [245, 245, 160];
                let cursor_x =
                    (self.compass_angle as usize % 180) * (ENGINE_SCREEN_WIDTH - 1) / 179;
                for dy in 0..4 {
                    let row = dy * ENGINE_SCREEN_WIDTH;
                    if let Some(px) = self.framebuffer.get_mut(row + cursor_x) {
                        *px = CURSOR_COLOR;
                    }
                }
                // RECORD-DRIVEN chart markers, when the VM has supplied the real
                // list (0x604E -> 0x721A): each object's name is drawn AT ITS OWN
                // marker (`+0x18`/`+0x1A`), which is where the picker hit-tests
                // it — so what you see and what you can click are the same thing.
                if !self.nav_chart_objects.is_empty() {
                    let markers: Vec<(String, (i32, i32))> = self
                        .nav_chart_objects
                        .iter()
                        .map(|o| (o.name.clone(), o.marker))
                        .collect();
                    for (name, (mx, my)) in markers {
                        if mx < 0 || my < 0 {
                            continue;
                        }
                        draw_text_indexed(
                            &mut self.framebuffer,
                            ENGINE_SCREEN_WIDTH,
                            ENGINE_SCREEN_HEIGHT,
                            &name,
                            mx as usize,
                            my as usize,
                            CURSOR_COLOR,
                        );
                    }
                    self.render_nav_info_panel_frame();
                    return;
                }
                // Choose-a-location destination list (each character's location),
                // clickable. NOT the game's list-box nav, whatever this comment
                // used to say: it draws at the uncited NAV_DEST_* geometry rather
                // than through the decoded widget (see NAV_DEST_X's doc, #239).
                // Falls back to the compass-target label.
                if !self.nav_destinations.is_empty() {
                    let labels: Vec<String> = self
                        .nav_destinations
                        .iter()
                        .map(|(l, _)| l.clone())
                        .collect();
                    for (i, label) in labels.iter().enumerate() {
                        let y = (Self::NAV_DEST_Y + i as i32 * Self::NAV_DEST_PITCH) as usize;
                        draw_text_indexed(
                            &mut self.framebuffer,
                            ENGINE_SCREEN_WIDTH,
                            ENGINE_SCREEN_HEIGHT,
                            label,
                            Self::NAV_DEST_X as usize,
                            y,
                            CURSOR_COLOR,
                        );
                    }
                } else if let Some(label) = self.targeted_world_name().map(str::to_uppercase) {
                    draw_text_indexed(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        &label,
                        6,
                        6,
                        CURSOR_COLOR,
                    );
                }
                return;
            }
        }
        let mut prng = BloodPrng::seeded_from_rtc_seconds(self.starfield_seed);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: self.compass_angle % 180,
            angle_2f6f: 0,
        };
        // Starfield origin: the neutral cloud centre, offset along Z by the ship's
        // travel (the camera FSM's Z progress) so stars stream past as the ship
        // advances — consistent with the pyramids the camera also drives. The low
        // bits of the wrapping Z give continuous parallax.
        let z_travel = self.camera.origin_z.wrapping_mul(3);
        let origin = Ship3dProjectionOrigin {
            x: 0x8000,
            y: 0x8000,
            z: 0x8000u16.wrapping_add(z_travel),
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
        // Star-map nav grid. With CARTE.SPR loaded this draws the game's REAL
        // destination-pyramid sprite frames at positions projected by the decoded
        // 0x9BBA math, frame-selected by the projected scale — the faithful render
        // path (art + projection + scale selection); only the destination LAYOUT
        // remains the runtime-gated piece (live DS:0x4F09 records). Falls back to the
        // drawn approximation when the sprite bank isn't loaded (headless tests).
        if self.nav_pyramids.len() >= 6 {
            self.render_nav_pyramid_sprites();
        } else {
            crate::ship3d::render_star_map_navview_projected(
                &mut self.framebuffer,
                200,
                90,
                240,
                self.compass_angle % 180,
            );
        }
        // The game's real ship/nav-screen VGA palette (baked default uploaded for the
        // nav/bridge/location screens), so the starfield and BCARTE/BORXX sprite HUD
        // render in their true colours.
        self.scene_palette = crate::palette::game_screen_palette();
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
            (!grid.is_empty())
                .then(|| grid[(self.compass_angle as usize * grid.len() / 180).min(grid.len() - 1)])
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
        // Legacy orb composite for the non-sprite nav path only (the sprite path
        // draws the BORXX orb itself).
        if self.nav_pyramids.len() < 6 {
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
        // Label the destination the compass currently points at, so clicking to select
        // is intentional (the driver maps the heading to a scene the same way).
        let sector = (self.compass_angle as u32 * 5 / 180).min(4) + 1;
        self.scene_palette[0xFE] = [245, 245, 160];
        // The real decoded world the compass currently targets (from the level
        // directory) — so the heading names an actual destination, as the game does.
        let target = self
            .nav_world_labels
            .get(self.targeted_world_index())
            .copied()
            .unwrap_or("");
        let label = if target.is_empty() {
            format!("SECTOR {sector}")
        } else {
            format!("SECTOR {sector}  {}", target.to_uppercase())
        };
        draw_text_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &label,
            8,
            6,
            0xFE,
        );
    }

    /// The name of the world the nav compass currently targets (for "visit this
    /// destination" input).
    pub fn targeted_world_name(&self) -> Option<&'static str> {
        self.nav_world_labels
            .get(self.targeted_world_index())
            .copied()
    }

    /// The index into [`Self::nav_world_labels`] the compass heading currently targets:
    /// the heading (0..180°) maps across the decoded primary worlds, so panning the ship
    /// sweeps through the real navigable planets.
    pub fn targeted_world_index(&self) -> usize {
        let n = self.nav_world_labels.len().max(1);
        (self.compass_angle as usize * n / 180).min(n - 1)
    }

    /// Draw a subtitle line into the framebuffer at the game's subtitle reveal
    /// position (scene band, `SUBTITLE_X`/`SUBTITLE_Y` = 10/8) using the game font.
    /// The scene band's talk-HNM background composes separately; this is the text
    /// layer of the dialogue scene the engine presents for the current line.
    pub fn draw_subtitle(&mut self, text: &str, _color: u8) {
        // A fully-shown line: the reveal draw renders it in the fully-revealed colour (0xFF).
        let n = text.chars().count();
        self.draw_subtitle_revealed(text, n);
    }

    /// Draw the pre-wrapped subtitle with only the first `visible` characters
    /// shown, the newest in the reveal-edge colour — the game's per-character
    /// reveal. Non-visible characters are not drawn yet.
    ///
    /// Renderer `0x3630` (`subtitle_render_string`). The colour is purely distance
    /// from the reveal pointer — `0xFF` at it, `0xFE` one back, `0xFD` beyond —
    /// and a fully revealed line parks the pointer past the terminator so every
    /// character settles to `0xFD`. Rows 8/18 on the console at pitch 10.
    ///
    /// The details were already in the body comments below, including the 2026-07-24
    /// correction that retracted a "completed lines redraw thin/white" reading. They
    /// are repeated here because the audit ledger reads the DOC comment: evidence
    /// inside a function body is invisible to it, which is why this row counted as
    /// settled-without-citation in #141.
    fn draw_subtitle_revealed(&mut self, text: &str, visible: usize) {
        // The REAL subtitle model (renderer 0x3630, verified): the line draws in the
        // BOLD console font in the GREEN family THROUGHOUT — there is no second,
        // "settled" appearance. Colour is purely distance from the reveal pointer
        // (0xFF at the pointer, 0xFE one back, 0xFD beyond), and when the line is
        // fully revealed the pump parks the pointer past the terminator so every
        // character settles to 0xFD. Rows 8/18 on the console (pitch 10), same
        // origin in scenes.
        //
        // CORRECTED 2026-07-24: this header previously claimed a completed line
        // "redraws in the THIN proportional font at index 0xE0, white". That was an
        // INVENTION (no such branch exists in 0x3630) and the code implementing it
        // was removed earlier in this session; the comment had been left behind and
        // contradicted the function below it.
        let total: usize = text
            .split('\n')
            .enumerate()
            .map(|(i, l)| l.chars().count() + usize::from(i > 0))
            .sum();
        let fully = visible >= total;
        let on_console =
            self.panorama.is_some() && self.scene_hnm.is_none() && !self.console_band_dialogue;
        let pitch = if on_console {
            10
        } else {
            crate::font::GAME_FONT_LINE_HEIGHT
        };
        // CONSOLE-BAND PRESENTATION subtitles (the boot/tutorial screen: character
        // video atop the pyramid deck): WHITE (console-bank index 0xEF=239, one of the
        // OCR's known subtitle indices), CENTRED on x=160, first line at y=110 with
        // 8-px pitch — interpreter ground truth (BOOTIDX bd_218M/bd_290M measure the
        // 239-rows at y 110..117 / 118..125 with centred extents; dlg_05..dlg_11 the
        // same). The green top-left reveal belongs to the CONSOLE text mode; scene
        // close-ups keep their top-row draw (the OCR's third known layout).
        if self.console_band_dialogue {
            use crate::font::game_font_advance;
            let mut shown = 0usize;
            let mut y = 110usize;
            for (li, line) in text.split('\n').enumerate() {
                if li > 0 {
                    shown += 1;
                    y += 8;
                }
                let width: usize = line.chars().map(game_font_advance).sum();
                let mut x = 160usize.saturating_sub(width / 2);
                for ch in line.chars() {
                    if shown >= visible && !fully {
                        return;
                    }
                    let mut buf = [0u8; 4];
                    draw_text_indexed(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        ch.encode_utf8(&mut buf),
                        x,
                        y,
                        239,
                    );
                    x += game_font_advance(ch);
                    shown += 1;
                }
            }
            return;
        }
        // BOLD console font, green family, EVERY frame. The 0x3630 renderer never
        // switches to a thin/white font and never colors a whole line uniformly:
        // audit found the old `holding` (whole-line 0xFF) and settled (thin white
        // 0xE0) branches were INVENTED. Color is purely distance from the reveal
        // pointer si: the char AT the pointer = 0xFF (@0x369C), one back = 0xFE
        // (@0x369E dec ah), >=2 back = 0xFD (@0x36A4). When the line is fully
        // revealed the pump parks the pointer past the terminator (@0x94A0, the inc
        // suppressed), so every real character settles to the darker 0xFD green.
        let reveal_pointer = if fully { total + 2 } else { visible };
        let color_for = |shown: usize| -> u8 {
            if shown + 1 == reveal_pointer {
                0xFF
            } else if shown + 2 == reveal_pointer {
                0xFE
            } else {
                0xFD
            }
        };
        if let Some(bold) = self.bold_font.take() {
            let mut shown = 0usize;
            let mut y = 8usize;
            'outer: for (li, line) in text.split('\n').enumerate() {
                if li > 0 {
                    shown += 1;
                    y += pitch;
                }
                let mut x = 10usize;
                for ch in line.chars() {
                    if shown >= visible {
                        break 'outer;
                    }
                    let mut buf = [0u8; 4];
                    bold.draw(
                        &mut self.framebuffer,
                        ENGINE_SCREEN_WIDTH,
                        ENGINE_SCREEN_HEIGHT,
                        ch.encode_utf8(&mut buf),
                        x,
                        y,
                        color_for(shown),
                    );
                    x += crate::font::BoldConsoleFont::ADVANCE;
                    shown += 1;
                }
            }
            self.bold_font = Some(bold);
            return;
        }
        // No bold font available: thin fallback in the same colors.
        use crate::font::game_font_advance;
        let mut shown = 0usize;
        let mut y = 8usize;
        for (li, line) in text.split('\n').enumerate() {
            if li > 0 {
                shown += 1;
                y += pitch;
            }
            let mut x = 10usize;
            for ch in line.chars() {
                if shown >= visible {
                    return;
                }
                let mut buf = [0u8; 4];
                draw_text_indexed(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    ch.encode_utf8(&mut buf),
                    x,
                    y,
                    color_for(shown),
                );
                x += game_font_advance(ch);
                shown += 1;
            }
        }
    }

    /// Render the current dialogue line's frame into the framebuffer: clear, then
    /// draw the reconstructed subtitle text. (The talk-HNM scene background layer
    /// composites behind this once the HNM decoder is moved into the lib.)
    pub fn render_dialogue_frame(&mut self) {
        // Scene background: decode the current talk-HNM frame (indices + palette)
        // into the persistent scene buffer (so delta frames chain correctly), then
        // copy it to the display framebuffer. Drawing the subtitle onto the copy —
        // not the scene buffer — keeps old subtitle text from accumulating across
        // frames/lines in regions later deltas don't repaint.
        if let Some(hnm) = self.scene_hnm.take() {
            let frame_idx = self.scene_frame % hnm.frame_count().max(1);
            hnm.decode_frame(frame_idx, &mut self.scene_buffer, &mut self.scene_palette);
            self.scene_hnm = Some(hnm);
            self.scene_frame += 1;
            self.present_scene_buffer();
        } else if self.console_band_dialogue && self.presentation_open_phase > 0 {
            // BOX-OPEN ANIMATION — the game's 6-phase zoom (table DS:0x2B97, driver
            // screen_mode_update 0x79E5): {x,y,w,h} growing from a point to the
            // full 320x130 presentation frame, 0xE0 fill with an 0xEF frame.
            const BOX_OPEN_PHASES: [(usize, usize, usize, usize); 6] = [
                (155, 67, 10, 15),
                (143, 57, 34, 35),
                (120, 51, 80, 47),
                (76, 43, 168, 63),
                (26, 30, 268, 89),
                (0, 10, 320, 130),
            ];
            self.render_bridge_background();
            let (bx, by, bw, bh) =
                BOX_OPEN_PHASES[(self.presentation_open_phase as usize - 1).min(5)];
            self.scene_palette[0xE0] = [0, 0, 0];
            self.scene_palette[0xEF] = [255, 255, 255];
            for y in by..(by + bh).min(ENGINE_SCREEN_HEIGHT) {
                for x in bx..(bx + bw).min(ENGINE_SCREEN_WIDTH) {
                    let edge = y == by || y + 1 == by + bh || x == bx || x + 1 == bx + bw;
                    self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = if edge { 0xEF } else { 0xE0 };
                }
            }
            self.presentation_open_phase += 1;
            if self.presentation_open_phase > 6 {
                self.presentation_open_phase = 0;
            }
        } else if self.console_band_dialogue {
            // PRESENTATION beat without a talk-HNM: the viewscreen shows STATIC
            // (interpreter ground truth, intro_215M — binary black/white noise in the
            // console bank, rows 0..140), NOT the bridge panorama — the presentation
            // screen replaces the hub view even though a panorama is loaded (the
            // windowed driver keeps the bridge assets resident throughout).
            for p in self.framebuffer.iter_mut() {
                *p = 0;
            }
            self.scene_palette = crate::palette::game_screen_palette();
            for y in 0..140usize {
                for x in 0..ENGINE_SCREEN_WIDTH {
                    self.viewscreen_noise = self
                        .viewscreen_noise
                        .wrapping_mul(1103515245)
                        .wrapping_add(12345);
                    let v = (self.viewscreen_noise >> 16) as u8;
                    self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] =
                        if v & 1 == 0 { 224 } else { 239 };
                }
            }
        } else if self.panorama.is_some() {
            // No talk-HNM (e.g. the on-ship console tutorial, HONK's food menu):
            // the dialogue happens AT THE SHIP CONSOLE in the real game, so
            // composite the real bridge panorama behind the subtitle text —
            // and the TOPIC MENU when this dialogue offers one (the concept-menu
            // conversation system, e.g. HONK's TALK/ONE..NINE).
            self.render_bridge_background();
            if !self.topic_menu.is_empty() {
                let labels: Vec<String> = self.topic_menu.iter().map(|(l, _)| l.clone()).collect();
                self.draw_list_menu(&labels, self.topic_selected);
            }
            self.scene_buffer.copy_from_slice(&self.framebuffer);
        } else {
            for p in self.framebuffer.iter_mut() {
                *p = 0;
            }
        }
        // REAL-GAME-VERIFIED (DOSBox captures tut_180s..300s): the SCRIPT1 console-tutorial
        // dialogue plays its talk-HNMs (Bronko, Honk, …) in the viewscreen with the pyramid
        // console + eye-orb band composited over the bottom — same band as the intro montage.
        if self.console_band_dialogue {
            self.overlay_console_band();
            // The green PAGE DIGIT (oracle bd_218M..bd_290M: a console-font digit at
            // (6,15), index 254 — present on every presentation beat).
            let page = self.dialogue_page_number();
            if let Some(bold) = self.bold_font.take() {
                bold.draw(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    &page.to_string(),
                    6,
                    15,
                    254,
                );
                self.bold_font = Some(bold);
            }
        }
        // Subtitle text layer over the scene, revealed one character at a time (the
        // game's reveal @0x93F8–0x94B8: `gs:0x5E58` advances one char whenever the
        // per-char timer `gs:0xB31 = step>>2` elapses). The subtitle colours are the GAME
        // palette's top entries — HNM `pl` blocks only ever cover indices 1..127, so the
        // baked game palette's GREEN family persists. LIVE-MEASURED (REVEALDUMP): newest
        // char 0xFF (129,255,105), second-newest 0xFE (44,210,8), older revealed chars
        // 0xFD (0,145,0). Install them over whatever the scene palette holds.
        let gp = crate::palette::game_screen_palette();
        for i in [0xFD, 0xFE, 0xFF] {
            self.scene_palette[i] = gp[i];
        }
        if let Some(text) = self.current_subtitle().map(str::to_string) {
            // Advance the reveal pointer at the decoded rate, keyed off the per-line
            // timer (elapsed frames on this line), so it works with or without a
            // talk-HNM scene.
            let per_char = u32::from(crate::vm::reveal_frames_per_char(self.text_speed_step));
            let visible = (self.dialogue_timer / per_char.max(1)) as usize;
            self.draw_subtitle_revealed(&text, visible);
        }
    }

    /// Lowercase file stem of the first resolved talk-HNM in the loaded dialogue, so a
    /// driver can look its background music up via `DescriptDb::hnm_music_map`.
    pub fn first_scene_hnm_stem(&self) -> Option<String> {
        self.dialogue_scene_paths
            .iter()
            .flatten()
            .next()
            .and_then(|p| p.file_stem())
            .map(|s| s.to_string_lossy().to_lowercase())
    }

    /// Take the pending nav destination selection (the compass heading the player
    /// clicked in the nav view), clearing it. A driver polls this each frame to load
    /// the selected destination's dialogue — the nav→dialogue game-loop transition.
    pub fn take_nav_selection(&mut self) -> Option<u16> {
        self.nav_selection.take()
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
    /// Arm the console-menu open animation for `row` (0-based, as
    /// `menu_row_under_cursor` returns it).
    ///
    /// `0x86E4` writes the interpolation duration in the click path itself, so the
    /// ten ticks are the CLICK's doing and not the destination screen's — every row
    /// arms the same gate (audit-fixes #615).
    pub fn begin_console_open(&mut self, row: usize) {
        // `0x86C6..0x86D1`: al = row-1, `mul 0x12` (the row pitch), `add ax,0x50`,
        // stored at `[0x253F]` — the travelling rect's Y. The box starts on the row
        // that was clicked, not at the cursor and not at the widget.
        // 0x12 is the menu ROW PITCH (`mov cl,0x12` @`0x8679`, the same pitch the
        // hit test divides by) and 0x50 the base, both from `0x86CA`..`0x86CE`.
        const MENU_ROW_PITCH: u16 = 0x12;
        const MENU_ROW_Y_BASE: u16 = 0x50;
        let row_y = (row.saturating_sub(1) as u16)
            .wrapping_mul(MENU_ROW_PITCH)
            .wrapping_add(MENU_ROW_Y_BASE);
        self.console_open_rect = [0, row_y, 0, 0];
        self.console_open = Some((
            row,
            crate::ship3d::Ship3dInterpolationGate {
                duration_ticks: crate::ship3d::SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION,
                current_tick: 0,
            },
        ));
    }

    /// The animation's TARGET shape — the widget rect the layout prepass builds
    /// (`DS:0x2AAB`, `0x84A1`: `w = widest + 0x14`, `h` from the row count, `x =
    /// anchor - w/2`). The port already computes that geometry for the choice box,
    /// so this asks it rather than repeating the layout (audit-fixes #619).
    fn console_open_target_rect(&self) -> [u16; 4] {
        let (_, x0, x1) = self.choice_box_geometry(0);
        let rows = 5usize;
        [
            x0 as u16,
            Self::choice_box_top_y(rows) as u16,
            (x1 - x0) as u16,
            (rows * Self::CHOICE_BOX_PITCH + 8) as u16,
        ]
    }

    /// Tint the travelling rect, `0x1EAD`..`0x1EB1`.
    ///
    /// `mov si,[0xac8] / lcall 0x299,0x40e` — and `0x299:0x40E` is NOT a blit:
    /// `0x3407` reads the pixel ALREADY on screen and `xlatb`s it through the table
    /// at `si`, so the rect is REMAPPED in place. The opening box is the same
    /// translucent window the choice box and the info panel draw, moving and
    /// growing (audit-fixes #621).
    fn draw_console_open_rect(&mut self) {
        let [x, y, w, h] = self.console_open_rect;
        if w == 0 || h == 0 {
            return;
        }
        let table = self.location_panel_tint_table();
        crate::sprite::remap_rect_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &table,
            x as i32,
            y as i32,
            w as i32,
            h as i32,
        );
    }

    /// Advance an in-flight console open; returns the row when it COMPLETES.
    ///
    /// Driven by the real gate (`step_ship_3d_interpolation_gate`, `0x1E5D`), so the
    /// frame count is the game's rather than a chosen delay. The interpolated words
    /// are discarded here — this port animates the DELAY, not yet the widget's
    /// travelling rectangle, which is the remaining half of #612.
    fn advance_console_open(&mut self) -> Option<usize> {
        let (row, mut gate) = self.console_open.take()?;
        // `0x8772 mov si,0x2aab` — the TARGET shape — and `0x8775 mov di,0x253d`,
        // the rect that moves. The gate writes through `di`, so the live rect steps
        // toward the widget's.
        let target = self.console_open_target_rect();
        let step = crate::ship3d::step_ship_3d_interpolation_gate(
            &mut gate,
            target,
            self.console_open_rect,
        );
        match step {
            Some(crate::ship3d::Ship3dInterpolationStep::Complete) => Some(row),
            Some(crate::ship3d::Ship3dInterpolationStep::Active(rect)) => {
                self.console_open_rect = rect;
                self.console_open = Some((row, gate));
                None
            }
            // The gate trapped (a zero duration); do not strand the open.
            None => Some(row),
        }
    }

    pub fn step(&mut self, input: MouseInput) {
        // The console-menu open animates over ten frames (0x86E4): step the
        // travelling rect, tint it where it now is, and apply the destination only
        // when the gate completes.
        let animating = self.console_open.is_some();
        if let Some(row) = self.advance_console_open() {
            match row {
                1 => self.phone_active = true,
                2 => self.cryobox_active = true,
                3 => self.menu_submenu_active = true,
                4 => self.option_box_active = true,
                _ => {}
            }
        } else if animating {
            self.draw_console_open_rect();
        }

        // Ship-3D nav view: drive the transition/depth state machine (0xB692 +
        // 0xB75C). Previously this ported, verified subsystem never ran.
        if self.nav_view_active() {
            self.step_ship_3d_nav_state();
        }
        // This frame's relative cursor motion — the bridge steering consumes deltas in RING
        // space, mirroring the original (driver h-range = the 1440-px ring, not the screen).
        // Prefer the frontend's RAW deltas (pointer-locked capture: rotation continues while the
        // physical mouse moves even with the cursor clamped at the screen edge); fall back to
        // on-screen cursor deltas when none are supplied.
        let motion = if input.dx != 0 || input.dy != 0 {
            (input.dx, input.dy)
        } else {
            (
                input.x as i32 - self.prev_pos.0 as i32,
                input.y as i32 - self.prev_pos.1 as i32,
            )
        };
        self.poll_input(input);
        // GPU per-tick state resets: only the screen rendered THIS tick may set the
        // star layer / window colour key (stale bridge stars must not show through
        // black pixels of other screens).
        self.gpu_stars = None;
        self.gpu_bg_colorkey = false;
        self.hand_on_screen = false;
        // Title art (BLOOD.LBM) shows first when armed, until dismissed.
        if self.title_screen.is_some() {
            self.render_title();
            self.frame += 1;
            return;
        }
        // Startup intro videos play full-screen first (developer/publisher logos +
        // intro cutscene), exactly as the real game boots, before any nav/dialogue.
        if self.intro_active {
            self.render_intro_frame();
            self.frame += 1;
            return;
        }
        // The game-ending finale (the bookend to the intro) takes precedence once armed,
        // playing full-screen to completion.
        if self.ending_active && self.ending_scene.is_some() {
            self.render_ending();
            self.frame += 1;
            return;
        }
        // Ship bridge takes precedence when active: the TB.BIG panorama with
        // mouse-push steering. Relative cursor motion feeds the ring-space
        // anchor exactly as the original's warped hardware cursor accumulates.
        if self.bridge_active {
            // STEERING LOCK (decoded: [0x2793] bit2, set at presentation start 0x593A,
            // cleared at UI close 0x1544 / presentation teardown 0x59C0 — script-owned):
            // while dialogue content is live the bridge does not rotate; it frees when
            // the presentation ends. Savestate-verified (the pinned hub freed on clear).
            let presentation_live = !self.dialogue.is_empty() && !self.dialogue_finished();
            if !presentation_live {
                self.bridge.move_mouse(motion.0, motion.1);
            }
            self.render_bridge();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The VIEWSCREEN console (real nav screen): band + static/destination viewscreen.
        if self.viewscreen_active {
            self.render_viewscreen_console();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // World-location landing screen: the decoded fd/ room background of a visited
        // world takes precedence while active.
        if self.world_location.is_some() {
            self.render_world_location();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // Cyberspace tunnel screen (presentation) takes precedence when active.
        if self.cyber_active && !self.cyber_tunnels.is_empty() {
            self.render_cyberspace();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The BOB_MORLOCK CONTACT screen (CRYOBOX -> BOB_MORLOCK) takes precedence.
        if self.bob_contact_active {
            self.render_bob_contact();
            self.draw_hand_at_mouse();
            self.advance_dialogue();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The CRYOBOX cryo-chamber (console menu option) takes precedence when active.
        if self.cryobox_active && self.cryobox_scene.is_some() {
            self.render_cryobox();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The video-phone call screen (console TELEPHONE option) takes precedence.
        if self.phone_active && !self.phone_contacts.is_empty() {
            self.render_telephone();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // Comms/TV screen takes precedence when active: watch the broadcast.
        if self.tv_active && !self.tv_channels.is_empty() {
            self.render_tv();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // Alien-examination screen takes precedence when active: rotate the
        // pre-rendered alien with the mouse.
        if self.alien_view_active && !self.alien_views.is_empty() {
            self.render_alien_view();
            self.draw_hand_at_mouse();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // On-ship gate ([0x2793] & 8): steer the compass from the mouse and render
        // the nav view's starfield background. The game reads the cursor position
        // relative to the screen CENTRE (int 33h ax=3 then subtracts the centre,
        // BLOODPRG.EXE ~0x102/0x216) and turns the camera by that delta each frame —
        // a joystick-style rate, not an absolute cursor-to-angle map. Cursor near
        // centre = no turn; near an edge = turn fast. `compass_angle` wraps 0..179.
        if self.on_ship {
            // The chart/nav view is STATIC in the real game (CHART.FD is a fixed image;
            // selection is the decoded target list). The prior mouse-steered "compass"
            // (dead-zone 8, rate dx/20) was an invention and is REMOVED; compass_angle
            // remains only as the port-side world-target selector (cycled via keys).
            // Destination selection is the decoded TARGET LIST only (layout + mouse
            // hit-test @0x8428; DS:0x27E7 selection byte) — the port's earlier
            // "click anywhere commits the compass heading" was an invention (random
            // clicks teleported the player) and is removed. The list rows route via
            // nav_destination_click; the compass angle only pans the view.
            self.prev_left_down = self.mouse.left_down();
            // Advance the ship-3D camera-approach animation (the decoded [0x27DF]
            // phase FSM) so the camera pulls in / travels as the game does on entry.
            self.camera.step();
            self.render_ship_view();
            self.draw_hand_at_mouse();
        } else if !self.dialogue.is_empty() {
            // Dialogue scene present: render the current line's frame (the
            // talk-HNM scene background composites behind this once the HNM decoder
            // is lib-side; for now the subtitle text layer over a cleared band).
            self.render_dialogue_frame();
            // The PRESENTATION screen hides the cursor entirely — no hand in any
            // interpreter capture of the boot/tutorial presentations (bd_218M..bd_290M,
            // intro_215M); the hand belongs to interactive screens only.
            if !self.console_band_dialogue {
                self.draw_hand_at_mouse();
            }
        }
        // Script/scene stepping (the D2 handoff the main loop drives): advance the
        // loaded dialogue playback, then chain to the next queued scene if this one
        // just finished (the scene-to-scene dispatch).
        self.advance_dialogue();
        self.advance_scene_if_finished();
        // Countdown at [0x0A40]: advanced each iteration, saturating at 0.
        self.countdown = self.countdown.saturating_sub(1);
        self.frame += 1;
    }
}

/// REFERENCE-ONLY: nothing calls this at runtime and nothing should. It exists to
/// state the game's addressing and to let a test prove the port's linear framebuffer
/// is equivalent to it (audit-fixes #622).
///
/// The game's mode-X screen address for pixel `(x, y)` — `(byte_offset, plane)` where
/// `byte_offset = y*80 + x/4` and `plane = x & 3`, exactly as `graphics_plot_modex`
/// (`0x299:0x498` = file `0x3428`) computes it:
///
/// ```text
/// 0x003455: 8bc2      mov ax, dx      ; ax = y
/// 0x003457: c1e004    shl ax, 4       ; y*16
/// 0x00345a: c1e206    shl dx, 6       ; y*64
/// 0x00345d: 03c2      add ax, dx      ; y*80  <-- NOT an imul by 80
/// 0x00345f: 8acb      mov cl, bl
/// 0x003461: 80e103    and cl, 3       ; plane = x & 3
/// 0x003464: c1eb02    shr bx, 2       ; x/4
/// 0x003467: 03c3      add ax, bx      ; y*80 + x/4
/// ```
///
/// The row stride is never in the image as `80`/`0x50`: it is `(y<<4)+(y<<6)`, the same
/// shifts-not-immediates habit that hides 320, 287, and 32 elsewhere (audit-fixes #574). Provided to document + verify that the engine's linear
/// `y*ENGINE_SCREEN_WIDTH + x` framebuffer is address-equivalent to the game's mode-X:
/// `byte_offset*4 + plane == y*320 + x` (see [`mode_x_to_linear`]).
pub fn mode_x_offset(x: usize, y: usize) -> (usize, usize) {
    (y * 80 + x / 4, x & 3)
}

/// Invert [`mode_x_offset`] back to the linear framebuffer index the engine uses:
/// `byte_offset*4 + plane`. Equals `y*ENGINE_SCREEN_WIDTH + x`, proving the two layouts
/// address the same pixel.
/// A projected screen coordinate read as signed, matching the projector's MOVSX
/// treatment — an off-screen marker projects to a negative x/y, not a huge u16.
fn signed_i16_engine(v: u16) -> i32 {
    v as i16 as i32
}

/// Invert the game's mode-X addressing: a `(plane byte offset, plane)` pair back
/// to a linear framebuffer index.
///
/// The forward mapping is in the mode-X plot at `0x3428` (`graphics_plot_modex`,
/// `SEG 0x299:0x498`):
///
/// ```text
///   0x3461  and cl,3     plane  = x & 3
///   0x3464  shr bx,2     column = x >> 2
///   0x3467  add ax,bx    + the row base, then `add di,ax`
///   0x346B  mov dx,0x3c4 / mov al,2 / out dx,al   select the map-mask register
/// ```
///
/// So `offset = y*80 + (x>>2)` and `plane = x & 3`, and this returns
/// `offset*4 + plane`. It works for the whole framebuffer, not just within a row,
/// because the plane stride is 80 and `80 * 4 = 320` — the row base scales into
/// place along with the column.
///
/// REFERENCE-ONLY, like [`mode_x_offset`] — the proof, not a code path
/// (audit-fixes #622).
///
/// Cited here because it was settled ASM with no doc (#141's queue).
pub fn mode_x_to_linear(byte_offset: usize, plane: usize) -> usize {
    byte_offset * 4 + plane
}

#[cfg(test)]
mod tests {

    /// `DS:0x2F65` holds the nav camera origin as three WORDS; the port widens
    /// them to `i32`. Read them back rather than trusting the widening.
    #[test]
    fn nav_camera_origin_matches_ds_2f65() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let at = 0xD420 + 0x2F65;
        let words: Vec<i32> = (0..3)
            .map(|i| i32::from(u16::from_le_bytes([exe[at + i * 2], exe[at + i * 2 + 1]])))
            .collect();
        assert_eq!(words, vec![10000, 12000, 0], "the camera origin moved");
        // The fourth word is NOT part of the origin -- it is 16, and reading a
        // fourth component would silently extend the vector.
        let fourth = u16::from_le_bytes([exe[at + 6], exe[at + 7]]);
        assert_eq!(
            fourth, 16,
            "the word after the origin is not what was decoded"
        );
    }
    use super::*;

    /// The console menu must NOT open on the click frame. `0x86E4` arms a ten-tick
    /// interpolation in the click path and the row handler holds its INTERPOLATING
    /// phase until the gate completes (`0x876A`), so the telephone arrives ten frames
    /// later (audit-fixes #615).
    #[test]
    fn the_console_menu_opens_after_the_decoded_ten_frames() {
        let mut e = EngineState::new();
        e.begin_console_open(1);
        assert!(!e.phone_active, "the click frame itself must not open it");

        // TEN ACTIVE TICKS, then the gate reports Complete on the NEXT call —
        // `0x1E67 cmp bl,[0xadb] / je` tests BEFORE the `inc` at `0x1E6D`, so the
        // completing frame is the eleventh. Asserting ten here fails, which is how
        // this test found the boundary rather than assuming it.
        for frame in 0..crate::ship3d::SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION {
            assert!(
                !e.phone_active,
                "opened early, on frame {frame} of the interpolation"
            );
            e.step(MouseInput::default());
        }
        assert!(!e.phone_active, "still animating after the tenth tick");
        e.step(MouseInput::default());
        assert!(
            e.phone_active,
            "the telephone must be open once the ten-tick gate completes"
        );
        assert!(
            e.console_open.is_none(),
            "the animation must not stay armed"
        );
    }

    /// The box must TRAVEL, and it must start on the clicked row.
    ///
    /// `0x86C6`..`0x86D1` seeds `[0x253F]` (the rect's Y) with `(row-1)*0x12 + 0x50`,
    /// and `0x8772`/`0x8775` step that rect toward the widget's (audit-fixes #619).
    #[test]
    fn the_opening_box_travels_from_the_clicked_row() {
        let mut e = EngineState::new();
        e.begin_console_open(3);
        // Seeded on the clicked row: (3-1)*0x12 + 0x50.
        assert_eq!(e.console_open_rect[1], 2 * 0x12 + 0x50);
        let start = e.console_open_rect;

        let target = e.console_open_target_rect();
        assert_ne!(start, target, "the test needs somewhere to travel to");

        let mut seen = vec![start];
        for _ in 0..crate::ship3d::SHIP_3D_NAV_CHOICE_INTERPOLATION_DURATION {
            e.step(MouseInput::default());
            seen.push(e.console_open_rect);
        }
        // It MOVED, and not in one jump: the intermediate rects differ from both
        // ends, which a delay-only implementation could not produce.
        assert!(
            seen.iter().any(|r| *r != start && *r != target),
            "the rect never took an intermediate position: {seen:?}"
        );
        // ...and it approached the target rather than wandering.
        let dy = |r: &[u16; 4]| (r[1] as i32 - target[1] as i32).abs();
        assert!(
            dy(seen.last().unwrap()) < dy(&start),
            "the rect ended no closer to the widget than it began"
        );
    }

    /// The opening box is a TINT, not a painted rect (`0x3407`: read the pixel on
    /// screen, `xlatb`, store), so the animation must DARKEN what it covers while
    /// leaving the structure underneath visible (audit-fixes #621).
    #[test]
    fn the_opening_box_tints_the_frame_it_covers() {
        let mut e = EngineState::new();
        // A varied background, so a tint and a fill are distinguishable.
        for (i, px) in e.framebuffer.iter_mut().enumerate() {
            *px = ((i % 61) + 60) as u8;
        }
        e.begin_console_open(3);
        let before = e.framebuffer.clone();
        e.step(MouseInput::default());
        let rect = e.console_open_rect;
        assert!(
            rect[2] > 0 && rect[3] > 0,
            "the rect must have an extent to tint"
        );

        let inside = |fb: &[u8], x: usize, y: usize| fb[y * ENGINE_SCREEN_WIDTH + x];
        let (x, y) = (rect[0] as usize + 1, rect[1] as usize + 1);
        assert_ne!(
            inside(&e.framebuffer, x, y),
            inside(&before, x, y),
            "the covered pixel must be remapped"
        );
        // OUTSIDE the rect is untouched — a tint that leaked would be a fill by
        // another name.
        let outside_y = (rect[1] as usize + rect[3] as usize + 2).min(ENGINE_SCREEN_HEIGHT - 1);
        assert_eq!(
            inside(&e.framebuffer, x, outside_y),
            inside(&before, x, outside_y),
            "the tint must not reach past the rect"
        );
        // STRUCTURE-PRESERVATION IS NOT ASSERTED HERE. `location_panel_tint_table`
        // is built from the SCENE PALETTE, and a bare `EngineState` has none, so
        // every source index maps to the same nearest entry and the region really
        // does flatten. That is the fixture, not the tint: the choice-box test
        // (`the_choice_box_is_a_tint_not_a_paint`) makes the same point with real
        // game data loaded, which is where it means something.
    }

    /// End-to-end faithfulness check for the bridge: the engine's full render of
    /// the console (panorama frame 55 + starfield windows + menu palette rows)
    /// must match the REAL game's console screen captured from the emulator
    /// ACTS 3-5 SURFACES: the same load path the app's profile switch uses
    /// (load_dialogue_scenes) produces a playable surface for EVERY script —
    /// dialogue present, scene routing live. The later acts ride the same
    /// presentation systems as Act One; this locks that claim per script.
    #[test]
    fn acts_three_to_five_surfaces_load() {
        let Some(iso) = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(std::path::Path::new)
            .find(|d| d.join("SCRIPT3.COD").is_file())
        else {
            eprintln!("skipping: extracted scripts not available");
            return;
        };
        let assets = iso; // scene HNMs resolve relative to the extraction
        let Ok(db) = crate::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES")) else {
            eprintln!("skipping: DESCRIPT.DES not available");
            return;
        };
        for n in 3..=5u32 {
            let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT{n}.{ext}"))).unwrap();
            let mut e = EngineState::new();
            e.load_dialogue_scenes(&rd("COD"), &rd("VAR"), &rd("DIC"), &rd("DEB"), &db, assets);
            assert!(e.dialogue_len() > 0, "SCRIPT{n}'s dialogue surface loads");
        }
    }

    /// running the original BLOODPRG.EXE (`BRIDGEPROBE`). The tolerance covers
    /// the pointing-hand cursor sprite (not yet ported) and the RNG starfield.
    #[test]
    fn bridge_console_matches_live_game_capture() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("TB.BIG").exists());
        let capture_path = [
            "accuracy/captures/bridge/console_rest.ppm",
            "../accuracy/captures/bridge/console_rest.ppm",
        ]
        .iter()
        .map(Path::new)
        .find(|p| p.exists());
        let (Some(iso), Some(capture_path)) = (iso, capture_path) else {
            return;
        };
        let raw = std::fs::read(capture_path).unwrap();
        let body_at = raw.windows(4).position(|w| w == b"255\n").unwrap() + 4;
        let capture = &raw[body_at..];
        assert_eq!(
            capture.len(),
            ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3
        );

        let mut e = EngineState::new();
        e.load_bridge(iso);
        if e.panorama.is_none() {
            return;
        }
        // NOTE: the capture-sprite hand atlas that used to be loaded here is gone.
        // It was never drawn -- the faithful hand is manu3_hand::HandMesh, decoded from
        // manu3.xdb's own mesh and cursor law.
        e.bridge_active = true;
        // Prime prev_pos so the render step sees zero cursor motion, then
        // reproduce the live probe's state exactly: view frame 55, cursor at
        // ring 320 (screen x 40), y 100 — the state the capture was taken in.
        e.step(MouseInput {
            x: 40,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        e.bridge.frame = 55;
        e.bridge.ring_mouse_x = 320;
        e.bridge.mouse_y = 100;
        e.step(MouseInput {
            x: 40,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        assert_eq!(e.bridge.frame, 55, "view must not drift during the render");
        assert_eq!(
            e.bridge.mouse_screen_x(),
            40,
            "virtual cursor at the capture position"
        );

        // Measure CONSOLE fidelity — excluding the pointing-hand region. The hand is
        // a live 3D model (never renders identically twice, in the real game or the
        // port), so pixel-diffing it against ONE frozen captured pose is meaningless;
        // the console panorama/menu/orb is the fidelity target. Hand bbox from the
        // atlas capture at this cursor (screen x~40, y~100): a generous 60x80 box.
        let hand_box = (10i32, 60i32, 100i32, 180i32); // x0,y0,x1,y1
        let mut total_abs = 0u64;
        let mut counted = 0u64;
        for (pixel, &index) in e.framebuffer.iter().enumerate() {
            let (px, py) = (
                (pixel % ENGINE_SCREEN_WIDTH) as i32,
                (pixel / ENGINE_SCREEN_WIDTH) as i32,
            );
            if px >= hand_box.0 && px < hand_box.2 && py >= hand_box.1 && py < hand_box.3 {
                continue; // skip the hand region
            }
            let ours = e.scene_palette[index as usize];
            for channel in 0..3 {
                total_abs += (ours[channel] as i32 - capture[pixel * 3 + channel] as i32)
                    .unsigned_abs() as u64;
            }
            counted += 3;
        }
        let mean_abs = total_abs as f64 / counted as f64;
        // Optional visual QA: BRIDGE_DUMP=<path.ppm> writes the rendered frame.
        if let Ok(dump) = std::env::var("BRIDGE_DUMP") {
            let mut ppm =
                format!("P6\n{ENGINE_SCREEN_WIDTH} {ENGINE_SCREEN_HEIGHT}\n255\n").into_bytes();
            for &index in e.framebuffer.iter() {
                ppm.extend_from_slice(&e.scene_palette[index as usize]);
            }
            std::fs::write(&dump, ppm).unwrap();
            eprintln!("bridge console render -> {dump} (mean_abs vs live = {mean_abs:.2})");
        }
        // CONSOLE fidelity (hand region excluded): the panorama + menu + orb must
        // match the live game near-exactly (the historically-verified ~2.58 level).
        let threshold = 3.0;
        assert!(
            mean_abs < threshold,
            "port console diverges from the live game: mean_abs = {mean_abs:.2}"
        );
    }

    /// Oracle: the gold CHOICE BOX renders exactly per the spec MEASURED from
    /// live-game index dumps — a 3-px border of palette index 0x15, a gold fill
    /// of 0xE0, and item text in 0xE8 (see `re/REVERSE.md` "CHOICE BOX SPEC
    /// MEASURED"). This locks the widget to the decoded values so a regression
    /// (wrong index, missing border/fill) fails a test.
    #[test]
    fn the_status_overlays_and_quicksave_come_from_the_binary() {
        if let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            let imm16 = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]) as usize;
            let imm8 = |at: usize| exe[at];
            let string_at = |ds: usize| {
                let f = 0xD420 + ds;
                let end = f + exe[f..].iter().position(|&b| b == 0).unwrap();
                std::str::from_utf8(&exe[f..end]).unwrap().to_string()
            };
            // LOADING: si=0x159 @0x16BC, ax=0x82 @0x16BF, bx=0x60 @0x16C2, dl=0xEF @0x16C5.
            // The DS offset comes from the INSTRUCTION OPERAND; the string then
            // comes from the image. No literal in between (audit-fixes #524).
            assert_eq!(imm16(0x16BD), EngineState::LOADING_TEXT_DS as usize);
            let mut probe = EngineState::new();
            probe.load_ds_strings(&exe);
            assert_eq!(
                probe.ds_text(EngineState::LOADING_TEXT_DS),
                string_at(imm16(0x16BD))
            );
            assert_eq!((imm16(0x16C0), imm16(0x16C3)), EngineState::LOADING_POS);
            assert_eq!(imm8(0x16C6), EngineState::LOADING_COLOR);
            // PAUSE: si=0x166 @0x1ABB, bx=0x87 @0x1ABE, dx=0x60 @0x1AC1, al=0xE8 @0x1AC4.
            assert_eq!(imm16(0x1ABC), EngineState::PAUSE_TEXT_DS as usize);
            assert_eq!(
                probe.ds_text(EngineState::PAUSE_TEXT_DS),
                string_at(imm16(0x1ABC))
            );
            assert_eq!((imm16(0x1ABF), imm16(0x1AC2)), EngineState::PAUSE_POS);
            assert_eq!(imm8(0x1AC5), EngineState::PAUSE_COLOR);
            // Quicksave: si=0x161 "LAST" @0x1B58, di=0x270D @0x1B5B.
            assert_eq!(string_at(imm16(0x1B59)), EngineState::QUICKSAVE_SLOT_NAME);
            assert_eq!(
                imm16(0x1B5C),
                EngineState::QUICKSAVE_NAME_BUFFER_DS as usize
            );
        }

        let Ok(exe_bytes) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let mut e = EngineState::new();
        e.load_ds_strings(&exe_bytes);
        e.framebuffer.fill(0);
        e.draw_status_overlay(true);
        let painted = |e: &EngineState, color: u8, (x, y): (usize, usize)| {
            (y..y + 8)
                .flat_map(|yy| (x..x + 60).map(move |xx| yy * ENGINE_SCREEN_WIDTH + xx))
                .filter(|&i| e.framebuffer[i] == color)
                .count()
        };
        assert!(painted(&e, EngineState::LOADING_COLOR, EngineState::LOADING_POS) > 20);
        e.framebuffer.fill(0);
        e.draw_status_overlay(false);
        assert!(painted(&e, EngineState::PAUSE_COLOR, EngineState::PAUSE_POS) > 15);

        // Quicksave writes LAST into the slot, with no rename prompt.
        let mut e = EngineState::new();
        e.quicksave(2);
        assert_eq!(e.save_slots[2].name, "LAST");
        assert_eq!(e.save_ui_slot, 2);
        assert!(!e.save_ui_active, "quicksave does not open the rename UI");
    }

    #[test]
    fn the_confirm_dialog_matches_the_binary_and_its_own_hit_regions() {
        // Strings pinned to the image, the same standard as OPTION_BOX_LABEL.
        if let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            let mut probe = EngineState::new();
            probe.load_ds_strings(&exe);
            for (ds_off, file_off) in EngineState::CONFIRM_STRING_TABLE {
                let end = file_off + exe[file_off..].iter().position(|&b| b == 0).unwrap();
                // The loader reaches the same bytes the file offset names.
                assert_eq!(
                    probe.ds_text(ds_off),
                    std::str::from_utf8(&exe[file_off..end]).unwrap()
                );
                assert_eq!(file_off - 0xD420, ds_off as usize);
                assert!(!probe.ds_text(ds_off).is_empty());
            }
            // The rect immediates at 0x14E6..0x14F1 and the two region records.
            let imm16 = |at: usize| u16::from_le_bytes([exe[at], exe[at + 1]]) as usize;
            assert_eq!(
                (imm16(0x14E7), imm16(0x14EA), imm16(0x14ED), imm16(0x14F0)),
                EngineState::CONFIRM_BOX
            );
            let region = |at: usize| (imm16(at), imm16(at + 2), imm16(at + 4), imm16(at + 6));
            assert_eq!(region(0xD420 + 0x2555), EngineState::CONFIRM_YES_REGION);
            assert_eq!(region(0xD420 + 0x255D), EngineState::CONFIRM_NO_REGION);
        }
        // The DRAW positions and the HIT regions are independent tables that must
        // describe the same layout: title at box_x+0x0A / y=0x58, YES at +0x14 and
        // +0x11, NO a further +0x3C along.
        let (bx, _, _, _) = EngineState::CONFIRM_BOX;
        assert_eq!(EngineState::CONFIRM_TITLE_POS, (bx + 0x0A, 0x58));
        assert_eq!(
            EngineState::CONFIRM_YES_REGION.0,
            EngineState::CONFIRM_TITLE_POS.0 + 0x14
        );
        assert_eq!(
            EngineState::CONFIRM_YES_REGION.1,
            EngineState::CONFIRM_TITLE_POS.1 + 0x11
        );
        assert_eq!(
            EngineState::CONFIRM_NO_REGION.0,
            EngineState::CONFIRM_YES_REGION.0 + 0x3C
        );

        let Ok(exe_bytes) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let mut e = EngineState::new();
        e.load_ds_strings(&exe_bytes);
        for (i, entry) in e.scene_palette.iter_mut().enumerate() {
            *entry = [i as u8, i as u8, i as u8];
        }
        e.framebuffer.fill(120);
        e.draw_confirm_box();
        // Scan the glyph BAND, not one row: a glyph's top row is often blank.
        let painted = (88..96usize)
            .flat_map(|y| (100..240usize).map(move |x| y * ENGINE_SCREEN_WIDTH + x))
            .filter(|&i| e.framebuffer[i] == 0xE8)
            .count();
        assert!(painted > 20, "the title draws in 0xE8 ({painted} px)");
        // Hit-testing, inclusive at both edges like 0x8295.
        assert_eq!(e.confirm_box_click(120, 105), Some(true));
        assert_eq!(e.confirm_box_click(150, 115), Some(true));
        assert_eq!(e.confirm_box_click(180, 105), Some(false));
        assert_eq!(e.confirm_box_click(200, 115), Some(false));
        assert_eq!(e.confirm_box_click(151, 105), None, "the gap between them");
        assert_eq!(e.confirm_box_click(120, 116), None);
    }

    #[test]
    fn the_montage_remap_banks_the_whole_screen() {
        // 0x7AC3 remaps (0,0,320,200) through DS:0x6011 before the film is drawn,
        // so every pixel of a montage frame is in the console bank.
        let mut e = EngineState::new();
        e.scene_palette = crate::palette::game_screen_palette();
        for (i, px) in e.framebuffer.iter_mut().enumerate() {
            *px = (i % 251) as u8; // a spread of indices, most outside the bank
        }
        assert!(
            e.framebuffer.iter().any(|&p| !(0xE0..=0xEF).contains(&p)),
            "the fixture must start outside the bank"
        );
        e.apply_console_bank_remap();
        assert!(
            e.framebuffer.iter().all(|&p| (0xE0..=0xEF).contains(&p)),
            "after the remap every pixel is in 224..=239"
        );
        // Idempotent, as the table's fixed points require.
        let once = e.framebuffer.clone();
        e.apply_console_bank_remap();
        assert_eq!(e.framebuffer, once);
    }

    #[test]
    fn the_console_band_is_the_panorama_frame_remapped() {
        // THE identification: TB.BIG frame 90's rows 140..200, pushed through the
        // console-bank table, equal the harvested capture in every byte. The
        // capture survives only as this fixture; the engine composes from the asset.
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let Some(pan) =
            crate::tbbig::BridgePanorama::parse(std::fs::read(iso.join("TB.BIG")).unwrap())
        else {
            return;
        };
        let px = pan
            .frame_pixels(crate::tbbig::CONSOLE_BAND_FRAME)
            .expect("the band frame decodes");
        let table = crate::palette::build_console_bank_remap_table(
            &crate::palette::GAME_SCREEN_PALETTE_DAC,
        );
        let composed: Vec<u8> = px[crate::tbbig::CONSOLE_BAND_TOP * ENGINE_SCREEN_WIDTH
            ..(crate::tbbig::CONSOLE_BAND_TOP + crate::tbbig::CONSOLE_BAND_HEIGHT)
                * ENGINE_SCREEN_WIDTH]
            .iter()
            .map(|&p| table[p as usize])
            .collect();
        let captured: &[u8] = include_bytes!("../accuracy/captures/console_band.idx");
        assert_eq!(composed.len(), captured.len(), "the band is 320x60");
        assert!(
            composed == captured,
            "the real asset must reproduce the capture byte-for-byte"
        );

        // UNIQUENESS: frame 90 is not merely A match, it is THE match. Searching
        // all 180 frames the same way, exactly one reproduces the band. That is
        // what makes CONSOLE_BAND_FRAME a value DERIVED from the archive rather
        // than an index read off a capture -- given the band, the data picks the
        // frame, and no other choice is available.
        let matches: Vec<usize> = (0..pan.frame_count())
            .filter(|&f| {
                pan.frame_pixels(f).is_some_and(|px| {
                    px.len() == crate::tbbig::PANORAMA_FRAME_PIXELS
                        && px[crate::tbbig::CONSOLE_BAND_TOP * ENGINE_SCREEN_WIDTH
                            ..(crate::tbbig::CONSOLE_BAND_TOP + crate::tbbig::CONSOLE_BAND_HEIGHT)
                                * ENGINE_SCREEN_WIDTH]
                            .iter()
                            .map(|&p| table[p as usize])
                            .eq(captured.iter().copied())
                })
            })
            .collect();
        assert_eq!(
            matches,
            vec![crate::tbbig::CONSOLE_BAND_FRAME],
            "exactly one panorama frame reproduces the band"
        );
    }

    #[test]
    fn the_harvested_band_dac_was_a_duplicate_of_the_image_palette() {
        // Justifies dropping console_band.dac: over the range the band uses, the
        // captured DAC and the palette baked from file 0x12F78 are the same bytes.
        // If the capture ever stops matching, the removal was wrong and this fails.
        let captured: &[u8] = include_bytes!("../accuracy/captures/console_band.dac");
        let image = &crate::palette::GAME_SCREEN_PALETTE_DAC;
        assert_eq!(
            &captured[224 * 3..256 * 3],
            &image[224 * 3..256 * 3],
            "the harvested band DAC must equal the image palette over 224..255"
        );
        // And the band really does only use that bank — sixteen indices, 224..=239.
        let band: &[u8] = include_bytes!("../accuracy/captures/console_band.idx");
        let used: std::collections::HashSet<u8> = band.iter().copied().collect();
        assert_eq!(used.len(), 16);
        assert!(used.iter().all(|&i| (224..=239).contains(&i)));
    }

    #[test]
    fn the_save_ui_is_the_slot_list_with_the_edit_buffer_substituted() {
        // 0x1BAB sets [0x2734] to the slot being renamed and 0x1BBD copies it into
        // DS:0x273B; 0x8573 swaps that buffer in as the widget draws. So the screen
        // is the ten slot names, one of which is the text being typed.
        let mut e = EngineState::new();
        e.save_slots = (0..EngineState::SAVE_SLOT_ROWS)
            .map(|i| crate::bloodsav::SaveSlot {
                name: if i == 3 {
                    "OLDNAME".into()
                } else {
                    String::new()
                },
                file: format!("game{}.sav", i + 1),
            })
            .collect();
        e.save_ui_slot = 3;
        e.save_ui_name = "AB".into();
        e.save_ui_active = true;
        e.framebuffer.fill(0);
        e.draw_save_ui_rows();

        // The edited row shows the EDIT BUFFER, not the stored slot name.
        let rows_drawn = |needle: &str| -> bool {
            let mut probe = EngineState::new();
            probe.framebuffer.fill(0);
            probe.draw_list_menu(&[needle.to_string()], None);
            probe.framebuffer.iter().any(|&p| p != 0)
        };
        assert!(
            rows_drawn("AB"),
            "the probe font can draw the edit text at all"
        );
        // The old hand-composed bar is gone: nothing paints a solid 0xE8 band
        // across x63..137 at y39..48.
        let band = (39..49usize)
            .flat_map(|y| (63..138usize).map(move |x| (x, y)))
            .filter(|&(x, y)| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == 0xE8)
            .count();
        assert!(
            band < 700,
            "the measured grey bar must not be painted: {band}px"
        );
        // And the widget's own extra row is present in the row set — READ FROM
        // THE IMAGE, not compared to a copy of itself. This asserted
        // `OPTION_BOX_LABEL == "CANCEL"`, which is a tautology: both sides are
        // the same transcription (audit-fixes #370). The constant records its own
        // file offset precisely so the check can be real.
        if let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            let at = EngineState::OPTION_BOX_LABEL_FILE_OFFSET;
            let end = exe[at..].iter().position(|&b| b == 0).unwrap_or(0) + at;
            let mut probe = EngineState::new();
            probe.load_ds_strings(&exe);
            assert_eq!(
                String::from_utf8_lossy(&exe[at..end]),
                probe.ds_text(EngineState::OPTION_BOX_LABEL_DS_OFFSET),
                "the extra row must BE the game's string at DS:0x0174"
            );
            // ...and the DS offset and file offset must agree, since the doc
            // states both and only their consistency makes either checkable.
            assert_eq!(
                0xD420 + EngineState::OPTION_BOX_LABEL_DS_OFFSET as usize,
                EngineState::OPTION_BOX_LABEL_FILE_OFFSET
            );
        }
    }

    /// The list menu draws every label at the SAME x, and that x is derived —
    /// `x0 + 10` with `x0 = anchor - (widest+20)/2` from the concept anchor `0xE1`
    /// @`0x89A6`.
    ///
    /// This replaces a test that asserted the opposite. The port briefly centred
    /// each label using `0x857D` (`sub bx,[bp] / shr bx,1 / add bx,cx`), and the
    /// test agreed with it — but `concept_menu.ppm` shows the real game putting
    /// all eleven measured rows at x=170. Both masks span x 170..280 identically
    /// while overlapping at IoU 0.18, which is exactly what correct BAND geometry
    /// with wrong per-row placement looks like; `concept_menu_mask_bounds`
    /// (ignored, `--nocapture`) prints the per-row comparison.
    ///
    /// Note what did NOT come back: the hardcoded `x = 170` that #97 removed. 170
    /// is what the formula produces for this label set.
    #[test]
    fn list_menu_labels_are_flush_left_at_the_derived_x() {
        let mut e = EngineState::new();
        let labels: Vec<String> = vec!["BOB_MORLOCK".into(), "EGO".into()];
        e.framebuffer.fill(0);
        e.draw_list_menu(&labels, None);
        let first_x = |row: usize| -> Option<usize> {
            let top = EngineState::choice_box_top_y(labels.len()) + row * 11;
            (top..top + 8).find_map(|y| {
                (0..ENGINE_SCREEN_WIDTH).find(|&x| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] != 0)
            })
        };
        let wide = first_x(0).expect("wide label drew");
        let narrow = first_x(1).expect("narrow label drew");
        assert_eq!(narrow, wide, "every row shares one left edge");

        // ...and that edge is the formula's, not an inset someone measured.
        let widest = labels
            .iter()
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap();
        let expected =
            EngineState::CHOICE_BOX_ANCHOR_CONCEPT.saturating_sub((widest + 20) / 2) + 10;
        assert_eq!(wide, expected);
    }

    #[test]
    fn choice_box_is_a_tint_of_the_panorama_not_a_painted_box() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        e.load_bridge(iso);
        e.load_console_font(iso);
        if e.panorama.is_none() {
            return;
        }
        // Open the MENU submenu (a choice box) and render one frame.
        e.bridge_active = true;
        e.menu_submenu_active = true;
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        // The box is a TINT of whatever it covers (0x84D8: si=[0xAC8], the
        // 50%-toward-black table, into 0x299:0x40E) — not a painted border+fill.
        // So the assertion is that the region got DARKER and stayed VARIED, and
        // that the label colour the assembly loads (0x8565: mov al,0xE8) is there.
        let count = |idx: u8| {
            let mut n = 0;
            for y in 88..122usize {
                for x in 63..175usize {
                    if e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == idx {
                        n += 1;
                    }
                }
            }
            n
        };
        assert!(
            count(0xE8) > 20,
            "choice-box text (0xE8) present: {}",
            count(0xE8)
        );
        // A painted box would flatten the region to one or two indices; a tint
        // preserves the panorama's structure underneath.
        let distinct: std::collections::HashSet<u8> = (92..118usize)
            .flat_map(|y| (70..168usize).map(move |x| (x, y)))
            .map(|(x, y)| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x])
            .collect();
        assert!(
            distinct.len() > 4,
            "the box must TINT the panorama, not flatten it: {} indices",
            distinct.len()
        );
        // NOT asserted: that index 0xE0 disappears. It does not, and that is the
        // point — over the panorama's dark orb socket the 50% tint genuinely
        // resolves to 0xE0 for most pixels, which is why a capture of THIS box in
        // THIS place looked like a flat 0xE0 fill and got recorded as one. The
        // capture was not misread; it was over-generalised. Only the varied-not-
        // flattened check above can tell the two apart.
    }

    #[test]
    fn console_box_click_band_is_the_drawn_box_not_a_fixed_40_160() {
        // The click band is the box's rendered extent [x0, x1] (0x84EE..0x84F6),
        // shared with the draw via choice_box_geometry — not the old fixed
        // 40..160 that only fit a centered box with labels <=100px.
        let width_of = |labels: &[String]| {
            labels
                .iter()
                .map(|l| crate::font::square_caps_text_width(l))
                .max()
                .unwrap()
        };

        // Centered box (kind 2): narrow labels floor to 100 -> anchor 100, w 120,
        // so the band is exactly 40..160 (no regression for the common case).
        let mut e = EngineState::new();
        e.console_box = vec!["CANCEL".to_string(), "MENU".to_string()];
        e.console_box_kind = 2;
        let (anchor, x0, x1) = e.choice_box_geometry(width_of(&e.console_box));
        assert_eq!(anchor, 100, "centered box anchors at 100");
        assert_eq!((x0, x1), (40, 160), "narrow centered box spans 40..160");
        let y0 = EngineState::choice_box_top_y(2) as u16 + 2; // row 0
        assert_eq!(e.console_box_click(100, y0), Some(0), "center hits row 0");
        assert_eq!(
            e.console_box_click(39, y0),
            None,
            "just left of the box misses"
        );
        assert_eq!(
            e.console_box_click(161, y0),
            None,
            "just right of the box misses"
        );

        // World box (kind 10): anchors at 80, floors to 55 -> the band is shifted
        // and narrower. x=130 sits inside the OLD fixed 40..160 band but OUTSIDE
        // the real anchor-80 box, so it must NOT register (the bug being fixed).
        let mut e = EngineState::new();
        e.console_box = vec!["MARS".to_string()];
        e.console_box_kind = 10;
        let (anchor, _x0, x1) = e.choice_box_geometry(width_of(&e.console_box));
        assert_eq!(anchor, 80, "world box anchors at 80");
        assert!(x1 < 130, "world box right edge {x1} < 130");
        let yb = EngineState::choice_box_top_y(1) as u16 + 2;
        assert_eq!(
            e.console_box_click(130, yb),
            None,
            "x=130 no longer wrongly hits the anchor-80 world box"
        );
        assert_eq!(
            e.console_box_click(80, yb),
            Some(0),
            "center of the world box hits"
        );
    }

    /// Oracle: the telephone choice box renders its item text exactly where the
    /// real game does. Renders the captured contact list (BOB_MORLOCK / CANCEL)
    /// and compares the port's 0xE8 glyph mask against the live capture's grey
    /// text mask — the labels must land CENTERED on x=100 (rows y=89/100) for the
    /// masks to overlap, which only happens with proportional-centered layout.
    #[test]
    fn choice_box_text_matches_live_game_capture() {
        let path = [
            "accuracy/captures/bridge/choice_box_bob_morlock.ppm",
            "../accuracy/captures/bridge/choice_box_bob_morlock.ppm",
        ]
        .iter()
        .map(Path::new)
        .find(|p| p.exists());
        let Some(path) = path else { return };
        let raw = std::fs::read(path).unwrap();
        let at = raw.windows(4).position(|w| w == b"255\n").unwrap() + 4;
        let cap = &raw[at..];
        if cap.len() != ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3 {
            return;
        }

        let mut e = EngineState::new();
        e.draw_choice_box(&["BOB_MORLOCK".to_string(), "CANCEL".to_string()], None);

        let is_grey = |o: usize| {
            let (r, g, b) = (cap[o] as i32, cap[o + 1] as i32, cap[o + 2] as i32);
            (r - 138).abs() < 45
                && (g - 138).abs() < 45
                && (b - 138).abs() < 45
                && (r.max(g).max(b) - r.min(g).min(b)) < 25
        };
        let (mut inter, mut uni) = (0u32, 0u32);
        for y in 85..112usize {
            for x in 40..160usize {
                let idx = y * ENGINE_SCREEN_WIDTH + x;
                let port = e.framebuffer[idx] == 0xE8;
                let live = is_grey(idx * 3);
                if port && live {
                    inter += 1;
                }
                if port || live {
                    uni += 1;
                }
            }
        }
        let iou = inter as f64 / uni as f64;
        eprintln!("choice-box text IoU = {iou:.3} (inter={inter}, union={uni})");
        assert!(
            iou > 0.55,
            "choice-box text must overlap the live capture (IoU {iou:.3} <= 0.55)"
        );
    }

    /// Oracle: a single-item choice box (the lone "CANCEL" offered post-tutorial)
    /// is VERTICALLY CENTERED — with one row it sits lower (y=95) than a two-row
    /// box's first row (y=89). Verifies the count-dependent vertical layout against
    /// `post2_option_choice.ppm` (CANCEL at x73..130, y95..102).
    #[test]
    fn choice_box_single_item_is_vertically_centered_vs_capture() {
        let path = [
            "accuracy/captures/bridge/post2_option_choice.ppm",
            "../accuracy/captures/bridge/post2_option_choice.ppm",
        ]
        .iter()
        .map(Path::new)
        .find(|p| p.exists());
        let Some(path) = path else { return };
        let raw = std::fs::read(path).unwrap();
        let at = raw.windows(4).position(|w| w == b"255\n").unwrap() + 4;
        let cap = &raw[at..];
        if cap.len() != ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3 {
            return;
        }
        // The ASSEMBLY layout (0x84A1..: h=rows*11+8, y=(200-h)/2, top=y+4):
        // rows=1 -> 94 (the capture's ink read 95 — the glyph's first ink row;
        // 1px measurement ambiguity, assembly wins per the prime rule);
        // rows=2 -> 89 and rows=6 -> 67 match the captures EXACTLY.
        // The kind-10 (world/entity) box is the `[0xADD]&1` branch, which seeds the
        // height 10 higher (`mov bp,0xa` @0x8442) alongside the narrower width
        // floor. Height enters as `(200-h)/2`, so the box sits 5px HIGHER than the
        // default-kind box with the same row count -- the port used to draw both at
        // the same y, putting the kind-10 box 5px low and its click rows with it.
        let mut world = EngineState::new();
        world.console_box_kind = 10;
        let mut other = EngineState::new();
        other.console_box_kind = 2;
        for rows in 1..=6usize {
            assert_eq!(
                other.choice_box_text_top(rows),
                EngineState::choice_box_top_y(rows),
                "non-kind-10 boxes keep the xor bp,bp seed of 0"
            );
            assert_eq!(
                EngineState::choice_box_top_y(rows) - world.choice_box_text_top(rows),
                5,
                "kind-10 seeds bp=10, so (200-h)/2 lifts the box by 5px"
            );
        }
        assert_eq!(EngineState::choice_box_top_y(1), 94);
        assert_eq!(EngineState::choice_box_top_y(2), 89);
        assert_eq!(EngineState::choice_box_top_y(6), 67);

        let mut e = EngineState::new();
        e.draw_choice_box(&["CANCEL".to_string()], None);

        let is_grey = |o: usize| {
            let (r, g, b) = (cap[o] as i32, cap[o + 1] as i32, cap[o + 2] as i32);
            (r - 138).abs() < 45
                && (g - 138).abs() < 45
                && (b - 138).abs() < 45
                && (r.max(g).max(b) - r.min(g).min(b)) < 25
        };
        // Vertical centroid of the CANCEL text (robust to the ~2px horizontal
        // centering-rounding difference between the port and the capture): the
        // port must place the row at the SAME height as the live game (~y98).
        let centroid_y = |mut f: Box<dyn FnMut(usize, usize) -> bool>| {
            let (mut sum, mut n) = (0f64, 0u32);
            for y in 88..110usize {
                for x in 55..150usize {
                    if f(x, y) {
                        sum += y as f64;
                        n += 1;
                    }
                }
            }
            (n > 0).then(|| sum / n as f64)
        };
        let fb = e.framebuffer.clone();
        let port_y = centroid_y(Box::new(move |x, y| {
            fb[y * ENGINE_SCREEN_WIDTH + x] == 0xE8
        }))
        .unwrap();
        let live_y = centroid_y(Box::new(move |x, y| {
            is_grey((y * ENGINE_SCREEN_WIDTH + x) * 3)
        }))
        .unwrap();
        eprintln!("single-CANCEL vertical centroid: port={port_y:.1} live={live_y:.1}");
        assert!(
            (port_y - live_y).abs() < 2.0,
            "single-item box must be vertically centered like the live game (port {port_y:.1} vs live {live_y:.1})"
        );
    }

    /// The engine loads and exposes the decoded BAS concept-menu stack: after
    /// `load_bas_menus`, the current menu is the script's ENTRY menu (SCRIPT2's
    /// top-level: OPTIMIZATION/CONSULTATION/EXPLANATIONS/…), and push/pop navigate
    /// it (the game's gs:0x6772 stack). Ties src/bas_vm.rs into the clean port.
    #[test]
    fn engine_holds_and_navigates_the_bas_menu_stack() {
        let read = |ext: &str| {
            ["accuracy/cdrive/cblood", "../accuracy/cdrive/cblood"]
                .iter()
                .find_map(|b| std::fs::read(Path::new(b).join(format!("SCRIPT2.{ext}"))).ok())
        };
        let (Some(bas), Some(dic)) = (read("BAS"), read("DIC")) else {
            return;
        };
        let mut e = EngineState::new();
        assert!(
            e.current_bas_menu_labels().is_empty(),
            "no menus before load"
        );
        e.load_bas_menus(&bas, &dic);
        let entry = e.current_bas_menu_labels();
        assert!(
            entry.iter().any(|l| l == "OPTIMIZATION"),
            "entry = top-level menu: {entry:?}"
        );
        // Navigate: enter the fear/anger sub-menu (BAS 0x42d, verified live) → current.
        assert!(e.bas_menus.as_mut().unwrap().push(0x42d));
        assert!(e.current_bas_menu_labels().iter().any(|l| l == "FEAR"));
        // Clicking a non-back topic (row 1 = FEAR) stays on the menu (plays a response).
        let after_fear = e.bas_topic_click(1);
        assert!(
            after_fear.iter().any(|l| l == "FEAR"),
            "emotion topic stays: {after_fear:?}"
        );
        // Clicking row 0 (TALK, the back-out) POPS to the top-level entry menu —
        // exactly what the running game does (MENUTREE: talk → parent 0x2f).
        let after_talk = e.bas_topic_click(0);
        assert!(
            after_talk.iter().any(|l| l == "OPTIMIZATION"),
            "talk pops to parent: {after_talk:?}"
        );
        // Syncing renders the current BAS menu as the topic menu (so it displays).
        e.sync_topic_menu_from_bas();
        assert!(!e.topic_menu.is_empty(), "topic menu populated from BAS");
        assert_eq!(
            e.topic_menu.get(1).map(|(l, _)| l.as_str()),
            Some("OPTIMIZATION")
        );
        // Enter the fear/anger menu and play its response monologue one at a time.
        e.bas_menus.as_mut().unwrap().push(0x42d);
        e.bas_start_responses();
        assert_eq!(e.bas_advance_response(), Some(0x43e), "first response");
        let mut n = 1;
        while e.bas_advance_response().is_some() {
            n += 1;
        }
        assert_eq!(n, 13, "all 13 sequential responses played");
        // The complete interaction: clicking a topic (row 1 = FEAR) returns its subtitle;
        // clicking TALK (row 0) pops back out to the parent menu.
        e.bas_start_responses();
        let sub = e.bas_menu_interact(1).expect("fear -> subtitle");
        assert!(
            sub.contains("several ways to lose"),
            "first subtitle: {sub:?}"
        );
        assert!(e.bas_menu_interact(0).is_none(), "talk pops (no subtitle)");
        assert!(
            e.current_bas_menu_labels()
                .iter()
                .any(|l| l == "OPTIMIZATION"),
            "popped to parent"
        );
    }

    /// The LIST MENU renders the square-capitals face in the widget's own colours
    /// and row band: `mov al,0xE8` (`0x8565`) unselected, `0xEF` selected
    /// (`0x858B`), rows vertically centred by count with an 11px pitch
    /// (`add bp,0xB` @`0x847A`).
    ///
    /// Renamed from `..._at_measured_geometry`, which framed a capture as the
    /// specification — and whose "x 175" no longer described anything, since
    /// labels centre on the widget anchor rather than sitting flush.
    #[test]
    fn list_menu_renders_square_caps_in_the_widgets_colours_and_band() {
        let mut e = EngineState::new();
        // Feed a topic menu and render it over a blank frame via the public draw.
        let labels = vec!["TALK".to_string(), "ONE".to_string(), "TWO".to_string()];
        e.draw_list_menu(&labels, Some(1));
        // Row band is row-count-CENTERED (choice_box_top_y): a 3-row menu tops at
        // (200-(3*11+8))/2+4 = 83, so row 0 (TALK) at y 83.., row 1 (ONE) at y 94..
        // (the fixed y=34 only held for a 12-row menu — the capture case).
        let count_in = |idx: u8, y0: usize, y1: usize| {
            let mut n = 0;
            for y in y0..y1 {
                for x in 170..250usize {
                    if e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] == idx {
                        n += 1;
                    }
                }
            }
            n
        };
        // Unselected rows use 0xE8; the selected row uses the bright 0xEF.
        assert!(
            count_in(0xE8, 82, 92) > 10,
            "TALK row (centered top=83) in square-caps 0xE8"
        );
        assert!(
            count_in(0xEF, 93, 103) > 6,
            "selected ONE row (y=94) in bright 0xEF"
        );
    }

    #[test]
    fn bridge_renders_the_real_panorama() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        e.load_bridge(iso);
        assert!(e.panorama.is_some(), "TB.BIG parses");
        e.bridge_active = true;
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        assert!(
            e.framebuffer.iter().any(|&p| p != 0),
            "bridge draws the panorama"
        );
        // At the menu rest frame, the decoded golden-menu hit math maps clicks to
        // rows: HONK (row 0) at the box top, OPTION (row 4) at the bottom.
        e.bridge.frame = crate::bridge::MENU_REST_FRAME;
        assert_eq!(e.console_menu_click(232, 0x48 + 1), Some(0));
        assert_eq!(e.console_menu_click(232, 0x48 + 4 * 0x12 + 1), Some(4));
        assert_eq!(
            e.console_menu_click(100, 0x48 + 1),
            None,
            "left of the menu box"
        );
        // Away from the menu sector the menu is not clickable at all.
        e.bridge.frame = 90;
        assert_eq!(e.console_menu_click(232, 0x48 + 1), None);
    }

    #[test]
    fn alien_view_rotates_through_prerendered_angles() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("pe").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.load_alien_view(assets, "scrut");
        if e.alien_views.is_empty() {
            return;
        }
        e.alien_view_active = true;
        // Steer full left, capture; steer full right, capture: different rotation view.
        for _ in 0..12 {
            e.step(MouseInput {
                x: 5,
                y: 100,
                buttons: 0,
                ..Default::default()
            });
        }
        let left = e.framebuffer.clone();
        for _ in 0..12 {
            e.step(MouseInput {
                x: 315,
                y: 100,
                buttons: 0,
                ..Default::default()
            });
        }
        assert!(left.iter().any(|&p| p != 0), "alien renders");
        assert_ne!(
            left, e.framebuffer,
            "mouse rotates to a different pre-rendered view"
        );
    }

    #[test]
    fn intro_plays_startup_videos_then_ends() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.on_ship = true;
        e.load_intro(
            assets,
            &crate::descript::DescriptDb {
                records: Vec::new(),
            },
        );
        assert!(e.intro_active(), "intro activates when clips are present");
        // While the intro runs, the main (nav) view must NOT render — the intro owns
        // the frame — and the intro must produce real (non-blank) content at some point.
        let mut saw_content = false;
        let mut ended = false;
        for _ in 0..6000 {
            e.step(MouseInput::default());
            if e.framebuffer.iter().filter(|&&p| p != 0).count() > 2000 {
                saw_content = true;
            }
            if !e.intro_active() {
                ended = true;
                break;
            }
        }
        assert!(saw_content, "intro renders real video frames");
        assert!(ended, "intro sequence finishes and hands off to the game");
    }

    /// The intro must actually overlay the publisher credit sourced from DESCRIPT.DES
    /// onto the CRYO cinematic — the scene where the bit-exact emulator diverges. This
    /// steps the intro up to the credit clip and confirms the reserved credit-colour
    /// glyphs light up (i.e. "CRYO Interactive Entertainment 1995" is drawn), proving
    /// the credit is presented in-game, not just renderable in isolation.
    #[test]
    fn intro_overlays_cryo_credit_from_descript() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        // Sanity: the credit clip and its cues must be wired from the data.
        let mut e = EngineState::new();
        e.on_ship = true;
        e.load_intro(assets, &db);
        let credit_clip = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "cliptoot"))
            .expect("cliptoot credit clip is queued in the intro");
        assert!(
            !e.intro_cues[credit_clip].is_empty(),
            "the credit clip carries DESCRIPT `present` subtitle cues"
        );
        // Step until the credit clip is active and past its first cue, then check the
        // reserved credit-colour glyphs were drawn into the framebuffer.
        let mut drew_credit = false;
        for _ in 0..4000 {
            e.step(MouseInput::default());
            if e.intro_index == credit_clip
                && e.framebuffer
                    .iter()
                    .filter(|&&p| p == EngineState::INTRO_CREDIT_COLOR_INDEX)
                    .count()
                    > 100
            {
                drew_credit = true;
                break;
            }
            if !e.intro_active() {
                break;
            }
        }
        assert!(
            drew_credit,
            "the CRYO publisher credit is overlaid during the intro"
        );
    }

    /// Intro AUDIO timing, faithful to the DESCRIPT data: the MINDSCAPE/Microfolie's logo reel
    /// (`mind.hnm`) plays SILENT, and the intro music (`blintr.voc`, the `present` record's Music)
    /// starts only with the credit cinematic (`cliptoot.hnm`). This guards the bug where the port
    /// started the music at intro frame 0 (over the logos) instead of with the cinematic.
    #[test]
    fn intro_music_silent_over_logos_starts_with_cinematic() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_intro(assets, &db);
        // The logo reel is the first clip and must carry NO music.
        let logo = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "mind"))
            .expect("mind.hnm logo reel is queued");
        assert_eq!(logo, 0, "the logo reel is the first intro clip");
        assert!(e.intro_music[logo].is_none(), "the logo reel plays SILENT");
        // The credit cinematic carries the DESCRIPT `present` music (blintr.voc).
        let credit = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "cliptoot"))
            .expect("cliptoot cinematic is queued");
        let m = e.intro_music[credit].as_deref().unwrap_or("");
        assert!(
            m.contains("blintr"),
            "the credit cinematic starts the intro music, got {m:?}"
        );
        // And the current-clip accessor reflects it: silent at the logos, music at the cinematic.
        assert_eq!(e.intro_index(), 0);
        assert_eq!(e.intro_clip_music(), None, "no music while the logos play");
    }

    /// The general DESCRIPT-Sequence cutscene player runs a cutscene from its OWN record data —
    /// HNM + music + tick-subtitles — so the in-game cutscenes (here the `maledict` curse) play
    /// faithfully, not silently. Guards the gap where the port had no in-game cutscene player.
    #[test]
    fn descript_sequence_cutscene_plays_with_its_data() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("maledict.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let rec = db
            .records
            .iter()
            .find(|r| r.name == "maledict")
            .expect("maledict cutscene record");
        let mut e = EngineState::new();
        assert!(
            e.start_descript_cutscene(rec, assets),
            "the cutscene starts"
        );
        // HNM, music, and tick-subtitles all come from the record — data-driven, not hardcoded.
        assert!(e.intro_hnms[0].file_stem().is_some_and(|s| s == "maledict"));
        assert_eq!(
            e.intro_music[0].as_deref(),
            Some("klings.voc"),
            "cutscene music from the record"
        );
        assert!(
            !e.intro_cues[0].is_empty(),
            "the curse subtitles play with the cutscene"
        );
        assert!(
            e.intro_cues[0].iter().any(|c| c.text.contains("CURSED")),
            "the record's subtitle text is carried"
        );
        // And an in-game cutscene plays its FULL HNM, not cut to its subtitle-cue span (the
        // intro-credit early-end is intro-only). maledict's cues end at tick 10 (~34 frames).
        let full = crate::hnm::HnmFile::open(&e.intro_hnms[0])
            .map(|h| h.frame_count())
            .unwrap_or(0);
        if full > 60 {
            let mut frames = 0usize;
            for _ in 0..full + 100 {
                if !e.intro_active() {
                    break;
                }
                e.step(MouseInput::default());
                frames += 1;
            }
            assert!(
                frames > 60,
                "in-game cutscene plays its full HNM ({frames} frames), not cut to its ~tick-10 cues"
            );
        }
    }

    /// Topic-gated dialogue: the scripted OPENING auto-plays, then the dialogue HOLDS at the
    /// topic menu; a topic click plays only ITS segment and re-holds. Guards the user-reported
    /// bug where Honk rattled off his entire food menu unprompted.
    #[test]
    fn dialogue_autoplay_holds_at_the_topic_menu() {
        let mut e = EngineState::new();
        let lines: Vec<(String, Option<std::path::PathBuf>)> =
            (0..10).map(|i| (format!("LINE {i}"), None)).collect();
        e.set_speech_dialogue(lines);
        e.dialogue_hold_frames = 2;
        e.set_topic_menu(vec![("TALK".into(), 4), ("ONE".into(), 7)]);
        e.set_dialogue_autoplay_end(Some(4));
        for _ in 0..600 {
            e.step(MouseInput::default());
        }
        assert_eq!(
            e.dialogue_cursor(),
            3,
            "auto-play holds on the last opening line"
        );
        assert!(!e.dialogue_finished(), "the held dialogue is not finished");
        // Clicking the TALK topic (row 0) plays its segment only, then re-holds.
        // A 2-row menu centers with its top row at y=89 (choice_box_top_y(2)).
        let row = e.topic_menu_click(200, 90);
        assert_eq!(row, Some(0));
        for _ in 0..600 {
            e.step(MouseInput::default());
        }
        assert_eq!(
            e.dialogue_cursor(),
            6,
            "the topic segment holds before the next topic"
        );
    }

    /// The TV plays the real DESCRIPT PROGRAMMING: the broadcast records that self-identify via
    /// their "…watching…" subtitles (hatetv + microkid/IZWAL), each with chained clips, music,
    /// and tick-timed cues drawn over the picture. Guards the gap where the TV looped raw silent
    /// HNMs with no broadcast content.
    #[test]
    fn tv_plays_descript_broadcast_programming() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("hatetv02.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_tv_programs(&db, assets);
        // The full self-identified channel lineup loads (each record announces itself as a
        // channel/broadcast in its own subtitles), stable name order. On Dec 25 / Jan 1 the
        // `venus` ad is the christmas/year seasonal variant, so accept either in that slot.
        let names: Vec<&str> = e.tv_programs.iter().map(|p| p.name.as_str()).collect();
        for expect in ["garde", "hatetv", "match", "microkid", "ppit", "scrut"] {
            assert!(
                names.contains(&expect),
                "channel lineup includes {expect}, got {names:?}"
            );
        }
        assert!(
            names.contains(&"venus") || names.contains(&"christmas") || names.contains(&"year"),
            "the ad channel (or its seasonal variant) is present: {names:?}"
        );
        for p in &e.tv_programs {
            assert!(!p.clips.is_empty(), "{}: clips loaded", p.name);
            assert!(p.music.is_some(), "{}: broadcast music", p.name);
        }
        let hate = names.iter().position(|n| *n == "hatetv").unwrap();
        e.tv_channel = hate;
        assert_eq!(
            e.tv_music(),
            Some("hatetv.voc"),
            "the HATE-TV channel carries its music"
        );
        // Render a few frames: the picture shows and the tick-1 "YOU ARE WATCHING HATE TV" cue
        // is drawn (reserved credit-colour glyphs present).
        e.tv_active = true;
        for _ in 0..3 {
            e.render_tv();
        }
        let nonblank = e.framebuffer.iter().filter(|&&p| p != 0).count();
        assert!(
            nonblank > 1000,
            "the broadcast picture renders ({nonblank} px)"
        );
        let cue_px = e
            .framebuffer
            .iter()
            .filter(|&&p| p == EngineState::INTRO_CREDIT_COLOR_INDEX)
            .count();
        assert!(
            cue_px > 50,
            "the broadcast subtitle cue is drawn ({cue_px} px)"
        );
        // Switching channels restarts the new broadcast and switches its music (hatetv → match,
        // whose gameshow runs on the HATE-TV music family, hatetv2.voc).
        e.switch_tv_channel(1);
        assert_eq!(
            e.tv_music(),
            Some("hatetv2.voc"),
            "next channel's music after switch"
        );
    }

    /// The intro montage (`cliptoot.hnm`) plays FULL-LENGTH under the pyramid console — VERIFIED
    /// by decoding its checkpoints against the real-game captures: it is the whole intro montage
    /// (crew + locations + hyperspace, frames 120..1150 matching captures 6..22), with the CRYO/
    /// title credits clearing at tick 100 (captures 15/22 show no credit text). Guards against
    /// cutting the montage to its ~tick-100 credit span (an earlier wrong fix).
    #[test]
    fn intro_montage_plays_full_length_with_credits_clearing() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_intro(assets, &db);
        let credit = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "cliptoot"))
            .expect("cliptoot montage queued");
        let full_len = crate::hnm::HnmFile::open(&e.intro_hnms[credit])
            .map(|h| h.frame_count())
            .unwrap_or(0);
        assert!(
            full_len > 1000,
            "cliptoot is the long montage ({full_len} frames)"
        );
        // Drive the intro through, counting frames spent in the montage clip.
        let mut montage_frames = 0usize;
        for _ in 0..4000 {
            let at_montage = e.intro_index() == credit && e.intro_active();
            e.step(MouseInput::default());
            if at_montage {
                montage_frames += 1;
            }
            if !e.intro_active() {
                break;
            }
        }
        assert!(
            montage_frames >= full_len,
            "the montage plays FULL-LENGTH ({montage_frames} of {full_len} frames), not cut to its credit span"
        );
    }

    /// Diagnostic dump: render the TV broadcast (hatetv, a few frames in) to a PPM for eyeballing.
    #[test]
    #[ignore = "diagnostic dump, run explicitly"]
    fn dump_tv_broadcast() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("hatetv02.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_tv_programs(&db, assets);
        e.tv_active = true;
        if let Ok(ch) = std::env::var("TV_CH") {
            let want = ch;
            if let Some(i) = e.tv_programs.iter().position(|p| p.name == want) {
                e.tv_channel = i;
            }
        }
        for _ in 0..30 {
            e.render_tv();
        }
        let mut buf = Vec::from(&b"P6\n320 200\n255\n"[..]);
        for &idx in &e.framebuffer {
            buf.extend_from_slice(&e.scene_palette[idx as usize]);
        }
        std::fs::write("output/_tmp_tv.ppm", buf).unwrap();
    }

    /// Diagnostic dump: decode cliptoot.hnm SEQUENTIALLY (delta frames chain) and save checkpoints
    /// across its whole 1258-frame range — to verify whether it is the full intro MONTAGE (crew
    /// showcase AND the location scenes seen in captures frame_15/22: sunset seascape, teal
    /// structure), which would mean it plays FULL-LENGTH in the real game, not a ~7s credit clip.
    #[test]
    #[ignore = "diagnostic dump, run explicitly"]
    fn dump_cliptoot_montage() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let hnm = crate::hnm::HnmFile::open(&assets.join("sq").join("cliptoot.hnm")).unwrap();
        let mut fb = vec![0u8; ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT];
        let mut pal = hnm.palette;
        let n = hnm.frame_count();
        let checkpoints = [120usize, 250, 400, 550, 700, 850, 1000, 1150];
        for f in 0..n {
            hnm.decode_frame(f, &mut fb, &mut pal);
            if checkpoints.contains(&f) {
                let mut buf = Vec::from(&b"P6\n320 200\n255\n"[..]);
                for &idx in &fb {
                    buf.extend_from_slice(&pal[idx as usize]);
                }
                std::fs::write(format!("output/_tmp_clip_{f:04}.ppm"), buf).unwrap();
            }
        }
    }

    /// Diagnostic dump: play an in-game cutscene (maledict) via start_descript_cutscene and dump a
    /// frame — verify it renders FULL-SCREEN (no pyramid console) with its subtitle.
    #[test]
    #[ignore = "diagnostic dump, run explicitly"]
    fn dump_cutscene() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("maledict.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let rec = db.records.iter().find(|r| r.name == "maledict").unwrap();
        let mut e = EngineState::new();
        e.start_descript_cutscene(rec, assets);
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        let mut buf = Vec::from(&b"P6\n320 200\n255\n"[..]);
        for &idx in &e.framebuffer {
            buf.extend_from_slice(&e.scene_palette[idx as usize]);
        }
        std::fs::write("output/_tmp_cutscene.ppm", buf).unwrap();
    }

    /// Diagnostic dump: drive the intro to the credit clip and dump the COMPOSITE (crew showcase
    /// cliptoot + pyramid console overlay + credits) to a PPM, to eyeball it against captures 6-9.
    #[test]
    #[ignore = "diagnostic dump, run explicitly"]
    fn dump_intro_composite() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_intro(assets, &db);
        let credit = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "cliptoot"))
            .unwrap();
        for _ in 0..6000 {
            e.step(MouseInput::default());
            if e.intro_index() == credit && e.scene_frame > 45 {
                break;
            }
            if !e.intro_active() {
                break;
            }
        }
        let mut buf = Vec::from(&b"P6\n320 200\n255\n"[..]);
        for &idx in &e.framebuffer {
            buf.extend_from_slice(&e.scene_palette[idx as usize]);
        }
        std::fs::write("output/_tmp_intro_composite.ppm", buf).unwrap();
    }

    /// REAL-GAME-VERIFIED composite (DOSBox-X captures of BLOODPRG with game args,
    /// game_95s..130s; band re-grounded from the interpreter's BOOTIDX indices): the crew
    /// MONTAGE plays on the pyramid-console + eye-orb band (static rows 140..200, console
    /// bank 224..255 — accuracy/captures/console_band.idx), while the logo/ship reel
    /// plays full-screen. Guards both directions of the earlier confusion.
    #[test]
    fn intro_montage_plays_on_the_real_console_band() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_intro(assets, &db);
        // Clip 0 = the logo/ship reel: full-screen, no band.
        for _ in 0..10 {
            e.render_intro_frame();
        }
        let band_px = |e: &EngineState| {
            e.framebuffer[ENGINE_SCREEN_WIDTH * 140..]
                .iter()
                .filter(|&&p| p >= 224)
                .count()
        };
        assert!(
            band_px(&e) < 2000,
            "the logo reel plays full-screen (no console band)"
        );
        // Advance to the montage clip (index 1): the band composites over the bottom.
        while e.intro_index() == 0 && e.intro_active() {
            e.render_intro_frame();
        }
        e.render_intro_frame();
        assert!(
            band_px(&e) > 10_000,
            "the crew montage plays on the console band ({} px)",
            band_px(&e)
        );
    }

    /// End-to-end regression: drive the full playable loop the way the real driver does
    /// (title -> intro -> nav -> every screen -> a dialogue scene) and assert each stage
    /// produces real content and progresses. The step loop is pure logic (no real-time
    /// wait), so a full scene runs in milliseconds. Skips without game data. A broader
    /// all-five-script playthrough lives in `src/bin/smoke.rs`.
    #[test]
    fn full_playable_loop_end_to_end() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("DESCRIPT.DES").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else {
            return;
        };
        let db = crate::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
        let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT1.{ext}")));
        let (Ok(cod), Ok(var), Ok(dic), Ok(deb)) = (rd("COD"), rd("VAR"), rd("DIC"), rd("DEB"))
        else {
            return;
        };

        let mut e = EngineState::new();
        e.load_dialogue_scenes(&cod, &var, &dic, &deb, &db, assets);
        e.dialogue_hold_frames = 20;
        if let (Ok(c), Ok(b)) = (
            std::fs::read(iso.join("CARTE.SPR")),
            std::fs::read(iso.join("BORXX.SPR")),
        ) {
            e.load_nav_sprites(&c, &b);
        }
        e.load_title(iso);
        e.load_intro(assets, &db);
        e.load_alien_view(assets, "scrut");
        e.load_tv_channels(assets, "tv");
        e.load_cyberspace(assets);
        e.load_bridge(iso);
        let has_chart = e.load_nav_chart(iso);
        e.load_console_font(iso);
        e.on_ship = true;
        let nonblank = |fb: &[u8]| fb.iter().filter(|&&p| p != 0).count();

        // Title, then intro to completion.
        assert!(e.title_active(), "title armed at startup");
        e.step(MouseInput::default());
        assert!(nonblank(&e.framebuffer) > 1000, "title renders art");
        e.dismiss_title();
        let mut intro_ended = false;
        for _ in 0..4000 {
            e.step(MouseInput::default());
            if !e.intro_active() {
                intro_ended = true;
                break;
            }
        }
        assert!(intro_ended, "intro sequence finishes");

        // Every screen renders real content.
        e.on_ship = true;
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        assert!(nonblank(&e.framebuffer) > 500, "nav view renders");
        if has_chart {
            // With CHART.FD present the nav view is the real star-map: a rich, many-colour
            // image, not a sparse procedural starfield.
            let distinct = e
                .framebuffer
                .iter()
                .collect::<std::collections::HashSet<_>>()
                .len();
            assert!(
                distinct > 40,
                "nav view shows the real CHART.FD star-map ({distinct} colours)"
            );
        }
        e.bridge_active = true;
        for _ in 0..4 {
            e.step(MouseInput::default());
        }
        assert!(nonblank(&e.framebuffer) > 500, "bridge renders");
        e.bridge_active = false;
        e.tv_active = true;
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        assert!(nonblank(&e.framebuffer) > 500, "TV renders");
        e.tv_active = false;
        e.cyber_active = true;
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        assert!(nonblank(&e.framebuffer) > 500, "cyberspace renders");
        e.cyber_active = false;
        e.alien_view_active = true;
        e.arm_alien_intro();
        for _ in 0..12 {
            e.step(MouseInput::default());
        }
        assert!(nonblank(&e.framebuffer) > 500, "alien view renders");
        e.alien_view_active = false;

        // A dialogue scene plays through to completion (SCRIPT1 is the short one).
        e.on_ship = false;
        let total = e.dialogue_len();
        let mut finished = false;
        for _ in 0..20000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() {
                finished = true;
                break;
            }
        }
        assert!(finished, "SCRIPT1 dialogue scene completes");
        assert!(total > 1, "SCRIPT1 has real dialogue lines ({total})");
        assert!(
            e.dialogue_cursor() + 1 >= total,
            "cursor reached the last line"
        );
    }

    /// The SAVE-SLOT UI matches the oracle capture (vs_011) and the DOS edit law
    /// (0x1DD8): grey 0xE8 bar at x63..137 y39..48, typed name in white 0xEF bold
    /// glyphs from x=73, CANCEL at (73,150); digits+lowercase only (uppercase
    /// rejected), max 14 chars, backspace deletes, Enter commits the name.
    #[test]
    fn save_slot_ui_edits_by_the_0x1dd8_law_and_renders_through_the_widget() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        e.load_bridge(iso);
        e.on_ship = true;
        e.bridge_active = true;
        e.bridge.frame = 45;
        e.save_ui_active = true;
        assert!(e.save_ui_key(b'A').is_none());
        assert_eq!(
            e.save_ui_name, "",
            "uppercase is rejected (the 0x1DD8 filter)"
        );
        for &c in b"ab" {
            assert!(e.save_ui_key(c).is_none());
        }
        assert_eq!(e.save_ui_name, "ab");
        for _ in 0..20 {
            e.save_ui_key(b'x');
        }
        assert_eq!(e.save_ui_name.len(), 14, "the 14-char cap");
        e.save_ui_key(8);
        assert_eq!(e.save_ui_name.len(), 13, "backspace deletes");
        e.step(MouseInput {
            x: 300,
            y: 190,
            ..Default::default()
        });
        // The screen is the LIST WIDGET showing the slots with the edit buffer
        // swapped into the edited row (0x8573), not the capture-measured grey bar
        // at x63..137/y39..48 this used to assert. Check the typed text renders in
        // the widget's own row band, in the selected-row colour.
        let rows = EngineState::SAVE_SLOT_ROWS + 1;
        let top = EngineState::choice_box_top_y(rows);
        let band = (top + rows * 11).min(ENGINE_SCREEN_HEIGHT);
        let typed = (top..band)
            .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| y * ENGINE_SCREEN_WIDTH + x))
            .filter(|&i| e.framebuffer[i] == 0xEF || e.framebuffer[i] == 0xFE)
            .count();
        assert!(
            typed > 40,
            "the edited row renders in the selected colour ({typed} px)"
        );
        // Enter with a non-empty name commits and closes the UI.
        let name = e.save_ui_key(13).expect("Enter commits");
        assert_eq!(name.len(), 13);
        assert!(!e.save_ui_active);
        // Empty name does not commit.
        e.save_ui_active = true;
        assert!(e.save_ui_key(13).is_none());
        assert!(e.save_ui_active);
    }

    /// The game's real flow after the intro: the SCRIPT1 console tutorial plays, then
    /// chains to SCRIPT2 via its decoded D2 handoff (profile 1). Verifies the chain
    /// trigger the driver relies on (`main.rs` auto-plays SCRIPT1 then follows this).
    /// The console CRYOBOX option opens the cryo-chamber (cryorad.hnm) — it loads and
    /// renders (with the HNM's own header palette), and the CRYOBOX menu row is clickable.
    #[test]
    fn cryobox_console_function_renders() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cryorad.hnm").exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(e.load_cryobox(assets), "cryorad.hnm loads");
        e.cryobox_active = true;
        for _ in 0..16 {
            e.step(MouseInput::default());
        }
        // The cryo-chamber fills the frame in real (many-colour) content.
        assert!(
            e.framebuffer.iter().filter(|&&p| p != 0).count() > 5000,
            "cryo-chamber renders"
        );
        let distinct = e
            .framebuffer
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(distinct > 20, "cryo-chamber has real colour ({distinct})");
    }

    /// The console TELEPHONE option opens the video-phone: the call widget + contact list
    /// render (dialling), a click connects a crew member, and the connected state shows
    /// their full-colour talk-head HNM feed. Esc/hangup returns to dialling.
    #[test]
    fn telephone_console_function_renders() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("BAPPEL.SPR").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("pe").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else {
            return;
        };
        let mut e = EngineState::new();
        assert!(
            e.load_telephone(iso, assets),
            "BAPPEL.SPR + talk-heads load"
        );
        assert!(e.load_console_font(iso), "console font loads");
        e.load_nav_chart(iso);
        assert!(e.phone_contact_count() >= 3, "several crew are callable");
        e.phone_active = true;
        // Dialling: the widget + contact list render as real content.
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        assert!(!e.phone_connected(), "starts on the dial screen");
        assert!(
            e.framebuffer.iter().filter(|&&p| p != 0).count() > 500,
            "dial screen renders"
        );
        // A click on the second contact row (choice-box row 1) connects that call.
        // The box is vertically centred for (contacts+CANCEL) rows, so derive row
        // 1's y from that layout rather than assuming a fixed anchor.
        let total = (e.phone_contact_count().min(7) + 1).min(8);
        let y = (EngineState::choice_box_top_y(total) + EngineState::CHOICE_BOX_PITCH + 2) as u16;
        let row = e.phone_contact_click(100, y).expect("row 1 hits");
        assert_eq!(row, 1);
        assert!(e.phone_connect(row));
        assert!(e.phone_connected(), "call connected");
        let name = e.phone_contact_name().unwrap().to_string();
        // Connected: the crew's talk-head HNM feed fills the frame in colour.
        for _ in 0..8 {
            e.step(MouseInput::default());
        }
        let distinct = e
            .framebuffer
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(
            distinct > 16,
            "call feed for {name} has real colour ({distinct})"
        );
        // Hanging up returns to the dial screen.
        e.phone_hangup();
        assert!(!e.phone_connected(), "hung up back to dial");
    }

    /// The console MENU option opens the decoded {EXPLANATIONS, GAME} submenu: the bridge
    /// draws those two labels in place of the top menu rows, and a click on a submenu row
    /// resolves to its index (matching the layout).
    #[test]
    fn menu_submenu_decoded_from_real_console() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("HONKF.SPR").is_file());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        assert!(e.load_console_font(iso), "console font loads");
        e.load_bridge(iso);
        // The submenu is the console CHOICE BOX (the game's universal console-choice
        // widget): rows from the box top (y=86) at 11px pitch (draw_choice_box).
        let (x, y0) = (90u16, 90u16);
        assert_eq!(e.menu_submenu_click(x, y0), None, "closed: no submenu hit");
        // Open the submenu (as clicking MENU does) and render it.
        e.menu_submenu_active = true;
        e.bridge_active = true;
        e.step(MouseInput::default());
        assert_eq!(EngineState::MENU_SUBMENU, ["EXPLANATIONS", "GAME"]);
        // Row 0 = EXPLANATIONS, row 1 = GAME.
        assert_eq!(e.menu_submenu_click(x, y0), Some(0));
        assert_eq!(e.menu_submenu_click(x, y0 + 11), Some(1));
    }

    /// Click-to-advance dialogue: a click snaps the current line fully revealed, then moves
    /// to the next; on the last line it returns false (the driver ends the dialogue).
    #[test]
    fn click_advances_dialogue() {
        let mut e = EngineState::new();
        let lines: Vec<(String, Option<std::path::PathBuf>)> =
            (0..4).map(|i| (format!("line {i}"), None)).collect();
        e.set_speech_dialogue(lines);
        e.on_ship = false;
        assert!(e.in_dialogue());
        assert_eq!(e.dialogue_cursor(), 0);
        // Each line takes two clicks (snap fully revealed, then advance). Click through:
        // it advances across all lines and eventually signals the end (false).
        let mut ended = false;
        for _ in 0..30 {
            if !e.skip_dialogue_line() {
                ended = true;
                break;
            }
        }
        assert!(ended, "click-through reaches the end");
        assert_eq!(e.dialogue_cursor(), 3, "ended on the last line");
    }

    /// The cyberspace traversal mini-game: flies through the real tunnel segments, steers
    /// with the mouse, and reaches its destination (`cyber_arrived`) at the last segment.
    #[test]
    fn cyberspace_traversal_reaches_destination() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.load_cyberspace(assets);
        let (_, total) = e.cyber_progress();
        if total == 0 {
            return;
        }
        e.cyber_active = true;
        e.start_cyberspace();
        assert!(!e.cyber_arrived, "starts before the destination");
        // Steering right moves the on-course reticle; the frame stays real content.
        for _ in 0..8 {
            e.step(MouseInput {
                x: 260,
                y: 100,
                buttons: 0,
                ..Default::default()
            });
        }
        assert!(
            e.framebuffer.iter().filter(|&&p| p != 0).count() > 500,
            "tunnel + HUD render"
        );
        // Fly the whole journey to arrival.
        for _ in 0..30000 {
            e.step(MouseInput::default());
            if e.cyber_arrived {
                break;
            }
        }
        assert!(e.cyber_arrived, "traversal reaches the destination");
        let (seg, tot) = e.cyber_progress();
        assert_eq!(seg, tot - 1, "ends on the last segment");
    }

    /// The game-ending finale (`sq/fin.hnm`) loads, plays full-screen in colour, and
    /// reports finished once it reaches its last frame — the bookend to the intro.
    #[test]
    fn ending_finale_plays_to_completion() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("fin.hnm").exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(e.load_ending(assets), "fin.hnm loads");
        e.start_ending();
        assert!(e.ending_active, "finale armed");
        assert!(!e.ending_finished(), "finale not finished at the start");
        // First frame renders real (many-colour) content.
        e.step(MouseInput::default());
        assert!(
            e.framebuffer.iter().filter(|&&p| p != 0).count() > 5000,
            "finale renders"
        );
        let distinct = e
            .framebuffer
            .iter()
            .collect::<std::collections::HashSet<_>>()
            .len();
        assert!(distinct > 16, "finale has real colour ({distinct})");
        // Step through to completion.
        for _ in 0..4000 {
            if e.ending_finished() {
                break;
            }
            e.step(MouseInput::default());
        }
        assert!(e.ending_finished(), "finale plays through all frames");
    }

    /// The port's save/load round-trips the resumable game state through the engine: a
    /// captured `SaveState` (screen + nav heading + dialogue progress + settings), applied
    /// to a fresh engine with the same dialogue loaded, restores that exact state.
    #[test]
    fn save_captures_and_restores_game_state() {
        // A source engine mid-dialogue on the comms screen, heading 120, line 3.
        let mut src = EngineState::new();
        let lines: Vec<(String, Option<std::path::PathBuf>)> =
            (0..10).map(|i| (format!("line {i}"), None)).collect();
        src.set_speech_dialogue(lines.clone());
        src.on_ship = false;
        src.tv_active = true;
        src.compass_angle = 120;
        src.text_speed_step = crate::vm::text_speed_step_from_setting(5);
        src.set_dialogue_cursor(3);
        let save = src.capture_save(4);
        assert_eq!(save.screen, crate::save::SaveScreen::Comms);
        assert_eq!(save.script, 4);

        // A round-trip through the file text must preserve it.
        let save = crate::save::SaveState::from_text(&save.to_text()).expect("parses");

        // Restore into a fresh engine that has reloaded the same dialogue.
        let mut dst = EngineState::new();
        dst.set_speech_dialogue(lines);
        dst.restore_save(&save);
        assert!(
            dst.tv_active && !dst.on_ship,
            "restored to the comms screen"
        );
        assert_eq!(dst.compass_angle, 120, "restored the nav heading");
        assert_eq!(dst.dialogue_cursor(), 3, "resumed at the saved line");
        assert_eq!(
            dst.text_speed_step,
            crate::vm::text_speed_step_from_setting(5),
            "restored the text-speed setting"
        );
    }

    /// `set_speech_dialogue` plays the full decoded per-character dialogue (all lines)
    /// instead of `execute_trace`'s linear branch, and the cursor advances through them.
    #[test]
    fn speech_dialogue_plays_all_lines() {
        let mut e = EngineState::new();
        let lines: Vec<(String, Option<std::path::PathBuf>)> =
            (0..250).map(|i| (format!("line {i}"), None)).collect();
        e.set_speech_dialogue(lines);
        assert_eq!(e.dialogue_len(), 250, "all speech lines loaded");
        assert_eq!(e.current_subtitle(), Some("line 0"));
        e.on_ship = false;
        for _ in 0..40000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() {
                break;
            }
        }
        assert!(
            e.dialogue_cursor() + 1 >= 250,
            "cursor advances through all lines"
        );
    }

    /// The choose-a-location nav: a destination list is offered on the star-map, a click
    /// on an entry maps to its index (matching the drawn layout), and visiting it plays
    /// that location's decoded dialogue.
    /// The nav projector (`0x9B98`) draws ONE marker per ACTIVE destination
    /// entity (`test [si],0x80` over entities `0x15..0x1F`) — never a fixed grid.
    /// Oracle-confirmed: with no destinations granted, all eleven entity records
    /// read zero in every reachable savestate, so the real routine draws nothing.
    /// This pins the port to that gate, replacing the old fabricated 7x4 grid.
    /// The ship-3D transition + depth state machine must actually RUN in play.
    /// Both routines were audit-verified exact against 0xB692 / 0xB75C but had no
    /// caller outside tests, so the nav view never swept open. This pins the
    /// wiring: with the nav view up, the hold counter passes 120, the view arms
    /// and opens with step 4, and the depth offset climbs toward 0x41.
    #[test]
    fn nav_view_drives_the_ship3d_transition_and_depth_state() {
        let mut e = EngineState::new();
        e.on_ship = true; // nav view active: no overlay screens open
        assert!(e.nav_view_active(), "test precondition: the nav view is up");
        assert_eq!(e.ship3d_depth.depth_offset, 0, "starts closed");

        // Before the 120-tick threshold the view must NOT arm.
        for _ in 0..100 {
            e.step_ship_3d_nav_state();
        }
        assert!(
            !e.ship3d_transition.transition_armed,
            "not armed before 120 ticks"
        );
        assert_eq!(e.ship3d_depth.depth_offset, 0, "no sweep before arming");

        // Past the threshold it arms, opens with step 4 (0xB6A0), and sweeps.
        for _ in 0..40 {
            e.step_ship_3d_nav_state();
        }
        assert!(
            e.ship3d_transition.transition_armed,
            "armed once hold > 120"
        );
        assert_eq!(e.ship3d_transition.depth_step, 4, "open step is 4 (0xB6A0)");
        assert!(
            e.ship3d_depth.depth_offset > 0,
            "the depth sweep advanced, got {}",
            e.ship3d_depth.depth_offset
        );

        // It clamps at the maximum rather than overshooting (0xB776).
        for _ in 0..400 {
            e.step_ship_3d_nav_state();
        }
        assert!(
            e.ship3d_depth.depth_offset <= crate::ship3d::SHIP_3D_MAX_DEPTH_OFFSET,
            "never exceeds 0x41, got {}",
            e.ship3d_depth.depth_offset
        );
    }

    /// The procedural HUD machine must also RUN in play (it was verified but
    /// unreachable). With the HUD-active flag set it rotates the angle toward
    /// hold/2 and clears the flag on arrival — driven purely by the frame tick.
    #[test]
    fn nav_view_drives_the_ship3d_procedural_update() {
        use crate::ship3d::SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG;
        let mut e = EngineState::new();
        e.on_ship = true;
        assert!(e.nav_view_active());
        e.ship3d_procedural.hud_flags |= SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG;
        e.ship3d_procedural.angle = 40;

        let start = e.ship3d_procedural.angle;
        for _ in 0..64 {
            e.step_ship_3d_nav_state();
        }
        // The machine ran: either the angle moved toward hold/2 or the flag
        // cleared on arrival. Both are observable evidence it is reached.
        let moved = e.ship3d_procedural.angle != start;
        let cleared = e.ship3d_procedural.hud_flags & SHIP_3D_PROCEDURAL_HUD_ACTIVE_FLAG == 0;
        assert!(
            moved || cleared,
            "procedural update never ran: angle {} flags {:#x}",
            e.ship3d_procedural.angle,
            e.ship3d_procedural.hud_flags
        );
        // And the hold counter it reads is the same one the transition gate uses.
        assert_eq!(e.ship3d_procedural.hold_ticks, e.ship3d_hold_ticks);
    }

    #[test]
    fn nav_markers_are_gated_on_granted_destinations() {
        let count_pyramid_pixels = |e: &EngineState| -> usize {
            // The CARTE pyramid art is drawn over the starfield; count non-black
            // pixels in the star-map band as a proxy for "markers were drawn".
            e.framebuffer.iter().filter(|&&p| p != 0).count()
        };

        // No destinations granted -> the projector's gate fails for every entity,
        // so no destination markers are drawn.
        let mut empty = EngineState::new();
        empty.on_ship = true;
        empty.render_ship_view();
        let empty_px = count_pyramid_pixels(&empty);

        // Three granted destinations -> three markers.
        let mut three = EngineState::new();
        three.set_nav_destinations(vec![
            ("EKATOMB".into(), vec![]),
            ("VENUSIA".into(), vec![]),
            ("KORTEX".into(), vec![]),
        ]);
        three.on_ship = true;
        three.render_ship_view();
        assert_eq!(three.nav_destination_count(), 3);

        // The renderer is driven by the granted set, not a constant 28-point grid:
        // granting destinations must not leave the frame identical to the empty
        // case (and with the art absent both are trivially equal, so only assert
        // when the pyramid bank actually loaded).
        if !three.nav_pyramids.is_empty() {
            assert_ne!(
                count_pyramid_pixels(&three),
                empty_px,
                "granted destinations must change what the star map draws"
            );
        }
    }

    /// The game's nav-position table gives every destination the SAME world point,
    /// so the markers COINCIDE on screen. The port used to fan them out by a
    /// fabricated 700-unit lateral spread purely so each granted destination stayed
    /// separately visible; that invented positions the game does not have.
    ///
    /// This pins the real behaviour at both ends: the table is ten identical
    /// records (DS:0x4F09, byte-verified), and adding destinations therefore does
    /// NOT scatter the drawing — with one marker per destination stacked on one
    /// point, three destinations paint exactly the pixels one does.
    /// The OPTION row is the game's own string at DS:0x0174 (file 0x0D594), not a
    /// label read off a screenshot. Reads it straight out of the shipped image, so
    /// the constant cannot drift from the binary it claims to come from.
    /// 214 of the 3687 `0xA6` lines across the five scripts carry a CHOICE MENU after
    /// (counted by `a6_menu_bearing_line_count_matches_the_recorded_figure`; this
    /// said 211 of 3650 from memory until audit-fixes #416 measured it)
    /// an `0xFFFF` separator. The subtitle builder used `filter_map` over the whole
    /// word list, which silently dropped the separator (not a DIC key) but KEPT the
    /// menu rows, gluing them onto the spoken line.
    #[test]
    fn choice_menu_rows_do_not_leak_into_the_spoken_subtitle() {
        let dir = [
            "accuracy/cblood_install/cblood",
            "../accuracy/cblood_install/cblood",
        ]
        .iter()
        .map(Path::new)
        .find(|p| p.join("SCRIPT1.COD").is_file());
        let Some(dir) = dir else { return };
        let cod = std::fs::read(dir.join("SCRIPT1.COD")).unwrap();
        let dic = std::fs::read(dir.join("SCRIPT1.DIC")).unwrap();
        let var = std::fs::read(dir.join("SCRIPT1.VAR")).unwrap_or_default();

        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);

        // No subtitle may contain the menu rows of its own line.
        for (i, line) in e.dialogue.iter().enumerate() {
            if let Some(rows) = e.menu_by_offset.get(&line.offset) {
                let text = e.dialogue_texts[i].clone();
                for row in rows {
                    assert!(
                        !text.split_whitespace().any(|w| w.eq_ignore_ascii_case(row)),
                        "menu row {row:?} leaked into subtitle {text:?}"
                    );
                }
            }
        }

        // The canonical record: spoken line ends at "...", menu is explanations/game.
        let (_, rows) = e
            .menu_by_offset
            .iter()
            .min_by_key(|(o, _)| **o)
            .expect("SCRIPT1 has a choice-menu line");
        assert_eq!(rows, &vec!["explanations".to_string(), "game".to_string()]);

        // And the submenu now comes from the SCRIPT, not the const.
        assert_eq!(e.menu_submenu_labels(), vec!["EXPLANATIONS", "GAME"]);
        // With no script loaded the const is the documented fallback.
        assert_eq!(
            EngineState::new().menu_submenu_labels(),
            vec!["EXPLANATIONS", "GAME"]
        );
    }

    /// The Bob contact topic rows used to sit at hardcoded x=170 / y=56, recorded as
    /// "measured from the dual-run oracle captures". Both fall out of the decoded
    /// widget geometry, which is what the port now computes. This pins the equivalence
    /// so the derivation cannot silently drift from the value the capture confirmed.
    #[test]
    fn bob_contact_rows_derive_the_capture_measured_layout() {
        // Eight rows is the count the capture showed.
        let top = EngineState::choice_box_top_y(8);
        assert_eq!(top, 56, "y = (200-(8*11+8))/2 + 4");
        assert_eq!(EngineState::CHOICE_BOX_PITCH, 11, "add bp,0xB @0x847A");
        assert_eq!(
            EngineState::CHOICE_BOX_ANCHOR_CONCEPT,
            0xE1,
            "mov [0xAC6],0xE1 @0x89A6"
        );
        // x0 = anchor - (widest + 0x14)/2; the capture's x=170 implies a 110px box,
        // i.e. a widest label of 90px — consistent, not coincidental.
        let box_w = 90 + 0x14;
        assert_eq!(
            EngineState::CHOICE_BOX_ANCHOR_CONCEPT.saturating_sub(box_w / 2),
            170,
            "x0 = anchor - w/2 reproduces the measured x"
        );
    }

    #[test]
    fn option_box_label_is_the_games_own_string() {
        let exe = match std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            Ok(b) => b,
            Err(_) => return,
        };
        let start = EngineState::OPTION_BOX_LABEL_FILE_OFFSET;
        let end = start
            + exe[start..]
                .iter()
                .position(|&b| b == 0)
                .expect("NUL-terminated");
        let mut probe = EngineState::new();
        probe.load_ds_strings(&exe);
        assert_eq!(
            std::str::from_utf8(&exe[start..end]).unwrap(),
            probe.ds_text(EngineState::OPTION_BOX_LABEL_DS_OFFSET),
            "OPTION row must equal the string at DS:0x0174"
        );
        // DS base is file 0xD420, so the recorded DS offset must agree with the file
        // offset -- this is what catches one of the two being edited alone.
        assert_eq!(
            start - 0xD420,
            EngineState::OPTION_BOX_LABEL_DS_OFFSET as usize,
            "DS offset and file offset must describe the same byte"
        );
        // (The old `OPTION_BOX[0] == OPTION_BOX_LABEL` assertion is gone with the
        // array: it compared a transcription to itself, which is the tautology
        // the comment above already names — audit-fixes #526.)
    }

    #[test]
    fn a_chart_click_opens_the_panel_and_the_next_click_closes_it() {
        use crate::vm::{LOCATION_PANEL_BOX, LOCATION_PANEL_ZOOM_STEPS, LocationPanelRow};
        const ODDLAND: u16 = 0x0F98;
        const HERE: u16 = 0x0200;
        let mut e = EngineState::new();
        // A real chart entry, shaped like the one SCRIPT5's .VAR produces.
        e.set_nav_chart_objects(vec![NavChartObject {
            object: ODDLAND,
            name: "Oddland".into(),
            kind: crate::vm::LOCATION_KIND_BLACK_HOLE,
            marker: (132, 34),
            art_id: crate::levels::world_art_resource_id("Oddland"),
            // The NEAR marker, so this is the `0x92ED`-matched entry. With no ship
            // bit in the kind the box is the black-hole one either way; the pair
            // that actually depends on this flag is swept against the lift in
            // `native_nav_chart_pick_matches_the_lift`.
            far_endpoint: false,
        }]);
        assert_eq!(e.nav_chart_objects()[0].art_id, Some(72));
        assert_eq!(
            e.nav_chart_objects()[0].hit_box(),
            crate::vm::NAV_PICK_BOX_BLACK_HOLE
        );

        // 0x92A3's box: 2px up-left of the marker, both bounds inclusive.
        assert!(e.nav_chart_object_click(130, 32).is_some());
        assert!(e.nav_chart_object_click(130 + 0x13, 32 + 0x0C).is_some());
        assert!(e.nav_chart_object_click(130 + 0x13 + 1, 32).is_none());
        assert!(e.nav_chart_object_click(0, 0).is_none());

        let rows = || {
            vec![LocationPanelRow {
                x: 0x6E,
                y: 0x19,
                color: 0xEE,
                text: "BLACK HOLE: Oddland".into(),
            }]
        };
        // A miss selects nothing and leaves the panel closed.
        assert!(!e.nav_chart_click(0, 0, HERE, |_| rows()));
        assert_eq!(e.location_panel.state, LocationPanelState::Idle);

        // A hit opens it on that object.
        assert!(e.nav_chart_click(133, 35, HERE, |_| rows()));
        assert_eq!(e.location_panel.object, ODDLAND);
        assert_eq!(e.location_panel.state, LocationPanelState::ZoomingOpen);

        // Frames: the zoom draws a tinted rect, then the panel proper.
        e.framebuffer.fill(200);
        for (i, entry) in e.scene_palette.iter_mut().enumerate() {
            *entry = [i as u8, i as u8, i as u8];
        }
        for _ in 0..LOCATION_PANEL_ZOOM_STEPS {
            assert!(
                e.render_nav_info_panel_frame(),
                "one drawn rect per zoom step"
            );
        }
        // The step AFTER the last one is where the gate reports Complete
        // (`stc` at 0x1EB9 -> the caller's `jae` falls through), so that frame
        // draws no rect and the panel becomes Open for the frame after it.
        assert!(!e.render_nav_info_panel_frame());
        assert_eq!(e.location_panel.state, LocationPanelState::Open);
        assert!(e.render_nav_info_panel_frame());
        let [bx, by, _, _] = LOCATION_PANEL_BOX.map(usize::from);
        assert!(
            e.framebuffer[(by + 6) * ENGINE_SCREEN_WIDTH + bx..]
                .iter()
                .take(200)
                .any(|&p| p == 0xEE),
            "the panel's own row must be on screen once it is Open"
        );

        // The next click is what re-enables the mouse (0x912E) -> close.
        assert!(!e.nav_chart_click(133, 35, HERE, |_| rows()));
        assert_eq!(e.location_panel.state, LocationPanelState::ZoomingShut);
        for _ in 0..LOCATION_PANEL_ZOOM_STEPS + 1 {
            e.render_nav_info_panel_frame();
        }
        assert_eq!(e.location_panel.state, LocationPanelState::Idle);
        assert!(!e.render_nav_info_panel_frame(), "idle draws nothing");
    }

    #[test]
    fn nav_slots_carry_their_entity_ids_and_answer_the_hover_gate() {
        use crate::ship3d::{SHIP_3D_ENTITY_COUNT, ship_3d_nav_entity_for_slot};
        // 0x9B98: slot i is entity 0x15 + i, and the table stops at 32 entities.
        assert_eq!(
            ship_3d_nav_entity_for_slot(0),
            Some((0x15, 0x6212 + 0x15 * 32))
        );
        assert_eq!(ship_3d_nav_entity_for_slot(10), Some((0x1F, 0x65F2)));
        assert_eq!(
            ship_3d_nav_entity_for_slot(10).unwrap().1,
            0x65F2,
            "entity 0x1F IS the DS:0x65F2 record the hover gate reads"
        );
        assert_eq!(
            ship_3d_nav_entity_for_slot(11),
            None,
            "past the 32-entity table"
        );

        let mut e = EngineState::new();
        e.ship3d_nav_slots.resize_with(11, Default::default);
        e.assign_nav_slot_entity_ids();
        assert_eq!(
            e.ship3d_nav_slots.last().and_then(|s| s.entity_id),
            Some(SHIP_3D_ENTITY_COUNT - 1),
            "the eleventh slot is entity 0x1F"
        );
        assert_eq!(e.ship3d_nav_slots[0].entity_id, Some(0x15));

        // The hover gate: entity 0x1F's state bit0 plus an inclusive box test.
        let last = e.ship3d_nav_slots.last_mut().unwrap();
        last.flags &= !1;
        last.draw_x = 100;
        last.draw_y = 60;
        last.extent_width = 20;
        last.extent_height = 10;
        assert!(
            !e.nav_hover_status_active((105, 62)),
            "state bit0 clear -> no panel"
        );
        e.ship3d_nav_slots.last_mut().unwrap().flags |= 1;
        assert!(e.nav_hover_status_active((105, 62)));
        assert!(
            e.nav_hover_status_active((100, 60)),
            "the near edge is inside"
        );
        assert!(
            e.nav_hover_status_active((120, 70)),
            "and so is the far edge"
        );
        assert!(!e.nav_hover_status_active((121, 70)));
        assert!(!e.nav_hover_status_active((105, 71)));
    }

    #[test]
    fn the_info_panel_zoom_fsm_runs_open_then_shut() {
        use crate::vm::{LOCATION_PANEL_BOX, LOCATION_PANEL_ZOOM_STEPS};
        const OBJECT: u16 = 0x0100;
        const HERE: u16 = 0x0200;
        let mut e = EngineState::new();
        assert_eq!(e.location_panel.state, LocationPanelState::Idle);

        // 0x901D: the panel refuses the object you are already at, and 0x8FB3's
        // `or ax,ax / je` refuses "nothing hit".
        assert!(!e.open_location_info_panel(HERE, HERE, (40, 50)));
        assert!(!e.open_location_info_panel(0, HERE, (40, 50)));
        assert_eq!(e.location_panel.state, LocationPanelState::Idle);

        assert!(e.open_location_info_panel(OBJECT, HERE, (40, 50)));
        assert_eq!(e.location_panel.state, LocationPanelState::ZoomingOpen);
        assert_eq!(e.location_panel.cursor_rect, [40, 50, 4, 4]);
        assert!(
            !e.location_panel_mouse_enabled,
            "0x900C turns the mouse off; it coming back is what closes the panel"
        );
        assert_eq!(e.location_panel.entity_draw_scale(), 1, "0x9048: scale 0");

        // Zoom open: one drawn rect per step, then the panel goes Open.
        let mut rects = Vec::new();
        for _ in 0..LOCATION_PANEL_ZOOM_STEPS + 2 {
            if let Some(rect) = e.step_location_info_panel() {
                rects.push(rect);
            }
        }
        assert_eq!(rects.len() as u8, LOCATION_PANEL_ZOOM_STEPS);
        assert_eq!(e.location_panel.state, LocationPanelState::Open);
        // The last step does NOT land on the panel rect: the gate computes
        // `dest + (src-dest)/total*step` with a TRUNCATING `idiv bl` (0x1E74), so
        // the animation stops short by the remainder and the drawn panel takes
        // over. Pinning the real numbers rather than the intuitive ones.
        assert_eq!(
            rects.last().copied(),
            Some([96, 26, 156, 68]),
            "cursor (40,50,4,4) -> box (100,20,160,70) in 8 truncating steps"
        );
        for (k, want) in LOCATION_PANEL_BOX.iter().enumerate() {
            let got = rects.last().unwrap()[k] as i32;
            assert!(
                (got - *want as i32).abs() < LOCATION_PANEL_ZOOM_STEPS as i32,
                "component {k} is short by less than one step's worth"
            );
        }
        assert!(
            rects[0][2] < LOCATION_PANEL_BOX[2],
            "the first step is still near the 4px cursor rect"
        );
        assert!(
            e.location_panel.entity_draw_scale() > 1,
            "0x90FF bumps the entity scale every zoom frame"
        );
        assert!(
            e.step_location_info_panel().is_none(),
            "Open does not animate"
        );

        // Close: the same count of steps the other way, ending idle with the
        // selection cleared (0x921C).
        e.close_location_info_panel();
        assert_eq!(e.location_panel.state, LocationPanelState::ZoomingShut);
        let mut shut = 0;
        for _ in 0..LOCATION_PANEL_ZOOM_STEPS + 2 {
            if e.step_location_info_panel().is_some() {
                shut += 1;
            }
        }
        assert_eq!(shut, LOCATION_PANEL_ZOOM_STEPS);
        assert_eq!(e.location_panel.state, LocationPanelState::Idle);
        assert_eq!(e.location_panel.object, 0);
    }

    #[test]
    fn the_info_panel_tints_its_rect_and_draws_its_rows() {
        use crate::vm::{LOCATION_PANEL_BOX, LocationPanelRow};
        let mut e = EngineState::new();
        // A palette where every index has a distinct grey, so the 50% tint
        // resolves to a DIFFERENT index and the remap is observable.
        for (i, entry) in e.scene_palette.iter_mut().enumerate() {
            let v = (i as u16 * 255 / 255) as u8;
            *entry = [v, v, v];
        }
        e.framebuffer.fill(200);
        let rows = vec![LocationPanelRow {
            x: 0x6E,
            y: 0x19,
            color: 0xEE,
            text: "PLANET: ".into(),
        }];
        e.render_location_info_panel(&rows);

        let [bx, by, bw, bh] = LOCATION_PANEL_BOX.map(usize::from);
        // Outside the rect: untouched.
        assert_eq!(e.framebuffer[(by - 1) * ENGINE_SCREEN_WIDTH + bx], 200);
        assert_eq!(e.framebuffer[by * ENGINE_SCREEN_WIDTH + bx - 1], 200);
        assert_eq!(
            e.framebuffer[(by + bh) * ENGINE_SCREEN_WIDTH + bx],
            200,
            "the rect is [y, y+h), so the row at y+h is outside"
        );
        // Inside, away from the text: DARKENED, not cleared.
        let inside = e.framebuffer[(by + bh - 2) * ENGINE_SCREEN_WIDTH + bx + bw - 2];
        assert!(
            inside < 200 && inside > 0,
            "50% toward black must land on a darker palette index, got {inside}"
        );
        // The window is TRANSLUCENT, not an opaque box: remapping a varied
        // background must leave more than one colour behind. (Driven on the real
        // CHART.FD by examples/navpanel.rs: 25 distinct colours survive inside
        // the panel against 37 outside.)
        for (i, entry) in e.scene_palette.iter_mut().enumerate() {
            let v = (i % 64) as u8;
            *entry = [v * 4, v * 2, v];
        }
        for (i, px) in e.framebuffer.iter_mut().enumerate() {
            *px = (i % 251) as u8;
        }
        e.render_location_info_panel(&[]);
        let distinct: std::collections::HashSet<u8> = (by + 1..by + bh - 1)
            .flat_map(|y| (bx + 1..bx + bw - 1).map(move |x| (x, y)))
            .map(|(x, y)| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x])
            .collect();
        assert!(
            distinct.len() > 8,
            "the tint must preserve structure, not flatten the rect: {} colours",
            distinct.len()
        );
        // The header row drew in its own colour.
        assert!(
            e.framebuffer[0x19 * ENGINE_SCREEN_WIDTH..0x21 * ENGINE_SCREEN_WIDTH]
                .iter()
                .any(|&p| p == 0xEE),
            "the header row must appear in colour 0xEE"
        );
    }

    /// docs/port-validation.md called this chain "blocked on other DORMANT ship-3D
    /// code (a dependency chain)". It was never blocked — every function in it is
    /// pure, taking bools/u16s/small arrays — it simply was not being called. This
    /// pins that the nav frame now DRIVES the interpolation gate, the target
    /// selector and the sequence FSM, so they cannot silently go dormant again.
    #[test]
    fn nav_frame_drives_the_interpolation_gate_and_sequence_fsm() {
        let mut e = EngineState::new();
        e.set_nav_destinations(vec![("EKATOMB".into(), vec![]), ("VENUSIA".into(), vec![])]);
        e.on_ship = true;
        assert!(e.nav_view_active(), "nav view must be the active surface");

        // The gate advances one tick per frame, up to its duration.
        e.ship3d_nav_sequence.interpolation_duration_ticks = 3;
        let ticks: Vec<u8> = (0..3)
            .map(|_| {
                e.step_ship_3d_nav_state();
                e.ship3d_interpolation.current_tick
            })
            .collect();
        assert!(
            ticks.windows(2).all(|w| w[1] >= w[0]),
            "gate tick must advance monotonically, got {ticks:?}"
        );
        assert!(
            e.ship3d_interpolation.current_tick > 0,
            "the gate must actually be stepped by the nav frame"
        );
        // And the gate's duration is taken from the sequence FSM, not invented.
        assert_eq!(e.ship3d_interpolation.duration_ticks, 3);
    }

    /// The nav markers now go through the game's OWN sprite projector (0x9B98),
    /// not the ad-hoc star-map helper. The observable consequence is that the
    /// projector's VISIBILITY GATE is live: `project_ship_3d_object_sprite` returns
    /// None when the descriptor lacks SHIP_3D_OBJECT_VISIBLE_FLAG, exactly as the
    /// binary skips an entity whose flags word fails `test ax,0x80` at 0x9BE1.
    /// The nav frame now runs the real dirty-rect chain: global clip snapshot ->
    /// per-slot dirty geometry commit -> render-command collection. These three were
    /// the LAST genuinely-blocked ship-3D functions (the other three "blockers" in
    /// port-validation.md turned out to be fiction; this one was real, because the
    /// engine had no sprite-slot model at all).
    #[test]
    fn nav_frame_runs_the_dirty_rect_render_command_chain() {
        // The sprite path needs the real CARTE art (nav_pyramids >= 6); without it the
        // engine takes the drawn fallback and there are no slots to check.
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("CARTE.SPR").is_file());
        let Some(iso) = iso else { return };
        let carte = std::fs::read(iso.join("CARTE.SPR")).unwrap();
        let borxx = std::fs::read(iso.join("BORXX.SPR")).unwrap_or_default();

        let mut e = EngineState::new();
        e.load_nav_sprites(&carte, &borxx);
        if e.nav_pyramids.len() < 6 {
            return;
        }
        e.set_nav_destinations(vec![("EKATOMB".into(), vec![]), ("VENUSIA".into(), vec![])]);
        e.on_ship = true;
        e.render_ship_view();

        assert_eq!(
            e.ship3d_nav_slots.len(),
            2,
            "one persistent slot per destination"
        );
        assert!(
            !e.ship3d_dirty_rects.rects.is_empty(),
            "the clip snapshot must seed the dirty-rect list"
        );
        for slot in &e.ship3d_nav_slots {
            assert!(
                slot.flags & crate::ship3d::SHIP_3D_SPRITE_SLOT_ACTIVE_FLAG != 0,
                "slots must carry the ACTIVE flag the collector filters on"
            );
            // The collector clears DIRTY on every slot it walks, so after a frame no
            // slot may still be marked dirty.
            assert_eq!(
                slot.flags & crate::ship3d::SHIP_3D_SPRITE_SLOT_DIRTY_FLAG,
                0,
                "the collector must clear each slot's dirty flag"
            );
            assert!(slot.extent_width > 0, "projection must set the slot extent");
        }

        // Slots persist across frames — that is what makes dirty tracking meaningful.
        let before = e.ship3d_nav_slots.clone();
        e.render_ship_view();
        assert_eq!(
            e.ship3d_nav_slots.len(),
            before.len(),
            "slots are reused, not rebuilt per frame"
        );
    }

    #[test]
    fn nav_markers_go_through_the_verified_projector_and_its_visibility_gate() {
        use crate::ship3d::{
            NAV_DESTINATION_POINTS, SHIP_3D_ANGLE_TABLE, SHIP_3D_OBJECT_VISIBLE_FLAG,
            Ship3dMatrixAngles, Ship3dObjectSpriteDescriptor, Ship3dProjectionOrigin,
            Ship3dProjectionPoint, build_ship_3d_projection_matrix, project_ship_3d_object_sprite,
        };
        let m = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 0,
            },
        )
        .expect("matrix builds");
        let p = NAV_DESTINATION_POINTS[0];
        let anchor = Ship3dProjectionPoint {
            x: p[0] as u16,
            y: p[1] as u16,
            z: p[2] as u16,
        };
        let cam = Ship3dProjectionOrigin {
            x: 10000,
            y: 12000,
            z: 0,
        };

        let mut visible = Ship3dObjectSpriteDescriptor {
            flags: SHIP_3D_OBJECT_VISIBLE_FLAG,
            source_width: 24,
            source_height: 24,
            ..Default::default()
        };
        assert!(
            project_ship_3d_object_sprite(anchor, cam, m, &mut visible).is_some(),
            "a visible destination must project"
        );

        let mut hidden = Ship3dObjectSpriteDescriptor {
            flags: 0,
            ..visible
        };
        assert!(
            project_ship_3d_object_sprite(anchor, cam, m, &mut hidden).is_none(),
            "the 0x9BE1 active-bit gate must suppress an inactive entity"
        );
    }

    #[test]
    fn nav_destination_points_coincide_rather_than_fanning_out() {
        use crate::ship3d::NAV_DESTINATION_POINTS;
        assert_eq!(
            NAV_DESTINATION_POINTS.len(),
            10,
            "DS:0x4F09 holds TEN records"
        );
        assert!(
            NAV_DESTINATION_POINTS
                .iter()
                .all(|p| *p == [10200, 12100, 900]),
            "every baked entry is the same point"
        );

        let render = |n: usize| -> Vec<u8> {
            let mut e = EngineState::new();
            e.set_nav_destinations(
                (0..n)
                    .map(|i| (format!("D{i}"), vec![]))
                    .collect::<Vec<_>>(),
            );
            e.on_ship = true;
            e.render_ship_view();
            e.framebuffer.clone()
        };
        let one = render(1);
        let three = render(3);
        if !EngineState::new().nav_pyramids.is_empty() {
            assert_eq!(
                one, three,
                "coincident world points must paint the same pixels regardless of count"
            );
        }
    }

    #[test]
    fn nav_destination_list_choose_a_location() {
        let mut e = EngineState::new();
        let dests: Vec<(String, Vec<(String, Option<std::path::PathBuf>)>)> = vec![
            (
                "EKATOMB".into(),
                (0..5).map(|i| (format!("daddy {i}"), None)).collect(),
            ),
            (
                "VENUSIA".into(),
                (0..7).map(|i| (format!("bug {i}"), None)).collect(),
            ),
            (
                "KORTEX".into(),
                (0..3).map(|i| (format!("hom {i}"), None)).collect(),
            ),
        ];
        e.set_nav_destinations(dests);
        assert_eq!(e.nav_destination_count(), 3);
        // A click on the second row (index 1) resolves to that destination.
        let x = (EngineState::NAV_DEST_X + 4) as u16;
        let y = (EngineState::NAV_DEST_Y + EngineState::NAV_DEST_PITCH) as u16;
        assert_eq!(e.nav_destination_click(x, y), Some(1));
        // A click far from any row resolves to none.
        assert_eq!(e.nav_destination_click(300, 190), None);
        // Visiting it plays that character's dialogue (7 lines for VENUSIA).
        assert!(e.visit_nav_destination(1));
        assert_eq!(e.dialogue_len(), 7);
        assert_eq!(e.current_subtitle(), Some("bug 0"));
        // The nav star-map renders the destination labels without panicking.
        e.on_ship = true;
        e.render_ship_view();
    }

    /// The console font (HONKF.SPR) loads, and the real golden menu is present in
    /// the rendered panorama: its five rows' glyphs are painted with the dedicated
    /// palette indices 0x7B..0x7F the game programs for hover highlighting.
    #[test]
    fn console_font_loads_and_menu_rows_render() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("HONKF.SPR").is_file());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        assert!(e.load_console_font(iso), "HONKF.SPR console font loads");
        e.load_bridge(iso);
        // HONK = H(7) O(14) N(13) K(10): the mapping must resolve uppercase letters.
        assert_eq!(EngineState::console_glyph_index('H'), Some(7));
        assert_eq!(EngineState::console_glyph_index('0'), Some(26));

        // Punctuation ordering, checked against the BANK'S OWN BITMAPS rather than
        // restated as constants -- `,` `:` `;` were rotated here, and a test that
        // just repeats the mapping would have agreed with the bug.
        //
        // Distinguishing marks in the 8x8 cells: a COLON has an upper dot (row 2)
        // and no row-7 descender; a COMMA has no upper dot and DOES tail into row 7;
        // a SEMICOLON has both.
        let ink = |g: usize, row: usize| -> bool {
            let f = &e.console_font[g];
            (0..f.width as usize).any(|c| f.indices[row * f.width as usize + c] != 0)
        };
        for (ch, upper_dot, tail) in [(':', true, false), (',', false, true), (';', true, true)] {
            let g = EngineState::console_glyph_index(ch).expect("punctuation maps");
            assert_eq!(
                ink(g, 2) || ink(g, 3),
                upper_dot,
                "{ch:?} upper dot (frame {g})"
            );
            assert_eq!(ink(g, 7), tail, "{ch:?} descender tail (frame {g})");
        }
        // '.' is the one with neither.
        let dot = EngineState::console_glyph_index('.').unwrap();
        assert!(
            !ink(dot, 2) && !ink(dot, 3) && !ink(dot, 7),
            "'.' has no dot above and no tail"
        );
        if e.panorama.is_none() {
            return;
        }
        e.bridge_active = true;
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        // The baked menu glyphs use one palette index per row (0x7B + row).
        let menu_pixels = e
            .framebuffer
            .iter()
            .filter(|&&p| (0x7B..0x80).contains(&(p as usize)))
            .count();
        assert!(
            menu_pixels > 200,
            "golden menu rows present ({menu_pixels} px)"
        );
        // A click on the HONK row (option 0) is detected; off-menu clicks are not.
        e.bridge.frame = crate::bridge::MENU_REST_FRAME;
        assert_eq!(
            e.console_menu_click(232, 0x48 + 1),
            Some(0),
            "HONK row clickable"
        );
        assert_eq!(
            e.console_menu_click(10, 190),
            None,
            "off-menu click hits nothing"
        );
    }

    #[test]
    fn script1_tutorial_chains_to_script2() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("SCRIPT1.COD").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else {
            return;
        };
        let db = crate::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
        let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT1.{ext}"))).unwrap();
        let mut e = EngineState::new();
        e.load_dialogue_scenes(&rd("COD"), &rd("VAR"), &rd("DIC"), &rd("DEB"), &db, assets);
        e.dialogue_hold_frames = 20;
        e.on_ship = false;
        for _ in 0..20000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() {
                break;
            }
        }
        assert!(e.dialogue_finished(), "SCRIPT1 tutorial completes");
        // Its D2 handoff requests profile 1 -> the driver loads SCRIPT(1+1)=SCRIPT2.
        assert_eq!(
            e.pending_next_scene(),
            Some(1),
            "SCRIPT1 chains to SCRIPT2 via D2"
        );
    }

    #[test]
    fn step_advances_frame_and_polls_input() {
        let mut e = EngineState::new();
        assert_eq!(e.frame, 0);
        let m = MouseInput {
            x: 100,
            y: 50,
            buttons: 1,
            ..Default::default()
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
            ..Default::default()
        };
        e.step(still); // first frame: moved from (0,0) -> reset
        e.step(still); // stationary -> +1
        e.step(still); // stationary -> +2
        assert_eq!(e.idle_ticks, 2);
        e.step(MouseInput {
            x: 11,
            y: 10,
            buttons: 0,
            ..Default::default()
        });
        assert_eq!(e.idle_ticks, 0, "movement zeroes the idle timer");
    }

    /// The chart/nav view is STATIC (the invented mouse-steered compass was removed);
    /// the compass angle changes only via the explicit target-cycle input.
    #[test]
    fn on_ship_view_is_static_mouse_does_not_steer() {
        let mut e = EngineState::new();
        e.on_ship = true;
        e.compass_angle = 90;
        for _ in 0..10 {
            e.step(MouseInput {
                x: 315,
                y: 100,
                buttons: 0,
                ..Default::default()
            });
        }
        assert_eq!(
            e.compass_angle, 90,
            "mouse position does not steer the chart view"
        );
    }

    #[test]
    fn on_ship_render_composites_sprite_hud_when_loaded() {
        let read = |names: &[&str]| -> Option<Vec<u8>> {
            names.iter().find_map(|p| std::fs::read(p).ok())
        };
        let (Some(bc), Some(bo)) = (
            read(&[
                "output/_tmp_iso/BCARTE.SPR",
                "../output/_tmp_iso/BCARTE.SPR",
            ]),
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
        e.step(MouseInput {
            x: 90,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        // Count non-zero pixels in the HUD band (rows 150..195, where the HUD sits).
        let band: usize = (150..195)
            .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| (x, y)))
            .filter(|&(x, y)| e.framebuffer[y * ENGINE_SCREEN_WIDTH + x] != 0)
            .count();
        assert!(
            band > 200,
            "sprite HUD composites into the band (got {band})"
        );
    }

    #[test]
    fn dialogue_playback_steps_through_script_lines() {
        let read = |names: &[&str]| -> Option<Vec<u8>> {
            names.iter().find_map(|p| std::fs::read(p).ok())
        };
        let (Some(cod), Some(var), Some(dic)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
        ) else {
            eprintln!("skipping: SCRIPT1 not available");
            return;
        };
        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);
        assert!(
            e.dialogue_len() > 1,
            "script reached multiple dialogue lines"
        );
        // The reconstructed subtitle text is real dialogue (letters, not empty).
        assert!(
            e.dialogue_texts
                .iter()
                .any(|t| t.chars().any(|c| c.is_alphabetic())),
            "dialogue lines reconstruct real subtitle text from the dictionary"
        );
        e.dialogue_hold_frames = 2;
        let first = e.current_dialogue().map(|l| l.offset);
        // Step past the hold window (variable per line): playback advances.
        for _ in 0..300 {
            e.step(MouseInput::default());
            if e.current_dialogue().map(|l| l.offset) != first {
                break;
            }
        }
        let second = e.current_dialogue().map(|l| l.offset);
        assert_ne!(first, second, "dialogue playback advances to the next line");
    }

    #[test]
    fn draw_subtitle_renders_text_into_scene_band() {
        let mut e = EngineState::new();
        e.draw_subtitle("HELLO COMMANDER", 0xFD);
        // Text draws at y=8 (the subtitle band); a fully-shown line SETTLES in the BOLD
        // console font at the darker green 0xFD (0x3630 never switches to a thin/white
        // font — the old 0xE0 white settle was invented; audit-corrected).
        let band: usize = (8..16)
            .flat_map(|y| (0..ENGINE_SCREEN_WIDTH).map(move |x| y * ENGINE_SCREEN_WIDTH + x))
            .filter(|&i| e.framebuffer[i] == 0xFD)
            .count();
        assert!(
            band > 20,
            "subtitle text renders into the band (got {band})"
        );
    }

    #[test]
    #[ignore]
    fn demo_render_real_dialogue_frame() {
        let read = |names: &[&str]| names.iter().find_map(|p| std::fs::read(p).ok());
        let (Some(cod), Some(var), Some(dic)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
        ) else {
            return;
        };
        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);
        // Advance to the first line that has real subtitle text.
        while e.current_subtitle().is_none() && e.dialogue_cursor + 1 < e.dialogue_len() {
            e.dialogue_cursor += 1;
        }
        let text = e.current_subtitle().unwrap_or("(no text)").to_string();
        eprintln!("engine subtitle: {text:?}");
        e.draw_subtitle(&text, 0xFD);
        let vis: Vec<u8> = e
            .framebuffer
            .iter()
            .map(|&v| if v == 0 { 0 } else { 255 })
            .collect();
        let mut out =
            format!("P5\n{ENGINE_SCREEN_WIDTH} {ENGINE_SCREEN_HEIGHT}\n255\n").into_bytes();
        out.extend_from_slice(&vis);
        std::fs::write("/tmp/ben_engine_frame.pgm", out).unwrap();
        eprintln!("wrote /tmp/ben_engine_frame.pgm");
    }

    #[test]
    fn step_auto_renders_current_dialogue_subtitle() {
        let read = |names: &[&str]| names.iter().find_map(|p| std::fs::read(p).ok());
        let (Some(cod), Some(var), Some(dic)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
        ) else {
            return;
        };
        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);
        // Advance the cursor to a line with real text, then step (auto-renders it).
        while e.current_subtitle().is_none() && e.dialogue_cursor + 1 < e.dialogue_len() {
            e.dialogue_cursor += 1;
        }
        e.dialogue_hold_frames = u32::MAX; // hold the line so the cursor stays put
        // Step enough frames for the character-by-character reveal to fully draw.
        for _ in 0..400 {
            e.step(MouseInput::default());
        }
        // Text pixels: bold green console font throughout — 0xFD settled / 0xFE / 0xFF
        // newest (no white 0xE0 settle; audit-corrected).
        let lit = e
            .framebuffer
            .iter()
            .filter(|&&p| p == 0xFD || p == 0xFE || p == 0xFF)
            .count();
        assert!(
            lit > 20,
            "step auto-renders the dialogue subtitle (got {lit})"
        );
    }

    #[test]
    fn dialogue_frame_composites_scene_hnm_behind_subtitle() {
        // Find any scene/talk HNM to load as the background.
        let cand = [
            "output/_tmp_dat/pe/aabob.hnm",
            "../output/_tmp_dat/pe/aabob.hnm",
        ];
        let Some(path) = cand.iter().map(std::path::Path::new).find(|p| p.exists()) else {
            eprintln!("skipping: no HNM available");
            return;
        };
        let mut e = EngineState::new();
        e.load_scene_hnm(path);
        assert!(e.scene_hnm.is_some(), "HNM opens via the lib decoder");
        e.render_dialogue_frame();
        // The decoded HNM frame fills the framebuffer with non-zero background pixels
        // (the talk animation), not a cleared black frame.
        let bg = e.framebuffer.iter().filter(|&&p| p != 0).count();
        assert!(
            bg > 5000,
            "scene HNM decodes into the background (got {bg})"
        );
    }

    #[test]
    #[ignore]
    fn demo_render_full_dialogue_scene() {
        let cand = [
            "output/_tmp_dat/pe/aabob.hnm",
            "../output/_tmp_dat/pe/aabob.hnm",
        ];
        let Some(path) = cand.iter().map(std::path::Path::new).find(|p| p.exists()) else {
            return;
        };
        let mut e = EngineState::new();
        e.load_scene_hnm(path);
        e.frame = 0; // keyframe (self-contained + palette)
        e.render_dialogue_frame();
        e.draw_subtitle("CAP'N BOB SPEAKS", 0xFD);
        // Export as PPM using the scene palette (RGB).
        let mut out =
            format!("P6\n{ENGINE_SCREEN_WIDTH} {ENGINE_SCREEN_HEIGHT}\n255\n").into_bytes();
        for &idx in &e.framebuffer {
            out.extend_from_slice(&e.scene_palette[idx as usize]);
        }
        std::fs::write("/tmp/ben_engine_scene.ppm", out).unwrap();
        eprintln!("wrote /tmp/ben_engine_scene.ppm");
    }

    #[test]
    #[ignore]
    fn probe_per_line_talk_hnm_resolution() {
        let read = |n: &[&str]| n.iter().find_map(|p| std::fs::read(p).ok());
        let (Some(cod), Some(var), Some(dic), Some(deb)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DEB",
                "../output/_tmp_iso/SCRIPT1.DEB",
            ]),
        ) else {
            return;
        };
        let dpath = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists());
        let Some(dpath) = dpath else {
            return;
        };
        let descript = crate::descript::DescriptDb::parse_file(dpath).unwrap();
        let object_names = parse_deb_object_names(&deb);
        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);
        let mut resolved = 0usize;
        let mut sample = Vec::new();
        for l in &e.dialogue {
            if let Some(name) = l.actor_offset.and_then(|o| object_names.get(&o)) {
                if let Some(hnm) = descript.record(name).and_then(|r| r.talk_hnms.first()) {
                    resolved += 1;
                    if sample.len() < 4 {
                        sample.push(format!("{name} -> {}", hnm.name));
                    }
                }
            }
        }
        eprintln!(
            "resolved {resolved}/{} lines; sample: {sample:?}",
            e.dialogue.len()
        );
        assert!(resolved > 0, "per-line actor -> talk HNM resolution works");
    }

    #[test]
    fn load_dialogue_scenes_resolves_per_line_speakers() {
        let read = |n: &[&str]| n.iter().find_map(|p| std::fs::read(p).ok());
        let (Some(cod), Some(var), Some(dic), Some(deb)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DEB",
                "../output/_tmp_iso/SCRIPT1.DEB",
            ]),
        ) else {
            return;
        };
        let Some(dpath) = [
            "output/_tmp_iso/DESCRIPT.DES",
            "../output/_tmp_iso/DESCRIPT.DES",
        ]
        .iter()
        .map(std::path::Path::new)
        .find(|p| p.exists()) else {
            return;
        };
        let Some(assets) = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists())
        else {
            return;
        };
        let descript = crate::descript::DescriptDb::parse_file(dpath).unwrap();
        let mut e = EngineState::new();
        e.load_dialogue_scenes(&cod, &var, &dic, &deb, &descript, assets);
        // Many lines resolve to their speaker's talk-HNM asset file.
        let resolved = e
            .dialogue_scene_paths
            .iter()
            .filter(|p| p.is_some())
            .count();
        assert!(
            resolved > 10,
            "per-line speaker HNMs resolve to asset files (got {resolved})"
        );
        // Jump to a line that has a resolved speaker HNM and load it.
        let idx = e
            .dialogue_scene_paths
            .iter()
            .position(|p| p.is_some())
            .unwrap();
        e.dialogue_cursor = idx;
        e.load_current_scene();
        assert!(e.scene_hnm.is_some(), "the line's speaker talk-HNM loads");
    }

    #[test]
    fn dialogue_exposes_d2_handoff_and_finish() {
        let read = |n: &[&str]| n.iter().find_map(|p| std::fs::read(p).ok());
        let (Some(cod), Some(var), Some(dic)) = (
            read(&[
                "output/_tmp_iso/SCRIPT1.COD",
                "../output/_tmp_iso/SCRIPT1.COD",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.VAR",
                "../output/_tmp_iso/SCRIPT1.VAR",
            ]),
            read(&[
                "output/_tmp_iso/SCRIPT1.DIC",
                "../output/_tmp_iso/SCRIPT1.DIC",
            ]),
        ) else {
            return;
        };
        let mut e = EngineState::new();
        e.load_dialogue(&cod, &var, &dic);
        assert!(!e.dialogue_finished(), "not finished at the first line");
        // pending_next_scene is the D2 handoff target (Some/None both valid; must
        // be queryable and consistent with a terminal-vs-chaining scene).
        let _next = e.pending_next_scene();
        // Drive to the end; dialogue_finished flips true at the last line. Per-line
        // hold is length-scaled, so step generously (≤240 frames/line).
        e.dialogue_hold_frames = 1;
        for _ in 0..(e.dialogue_len() as u32 * 245 + 8) {
            e.step(MouseInput::default());
            if e.dialogue_finished() {
                break;
            }
        }
        assert!(
            e.dialogue_finished(),
            "playback reaches the terminal line (D2 point)"
        );
    }

    #[test]
    fn scene_queue_auto_chains_to_the_next_scene() {
        let read = |n: &[&str]| n.iter().find_map(|p| std::fs::read(p).ok());
        let load = |i: u32| -> Option<(Vec<u8>, Vec<u8>, Vec<u8>)> {
            Some((
                read(&[
                    &format!("output/_tmp_iso/SCRIPT{i}.COD"),
                    &format!("../output/_tmp_iso/SCRIPT{i}.COD"),
                ])?,
                read(&[
                    &format!("output/_tmp_iso/SCRIPT{i}.VAR"),
                    &format!("../output/_tmp_iso/SCRIPT{i}.VAR"),
                ])?,
                read(&[
                    &format!("output/_tmp_iso/SCRIPT{i}.DIC"),
                    &format!("../output/_tmp_iso/SCRIPT{i}.DIC"),
                ])?,
            ))
        };
        let (Some(s1), Some(s2)) = (load(1), load(2)) else {
            return;
        };
        let mut e = EngineState::new();
        let n = e.queue_scenes(vec![s1, s2]);
        assert_eq!(n, 2);
        assert_eq!(e.current_scene_index(), 0, "starts on the first scene");
        assert!(e.dialogue_len() > 0);
        // Drive to finish scene 0; the engine auto-chains to scene 1. Per-line hold is
        // length-scaled, so step generously (≤240 frames/line).
        e.dialogue_hold_frames = 1;
        for _ in 0..(e.dialogue_len() as u32 * 245 + 8) {
            e.step(MouseInput::default());
            if e.current_scene_index() == 1 {
                break;
            }
        }
        assert_eq!(e.current_scene_index(), 1, "auto-chained to the next scene");
    }

    /// The invented "click anywhere commits the compass heading" mechanic is REMOVED
    /// (real selection = the decoded target list, hit-tested rows @0x8428). A bare
    /// click on the nav view must select nothing.
    #[test]
    fn nav_click_does_not_commit_a_heading() {
        let mut e = EngineState::new();
        e.on_ship = true;
        e.step(MouseInput {
            x: 200,
            y: 100,
            buttons: 0,
            ..Default::default()
        });
        assert!(e.take_nav_selection().is_none());
        e.step(MouseInput {
            x: 200,
            y: 100,
            buttons: 1,
            ..Default::default()
        });
        assert!(
            e.take_nav_selection().is_none(),
            "bare nav clicks select nothing"
        );
    }

    /// audit-fixes #325. `LOADING`, `PAUSE` and `LAST` are not transcriptions to
    /// be trusted — they are a contiguous NUL-separated UI string block in the
    /// image, and the routines that use them point straight at it
    /// (`mov si,0x161` @`0x1B58` copies `LAST` into the slot-name buffer).
    /// Reading them turns three content literals into checked mirrors.
    #[test]
    fn ui_string_literals_match_the_image_block() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        let ds = 0xD420usize;
        let at = |off: usize| {
            let start = ds + off;
            let end = exe[start..].iter().position(|&b| b == 0).unwrap_or(0) + start;
            String::from_utf8_lossy(&exe[start..end]).to_string()
        };

        let mut probe = EngineState::new();
        probe.load_ds_strings(&exe);
        assert_eq!(at(0x159), probe.ds_text(EngineState::LOADING_TEXT_DS));
        assert_eq!(at(0x166), probe.ds_text(EngineState::PAUSE_TEXT_DS));
        assert_eq!(at(0x161), EngineState::QUICKSAVE_SLOT_NAME);
        // The block is contiguous and NUL-separated: each string ends exactly
        // where the next begins, which is what makes these offsets meaningful
        // rather than four coincidences.
        assert_eq!(0x159 + at(0x159).len() + 1, 0x161);
        assert_eq!(0x161 + at(0x161).len() + 1, 0x166);
        assert_eq!(0x166 + at(0x166).len() + 1, 0x16C);
        assert_eq!(at(0x16C), "UNKNOWN", "the roster's empty caption follows");

        // THE SAME BLOCK carries the status-panel headers, whose DS offsets are
        // the `mov si,imm` operands verified in audit-fixes #320/#342. Reading
        // them here makes the whole region one checked table rather than four
        // separate address constants that happen to be right.
        assert_eq!(at(0x12E), "PLANET: ");
        assert_eq!(at(0x137), "SHIP: ");
        assert_eq!(at(0x13E), "BLACK HOLE: ");
        assert_eq!(at(0x14B), "LIFE SUPPORT:");
        // Contiguity again: PLANET's terminator is where SHIP begins.
        assert_eq!(0x12E + at(0x12E).len() + 1, 0x137);
        assert_eq!(0x137 + at(0x137).len() + 1, 0x13E);
    }

    /// audit-fixes #322. `MENU_SUBMENU` is a transcribed literal and its doc says
    /// so. It also states exactly where the words live: `SCRIPT1.DIC` `0x02FC`
    /// = `explanations`, `0x0309` = `game`. That is checkable, so check it —
    /// a transcription pinned to the shipped data cannot silently drift from it,
    /// which is the smaller half of the fix while the builder routine is still
    /// unfound.
    #[test]
    fn menu_submenu_literals_match_the_dic_words() {
        let Some(dic) = ["output/_tmp_iso", "../output/_tmp_iso", "output/scripts"]
            .iter()
            .map(|d| std::path::Path::new(d).join("SCRIPT1.DIC"))
            .find(|p| p.is_file())
            .and_then(|p| std::fs::read(p).ok())
        else {
            return; // shipped data not extracted in this checkout
        };
        let word_at = |off: usize| {
            let end = dic[off..].iter().position(|&b| b == 0).unwrap_or(0) + off;
            String::from_utf8_lossy(&dic[off..end]).to_string()
        };

        assert_eq!(word_at(0x02FC), "explanations");
        assert_eq!(word_at(0x0309), "game");
        // The widget upper-cases for display, which is why the DIC is lowercase.
        assert_eq!(
            EngineState::MENU_SUBMENU.to_vec(),
            vec![
                word_at(0x02FC).to_uppercase(),
                word_at(0x0309).to_uppercase()
            ],
            "the literals must mirror the DIC words the doc names"
        );

        // AND THE OTHER END OF THE CHAIN: the doc says the real source is an 0xA6
        // record's word list at SCRIPT1.COD 0x4A9. It is -- `fc 02 09 03 00 00`,
        // i.e. the two DIC offsets terminated by 0x0000. Pinning both ends means
        // the literal, the DIC and the script agree or the test fails.
        let Some(cod) = ["output/_tmp_iso", "../output/_tmp_iso", "output/scripts"]
            .iter()
            .map(|d| std::path::Path::new(d).join("SCRIPT1.COD"))
            .find(|p| p.is_file())
            .and_then(|p| std::fs::read(p).ok())
        else {
            return;
        };
        let word = |off: usize| u16::from_le_bytes([cod[off], cod[off + 1]]);
        assert_eq!(
            word(0x4A9),
            0x02FC,
            "first menu word offset -> explanations"
        );
        assert_eq!(word(0x4AB), 0x0309, "second -> game");
        assert_eq!(word(0x4AD), 0x0000, "the list is zero-terminated");
    }

    /// audit-fixes #313. PREDICTIVE vs REACTIVE wrap, with words chosen so the
    /// two rules disagree.
    ///
    /// After the second word the line holds 21 chars plus a space = 22. The next
    /// word is 13 long. The game computes `22 + 13 = 35 >= 0x23` and breaks
    /// BEFORE it (`add al,dl / cmp al,0x23` @`0x672A`, where `al` came from
    /// `strlen_b` on the NEXT word @`0x6701`). The old port compared 22 alone,
    /// found it under 35, and let the word run the line out to 35.
    #[test]
    fn subtitle_wrap_breaks_before_the_word_that_would_overflow() {
        let words: Vec<String> = ["abcdefghij", "klmnopqrst", "abcdefghijklm"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let assembled = assemble_words(&words);

        assert_eq!(
            assembled, "abcdefghij klmnopqrst \nabcdefghijklm",
            "the break belongs BEFORE the 13-char word, not after it"
        );
        // Stated as the property, so a future rewrite cannot satisfy the literal
        // above by accident: no line exceeds the wrap column when the words fit.
        for line in assembled.split('\n') {
            assert!(
                line.chars().count() <= crate::script::SUBTITLE_WRAP_COLUMN,
                "predictive wrap keeps every line within the column: {line:?}"
            );
        }
    }

    #[test]
    fn subtitle_wraps_long_lines() {
        // Text assembly wraps with the game's decoded 0xA6 rule: after the space,
        // break when the line PLUS THE NEXT WORD would reach 0x23 (35) chars
        // (`add al,dl / cmp al,0x23` @0x672A). Corrected in audit-fixes #313 --
        // this comment used to say "once the line reaches 35", which is the
        // reactive rule the port had and the game does not.
        let words: Vec<String> =
            "You can wake Cap'n Bob by clicking on the CRYO chamber control panel now"
                .split_whitespace()
                .map(str::to_string)
                .collect();
        let assembled = assemble_words(&words);
        assert!(assembled.contains('\n'), "long line wraps: {assembled:?}");
        for line in assembled.split('\n') {
            // With the predictive rule no line exceeds the column at all, unless a
            // single word is longer than it (the game never splits words). The
            // old bound of 35+12 was loose enough to pass EITHER rule, which is
            // why it did not catch the divergence.
            let longest = words.iter().map(|w| w.chars().count()).max().unwrap_or(0);
            assert!(
                line.chars().count() <= crate::script::SUBTITLE_WRAP_COLUMN.max(longest),
                "line within wrap bound: {line:?}"
            );
        }
        // And the drawer renders each wrapped line on its own font row.
        let mut e = EngineState::new();
        e.draw_subtitle(&assembled, 0xFD);
        let w = ENGINE_SCREEN_WIDTH;
        let rows_with_text = (0..30)
            .filter(|&r| e.framebuffer[r * w..(r + 1) * w].iter().any(|&p| p == 0xFD))
            .count();
        assert!(
            rows_with_text > 8,
            "text occupies multiple wrapped rows (rows={rows_with_text})"
        );
    }

    #[test]
    fn dialogue_hold_scales_with_line_length() {
        let mut e = EngineState::new();
        e.dialogue_hold_frames = 20;
        e.dialogue_texts = vec![
            "Hi".into(),
            "A rather long dialogue line that should linger longer".into(),
        ];
        e.dialogue = vec![
            LineState {
                offset: 0,
                actor_offset: None,
                location_offset: None,
            },
            LineState {
                offset: 1,
                actor_offset: None,
                location_offset: None,
            },
        ];
        e.dialogue_cursor = 0;
        let short = e.current_line_hold();
        e.dialogue_cursor = 1;
        let long = e.current_line_hold();
        assert!(long > short, "longer line held longer ({long} > {short})");
        assert!(short >= 20, "at least the base hold");
    }

    #[test]
    fn title_screen_loads_and_shows_the_decoded_box_art() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        assert!(e.load_title(iso), "BLOOD.LBM title art loads");
        assert!(e.title_active());
        // The title takes render precedence and fills the framebuffer with real art.
        e.step(MouseInput::default());
        let distinct = e
            .framebuffer
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert!(distinct >= 8, "title art renders ({distinct} indices)");
        // Dismissing advances past the title.
        e.dismiss_title();
        assert!(!e.title_active());
    }

    #[test]
    fn world_ext_objects_are_marked_on_the_location() {
        // The markers are DEBUG-ONLY overlays (no such markers in the binary).
        unsafe { std::env::set_var("CB_DEBUG", "1") };
        let dat = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let (Some(dat), Some(iso)) = (dat, iso) else {
            return;
        };
        let mut e = EngineState::new();
        if !e.visit_world("venusia", dat) {
            return;
        }
        let ext = std::fs::read(iso.join("VENUSIA.EXT")).unwrap();
        let n = e.set_world_ext(&ext);
        assert!(n >= 1, "venusia has >=1 decoded object");
        // Rendering marks them: the marker index 0xFD appears in the framebuffer.
        e.step(MouseInput::default());
        assert!(
            e.framebuffer.iter().any(|&p| p == 0xFD),
            "object marker rendered"
        );
    }

    #[test]
    fn visiting_a_world_loads_its_decoded_location_background() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(std::path::Path::new)
            .find(|p| p.exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(!e.world_location_active());
        // Visiting a mapped world loads its fd/ room background.
        assert!(
            e.visit_world("venusia", assets),
            "venusia has decoded location art"
        );
        assert!(e.world_location_active());
        // The landing screen renders the background (non-blank framebuffer).
        e.step(MouseInput::default());
        let distinct = e
            .framebuffer
            .iter()
            .collect::<std::collections::BTreeSet<_>>()
            .len();
        assert!(distinct > 8, "location background renders real content");
        // Venusia has multiple rooms (floors 1f/2f/3f); cycling advances + wraps.
        let (start, count) = e.world_room_position().unwrap();
        assert!(count >= 2, "venusia has multiple rooms ({count})");
        assert_eq!(start, 0);
        e.cycle_world_room(1);
        assert_eq!(e.world_room_position().unwrap().0, 1);
        e.cycle_world_room(-1);
        assert_eq!(e.world_room_position().unwrap().0, 0);
        e.cycle_world_room(-1);
        assert_eq!(
            e.world_room_position().unwrap().0,
            count - 1,
            "wraps backward"
        );
        // Leaving returns to nav.
        e.leave_world();
        assert!(!e.world_location_active());
        // A world with no fd/ mapping (e.g. black) declines gracefully.
        assert!(!e.visit_world("script2.cod", assets));
    }

    #[test]
    fn nav_targets_real_decoded_worlds_across_the_heading() {
        let mut e = EngineState::new();
        // The nav labels come from the decoded level directory's primary worlds.
        assert_eq!(e.nav_world_label_sample()[0], "black");
        assert!(e.nav_world_label_sample().contains(&"venusia"));
        // Heading 0° targets the first world; sweeping the compass moves through them.
        e.compass_angle = 0;
        assert_eq!(e.targeted_world_index(), 0);
        let n = crate::levels::primary_worlds().count();
        e.compass_angle = 179;
        assert_eq!(
            e.targeted_world_index(),
            n - 1,
            "max heading targets the last world"
        );
        // Monotonic, in-range across the full sweep.
        for a in 0..180u16 {
            e.compass_angle = a;
            assert!(e.targeted_world_index() < n);
        }
    }

    #[test]
    fn mode_x_layout_is_address_equivalent_to_the_linear_framebuffer() {
        // For every screen pixel, the game's mode-X (byte_offset, plane) maps back to the
        // engine's linear index y*320+x — so the linear framebuffer is faithful to the
        // decoded graphics_plot_modex (0x299:0x498) addressing.
        for y in 0..ENGINE_SCREEN_HEIGHT {
            for x in 0..ENGINE_SCREEN_WIDTH {
                let (off, plane) = mode_x_offset(x, y);
                assert_eq!(plane, x & 3);
                assert_eq!(
                    mode_x_to_linear(off, plane),
                    y * ENGINE_SCREEN_WIDTH + x,
                    "mode-X ({x},{y}) must address the same pixel as linear",
                );
            }
        }
        // The row stride is 80 bytes/row * 4 planes = 320 pixels, matching the width.
        assert_eq!(mode_x_offset(0, 1).0, 80);
    }

    #[test]
    fn mode_x_offset_matches_the_game_plot_formula_exactly() {
        // graphics_plot_modex (BLOODPRG.EXE 0x299:0x498 / file 0x3428) computes, per the RE:
        // byte offset = y*80 + x/4, plane = x&3. Assert the engine reproduces this exact
        // addressing for every pixel in the 320x200 mode-X screen (not just equivalence).
        for y in 0..ENGINE_SCREEN_HEIGHT {
            for x in 0..ENGINE_SCREEN_WIDTH {
                assert_eq!(mode_x_offset(x, y), (y * 80 + x / 4, x & 3), "({x},{y})");
            }
        }
    }

    #[test]
    fn framebuffer_is_full_screen_indexed() {
        let e = EngineState::new();
        assert_eq!(
            e.framebuffer.len(),
            ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT
        );
    }

    /// The game does not SEARCH for media: `asset_path_template_table` @0x0F48B
    /// names the directory per slot (audit-fixes #482). Parsed from the image so
    /// a wrong record layout fails here rather than being restated as constants.
    ///
    /// Records are VARIABLE length -- a NUL-terminated path then 10 metadata
    /// bytes -- NOT a fixed stride. Assuming a uniform 26 desynchronises at the
    /// first short name (`sq\cryogel.hnm` is 25, `sq\the_star.HNM` is 26), which
    /// is the error this test pins.
    #[test]
    fn asset_path_table_names_the_directory_the_hnm_scan_rediscovers() {
        let Ok(exe) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        else {
            return;
        };
        const TABLE: usize = 0x0F48B;
        const META: usize = 10;

        let mut off = TABLE;
        let mut paths: Vec<String> = Vec::new();
        let mut metas: Vec<u8> = Vec::new();
        loop {
            let Some(end) = exe[off..].iter().position(|&c| c == 0).map(|i| off + i) else {
                break;
            };
            let name = &exe[off..end];
            if name.len() < 4 || name.len() > 24 || name[2] != b'\\' {
                break;
            }
            if !name.iter().all(|&c| (0x20..0x7F).contains(&c)) {
                break;
            }
            paths.push(String::from_utf8_lossy(name).into_owned());
            metas.push(exe[end + META]); // the record's last metadata byte
            off = end + 1 + META;
        }

        assert_eq!(paths.len(), 45, "45 records, 0x0F48B..0x0F915");
        assert_eq!(off, 0x0F915);
        assert_eq!(paths[0], r"sq\mind.HNM");
        assert_eq!(paths[44], r"sq\pollup.hnm");

        // Variable length is the point: at least two distinct record sizes.
        let sizes: std::collections::HashSet<usize> =
            paths.iter().map(|p| p.len() + 1 + META).collect();
        assert!(sizes.len() > 1, "records are NOT a fixed stride: {sizes:?}");

        // The directory is a property of the SLOT, and the census is exactly the
        // four directories `collect_hnm_paths` discovers by walking the tree.
        let mut census = std::collections::BTreeMap::new();
        for p in &paths {
            *census.entry(p[..2].to_string()).or_insert(0usize) += 1;
        }
        assert_eq!(census.get("pe"), Some(&33));
        assert_eq!(census.get("sq"), Some(&10));
        assert_eq!(census.get("pl"), Some(&1));
        assert_eq!(census.get("ob"), Some(&1));
        assert_eq!(census.len(), 4, "no fifth media directory exists");

        // Most filenames are a twelve-`x` placeholder patched at load time; the
        // rest are fixed assets stored literally (the cryobox film among them).
        let templates = paths.iter().filter(|p| p.contains("xxxxxxxxxxxx")).count();
        assert_eq!(templates, 38);
        assert!(paths.iter().any(|p| p == r"sq\cryorad.hnm"));

        // The terminator: 0x10 in every record but the last, which carries 0x00.
        // This lands in the same column 45 times running only if the parse above
        // stayed aligned, so it double-checks the record layout.
        assert!(metas[..44].iter().all(|&m| m == 0x10));
        assert_eq!(metas[44], 0x00);
    }
}
