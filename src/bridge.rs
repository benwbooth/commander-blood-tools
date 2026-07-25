//! The ship bridge — the game's hub screen — decompiled from BLOODPRG.EXE.
//!
//! The bridge is a 360° pre-rendered panorama ([`crate::tbbig`]) the player
//! rotates through with the mouse. This module is the *interaction* layer: the
//! steering/station-seek state machine, the eye-orb click targets, and the
//! golden console-menu hit testing + palette-row highlighting, all ported from
//! the original code (file offsets below are into `re/bin/BLOODPRG.EXE`; DS
//! offsets are the game's data-segment globals, see `re/labels.csv`).
//!
//! Coordinate system (recovered at `0x9656..0x981A`):
//! * The panorama ring is **1440 px around** (180 frames x 8 px per 2° frame).
//! * The hardware mouse cursor is kept in *ring space*: every tick the game
//!   warps it to `ring_x + 0x5A0` (int 33h AX=4 at `0x9722`) so relative mouse
//!   motion accumulates without hitting range clamps — push steering.
//! * `DS:0x27A7 = frame * 8 - 160` converts ring x to *screen* x
//!   (`screen = ring - (frame * 8 - 160)`, i.e. the view centre column shows
//!   ring position `frame * 8`); `0x97FC..0x981A` rebases the stored mouse x
//!   into screen space each tick for the click hit tests.
//!
//! Verified live against the real game running in the recomp emulator
//! (`runtime_boot` env `BRIDGEPROBE`): with the cursor parked at ring 320 the
//! view settles on frame 55 = `(320/4 + 30) / 2`, at the left edge on 15, at
//! the right edge on 64 — all three match this module's `update_view` exactly.

use crate::tbbig::{ANGLE_UNITS_PER_REVOLUTION, PANORAMA_FRAME_COUNT};

/// Ring pixels per panorama frame (8 px = 2° of the 1440-px ring).
pub const RING_PX_PER_FRAME: i32 = 8;
/// Ring x of the view centre at frame f is `f * 8`; the screen shows
/// `ring_of_view - 160 .. + 160`.
pub const HALF_SCREEN: i32 = 160;

/// The "arc" scale the original steering math runs in: quarter-ring units
/// (`ring / 4`, 360 per revolution — 2 arc units per frame).
const ARC_PER_REVOLUTION: i32 = 360;
/// Steering dead zone in arc units (`0x9752`: distances <= 0x1F don't move).
const STEER_DEAD_ZONE_ARC: i32 = 0x1F;
/// When steering, the view lands 0x1E arc units (15 frames) short of the mouse
/// (`0x97C4`/`0x97D4`) — the view trails the pushed cursor.
const STEER_TRAIL_ARC: i32 = 0x1E;
/// While a menu item is engaged (`DS:0x2793` bit 2), the cursor is dragged back
/// toward the view once it strays this many arc units (`0x9762`: 0x28).
const MENU_CLAMP_ARC: i32 = 0x28;

/// Golden console menu: hit box and row metrics from `0x8613..0x868D`.
/// The menu is baked into panorama frames 40..=60 and is only clickable there.
pub const MENU_FRAME_MIN: u16 = 0x28;
pub const MENU_FRAME_MAX: u16 = 0x3C;
/// The menu's rest frame (`0x8642` subtracts 0x2D; the click seeks to
/// `[0x279B] = 0x5A` = arc 90 = frame 45).
pub const MENU_REST_FRAME: u16 = 45;
/// At the rest frame the menu box spans screen x `177..=287` (`0x8650..0x8663`:
/// right = 0xE8 + 0x37 = 287, left = right - 0x6E = 177); it scrolls with the
/// panorama at -8 px per frame of delta.
const MENU_RIGHT_AT_REST: i32 = 0xE8 + 0x37;
const MENU_WIDTH: i32 = 0x6E;
/// Row metrics (`0x8671..0x868B`): top row y = 0x48 + |δ| * 1.25, row pitch
/// = 0x12 - |δ| / 8, five rows.
const MENU_TOP_AT_REST: i32 = 0x48;
const MENU_ROW_PITCH_AT_REST: i32 = 0x12;
pub const MENU_ROW_COUNT: usize = 5;

