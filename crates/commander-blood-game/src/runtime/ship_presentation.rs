//! Concrete host for the recovered top-level ship presentation state machine.

use anyhow::Result;

use crate::native::bloodprg::{
    GameLifecycleState, GameSceneLink, ShipPresentationHost, ShipPresentationOutcome,
    ShipPresentationState, ShipViewEntityId, decode_active_presentation_line,
    encode_active_presentation_line, update_ship_presentation,
};

use super::{ModernGameServices, RuntimePlatformHost};

const SHIP_PRESENTATION_ACTIVE_FLAG: u16 = 1;
const LOW_UI_STATE_MASK: u16 = 15;

/// Run one ship presentation frame over the canonical flat runtime state.
pub(super) fn update_runtime_ship_presentation<'window>(
    services: &mut ModernGameServices<'window>,
    scene_link: GameSceneLink,
    lifecycle: &mut GameLifecycleState,
    platform: &mut RuntimePlatformHost<'window>,
) -> Result<ShipPresentationOutcome> {
    let mut state = std::mem::take(services.ship_presentation_state_mut());
    import_lifecycle_presentation_state(&mut state, lifecycle);
    let outcome;
    let deferred_error;
    {
        let mut backend = RuntimeShipPresentationBackend {
            services,
            lifecycle,
            platform,
            deferred_error: None,
        };
        outcome = update_ship_presentation(&mut state, &scene_link, &mut backend);
        deferred_error = backend.deferred_error.take();
    }
    *services.ship_presentation_state_mut() = state;
    export_lifecycle_presentation_state(&state, lifecycle);
    if let Some(error) = deferred_error {
        return Err(error);
    }
    Ok(outcome)
}

fn import_lifecycle_presentation_state(
    state: &mut ShipPresentationState,
    lifecycle: &GameLifecycleState,
) {
    state.ui_state = (state.ui_state & !LOW_UI_STATE_MASK) | lifecycle.low_ui_state_word();
    state.active_line = encode_active_presentation_line(lifecycle.presentation.active_line);
    state.presentation_gate = (state.presentation_gate & !SHIP_PRESENTATION_ACTIVE_FLAG)
        | u16::from(lifecycle.presentation.c2_presentation_gate);
}

fn export_lifecycle_presentation_state(
    state: &ShipPresentationState,
    lifecycle: &mut GameLifecycleState,
) {
    lifecycle.set_low_ui_state_word(state.ui_state);
    lifecycle.presentation.ship_active = state.flags & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN;
    lifecycle.presentation.active_line = decode_active_presentation_line(state.active_line);
    lifecycle.presentation.c2_presentation_gate =
        state.presentation_gate & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN;
}

struct RuntimeShipPresentationBackend<'services, 'window, 'lifecycle, 'platform> {
    services: &'services mut ModernGameServices<'window>,
    lifecycle: &'lifecycle mut GameLifecycleState,
    platform: &'platform mut RuntimePlatformHost<'window>,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeShipPresentationBackend<'_, '_, '_, '_> {
    fn record<T>(&mut self, result: Result<T>, fallback: T) -> T {
        match result {
            Ok(value) => value,
            Err(error) => {
                if self.deferred_error.is_none() {
                    self.deferred_error = Some(error);
                }
                fallback
            }
        }
    }

    fn import_state(&mut self, state: &ShipPresentationState) {
        *self.services.ship_presentation_state_mut() = *state;
    }

    fn export_state(&self, state: &mut ShipPresentationState) {
        *state = *self.services.ship_presentation_state();
    }

    fn merge_lifecycle_ui_into_native_state(&mut self) {
        let lifecycle_ui = self.lifecycle.low_ui_state_word();
        let native_state = self.services.ship_presentation_state_mut();
        native_state.ui_state = (native_state.ui_state & !LOW_UI_STATE_MASK) | lifecycle_ui;
    }
}

impl ShipPresentationHost for RuntimeShipPresentationBackend<'_, '_, '_, '_> {
    type SceneLink = GameSceneLink;

    fn transition_entity(&mut self, entity: ShipViewEntityId) {
        let result = self
            .services
            .transition_ship_view_entity(entity)
            .map(|_| ());
        self.record(result, ());
    }

    fn advance_depth(&mut self, state: &mut ShipPresentationState) {
        self.import_state(state);
        self.services.advance_ship_depth();
        self.export_state(state);
    }

    fn compose_depth_band(&mut self, state: &mut ShipPresentationState) {
        self.import_state(state);
        let result = self.services.compose_ship_depth_bands().map(|_| ());
        self.export_state(state);
        self.record(result, ());
    }

    fn dispatch_scene(&mut self, state: &mut ShipPresentationState, _scene_link: &Self::SceneLink) {
        self.import_state(state);
        let result = self.services.dispatch_ship_scene().map(|_| ());
        self.export_state(state);
        self.record(result, ());
    }

