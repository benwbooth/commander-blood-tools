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

use crate::font::draw_text_indexed;
use crate::hnm::HnmFile;
use crate::ship3d::{
    BloodPrng, Ship3dMatrixAngles, Ship3dProjectionOrigin, Ship3dProjectionViewport,
    render_ship_3d_starfield,
};
use crate::sprite::{SpriteFrameImage, blit_sprite_frame_centered, decode_sprite_bank_indices};
use crate::vm::{LineState, VmToken, execute_trace, walk};
use std::collections::HashMap;
use std::path::Path;

/// Parse a `SCRIPTn.DIC` dictionary: NUL-terminated words keyed by their byte
/// offset (a Text token's `word_offsets` index into this).
fn parse_dictionary(dic: &[u8]) -> HashMap<u16, String> {
    let mut words = HashMap::new();
    let mut pos = 0usize;
    while pos < dic.len() {
        let start = pos;
        while pos < dic.len() && dic[pos] != 0 {
            pos += 1;
        }
        if pos > start {
            words.insert(
                start as u16,
                String::from_utf8_lossy(&dic[start..pos]).into_owned(),
            );
        }
        pos += 1;
    }
    words
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
            names.insert(offset, String::from_utf8_lossy(&r[..nl]).into_owned());
        }
    }
    names
}