/// The five menu rows' glyphs in the panorama frames are painted with one
/// dedicated palette index per row, starting here (`0x8697`: DAC index
/// 0x7B + row). Re-programming these five DAC entries is how the game
/// highlights the hovered row without redrawing pixels.
pub const MENU_ROW_DAC_BASE: usize = 0x7B;
/// Idle row colour, 6-bit DAC (`0x8636..0x863F`: five entries of 16,12,0 —
/// dark gold).
pub const MENU_ROW_IDLE_DAC: [u8; 3] = [0x10, 0x0C, 0x00];
/// Hovered row colour, 6-bit DAC (`0x869D..0x86A3`: 63,0,0 — bright red).
pub const MENU_ROW_HOVER_DAC: [u8; 3] = [0x3F, 0x00, 0x00];

/// One clickable record of the 6-entry table at `DS:0x2A1B` (0x18-byte stride).
/// The four panorama stations' records get their orb rectangle refreshed from
/// the loaded frame's chunk header; their seek target (stored doubled, at
/// +0x0A) is the station's rest frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct StationRecord {
    /// +0x00 flags: bit 0 = active/clickable, bit 3 = set by the hit test while
    /// the button is down inside the rectangle (`0x828F`).
    pub active: bool,
    /// +0x0A seek target in arc units (2 * rest frame).
    pub target_arc: u16,
    /// +0x0C..0x13 the eye-orb's clickable rectangle {x, y, w, h} in screen
    /// space — copied from the current panorama frame's chunk header
    /// (`0x9877..0x9889`), -1 = none.
    pub orb_box: Option<[u16; 4]>,
}

/// Station rest frames, read from the STATIC station-record table in the image —
/// not from a probe dump, which is what this used to cite.
///
/// `DS:0x2A1B` (file `0xFE3B`) is an array of SIX 24-byte records: the walker at
/// `0x7DAB` is `mov bp,0x2A1B / mov cx,6 / ... / add bp,0x18 / loop`, dispatching
/// each through `call word cs:[bx+0x6D4]`. `0x9642` clears the array the same way
/// (`mov word [bp],0`, 6 iterations) and `0x985F` fills `+0xC..+0x1C` with
/// `0xFFFFFFFF` — the orb box, four words, matching [`StationRecord::orb_box`].
///
/// Field `+0xA` is the rest ANGLE in degrees: records 0..5 hold
/// `0x000, 0x05A, 0x0B4, 0x10E, 0, 0` in the file at `0xFE45`, `0xFE5D`, `0xFE75`,
/// `0xFE8D`. At 2° per panorama frame ([`crate::tbbig::PANORAMA_FRAME_COUNT`] =
/// 180 over 360°) that is frames 0, 45, 90, 135 — helm, golden menu, pyramid nav
/// room, organic Orxx. `station_rest_frames_match_the_static_record_table` reads
/// those bytes back out of BLOODPRG.EXE.
/// DERIVED, so `tools/check_literal_tables.py` reports it ABSENT and that is
/// correct: the file stores ANGLES (`0x000, 0x05A, 0x0B4, 0x10E` = 0, 90, 180,
/// 270 degrees) and these are those angles halved, because a panorama frame is
/// 2 degrees. The stored bytes are checked by
/// `station_rest_frames_match_the_static_record_table`; this array is the port's
/// conversion of them, not a copy.
pub const STATION_REST_FRAMES: [u16; 4] = [0, 45, 90, 135];

/// `DS:0x2A1B` -> file offset of the station-record array (`0xD420 + 0x2A1B`).
pub const STATION_RECORD_TABLE_FILE_OFFSET: usize = 0xFE3B;
/// Six records (`mov cx,6` @`0x7DA8`).
pub const STATION_RECORD_COUNT: usize = 6;
/// 24 bytes each (`add bp,0x18` @`0x7E0E`).
pub const STATION_RECORD_STRIDE: usize = 0x18;
/// Rest angle in degrees at `+0xA`; the panorama frame is half of it.
pub const STATION_RECORD_REST_ANGLE_OFFSET: usize = 0x0A;