    fn update_hud(&mut self, state: &mut ShipPresentationState) {
        self.import_state(state);
        let result = self
            .services
            .update_runtime_ship_hud(self.lifecycle)
            .map(|_| ());
        self.merge_lifecycle_ui_into_native_state();
        self.export_state(state);
        self.record(result, ());
    }

    fn clear_travel_band(&mut self) {
        self.services.clear_ship_travel_display();
    }

    fn update_navigation(&mut self, state: &mut ShipPresentationState) {
        self.import_state(state);
        let result = self
            .services
            .update_runtime_ship_navigation(self.lifecycle, self.platform)
            .map(|_| ());
        self.merge_lifecycle_ui_into_native_state();
        self.export_state(state);
        self.record(result, ());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNOWNED_PRESENTATION_GATE_BITS: u16 = 0x1200;
    const IZWALITO_IDLE_LINE: u16 = 8;
    const SECOND_BAND_PRESENTATION_MODE: u16 = 64;
    const ACTIVE_LOW_UI_STATE: u16 = 13;

    #[test]
    fn lifecycle_line_and_gate_are_imported_without_losing_valid_line_zero() {
        let mut state = ShipPresentationState {
            presentation_gate: UNOWNED_PRESENTATION_GATE_BITS,
            ui_state: SECOND_BAND_PRESENTATION_MODE,
            ..ShipPresentationState::default()
        };
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_presentation_interface_active(true);
        lifecycle.set_modal_ui_busy(true);
        lifecycle.set_navigation_ui_busy(true);
        lifecycle.presentation.active_line = Some(IZWALITO_IDLE_LINE);
        lifecycle.presentation.c2_presentation_gate = true;

        import_lifecycle_presentation_state(&mut state, &lifecycle);

        assert_eq!(state.active_line, IZWALITO_IDLE_LINE);
        assert_eq!(
            state.ui_state,
            SECOND_BAND_PRESENTATION_MODE | ACTIVE_LOW_UI_STATE
        );
        assert_eq!(
            state.presentation_gate,
            UNOWNED_PRESENTATION_GATE_BITS | SHIP_PRESENTATION_ACTIVE_FLAG
        );

        state.active_line = u16::MIN;
        state.ui_state = u16::MIN;
        state.presentation_gate &= !SHIP_PRESENTATION_ACTIVE_FLAG;
        export_lifecycle_presentation_state(&state, &mut lifecycle);

        assert_eq!(lifecycle.presentation.active_line, Some(u16::MIN));
        assert!(!lifecycle.presentation.c2_presentation_gate);
        assert!(!lifecycle.presentation_interface_active());
        assert!(!lifecycle.modal_ui_busy());
        assert!(!lifecycle.profile_ui_blocked());
    }

    #[test]
    fn phone_presentation_initialization_clears_the_canonical_ui_word() {
        let mut state = ShipPresentationState {
            flags: SHIP_PRESENTATION_ACTIVE_FLAG,
            ..ShipPresentationState::default()
        };
        let mut lifecycle = GameLifecycleState::default();
        lifecycle.set_presentation_interface_active(true);
        lifecycle.set_modal_ui_busy(true);
        import_lifecycle_presentation_state(&mut state, &lifecycle);

        struct NoopHost;

        impl crate::native::bloodprg::ShipPresentationHost for NoopHost {
            type SceneLink = GameSceneLink;

            fn transition_entity(&mut self, _entity: ShipViewEntityId) {}
            fn advance_depth(&mut self, _state: &mut ShipPresentationState) {}
            fn compose_depth_band(&mut self, _state: &mut ShipPresentationState) {}
            fn dispatch_scene(
                &mut self,
                _state: &mut ShipPresentationState,
                _scene_link: &Self::SceneLink,
            ) {
            }
            fn update_hud(&mut self, _state: &mut ShipPresentationState) {}
            fn clear_travel_band(&mut self) {}
            fn update_navigation(&mut self, _state: &mut ShipPresentationState) {}
        }

        let outcome = update_ship_presentation(&mut state, &GameSceneLink::Initial, &mut NoopHost);
        export_lifecycle_presentation_state(&state, &mut lifecycle);

        assert_eq!(outcome, ShipPresentationOutcome::Initialized);
        assert_eq!(state.ui_state, u16::MIN);
        assert!(!lifecycle.presentation_interface_active());
        assert!(!lifecycle.modal_ui_busy());
    }

    #[test]
    fn default_ship_state_uses_the_native_no_line_sentinel() {
        let mut lifecycle = GameLifecycleState::default();
        let state = ShipPresentationState::default();

        export_lifecycle_presentation_state(&state, &mut lifecycle);

        assert_eq!(lifecycle.presentation.active_line, None);
    }
}
