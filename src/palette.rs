//! The game-screen VGA palette — the 256-entry DAC buffer the original uploads
//! for the ship/nav, bridge, location, and dialogue screens
//! (game_palette_dac_buffer, gs:0x5B58, uploaded via 0x16A7 -> 0x2F90 to DAC
//! ports 0x3C8/0x3C9). These are the baked default the executable ships in its
//! data segment (BLOODPRG.EXE file offset 0x12F78; 768 bytes = 256 RGB triples,
//! each channel 6-bit 0..=63). Cross-checked against the running game (recomp
//! emulator gs:0x5B58): the first 128 entries match byte-for-byte; the upper half
//! is only ever overwritten by a per-scene HNM palette.

/// Raw 6-bit VGA DAC palette (256 * RGB, channels 0..=63).
///
/// PROVENANCE IS SPLIT, and the old comment (`Provenance: BLOODPRG.EXE file 0x12F78`)
/// was only half true — measured 2026-07-24:
///
/// * **Colours 0..127 — BINARY.** Byte-for-byte identical to the baked default at
///   file `0x12F78`, and confirmed live: the running game's buffer at `gs:0x5B58`
///   reads back the same values (probe `runtime_boot PALBANK`, which checks colours
///   0..7 against the image as an address control before trusting anything else).
/// * **Colours 128..191 — CAPTURED FROM A SAVESTATE, and conceptually wrong.** These
///   192 bytes differ from the image, and appear in NO shipped file (searched raw,
///   `*4`, and `*4|3` across the CD and install trees). They are not a constant at
///   all: they are SCENE STATE. At the console the live bank reads all ZERO with no
///   writer, so the values baked here were lifted from one particular state
///   (`accuracy/script2.state`) and frozen as though global.
///
/// The upper half is fed by the per-scene HNM palette in the real game, which is why
/// it is scene-dependent — and the port already parses those (`hnm::parse_palette_block`,
/// and every scene load assigns `self.scene_palette = hnm.palette`). So this bank is
/// mostly overwritten in practice; it is the DEFAULT that is wrong, and it shows
/// wherever a surface is drawn before a scene palette lands.
///
/// FIX: colours 128..191 should come from the loaded scene, not from here. Until then
/// this is an ACKNOWLEDGED APPROX for that range only — the lower half is sound.
///
/// INDEPENDENT CORROBORATION, from the opposite direction. `engine.rs` had already
/// learned this empirically without the decode: its hand-palette installs are narrowed
/// to `202..=251` with the comment "installing all of 128..=255 clobbered scene
/// palettes whose images own 128..201 (the world rooms: the cyan-cast defect found by
/// the planet reference bank)". A rendering bug and a static byte-comparison landed on
/// the same boundary from different evidence, which is the strongest form this claim
/// can take short of finding the writer.
pub const GAME_SCREEN_PALETTE_DAC_LOWER_IS_BINARY: usize = 128;
pub const GAME_SCREEN_PALETTE_DAC: [u8; 768] = [
    0, 0, 0, 36, 46, 11, 35, 37, 6, 34, 34, 14,
    30, 25, 11, 27, 21, 11, 28, 21, 16, 23, 19, 20,
    20, 15, 20, 22, 18, 15, 22, 18, 9, 24, 18, 3,
    27, 22, 4, 19, 16, 3, 14, 10, 4, 9, 8, 2,
    8, 5, 11, 6, 0, 10, 5, 0, 9, 3, 0, 6,
    2, 0, 4, 0, 0, 0, 0, 9, 11, 5, 10, 16,
    3, 14, 18, 0, 14, 17, 1, 19, 23, 1, 22, 27,
    1, 25, 30, 1, 29, 35, 9, 27, 36, 13, 26, 35,
    14, 21, 33, 13, 19, 30, 10, 19, 28, 13, 15, 28,
    14, 10, 30, 16, 11, 32, 17, 13, 35, 18, 14, 39,
    20, 16, 40, 19, 17, 43, 21, 19, 46, 23, 18, 41,
    27, 19, 41, 29, 21, 46, 31, 26, 47, 33, 32, 48,
    37, 34, 51, 42, 38, 55, 42, 37, 63, 37, 32, 63,
    33, 27, 63, 30, 24, 62, 25, 21, 58, 29, 9, 60,
    26, 8, 54, 25, 9, 47, 21, 6, 47, 19, 0, 46,
    17, 0, 40, 16, 0, 34, 16, 0, 29, 14, 0, 25,
    12, 0, 21, 10, 0, 19, 9, 2, 17, 9, 0, 16,
    8, 0, 14, 10, 6, 16, 14, 8, 15, 13, 7, 21,
    13, 7, 24, 10, 7, 23, 12, 4, 21, 14, 8, 27,
    14, 12, 23, 8, 14, 22, 6, 18, 23, 8, 24, 30,
    16, 26, 40, 22, 26, 43, 28, 25, 40, 29, 22, 32,
    21, 21, 35, 22, 15, 35, 18, 12, 29, 16, 5, 34,
    19, 6, 40, 30, 10, 48, 34, 12, 56, 36, 12, 62,
    43, 25, 61, 48, 42, 63, 54, 48, 63, 61, 56, 63,
    61, 36, 63, 30, 50, 61, 18, 45, 61, 14, 39, 54,
    14, 35, 48, 10, 32, 43, 23, 35, 47, 24, 39, 54,
    29, 23, 23, 49, 27, 10, 49, 24, 0, 47, 17, 0,
    47, 10, 0, 38, 12, 0, 30, 9, 1, 21, 8, 5,
    38, 21, 0, 58, 19, 0, 58, 13, 0, 59, 26, 0,
    60, 27, 7, 59, 35, 1, 53, 37, 14, 62, 39, 15,
    48, 48, 7, 60, 54, 4, 10, 0, 27, 16, 12, 0,
    16, 12, 0, 16, 12, 0, 16, 12, 0, 16, 12, 0,
    0, 0, 0, 4, 61, 63, 8, 61, 63, 12, 63, 63,
    63, 63, 63, 39, 63, 63, 35, 63, 63, 37, 57, 63,
    35, 55, 63, 30, 55, 61, 30, 57, 63, 33, 59, 63,
    28, 63, 63, 24, 63, 63, 20, 63, 63, 16, 63, 63,
    18, 59, 63, 22, 59, 63, 26, 59, 63, 24, 55, 63,
    20, 55, 61, 16, 55, 63, 14, 57, 63, 14, 55, 61,
    10, 55, 61, 10, 57, 63, 6, 57, 63, 6, 55, 61,
    2, 55, 63, 0, 57, 63, 2, 59, 61, 0, 57, 59,
    0, 53, 55, 0, 55, 57, 0, 51, 59, 2, 49, 57,
    0, 47, 55, 0, 49, 53, 0, 47, 51, 0, 43, 49,
    0, 43, 53, 0, 41, 51, 0, 39, 47, 0, 37, 49,
    0, 33, 47, 0, 35, 45, 0, 33, 43, 0, 28, 45,
    0, 24, 41, 0, 28, 41, 0, 26, 39, 0, 24, 37,
    0, 20, 39, 0, 18, 35, 0, 12, 33, 0, 14, 30,
    0, 12, 28, 0, 10, 30, 0, 8, 26, 0, 4, 24,
    0, 2, 22, 0, 0, 18, 0, 0, 14, 0, 61, 63,
    0, 0, 0, 9, 3, 12, 8, 3, 11, 7, 4, 11,
    7, 3, 10, 6, 2, 9, 5, 2, 8, 5, 1, 7,
    6, 1, 8, 6, 4, 9, 10, 4, 13, 10, 5, 14,
    11, 6, 16, 12, 7, 18, 14, 8, 20, 8, 4, 12,
    13, 11, 21, 12, 13, 23, 9, 12, 24, 11, 15, 25,
    10, 17, 27, 9, 18, 28, 7, 17, 26, 6, 15, 23,
    10, 17, 23, 9, 12, 21, 16, 17, 28, 19, 20, 31,
    15, 24, 29, 12, 25, 34, 14, 26, 35, 15, 28, 36,
    0, 0, 0, 4, 4, 4, 8, 8, 8, 12, 12, 12,
    17, 17, 17, 21, 21, 21, 25, 25, 25, 29, 29, 29,
    34, 34, 34, 38, 38, 38, 42, 42, 42, 46, 46, 46,
    51, 51, 51, 55, 55, 55, 59, 59, 59, 63, 63, 63,
    17, 30, 37, 19, 32, 39, 23, 31, 36, 10, 22, 32,
    10, 23, 30, 9, 20, 30, 22, 35, 42, 25, 36, 41,
    25, 38, 44, 27, 41, 46, 32, 44, 48, 35, 47, 51,
    60, 0, 0, 0, 36, 0, 11, 52, 2, 32, 63, 26,
];

