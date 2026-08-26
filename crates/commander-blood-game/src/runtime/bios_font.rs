//! Owned VGA BIOS font used by the original fixed-width text renderer.

use crate::native::bloodprg::BiosFont8x8;

const BIOS_FONT_CHARACTER_COUNT: usize = 256;
const BIOS_FONT_GLYPH_HEIGHT: usize = 8;
const BIOS_FONT_BYTE_COUNT: usize = BIOS_FONT_CHARACTER_COUNT * BIOS_FONT_GLYPH_HEIGHT;
const RAW_VGA_BIOS_FONT: &[u8; BIOS_FONT_BYTE_COUNT] =
    include_bytes!("../../assets/vga-bios-8x8.bin");

/// Complete VGA 8x8 ROM font captured through the same BIOS calls as BLOODPRG.
pub const VGA_BIOS_FONT_8X8: BiosFont8x8 = decode_bios_font(RAW_VGA_BIOS_FONT);

const fn decode_bios_font(bytes: &[u8; BIOS_FONT_BYTE_COUNT]) -> BiosFont8x8 {
    let mut font = [[u8::MIN; BIOS_FONT_GLYPH_HEIGHT]; BIOS_FONT_CHARACTER_COUNT];
    let mut character = usize::MIN;
    while character < BIOS_FONT_CHARACTER_COUNT {
        let mut row = usize::MIN;
        while row < BIOS_FONT_GLYPH_HEIGHT {
            font[character][row] = bytes[character * BIOS_FONT_GLYPH_HEIGHT + row];
            row += 1;
        }
        character += 1;
    }
    font
}

#[cfg(test)]
mod tests {
    use sha2::{Digest, Sha256};

    use super::*;

    const REFERENCE_FONT_SHA256: &str =
        "75c79a7e7fa423dda67ec6d6d76cec86b63f85677726368750c75b0920ddf319";

    #[test]
    fn embedded_font_matches_both_reference_emulators() {
        let digest = Sha256::digest(RAW_VGA_BIOS_FONT);
        assert_eq!(format!("{digest:x}"), REFERENCE_FONT_SHA256);
        assert_eq!(
            VGA_BIOS_FONT_8X8[b'L' as usize],
            [240, 96, 96, 96, 98, 102, 254, 0]
        );
    }
}
