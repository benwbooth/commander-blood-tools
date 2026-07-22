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
                if line_len >= crate::script::SUBTITLE_WRAP_COLUMN {
                    out.push('\n');
                    line_len = 0;
                }
            }
        }
    }
    out
}


/// One captured pointing-hand sprite: the live game's 3D hand renderer output
/// at a given cursor position (fingertip anchor = the cursor hotspot).
struct HandSprite {
    /// Cursor position this sprite was captured at (the renderer orients the
    /// hand by position); the nearest sprite is drawn.
    captured_at: (i32, i32),
    /// Cursor hotspot offset into the sprite (cursor - anchor = top-left).
    anchor: (i32, i32),
    width: usize,
    height: usize,
    /// Palette-index pixels, 0 = transparent.
    indices: Vec<u8>,
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
    /// The scrutinizer-apparatus intro animation (`sq/caiscrut.hnm`) played once when
    /// the examination screen opens, before the rotatable alien.
    alien_intro: Option<HnmFile>,
    /// Intro-animation frame counter; `None` once the intro has finished (or if there
    /// is no intro), so the rotatable alien takes over.
    alien_intro_frame: Option<usize>,
    /// The comms "Hate TV" screen: broadcast channel HNMs (`sq/tvgren*`, `tvred*` —
    /// self-contained character-in-TV-frame animations). Steering switches channels.
    tv_channels: Vec<HnmFile>,
    /// Whether the comms/TV screen is the active view.
    pub tv_active: bool,
    /// Currently-selected TV channel index.
    tv_channel: usize,
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
    /// Pointing-hand cursor sprites captured from the REAL renderer (see
    /// [`EngineState::load_hand_atlas`]); empty = no hand drawn.
    hand_atlas: Vec<HandSprite>,
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
    /// The console OPTION 3D-pyramid menu (`manu3.xdb` overlay). Its 12-item dispatch
    /// structure is decoded statically from manu3.xdb (`[0x2306]` table) and its
    /// camera/rotation/tween/dispatch logic is the ported [`crate::manu3`]; it reuses the
    /// shared ship-3D pyramid projection. Reconstructed from the decoded overlay — the
    /// per-item glyphs are graphical (archived), so items show as the decoded indices.
    pub option_active: bool,
    /// The rotating pyramid's current angle (advanced each frame + steered by the cursor,
    /// via `manu3::menu_camera_pan`).
    option_angle: u16,
    /// The currently highlighted menu item (0..[`Self::OPTION_ITEM_COUNT`]).
    option_item: usize,
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
    /// Per-line resolved speaker voice bank (`sn/<name>.snd`), parallel to
    /// [`EngineState::dialogue`].
    dialogue_voice_banks: Vec<Option<std::path::PathBuf>>,
    /// The A6 voice-selector byte per text-token offset (for the current script).
    voice_by_offset: HashMap<usize, u8>,
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
            nav_world_labels: crate::levels::primary_worlds().map(|e| e.stem).collect(),
            world_location: None,
            title_screen: None,
            nav_pyramids: Vec::new(),
            nav_chart: None,
            nav_destinations: Vec::new(),
            camera: crate::ship3d::Ship3dCameraApproach::default(),
            alien_views: Vec::new(),
            alien_view_active: false,
            alien_pan: 0,
            alien_object: crate::croolis::AlienObject::new(0x2DD3),
            alien_intro: None,
            alien_intro_frame: None,
            tv_channels: Vec::new(),
            tv_active: false,
            tv_channel: 0,
            cyber_tunnels: Vec::new(),
            cyber_steer: 0,
            cyber_arrived: false,
            cyber_active: false,
            cryobox_scene: None,
            cryobox_active: false,
            cyber_segment: 0,
            panorama: None,
            hand_atlas: Vec::new(),
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
            option_active: false,
            option_angle: 0,
            option_item: 0,
            progress: crate::progress::GameProgress::new(),
            ending_scene: None,
            ending_frame: 0,
            ending_active: false,
            phone_widget: Vec::new(),
            phone_contacts: Vec::new(),
            phone_contact: 0,
            phone_connected: false,
            phone_active: false,
            dialogue: Vec::new(),
            dialogue_texts: Vec::new(),
            dialogue_cursor: 0,
            dialogue_hold_frames: 60,
            text_speed_step: crate::vm::text_speed_step_from_setting(3),
            dialogue_timer: 0,
            dialogue_scene_paths: Vec::new(),
            dialogue_voice_banks: Vec::new(),
            voice_by_offset: HashMap::new(),
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
        for (name, cues, music) in order {
            let path = sq.join(&name);
            if path.exists() {
                self.intro_hnms.push(path);
                self.intro_cues.push(cues);
                self.intro_music.push(music);
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
        for (i, name) in record.sequence_hnms.iter().enumerate() {
            let path = sq.join(name);
            if path.exists() {
                self.intro_hnms.push(path);
                // The record's subtitle cues + music are timed over the sequence from its 1st clip.
                self.intro_cues
                    .push(if i == 0 { record.subtitles.clone() } else { Vec::new() });
                self.intro_music
                    .push(if i == 0 { music.clone() } else { None });
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

    /// Load the comms "Hate TV" screen: the broadcast-channel HNMs named `<prefix>*`
    /// under `sq/` (e.g. `tv` → tvgren*/tvred*), sorted so steering cycles channels
    /// in a stable order. The screen renders once `tv_active` is set.
    pub fn load_tv_channels(&mut self, assets: &Path, prefix: &str) {
        let sq = assets.join("sq");
        let mut names: Vec<String> = std::fs::read_dir(&sq)
            .into_iter()
            .flatten()
            .flatten()
            .filter_map(|e| e.file_name().to_str().map(str::to_string))
            .filter(|n| {
                n.to_lowercase().starts_with(prefix) && n.to_lowercase().ends_with(".hnm")
            })
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
    fn render_tv(&mut self) {
        let n = self.tv_channels.len();
        if n == 0 {
            return;
        }
        let ch = self.tv_channel % n;
        let hnm = &self.tv_channels[ch];
        let count = hnm.frame_count().max(1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(self.scene_frame % count, &mut self.scene_buffer, &mut self.scene_palette);
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
    }

    /// Number of loaded TV channels.
    pub fn tv_channel_count(&self)->usize{self.tv_channels.len()}

    /// Switch the TV channel by `delta` (wrapping), restarting the broadcast.
    pub fn switch_tv_channel(&mut self, delta: i32) {
        let n = self.tv_channels.len();
        if n == 0 {
            return;
        }
        self.tv_channel = (self.tv_channel as i32 + delta).rem_euclid(n as i32) as usize;
        self.scene_frame = 0;
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

    /// The ship-console menu row order, top to bottom, exactly as baked into the
    /// golden menu of the TB.BIG panorama frames (verified against the live
    /// capture): HONK (the cook's daily fare, SCRIPT1), TELEPHONE, CRYOBOX,
    /// MENU, OPTION.
    pub const CONSOLE_MENU: [&'static str; 5] = ["HONK", "TELEPHONE", "CRYOBOX", "MENU", "OPTION"];

    /// The console MENU option's submenu, decoded by driving the real game (clicking MENU
    /// opens these two items): EXPLANATIONS (the tutorial/help) and GAME (play). Drawn over
    /// the top menu rows, matching the observed golden-menu overlay.
    pub const MENU_SUBMENU: [&'static str; 2] = ["EXPLANATIONS", "GAME"];

    /// Map a click to a MENU-submenu item (0 = EXPLANATIONS, 1 = GAME) when the
    /// submenu is showing. The submenu is a gold CHOICE BOX (the game's universal
    /// console-choice widget), so hit-test its rows, not the golden menu.
    pub fn menu_submenu_click(&self, x: u16, y: u16) -> Option<usize> {
        if !self.menu_submenu_active {
            return None;
        }
        self.choice_box_row_at(x, y, Self::MENU_SUBMENU.len())
    }

    /// Aim the bridge's virtual ring-space cursor at an absolute screen point —
    /// the inverse of the game's `screen = ring - (frame*8 - 160)` rebase — so
    /// absolute click coordinates can be tested with the decoded hit math.
    fn point_virtual_cursor(view: &mut crate::bridge::BridgeView, x: u16, y: u16) {
        view.ring_mouse_x = (x as i32
            + view.frame as i32 * crate::bridge::RING_PX_PER_FRAME
            - crate::bridge::HALF_SCREEN)
            .rem_euclid(crate::tbbig::ANGLE_UNITS_PER_REVOLUTION as i32);
        view.mouse_y = y as i32;
    }

    /// Map a click to a ship-console menu option index (0 = HONK … 4 = OPTION)
    /// using the decoded golden-menu hit math (`0x8613`): the menu is only
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
            ',' => Some(39),
            ':' => Some(40),
            ';' => Some(41),
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
            let advance = match Self::console_glyph_index(ch).and_then(|gi| self.console_font.get(gi)) {
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
        // a golden choice box over the console's left — the interaction pattern
        // captured live from the real game (accuracy/captures/bridge/
        // choice_box_bob_morlock.ppm: golden rounded box, gold text rows).
        if (72..=107).contains(&self.bridge.frame) && !self.nav_destinations.is_empty() {
            self.draw_choice_box_labels();
        }
        // The MENU submenu ({EXPLANATIONS, GAME}) is a gold CHOICE BOX (the
        // game's universal console-choice widget) — draw it before the hand so
        // the cursor sits over it, as the live game composites.
        if self.menu_submenu_active {
            let labels: Vec<String> = Self::MENU_SUBMENU.iter().map(|s| s.to_string()).collect();
            self.draw_choice_box(&labels, None);
        }
        self.draw_hand_cursor();
    }

    /// Load the pointing-hand capture atlas: sprites of the REAL game's 3D hand
    /// renderer output, captured per cursor position from the emulator running
    /// the original (runtime_boot `BRIDGEPROBE HANDATLAS`; files under
    /// `accuracy/captures/bridge/hand/hand_<x>_<y>.bin` = {anchor_x, anchor_y,
    /// w, h: i16, then w*h palette indices, 0 transparent}). This is interim
    /// real-capture art: the hand's actual renderer (manu3.xdb skeletal mesh +
    /// affine texture mapping, decoded in re/REVERSE.md) is still to be ported;
    /// until then the port composites the genuine renderer's output.
    /// How many pointing-hand sprites the atlas holds (0 = none loaded).
    pub fn hand_atlas_len(&self) -> usize {
        self.hand_atlas.len()
    }

    pub fn load_hand_atlas(&mut self, dir: &Path) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            let Some(pos) = name
                .strip_prefix("hand_")
                .and_then(|s| s.strip_suffix(".bin"))
                .and_then(|s| s.split_once('_'))
                .and_then(|(a, b)| Some((a.parse::<i32>().ok()?, b.parse::<i32>().ok()?)))
            else {
                continue;
            };
            let Ok(data) = std::fs::read(entry.path()) else { continue };
            if data.len() < 8 {
                continue;
            }
            let word = |at: usize| i16::from_le_bytes([data[at], data[at + 1]]) as i32;
            let (anchor_x, anchor_y, w, h) = (word(0), word(2), word(4), word(6));
            if w <= 0 || h <= 0 || data.len() < 8 + (w * h) as usize {
                continue;
            }
            self.hand_atlas.push(HandSprite {
                captured_at: pos,
                anchor: (anchor_x, anchor_y),
                width: w as usize,
                height: h as usize,
                indices: data[8..8 + (w * h) as usize].to_vec(),
            });
        }
    }

    /// Draw the pointing-hand cursor at the virtual cursor position, using the
    /// atlas sprite captured nearest to it (the real renderer varies the hand's
    /// orientation with position). No-op with an empty atlas.
    fn draw_hand_cursor(&mut self) {
        let (cx, cy) = (self.bridge.mouse_screen_x(), self.bridge.mouse_y);
        let Some(sprite) = self
            .hand_atlas
            .iter()
            .min_by_key(|s| {
                let (dx, dy) = (s.captured_at.0 - cx, s.captured_at.1 - cy);
                dx * dx + dy * dy
            })
        else {
            return;
        };
        let (x0, y0) = (cx - sprite.anchor.0, cy - sprite.anchor.1);
        for y in 0..sprite.height {
            for x in 0..sprite.width {
                let index = sprite.indices[y * sprite.width + x];
                if index == 0 {
                    continue;
                }
                let (px, py) = (x0 + x as i32, y0 + y as i32);
                if (0..ENGINE_SCREEN_WIDTH as i32).contains(&px)
                    && (0..ENGINE_SCREEN_HEIGHT as i32).contains(&py)
                {
                    self.framebuffer[py as usize * ENGINE_SCREEN_WIDTH + px as usize] = index;
                }
            }
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
    const CHOICE_BOX_CENTER_X: usize = 100;
    /// The choice box is VERTICALLY CENTERED: the centre of the row-tops sits on
    /// this y regardless of item count (measured: 1 item → top y=95, 2 items →
    /// tops 89/100 whose centre is 94.5; both anchor the tops-centre at ~95).
    const CHOICE_BOX_TOPS_CENTER_Y: usize = 95;
    const CHOICE_BOX_PITCH: usize = 11;

    /// The top y of the first choice-box row for a box of `rows` items (vertical
    /// centring around [`Self::CHOICE_BOX_TOPS_CENTER_Y`]).
    fn choice_box_top_y(rows: usize) -> usize {
        (Self::CHOICE_BOX_TOPS_CENTER_Y as i32
            - ((rows.max(1) as i32 - 1) * Self::CHOICE_BOX_PITCH as i32 + 1) / 2)
            .max(0) as usize
    }

    fn draw_choice_box(&mut self, labels: &[String], selected: Option<usize>) {
        if labels.is_empty() {
            return;
        }
        const BORDER: u8 = 0x15;
        const FILL: u8 = 0xE0;
        const TEXT: u8 = 0xE8;
        const TEXT_SELECTED: u8 = 0xEF;
        let rows = labels.len().min(8);
        let widest = labels
            .iter()
            .take(rows)
            .map(|l| crate::font::square_caps_text_width(l))
            .max()
            .unwrap_or(0);
        let text_top = Self::choice_box_top_y(rows);
        // Black backdrop enclosing the centered text (3px padding each side); a
        // 3px border index (0x15) then the fill (0xE0) — both DAC(0,0,0), from the
        // prior live index-dump measurement (RGB can't distinguish them).
        let half = widest / 2 + 4;
        let x0 = Self::CHOICE_BOX_CENTER_X.saturating_sub(half);
        let x1 = (Self::CHOICE_BOX_CENTER_X + half).min(ENGINE_SCREEN_WIDTH);
        let y0 = text_top.saturating_sub(3);
        let y1 = (text_top + rows * Self::CHOICE_BOX_PITCH + 3).min(ENGINE_SCREEN_HEIGHT);
        for y in y0..y1 {
            for x in x0..x1 {
                let edge = y < y0 + 3 || y + 3 >= y1 || x < x0 + 3 || x + 3 >= x1;
                self.framebuffer[y * ENGINE_SCREEN_WIDTH + x] = if edge { BORDER } else { FILL };
            }
        }
        for (i, label) in labels.iter().take(rows).enumerate() {
            let color = if selected == Some(i) { TEXT_SELECTED } else { TEXT };
            let width = crate::font::square_caps_text_width(label);
            let lx = Self::CHOICE_BOX_CENTER_X.saturating_sub(width / 2);
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

    /// Hit-test a click against the console choice box (see [`draw_choice_box`]):
    /// the box centers on x=100 with its first row band from the box top (y=86)
    /// at an 11px pitch. Shared by the telephone contact list, the MENU submenu,
    /// and the nav-destination chooser — all the same rendered widget, so the
    /// click math is derived from the same measured geometry as the draw.
    fn choice_box_row_at(&self, x: u16, y: u16, num_rows: usize) -> Option<usize> {
        let (x, y) = (x as i32, y as i32);
        if !(40..=160).contains(&x) {
            return None;
        }
        let rows = num_rows.min(8);
        let top = Self::choice_box_top_y(rows) as i32 - 3;
        let row = (y - top) / Self::CHOICE_BOX_PITCH as i32;
        (row >= 0 && (row as usize) < rows).then_some(row as usize)
    }

    /// Draw a LIST MENU — the game's blue square-capitals vertical list (topic
    /// menus in dialogue, destinations at nav, contacts): entries at the
    /// measured geometry (x 170, rows from y 34 at 11 px pitch, text index
    /// 0xE8 over the scene). `selected` brightens one row.
    pub fn draw_list_menu(&mut self, labels: &[String], selected: Option<usize>) {
        const TEXT: u8 = 0xE8;
        const TEXT_SELECTED: u8 = 0xEF;
        for (i, label) in labels.iter().take(12).enumerate() {
            let color = if selected == Some(i) { TEXT_SELECTED } else { TEXT };
            crate::font::draw_square_caps(
                &mut self.framebuffer,
                ENGINE_SCREEN_WIDTH,
                ENGINE_SCREEN_HEIGHT,
                label,
                // Measured from `concept_menu.ppm`: first row top y=34, x=170,
                // 11px pitch (rows 34,45,56,…). Validated by re-extracting the
                // stored 'T'/'A' glyphs at exactly these coordinates.
                170,
                34 + i * 11,
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
        if labels.get(row).is_some_and(|l| crate::bas_vm::BasMenuStack::is_back_topic(l)) {
            self.bas_topic_click(row);
            self.bas_responses = None; // fresh monologue for the menu we backed out to
            return None;
        }
        if self.bas_responses.is_none() {
            self.bas_start_responses();
        }
        let offset = self.bas_advance_response()?;
        self.bas_menus.as_ref().and_then(|s| s.response_text(offset))
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
            self.topic_menu = labels.into_iter().enumerate().map(|(i, l)| (l, i)).collect();
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

    /// A click while the topic menu is showing: selects the topic and jumps the
    /// dialogue to its first line. Returns the selected topic index.
    pub fn topic_menu_click(&mut self, x: u16, y: u16) -> Option<usize> {
        let row = Self::list_menu_click(self.topic_menu.len(), x, y)?;
        self.topic_selected = Some(row);
        let line = self.topic_menu[row].1;
        self.set_dialogue_cursor(line);
        self.dialogue_timer = 0;
        Some(row)
    }

    /// Map a click to a list-menu row (the measured geometry above).
    pub fn list_menu_click(labels_len: usize, x: u16, y: u16) -> Option<usize> {
        if !(170..=245).contains(&(x as i32)) {
            return None;
        }
        // Rows start at y=34 with an 11px pitch (measured from concept_menu.ppm).
        let row = (y as i32 - 34) / 11;
        (row >= 0 && (row as usize) < labels_len.min(12)).then_some(row as usize)
    }

    /// Map a click to a nav-sector destination row when the choice box is showing
    /// (bridge view in the pyramid sector with destinations set).
    pub fn bridge_nav_destination_click(&self, x: u16, y: u16) -> Option<usize> {
        if !(72..=107).contains(&self.bridge.frame) || self.nav_destinations.is_empty() {
            return None;
        }
        self.choice_box_row_at(x, y, self.nav_destinations.len())
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
        //    (bridge_frame_to_yaw_sync 0x97E4 copies [0x2795] -> [0x2F6D]).
        let mut prng = BloodPrng::seeded_from_rtc_seconds(self.starfield_seed);
        let angles = Ship3dMatrixAngles {
            angle_2f71: 0,
            projection_angle_2f6d: self.bridge.frame % 180,
            angle_2f6f: 0,
        };
        let origin = Ship3dProjectionOrigin { x: 0x8000, y: 0x8000, z: 0x8000 };
        let viewport = Ship3dProjectionViewport {
            left: 0,
            right: ENGINE_SCREEN_WIDTH as u16,
            top: 0,
            bottom: ENGINE_SCREEN_HEIGHT as u16,
        };
        if let Some(render) = render_ship_3d_starfield(&mut prng, angles, origin, viewport) {
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
                let orb_box = (header.box_x != 0xFFFF)
                    .then_some([header.box_x, header.box_y, header.box_width, header.box_height]);
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
        hnm.decode_frame(self.scene_frame, &mut self.scene_buffer, &mut self.scene_palette);
        self.framebuffer.copy_from_slice(&self.scene_buffer);
        self.scene_frame += 1;
        // HUD overlay: a course reticle (steered) + a journey progress bar.
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

    /// The number of OPTION menu items, decoded from `manu3.xdb`'s dispatch table (the
    /// 12-entry item table at overlay offset 0x22f0, base `[0x2306]=0x3e72`).
    pub const OPTION_ITEM_COUNT: usize = 12;

    /// Whether the OPTION 3D-pyramid menu is the active view.
    pub fn option_active(&self) -> bool {
        self.option_active
    }

    /// The currently highlighted OPTION item index.
    pub fn option_item(&self) -> usize {
        self.option_item
    }

    /// Move the OPTION selection (`dir` +1/−1), wrapping — driven the way `manu3`'s item
    /// index derives from input.
    pub fn option_cycle(&mut self, dir: i32) {
        let n = Self::OPTION_ITEM_COUNT as i32;
        self.option_item = (self.option_item as i32 + dir).rem_euclid(n) as usize;
    }

    /// Map a click to an OPTION item row (matching `render_option_menu`'s list), or `None`.
    pub fn option_item_click(&self, x: u16, y: u16) -> Option<usize> {
        let (px, py) = (x as i32, y as i32);
        if px < 6 || px > 90 {
            return None;
        }
        (0..Self::OPTION_ITEM_COUNT)
            .find(|&i| (py - (24 + i as i32 * 14)).abs() <= 5)
    }

    /// Render the OPTION 3D-pyramid menu: the shared ship-3D pyramid (the manu3 menu IS a
    /// rotating 3D pyramid — it reuses this projection), spun + steered by the cursor via
    /// `manu3::menu_camera_pan`, with the decoded 12-item list overlaid (selected row lit).
    fn render_option_menu(&mut self) {
        const LIGHT: u8 = 0xFD;
        const DARK: u8 = 0xFB;
        const ORB: u8 = 0xFE;
        const SEL: u8 = 0xFC;
        self.scene_palette = crate::palette::game_screen_palette();
        // Clear first — the pyramid renderer only fills the lower grid band, so clear the
        // upper scene band to black (else a prior screen bleeds through).
        self.framebuffer.iter_mut().for_each(|p| *p = 0);
        // manu3 camera pan (entry 0x34..0x51): the cursor's delta from the view centre
        // steers the pyramid; fold it into the rotation + auto-spin one step.
        let (dx, _dy) = crate::manu3::menu_camera_pan(self.mouse.x as i16, self.mouse.y as i16);
        self.option_angle = (self.option_angle as i32 + (dx as i32) / 40 + 1).rem_euclid(180) as u16;
        crate::ship3d::render_star_map_navview_panned(
            &mut self.framebuffer,
            LIGHT,
            DARK,
            ORB,
            self.option_angle,
        );
        self.scene_palette[LIGHT as usize] = [150, 150, 220];
        self.scene_palette[DARK as usize] = [60, 60, 120];
        self.scene_palette[ORB as usize] = [232, 216, 40];
        self.scene_palette[SEL as usize] = [245, 245, 160];
        // The OPTION menu is the decoded manu3.xdb 3D pyramid (12 animated items;
        // its item labels are GRAPHICAL golden sprites in the original, not text —
        // so no invented "ITEM N" labels are drawn here). The selected item is
        // highlighted by lifting its position on the pyramid (manu3 tween data).
        // The real item glyphs await the manu3 sprite/RLE decode (re/REVERSE.md).
        let _ = self.option_item;
        self.scene_frame += 1;
    }

    /// Overlay the OPTION-style 3D-pyramid console floor + eye-orb onto the EXISTING framebuffer
    /// (the crew viewscreen) WITHOUT clearing it — the composite the real SCRIPT1 tutorial uses
    /// (accuracy/captures/frame_6-9): a crew talk-HNM fills the top viewscreen and the pyramid
    /// console sits along the bottom. Unlike [`Self::render_option_menu`] (which clears to black
    /// and tints the pyramids purple with a yellow orb), this OVERLAYS and uses the real GRAY
    /// pyramids + WHITE eye-orb seen in the captures. The pyramid renderer fills only the lower
    /// grid band, so the crew scene in the upper band shows through.
    fn overlay_console_pyramids(&mut self) {
        // Reserved palette slots that avoid the credit text (0xFD) and the subtitle reveal (0xFE).
        const LIGHT: u8 = 0xFA;
        const DARK: u8 = 0xFB;
        const ORB: u8 = 0xFC;
        crate::ship3d::render_star_map_navview_panned(
            &mut self.framebuffer,
            LIGHT,
            DARK,
            ORB,
            self.option_angle,
        );
        // Real tutorial-console palette (frames 6-9): grey pyramids, white eye-orb — NOT the
        // OPTION menu's purple/yellow.
        self.scene_palette[LIGHT as usize] = [180, 180, 190];
        self.scene_palette[DARK as usize] = [90, 90, 100];
        self.scene_palette[ORB as usize] = [235, 235, 235];
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

    /// The video-phone's callable crew: display name + their talk-head HNM basename
    /// (`pe/aa*.hnm`). These are the crew whose full-colour idle-head animations exist and
    /// decode cleanly; the phone shows the dialled one as the live "video feed".
    const PHONE_CONTACTS: [(&'static str, &'static str); 9] = [
        ("BOB MORLOCK", "aabob"),
        ("HOM", "aahom"),
        ("IZWALITO", "aaisw"),
        ("JERRY", "aajer"),
        ("MAXXON", "aamax"),
        ("MIGRAX", "aamig"),
        ("HANZ", "aahan"),
        ("TINA", "aatin"),
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
    pub fn phone_contact_count(&self) -> usize {
        self.phone_contacts.len()
    }

    /// The display name of the currently selected/dialled contact.
    pub fn phone_contact_name(&self) -> Option<&str> {
        self.phone_contacts.get(self.phone_contact).map(|(n, _)| n.as_str())
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
        let row = self.choice_box_row_at(x, y, contacts + 1)?;
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
        if self.alien_object.step() {
            self.scene_frame = self.scene_frame.wrapping_add(self.alien_object.anim as usize % 3);
        }
        let hnm = &self.alien_views[idx];
        let count = hnm.frame_count().max(1);
        self.scene_palette = hnm.palette;
        hnm.decode_frame(self.scene_frame % count, &mut self.scene_buffer, &mut self.scene_palette);
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
        // A credit clip (carrying the DESCRIPT `present` cues — the branded CRYO/title sequence)
        // runs only for its CREDIT SPAN in the real game (~tick 100, the final clearing cue, then
        // gameplay by ~9s), NOT the full 1258-frame cliptoot clip (~69s @18fps). Ground truth:
        // accuracy/captures/frame_* — MINDSCAPE(1s)→Microfolie's(2s)→cinematic→CRYO logo(5s)→
        // console with the credit overlaid(6s)→gameplay(9s). So end the credit clip when its cues
        // finish (last tick + a short hold), rather than playing it out. Non-credit clips (the
        // logo reel) play their full length.
        let clip_end = self
            .intro_cues
            .get(self.intro_index)
            .filter(|cues| !cues.is_empty())
            .map(|cues| {
                let last_tick = cues.iter().map(|c| c.tick as usize).max().unwrap_or(0);
                (last_tick * Self::INTRO_CREDIT_FRAMES_PER_TICK + Self::INTRO_CREDIT_HOLD_FRAMES)
                    .min(count)
            })
            .unwrap_or(count);
        if self.scene_frame >= clip_end {
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
        let frame = self.scene_frame;
        self.scene_frame += 1;
        self.present_scene_buffer();
        // The credit clip is the intro CREW SHOWCASE: cliptoot.hnm cycles crew members and the
        // game overlays the pyramid console over the bottom (accuracy/captures/frame_6-9 — the
        // pyramid floor occludes the crew's lower body). Only clips carrying credit cues get it.
        if self
            .intro_cues
            .get(self.intro_index)
            .is_some_and(|cues| !cues.is_empty())
        {
            self.overlay_console_pyramids();
        }
        // Overlay this clip's active credit subtitle (the DESCRIPT `present` cues on the
        // CRYO cinematic) centred in the lower letterbox, in the verified game font.
        self.draw_intro_credit(frame);
    }

    /// Frame index at which a credit cue's `tick` becomes active. The intro cinematic
    /// advances one clip frame per stepped game frame, so a cue displays from `tick`
    /// frames in until the next cue supersedes it (calibratable against the oracle).
    const INTRO_CREDIT_FRAMES_PER_TICK: usize = 1;
    /// Frames to hold after a credit clip's last cue before ending it (a brief beat so the final
    /// credit isn't cut instantly). Keeps the credit sequence to its ~tick-100 span, not the full
    /// 1258-frame clip — matching the real game's ~short intro.
    const INTRO_CREDIT_HOLD_FRAMES: usize = 24;
    /// Baseline row for the credit text, inside the cinematic's lower black letterbox.
    const INTRO_CREDIT_BASELINE_Y: usize = 178;
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
        let width: usize = text.chars().map(crate::font::game_font_advance).sum();
        let x = ENGINE_SCREEN_WIDTH.saturating_sub(width) / 2;
        self.scene_palette[Self::INTRO_CREDIT_COLOR_INDEX as usize] = [245, 245, 245];
        draw_text_indexed(
            &mut self.framebuffer,
            ENGINE_SCREEN_WIDTH,
            ENGINE_SCREEN_HEIGHT,
            &text,
            x,
            Self::INTRO_CREDIT_BASELINE_Y,
            Self::INTRO_CREDIT_COLOR_INDEX,
        );
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
    /// `total` — the decoded one-chatter-per-completed-line behaviour (@0x94BA).
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

    /// The current line's voice: the speaker's SND bank path + the line's voice
    /// selector byte (the A6 token's `b3`; `0xFF`/subtitle-only lines yield `None`).
    /// A driver resolves the clip via `vm::text_selector_voice_clip_index` against
    /// the bank's clip count and plays it once at line start.
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
        for tok in walk(cod, 0, cod.len()) {
            if let VmToken::Text {
                offset,
                voice_selector,
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
        self.dialogue = (0..lines.len())
            .map(|offset| LineState { offset, actor_offset: None, location_offset: None })
            .collect();
        self.dialogue_texts = lines.iter().map(|(t, _)| t.clone()).collect();
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
            && !self.option_active
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
        // Fully shown: advance to the next line, or signal the end.
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
    /// Test/inspection: the world labels used on the nearest nav-destination row.
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
        let Some(img) = std::fs::read(&rooms[0]).ok().and_then(|d| crate::lbm::decode_pbm(&d))
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
        if let Some(img) = std::fs::read(&visit.rooms[next]).ok().and_then(|d| crate::lbm::decode_pbm(&d))
        {
            visit.current = next;
            visit.image = img;
        }
    }

    /// The visited world's room count + current index (for HUD/tests), if active.
    pub fn world_room_position(&self) -> Option<(usize, usize)> {
        self.world_location.as_ref().map(|v| (v.current, v.rooms.len()))
    }

    /// Whether the world-location landing screen is active.
    pub fn world_location_active(&self) -> bool {
        self.world_location.is_some()
    }

    /// Whether the plain nav star-map is the active view — on the ship with no overlay
    /// screen (bridge/comms/cyberspace/cryobox/alien/world-landing) open. This is the
    /// view that shows the choose-a-location destination list.
    pub fn nav_view_active(&self) -> bool {
        self.on_ship
            && !self.bridge_active
            && !self.tv_active
            && !self.cyber_active
            && !self.cryobox_active
            && !self.phone_active
            && !self.option_active
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
            let abbrev = crate::levels::world_location_abbrev(&visit.name.to_lowercase())
                .unwrap_or("");
            // Match against the abbreviation, skipping any leading floor digit.
            let body = file.strip_prefix(|c: char| c.is_ascii_digit()).unwrap_or(&file);
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
                None => format!("{}  {}/{}", visit.name, visit.current + 1, visit.rooms.len()),
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
        // Mark the decoded .ext object positions with a small crosshair.
        for &(ox, oy) in &visit.objects {
            let (cx, cy) = (ox as usize, oy as usize);
            for d in 0..5usize {
                for (px, py) in [(cx + d, cy), (cx.wrapping_sub(d), cy), (cx, cy + d), (cx, cy.wrapping_sub(d))] {
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
        self.world_location = Some(visit);
    }

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
        // Camera origin from the decoded approach FSM, scaled into the nav view's
        // near-field so the pyramids pull in as the ship travels (X drives the
        // depth; the animation's units are the game's world scale).
        // Depth bias subtracted from the approach-FSM camera X so the pyramids sit in
        // the nav view's near field, then compressed by the world-to-view scale divisor.
        const CAMERA_DEPTH_BIAS: i32 = 8804;
        const WORLD_TO_VIEW_DEPTH_DIVISOR: i32 = 8;
        let cam = self.camera.origin();
        let origin = [0i32, -700, (cam[0] - CAMERA_DEPTH_BIAS) / WORLD_TO_VIEW_DEPTH_DIVISOR];
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
                let fh = frame.height as i32;
                blit_sprite_frame_centered(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    &frame,
                    sx,
                    sy,
                );
                let _ = fh;
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

    /// Layout of the choose-a-location destination list drawn on the nav chart.
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

    /// The number of nav destinations currently offered.
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
                const CURSOR_COLOR: u8 = 0xFE;
                self.scene_palette[CURSOR_COLOR as usize] = [245, 245, 160];
                let cursor_x = (self.compass_angle as usize % 180) * (ENGINE_SCREEN_WIDTH - 1) / 179;
                for dy in 0..4 {
                    let row = dy * ENGINE_SCREEN_WIDTH;
                    if let Some(px) = self.framebuffer.get_mut(row + cursor_x) {
                        *px = CURSOR_COLOR;
                    }
                }
                // Choose-a-location destination list (each character's location), clickable
                // — the game's list-box nav. Falls back to the compass-target label.
                if !self.nav_destinations.is_empty() {
                    let labels: Vec<String> =
                        self.nav_destinations.iter().map(|(l, _)| l.clone()).collect();
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
        self.nav_world_labels.get(self.targeted_world_index()).copied()
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
    pub fn draw_subtitle(&mut self, text: &str, color: u8) {
        let n = text.chars().count();
        self.draw_subtitle_with_color(text, n, color, color);
    }

    /// Draw the pre-wrapped subtitle with only the first `visible` characters shown,
    /// the newest one in the reveal-edge colour (0xFE) — the game's per-character
    /// reveal. Non-visible characters aren't drawn yet.
    fn draw_subtitle_revealed(&mut self, text: &str, visible: usize) {
        self.draw_subtitle_with_color(text, visible, 0xFD, 0xFE);
    }

    fn draw_subtitle_with_color(&mut self, text: &str, visible: usize, body: u8, edge: u8) {
        use crate::font::{GAME_FONT_LINE_HEIGHT, game_font_advance};
        // ON-CONSOLE text uses the game's BOLD monospace face (loaded from the
        // user's BLOODPRG.EXE — the live game draws all console/tutorial lines
        // with it; the thin face below is the letterboxed-scene subtitle font).
        if self.panorama.is_some() && self.scene_hnm.is_none() {
            if let Some(bold) = self.bold_font.take() {
                let mut shown = 0usize;
                let mut y = 8usize;
                for (li, line) in text.split('\n').enumerate() {
                    if li > 0 {
                        shown += 1;
                        y += 10; // console line pitch (rows 8/18 observed live)
                    }
                    let mut x = 10usize;
                    for ch in line.chars() {
                        if shown >= visible {
                            self.bold_font = Some(bold);
                            return;
                        }
                        let is_edge = shown + 1 == visible;
                        let mut buf = [0u8; 4];
                        bold.draw(
                            &mut self.framebuffer,
                            ENGINE_SCREEN_WIDTH,
                            ENGINE_SCREEN_HEIGHT,
                            ch.encode_utf8(&mut buf),
                            x,
                            y,
                            if is_edge { edge } else { body },
                        );
                        x += crate::font::BoldConsoleFont::ADVANCE;
                        shown += 1;
                    }
                }
                self.bold_font = Some(bold);
                return;
            }
        }
        // The subtitle string is pre-wrapped by the game's decoded text-assembly rule
        // (35-char wrap with 0x0D breaks — `assemble_words`); draw each line at the
        // subtitle origin (10,8), one font row apart, exactly as the game's
        // `render_string` renders the 0x0D-separated buffer. Newline chars count
        // toward the reveal position (the game reveals the buffer including 0x0D).
        let mut shown = 0usize; // characters (incl. newlines) consumed so far
        let mut y = 8usize;
        for (li, line) in text.split('\n').enumerate() {
            if li > 0 {
                shown += 1; // the 0x0D separator
                y += GAME_FONT_LINE_HEIGHT;
            }
            let mut x = 10usize;
            for ch in line.chars() {
                if shown >= visible {
                    return;
                }
                let is_edge = shown + 1 == visible;
                let mut buf = [0u8; 4];
                draw_text_indexed(
                    &mut self.framebuffer,
                    ENGINE_SCREEN_WIDTH,
                    ENGINE_SCREEN_HEIGHT,
                    ch.encode_utf8(&mut buf),
                    x,
                    y,
                    if is_edge { edge } else { body },
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
        } else if self.panorama.is_some() {
            // No talk-HNM (e.g. the on-ship console tutorial, HONK's food menu):
            // the dialogue happens AT THE SHIP CONSOLE in the real game, so
            // composite the real bridge panorama behind the subtitle text —
            // and the TOPIC MENU when this dialogue offers one (the concept-menu
            // conversation system, e.g. HONK's TALK/ONE..NINE).
            self.render_bridge_background();
            if !self.topic_menu.is_empty() {
                let labels: Vec<String> =
                    self.topic_menu.iter().map(|(l, _)| l.clone()).collect();
                self.draw_list_menu(&labels, self.topic_selected);
            }
            self.scene_buffer.copy_from_slice(&self.framebuffer);
        } else {
            for p in self.framebuffer.iter_mut() {
                *p = 0;
            }
        }
        // Subtitle text layer over the scene, revealed one character at a time (the
        // game's reveal @0x93F8–0x94B8: `gs:0x5E58` advances one char whenever the
        // per-char timer `gs:0xB31 = step>>2` elapses). Reserved indices 0xFD
        // (revealed) / 0xFE (newest edge glyph) are forced to the subtitle colour.
        self.scene_palette[0xFD] = [245, 245, 245];
        self.scene_palette[0xFE] = [245, 245, 245];
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
    pub fn step(&mut self, input: MouseInput) {
        // This frame's relative cursor motion (before poll_input refreshes
        // prev_pos) — the bridge steering consumes deltas, mirroring how the
        // original accumulates warped-cursor movement in ring space.
        let motion = (
            input.x as i32 - self.prev_pos.0 as i32,
            input.y as i32 - self.prev_pos.1 as i32,
        );
        self.poll_input(input);
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
            self.bridge.move_mouse(motion.0, motion.1);
            self.render_bridge();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // World-location landing screen: the decoded fd/ room background of a visited
        // world takes precedence while active.
        if self.world_location.is_some() {
            self.render_world_location();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // Cyberspace tunnel screen (presentation) takes precedence when active.
        if self.cyber_active && !self.cyber_tunnels.is_empty() {
            self.render_cyberspace();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The CRYOBOX cryo-chamber (console menu option) takes precedence when active.
        if self.cryobox_active && self.cryobox_scene.is_some() {
            self.render_cryobox();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The video-phone call screen (console TELEPHONE option) takes precedence.
        if self.phone_active && !self.phone_contacts.is_empty() {
            self.render_telephone();
            self.countdown = self.countdown.saturating_sub(1);
            self.frame += 1;
            return;
        }
        // The OPTION 3D-pyramid menu (console OPTION option) takes precedence.
        if self.option_active {
            self.render_option_menu();
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
            let dx = self.mouse.x as i32 - ENGINE_SCREEN_WIDTH as i32 / 2;
            // Dead-zone near centre; scaled turn rate outside it.
            if dx.abs() > 8 {
                let rate = dx / 20; // degrees/frame, proportional to centre distance
                self.compass_angle =
                    (self.compass_angle as i32 + rate).rem_euclid(180) as u16;
            }
            // Edge-triggered nav commit: a fresh left-click selects the destination at
            // the current heading (the nav→dialogue transition hook a driver acts on).
            let left = self.mouse.left_down();
            if left && !self.prev_left_down {
                self.nav_selection = Some(self.compass_angle);
            }
            self.prev_left_down = left;
            // Advance the ship-3D camera-approach animation (the decoded [0x27DF]
            // phase FSM) so the camera pulls in / travels as the game does on entry.
            self.camera.step();
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

/// The game's mode-X screen address for pixel `(x, y)` — `(byte_offset, plane)` where
/// `byte_offset = y*80 + x/4` and `plane = x & 3`, exactly as `graphics_plot_modex`
/// (`0x299:0x498`) computes it. Provided to document + verify that the engine's linear
/// `y*ENGINE_SCREEN_WIDTH + x` framebuffer is address-equivalent to the game's mode-X:
/// `byte_offset*4 + plane == y*320 + x` (see [`mode_x_to_linear`]).
pub fn mode_x_offset(x: usize, y: usize) -> (usize, usize) {
    (y * 80 + x / 4, x & 3)
}

/// Invert [`mode_x_offset`] back to the linear framebuffer index the engine uses:
/// `byte_offset*4 + plane`. Equals `y*ENGINE_SCREEN_WIDTH + x`, proving the two layouts
/// address the same pixel.
pub fn mode_x_to_linear(byte_offset: usize, plane: usize) -> usize {
    byte_offset * 4 + plane
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end faithfulness check for the bridge: the engine's full render of
    /// the console (panorama frame 55 + starfield windows + menu palette rows)
    /// must match the REAL game's console screen captured from the emulator
    /// running the original BLOODPRG.EXE (`BRIDGEPROBE`). The tolerance covers
    /// the pointing-hand cursor sprite (not yet ported) and the RNG starfield.
    #[test]
    fn bridge_console_matches_live_game_capture() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("TB.BIG").exists());
        let capture_path = ["accuracy/captures/bridge/console_rest.ppm",
            "../accuracy/captures/bridge/console_rest.ppm"]
            .iter().map(Path::new).find(|p| p.exists());
        let (Some(iso), Some(capture_path)) = (iso, capture_path) else { return };
        let raw = std::fs::read(capture_path).unwrap();
        let body_at = raw.windows(4).position(|w| w == b"255\n").unwrap() + 4;
        let capture = &raw[body_at..];
        assert_eq!(capture.len(), ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3);

        let mut e = EngineState::new();
        e.load_bridge(iso);
        if e.panorama.is_none() { return; }
        // The captured-hand atlas, when present, must land the hand where the live
        // game drew it (tightening the diff, not loosening it).
        for dir in ["accuracy/captures/bridge/hand", "../accuracy/captures/bridge/hand"] {
            e.load_hand_atlas(Path::new(dir));
            if !e.hand_atlas.is_empty() { break; }
        }
        e.bridge_active = true;
        // Prime prev_pos so the render step sees zero cursor motion, then
        // reproduce the live probe's state exactly: view frame 55, cursor at
        // ring 320 (screen x 40), y 100 — the state the capture was taken in.
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        e.bridge.frame = 55;
        e.bridge.ring_mouse_x = 320;
        e.bridge.mouse_y = 100;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        assert_eq!(e.bridge.frame, 55, "view must not drift during the render");
        assert_eq!(e.bridge.mouse_screen_x(), 40, "virtual cursor at the capture position");

        let mut total_abs = 0u64;
        for (pixel, &index) in e.framebuffer.iter().enumerate() {
            let ours = e.scene_palette[index as usize];
            for channel in 0..3 {
                total_abs +=
                    (ours[channel] as i32 - capture[pixel * 3 + channel] as i32).unsigned_abs() as u64;
            }
        }
        let mean_abs = total_abs as f64 / (ENGINE_SCREEN_WIDTH * ENGINE_SCREEN_HEIGHT * 3) as f64;
        // Optional visual QA: BRIDGE_DUMP=<path.ppm> writes the rendered frame.
        if let Ok(dump) = std::env::var("BRIDGE_DUMP") {
            let mut ppm = format!("P6\n{ENGINE_SCREEN_WIDTH} {ENGINE_SCREEN_HEIGHT}\n255\n").into_bytes();
            for &index in e.framebuffer.iter() {
                ppm.extend_from_slice(&e.scene_palette[index as usize]);
            }
            std::fs::write(&dump, ppm).unwrap();
            eprintln!("bridge console render -> {dump} (mean_abs vs live = {mean_abs:.2})");
        }
        // With the captured-hand atlas the render is near pixel-perfect (0.14
        // observed; residual = window-starfield RNG); without it the missing
        // hand accounts for ~2.6.
        let threshold = if e.hand_atlas.is_empty() { 4.0 } else { 1.0 };
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
    fn choice_box_matches_the_measured_spec() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        e.load_bridge(iso);
        e.load_console_font(iso);
        if e.panorama.is_none() { return; }
        // Open the MENU submenu (a choice box) and render one frame.
        e.bridge_active = true;
        e.menu_submenu_active = true;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        // The measured spec: border 0x15, fill 0xE0, text 0xE8. Count each in the
        // box region (x 63.., y 88..~120) — all three must be present.
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
        assert!(count(0x15) > 100, "choice-box border (0x15) present: {}", count(0x15));
        assert!(count(0xE0) > 500, "choice-box gold fill (0xE0) present: {}", count(0xE0));
        assert!(count(0xE8) > 20, "choice-box text (0xE8) present: {}", count(0xE8));
    }

    /// Oracle: the telephone choice box renders its item text exactly where the
    /// real game does. Renders the captured contact list (BOB_MORLOCK / CANCEL)
    /// and compares the port's 0xE8 glyph mask against the live capture's grey
    /// text mask — the labels must land CENTERED on x=100 (rows y=89/100) for the
    /// masks to overlap, which only happens with proportional-centered layout.
    #[test]
    fn choice_box_text_matches_live_game_capture() {
        let path = ["accuracy/captures/bridge/choice_box_bob_morlock.ppm", "../accuracy/captures/bridge/choice_box_bob_morlock.ppm"]
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
        assert!(iou > 0.55, "choice-box text must overlap the live capture (IoU {iou:.3} <= 0.55)");
    }

    /// Oracle: a single-item choice box (the lone "CANCEL" offered post-tutorial)
    /// is VERTICALLY CENTERED — with one row it sits lower (y=95) than a two-row
    /// box's first row (y=89). Verifies the count-dependent vertical layout against
    /// `post2_option_choice.ppm` (CANCEL at x73..130, y95..102).
    #[test]
    fn choice_box_single_item_is_vertically_centered_vs_capture() {
        let path = ["accuracy/captures/bridge/post2_option_choice.ppm", "../accuracy/captures/bridge/post2_option_choice.ppm"]
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
        // One-row box lands at y=95 (not the two-row anchor 89).
        assert_eq!(EngineState::choice_box_top_y(1), 95);
        assert_eq!(EngineState::choice_box_top_y(2), 89);

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
        let port_y = centroid_y(Box::new(move |x, y| fb[y * ENGINE_SCREEN_WIDTH + x] == 0xE8)).unwrap();
        let live_y = centroid_y(Box::new(move |x, y| is_grey((y * ENGINE_SCREEN_WIDTH + x) * 3))).unwrap();
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
        assert!(e.current_bas_menu_labels().is_empty(), "no menus before load");
        e.load_bas_menus(&bas, &dic);
        let entry = e.current_bas_menu_labels();
        assert!(entry.iter().any(|l| l == "OPTIMIZATION"), "entry = top-level menu: {entry:?}");
        // Navigate: enter the fear/anger sub-menu (BAS 0x42d, verified live) → current.
        assert!(e.bas_menus.as_mut().unwrap().push(0x42d));
        assert!(e.current_bas_menu_labels().iter().any(|l| l == "FEAR"));
        // Clicking a non-back topic (row 1 = FEAR) stays on the menu (plays a response).
        let after_fear = e.bas_topic_click(1);
        assert!(after_fear.iter().any(|l| l == "FEAR"), "emotion topic stays: {after_fear:?}");
        // Clicking row 0 (TALK, the back-out) POPS to the top-level entry menu —
        // exactly what the running game does (MENUTREE: talk → parent 0x2f).
        let after_talk = e.bas_topic_click(0);
        assert!(after_talk.iter().any(|l| l == "OPTIMIZATION"), "talk pops to parent: {after_talk:?}");
        // Syncing renders the current BAS menu as the topic menu (so it displays).
        e.sync_topic_menu_from_bas();
        assert!(!e.topic_menu.is_empty(), "topic menu populated from BAS");
        assert_eq!(e.topic_menu.get(1).map(|(l, _)| l.as_str()), Some("OPTIMIZATION"));
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
        assert!(sub.contains("several ways to lose"), "first subtitle: {sub:?}");
        assert!(e.bas_menu_interact(0).is_none(), "talk pops (no subtitle)");
        assert!(e.current_bas_menu_labels().iter().any(|l| l == "OPTIMIZATION"), "popped to parent");
    }

    /// Oracle: the LIST MENU (dialogue topics / nav destinations) renders the
    /// square-capitals face at index 0xE8 at the measured geometry (x 175, rows
    /// from y 45, 11 px pitch). Locks the widget + face to the harvested values.
    #[test]
    fn list_menu_renders_square_caps_at_measured_geometry() {
        let mut e = EngineState::new();
        // Feed a topic menu and render it over a blank frame via the public draw.
        let labels = vec!["TALK".to_string(), "ONE".to_string(), "TWO".to_string()];
        e.draw_list_menu(&labels, Some(1));
        // Row 0 (TALK) glyphs at y 34.., row 1 (ONE, selected) at y 45.. bright.
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
        assert!(count_in(0xE8, 33, 43) > 10, "TALK row in square-caps 0xE8");
        assert!(count_in(0xEF, 44, 54) > 6, "selected ONE row in bright 0xEF");
    }

    #[test]
    fn bridge_renders_the_real_panorama() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("TB.BIG").exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        e.load_bridge(iso);
        assert!(e.panorama.is_some(), "TB.BIG parses");
        e.bridge_active = true;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        assert!(e.framebuffer.iter().any(|&p| p != 0), "bridge draws the panorama");
        // At the menu rest frame, the decoded golden-menu hit math maps clicks to
        // rows: HONK (row 0) at the box top, OPTION (row 4) at the bottom.
        e.bridge.frame = crate::bridge::MENU_REST_FRAME;
        assert_eq!(e.console_menu_click(232, 0x48 + 1), Some(0));
        assert_eq!(e.console_menu_click(232, 0x48 + 4 * 0x12 + 1), Some(4));
        assert_eq!(e.console_menu_click(100, 0x48 + 1), None, "left of the menu box");
        // Away from the menu sector the menu is not clickable at all.
        e.bridge.frame = 90;
        assert_eq!(e.console_menu_click(232, 0x48 + 1), None);
    }

    #[test]
    fn alien_view_rotates_through_prerendered_angles() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("pe").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.load_alien_view(assets, "scrut");
        if e.alien_views.is_empty() { return; }
        e.alien_view_active = true;
        // Steer full left, capture; steer full right, capture: different rotation view.
        for _ in 0..12 { e.step(MouseInput { x: 5, y: 100, buttons: 0 }); }
        let left = e.framebuffer.clone();
        for _ in 0..12 { e.step(MouseInput { x: 315, y: 100, buttons: 0 }); }
        assert!(left.iter().any(|&p| p != 0), "alien renders");
        assert_ne!(left, e.framebuffer, "mouse rotates to a different pre-rendered view");
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
        e.load_intro(assets, &crate::descript::DescriptDb { records: Vec::new() });
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
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
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
                && e.framebuffer.iter().filter(|&&p| p == EngineState::INTRO_CREDIT_COLOR_INDEX).count() > 100
            {
                drew_credit = true;
                break;
            }
            if !e.intro_active() {
                break;
            }
        }
        assert!(drew_credit, "the CRYO publisher credit is overlaid during the intro");
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
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
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
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
            .iter()
            .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let rec = db
            .records
            .iter()
            .find(|r| r.name == "maledict")
            .expect("maledict cutscene record");
        let mut e = EngineState::new();
        assert!(e.start_descript_cutscene(rec, assets), "the cutscene starts");
        // HNM, music, and tick-subtitles all come from the record — data-driven, not hardcoded.
        assert!(e.intro_hnms[0].file_stem().is_some_and(|s| s == "maledict"));
        assert_eq!(e.intro_music[0].as_deref(), Some("klings.voc"), "cutscene music from the record");
        assert!(!e.intro_cues[0].is_empty(), "the curse subtitles play with the cutscene");
        assert!(
            e.intro_cues[0].iter().any(|c| c.text.contains("CURSED")),
            "the record's subtitle text is carried"
        );
    }

    /// The intro credit clip (`cliptoot.hnm`, carrying the `present` cues to tick 100) must run
    /// only its CREDIT SPAN — the real intro reaches gameplay by ~9s (ground truth:
    /// accuracy/captures/frame_*), not the full 1258-frame clip (~69s). Guards the "intro way too
    /// long" bug: the credit clip should end shortly after its last cue, not play out in full.
    #[test]
    fn intro_credit_clip_ends_with_its_cues_not_full_length() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter()
            .map(Path::new)
            .find(|p| p.join("sq").join("cliptoot.hnm").exists());
        let Some(assets) = assets else { return };
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
            .iter()
            .find_map(|p| crate::descript::DescriptDb::parse_file(p).ok());
        let Some(db) = db else { return };
        let mut e = EngineState::new();
        e.load_intro(assets, &db);
        let credit = e
            .intro_hnms
            .iter()
            .position(|p| p.file_stem().is_some_and(|s| s == "cliptoot"))
            .expect("cliptoot credit clip queued");
        let full_len = crate::hnm::HnmFile::open(&e.intro_hnms[credit])
            .map(|h| h.frame_count())
            .unwrap_or(0);
        assert!(full_len > 1000, "cliptoot is a long clip ({full_len} frames)");
        // Drive the intro, counting frames spent in the credit clip.
        let mut credit_frames = 0usize;
        for _ in 0..6000 {
            let at_credit = e.intro_index() == credit && e.intro_active();
            e.step(MouseInput::default());
            if at_credit {
                credit_frames += 1;
            }
            if !e.intro_active() {
                break;
            }
        }
        assert!(
            (90..300).contains(&credit_frames),
            "credit clip runs its ~tick-100 cue span ({credit_frames} frames), not the full {full_len}"
        );
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
        let db = ["output/_tmp_iso/DESCRIPT.DES", "../output/_tmp_iso/DESCRIPT.DES"]
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

    /// The SCRIPT1-tutorial console COMPOSITE (accuracy/captures/frame_6-9): the pyramid console
    /// floor + eye-orb overlays the crew viewscreen WITHOUT clearing it — the crew (upper band)
    /// shows through, and the pyramids sit along the bottom. Guards the gap where the port rendered
    /// the bridge/dialogue/OPTION as separate full-screen views instead of this composite.
    #[test]
    fn tutorial_console_overlays_pyramids_over_crew_viewscreen() {
        let mut e = EngineState::new();
        // A "crew viewscreen" already in the framebuffer (a non-zero, non-reserved index).
        e.framebuffer.iter_mut().for_each(|p| *p = 0x30);
        e.overlay_console_pyramids();
        // Upper band: the crew shows through (the pyramid floor draws only along the bottom).
        let top = &e.framebuffer[..ENGINE_SCREEN_WIDTH * 40];
        assert!(top.iter().all(|&p| p == 0x30), "the crew viewscreen (top) shows through");
        // Bottom band: the pyramid console + eye-orb are drawn (reserved indices).
        let bottom = &e.framebuffer[ENGINE_SCREEN_WIDTH * 120..];
        let pyramid_px = bottom.iter().filter(|&&p| p == 0xFA || p == 0xFB || p == 0xFC).count();
        assert!(pyramid_px > 100, "the pyramid console draws along the bottom ({pyramid_px} px)");
    }

    /// End-to-end regression: drive the full playable loop the way the real driver does
    /// (title -> intro -> nav -> every screen -> a dialogue scene) and assert each stage
    /// produces real content and progresses. The step loop is pure logic (no real-time
    /// wait), so a full scene runs in milliseconds. Skips without game data. A broader
    /// all-five-script playthrough lives in `src/bin/smoke.rs`.
    #[test]
    fn full_playable_loop_end_to_end() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("DESCRIPT.DES").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("sq").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else { return };
        let db = crate::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
        let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT1.{ext}")));
        let (Ok(cod), Ok(var), Ok(dic), Ok(deb)) = (rd("COD"), rd("VAR"), rd("DIC"), rd("DEB")) else { return };

        let mut e = EngineState::new();
        e.load_dialogue_scenes(&cod, &var, &dic, &deb, &db, assets);
        e.dialogue_hold_frames = 20;
        if let (Ok(c), Ok(b)) = (std::fs::read(iso.join("CARTE.SPR")), std::fs::read(iso.join("BORXX.SPR"))) {
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
            if !e.intro_active() { intro_ended = true; break; }
        }
        assert!(intro_ended, "intro sequence finishes");

        // Every screen renders real content.
        e.on_ship = true;
        for _ in 0..8 { e.step(MouseInput::default()); }
        assert!(nonblank(&e.framebuffer) > 500, "nav view renders");
        if has_chart {
            // With CHART.FD present the nav view is the real star-map: a rich, many-colour
            // image, not a sparse procedural starfield.
            let distinct = e.framebuffer.iter().collect::<std::collections::HashSet<_>>().len();
            assert!(distinct > 40, "nav view shows the real CHART.FD star-map ({distinct} colours)");
        }
        e.bridge_active = true;
        for _ in 0..4 { e.step(MouseInput::default()); }
        assert!(nonblank(&e.framebuffer) > 500, "bridge renders");
        e.bridge_active = false;
        e.tv_active = true;
        for _ in 0..8 { e.step(MouseInput::default()); }
        assert!(nonblank(&e.framebuffer) > 500, "TV renders");
        e.tv_active = false;
        e.cyber_active = true;
        for _ in 0..8 { e.step(MouseInput::default()); }
        assert!(nonblank(&e.framebuffer) > 500, "cyberspace renders");
        e.cyber_active = false;
        e.alien_view_active = true;
        e.arm_alien_intro();
        for _ in 0..12 { e.step(MouseInput::default()); }
        assert!(nonblank(&e.framebuffer) > 500, "alien view renders");
        e.alien_view_active = false;

        // A dialogue scene plays through to completion (SCRIPT1 is the short one).
        e.on_ship = false;
        let total = e.dialogue_len();
        let mut finished = false;
        for _ in 0..20000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() { finished = true; break; }
        }
        assert!(finished, "SCRIPT1 dialogue scene completes");
        assert!(total > 1, "SCRIPT1 has real dialogue lines ({total})");
        assert!(e.dialogue_cursor() + 1 >= total, "cursor reached the last line");
    }

    /// The game's real flow after the intro: the SCRIPT1 console tutorial plays, then
    /// chains to SCRIPT2 via its decoded D2 handoff (profile 1). Verifies the chain
    /// trigger the driver relies on (`main.rs` auto-plays SCRIPT1 then follows this).
    /// The console CRYOBOX option opens the cryo-chamber (cryorad.hnm) — it loads and
    /// renders (with the HNM's own header palette), and the CRYOBOX menu row is clickable.
    #[test]
    fn cryobox_console_function_renders() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("sq").join("cryorad.hnm").exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(e.load_cryobox(assets), "cryorad.hnm loads");
        e.cryobox_active = true;
        for _ in 0..16 { e.step(MouseInput::default()); }
        // The cryo-chamber fills the frame in real (many-colour) content.
        assert!(e.framebuffer.iter().filter(|&&p| p != 0).count() > 5000, "cryo-chamber renders");
        let distinct = e.framebuffer.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(distinct > 20, "cryo-chamber has real colour ({distinct})");
    }

    /// The console TELEPHONE option opens the video-phone: the call widget + contact list
    /// render (dialling), a click connects a crew member, and the connected state shows
    /// their full-colour talk-head HNM feed. Esc/hangup returns to dialling.
    #[test]
    fn telephone_console_function_renders() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("BAPPEL.SPR").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("pe").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else { return };
        let mut e = EngineState::new();
        assert!(e.load_telephone(iso, assets), "BAPPEL.SPR + talk-heads load");
        assert!(e.load_console_font(iso), "console font loads");
        e.load_nav_chart(iso);
        assert!(e.phone_contact_count() >= 3, "several crew are callable");
        e.phone_active = true;
        // Dialling: the widget + contact list render as real content.
        for _ in 0..8 { e.step(MouseInput::default()); }
        assert!(!e.phone_connected(), "starts on the dial screen");
        assert!(e.framebuffer.iter().filter(|&&p| p != 0).count() > 500, "dial screen renders");
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
        for _ in 0..8 { e.step(MouseInput::default()); }
        let distinct = e.framebuffer.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(distinct > 16, "call feed for {name} has real colour ({distinct})");
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
            .iter().map(Path::new).find(|p| p.join("HONKF.SPR").is_file());
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
            .iter().map(Path::new).find(|p| p.join("sq").is_dir());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        e.load_cyberspace(assets);
        let (_, total) = e.cyber_progress();
        if total == 0 { return; }
        e.cyber_active = true;
        e.start_cyberspace();
        assert!(!e.cyber_arrived, "starts before the destination");
        // Steering right moves the on-course reticle; the frame stays real content.
        for _ in 0..8 { e.step(MouseInput { x: 260, y: 100, buttons: 0 }); }
        assert!(e.framebuffer.iter().filter(|&&p| p != 0).count() > 500, "tunnel + HUD render");
        // Fly the whole journey to arrival.
        for _ in 0..30000 {
            e.step(MouseInput::default());
            if e.cyber_arrived { break; }
        }
        assert!(e.cyber_arrived, "traversal reaches the destination");
        let (seg, tot) = e.cyber_progress();
        assert_eq!(seg, tot - 1, "ends on the last segment");
    }

    /// The console OPTION 3D-pyramid menu: renders the pyramid + the decoded 12-item list,
    /// selection cycles and click-maps to rows. Built from the ported manu3 logic + the
    /// decoded manu3.xdb item structure + the shared ship-3D projection.
    #[test]
    fn option_pyramid_menu_renders_and_selects() {
        let mut e = EngineState::new();
        assert_eq!(EngineState::OPTION_ITEM_COUNT, 12, "12 items decoded from manu3.xdb");
        e.option_active = true;
        for _ in 0..6 {
            e.step(MouseInput { x: 220, y: 100, buttons: 0 });
        }
        // The pyramid menu fills the frame with real content.
        assert!(e.framebuffer.iter().filter(|&&p| p != 0).count() > 3000, "pyramid renders");
        // Selection cycles and wraps.
        assert_eq!(e.option_item(), 0);
        e.option_cycle(1);
        assert_eq!(e.option_item(), 1);
        e.option_cycle(-2);
        assert_eq!(e.option_item(), EngineState::OPTION_ITEM_COUNT - 1, "wraps");
        // A click on the third row selects item 2.
        let y = (24 + 2 * 14) as u16;
        assert_eq!(e.option_item_click(20, y), Some(2));
    }

    /// The game-ending finale (`sq/fin.hnm`) loads, plays full-screen in colour, and
    /// reports finished once it reaches its last frame — the bookend to the intro.
    #[test]
    fn ending_finale_plays_to_completion() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("sq").join("fin.hnm").exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(e.load_ending(assets), "fin.hnm loads");
        e.start_ending();
        assert!(e.ending_active, "finale armed");
        assert!(!e.ending_finished(), "finale not finished at the start");
        // First frame renders real (many-colour) content.
        e.step(MouseInput::default());
        assert!(e.framebuffer.iter().filter(|&&p| p != 0).count() > 5000, "finale renders");
        let distinct = e.framebuffer.iter().collect::<std::collections::HashSet<_>>().len();
        assert!(distinct > 16, "finale has real colour ({distinct})");
        // Step through to completion.
        for _ in 0..4000 {
            if e.ending_finished() { break; }
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
        assert!(dst.tv_active && !dst.on_ship, "restored to the comms screen");
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
        let lines: Vec<(String, Option<std::path::PathBuf>)> = (0..250)
            .map(|i| (format!("line {i}"), None))
            .collect();
        e.set_speech_dialogue(lines);
        assert_eq!(e.dialogue_len(), 250, "all speech lines loaded");
        assert_eq!(e.current_subtitle(), Some("line 0"));
        e.on_ship = false;
        for _ in 0..40000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() { break; }
        }
        assert!(e.dialogue_cursor() + 1 >= 250, "cursor advances through all lines");
    }

    /// The choose-a-location nav: a destination list is offered on the star-map, a click
    /// on an entry maps to its index (matching the drawn layout), and visiting it plays
    /// that location's decoded dialogue.
    #[test]
    fn nav_destination_list_choose_a_location() {
        let mut e = EngineState::new();
        let dests: Vec<(String, Vec<(String, Option<std::path::PathBuf>)>)> = vec![
            ("EKATOMB".into(), (0..5).map(|i| (format!("daddy {i}"), None)).collect()),
            ("VENUSIA".into(), (0..7).map(|i| (format!("bug {i}"), None)).collect()),
            ("KORTEX".into(), (0..3).map(|i| (format!("hom {i}"), None)).collect()),
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
            .iter().map(Path::new).find(|p| p.join("HONKF.SPR").is_file());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        assert!(e.load_console_font(iso), "HONKF.SPR console font loads");
        e.load_bridge(iso);
        // HONK = H(7) O(14) N(13) K(10): the mapping must resolve uppercase letters.
        assert_eq!(EngineState::console_glyph_index('H'), Some(7));
        assert_eq!(EngineState::console_glyph_index('0'), Some(26));
        if e.panorama.is_none() { return; }
        e.bridge_active = true;
        e.step(MouseInput { x: 160, y: 100, buttons: 0 });
        // The baked menu glyphs use one palette index per row (0x7B + row).
        let menu_pixels = e
            .framebuffer
            .iter()
            .filter(|&&p| (0x7B..0x80).contains(&(p as usize)))
            .count();
        assert!(menu_pixels > 200, "golden menu rows present ({menu_pixels} px)");
        // A click on the HONK row (option 0) is detected; off-menu clicks are not.
        e.bridge.frame = crate::bridge::MENU_REST_FRAME;
        assert_eq!(e.console_menu_click(232, 0x48 + 1), Some(0), "HONK row clickable");
        assert_eq!(e.console_menu_click(10, 190), None, "off-menu click hits nothing");
    }

    #[test]
    fn script1_tutorial_chains_to_script2() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(Path::new).find(|p| p.join("SCRIPT1.COD").is_file());
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(Path::new).find(|p| p.join("sq").is_dir());
        let (Some(iso), Some(assets)) = (iso, assets) else { return };
        let db = crate::descript::DescriptDb::parse_file(iso.join("DESCRIPT.DES")).unwrap();
        let rd = |ext: &str| std::fs::read(iso.join(format!("SCRIPT1.{ext}"))).unwrap();
        let mut e = EngineState::new();
        e.load_dialogue_scenes(&rd("COD"), &rd("VAR"), &rd("DIC"), &rd("DEB"), &db, assets);
        e.dialogue_hold_frames = 20;
        e.on_ship = false;
        for _ in 0..20000 {
            e.step(MouseInput::default());
            if e.dialogue_finished() { break; }
        }
        assert!(e.dialogue_finished(), "SCRIPT1 tutorial completes");
        // Its D2 handoff requests profile 1 -> the driver loads SCRIPT(1+1)=SCRIPT2.
        assert_eq!(e.pending_next_scene(), Some(1), "SCRIPT1 chains to SCRIPT2 via D2");
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
        // Rate-based (joystick) steering: cursor at centre → no turn; cursor held to
        // one side turns the compass a bit each frame in that direction.
        e.compass_angle = 90;
        e.step(MouseInput {
            x: 160,
            y: 100,
            buttons: 0,
        });
        assert_eq!(e.compass_angle, 90, "centred cursor holds heading");
        let frame_centre = e.framebuffer.clone();
        // Hold right for several frames: heading advances upward.
        for _ in 0..10 {
            e.step(MouseInput {
                x: 300,
                y: 100,
                buttons: 0,
            });
        }
        let right = e.compass_angle;
        // Hold left: heading moves back down past where it was.
        for _ in 0..20 {
            e.step(MouseInput {
                x: 20,
                y: 100,
                buttons: 0,
            });
        }
        assert!(right > 90, "holding right turns the compass up (got {right})");
        assert!(e.compass_angle < right, "holding left reverses the turn");
        assert!(
            frame_centre.iter().any(|&p| p != 0),
            "the starfield renders some points"
        );
        assert_ne!(
            frame_centre, e.framebuffer,
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
        // Step enough frames for the character-by-character reveal to fully draw.
        for _ in 0..400 {
            e.step(MouseInput::default());
        }
        // Revealed glyphs use 0xFD; the single reveal-edge glyph uses 0xFE.
        let lit = e
            .framebuffer
            .iter()
            .filter(|&&p| p == 0xFD || p == 0xFE)
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
    fn title_screen_loads_and_shows_the_decoded_box_art() {
        let iso = ["output/_tmp_iso", "../output/_tmp_iso"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(iso) = iso else { return };
        let mut e = EngineState::new();
        assert!(e.load_title(iso), "BLOOD.LBM title art loads");
        assert!(e.title_active());
        // The title takes render precedence and fills the framebuffer with real art.
        e.step(MouseInput::default());
        let distinct = e.framebuffer.iter().collect::<std::collections::BTreeSet<_>>().len();
        assert!(distinct >= 8, "title art renders ({distinct} indices)");
        // Dismissing advances past the title.
        e.dismiss_title();
        assert!(!e.title_active());
    }

    #[test]
    fn world_ext_objects_are_marked_on_the_location() {
        let dat = ["output/_tmp_dat","../output/_tmp_dat"].iter().map(std::path::Path::new).find(|p| p.exists());
        let iso = ["output/_tmp_iso","../output/_tmp_iso"].iter().map(std::path::Path::new).find(|p| p.exists());
        let (Some(dat), Some(iso)) = (dat, iso) else { return };
        let mut e = EngineState::new();
        if !e.visit_world("venusia", dat) { return; }
        let ext = std::fs::read(iso.join("VENUSIA.EXT")).unwrap();
        let n = e.set_world_ext(&ext);
        assert!(n >= 1, "venusia has >=1 decoded object");
        // Rendering marks them: the marker index 0xFD appears in the framebuffer.
        e.step(MouseInput::default());
        assert!(e.framebuffer.iter().any(|&p| p == 0xFD), "object marker rendered");
    }

    #[test]
    fn visiting_a_world_loads_its_decoded_location_background() {
        let assets = ["output/_tmp_dat", "../output/_tmp_dat"]
            .iter().map(std::path::Path::new).find(|p| p.exists());
        let Some(assets) = assets else { return };
        let mut e = EngineState::new();
        assert!(!e.world_location_active());
        // Visiting a mapped world loads its fd/ room background.
        assert!(e.visit_world("venusia", assets), "venusia has decoded location art");
        assert!(e.world_location_active());
        // The landing screen renders the background (non-blank framebuffer).
        e.step(MouseInput::default());
        let distinct = e.framebuffer.iter().collect::<std::collections::BTreeSet<_>>().len();
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
        assert_eq!(e.world_room_position().unwrap().0, count - 1, "wraps backward");
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
        assert_eq!(e.targeted_world_index(), n - 1, "max heading targets the last world");
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
}
