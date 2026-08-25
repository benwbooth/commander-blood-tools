//! Bridge actor-slot coordination over typed flags and callbacks.

/// Number of authored bridge actor slots.
pub const NAV_ACTOR_SLOT_COUNT: usize = 6;

/// Conditions that suspend all bridge actor-slot updates.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavActorBusyState {
    /// Another presentation owns the bridge.
    pub presentation_active: bool,
    /// A dialogue or scene presentation remains queued.
    pub scene_presentation_queued: bool,
    /// Navigation-choice animation is active.
    pub choice_active: bool,
    /// A save request is active.
    pub save_active: bool,
    /// A load request is active.
    pub load_active: bool,
    /// A console item is selected.
    pub console_item_selected: bool,
    /// Target selection owns input.
    pub target_selection_active: bool,
    /// A navigation transition is pending.
    pub transition_pending: bool,
    /// Navigation-choice audio owns the actor state.
    pub choice_sound_active: bool,
}

impl NavActorBusyState {
    /// Return whether any subsystem currently suspends actor updates.
    pub const fn any(self) -> bool {
        self.presentation_active
            || self.scene_presentation_queued
            || self.choice_active
            || self.save_active
            || self.load_active
            || self.console_item_selected
            || self.target_selection_active
            || self.transition_pending
            || self.choice_sound_active
    }
}

/// Semantic low-bit state shared by bridge actor slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavActorSlotFlags {
    /// The slot participates in mouse and arc handling.
    pub active: bool,
    /// Arc mismatch resets the slot when no seek has priority.
    pub locked: bool,
    /// Mouse edge latches are cleared before hit testing.
    pub clear_mouse_before_hit: bool,
    /// The slot requests an automatic bridge seek.
    pub auto_seek: bool,
}

impl NavActorSlotFlags {
    const fn active_only() -> Self {
        Self {
            active: true,
            locked: false,
            clear_mouse_before_hit: false,
            auto_seek: false,
        }
    }
}

/// One bridge actor's coordinator-visible state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavActorSlot {
    /// Mouse/arc behavior for this slot.
    pub flags: NavActorSlotFlags,
    /// Authored target arc in the doubled bridge-frame coordinate domain.
    pub target_arc: u16,
}

/// Mouse edge state consumed by active actor slots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavActorMouseState {
    /// Primary-button edge is currently latched.
    pub primary_pressed: bool,
    /// A mouse press remains pending for dispatch.
    pub press_pending: bool,
}

/// Bridge seek state published by actor-slot auto-seek behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NavActorSeekState {
    /// Arc requested by the latest mismatched auto-seek slot.
    pub target_arc: u16,
    /// The bridge UI must process the requested seek.
    pub requested: bool,
}

/// Recovered handler assigned to one authored actor slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavActorHandler {
    /// Handler selected for the final authored slot.
    Zero,
    /// Handler selected for slot five in the native countdown.
    One,
    /// Handler selected for slot four in the native countdown.
    Two,
    /// Handler selected for slot three in the native countdown.
    Three,
    /// Handler selected for slot two in the native countdown.
    Four,
    /// Handler selected for the first authored slot.
    Five,
}

impl NavActorHandler {
    const fn for_slot(slot_index: usize) -> Self {
        match slot_index {
            0 => Self::Five,
            1 => Self::Four,
            2 => Self::Three,
            3 => Self::Two,
            4 => Self::One,
            5 => Self::Zero,
            _ => unreachable!(),
        }
    }
}

/// Mouse, entity, and actor-handler boundaries used by the coordinator.
pub trait NavActorSlotBackend {
    /// Hit-test one active slot and update its typed flags as needed.
    fn hit_test(&mut self, slot_index: usize, slot: &mut NavActorSlot);

    /// Reset the shared presentation entity after a locked arc mismatch.
    fn reset_presentation_entity(&mut self, slot_index: usize);

    /// Run the recovered handler associated with this slot.
    ///
    /// The complete array is provided because handlers can intentionally change
    /// state observed by slots later in the same update pass.
    fn update_actor(
        &mut self,
        handler: NavActorHandler,
        slot_index: usize,
        slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    );
}

/// Terminal result of one actor-slot update pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavActorSlotUpdateOutcome {
    /// A busy subsystem suppressed every slot and callback.
    Busy,
    /// All six actor handlers ran in recovered order.
    Updated,
}

/// Deactivate every bridge actor slot while preserving authored slot data.
///
/// This translates the misnamed `matrix_table_clear_2a1b` at BLOODPRG routine
/// offset `0x00963F`. Address `0x2A1B` is the same six-record actor-slot table
/// consumed by `nav_actor_slot_update_loop`; the original clears only each
/// record's flags word. No address or record padding enters the Rust model.
pub fn deactivate_nav_actor_slots(slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT]) {
    for slot in slots {
        slot.flags = NavActorSlotFlags::default();
    }
}

