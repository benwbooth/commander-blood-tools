//! Pointer-button state sampled atomically by the modern event loop.

/// Pointer buttons observed by native game logic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u16)]
pub enum PointerButton {
    /// Primary selection button.
    Primary = 1,
    /// Secondary selection button.
    Secondary = 2,
}

/// Complete host pointer-button sample.
///
/// Unrecognized bits remain intact because the original routine snapshots the
/// complete word even though edge detection examines only its low byte.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerButtons(u16);

impl PointerButtons {
    /// No pointer buttons are pressed.
    pub const NONE: Self = Self(u16::MIN);

    /// Preserve one complete host button sample.
    pub const fn from_bits(bits: u16) -> Self {
        Self(bits)
    }

    /// Return the complete host button sample.
    pub const fn bits(self) -> u16 {
        self.0
    }

    /// Test one semantic button.
    pub const fn contains(self, button: PointerButton) -> bool {
        self.0 & button as u16 != u16::MIN
    }

    const fn low_byte(self) -> u8 {
        self.0 as u8
    }
}

/// Press latches retained until the owning interaction consumes them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerButtonEdges {
    /// A primary press has been observed.
    pub primary_pressed: bool,
    /// A secondary press has been observed.
    pub secondary_pressed: bool,
    /// At least one press is waiting to be consumed.
    pub press_pending: bool,
}

/// Persistent button snapshot and edge latches.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PointerButtonState {
    /// Button sample retained from the preceding update.
    pub previous: PointerButtons,
    /// Press latches retained for the interaction dispatcher.
    pub edges: PointerButtonEdges,
}

/// Update button press latches from one atomic host sample.
///
/// This translates `mouse_button_edges_update` at BLOODPRG routine offset
/// `0x001FBC`. Its mutable low-byte intersection is intentionally preserved:
/// simultaneous new primary and secondary presses latch only the primary
/// press, and an unrelated held button can suppress a secondary press. SDL
/// provides one stable sample per event-loop update, replacing asynchronous
/// rereads of native global words with an ordinary value.
pub fn update_pointer_button_edges(
    state: &mut PointerButtonState,
    current: PointerButtons,
) -> PointerButtons {
    let mut buttons = current.low_byte();
    let previous = state.previous.low_byte();

    if buttons & PointerButton::Primary as u8 != u8::MIN {
        buttons &= previous;
        if buttons == u8::MIN {
            state.edges.primary_pressed = true;
            state.edges.press_pending = true;
        }
    }

    if buttons & PointerButton::Secondary as u8 != u8::MIN {
        buttons &= previous;
        if buttons == u8::MIN {
            state.edges.secondary_pressed = true;
            state.edges.press_pending = true;
        }
    }

    state.previous = current;
    current
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 15;
    const STABLE_SAMPLE_VECTOR_COUNT: usize = 13;
    const VOLATILE_PROBE_VECTOR_COUNT: usize = ORACLE_VECTOR_COUNT - STABLE_SAMPLE_VECTOR_COUNT;
    const NATIVE_LATCHED_VALUE: u8 = 1;

    #[derive(Deserialize)]
    struct ButtonEdgeOracle {
        name: String,
        current_initial: u16,
        previous_initial: u16,
        previous_second_read: Option<u16>,
        current_final_read: u16,
        primary_after: u8,
        secondary_after: u8,
        pending_after: u8,
        result_ax: u16,
    }

    #[test]
    fn atomic_updates_match_every_stable_original_vector() {
        let vectors = oracle_vectors();
        let mut stable_samples = usize::MIN;

        for vector in vectors.iter().filter(|vector| {
            vector.previous_second_read.is_none()
                && vector.current_initial == vector.current_final_read
        }) {
            stable_samples += 1;
            let mut state = PointerButtonState {
                previous: PointerButtons::from_bits(vector.previous_initial),
                edges: PointerButtonEdges::default(),
            };
            let current = PointerButtons::from_bits(vector.current_initial);

            let result = update_pointer_button_edges(&mut state, current);

            assert_eq!(result.bits(), vector.result_ax, "{}", vector.name);
            assert_eq!(state.previous, current, "{}", vector.name);
            assert_eq!(
                state.edges.primary_pressed,
                vector.primary_after == NATIVE_LATCHED_VALUE,
                "{}",
                vector.name
            );
            assert_eq!(
                state.edges.secondary_pressed,
                vector.secondary_after == NATIVE_LATCHED_VALUE,
                "{}",
                vector.name
            );
            assert_eq!(
                state.edges.press_pending,
                vector.pending_after == NATIVE_LATCHED_VALUE,
                "{}",
                vector.name
            );
        }

        assert_eq!(stable_samples, STABLE_SAMPLE_VECTOR_COUNT);
    }

    #[test]
    fn flat_runtime_replaces_mid_call_memory_mutation_with_one_sample() {
        let vectors = oracle_vectors();
        let volatile_probes = vectors
            .iter()
            .filter(|vector| {
                vector.previous_second_read.is_some()
                    || vector.current_initial != vector.current_final_read
            })
            .collect::<Vec<_>>();
        assert_eq!(volatile_probes.len(), VOLATILE_PROBE_VECTOR_COUNT);

        for vector in volatile_probes {
            let mut state = PointerButtonState {
                previous: PointerButtons::from_bits(vector.previous_initial),
                edges: PointerButtonEdges::default(),
            };
            let current = PointerButtons::from_bits(vector.current_initial);

            assert_eq!(update_pointer_button_edges(&mut state, current), current);
            assert_eq!(state.previous, current);
        }
    }

    #[test]
    fn existing_press_latches_remain_set_until_the_owner_consumes_them() {
        let mut state = PointerButtonState {
            previous: PointerButtons::NONE,
            edges: PointerButtonEdges {
                primary_pressed: true,
                secondary_pressed: true,
                press_pending: true,
            },
        };

        update_pointer_button_edges(&mut state, PointerButtons::NONE);

        assert!(state.edges.primary_pressed);
        assert!(state.edges.secondary_pressed);
        assert!(state.edges.press_pending);
    }

    fn oracle_vectors() -> Vec<ButtonEdgeOracle> {
        let vectors: Vec<ButtonEdgeOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_1fbc_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        vectors
    }
}
