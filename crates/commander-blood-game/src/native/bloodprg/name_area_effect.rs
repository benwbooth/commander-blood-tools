//! Flat framebuffer implementation of the bridge name-area palette effect.

use std::fmt;

use commander_blood_formats::name_area_effect::{
    NameAreaEffectFrame, NameAreaEffectOperation, NameAreaEffectSequence,
};

const LOGICAL_FRAMEBUFFER_WIDTH: usize = 320;
const LOGICAL_FRAMEBUFFER_HEIGHT: usize = 200;
const LOGICAL_FRAMEBUFFER_PIXEL_COUNT: usize =
    LOGICAL_FRAMEBUFFER_WIDTH * LOGICAL_FRAMEBUFFER_HEIGHT;
const EFFECT_PALETTE_FIRST: u8 = 224;
const EFFECT_PALETTE_LAST: u8 = 239;
const EFFECT_PALETTE_MASK: u8 = 15;
const CYCLE_STEP: u8 = 2;
const CYCLE_EXCLUDED_INDEX: u8 = 14;

/// Runtime cursor and semantic control state for the name-area effect.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NameAreaEffectState {
    /// Whether the effect currently updates the bridge framebuffer.
    pub active: bool,
    /// Whether the deterministic opening sequence should restart.
    pub restart_requested: bool,
    /// Current decoded sequence index.
    pub sequence_index: usize,
    /// Next frame within the current decoded sequence.
    pub frame_index: usize,
    /// Frames remaining before a random sequence is selected.
    pub frames_remaining: u8,
    /// Current semantic palette operation.
    pub operation: NameAreaEffectOperation,
}

impl Default for NameAreaEffectState {
    fn default() -> Self {
        Self {
            active: false,
            restart_requested: false,
            sequence_index: usize::MIN,
            frame_index: usize::MIN,
            frames_remaining: u8::MIN,
            operation: NameAreaEffectOperation::CollapseToFirst,
        }
    }
}

/// Invalid decoded data or runtime state supplied to the effect update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameAreaEffectError {
    /// The deterministic opening sequence is missing.
    MissingOpeningSequence,
    /// No random sequence follows the deterministic opening.
    MissingRandomSequences,
    /// The random provider returned an index outside its requested domain.
    RandomIndexOutOfRange {
        /// Returned zero-based random index.
        index: usize,
        /// Exclusive requested upper bound.
        count: usize,
    },
    /// Runtime state refers to a missing decoded sequence.
    SequenceOutOfRange(usize),
    /// Runtime state refers to a missing frame within the selected sequence.
    FrameOutOfRange {
        /// Selected sequence.
        sequence: usize,
        /// Selected frame.
        frame: usize,
    },
    /// A sequence contains more frames than the recovered byte countdown.
    SequenceTooLong(usize),
    /// The caller did not provide a complete logical framebuffer.
    FramebufferTooShort(usize),
    /// An authored frame lies outside the logical 320 by 200 display.
    FrameOutsideDisplay(NameAreaEffectFrame),
}

impl fmt::Display for NameAreaEffectError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for NameAreaEffectError {}

/// Observable result of one name-area effect update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NameAreaEffectOutcome {
    /// The effect is disabled and state was untouched.
    Inactive,
    /// One decoded frame transformed the indexed framebuffer.
    Rendered {
        /// Sequence that supplied the frame.
        sequence_index: usize,
        /// Frame consumed within that sequence.
        frame_index: usize,
        /// Decoded logical rectangle.
        frame: NameAreaEffectFrame,
        /// Semantic palette operation applied.
        operation: NameAreaEffectOperation,
    },
}

/// Advance and render one name-area palette-effect frame.
///
/// This translates `name_area_palette_effect_update` at BLOODPRG routine
/// offset `0x008BAB`. Decoded owned sequences and checked flat pixel indices
/// replace stream pointers, segment aliases, and 64 KiB address wrapping. The
/// shipped frames preserve the original pixel result exactly; malformed legacy
/// packed dimensions are rejected instead of becoming out-of-bounds accesses.
pub fn update_name_area_effect(
    sequences: &[NameAreaEffectSequence],
    state: &mut NameAreaEffectState,
    framebuffer: &mut [u8],
    random_index: &mut impl FnMut(usize) -> usize,
) -> Result<NameAreaEffectOutcome, NameAreaEffectError> {
    if !state.active {
        return Ok(NameAreaEffectOutcome::Inactive);
    }
    if framebuffer.len() < LOGICAL_FRAMEBUFFER_PIXEL_COUNT {
        return Err(NameAreaEffectError::FramebufferTooShort(framebuffer.len()));
    }

    let mut next = *state;
    if next.restart_requested {
        select_sequence(sequences, usize::MIN, &mut next)?;
        next.restart_requested = false;
    }
    if next.frames_remaining == u8::MIN {
        let random_count = sequences
            .len()
            .checked_sub(1)
            .ok_or(NameAreaEffectError::MissingOpeningSequence)?;
        if random_count == usize::MIN {
            return Err(NameAreaEffectError::MissingRandomSequences);
        }
        let selected = random_index(random_count);
        if selected >= random_count {
            return Err(NameAreaEffectError::RandomIndexOutOfRange {
                index: selected,
                count: random_count,
            });
        }
        select_sequence(sequences, selected + 1, &mut next)?;
    }

    let sequence = sequences
        .get(next.sequence_index)
        .ok_or(NameAreaEffectError::SequenceOutOfRange(next.sequence_index))?;
    let frame_index = next.frame_index;
    let frame = *sequence
        .frames
        .get(frame_index)
        .ok_or(NameAreaEffectError::FrameOutOfRange {
            sequence: next.sequence_index,
            frame: frame_index,
        })?;
    validate_frame(frame)?;

    next.frames_remaining = next.frames_remaining.wrapping_sub(1);
    next.frame_index = next.frame_index.saturating_add(1);
    apply_frame(framebuffer, frame, next.operation);
    *state = next;
    Ok(NameAreaEffectOutcome::Rendered {
        sequence_index: next.sequence_index,
        frame_index,
        frame,
        operation: next.operation,
    })
}

