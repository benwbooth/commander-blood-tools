//! Palette transition and renderer-upload coordination.

const PALETTE_DIRTY: u8 = 1;
const PALETTE_TRANSITION_COMPLETE: u16 = 100;

/// Mutable state consumed by the palette-upload gate.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaletteUploadState {
    /// Bit zero requests an upload of the current live palette.
    pub dirty_flags: u8,
    /// Primary-button latch cleared after a successful upload.
    pub primary_pressed: u8,
    /// Secondary-button latch intentionally preserved by the native routine.
    pub secondary_pressed: u8,
    /// Pending mouse press count cleared after a successful upload.
    pub press_pending: u8,
}

/// Translate BLOODPRG routine `0x00178B` to a renderer upload decision.
///
/// wgpu presentation replaces the VGA retrace wait and DAC write. The exact
/// dirty-bit gate and post-upload primary/pending latch clears remain here;
/// unrelated dirty bits and all latches remain untouched on the clean path.
pub fn take_palette_upload_request(state: &mut PaletteUploadState) -> bool {
    if state.dirty_flags & PALETTE_DIRTY == u8::MIN {
        return false;
    }
    state.dirty_flags = u8::MIN;
    state.press_pending = u8::MIN;
    state.primary_pressed = u8::MIN;
    true
}

/// Mutable state of the native zero-to-100 palette transition.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PaletteTransitionState {
    /// Current wrapping transition percentage.
    pub percent: u16,
    /// Wrapping amount added on each active step.
    pub increment: u16,
    /// First inclusive palette index.
    pub first: u8,
    /// Last inclusive palette index.
    pub last: u8,
    /// Palette upload flags replaced with one after interpolation.
    pub dirty_flags: u8,
}

/// Exact interpolation request emitted by one active transition step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PaletteInterpolationRequest {
    /// Signed low byte consumed by the recovered interpolation routine.
    pub percent: i8,
    /// First inclusive palette index.
    pub first: u8,
    /// Last inclusive palette index.
    pub last: u8,
}

/// Translate BLOODPRG routine `0x001F78` to typed transition state.
///
/// Only exactly 100 is complete. Active updates retain wrapping word addition,
/// signed comparison for the upper clamp, signed low-byte interpolation input,
/// store-before-interpolate ordering, and replacement of dirty flags with one.
pub fn advance_palette_transition(
    state: &mut PaletteTransitionState,
) -> Option<PaletteInterpolationRequest> {
    if state.percent == PALETTE_TRANSITION_COMPLETE {
        return None;
    }

    let mut percent = state.percent.wrapping_add(state.increment);
    if (percent as i16) > PALETTE_TRANSITION_COMPLETE as i16 {
        percent = PALETTE_TRANSITION_COMPLETE;
    }
    state.percent = percent;
    state.dirty_flags = PALETTE_DIRTY;
    Some(PaletteInterpolationRequest {
        percent: percent as u8 as i8,
        first: state.first,
        last: state.last,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    #[derive(Deserialize)]
    struct PaletteUploadVector {
        name: String,
        dirty_before: u8,
        dirty_path: bool,
        dirty_after: u8,
        primary_after: u8,
        secondary_after: u8,
        pending_after: u8,
    }

    #[derive(Deserialize)]
    struct PaletteTransitionVector {
        name: String,
        initial_percent: u16,
        increment: u16,
        result_percent: u16,
        active: bool,
        first: u8,
        last: u8,
        dirty_before: u8,
        dirty_after: u8,
        helper_call: Option<PaletteHelperCall>,
    }

    #[derive(Deserialize)]
    struct PaletteHelperCall {
        percent_signed_byte: i8,
    }

    #[test]
    fn upload_gate_matches_every_original_palette_vector() {
        let vectors: Vec<PaletteUploadVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_178b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 7);

        for vector in vectors {
            let mut state = PaletteUploadState {
                dirty_flags: vector.dirty_before,
                primary_pressed: vector.primary_after.max(1),
                secondary_pressed: vector.secondary_after,
                press_pending: vector.pending_after.max(1),
            };
            let requested = take_palette_upload_request(&mut state);
            assert_eq!(requested, vector.dirty_path, "{}", vector.name);
            assert_eq!(state.dirty_flags, vector.dirty_after, "{}", vector.name);
            assert_eq!(
                state.primary_pressed, vector.primary_after,
                "{}",
                vector.name
            );
            assert_eq!(
                state.secondary_pressed, vector.secondary_after,
                "{}",
                vector.name
            );
            assert_eq!(state.press_pending, vector.pending_after, "{}", vector.name);
        }
    }

    #[test]
    fn transition_step_matches_every_original_palette_vector() {
        let vectors: Vec<PaletteTransitionVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1f78_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), 9);

        for vector in vectors {
            let mut state = PaletteTransitionState {
                percent: vector.initial_percent,
                increment: vector.increment,
                first: vector.first,
                last: vector.last,
                dirty_flags: vector.dirty_before,
            };
            let request = advance_palette_transition(&mut state);
            assert_eq!(request.is_some(), vector.active, "{}", vector.name);
            assert_eq!(state.percent, vector.result_percent, "{}", vector.name);
            assert_eq!(state.dirty_flags, vector.dirty_after, "{}", vector.name);
            assert_eq!(
                request.map(|request| request.percent),
                vector.helper_call.map(|helper| helper.percent_signed_byte),
                "{}",
                vector.name
            );
            if let Some(request) = request {
                assert_eq!([request.first, request.last], [vector.first, vector.last]);
            }
        }
    }
}