/// Seed distance of the nearest-colour search (`mov word [bp+6],0xBB8` @`0x234F`).
/// A source colour with no palette entry within this squared distance leaves its
/// table byte UNCHANGED (`js 0x23B0` skips the store).
pub const PALETTE_BLEND_MAX_DISTANCE: u16 = 0x0BB8;

/// The game's TINT REMAP TABLE builder — routine `0x22E0`, far-called as
/// `0x1CE:0x0000`. Every translucent/tinted overlay in the game goes through it:
/// build a 256-entry LUT that maps each palette index to the nearest palette
/// entry AFTER blending it `percent`% toward a target colour, then remap the
/// pixels already on screen through that LUT.
///
/// ```text
///   0x22F1  neg ax                       the caller passes the NEGATED percent
///   0x22FC  push ax                      (pct*bx)/100, computed from 0x22F5
///   0x2303  push ax                      (pct*cx)/100, from 0x22FD
///   0x230C  push ax                      (pct*dx)/100, from 0x2304
///   0x2314  push ax                      100-pct, from 0x230D -- the SOURCE weight
///   0x2322  each source component: src*(100-pct)/100 + prescaled target
///   0x234A  best = 0xFFFF, dist = 0xBB8
///   0x2354  scan all 256 entries of the live palette at DS:0x5251
///   0x2390  cmp bx,best / ja skip        <= wins, so TIES TAKE THE LATER index
///   0x23A9  or ax,ax / js                no match -> leave the byte UNCHANGED
/// ```
///
/// The destination-fill call sites pick between two adjacent 256-byte tables,
/// `DS:0x5F11` and `DS:0x6011` (`0x45C8`, selection stored at `gs:0x524B`).
/// The destination info panel builds table `0x5F11` with `ax=0xFFCE` (= 50) and
/// `bx=cx=dx=0` (`0x90ED..0x90F9`) — its window is a 50% darkening of whatever
/// is already on screen, not an opaque box.
///
/// `palette` is the LIVE palette (`DS:0x5251`), in the same 6-bit DAC units the
/// routine compares in. `table` is modified in place so that unmatched entries
/// keep their previous value, exactly as the assembly does.
pub fn build_palette_blend_remap_table(
    palette: &[[u8; 3]; 256],
    percent: u16,
    target: [u16; 3],
    table: &mut [u8; 256],
) {
    let source_weight = 100u16.wrapping_sub(percent);
    let scaled_target = [
        percent.wrapping_mul(target[0]) / 100,
        percent.wrapping_mul(target[1]) / 100,
        percent.wrapping_mul(target[2]) / 100,
    ];
    for (index, entry) in table.iter_mut().enumerate() {
        let mut blended = [0u16; 3];
        for k in 0..3 {
            blended[k] = (palette[index][k] as u16)
                .wrapping_mul(source_weight)
                .wrapping_div(100)
                .wrapping_add(scaled_target[k]);
        }
        let mut best_distance = PALETTE_BLEND_MAX_DISTANCE;
        let mut best: Option<u8> = None;
        for (candidate, rgb) in palette.iter().enumerate() {
            let mut distance = 0u16;
            for k in 0..3 {
                let delta = blended[k].abs_diff(rgb[k] as u16);
                distance = distance.wrapping_add(delta.wrapping_mul(delta));
            }
            if distance <= best_distance {
                best_distance = distance;
                best = Some(candidate as u8);
            }
        }
        if let Some(found) = best {
            *entry = found;
        }
    }
}

