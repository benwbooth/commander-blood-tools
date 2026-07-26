//! `TB.BIG` — the ship-bridge *panorama* archive ("tableau de bord" = dashboard).
//!
//! The bridge of the Ark is not a set of separate screens: it is one pre-rendered
//! 360° panorama of 180 full-screen frames (2° per frame), and the player's mouse
//! steering rotates the view through them. The four ship "stations" (wide helm
//! view with the eye-orb, the golden console menu, the pyramid navigation room,
//! and the organic Orxx mass) are simply sectors of this ring; the golden
//! HONK/TELEPHONE/CRYOBOX/MENU/OPTION menu text is baked into the frames.
//!
//! Reverse-engineered from BLOODPRG.EXE (all offsets are file offsets in
//! `re/bin/BLOODPRG.EXE`; game globals are data-segment offsets, see
//! `re/labels.csv`):
//!
//! * `0x0F68` opens `tb.big` (name at DS:0x00D3) and keeps the DOS handle in
//!   DS:0x0AC4 for the whole session.
//! * `0x981B` (`bridge_panorama_frame_load`) — given a frame index: seeks to
//!   `index * 8`, reads the `{offset: u32, size: u32}` directory entry, seeks to
//!   the chunk, and reads it into the load buffer. It then copies the chunk's
//!   8-byte bounding-box record into the 4-entry station table at DS:0x2A1B
//!   (0x18-byte stride, box at +0x0C; the entry is picked by the chunk's own
//!   station word at data offset +8) and calls the unpacker.
//! * `0x2D50` (far `0x1CE:0x0A70`, `bridge_panorama_frame_unpack`) — the RLE
//!   unpacker, decoding *exactly* 64000 pixels (full 320x200) onto the linear
//!   back buffer. Two variants, selected by the game flag DS:0x5B57 bit 0:
//!   transparent (colour 0 leaves the buffer pixel — used every frame while the
//!   view rotates, so the starfield drawn in the black window areas survives)
//!   and opaque (colour 0 written — used to (re)establish a frame from scratch).
//! * `DS:0x2795` — current panorama frame index (0..179). It is copied verbatim
//!   into the ship-3D yaw index DS:0x2F6D (`0x97E7`), which is why the panorama
//!   has exactly 180 frames: they correspond 1:1 with the ship-3D angle table's
//!   180 x 2° steps (`ship3d::SHIP_3D_ANGLE_TABLE`).
//! * `DS:0x0A2A` — bridge view angle in 1440ths of a revolution; 8 units per
//!   panorama frame (`0x97E7` computes `frame * 8 - 0xA0` and wraps into
//!   `0..0x5A0`).
//!
//! Live-verified against the real game running in the recomp emulator
//! (`runtime_boot` env `BRIDGEPROBE`): at the interactive console the game rests
//! on frame 55 (angle 0x28) and our decode of frame 55 matches the emulator's
//! VGA output at mean_abs = 2.47 (ground truth in `accuracy/captures/bridge/`;
//! the residual is the pointing-hand cursor sprite and the window starfield,
//! which are drawn over the panorama at runtime). Steering: mouse at the left
//! screen edge rotated the live game to frame 15, right edge to frame 64, and
//! the view springs back to the station rest frame at centre.

/// Width * height of one panorama frame — the unpacker at `0x2D50` decodes
/// exactly this many pixels (`ebp` starts at 0xFA00) regardless of content.
pub const PANORAMA_FRAME_PIXELS: usize = 320 * 200;

/// Full revolution = 180 frames at 2° per frame (matches the ship-3D angle table).
pub const PANORAMA_FRAME_COUNT: usize = 180;

/// View-angle units (DS:0x0A2A) per panorama frame: 0x5A0 units / 180 frames.
/// View-angle units per panorama frame — DERIVED, not an immediate anywhere:
/// `ANGLE_UNITS_PER_REVOLUTION / PANORAMA_FRAME_COUNT` = `0x5A0 / 180` = 8.
///
/// Both operands are cited: the revolution is `add bx,0x5a0` @`0x9807`, and the
/// frame count is the panorama's own directory length. The quotient is exact, so
/// this is an identity rather than a measurement — if either operand were wrong
/// the division would not come out whole.
pub const ANGLE_UNITS_PER_FRAME: u16 = 8;

/// View-angle wrap modulus: 1440 units per revolution (0x97F6..0x9815 wraps
/// DS:0x0A2A into this range).
pub const ANGLE_UNITS_PER_REVOLUTION: u16 = 0x5A0;

