//! Bridge screen initialization over flat palettes and typed actor slots.

use super::{IndexedGamePalette, NAV_ACTOR_SLOT_COUNT, NavActorSlot, deactivate_nav_actor_slots};

/// Palette adjustment used by bridge sprite rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgePaletteAdjustment {
    /// Signed brightness percentage applied to every channel.
    pub brightness_percent: i16,
    /// Additive RGB adjustment.
    pub rgb_offset: [i16; 3],
}

/// Recovered dark palette adjustment used during bridge setup.
pub const BRIDGE_DARK_PALETTE_ADJUSTMENT: BridgePaletteAdjustment = BridgePaletteAdjustment {
    brightness_percent: -50,
    rgb_offset: [0; 3],
};
/// First palette index reserved for the bridge console tint table.
pub const BRIDGE_CONSOLE_TINT_FIRST: u8 = 224;

/// Mutable semantic state reset while rebuilding the bridge screen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct BridgeScreenInitializationState {
    /// A subsequent frame still needs complete screen initialization.
    pub screen_rebuild_pending: bool,
    /// Panorama loading should immediately publish its palette.
    pub palette_refresh_in_progress: bool,
    /// The palette must be uploaded to the host renderer.
    pub palette_dirty: bool,
    /// All bridge actor work completed on the previous frame.
    pub actor_completion_latched: bool,
    /// Current sprite clipping state has a valid snapshot.
    pub clip_snapshot_ready: bool,
    /// Palette index zero is interpreted as transparent.
    pub transparent_zero: bool,
    /// Retained dirty regions should be copied.
    pub dirty_copy_requested: bool,
    /// Current ship projection depth adjustment.
    pub ship_depth_offset: u16,
    /// Reverse presentation owns actor-slot state and suppresses deactivation.
    pub reverse_presentation_active: bool,
}

/// Branch and actor-slot result of one bridge screen initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BridgeScreenInitializationOutcome {
    /// Rendering path selected by the pending transition state.
    pub path: BridgeScreenInitializationPath,
    /// Whether all six bridge actor slots were deactivated.
    pub actor_slots_deactivated: bool,
}

/// Rendering path selected during bridge screen initialization.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BridgeScreenInitializationPath {
    /// The ordinary retained page was prepared and the presentation entity dirtied.
    PagePrepared,
    /// The current panorama and its bridge background entity were loaded directly.
    PanoramaTransition,
}

/// Rendering, entity, and palette-table services used during initialization.
pub trait BridgeScreenInitializationBackend {
    /// Backend error returned by rendering or resource work.
    type Error;