/// The decompiled bridge view state — the DS globals the steering, seek, and
/// menu code touch, as one struct.
#[derive(Clone, Debug)]
pub struct BridgeView {
    /// The console row whose surface (choice box / engaged state) is open — its
    /// label renders pure red (oracle capture: (255,0,0) while engaged).
    pub engaged_row: Option<usize>,
    /// Current panorama frame, `DS:0x2795` (0..179).
    pub frame: u16,
    /// The mouse's position around the panorama ring (`DS:0x0A2A` before the
    /// screen-space rebase; kept ring-absolute here, the warp trick's anchor).
    pub ring_mouse_x: i32,
    /// Mouse y in screen space (`DS:0x0A2C`).
    pub mouse_y: i32,
    /// Seek mode (`DS:0x2793` bit 3): auto-rotating toward `seek_target_arc`.
    pub seeking: bool,
    /// Menu-engaged mode (`DS:0x2793` bit 2): set with the seek when a menu row
    /// is clicked; clamps how far the cursor may stray from the view.
    pub menu_engaged: bool,
    /// Seek target in arc units = 2 * target frame (`DS:0x279B`).
    pub seek_target_arc: u16,
    /// First-tick seek distance memo in frames (`DS:0x279D`); long seeks
    /// (>= 0x28 frames) drag the cursor anchor along with the rotation.
    pub seek_initial_frames: u16,
    /// Selected console-menu item, 1..=5 (`DS:0x2A19`; 0 = none). Order as
    /// baked into the frames, top to bottom: HONK, TELEPHONE, CRYOBOX, MENU,
    /// OPTION.
    pub selected_menu_item: u16,
    /// The 6-record clickable table (`DS:0x2A1B`).
    pub stations: [StationRecord; 6],
}

impl Default for BridgeView {
    fn default() -> Self {
        let mut stations = [StationRecord::default(); 6];
        for (index, &rest) in STATION_REST_FRAMES.iter().enumerate() {
            stations[index].target_arc = rest * 2;
        }
        // Live table at the console: the menu (1) and nav (2) station records
        // are the active/clickable ones.
        stations[1].active = true;
        stations[2].active = true;
        BridgeView {
            engaged_row: None,
            frame: MENU_REST_FRAME,
            ring_mouse_x: (MENU_REST_FRAME as i32) * RING_PX_PER_FRAME,
            mouse_y: 100,
            seeking: false,
            menu_engaged: false,
            seek_target_arc: 0,
            seek_initial_frames: 0,
            selected_menu_item: 0,
            stations,
        }
    }
}

fn wrap(value: i32, modulus: i32) -> i32 {
    value.rem_euclid(modulus)
}

/// Shortest signed distance from `from` to `to` on a ring of `modulus`.
fn ring_delta(from: i32, to: i32, modulus: i32) -> i32 {
    let mut d = wrap(to - from, modulus);
    if d > modulus / 2 {
        d -= modulus;
    }
    d
}

impl BridgeView {
    /// The mouse cursor's screen-space x for the current view
    /// (`ring - (frame * 8 - 160)`, the `0x97FC` rebase).
    pub fn mouse_screen_x(&self) -> i32 {
        wrap(
            self.ring_mouse_x - (self.frame as i32 * RING_PX_PER_FRAME - HALF_SCREEN),
            ANGLE_UNITS_PER_REVOLUTION as i32,
        )
    }

    /// Feed relative mouse motion (the port's stand-in for the original's
    /// warped hardware cursor: motion accumulates in ring space).
    pub fn move_mouse(&mut self, dx: i32, dy: i32) {
        self.ring_mouse_x = wrap(self.ring_mouse_x + dx, ANGLE_UNITS_PER_REVOLUTION as i32);
        self.mouse_y = (self.mouse_y + dy).clamp(0, 199);
    }