/// The panorama frame the live game rests on at the interactive console
/// station (observed via `BRIDGEPROBE`; verified pixel-close to the emulator
/// capture `accuracy/captures/bridge/console_rest.ppm`).
pub const CONSOLE_REST_FRAME: usize = 55;
/// The panorama frame whose BOTTOM 60 ROWS are the intro montage's console band.
///
/// Proven by construction: frame 90's rows 140..200, pushed through the
/// console-bank remap table (`palette::build_console_bank_remap_table`), equal the
/// harvested `console_band.idx` in ALL 19200 bytes. So the band was never a
/// separate asset — it is this panorama frame, colour-reduced by the montage's
/// full-screen remap (`0x7AC3`).
///
/// AND IT IS UNIQUE. `the_console_band_is_the_panorama_frame_remapped` now
/// searches all 180 frames the same way and asserts that EXACTLY ONE reproduces
/// the band. That is the difference between an index read off a capture and one
/// determined by the archive: given the band, the data leaves no other choice.
/// The constant is therefore DATA, not an oracle measurement — the capture is the
/// target of the search, never the source of the number.
pub const CONSOLE_BAND_FRAME: usize = 90;
/// The band's screen origin and height (rows 140..200).
pub const CONSOLE_BAND_TOP: usize = 140;
pub const CONSOLE_BAND_HEIGHT: usize = 60;

/// One frame's directory entry, exactly as stored at `index * 8` from the start
/// of `TB.BIG`: little-endian `{offset, size}` into the archive file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanoramaDirEntry {
    pub offset: u32,
    pub size: u32,
}

/// The 10-byte header at the start of each frame chunk. The first 8 bytes are
/// the **eye-orb's clickable rectangle `{x, y, w, h}`** in this frame (field
/// order proven by the hit test at `0x8269`: `cmp ax, word ptr [si]` @0x8274 is
/// mouse x (`[0xA2A]`) against the box x, `sub ax, word ptr [si + 4]` @0x8278
/// subtracts w, then `[si+2]`/`[si+6]` repeat it for y/h with mouse y
/// (`[0xA2C]`) — so the box is inclusive, x <= mx <= x+w. The whole test is
/// gated by `test byte ptr [0xa3e], 1` @0x826A, the same mouse-present flag the
/// choice box uses for its selected-row colour.
///
/// Copied by `0x981B` into the station table at DS:0x2A1B + station * 0x18 +
/// 0x0C. Both halves verified: the RESET at 0x985F..0x9875 does `mov cx, 4` /
/// `add di, 0xc` / two `stosd` of 0xFFFFFFFF / `add di, 4` — 0xC + 8 + 4 = the
/// 0x18 stride, so all four stations' boxes are blanked; the COPY at 0x9877
/// does `mov ax, word ptr [si + 8]` (the ninth/tenth byte — the station word),
/// `mov dx, 0x18` / `mul dx` / `add ax, 0xc`. A box of all-0xFFFF (frames 21,
/// 64, 71, …) is therefore the RESET value left in place: "no orb here".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PanoramaFrameHeader {
    pub box_x: u16,
    pub box_y: u16,
    pub box_width: u16,
    pub box_height: u16,
    /// Which of the four bridge stations this frame belongs to (0..=3):
    /// 0 = wide helm view (frames 0..=21 and 160..=179 — the sector wraps),
    /// 1 = golden console menu (22..=71), 2 = pyramid navigation room
    /// (72..=107), 3 = organic Orxx mass (108..=159).
    pub station: u16,
}

/// The parsed `TB.BIG` archive: the raw file plus its decoded directory.
pub struct BridgePanorama {
    data: Vec<u8>,
    directory: Vec<PanoramaDirEntry>,
}

impl BridgePanorama {
    /// Parse the archive. The directory is the contiguous run of
    /// `{offset, size}` pairs from file start; its length is implied by the
    /// first frame's offset (0x5A0 = 180 entries in the shipped file), exactly
    /// how `0x981B` treats the file (it never range-checks the index — the
    /// caller's frame arithmetic keeps it in 0..180).
    pub fn parse(data: Vec<u8>) -> Option<BridgePanorama> {
        if data.len() < 8 {
            return None;
        }
        let first_offset = u32::from_le_bytes(data[0..4].try_into().unwrap());
        let count = (first_offset / 8) as usize;
        if count == 0 {
            return None;
        }
        let mut directory = Vec::with_capacity(count);
        for index in 0..count {
            let at = index * 8;
            let entry = PanoramaDirEntry {
                offset: u32::from_le_bytes(data.get(at..at + 4)?.try_into().unwrap()),
                size: u32::from_le_bytes(data.get(at + 4..at + 8)?.try_into().unwrap()),
            };
            if entry.offset as usize + entry.size as usize > data.len() {
                return None;
            }
            directory.push(entry);
        }
        Some(BridgePanorama { data, directory })
    }