fn select_sequence(
    sequences: &[NameAreaEffectSequence],
    index: usize,
    state: &mut NameAreaEffectState,
) -> Result<(), NameAreaEffectError> {
    let sequence = sequences
        .get(index)
        .ok_or(NameAreaEffectError::SequenceOutOfRange(index))?;
    let frame_count = u8::try_from(sequence.frames.len())
        .map_err(|_| NameAreaEffectError::SequenceTooLong(sequence.frames.len()))?;
    if frame_count == u8::MIN {
        return Err(NameAreaEffectError::FrameOutOfRange {
            sequence: index,
            frame: usize::MIN,
        });
    }
    state.sequence_index = index;
    state.frame_index = usize::MIN;
    state.frames_remaining = frame_count;
    state.operation = sequence.operation;
    Ok(())
}

fn validate_frame(frame: NameAreaEffectFrame) -> Result<(), NameAreaEffectError> {
    let right = usize::from(frame.origin[0]).checked_add(usize::from(frame.size[0]));
    let bottom = usize::from(frame.origin[1]).checked_add(usize::from(frame.size[1]));
    if frame.size.contains(&u16::MIN)
        || right.is_none_or(|edge| edge > LOGICAL_FRAMEBUFFER_WIDTH)
        || bottom.is_none_or(|edge| edge > LOGICAL_FRAMEBUFFER_HEIGHT)
    {
        return Err(NameAreaEffectError::FrameOutsideDisplay(frame));
    }
    Ok(())
}

fn apply_frame(
    framebuffer: &mut [u8],
    frame: NameAreaEffectFrame,
    operation: NameAreaEffectOperation,
) {
    let x = usize::from(frame.origin[0]);
    let y = usize::from(frame.origin[1]);
    let width = usize::from(frame.size[0]);
    let height = usize::from(frame.size[1]);
    for row in y..y + height {
        let row_start = row * LOGICAL_FRAMEBUFFER_WIDTH + x;
        for pixel in &mut framebuffer[row_start..row_start + width] {
            *pixel = transform_pixel(*pixel, operation);
        }
    }
}

