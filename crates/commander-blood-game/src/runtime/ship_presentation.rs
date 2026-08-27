//! Concrete host for the recovered top-level ship presentation state machine.

use anyhow::Result;

use crate::native::bloodprg::{
    GameLifecycleState, GameSceneLink, ShipPresentationHost, ShipPresentationOutcome,
    ShipPresentationState, ShipViewEntityId, update_ship_presentation,
};

use super::ModernGameServices;

const SHIP_PRESENTATION_ACTIVE_FLAG: u16 = 1;

/// Run one ship presentation frame over the canonical flat runtime state.
pub(super) fn update_runtime_ship_presentation(
    services: &mut ModernGameServices<'_>,
    scene_link: GameSceneLink,
    lifecycle: &mut GameLifecycleState,
) -> Result<ShipPresentationOutcome> {
    let mut state = std::mem::take(services.ship_presentation_state_mut());
    let outcome;
    let deferred_error;
    {
        let mut backend = RuntimeShipPresentationBackend {
            services,
            lifecycle,
            deferred_error: None,
        };
        outcome = update_ship_presentation(&mut state, &scene_link, &mut backend);
        deferred_error = backend.deferred_error.take();
    }
    *services.ship_presentation_state_mut() = state;
    lifecycle.presentation.ship_active = state.flags & SHIP_PRESENTATION_ACTIVE_FLAG != u16::MIN;
    lifecycle.presentation.active_line =
        (state.active_line != u16::MIN).then_some(state.active_line);
    lifecycle.presentation.c2_presentation_gate = state.presentation_gate != u16::MIN;
    if let Some(error) = deferred_error {
        return Err(error);
    }
    Ok(outcome)
}

struct RuntimeShipPresentationBackend<'services, 'window, 'lifecycle> {
    services: &'services mut ModernGameServices<'window>,
    lifecycle: &'lifecycle mut GameLifecycleState,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeShipPresentationBackend<'_, '_, '_> {
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
}

impl ShipPresentationHost for RuntimeShipPresentationBackend<'_, '_, '_> {
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
            .update_runtime_ship_navigation(self.lifecycle)
            .map(|_| ());
        self.export_state(state);
        self.record(result, ());
    }
}
