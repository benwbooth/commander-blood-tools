//! The game-screen VGA palette — the 256-entry DAC buffer the original uploads
//! for the ship/nav, bridge, location, and dialogue screens
//! (game_palette_dac_buffer, gs:0x5B58, uploaded via 0x16A7 -> 0x2F90 to DAC
//! ports 0x3C8/0x3C9). These are the baked default the executable ships in its
//! data segment (BLOODPRG.EXE file offset 0x12F78; 768 bytes = 256 RGB triples,
//! each channel 6-bit 0..=63). Cross-checked against the running game (recomp
//! emulator gs:0x5B58): the first 128 entries match byte-for-byte; the upper half
//! is only ever overwritten by a per-scene HNM palette.

/// Raw 6-bit VGA DAC palette (256 * RGB, channels 0..=63), exactly as the game
/// stores it. Provenance: BLOODPRG.EXE file 0x12F78.
// The HUB-state DAC captured from the interpreter oracle at accuracy/script2.state
// (INDEXDUMP probe -> accuracy/captures/hub_dac.bin). Verified: TB.BIG frame 45's raw
// indices match the live hub screen at 95.5%; entries 128..191 (the manu3 hand / orb /
// menu bank) are the values the game actually programs at the hub - the prior constant
// froze a different state's bank there (the visible "miscolored hand/menu").
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
    fn palette_is_valid_dac_data() {
        assert!(GAME_SCREEN_PALETTE_DAC.iter().all(|&c| c <= 63), "6-bit DAC channels");
        assert_eq!(&GAME_SCREEN_PALETTE_DAC[0..3], &[0, 0, 0], "index 0 is black");
        assert_eq!(game_screen_palette()[0], [0, 0, 0]);
        assert_eq!((63u16 * 255 / 63) as u8, 255);
    }
}