    /// One tick of the view state machine (`0x9656..0x97E3`). Returns true if
    /// the frame changed (the original's carry flag, telling the caller to
    /// redraw the panorama).
    ///
    /// Every path through the steer/seek update FALLS INTO the frame->yaw sync
    /// at `0x97E4`, which snaps the ring cursor to an 8-unit boundary
    /// (`0x97F6 and word [0xa2a],0xfff8`) before the screen rebase — 8 ring
    /// units = one whole panorama frame (2°), so the stored cursor is always
    /// frame-aligned. [`update_view_steer`] is the `0x9656..0x97E3` body; this
    /// wrapper applies the sync's snap the way the fall-through does.
    pub fn update_view(&mut self) -> bool {
        let changed = self.update_view_steer();
        // 0x97F6: `and [0xa2a],0xfff8` — clear the low 3 bits of the ring x.
        self.ring_mouse_x &= !(RING_PX_PER_FRAME - 1);
        changed
    }

    fn update_view_steer(&mut self) -> bool {
        let arc_view = self.frame as i32 * 2;
        let arc_mouse = wrap(self.ring_mouse_x, ANGLE_UNITS_PER_REVOLUTION as i32) / 4;

        if self.seeking {
            // Station seek (`0x9667..0x96F5`): ease toward the target half the
            // remaining distance per tick, shortest way around the 180-ring.
            let target_frame = (self.seek_target_arc / 2) as i32;
            if self.frame as i32 == target_frame {
                self.seeking = false;
                self.seek_initial_frames = 0;
                return false;
            }
            let delta = ring_delta(self.frame as i32, target_frame, PANORAMA_FRAME_COUNT as i32);
            let distance = delta.abs();
            if self.seek_initial_frames == 0 {
                self.seek_initial_frames = distance as u16;
            }
            let step = (distance / 2).max(1) * delta.signum();
            // Long seeks (initial distance >= 0x28 frames) drag the cursor's
            // ring anchor along so it stays put on screen (`0x96D0..0x96DD`).
            if self.seek_initial_frames >= 0x28 {
                self.ring_mouse_x = wrap(
                    self.ring_mouse_x + delta.signum() * distance * 4,
                    ANGLE_UNITS_PER_REVOLUTION as i32,
                );
            }
            self.frame =
                wrap(self.frame as i32 + step, PANORAMA_FRAME_COUNT as i32) as u16;
            return true;
        }

        // Mouse-push steering (`0x973D..0x97E3`).
        let distance = ring_delta(arc_view, arc_mouse, ARC_PER_REVOLUTION).abs();
        if distance <= STEER_DEAD_ZONE_ARC {
            return false;
        }
        if self.menu_engaged {
            // Menu engaged: don't rotate; once the cursor strays >= 0x28 arc
            // units, drag it back to 0x28 from the view (`0x976A..0x97AB`).
            if distance < MENU_CLAMP_ARC {
                return false;
            }
            let toward_view = ring_delta(arc_mouse, arc_view, ARC_PER_REVOLUTION).signum();
            let clamped_arc = wrap(
                arc_view - toward_view * MENU_CLAMP_ARC,
                ARC_PER_REVOLUTION,
            );
            self.ring_mouse_x = clamped_arc * 4;
            return false;
        }
        // The view lands STEER_TRAIL_ARC short of the mouse, on the near side
        // (`0x97BB..0x97E3`): direction from whether view = mouse + distance.
        let ahead = wrap(arc_mouse + distance, ARC_PER_REVOLUTION) == arc_view;
        let arc_new = if ahead {
            wrap(arc_mouse + STEER_TRAIL_ARC, ARC_PER_REVOLUTION)
        } else {
            wrap(arc_mouse - STEER_TRAIL_ARC, ARC_PER_REVOLUTION)
        };
        self.frame = (arc_new / 2) as u16 % PANORAMA_FRAME_COUNT as u16;
        true
    }

    /// Refresh the current frame's station record with its orb rectangle, as
    /// the frame loader does (`0x9860..0x9889`): all four boxes reset, then the
    /// loaded chunk's box stored on its own station's record.
    pub fn set_frame_orb_box(&mut self, station: u16, orb_box: Option<[u16; 4]>) {
        for record in self.stations.iter_mut().take(4) {
            record.orb_box = None;
        }
        if let Some(record) = self.stations.get_mut(station as usize) {
            record.orb_box = orb_box;
        }
    }

