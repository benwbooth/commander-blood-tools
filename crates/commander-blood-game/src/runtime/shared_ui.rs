//! Shared boundary for the recovered low four bits of the native UI word.

use crate::native::bloodprg::GameLifecycleState;

const LOW_UI_STATE_MASK: u16 = 15;

/// Replace only the recovered low UI bits in a subsystem-owned native word.
pub(super) fn import_low_ui_state(subsystem_word: u16, lifecycle: &GameLifecycleState) -> u16 {
    (subsystem_word & !LOW_UI_STATE_MASK) | lifecycle.low_ui_state_word()
}

/// Publish only the recovered low UI bits from a subsystem-owned native word.
pub(super) fn export_low_ui_state(subsystem_word: u16, lifecycle: &mut GameLifecycleState) {
    lifecycle.set_low_ui_state_word(subsystem_word);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shared_boundary_preserves_subsystem_bits_and_replaces_only_canonical_low_bits() {
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_low_ui_state_word(5);

        let imported = import_low_ui_state(0b1010_0000, &lifecycle);

        assert_eq!(imported, 0b1010_0101);
        lifecycle.set_low_ui_state_word(15);
        export_low_ui_state(imported, &mut lifecycle);
        assert_eq!(lifecycle.low_ui_state_word(), 5);
    }
}
