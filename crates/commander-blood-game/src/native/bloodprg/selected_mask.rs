//! Number-mask overlay used by the six-choice bridge presentation.

const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
const MASK_WIDTH: usize = 16;
const MASK_HEIGHT: usize = 16;
const MASK_ORIGIN_X: usize = 5;
const MASK_ORIGIN_Y: usize = 15;
const MASK_COLOR: u8 = 254;
const FIRST_ROW_BIT: u16 = 0x8000;
const REQUIRED_FRAMEBUFFER_LEN: usize =
    (MASK_ORIGIN_Y + MASK_HEIGHT - 1) * LOGICAL_FRAMEBUFFER_WIDTH + MASK_ORIGIN_X + MASK_WIDTH;

const PRESENTATION_CHOICE_MASKS: [[u16; MASK_HEIGHT]; PresentationChoiceNumber::COUNT] = [
    [
        0x0F00, 0x1F00, 0x3F00, 0x0700, 0x0700, 0x0700, 0x0700, 0x0700, 0x0700, 0x0700, 0x0700,
        0x0700, 0x7FF0, 0x7FF0, 0x0000, 0x0000,
    ],
    [
        0x7FF0, 0xFFF8, 0xE038, 0xE038, 0x0038, 0x0038, 0x0038, 0x7FF8, 0xFFF0, 0xE000, 0xE000,
        0xE000, 0xFFF8, 0xFFF8, 0x0000, 0x0000,
    ],
    [
        0x7FF0, 0xFFF8, 0xE038, 0x0038, 0x0038, 0x0038, 0x0FF8, 0x0FF0, 0x0038, 0x0038, 0x0038,
        0xE038, 0xFFF8, 0x7FF0, 0x0000, 0x0000,
    ],
    [
        0xE000, 0xE000, 0xE000, 0xE038, 0xE038, 0xE038, 0xE038, 0xE038, 0xFFF8, 0xFFF8, 0x0038,
        0x0038, 0x0038, 0x0038, 0x0000, 0x0000,
    ],
    [
        0xFFF8, 0xFFF8, 0xE000, 0xE000, 0xE000, 0xFFF0, 0x7FF8, 0x0038, 0x0038, 0x0038, 0xE038,
        0xE038, 0xFFF8, 0x7FF0, 0x0000, 0x0000,
    ],
    [
        0x7FF0, 0xFFF8, 0xE038, 0xE000, 0xE000, 0xFFF0, 0xFFF8, 0xE038, 0xE038, 0xE038, 0xE038,
        0xE038, 0xFFF8, 0x7FF0, 0x0000, 0x0000,
    ],
];

/// One of the six numbered presentation choices displayed on the bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PresentationChoiceNumber {
    /// First choice.
    One,
    /// Second choice.
    Two,
    /// Third choice.
    Three,
    /// Fourth choice.
    Four,
    /// Fifth choice.
    Five,
    /// Sixth choice.
    Six,
}

impl PresentationChoiceNumber {
    /// Number of authored presentation-choice masks.
    pub const COUNT: usize = 6;

    /// Decode the zero-based selector maintained by the original screen state.
    pub const fn from_index(index: u8) -> Option<Self> {
        match index {
            0 => Some(Self::One),
            1 => Some(Self::Two),
            2 => Some(Self::Three),
            3 => Some(Self::Four),
            4 => Some(Self::Five),
            5 => Some(Self::Six),
            _ => None,
        }
    }

    /// Return the zero-based authored mask index.
    pub const fn index(self) -> usize {
        self as usize
    }
}

/// Failure while drawing a choice number into the logical indexed framebuffer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PresentationChoiceMaskError {
    /// The framebuffer cannot contain the fixed mask rectangle.
    FramebufferTooSmall {
        /// Minimum required logical pixel count.
        required: usize,
        /// Actual supplied logical pixel count.
        actual: usize,
    },
}

