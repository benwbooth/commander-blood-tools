//! Typed live-palette publication replacing direct VGA DAC writes.

use commander_blood_formats::lbm::{PALETTE_ENTRY_COUNT, RGB_COMPONENT_COUNT};

use super::IndexedGamePalette;

/// Modern renderer boundary for one complete indexed palette.
pub trait IndexedPalettePublisher {
    /// Renderer publication failure.
    type Error;

    /// Publish all authored DAC components to modern presentation state.
    fn publish_palette(&mut self, palette: &IndexedGamePalette) -> Result<(), Self::Error>;
}

/// Publish one complete live indexed palette.
///
/// This translates `vga_palette_write` at BLOODPRG offset `0x002F90`.
/// A typed palette replaces the wrapping near source pointer, and one renderer
/// operation replaces the DAC index write plus 768 byte-wide port writes.
pub fn publish_live_palette<Publisher: IndexedPalettePublisher>(
    palette: &IndexedGamePalette,
    publisher: &mut Publisher,
) -> Result<(), Publisher::Error> {
    publisher.publish_palette(palette)
}

/// Publish a completely black live indexed palette.
///
/// This translates `vga_dac_clear` at BLOODPRG offset `0x002FA6`. Palette
/// ownership remains with the renderer instead of being implicit VGA state.
pub fn clear_live_palette<Publisher: IndexedPalettePublisher>(
    publisher: &mut Publisher,
) -> Result<(), Publisher::Error> {
    publisher.publish_palette(&black_palette())
}

const fn black_palette() -> IndexedGamePalette {
    [[u8::MIN; RGB_COMPONENT_COUNT]; PALETTE_ENTRY_COUNT]
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;
    use sha2::{Digest, Sha256};

    use super::*;

    const PALETTE_VECTOR_COUNT: usize = 3;
    const DAC_INDEX_WRITE_COUNT: usize = 1;
    const DAC_COMPONENT_WRITE_COUNT: usize = PALETTE_ENTRY_COUNT * RGB_COMPONENT_COUNT;
    const SOURCE_BYTE_COUNT: usize = u16::MAX as usize + 1;
    const SOURCE_MULTIPLIER: usize = 37;
    const SOURCE_SEEDS: [usize; PALETTE_VECTOR_COUNT] = [3, 29, 101];
    const ORACLE_EDGE_COMPONENT_COUNT: usize = 12;

    #[derive(Deserialize)]
    struct PaletteOracle {
        source_offset: usize,
        write_count: usize,
        palette_head: Vec<u8>,
        palette_tail: Vec<u8>,
        palette_sha256: String,
    }

    #[derive(Deserialize)]
    struct ClearOracle {
        write_count: usize,
        zero_data_writes: usize,
    }

    #[derive(Default)]
    struct RecordingPublisher {
        palettes: Vec<IndexedGamePalette>,
    }

    impl IndexedPalettePublisher for RecordingPublisher {
        type Error = Infallible;

        fn publish_palette(&mut self, palette: &IndexedGamePalette) -> Result<(), Self::Error> {
            self.palettes.push(*palette);
            Ok(())
        }
    }

    #[test]
    fn publication_matches_every_original_dac_payload() {
        let vectors: Vec<PaletteOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2f90_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PALETTE_VECTOR_COUNT);

        for (vector, seed) in vectors.into_iter().zip(SOURCE_SEEDS) {
            let bytes = oracle_palette_bytes(vector.source_offset, seed);
            let palette = palette_from_bytes(&bytes);
            let mut publisher = RecordingPublisher::default();
            publish_live_palette(&palette, &mut publisher).unwrap();

            assert_eq!(publisher.palettes, vec![palette]);
            assert_eq!(
                vector.write_count,
                DAC_INDEX_WRITE_COUNT + DAC_COMPONENT_WRITE_COUNT
            );
            assert_eq!(
                &bytes[..ORACLE_EDGE_COMPONENT_COUNT],
                vector.palette_head.as_slice()
            );
            assert_eq!(
                &bytes[bytes.len() - ORACLE_EDGE_COMPONENT_COUNT..],
                vector.palette_tail.as_slice()
            );
            assert_eq!(
                format!("{:x}", Sha256::digest(bytes)),
                vector.palette_sha256
            );
        }
    }

    #[test]
    fn clear_matches_every_original_zero_payload() {
        let vectors: Vec<ClearOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_2fa6_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), PALETTE_VECTOR_COUNT);

        for vector in vectors {
            let mut publisher = RecordingPublisher::default();
            clear_live_palette(&mut publisher).unwrap();
            assert_eq!(publisher.palettes, vec![black_palette()]);
            assert_eq!(
                vector.write_count,
                DAC_INDEX_WRITE_COUNT + DAC_COMPONENT_WRITE_COUNT
            );
            assert_eq!(vector.zero_data_writes, DAC_COMPONENT_WRITE_COUNT);
        }
    }

    fn oracle_palette_bytes(source_offset: usize, seed: usize) -> Vec<u8> {
        let source: Vec<u8> = (0..SOURCE_BYTE_COUNT)
            .map(|index| (index * SOURCE_MULTIPLIER + seed) as u8)
            .collect();
        (0..DAC_COMPONENT_WRITE_COUNT)
            .map(|index| source[(source_offset + index) % SOURCE_BYTE_COUNT])
            .collect()
    }

    fn palette_from_bytes(bytes: &[u8]) -> IndexedGamePalette {
        let mut palette = black_palette();
        for (color, components) in palette
            .iter_mut()
            .zip(bytes.chunks_exact(RGB_COMPONENT_COUNT))
        {
            color.copy_from_slice(components);
        }
        palette
    }
}