    /// Prepare the ordinary retained bridge page.
    fn prepare_page(
        &mut self,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Load one decoded panorama frame.
    fn load_panorama_frame(
        &mut self,
        frame: u16,
        panorama_palette: &mut IndexedGamePalette,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Clear the retained page to palette index zero.
    fn clear_secondary_page(
        &mut self,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Populate the decoded bridge-background entity on the retained page.
    fn populate_bridge_background(
        &mut self,
        panorama_palette: &mut IndexedGamePalette,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Mark the shared presentation entity for a state transition.
    fn mark_presentation_entity_dirty(
        &mut self,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Build the bridge's dark palette remap.
    fn build_palette_adjustment(
        &mut self,
        adjustment: BridgePaletteAdjustment,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
    /// Build the bridge console tint table at its reserved palette index.
    fn build_console_tint(
        &mut self,
        first_palette_index: u8,
        state: &mut BridgeScreenInitializationState,
    ) -> Result<(), Self::Error>;
}

/// Reset bridge screen state, restore its palette, and prepare actor slots.
///
/// This translates `screen_flags_init` at BLOODPRG routine offset `0x00959D`.
/// Flat RGB arrays replace far palette copies, typed booleans replace shared
/// flag bytes, and the actor-slot array replaces the address-based table walk.
pub fn initialize_bridge_screen<Backend: BridgeScreenInitializationBackend>(
    transition_pending: bool,
    panorama_frame: u16,
    state: &mut BridgeScreenInitializationState,
    panorama_palette: &mut IndexedGamePalette,
    live_palette: &mut IndexedGamePalette,
    actor_slots: &mut [NavActorSlot; NAV_ACTOR_SLOT_COUNT],
    backend: &mut Backend,
) -> Result<BridgeScreenInitializationOutcome, Backend::Error> {
    state.screen_rebuild_pending = false;
    state.palette_refresh_in_progress = true;
    state.palette_dirty = true;
    state.actor_completion_latched = false;
    state.clip_snapshot_ready = true;

    let path = if transition_pending {
        state.transparent_zero = false;
        state.dirty_copy_requested = false;
        backend.load_panorama_frame(panorama_frame, panorama_palette, state)?;
        backend.clear_secondary_page(state)?;
        backend.populate_bridge_background(panorama_palette, state)?;
        BridgeScreenInitializationPath::PanoramaTransition
    } else {
        backend.prepare_page(state)?;
        backend.mark_presentation_entity_dirty(state)?;
        BridgeScreenInitializationPath::PagePrepared
    };

    state.palette_refresh_in_progress = false;
    state.ship_depth_offset = u16::MIN;
    live_palette.copy_from_slice(panorama_palette);
    backend.build_palette_adjustment(BRIDGE_DARK_PALETTE_ADJUSTMENT, state)?;
    backend.build_console_tint(BRIDGE_CONSOLE_TINT_FIRST, state)?;

    let actor_slots_deactivated = !state.reverse_presentation_active;
    if actor_slots_deactivated {
        deactivate_nav_actor_slots(actor_slots);
    }

    Ok(BridgeScreenInitializationOutcome {
        path,
        actor_slots_deactivated,
    })
}

#[cfg(test)]
mod tests {
    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 10;
    const NATIVE_TRANSITION_FLAG: u8 = 1;
    const NATIVE_REVERSE_PRESENTATION_FLAG: u8 = 1;
    const PALETTE_MUTATION_MASK: u8 = 165;

    #[derive(Deserialize)]
    struct ScreenOracle {
        name: String,
        transition: u8,
        mode_before: u8,
        mode_after: u8,
        frame: u16,
        matrix_clear: bool,
        palette_mutated: bool,
        calls: Vec<CallOracle>,
    }

    #[derive(Deserialize)]
    struct CallOracle {
        call: String,
        palette_refresh: u8,
        palette_dirty: u8,
        rebuild: u8,
        completion: u8,
        clip_snapshot: u16,
        transparent_zero: u8,
        dirty_copy: u8,
        mode: u8,
        ship_depth: u16,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    enum Event {
        PreparePage,
        LoadPanorama,
        ClearSecondary,
        PopulateBackground,
        PresentationEntity,
        PaletteAdjustment,
        ConsoleTint,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct StateSnapshot {
        palette_refresh: bool,
        palette_dirty: bool,
        rebuild: bool,
        completion: bool,
        clip_snapshot: bool,
        transparent_zero: bool,
        dirty_copy: bool,
        reverse_presentation: bool,
        ship_depth: u16,
    }

    struct OracleBackend {
        events: Vec<(Event, StateSnapshot)>,
        mutation_event: Option<Event>,
        reverse_after_mutation: bool,
        mutate_palette: bool,
    }

    impl OracleBackend {
        fn record(&mut self, event: Event, state: &BridgeScreenInitializationState) {
            self.events.push((event, snapshot(state)));
        }

        fn apply_state_mutation(&self, event: Event, state: &mut BridgeScreenInitializationState) {
            if self.mutation_event == Some(event) {
                state.reverse_presentation_active = self.reverse_after_mutation;
            }
        }
    }

    impl BridgeScreenInitializationBackend for OracleBackend {
        type Error = std::convert::Infallible;

        fn prepare_page(
            &mut self,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            self.record(Event::PreparePage, state);
            self.apply_state_mutation(Event::PreparePage, state);
            Ok(())
        }

        fn load_panorama_frame(
            &mut self,
            _frame: u16,
            _panorama_palette: &mut IndexedGamePalette,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            self.record(Event::LoadPanorama, state);
            self.apply_state_mutation(Event::LoadPanorama, state);
            Ok(())
        }

        fn clear_secondary_page(
            &mut self,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            self.record(Event::ClearSecondary, state);
            self.apply_state_mutation(Event::ClearSecondary, state);
            Ok(())
        }

        fn populate_bridge_background(
            &mut self,
            panorama_palette: &mut IndexedGamePalette,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            self.record(Event::PopulateBackground, state);
            if self.mutate_palette {
                for color in panorama_palette {
                    for component in color {
                        *component ^= PALETTE_MUTATION_MASK;
                    }
                }
            }
            self.apply_state_mutation(Event::PopulateBackground, state);
            Ok(())
        }

        fn mark_presentation_entity_dirty(
            &mut self,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            self.record(Event::PresentationEntity, state);
            self.apply_state_mutation(Event::PresentationEntity, state);
            Ok(())
        }

        fn build_palette_adjustment(
            &mut self,
            adjustment: BridgePaletteAdjustment,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            assert_eq!(adjustment, BRIDGE_DARK_PALETTE_ADJUSTMENT);
            self.record(Event::PaletteAdjustment, state);
            self.apply_state_mutation(Event::PaletteAdjustment, state);
            Ok(())
        }

        fn build_console_tint(
            &mut self,
            first_palette_index: u8,
            state: &mut BridgeScreenInitializationState,
        ) -> Result<(), Self::Error> {
            assert_eq!(first_palette_index, BRIDGE_CONSOLE_TINT_FIRST);
            self.record(Event::ConsoleTint, state);
            self.apply_state_mutation(Event::ConsoleTint, state);
            Ok(())
        }
    }

    #[test]
    fn screen_initialization_matches_every_original_semantic_vector() {
        let vectors: Vec<ScreenOracle> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/func_959d_natural.json"
        ))
        .unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);

        for (case_index, vector) in vectors.into_iter().enumerate() {
            let transition_pending = vector.transition & NATIVE_TRANSITION_FLAG != u8::MIN;
            let first_call = &vector.calls[0];
            let mut state = BridgeScreenInitializationState {
                screen_rebuild_pending: true,
                palette_refresh_in_progress: false,
                palette_dirty: false,
                actor_completion_latched: true,
                clip_snapshot_ready: false,
                transparent_zero: first_call.transparent_zero != u8::MIN,
                dirty_copy_requested: first_call.dirty_copy != u8::MIN,
                ship_depth_offset: first_call.ship_depth,
                reverse_presentation_active: vector.mode_before & NATIVE_REVERSE_PRESENTATION_FLAG
                    != u8::MIN,
            };
            let mut panorama_palette = indexed_palette(case_index, 17);
            let mut expected_palette = panorama_palette;
            if vector.palette_mutated {
                xor_palette(&mut expected_palette, PALETTE_MUTATION_MASK);
            }
            let mut live_palette = indexed_palette(case_index, 29);
            let mut actor_slots = [NavActorSlot::default(); NAV_ACTOR_SLOT_COUNT];
            for slot in &mut actor_slots {
                slot.flags.active = true;
            }
            let mutation_event = mutation_event(&vector.name);
            let mut backend = OracleBackend {
                events: Vec::new(),
                mutation_event,
                reverse_after_mutation: vector.mode_after & NATIVE_REVERSE_PRESENTATION_FLAG
                    != u8::MIN,
                mutate_palette: vector.palette_mutated,
            };

            let outcome = initialize_bridge_screen(
                transition_pending,
                vector.frame,
                &mut state,
                &mut panorama_palette,
                &mut live_palette,
                &mut actor_slots,
                &mut backend,
            )
            .unwrap();

            assert_eq!(
                outcome,
                BridgeScreenInitializationOutcome {
                    path: if transition_pending {
                        BridgeScreenInitializationPath::PanoramaTransition
                    } else {
                        BridgeScreenInitializationPath::PagePrepared
                    },
                    actor_slots_deactivated: vector.matrix_clear,
                },
                "{}",
                vector.name
            );
            assert_eq!(live_palette, expected_palette, "{}", vector.name);
            assert_eq!(panorama_palette, expected_palette, "{}", vector.name);
            assert!(!state.screen_rebuild_pending, "{}", vector.name);
            assert!(!state.palette_refresh_in_progress, "{}", vector.name);
            assert!(state.palette_dirty, "{}", vector.name);
            assert!(!state.actor_completion_latched, "{}", vector.name);
            assert!(state.clip_snapshot_ready, "{}", vector.name);
            assert_eq!(state.ship_depth_offset, u16::MIN, "{}", vector.name);
            assert_eq!(
                actor_slots.iter().all(|slot| !slot.flags.active),
                vector.matrix_clear,
                "{}",
                vector.name
            );
            assert_call_snapshots(&backend.events, &vector.calls, &vector.name);
        }
    }

    fn snapshot(state: &BridgeScreenInitializationState) -> StateSnapshot {
        StateSnapshot {
            palette_refresh: state.palette_refresh_in_progress,
            palette_dirty: state.palette_dirty,
            rebuild: state.screen_rebuild_pending,
            completion: state.actor_completion_latched,
            clip_snapshot: state.clip_snapshot_ready,
            transparent_zero: state.transparent_zero,
            dirty_copy: state.dirty_copy_requested,
            reverse_presentation: state.reverse_presentation_active,
            ship_depth: state.ship_depth_offset,
        }
    }

    fn assert_call_snapshots(events: &[(Event, StateSnapshot)], calls: &[CallOracle], name: &str) {
        let expected: Vec<&CallOracle> = calls
            .iter()
            .filter(|call| call.call != "matrix_table_clear_2a1b")
            .collect();
        assert_eq!(events.len(), expected.len(), "{name}");
        for ((event, actual), expected) in events.iter().zip(expected) {
            assert_eq!(*event, event_for_call(&expected.call), "{name}");
            assert_eq!(
                actual.palette_refresh,
                expected.palette_refresh != u8::MIN,
                "{name}"
            );
            assert_eq!(
                actual.palette_dirty,
                expected.palette_dirty != u8::MIN,
                "{name}"
            );
            assert_eq!(actual.rebuild, expected.rebuild != u8::MIN, "{name}");
            assert_eq!(actual.completion, expected.completion != u8::MIN, "{name}");
            assert_eq!(
                actual.clip_snapshot,
                expected.clip_snapshot != u16::MIN,
                "{name}"
            );
            assert_eq!(
                actual.transparent_zero,
                expected.transparent_zero != u8::MIN,
                "{name}"
            );
            assert_eq!(actual.dirty_copy, expected.dirty_copy != u8::MIN, "{name}");
            assert_eq!(
                actual.reverse_presentation,
                expected.mode & NATIVE_REVERSE_PRESENTATION_FLAG != u8::MIN,
                "{name}"
            );
            assert_eq!(actual.ship_depth, expected.ship_depth, "{name}");
        }
    }

    fn event_for_call(call: &str) -> Event {
        match call {
            "page_flip" => Event::PreparePage,
            "bridge_panorama_frame_load" => Event::LoadPanorama,
            "blit_fill_row_5221" => Event::ClearSecondary,
            "entity_object_populate" => Event::PopulateBackground,
            "entity_flag_state_transition" => Event::PresentationEntity,
            "palette_blend_remap_table_build" => Event::PaletteAdjustment,
            "tint_table_build_banked" => Event::ConsoleTint,
            _ => panic!("unknown recovered call {call}"),
        }
    }

    fn mutation_event(name: &str) -> Option<Event> {
        match name {
            "page_flip_callback_sets_mode" => Some(Event::PreparePage),
            "entity_callback_clears_mode" => Some(Event::PresentationEntity),
            "populate_callback_updates_palette_and_mode" => Some(Event::PopulateBackground),
            _ => None,
        }
    }

    fn indexed_palette(case_index: usize, multiplier: usize) -> IndexedGamePalette {
        std::array::from_fn(|color| {
            std::array::from_fn(|component| {
                (color * multiplier + component * 31 + case_index * 37) as u8
            })
        })
    }

    fn xor_palette(palette: &mut IndexedGamePalette, mask: u8) {
        for color in palette {
            for component in color {
                *component ^= mask;
            }
        }
    }
}