    /// Menu row under the cursor, if the menu is in view and the cursor is
    /// inside the box — the hit math of `0x8613..0x868D`. Returns 0..=4 top to
    /// bottom (HONK, TELEPHONE, CRYOBOX, MENU, OPTION).
    pub fn menu_row_under_cursor(&self) -> Option<usize> {
        if self.frame < MENU_FRAME_MIN || self.frame > MENU_FRAME_MAX {
            return None;
        }
        let delta = self.frame as i32 - MENU_REST_FRAME as i32;
        let mouse_x = self.mouse_screen_x();
        // Box right edge scrolls with the panorama: 287 - δ*8; left = right-110.
        let right = MENU_RIGHT_AT_REST - delta * RING_PX_PER_FRAME;
        let left = right - MENU_WIDTH;
        if mouse_x > right || mouse_x < left {
            return None;
        }
        // Rows shift down and compress slightly as the view rotates off-centre.
        let skew = delta.abs();
        let top = MENU_TOP_AT_REST + skew + skew / 4;
        let pitch = MENU_ROW_PITCH_AT_REST - (skew / 4) / 2;
        let below_top = self.mouse_y - top;
        if below_top < 0 || pitch <= 0 {
            return None;
        }
        let row = (below_top / pitch) as usize;
        (row < MENU_ROW_COUNT).then_some(row)
    }

    /// A mouse-button press on the bridge (`0x86A4..0x86F1` menu path,
    /// `0x7DC8..0x7DE6` orb path). Returns the newly selected menu item
    /// (1..=5) if a menu row was clicked.
    pub fn click(&mut self) -> Option<u16> {
        if let Some(row) = self.menu_row_under_cursor() {
            // Select the item, centre the menu, and engage the cursor clamp
            // (`0x86AB..0x86C1`: [0x2A19]=row+1, [0x2793]|=0xC, [0x279B]=0x5A).
            self.selected_menu_item = row as u16 + 1;
            self.seeking = true;
            self.menu_engaged = true;
            self.seek_target_arc = MENU_REST_FRAME * 2;
            // The initial-distance memo [0x279d] is cleared only at seek COMPLETION
            // (0x9676), not at arm — a seek re-armed mid-flight keeps the prior memo.
            return Some(self.selected_menu_item);
        }
        // Eye-orb / station click: while a menu item is engaged the whole
        // record scan is skipped (`0x7D96` ors [0x2A19] into the busy gate).
        if self.selected_menu_item != 0 {
            return None;
        }
        let mouse_x = self.mouse_screen_x();
        for record in self.stations {
            if !record.active {
                continue;
            }
            let Some([x, y, w, h]) = record.orb_box else {
                continue;
            };
            let inside = mouse_x >= x as i32
                && mouse_x <= x as i32 + w as i32
                && self.mouse_y >= y as i32
                && self.mouse_y <= y as i32 + h as i32;
            if inside && self.frame * 2 != record.target_arc {
                self.seek_target_arc = record.target_arc;
                self.seeking = true;
                // Memo cleared only at completion (0x9676), not at arm.
                break;
            }
        }
        None
    }

    /// Release the engaged menu item (the selected screen was closed): clears
    /// the selection and the cursor clamp.
    pub fn release_menu(&mut self) {
        self.selected_menu_item = 0;
        self.menu_engaged = false;
    }

    /// Program the five menu-row DAC entries (game palette indices
    /// 0x7B..0x7F): all idle, then the hovered row bright — the per-tick DAC
    /// writes of `0x862B..0x86A3`. Only touches the palette while the menu
    /// sector is in view, like the original gate.
    pub fn apply_menu_palette(&self, palette: &mut [[u8; 3]; 256]) {
        if self.frame < MENU_FRAME_MIN || self.frame > MENU_FRAME_MAX {
            return;
        }
        let expand = |c: [u8; 3]| [c[0] << 2 | c[0] >> 4, c[1] << 2 | c[1] >> 4, c[2] << 2 | c[2] >> 4];
        for row in 0..MENU_ROW_COUNT {
            palette[MENU_ROW_DAC_BASE + row] = expand(MENU_ROW_IDLE_DAC);
        }
        if let Some(row) = self.menu_row_under_cursor() {
            palette[MENU_ROW_DAC_BASE + row] = expand(MENU_ROW_HOVER_DAC);
        }
        // The ENGAGED row (its surface open) renders PURE RED — oracle capture:
        // the row label reads (255,0,0) while its choice box / state is active.
        if let Some(row) = self.engaged_row {
            if row < MENU_ROW_COUNT {
                palette[MENU_ROW_DAC_BASE + row] = [255, 0, 0];
            }
        }
    }
}