/// Draw the selected choice number with transparent zero bits.
///
/// This translates `selected_mask_overlay` at BLOODPRG file offset `0x007CB4`.
/// The modern input is a bounded semantic choice and a flat pixel slice; the
/// original stored framebuffer pointer and signed table indexing are absent.
pub fn draw_presentation_choice_number(
    choice: PresentationChoiceNumber,
    framebuffer: &mut [u8],
) -> Result<usize, PresentationChoiceMaskError> {
    draw_mask_rows(&PRESENTATION_CHOICE_MASKS[choice.index()], framebuffer)
}

fn draw_mask_rows(
    rows: &[u16; MASK_HEIGHT],
    framebuffer: &mut [u8],
) -> Result<usize, PresentationChoiceMaskError> {
    if framebuffer.len() < REQUIRED_FRAMEBUFFER_LEN {
        return Err(PresentationChoiceMaskError::FramebufferTooSmall {
            required: REQUIRED_FRAMEBUFFER_LEN,
            actual: framebuffer.len(),
        });
    }

    let mut changed_pixel_count = usize::MIN;
    for (row, bits) in rows.iter().copied().enumerate() {
        let row_start = (MASK_ORIGIN_Y + row) * LOGICAL_FRAMEBUFFER_WIDTH + MASK_ORIGIN_X;
        for column in 0..MASK_WIDTH {
            if bits & (FIRST_ROW_BIT >> column) != u16::MIN {
                let pixel = &mut framebuffer[row_start + column];
                changed_pixel_count += usize::from(*pixel != MASK_COLOR);
                *pixel = MASK_COLOR;
            }
        }
    }
    Ok(changed_pixel_count)
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 6;
    const LOGICAL_FRAMEBUFFER_HEIGHT: usize = 200;
    const ORIGINAL_MASK_FILE_OFFSET: usize = 0x14FD8;
    const ORIGINAL_MASK_BYTE_COUNT: usize =
        PresentationChoiceNumber::COUNT * MASK_HEIGHT * size_of::<u16>();
    const INITIAL_PIXEL: u8 = 49;

    #[derive(Deserialize)]
    struct MaskOracle {
        index: u8,
        big_endian_rows: [u16; MASK_HEIGHT],
        color: u8,
        changed_offsets: Vec<usize>,
    }

    #[test]
    fn mask_rasterization_matches_every_original_pattern_vector() {
        let vectors: Vec<MaskOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7cb4_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut framebuffer =
                vec![INITIAL_PIXEL; LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT];
            let changed = draw_mask_rows(&vector.big_endian_rows, &mut framebuffer).unwrap();
            let actual_offsets: Vec<_> = framebuffer
                .iter()
                .enumerate()
                .filter_map(|(offset, pixel)| (*pixel == vector.color).then_some(offset))
                .collect();

            assert_eq!(vector.color, MASK_COLOR, "mask {}", vector.index);
            assert_eq!(
                actual_offsets, vector.changed_offsets,
                "mask {}",
                vector.index
            );
            assert_eq!(
                changed,
                vector.changed_offsets.len(),
                "mask {}",
                vector.index
            );
        }
    }

    #[test]
    fn authored_choice_masks_match_the_original_executable_data() {
        let executable = include_bytes!("../../../../../re/bin/BLOODPRG.EXE");
        let original = &executable
            [ORIGINAL_MASK_FILE_OFFSET..ORIGINAL_MASK_FILE_OFFSET + ORIGINAL_MASK_BYTE_COUNT];
        let encoded: Vec<_> = PRESENTATION_CHOICE_MASKS
            .iter()
            .flatten()
            .flat_map(|row| row.to_be_bytes())
            .collect();

        assert_eq!(encoded, original);
    }

    #[test]
    fn choice_domain_and_framebuffer_bounds_are_explicit() {
        assert!(PresentationChoiceNumber::from_index(5).is_some());
        assert!(PresentationChoiceNumber::from_index(6).is_none());
        assert_eq!(
            draw_presentation_choice_number(
                PresentationChoiceNumber::One,
                &mut vec![0; REQUIRED_FRAMEBUFFER_LEN - 1],
            ),
            Err(PresentationChoiceMaskError::FramebufferTooSmall {
                required: REQUIRED_FRAMEBUFFER_LEN,
                actual: REQUIRED_FRAMEBUFFER_LEN - 1,
            })
        );
    }
}