    pub fn frame_count(&self) -> usize {
        self.directory.len()
    }

    /// The raw chunk bytes of one frame (header + RLE stream).
    fn chunk(&self, frame: usize) -> Option<&[u8]> {
        let entry = self.directory.get(frame)?;
        self.data
            .get(entry.offset as usize..(entry.offset + entry.size) as usize)
    }

    /// Read a frame chunk's 10-byte header into [`PanoramaFrameHeader`].
    ///
    /// The field ORDER is not a guess and not a layout inferred from the data —
    /// it is the game's own hit test, verified in audit-fixes #388: `cmp ax,
    /// word ptr [si]` @`0x8274` compares mouse x (`[0xA2A]`) against `[si]`,
    /// `sub ax, word ptr [si + 4]` @`0x8278` subtracts the width, and `[si+2]` /
    /// `[si+6]` repeat it for y/h against mouse y (`[0xA2C]`). The station word
    /// at `+8` is what `mov ax, word ptr [si + 8]` @`0x987A` multiplies by the
    /// `0x18` station-table stride.
    ///
    /// The 10-byte minimum below is that layout's extent: four box words plus
    /// the station word.
    pub fn frame_header(&self, frame: usize) -> Option<PanoramaFrameHeader> {
        let chunk = self.chunk(frame)?;
        if chunk.len() < 10 {
            return None;
        }
        let word = |at: usize| u16::from_le_bytes([chunk[at], chunk[at + 1]]);
        Some(PanoramaFrameHeader {
            box_x: word(0),
            box_y: word(2),
            box_width: word(4),
            box_height: word(6),
            station: word(8),
        })
    }

    /// Unpack one frame over `screen` (a 320x200 palette-index buffer),
    /// mirroring `0x2D50` exactly. `transparent` selects the DS:0x5B57 bit-0
    /// variant: when set, colour 0 skips (the underlying pixel — window
    /// starfield, previous frame — shows through); when clear, colour 0 is
    /// written like any other colour.
    ///
    /// Stream format (signed control byte, then decode until 64000 pixels are
    /// emitted): `ctrl < 0` = run of `-ctrl + 1` copies of the next byte;
    /// `ctrl >= 0` = `ctrl + 1` literal bytes.
    pub fn unpack_frame_over(
        &self,
        frame: usize,
        screen: &mut [u8],
        transparent: bool,
    ) -> Option<PanoramaFrameHeader> {
        let header = self.frame_header(frame)?;
        let chunk = self.chunk(frame)?;
        if screen.len() < PANORAMA_FRAME_PIXELS {
            return None;
        }
        let mut src = 10usize; // past the 10-byte header, where 0x981B leaves si
        let mut dst = 0usize;
        let mut remaining = PANORAMA_FRAME_PIXELS as isize;
        while remaining > 0 {
            let ctrl = *chunk.get(src)? as i8;
            src += 1;
            if ctrl < 0 {
                let count = (-(ctrl as i16) + 1) as usize;
                let value = *chunk.get(src)?;
                src += 1;
                remaining -= count as isize;
                if transparent && value == 0 {
                    dst += count;
                } else {
                    screen.get_mut(dst..dst + count)?.fill(value);
                    dst += count;
                }
            } else {
                let count = ctrl as usize + 1;
                remaining -= count as isize;
                let literals = chunk.get(src..src + count)?;
                if transparent {
                    for &value in literals {
                        if value != 0 {
                            screen[dst] = value;
                        }
                        dst += 1;
                    }
                } else {
                    screen[dst..dst + count].copy_from_slice(literals);
                    dst += count;
                }
                src += count;
            }
        }
        Some(header)
    }

    /// Convenience: decode a frame standalone onto black (opaque variant).
    pub fn frame_pixels(&self, frame: usize) -> Option<Vec<u8>> {
        let mut screen = vec![0u8; PANORAMA_FRAME_PIXELS];
        self.unpack_frame_over(frame, &mut screen, false)?;
        Some(screen)
    }
}

