//! Typed rectangle interpolation for indexed-framebuffer transitions.

use std::fmt;

/// One signed rectangle from the original transition tables.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionRect {
    /// Horizontal origin.
    pub x: i16,
    /// Vertical origin.
    pub y: i16,
    /// Rectangle width.
    pub width: i16,
    /// Rectangle height.
    pub height: i16,
}

impl TransitionRect {
    /// Build a rectangle in original field order.
    pub const fn new(x: i16, y: i16, width: i16, height: i16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    const fn fields(self) -> [i16; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

/// Unsigned renderer region emitted by one transition step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransitionRenderRegion {
    /// Horizontal origin after native word wrapping.
    pub x: u16,
    /// Vertical origin after native word wrapping.
    pub y: u16,
    /// Width after native word wrapping.
    pub width: u16,
    /// Height after native word wrapping.
    pub height: u16,
}

impl TransitionRenderRegion {
    const fn from_fields(fields: [u16; 4]) -> Self {
        Self {
            x: fields[0],
            y: fields[1],
            width: fields[2],
            height: fields[3],
        }
    }

    #[cfg(test)]
    const fn fields(self) -> [u16; 4] {
        [self.x, self.y, self.width, self.height]
    }
}

/// Current byte-sized progress of a rectangle transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FramebufferTransitionState {
    /// Authored total step count, interpreted as a signed byte during division.
    pub total_steps: u8,
    /// Current step, incremented before interpolation.
    pub current_step: u8,
}

/// Arithmetic state that would have trapped the original signed division.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FramebufferTransitionError {
    /// An active transition supplied a zero signed divisor.
    ZeroStepDivisor,
    /// Signed division overflowed for `i16::MIN / -1`.
    DivisionOverflow,
}

impl fmt::Display for FramebufferTransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid framebuffer transition: {self:?}")
    }
}

impl std::error::Error for FramebufferTransitionError {}

/// Translate BLOODPRG routine `0x001E5D` over typed rectangles.
///
/// Completion compares the raw bytes before any arithmetic. Active steps retain
/// pre-increment wrapping, signed-byte total/current values, signed-word delta
/// wrapping, division before multiplication, and low-word result wrapping. The
/// returned region is the exact input for the palette-remap renderer operation.
pub fn advance_framebuffer_rect_transition(
    state: &mut FramebufferTransitionState,
    source: TransitionRect,
    target: TransitionRect,
) -> Result<Option<TransitionRenderRegion>, FramebufferTransitionError> {
    if state.total_steps == state.current_step {
        return Ok(None);
    }

    state.current_step = state.current_step.wrapping_add(1);
    let divisor = i16::from(state.total_steps as i8);
    if divisor == 0 {
        return Err(FramebufferTransitionError::ZeroStepDivisor);
    }
    let current = i16::from(state.current_step as i8);
    let source = source.fields();
    let target = target.fields();
    let mut interpolated = [u16::MIN; 4];

    for field in 0..interpolated.len() {
        let delta = source[field].wrapping_sub(target[field]);
        let quotient = delta
            .checked_div(divisor)
            .ok_or(FramebufferTransitionError::DivisionOverflow)?;
        interpolated[field] = target[field].wrapping_add(quotient.wrapping_mul(current)) as u16;
    }

    Ok(Some(TransitionRenderRegion::from_fields(interpolated)))
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct TransitionVector {
        name: String,
        total_steps: u8,
        initial_step: u8,
        result_step: u8,
        source: [i16; 4],
        target: [i16; 4],
        active: bool,
        interpolated_u16: Option<[u16; 4]>,
    }

    #[test]
    fn rectangle_interpolation_matches_every_original_vector() {
        let vectors: Vec<TransitionVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1e5d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 12);

        for vector in vectors {
            let mut state = FramebufferTransitionState {
                total_steps: vector.total_steps,
                current_step: vector.initial_step,
            };
            let source = TransitionRect::new(
                vector.source[0],
                vector.source[1],
                vector.source[2],
                vector.source[3],
            );
            let target = TransitionRect::new(
                vector.target[0],
                vector.target[1],
                vector.target[2],
                vector.target[3],
            );

            let region = advance_framebuffer_rect_transition(&mut state, source, target)
                .unwrap_or_else(|error| panic!("{}: {error}", vector.name));

            assert_eq!(state.current_step, vector.result_step, "{}", vector.name);
            assert_eq!(region.is_some(), vector.active, "{}", vector.name);
            assert_eq!(
                region.map(TransitionRenderRegion::fields),
                vector.interpolated_u16,
                "{}",
                vector.name
            );
        }
    }

    #[test]
    fn malformed_active_zero_divisor_is_rejected() {
        let mut state = FramebufferTransitionState {
            total_steps: u8::MIN,
            current_step: 1,
        };
        assert_eq!(
            advance_framebuffer_rect_transition(
                &mut state,
                TransitionRect::new(1, 2, 3, 4),
                TransitionRect::new(5, 6, 7, 8),
            ),
            Err(FramebufferTransitionError::ZeroStepDivisor)
        );
    }
}