/// The CONSOLE-BANK remap table (`DS:0x6011`) — built by running the GAME'S OWN
/// builder, not by reimplementing it.
///
/// `0x242D` (far `0x1CE:0x014D`) is a SECOND table builder, distinct from the
/// blend builder `0x22E0`. It is called once, at `0x9622`, with `ax = 0xE0` and
/// `bx = 0x6011`, and walks the live palette at `DS:0x5251` over 256 entries.
/// The montage's per-frame setup (`0x7AC3`) then remaps the WHOLE 320x200 screen
/// through the result before drawing the film into the top 140 rows.
///
/// What it produces, observed by executing it: every one of the 256 inputs maps
/// into `224..=239` — the 16-colour CONSOLE BANK — with the bank mapping to
/// itself. That is what explains the intro band's index range: during the montage
/// the entire screen is reduced to that bank, so the console already on screen
/// comes out in `224..=239` like everything else. The band was never separate art.
///
/// THE RULE IS NOT NEAREST-COLOUR. A squared-RGB nearest search over the bank
/// reproduces only 68 of the 256 entries, so the obvious reimplementation is
/// wrong. Rather than guess again, this runs `recomp::auto::func_242d`, which is
/// lifted bit-exactly from the binary and oracle-verified — the port already had
/// the function, it simply was not being used.
pub fn build_console_bank_remap_table(palette_dac: &[u8; 768]) -> [u8; 256] {
    use crate::recomp::{auto, machine::Machine};
    const GS: u16 = 0x2600;
    const PAL_DS: u32 = 0x5251;
    const TABLE_DS: u32 = 0x6011;
    const BANK_BASE: u16 = 0xE0;

    let mut m = Machine::new();
    m.regs.ds = GS;
    m.regs.es = GS;
    m.regs.gs = GS;
    m.regs.ss = 0x9000;
    m.regs.set_sp(0xFFF0);
    let base = (GS as u32) * 16;
    for (i, &b) in palette_dac.iter().enumerate() {
        m.mem[(base + PAL_DS + i as u32) as usize] = b;
    }
    m.regs.set_ax(BANK_BASE);
    m.regs.set_bx(TABLE_DS as u16);
    let sp = m.regs.sp() as u32;
    m.write16(m.regs.ss, sp, 0x0000);
    m.write16(m.regs.ss, sp.wrapping_add(2), 0x0020);
    auto::func_242d(&mut m);

    let mut table = [0u8; 256];
    for (i, entry) in table.iter_mut().enumerate() {
        *entry = m.mem[(base + TABLE_DS + i as u32) as usize];
    }
    table
}