/// Map a bridge view angle (DS:0x0A2A units, 1440/revolution) to its panorama
/// frame index, the inverse of `0x97E7`'s `frame * 8 - 0xA0` relation.
pub fn panorama_frame_for_angle(angle_units: u16) -> usize {
    ((angle_units % ANGLE_UNITS_PER_REVOLUTION) / ANGLE_UNITS_PER_FRAME) as usize
        % PANORAMA_FRAME_COUNT
}

#[cfg(test)]
mod tests {

    /// A CAPTURE CHECK, in the only direction the prime rule allows: the port
    /// decodes `TB.BIG` from the game's own archive, and the capture is used to
    /// confirm the output — never to derive it.
    ///
    /// `accuracy/captures/bridge/script2_first_frame.ppm` measures as 320x200 with
    /// its top 40 rows a single colour. Nothing in the decode says "40": the
    /// panorama is full-screen ([`PANORAMA_FRAME_PIXELS`]) and that band is the
    /// frame's own CONTENT. So the falsifiable claim is about the archive: some
    /// decoded frame must open with a long run of one index.
    #[test]
    fn some_panorama_frame_opens_with_a_flat_band_like_the_capture() {
        let mut path = std::path::PathBuf::from("accuracy/cdrive/cblood/TB.BIG");
        if !path.exists() {
            path = std::path::PathBuf::from("../accuracy/cdrive/cblood/TB.BIG");
        }
        let Ok(bytes) = std::fs::read(&path) else { return };
        let Some(archive) = BridgePanorama::parse(bytes) else { return };
        assert!(archive.frame_count() > 0);

        // How many leading rows are a single palette index, per frame.
        let flat_rows = |pixels: &[u8]| -> usize {
            let first = pixels[0];
            (0..200)
                .take_while(|y| pixels[y * 320..(y + 1) * 320].iter().all(|&p| p == first))
                .count()
        };

        let mut best = 0usize;
        for frame in 0..archive.frame_count() {
            if let Some(pixels) = archive.frame_pixels(frame) {
                if pixels.len() == PANORAMA_FRAME_PIXELS {
                    best = best.max(flat_rows(&pixels));
                }
            }
        }
        assert!(
            best >= 40,
            "no decoded frame opens with 40+ uniform rows (best {best}); the \
             capture shows the real game does, so the decoder or the archive \
             index is wrong"
        );
    }

    /// The angle-per-frame constant is a QUOTIENT of two cited facts, and it comes
    /// out exact — which is the check: a wrong revolution or frame count would
    /// leave a remainder.
    #[test]
    fn angle_units_per_frame_divides_the_revolution_exactly() {
        assert_eq!(
            ANGLE_UNITS_PER_REVOLUTION as usize % PANORAMA_FRAME_COUNT,
            0,
            "0x5A0 must divide 180 frames without remainder"
        );
        assert_eq!(
            ANGLE_UNITS_PER_REVOLUTION as usize / PANORAMA_FRAME_COUNT,
            ANGLE_UNITS_PER_FRAME as usize
        );
        // And the station rest angles are whole frames under the same division:
        // 0x000/0x05A/0x0B4/0x10E are degrees, half of each is the frame (#107).
        for angle in [0x000u16, 0x05A, 0x0B4, 0x10E] {
            assert_eq!(angle % 2, 0, "rest angle {angle:#x} halves to a whole frame");
        }
    }
    use super::*;
    use std::path::Path;

    fn load_real_archive() -> Option<BridgePanorama> {
        let path = Path::new("output/_tmp_iso/TB.BIG");
        if !path.exists() {
            return None;
        }
        BridgePanorama::parse(std::fs::read(path).unwrap())
    }

    #[test]
    fn parses_directory_of_180_frames() {
        let Some(pan) = load_real_archive() else { return };
        assert_eq!(pan.frame_count(), PANORAMA_FRAME_COUNT);
    }

    #[test]
    fn every_frame_unpacks_to_exactly_one_screen() {
        let Some(pan) = load_real_archive() else { return };
        for frame in 0..pan.frame_count() {
            let mut screen = vec![0u8; PANORAMA_FRAME_PIXELS];
            let header = pan
                .unpack_frame_over(frame, &mut screen, false)
                .unwrap_or_else(|| panic!("frame {frame} failed to unpack"));
            assert!(header.station < 4, "frame {frame} station {}", header.station);
        }
    }