#[cfg(test)]
mod tests {

    /// The rest frames come from the game's own STATION-RECORD TABLE, so they are
    /// checkable against the image rather than against a memory dump. Six 24-byte
    /// records at file 0xFE3B (`DS:0x2A1B`); field +0xA is the rest angle in
    /// degrees, and the panorama frame is half of it (2 degrees per frame).
    #[test]
    fn station_rest_frames_match_the_static_record_table() {
        let Some(image) = std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
            .ok()
        else {
            eprintln!("skipping: BLOODPRG.EXE not available");
            return;
        };

        let angle_at = |record: usize| -> u16 {
            let at = STATION_RECORD_TABLE_FILE_OFFSET
                + record * STATION_RECORD_STRIDE
                + STATION_RECORD_REST_ANGLE_OFFSET;
            u16::from_le_bytes([image[at], image[at + 1]])
        };

        // The four stations the bridge exposes, and the two unused trailing records.
        assert_eq!(
            (0..STATION_RECORD_COUNT).map(angle_at).collect::<Vec<_>>(),
            vec![0x000, 0x05A, 0x0B4, 0x10E, 0, 0],
            "station rest angles in the static image"
        );
        for (station, &frame) in STATION_REST_FRAMES.iter().enumerate() {
            assert_eq!(
                frame,
                angle_at(station) / 2,
                "station {station}: frame is half the recorded angle"
            );
        }
    }
    use super::*;

    /// The three live BRIDGEPROBE observations, replayed through the ported
    /// steering law: parked cursor ring positions 320 / ~0 / ~636 settle the
    /// view on frames 55 / 15 / 64 exactly as the real game did.
    #[test]
    fn steering_settles_where_the_live_game_did() {
        for (ring, expected_frame, from_frame) in [(320, 55, 64), (2, 15, 55), (636, 64, 15)] {
            let mut view = BridgeView {
                frame: from_frame,
                ring_mouse_x: ring,
                ..BridgeView::default()
            };
            for _ in 0..64 {
                view.update_view();
            }
            assert_eq!(
                view.frame, expected_frame,
                "cursor at ring {ring} from frame {from_frame}"
            );
        }
    }

    #[test]
    fn ring_cursor_snaps_to_the_frame_grid() {
        // 0x97F6 `and word [0xa2a],0xfff8`: the frame->yaw sync every steer/seek
        // tick falls into clears the ring x's low 3 bits, so the stored cursor
        // is always on an 8-unit (one-panorama-frame) boundary.
        for start in [45 * 8 + 3, 45 * 8 + 7, 2, 639] {
            let mut view = BridgeView {
                frame: 45,
                ring_mouse_x: start,
                ..BridgeView::default()
            };
            view.update_view();
            assert_eq!(
                view.ring_mouse_x % RING_PX_PER_FRAME,
                0,
                "ring x {} snapped to the 8-unit grid, got {}",
                start,
                view.ring_mouse_x
            );
            assert!(
                view.ring_mouse_x <= start,
                "the snap rounds DOWN (an AND mask), never up"
            );
        }
    }

    #[test]
    fn dead_zone_holds_the_view_still() {
        // Cursor within 31 arc units (124 ring px) of the view centre: no move.
        let mut view = BridgeView {
            frame: 45,
            ring_mouse_x: 45 * 8 + 120,
            ..BridgeView::default()
        };
        assert!(!view.update_view());
        assert_eq!(view.frame, 45);
    }