/// Recursively collect `*.hnm` asset paths under `dir`, keyed by lowercase
/// filename, so a DESCRIPT talk-HNM name resolves to its file.
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
fn assemble_words(parts: &[String]) -> String {
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
                if line_len >= 0x23 {
                    out.push('\n');
                    line_len = 0;
                }
            }
        }
    }
    out
}

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
    /// Set to the compass heading of the destination the player committed by clicking
    /// in the nav view (edge-triggered). A driver polls [`EngineState::take_nav_selection`]
    /// to load that destination's dialogue — the nav→dialogue game-loop hook.
    nav_selection: Option<u16>,
    /// Previous-frame left-button state, for edge-detecting nav clicks.
    prev_left_down: bool,
    /// Deterministic PRNG seed for the starfield point cloud (the engine seeds
    /// from CMOS RTC seconds at runtime; fixed here for reproducibility).
    pub starfield_seed: u8,
    /// Decoded ship-nav HUD sprite banks: BCARTE perspective grid frames.
    hud_grid: Vec<SpriteFrameImage>,
    /// Decoded ship-nav HUD orb sprite frames (BORXX).
    hud_orb: Vec<SpriteFrameImage>,
    /// The game's star-map destination pyramid frames (CARTE.SPR f0..f5, six
    /// pre-scaled sizes) + selection reticle (f6) — the real art drawn by the sprite
    /// path at projected destination positions.
    nav_pyramids: Vec<SpriteFrameImage>,
    /// Dialogue line sequence for the loaded script (from the VM trace), played
    /// back frame-by-frame — the script/scene stepping the main loop drives.
    dialogue: Vec<LineState>,
    /// The reconstructed subtitle text for each `dialogue` line (parallel vec).
    dialogue_texts: Vec<String>,
    /// Playback cursor into [`EngineState::dialogue`].
    dialogue_cursor: usize,
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
            hud_grid: Vec::new(),
            hud_orb: Vec::new(),
            nav_pyramids: Vec::new(),
            dialogue: Vec::new(),
            dialogue_texts: Vec::new(),
            dialogue_cursor: 0,
            dialogue_hold_frames: 60,
            text_speed_step: crate::vm::text_speed_step_from_setting(3),
            dialogue_timer: 0,
            dialogue_scene_paths: Vec::new(),
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
    /// black bars above/below, full-screen clips copy 1:1 — the engine-level analogue
    /// of the game's `gs:[0x1fa7]` blit base.
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
    /// skipped. `sq/mind.hnm` is the complete boot reel (verified by decoding: frames
    /// ~0-30 MINDSCAPE logo, ~40-80 Microfolie's logo zoom, ~100-200 the
    /// ship-over-planet cutscene, tail the CRYO card) — matching the oracle-captured
    /// boot order exactly — followed by the fire "COMMANDER Blood" title.
    /// (`microfol.hnm` is a shorter variant of the same reel without MINDSCAPE;
    /// `inter_sh` is the ship interior, `cryogel`/`cryorad` cryo-chamber scenes,
    /// `logo01/02` the HATE-TV logo — none of them boot clips.)
    pub fn load_intro(&mut self, assets: &Path) {
        let sq = assets.join("sq");
        let order = [
            "mind",    // complete boot reel: MINDSCAPE + Microfolie's + ship + CRYO
            "logo_bl", // fire "COMMANDER Blood" title
        ];
        self.intro_hnms = order
            .iter()
            .map(|n| sq.join(format!("{n}.hnm")))
            .filter(|p| p.exists())
            .collect();
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

    /// Render one frame of the current intro clip full-screen; when a clip's frames are
    /// exhausted, advance to the next; when the sequence ends, deactivate the intro so
    /// the main loop takes over.
    fn render_intro_frame(&mut self) {
        let Some(hnm) = self.scene_hnm.take() else {
            self.intro_active = false;
            return;
        };
        let count = hnm.frame_count().max(1);
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
        hnm.decode_frame(self.scene_frame, &mut self.scene_buffer, &mut self.scene_palette);
        self.scene_hnm = Some(hnm);
        self.scene_frame += 1;
        self.present_scene_buffer();
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
        self.load_current_scene();
    }

    /// Load the talk-HNM resolved for the current dialogue line (if any).
    fn load_current_scene(&mut self) {
        if let Some(Some(path)) = self.dialogue_scene_paths.get(self.dialogue_cursor).cloned() {
            self.load_scene_hnm(&path);
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
        for tok in walk(cod, 0, cod.len()) {
            if let VmToken::Text {
                offset,
                word_offsets,
                ..
            } = tok
            {
                let parts: Vec<String> = word_offsets
                    .iter()
                    .filter_map(|o| words.get(o).cloned())
                    .collect();
                if !parts.is_empty() {
                    text_by_offset.insert(offset, assemble_words(&parts));
                }
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
        use crate::vm::{reveal_complete_hold_ticks, reveal_frames_per_char};
        let len = self
            .dialogue_texts
            .get(self.dialogue_cursor)
            .map(|t| t.chars().count() as u32)
            .unwrap_or(0);
        let step = self.text_speed_step;
        let reveal = len.saturating_mul(u32::from(reveal_frames_per_char(step)));
        let hold = u32::from(reveal_complete_hold_ticks(step));
        self.dialogue_hold_frames.max(reveal.saturating_add(hold))
    }

    fn advance_dialogue(&mut self) {
        if self.dialogue.is_empty() {
            return;
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
    fn render_nav_pyramid_sprites(&mut self) {
        use crate::ship3d::{
            SHIP_3D_ANGLE_TABLE, Ship3dMatrixAngles, build_ship_3d_projection_matrix,
            project_star_map_point,
        };
        let Some(m) = build_ship_3d_projection_matrix(
            &SHIP_3D_ANGLE_TABLE,
            Ship3dMatrixAngles {
                angle_2f71: 0,
                projection_angle_2f6d: 0,
                angle_2f6f: 10,
            },
        ) else {
            return;
        };
        let origin = [0i32, -700, 0];
        let pan = (self.compass_angle as i32 % 180 - 90) * 8;
        // Base pyramid dimension: the biggest CARTE pyramid frame (f4, 24px wide).
        let base_w = self.nav_pyramids[4].width.max(1) as u32;
        const ROW_Z: [i32; 4] = [600, 1500, 3000, 5600];
        for (zi, &z) in ROW_Z.iter().enumerate() {
            let _ = zi;
            for xi in -3..=3i32 {
                let d = [xi * 700 + pan, 0, z];
                let Some((sx, sy, scale)) = project_star_map_point(d, origin, &m) else {
                    continue;
                };
                if !(0..ENGINE_SCREEN_WIDTH as i32).contains(&sx)
                    || !(0..ENGINE_SCREEN_HEIGHT as i32).contains(&sy)
                {
                    continue;
                }
                // The sprite path's exact dimension scaling: `dim * depth_scale >> 10`
                // with `depth_scale = 0x100000/depth` (== `scale` here), then the
                // closest pre-scaled CARTE frame (f0..f5).
                let sw = ((base_w.saturating_mul(scale as u32 & 0xFFFF)) >> 10).max(2) as i32;
                let fi = (0..6)
                    .min_by_key(|&i| (self.nav_pyramids[i].width as i32 - sw).abs())
                    .unwrap_or(4);
                let frame = self.nav_pyramids[fi].clone();
                blit_sprite_frame_centered(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    &frame,
                    sx,
                    sy,
                );
            }
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
        // Display palette for the ship view: a grey ramp for the starfield depth
        // shades + the nav-grid face/orb indices (framebuffer is indexed).
        for (i, slot) in self.scene_palette.iter_mut().enumerate() {
            let g = (i.min(255)) as u8;
            *slot = [g, g, g];
        }
        self.scene_palette[90] = [96, 96, 104];
        self.scene_palette[200] = [176, 176, 184];
        self.scene_palette[240] = [232, 232, 240];
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
        draw_text_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &format!("SECTOR {sector}"),
            8,
            6,
            0xFE,
        );
    }

    /// Draw a subtitle line into the framebuffer at the game's subtitle reveal
    /// position (scene band, `SUBTITLE_X`/`SUBTITLE_Y` = 10/8) using the game font.
    /// The scene band's talk-HNM background composes separately; this is the text
    /// layer of the dialogue scene the engine presents for the current line.
    pub fn draw_subtitle(&mut self, text: &str, color: u8) {
        use crate::font::GAME_FONT_LINE_HEIGHT;
        // The subtitle string is pre-wrapped by the game's decoded text-assembly rule
        // (35-char wrap with 0x0D breaks — `assemble_words`); draw each line at the
        // subtitle origin, one font row apart, exactly as the game's `render_string`
        // renders the 0x0D-separated buffer.
        for (idx, line) in text.split('\n').enumerate() {
            draw_text_indexed(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                line,
                10,
                8 + idx * GAME_FONT_LINE_HEIGHT,
                color,
            );
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
        } else {
            for p in self.framebuffer.iter_mut() {
                *p = 0;
            }
        }
        // Subtitle text layer over the scene. Force the reserved subtitle index to
        // white so it's visible regardless of the scene palette (mirrors the game's
        // reserved high-palette subtitle colour).
        self.scene_palette[0xFD] = [245, 245, 245];
        if let Some(text) = self.current_subtitle().map(str::to_string) {
            self.draw_subtitle(&text, 0xFD);
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
    pub fn step(&mut self, input: MouseInput) {
        self.poll_input(input);
        // Startup intro videos play full-screen first (developer/publisher logos +
        // intro cutscene), exactly as the real game boots, before any nav/dialogue.
        if self.intro_active {
            self.render_intro_frame();
            self.frame += 1;
            return;
        }
        // On-ship gate ([0x2793] & 8): steer the compass from the mouse and render
        // the nav view's starfield background (the render subsystems the main loop
        // calls). Mouse x across the screen maps to the 0..179 compass rotation.
        if self.on_ship {
            self.compass_angle =
                ((self.mouse.x as u32 * 180) / ENGINE_SCREEN_WIDTH as u32).min(179) as u16;
            // Edge-triggered nav commit: a fresh left-click selects the destination at
            // the current heading (the nav→dialogue transition hook a driver acts on).
            let left = self.mouse.left_down();
            if left && !self.prev_left_down {
                self.nav_selection = Some(self.compass_angle);
            }
            self.prev_left_down = left;
            self.render_ship_view();
        } else if !self.dialogue.is_empty() {
            // Dialogue scene present: render the current line's frame (the
            // talk-HNM scene background composites behind this once the HNM decoder
            // is lib-side; for now the subtitle text layer over a cleared band).
            self.render_dialogue_frame();
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn intro_plays_startup_videos_then_ends() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.on_ship = true;
        e.load_intro(assets);
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
        e.step(MouseInput {
            x: 0,
            y: 100,
            buttons: 0,
        });
        let angle_left = e.compass_angle;
        let frame_left = e.framebuffer.clone();
        e.step(MouseInput {
            x: 319,
            y: 100,
            buttons: 0,
        });
        assert_eq!(angle_left, 0);
        assert!(e.compass_angle > 150, "mouse right steers the compass high");
        assert!(
            frame_left.iter().any(|&p| p != 0),
            "the starfield renders some points"
        );
        assert_ne!(
            frame_left, e.framebuffer,
            "different angle -> different view"
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
        // Text draws at y=8 (the subtitle band); pixels appear in that row range.
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
        e.step(MouseInput::default());
        let lit = e.framebuffer.iter().filter(|&&p| p == 0xFD).count();
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

    #[test]
    fn nav_click_commits_a_destination_selection() {
        let mut e = EngineState::new();
        e.on_ship = true;
        // move to a heading, no click yet -> no selection
        e.step(MouseInput { x: 200, y: 100, buttons: 0 });
        assert!(e.take_nav_selection().is_none());
        // click at a heading -> selection committed at that compass angle
        e.step(MouseInput { x: 200, y: 100, buttons: 1 });
        let sel = e.take_nav_selection();
        assert_eq!(sel, Some(e.compass_angle));
        // taken once, cleared
        assert!(e.take_nav_selection().is_none());
        // holding the button (no new edge) does not re-commit
        e.step(MouseInput { x: 200, y: 100, buttons: 1 });
        assert!(e.take_nav_selection().is_none());
    }

    #[test]
    fn subtitle_wraps_long_lines() {
        // Text assembly wraps with the game's decoded 0xA6 rule: a line break after
        // the space once the line reaches 0x23 (35) chars.
        let words: Vec<String> = "You can wake Cap'n Bob by clicking on the CRYO chamber control panel now"
            .split_whitespace()
            .map(str::to_string)
            .collect();
        let assembled = assemble_words(&words);
        assert!(assembled.contains('\n'), "long line wraps: {assembled:?}");
        for line in assembled.split('\n') {
            // 35 chars plus at most one unsplit word beyond the boundary.
            assert!(line.chars().count() <= 35 + 12, "line within wrap bound: {line:?}");
        }
        // And the drawer renders each wrapped line on its own font row.
        let mut e = EngineState::new();
        e.scene_palette[0xFD] = [255, 255, 255];
        e.draw_subtitle(&assembled, 0xFD);
        let w = ENGINE_SCREEN_WIDTH;
        let rows_with_text = (0..30)
            .filter(|&r| e.framebuffer[r * w..(r + 1) * w].iter().any(|&p| p == 0xFD))
            .count();
        assert!(rows_with_text > 8, "text occupies multiple wrapped rows (rows={rows_with_text})");
    }

    #[test]
    fn dialogue_hold_scales_with_line_length() {
        let mut e = EngineState::new();
        e.dialogue_hold_frames = 20;
        e.dialogue_texts = vec!["Hi".into(), "A rather long dialogue line that should linger longer".into()];
        e.dialogue = vec![
            LineState { offset: 0, actor_offset: None, location_offset: None },
            LineState { offset: 1, actor_offset: None, location_offset: None },
        ];
        e.dialogue_cursor = 0;
        let short = e.current_line_hold();
        e.dialogue_cursor = 1;
        let long = e.current_line_hold();
        assert!(long > short, "longer line held longer ({long} > {short})");
        assert!(short >= 20, "at least the base hold");
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