    #[test]
    fn frame_zero_header_matches_binary_observation() {
        let Some(pan) = load_real_archive() else { return };
        // Directly observed values of the first chunk's header: the orange
        // eye-orb of the wide helm view sits at x 133..184, y 130..174.
        assert_eq!(
            pan.frame_header(0).unwrap(),
            PanoramaFrameHeader {
                box_x: 133,
                box_y: 130,
                box_width: 51,
                box_height: 44,
                station: 0
            }
        );
    }

    #[test]
    fn station_sectors_partition_the_ring() {
        let Some(pan) = load_real_archive() else { return };
        let stations: Vec<u16> = (0..pan.frame_count())
            .map(|f| pan.frame_header(f).unwrap().station)
            .collect();
        assert!(stations[0..=21].iter().all(|&s| s == 0));
        assert!(stations[22..=71].iter().all(|&s| s == 1));
        assert!(stations[72..=107].iter().all(|&s| s == 2));
        assert!(stations[108..=159].iter().all(|&s| s == 3));
        assert!(stations[160..].iter().all(|&s| s == 0));
    }

    /// The decisive faithfulness check: our decode of the console rest frame
    /// must match the REAL game's console screen as captured from the recomp
    /// emulator running the original BLOODPRG.EXE (`BRIDGEPROBE`). The small
    /// residual is the pointing-hand mouse cursor and the window starfield the
    /// game draws over the panorama.
    #[test]
    fn console_rest_frame_matches_live_game_capture() {
        let Some(pan) = load_real_archive() else { return };
        let capture_path = Path::new("accuracy/captures/bridge/console_rest.ppm");
        if !capture_path.exists() {
            return;
        }
        let capture = read_ppm_320x200(&std::fs::read(capture_path).unwrap());

        let indices = pan.frame_pixels(CONSOLE_REST_FRAME).unwrap();
        let dac = &crate::palette::GAME_SCREEN_PALETTE_DAC;
        let expand = |v: u8| (v << 2) | (v >> 4); // 6-bit DAC -> 8-bit
        let mut total_abs = 0u64;
        for (pixel, &index) in indices.iter().enumerate() {
            for channel in 0..3 {
                let ours = expand(dac[index as usize * 3 + channel]);
                let live = capture[pixel * 3 + channel];
                total_abs += (ours as i32 - live as i32).unsigned_abs() as u64;
            }
        }
        let mean_abs = total_abs as f64 / (PANORAMA_FRAME_PIXELS * 3) as f64;
        assert!(
            mean_abs < 3.0,
            "console frame diverges from the live game: mean_abs = {mean_abs:.2}"
        );
    }

    /// Oracle: decoding an arbitrary steered frame must match the live game at
    /// that view too — frames 15 (mouse at the left screen edge) and 64 (right
    /// edge) were captured live (`BRIDGEPROBE` steering). The residual is the
    /// hand cursor + window starfield the game overlays; a small threshold
    /// catches any decode regression across the ring, not just the rest frame.
    #[test]
    fn steered_frames_match_live_game_captures() {
        let Some(pan) = load_real_archive() else { return };
        let dac = &crate::palette::GAME_SCREEN_PALETTE_DAC;
        let expand = |v: u8| (v << 2) | (v >> 4);
        for (frame, capture_name) in [(15usize, "rotate_left"), (64, "rotate_right")] {
            let path = format!("accuracy/captures/bridge/{capture_name}.ppm");
            let path = Path::new(&path);
            if !path.exists() {
                continue;
            }
            let capture = read_ppm_320x200(&std::fs::read(path).unwrap());
            let indices = pan.frame_pixels(frame).unwrap();
            let mut total_abs = 0u64;
            for (pixel, &index) in indices.iter().enumerate() {
                for channel in 0..3 {
                    let ours = expand(dac[index as usize * 3 + channel]);
                    let live = capture[pixel * 3 + channel];
                    total_abs += (ours as i32 - live as i32).unsigned_abs() as u64;
                }
            }
            let mean_abs = total_abs as f64 / (PANORAMA_FRAME_PIXELS * 3) as f64;
            assert!(
                mean_abs < 5.0,
                "frame {frame} ({capture_name}) diverges: mean_abs = {mean_abs:.2}"
            );
        }
    }

    /// Minimal P6 reader for the emulator's fixed-format 320x200 captures.
    fn read_ppm_320x200(raw: &[u8]) -> Vec<u8> {
        let header_end = raw
            .windows(4)
            .position(|w| w == b"255\n")
            .expect("PPM maxval")
            + 4;
        let body = &raw[header_end..];
        assert_eq!(body.len(), PANORAMA_FRAME_PIXELS * 3);
        body.to_vec()
    }
}
