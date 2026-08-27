//! Production host for the recovered camera-approach and hyperspace coordinator.

use anyhow::{Context, Result};

use crate::native::bloodprg::{
    CameraApproachContext, CameraApproachHost, CameraApproachOutcome, GameLifecycleState,
    GameSceneLink, ShipViewEntityId, update_camera_approach,
};

use super::ModernGameServices;

const PRESENTATION_GATE_ACTIVE: u16 = 1;

/// Advance camera travel once and synchronize its shared lifecycle fields.
pub(super) fn update_runtime_camera_approach<'window>(
    services: &mut ModernGameServices<'window>,
    scene_link: GameSceneLink,
    lifecycle: &mut GameLifecycleState,
) -> Result<Option<CameraApproachOutcome>> {
    if !services.runtime().camera_approach().transition_pending {
        return Ok(None);
    }

    let hyperspace_resources = services
        .runtime()
        .data()
        .hyperspace_resources()
        .sequence_names()
        .clone();
    let mut state = services.runtime_mut().take_camera_approach();
    {
        let ship = services.ship_presentation_state();
        state.active_line = ship.active_line;
        state.presentation_pending = ship.presentation_gate & PRESENTATION_GATE_ACTIVE != u16::MIN;
    }

    let native_result;
    let deferred_error;
    {
        let mut host = RuntimeCameraApproachHost {
            services,
            deferred_error: None,
        };
        native_result = update_camera_approach(
            &mut state,
            CameraApproachContext {
                scene_link: &scene_link,
                hyperspace_resources: &hyperspace_resources,
            },
            &mut host,
        );
        deferred_error = host.deferred_error;
    }

    let mut integration_error = deferred_error;
    let outcome = match native_result {
        Ok(outcome) => Some(outcome),
        Err(error) => {
            record_first_error(&mut integration_error, error);
            None
        }
    };

    if outcome == Some(CameraApproachOutcome::HyperspaceQueued) {
        record_result(
            &mut integration_error,
            services
                .select_hyperspace_video(&state.hyperspace_resource)
                .context("selecting the executable-authored hyperspace clip"),
        );
    }
    record_result(
        &mut integration_error,
        services
            .set_camera_approach_pose(state.camera, state.projection_angle)
            .context("synchronizing camera travel with the bridge renderer"),
    );

    {
        let ship = services.ship_presentation_state_mut();
        ship.active_line = state.active_line;
        ship.presentation_gate = (ship.presentation_gate & !PRESENTATION_GATE_ACTIVE)
            | u16::from(state.presentation_pending);
    }
    lifecycle.presentation.active_line =
        (state.active_line != u16::MIN).then_some(state.active_line);
    lifecycle.presentation.c2_presentation_gate = state.presentation_pending;
    lifecycle.set_modal_ui_busy(state.ui_active);
    lifecycle
        .profile_change_blockers
        .navigation_actor_transition_active = state.transition_pending;
    services
        .runtime_mut()
        .set_camera_transition_pending(state.transition_pending);
    services.runtime_mut().restore_camera_approach(state);

    if let Some(error) = integration_error {
        return Err(error);
    }
    Ok(outcome)
}

struct RuntimeCameraApproachHost<'services, 'window> {
    services: &'services mut ModernGameServices<'window>,
    deferred_error: Option<anyhow::Error>,
}

impl RuntimeCameraApproachHost<'_, '_> {
    fn record(&mut self, result: Result<()>) {
        record_result(&mut self.deferred_error, result);
    }
}

impl CameraApproachHost<GameSceneLink> for RuntimeCameraApproachHost<'_, '_> {
    type Error = anyhow::Error;

    fn initialize_screen_flags(&mut self) {
        let result = self.services.initialize_camera_transition_screen();
        self.record(result);
    }

    fn mark_entity_range_dirty(&mut self, first: u16, last: u16) {
        let result = self
            .services
            .mark_camera_transition_entities(first, last)
            .map(|_| ());
        self.record(result);
    }

    fn transition_entity(&mut self, entity: u16) {
        let result = self
            .services
            .transition_ship_view_entity(ShipViewEntityId::new(entity))
            .map(|_| ());
        self.record(result);
    }

    fn dispatch_scene(
        &mut self,
        _scene_link: &GameSceneLink,
        presentation_pending: &mut bool,
    ) -> Result<()> {
        {
            let ship = self.services.ship_presentation_state_mut();
            ship.presentation_gate = (ship.presentation_gate & !PRESENTATION_GATE_ACTIVE)
                | u16::from(*presentation_pending);
        }
        self.services.dispatch_ship_scene()?;
        *presentation_pending = self.services.ship_presentation_state().presentation_gate
            & PRESENTATION_GATE_ACTIVE
            != u16::MIN;
        Ok(())
    }

    fn snapshot_ship_hud_and_reset_camera(&mut self, camera: &mut [i16; 3]) {
        match self.services.snapshot_camera_transition_hud() {
            Ok(reset) => *camera = reset,
            Err(error) => record_first_error(&mut self.deferred_error, error),
        }
    }

    fn clear_projection_row(&mut self, color: u8) {
        let result = self.services.clear_camera_projection_band(color);
        self.record(result);
    }

    fn build_projection_matrix(&mut self, camera: [i16; 3], projection_angle: u16) {
        let result = self
            .services
            .build_camera_projection_matrix(camera, projection_angle);
        self.record(result);
    }

    fn project_point_cloud(&mut self) {
        let result = self.services.project_camera_point_cloud();
        self.record(result);
    }

    fn project_object_sprites(&mut self) {
        let result = self.services.project_camera_object_sprites();
        self.record(result);
    }
}

fn record_result(slot: &mut Option<anyhow::Error>, result: Result<()>) {
    if let Err(error) = result {
        record_first_error(slot, error);
    }
}

fn record_first_error(slot: &mut Option<anyhow::Error>, error: anyhow::Error) {
    if slot.is_none() {
        *slot = Some(error);
    }
}