/// Update all bridge actor slots and publish mouse/seek side effects.
///
/// This translates `nav_actor_slot_update_loop` at BLOODPRG routine offset
/// `0x007D7B`. Typed booleans, a fixed Rust array, and explicit callbacks replace
/// shared busy bytes, packed slot records, and the native code-address table.
pub fn update_nav_actor_slots<Backend: NavActorSlotBackend>(
    busy: NavActorBusyState,
    bridge_view_frame: u16,
    mouse: &mut NavActorMouseState,
    seek: &mut NavActorSeekState,
    slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    backend: &mut Backend,
) -> NavActorSlotUpdateOutcome {
    if busy.any() {
        return NavActorSlotUpdateOutcome::Busy;
    }

    for slot_index in 0..NAV_ACTOR_SLOT_COUNT {
        if slots[slot_index].flags.active {
            if slots[slot_index].flags.clear_mouse_before_hit {
                mouse.primary_pressed = false;
                mouse.press_pending = false;
            }

            backend.hit_test(slot_index, &mut slots[slot_index]);
            let flags = slots[slot_index].flags;
            let current_arc = bridge_view_frame.wrapping_mul(2);
            if flags.auto_seek && current_arc != slots[slot_index].target_arc {
                seek.target_arc = slots[slot_index].target_arc;
                seek.requested = true;
            } else if flags.locked && current_arc != slots[slot_index].target_arc {
                slots[slot_index].flags = NavActorSlotFlags::active_only();
                backend.reset_presentation_entity(slot_index);
            }
        }

        backend.update_actor(NavActorHandler::for_slot(slot_index), slot_index, slots);
    }
    NavActorSlotUpdateOutcome::Updated
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 20;
    const DEACTIVATE_ORACLE_VECTOR_COUNT: usize = 5;
    const QUARTER_TURN_ARC: u16 = 90;
    const NATIVE_ACTIVE_FLAG: u16 = 1;
    const NATIVE_LOCKED_FLAG: u16 = 2;
    const NATIVE_CLEAR_MOUSE_FLAG: u16 = 4;
    const NATIVE_AUTO_SEEK_FLAG: u16 = 8;
    const TARGET_ARCS: [u16; NAV_ACTOR_SLOT_COUNT] = [0, 90, 180, 270, 51, 0];

    #[derive(Deserialize)]
    struct SlotOracle {
        name: String,
        gate_value: u8,
        frame: u16,
        ui_word_before: u16,
        ui_word_after: u16,
        seek_target_before: u16,
        seek_target_after: u16,
        mouse_primary_after: u8,
        mouse_pending_after: u8,
        slot_flags_after: [u8; NAV_ACTOR_SLOT_COUNT],
        call_sequence: Vec<String>,
    }

    #[derive(Deserialize)]
    struct DeactivateOracle {
        name: String,
        first_words_before: [u16; NAV_ACTOR_SLOT_COUNT],
    }

    struct OracleBackend {
        calls: Vec<String>,
        hit_slot: Option<usize>,
        mutate_after_slot: Option<(usize, usize, NavActorSlotFlags)>,
    }

    impl NavActorSlotBackend for OracleBackend {
        fn hit_test(&mut self, slot_index: usize, slot: &mut NavActorSlot) {
            self.calls.push(String::from("mouse_hit_test"));
            if self.hit_slot == Some(slot_index) {
                slot.flags.auto_seek = true;
            }
        }

        fn reset_presentation_entity(&mut self, _slot_index: usize) {
            self.calls
                .push(String::from("entity_flag_state_transition"));
        }

        fn update_actor(
            &mut self,
            handler: NavActorHandler,
            slot_index: usize,
            slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
        ) {
            self.calls.push(format!(
                "actor_handler_{}",
                match handler {
                    NavActorHandler::Zero => 0,
                    NavActorHandler::One => 1,
                    NavActorHandler::Two => 2,
                    NavActorHandler::Three => 3,
                    NavActorHandler::Four => 4,
                    NavActorHandler::Five => 5,
                }
            ));
            if let Some((after_slot, target_slot, flags)) = self.mutate_after_slot
                && after_slot == slot_index
            {
                slots[target_slot].flags = flags;
            }
        }
    }

    #[test]
    fn update_loop_matches_every_original_semantic_vector() {
        let vectors: Vec<SlotOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_7d7b_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let busy = oracle_busy_state(&vector.name);
            assert_eq!(busy.any(), vector.gate_value != u8::MIN, "{}", vector.name);
            let mut slots = oracle_slots(&vector.name);
            let mut mouse = NavActorMouseState {
                primary_pressed: true,
                press_pending: true,
            };
            let mut seek = NavActorSeekState {
                target_arc: vector.seek_target_before,
                requested: vector.ui_word_before & 8 != u16::MIN,
            };
            let mut backend = oracle_backend(&vector.name);

            let outcome = update_nav_actor_slots(
                busy,
                vector.frame,
                &mut mouse,
                &mut seek,
                &mut slots,
                &mut backend,
            );

            assert_eq!(
                outcome,
                if vector.gate_value == u8::MIN {
                    NavActorSlotUpdateOutcome::Updated
                } else {
                    NavActorSlotUpdateOutcome::Busy
                },
                "{}",
                vector.name,
            );
            assert_eq!(backend.calls, vector.call_sequence, "{}", vector.name);
            assert_eq!(
                mouse.primary_pressed,
                vector.mouse_primary_after != u8::MIN,
                "{}",
                vector.name,
            );
            assert_eq!(
                mouse.press_pending,
                vector.mouse_pending_after != u8::MIN,
                "{}",
                vector.name,
            );
            assert_eq!(seek.target_arc, vector.seek_target_after, "{}", vector.name);
            assert_eq!(
                seek.requested,
                vector.ui_word_after & 8 != u16::MIN,
                "{}",
                vector.name,
            );
            assert_eq!(
                slots.map(|slot| encode_flags(slot.flags)),
                vector.slot_flags_after.map(|flags| flags & 15),
                "{}",
                vector.name,
            );
        }
    }

    #[test]
    fn deactivation_matches_every_original_first_word_clear_vector() {
        let vectors: Vec<DeactivateOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_963f_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), DEACTIVATE_ORACLE_VECTOR_COUNT);

        for vector in vectors {
            let mut slots = std::array::from_fn(|index| NavActorSlot {
                flags: flags_from_native_word(vector.first_words_before[index]),
                target_arc: index as u16 * QUARTER_TURN_ARC,
            });
            let target_arcs = slots.map(|slot| slot.target_arc);

            deactivate_nav_actor_slots(&mut slots);

            assert!(
                slots
                    .iter()
                    .all(|slot| slot.flags == NavActorSlotFlags::default()),
                "{}",
                vector.name
            );
            assert_eq!(
                slots.map(|slot| slot.target_arc),
                target_arcs,
                "{}",
                vector.name
            );
        }
    }

    const fn flags_from_native_word(word: u16) -> NavActorSlotFlags {
        NavActorSlotFlags {
            active: word & NATIVE_ACTIVE_FLAG != 0,
            locked: word & NATIVE_LOCKED_FLAG != 0,
            clear_mouse_before_hit: word & NATIVE_CLEAR_MOUSE_FLAG != 0,
            auto_seek: word & NATIVE_AUTO_SEEK_FLAG != 0,
        }
    }

    fn oracle_busy_state(name: &str) -> NavActorBusyState {
        NavActorBusyState {
            presentation_active: name == "gate_presentation_active_high_bit",
            scene_presentation_queued: name == "gate_c2_presentation",
            choice_active: name == "gate_choice_phase",
            save_active: name == "gate_save_request",
            load_active: name == "gate_load_request",
            console_item_selected: name == "gate_selected_item_low",
            target_selection_active: name == "gate_target_selection",
            transition_pending: name == "gate_transition_pending",
            choice_sound_active: name == "gate_choice_sound",
        }
    }

    fn oracle_slots(name: &str) -> [NavActorSlot; NAV_ACTOR_SLOT_COUNT] {
        let mut slots = std::array::from_fn(|index| NavActorSlot {
            flags: NavActorSlotFlags::default(),
            target_arc: TARGET_ARCS[index],
        });
        let (slot, flags) = match name {
            "active_bit_four_clears_mouse_before_hit" => (Some(0), 5),
            "hit_sets_auto_seek" => (Some(2), 1),
            "existing_auto_seek" => (Some(3), 9),
            "hit_seek_equal_then_lock_equal" => (Some(1), 3),
            "lock_mismatch_resets_slot" => (Some(4), 3),
            "auto_seek_precedes_lock_reset" => (Some(4), 11),
            "doubled_frame_wrap_matches_zero_target" => (Some(5), 3),
            "inactive_high_flag_bits_do_not_hit_test" => (Some(3), 14),
            _ => (None, 0),
        };
        if let Some(slot) = slot {
            slots[slot].flags = decode_flags(flags);
        }
        slots
    }

    fn oracle_backend(name: &str) -> OracleBackend {
        OracleBackend {
            calls: Vec::new(),
            hit_slot: match name {
                "hit_sets_auto_seek" => Some(2),
                "hit_seek_equal_then_lock_equal" => Some(1),
                _ => None,
            },
            mutate_after_slot: (name == "handler_mutates_next_slot").then_some((
                0,
                1,
                decode_flags(3),
            )),
        }
    }

    const fn decode_flags(flags: u8) -> NavActorSlotFlags {
        NavActorSlotFlags {
            active: flags & 1 != u8::MIN,
            locked: flags & 2 != u8::MIN,
            clear_mouse_before_hit: flags & 4 != u8::MIN,
            auto_seek: flags & 8 != u8::MIN,
        }
    }

    const fn encode_flags(flags: NavActorSlotFlags) -> u8 {
        flags.active as u8
            | (flags.locked as u8) << 1
            | (flags.clear_mouse_before_hit as u8) << 2
            | (flags.auto_seek as u8) << 3
    }
}