const fn transform_pixel(pixel: u8, operation: NameAreaEffectOperation) -> u8 {
    let palette_index = pixel ^ EFFECT_PALETTE_FIRST;
    match operation {
        NameAreaEffectOperation::CollapseToFirst if palette_index <= EFFECT_PALETTE_MASK => {
            EFFECT_PALETTE_FIRST
        }
        NameAreaEffectOperation::CollapseToLast if palette_index <= EFFECT_PALETTE_MASK => {
            EFFECT_PALETTE_LAST
        }
        NameAreaEffectOperation::CycleForward
            if palette_index < EFFECT_PALETTE_MASK && palette_index != CYCLE_EXCLUDED_INDEX =>
        {
            EFFECT_PALETTE_FIRST + ((palette_index + CYCLE_STEP) & EFFECT_PALETTE_MASK)
        }
        NameAreaEffectOperation::FadeBackward if palette_index <= EFFECT_PALETTE_MASK => {
            EFFECT_PALETTE_FIRST + palette_index.saturating_sub(1)
        }
        _ => pixel,
    }
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 12;
    const SOURCE_PIXELS: [u8; 6] = [223, 224, 225, 238, 239, 240];

    #[derive(Deserialize)]
    struct EffectOracle {
        name: String,
        active: u8,
        restart: u8,
        control_before: u16,
        control_after: u16,
        render_operation: u8,
        frame: [u16; 4],
        calls: Vec<RandomCall>,
    }

    #[derive(Deserialize)]
    struct RandomCall {
        modulus: usize,
        result: usize,
    }

    #[test]
    fn effect_matches_original_vectors_in_the_flat_valid_domain() {
        let vectors: Vec<EffectOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_8bab_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let frame = NameAreaEffectFrame {
                origin: [vector.frame[0], vector.frame[1]],
                size: [vector.frame[2], vector.frame[3]],
            };
            let initial_operation = operation(vector.control_before as u8);
            let selected_operation = operation(vector.control_after as u8);
            let selected_frame_count = ((vector.control_after >> 8) as u8).wrapping_add(1);
            let sequences = synthetic_sequences(frame, selected_operation, selected_frame_count);
            let mut state = NameAreaEffectState {
                active: vector.active & 1 != 0,
                restart_requested: vector.restart & 1 != 0,
                sequence_index: usize::MIN,
                frame_index: usize::MIN,
                frames_remaining: (vector.control_before >> 8) as u8,
                operation: initial_operation,
            };
            let mut framebuffer = vec![u8::MIN; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
            if validate_frame(frame).is_ok() {
                seed_frame(&mut framebuffer, frame);
            }
            let mut calls = vector.calls.iter();
            let mut random = |count| {
                let call = calls.next().unwrap();
                assert_eq!(count, call.modulus);
                call.result
            };
            let outcome =
                update_name_area_effect(&sequences, &mut state, &mut framebuffer, &mut random);

            if vector.name == "packed_y_uses_byte_swap_row_address" {
                assert_eq!(
                    outcome,
                    Err(NameAreaEffectError::FrameOutsideDisplay(frame))
                );
                continue;
            }
            if vector.active & 1 == 0 {
                assert_eq!(outcome.unwrap(), NameAreaEffectOutcome::Inactive);
                continue;
            }

            let rendered = outcome.unwrap();
            let rendered_operation = match rendered {
                NameAreaEffectOutcome::Rendered { operation, .. } => operation,
                NameAreaEffectOutcome::Inactive => unreachable!(),
            };
            let expected_operation = if vector.name == "gs_operation_controls_framebuffer_pass" {
                initial_operation
            } else {
                operation(vector.render_operation)
            };
            assert_eq!(rendered_operation, expected_operation, "{}", vector.name);
            assert_eq!(
                state.frames_remaining,
                (vector.control_after >> 8) as u8,
                "{}",
                vector.name
            );
            assert!(!state.restart_requested, "{}", vector.name);
            assert_frame_transformed(&framebuffer, frame, expected_operation);
            assert!(calls.next().is_none(), "{}", vector.name);
        }
    }

    fn synthetic_sequences(
        frame: NameAreaEffectFrame,
        operation: NameAreaEffectOperation,
        frame_count: u8,
    ) -> Vec<NameAreaEffectSequence> {
        (0..10)
            .map(|_| NameAreaEffectSequence {
                operation,
                frames: vec![frame; usize::from(frame_count.max(1))].into_boxed_slice(),
            })
            .collect()
    }

    fn seed_frame(framebuffer: &mut [u8], frame: NameAreaEffectFrame) {
        let x = usize::from(frame.origin[0]);
        let y = usize::from(frame.origin[1]);
        let width = usize::from(frame.size[0]);
        let height = usize::from(frame.size[1]);
        for row in y..y + height {
            let start = row * LOGICAL_FRAMEBUFFER_WIDTH + x;
            for column in 0..width {
                framebuffer[start + column] = SOURCE_PIXELS[column % SOURCE_PIXELS.len()];
            }
        }
    }

    fn assert_frame_transformed(
        framebuffer: &[u8],
        frame: NameAreaEffectFrame,
        operation: NameAreaEffectOperation,
    ) {
        let start =
            usize::from(frame.origin[1]) * LOGICAL_FRAMEBUFFER_WIDTH + usize::from(frame.origin[0]);
        for column in 0..usize::from(frame.size[0]) {
            assert_eq!(
                framebuffer[start + column],
                transform_pixel(SOURCE_PIXELS[column % SOURCE_PIXELS.len()], operation)
            );
        }
    }

    const fn operation(value: u8) -> NameAreaEffectOperation {
        match value {
            0 => NameAreaEffectOperation::CollapseToFirst,
            1 => NameAreaEffectOperation::CollapseToLast,
            2 => NameAreaEffectOperation::CycleForward,
            _ => NameAreaEffectOperation::FadeBackward,
        }
    }

    #[test]
    fn full_width_is_processed_and_invalid_random_indices_are_rejected() {
        let frame = NameAreaEffectFrame {
            origin: [2, 0],
            size: [259, 2],
        };
        let sequences = synthetic_sequences(frame, NameAreaEffectOperation::FadeBackward, 2);
        let mut state = NameAreaEffectState {
            active: true,
            frames_remaining: 2,
            operation: NameAreaEffectOperation::FadeBackward,
            ..NameAreaEffectState::default()
        };
        let mut framebuffer = vec![EFFECT_PALETTE_LAST; LOGICAL_FRAMEBUFFER_PIXEL_COUNT];
        update_name_area_effect(&sequences, &mut state, &mut framebuffer, &mut |_| 0).unwrap();
        assert_eq!(framebuffer[2 + 258], EFFECT_PALETTE_LAST - 1);

        state.frames_remaining = u8::MIN;
        assert_eq!(
            update_name_area_effect(&sequences, &mut state, &mut framebuffer, &mut |count| count),
            Err(NameAreaEffectError::RandomIndexOutOfRange { index: 9, count: 9 })
        );
    }
}