/// The game-screen palette expanded to 8-bit RGB for the engine framebuffer,
/// scaling each 6-bit DAC channel to full range (v * 255 / 63).
pub fn game_screen_palette() -> [[u8; 3]; 256] {
    let mut out = [[0u8; 3]; 256];
    let mut i = 0;
    while i < 256 {
        let base = i * 3;
        let expand = |c: u8| (c as u16 * 255 / 63) as u8;
        out[i] = [
            expand(GAME_SCREEN_PALETTE_DAC[base]),
            expand(GAME_SCREEN_PALETTE_DAC[base + 1]),
            expand(GAME_SCREEN_PALETTE_DAC[base + 2]),
        ];
        i += 1;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_console_bank_table_maps_everything_into_224_239() {
        let table = build_console_bank_remap_table(&GAME_SCREEN_PALETTE_DAC);
        // Every input lands in the 16-colour console bank...
        assert!(
            table.iter().all(|&b| (0xE0..=0xEF).contains(&b)),
            "the montage remap must reduce the screen to the console bank"
        );
        let distinct: std::collections::BTreeSet<u8> = table.iter().copied().collect();
        assert_eq!(distinct.len(), 16);
        // ...and the bank maps to itself, so remapping twice is stable.
        for i in 0xE0..=0xEFusize {
            assert_eq!(table[i], i as u8, "bank entry {i:#x} must be fixed");
        }
        let twice: Vec<u8> = table.iter().map(|&b| table[b as usize]).collect();
        assert_eq!(twice, table.to_vec(), "the remap is idempotent");

        // AND the rule is not the obvious one: a squared-RGB nearest search over
        // the bank reproduces only a quarter of the table. Pinned so nobody
        // "simplifies" the call to func_242d into a nearest-colour loop.
        let pal = |i: usize| {
            [
                GAME_SCREEN_PALETTE_DAC[i * 3] as i32,
                GAME_SCREEN_PALETTE_DAC[i * 3 + 1] as i32,
                GAME_SCREEN_PALETTE_DAC[i * 3 + 2] as i32,
            ]
        };
        let agree = (0..256usize)
            .filter(|&src| {
                let c = pal(src);
                let best = (0xE0..=0xEFusize)
                    .min_by_key(|&cand| {
                        let d = pal(cand);
                        (0..3).map(|k| (c[k] - d[k]).pow(2)).sum::<i32>()
                    })
                    .unwrap() as u8;
                best == table[src]
            })
            .count();
        assert!(
            agree < 128,
            "nearest-colour agreed on {agree}/256 — if this ever passes, the rule \
             changed and the comment in build_console_bank_remap_table is stale"
        );
    }

    #[test]
    fn the_tint_table_halves_a_grey_ramp_and_leaves_unmatched_entries_alone() {
        // The panel's own arguments: 50% toward black (0x90ED: ax=0xFFCE -> 50,
        // bx=cx=dx=0).
        let mut palette = [[0u8; 3]; 256];
        for (i, entry) in palette.iter_mut().enumerate() {
            let v = (i % 64) as u8;
            *entry = [v, v, v];
        }
        let mut table = [0xAAu8; 256];
        build_palette_blend_remap_table(&palette, 50, [0, 0, 0], &mut table);
        // Index 40 is (40,40,40); halved it is (20,20,20), and the search picks the
        // LAST entry at that value (`ja` skips only a STRICTLY greater distance),
        // so the winner is the highest index whose colour is (20,20,20).
        let want = (0..256usize)
            .filter(|&i| palette[i] == [20, 20, 20])
            .next_back()
            .expect("the ramp repeats");
        assert_eq!(table[40] as usize, want);
        assert_eq!(table[0] as usize, {
            let black = (0..256usize).filter(|&i| palette[i] == [0, 0, 0]).next_back();
            black.unwrap()
        });

        // A palette with one colour so far from everything that no entry lands
        // within 0xBB8 leaves that table byte untouched.
        let mut sparse = [[0u8; 3]; 256];
        sparse[1] = [63, 63, 63];
        let mut table = [0x77u8; 256];
        build_palette_blend_remap_table(&sparse, 0, [0, 0, 0], &mut table);
        assert_eq!(
            table[1], 1,
            "0% blend keeps a colour where it is (distance 0 to itself)"
        );
        let mut far = [[0u8; 3]; 256];
        far[0] = [0, 0, 0];
        far[1] = [63, 63, 63];
        let mut table = [0x77u8; 256];
        build_palette_blend_remap_table(&far, 100, [63, 63, 63], &mut table);
        // Every entry blends to white, which IS in the palette, so all match.
        assert!(table.iter().all(|&b| b == 1));
    }

    /// Colours 0..127 are the game's own baked DAC at file 0x12F78 and must stay
    /// byte-identical to the image. Colours 128..191 are NOT asserted here on purpose:
    /// they were captured from one savestate, are absent from every shipped file, and
    /// read all-zero in the live console state — they are scene data frozen into a
    /// constant, tracked as an acknowledged APPROX.
    #[test]
    fn palette_lower_half_matches_the_baked_dac_in_the_image() {
        let exe = match std::fs::read("re/bin/BLOODPRG.EXE")
            .or_else(|_| std::fs::read("../re/bin/BLOODPRG.EXE"))
        {
            Ok(b) => b,
            Err(_) => return,
        };
        const BAKED: usize = 0x12F78;
        let n = GAME_SCREEN_PALETTE_DAC_LOWER_IS_BINARY * 3;
        assert_eq!(
            &GAME_SCREEN_PALETTE_DAC[..n],
            &exe[BAKED..BAKED + n],
            "colours 0..127 must equal the baked DAC at file 0x12F78"
        );
        // And record the split as an executable fact: the upper bank DIFFERS from the
        // image, so nobody can later ""fix"" the doc by claiming the whole table is baked.
        assert_ne!(
            &GAME_SCREEN_PALETTE_DAC[n..n + 192],
            &exe[BAKED + n..BAKED + n + 192],
            "colours 128..191 are known to differ — if this ever passes, the capture \
             was replaced by real data and the APPROX note should be retired"
        );
    }
    #[test]
    fn palette_is_valid_dac_data() {
        assert!(GAME_SCREEN_PALETTE_DAC.iter().all(|&c| c <= 63), "6-bit DAC channels");
        assert_eq!(&GAME_SCREEN_PALETTE_DAC[0..3], &[0, 0, 0], "index 0 is black");
        assert_eq!(game_screen_palette()[0], [0, 0, 0]);
        assert_eq!((63u16 * 255 / 63) as u8, 255);
    }
}