    #[test]
    fn station_seek_eases_and_arrives() {
        let mut view = BridgeView::default();
        view.frame = 90;
        view.seeking = true;
        view.seek_target_arc = 45 * 2;
        let mut steps = 0;
        while view.seeking && steps < 32 {
            view.update_view();
            steps += 1;
        }
        assert_eq!(view.frame, 45);
        assert!(steps >= 5, "half-distance easing takes several ticks, got {steps}");
    }

    #[test]
    fn seek_wraps_the_short_way_around() {
        let mut view = BridgeView::default();
        view.frame = 170;
        view.seeking = true;
        view.seek_target_arc = 0; // helm, 10 frames forward across the wrap
        view.update_view();
        assert!(view.frame > 170 || view.frame == 0, "went the short way: {}", view.frame);
    }

    #[test]
    fn menu_hit_boxes_match_the_binary_math() {
        let mut view = BridgeView::default();
        view.frame = MENU_REST_FRAME;
        view.ring_mouse_x = MENU_REST_FRAME as i32 * 8 + (232 - 160); // screen x 232
        view.mouse_y = 0x48 + 1; // first row
        assert_eq!(view.menu_row_under_cursor(), Some(0));
        view.mouse_y = 0x48 + 0x12 * 4 + 1; // fifth row
        assert_eq!(view.menu_row_under_cursor(), Some(4));
        view.mouse_y = 0x48 + 0x12 * 5 + 1; // below the menu
        assert_eq!(view.menu_row_under_cursor(), None);
        // Outside the box horizontally (screen x 100).
        view.ring_mouse_x = MENU_REST_FRAME as i32 * 8 + (100 - 160);
        view.mouse_y = 0x48 + 1;
        assert_eq!(view.menu_row_under_cursor(), None);
        // Menu not in view at all.
        view.frame = 90;
        assert_eq!(view.menu_row_under_cursor(), None);
    }

    #[test]
    fn menu_click_selects_seeks_and_engages_clamp() {
        let mut view = BridgeView::default();
        view.frame = 50; // menu visible, off-centre by 5 frames
        view.ring_mouse_x = 50 * 8 + (200 - 160); // screen x 200, inside box (right=247,left=137)
        view.mouse_y = 0x48 + 5 + 5 / 4 + 1; // first row at skew 5
        let selected = view.click();
        assert_eq!(selected, Some(1), "HONK selected");
        assert!(view.seeking && view.menu_engaged);
        assert_eq!(view.seek_target_arc, MENU_REST_FRAME * 2);
        while view.seeking {
            view.update_view();
        }
        assert_eq!(view.frame, MENU_REST_FRAME, "click centres the menu");
    }

    #[test]
    fn orb_click_seeks_to_the_station_rest_frame() {
        let mut view = BridgeView::default();
        view.frame = 55;
        // Frame 55's real orb box (chunk header): {14, 106, 48, 35}.
        view.set_frame_orb_box(1, Some([14, 106, 48, 35]));
        view.ring_mouse_x = 55 * 8 + (30 - 160); // screen x 30, inside the orb
        view.mouse_y = 120;
        assert_eq!(view.click(), None);
        assert!(view.seeking);
        assert_eq!(view.seek_target_arc, 45 * 2, "seeks the menu station rest");
    }

    #[test]
    fn hover_highlight_programs_the_row_dac_entry() {
        let mut view = BridgeView::default();
        view.frame = MENU_REST_FRAME;
        view.ring_mouse_x = MENU_REST_FRAME as i32 * 8 + (232 - 160);
        view.mouse_y = 0x48 + 0x12 * 2 + 1; // third row (CRYOBOX)
        let mut palette = [[0u8; 3]; 256];
        view.apply_menu_palette(&mut palette);
        let idle = [(0x10 << 2) | (0x10 >> 4), (0x0C << 2) | (0x0C >> 4), 0];
        assert_eq!(palette[MENU_ROW_DAC_BASE], idle);
        assert_eq!(palette[MENU_ROW_DAC_BASE + 2], [0xFF, 0, 0]);
        assert_eq!(palette[MENU_ROW_DAC_BASE + 4], idle);
    }
}
