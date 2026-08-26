//! Palette resources embedded in `BLOODPRG.EXE`.

use crate::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

/// First reserved palette index used by the MANU3 hand texture.
pub const MANU3_PALETTE_START: usize = 202;
/// Last reserved palette index installed for the MANU3 hand texture.
pub const MANU3_PALETTE_END: usize = 251;

const DEFAULT_PALETTE_FILE_OFFSET: usize = 0x12f78;
const DEFAULT_PALETTE_BYTE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
const VGA_DAC_CHANNEL_MAXIMUM: u16 = 63;
const EIGHT_BIT_CHANNEL_MAXIMUM: u16 = 255;

/// Decode the executable's 256-entry palette without changing its six-bit VGA DAC values.
pub fn decode_bloodprg_default_vga_palette(
    executable: &[u8],
) -> Option<[[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT]> {
    let bytes = executable.get(
        DEFAULT_PALETTE_FILE_OFFSET
            ..DEFAULT_PALETTE_FILE_OFFSET.checked_add(DEFAULT_PALETTE_BYTE_COUNT)?,
    )?;
    let mut palette = [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT];
    for (entry, source) in palette
        .iter_mut()
        .zip(bytes.chunks_exact(RGB_COMPONENT_COUNT))
    {
        for (component, value) in entry.iter_mut().zip(source) {
            if u16::from(*value) > VGA_DAC_CHANNEL_MAXIMUM {
                return None;
            }
            *component = *value;
        }
    }
    Some(palette)
}

/// Decode the executable's 256-entry default VGA DAC palette and expand each
/// six-bit channel to an eight-bit color component.
pub fn decode_bloodprg_default_palette(
    executable: &[u8],
) -> Option<[[u8; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT]> {
    let mut palette = decode_bloodprg_default_vga_palette(executable)?;
    for entry in &mut palette {
        for component in entry {
            *component =
                (u16::from(*component) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8;
        }
    }
    Some(palette)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::*;

    const LAST_MANU3_COLOR: [u8; RGB_COMPONENT_COUNT] = [141, 190, 206];

    fn original_executable() -> Option<PathBuf> {
        [
            Path::new("output/_tmp_iso/BLOODPRG.EXE"),
            Path::new("../../output/_tmp_iso/BLOODPRG.EXE"),
            Path::new("commander-blood-audio/_tmp_iso/BLOODPRG.EXE"),
            Path::new("../../commander-blood-audio/_tmp_iso/BLOODPRG.EXE"),
        ]
        .into_iter()
        .find(|path| path.is_file())
        .map(Path::to_owned)
    }

    #[test]
    fn decodes_the_original_reserved_manu3_palette_bank() {
        let Some(path) = original_executable() else {
            return;
        };
        let palette = decode_bloodprg_default_palette(&std::fs::read(path).unwrap()).unwrap();
        assert_eq!(palette[usize::MIN], [u8::MIN; RGB_COMPONENT_COUNT]);
        assert!(palette[MANU3_PALETTE_START] != [u8::MIN; RGB_COMPONENT_COUNT]);
        assert_eq!(palette[MANU3_PALETTE_END], LAST_MANU3_COLOR);
    }

    #[test]
    fn raw_palette_retains_native_six_bit_dac_components() {
        let Some(path) = original_executable() else {
            return;
        };
        let executable = std::fs::read(path).unwrap();
        let raw = decode_bloodprg_default_vga_palette(&executable).unwrap();
        let expanded = decode_bloodprg_default_palette(&executable).unwrap();

        assert!(raw.iter().flatten().all(|component| *component <= 63));
        assert!(expanded.iter().flatten().any(|component| *component > 63));
        for (raw, expanded) in raw.iter().flatten().zip(expanded.iter().flatten()) {
            assert_eq!(
                *expanded,
                (u16::from(*raw) * EIGHT_BIT_CHANNEL_MAXIMUM / VGA_DAC_CHANNEL_MAXIMUM) as u8
            );
        }
    }

    #[test]
    fn rejects_missing_or_non_dac_palette_data() {
        assert!(decode_bloodprg_default_palette(&[]).is_none());
        let mut executable =
            vec![u8::MIN; DEFAULT_PALETTE_FILE_OFFSET + DEFAULT_PALETTE_BYTE_COUNT];
        executable[DEFAULT_PALETTE_FILE_OFFSET] = VGA_DAC_CHANNEL_MAXIMUM as u8 + 1;
        assert!(decode_bloodprg_default_palette(&executable).is_none());
        assert!(decode_bloodprg_default_vga_palette(&executable).is_none());
    }
}
