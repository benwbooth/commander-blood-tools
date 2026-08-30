//! Ship-camera approach and hyperspace transition coordinator.

use super::NO_PRESENTATION_LINE;

const PHASE_MASK: u8 = 7;
const CAMERA_X_LIMIT: i16 = 9_000;
const CAMERA_X_STEP: i16 = 100;
const PROJECTION_ANGLE_WRAP: u16 = 180;
const CAMERA_Z_CRUISE: u16 = 20_000;
const CAMERA_Z_ACCELERATION_STEP: u16 = 100;
const CAMERA_Z_FINAL: i16 = 30_000;
const CAMERA_X_RESET: i16 = 10_000;
const CAMERA_Y_RESET: i16 = 12_000;
const HYPERSPACE_ACTIVE_LINE: u16 = 6;
const TRANSITION_ENTITY: u16 = 4;
const FIRST_DIRTY_ENTITY: u16 = 21;
const LAST_DIRTY_ENTITY: u16 = 31;
const COMPLETED_ACCELERATION: u16 = 16;
const RENDER_CLEAR_COLOR: u8 = 0;

/// Number of authored hyperspace clips cycled by the camera approach.
pub const HYPERSPACE_SEQUENCE_COUNT: usize = 8;

/// Presentation mode published to the navigation actor coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraApproachPresentation {
    /// No actor presentation is currently active.
    Inactive,
    /// The ordinary navigation actor presentation is active.
    Active,
    /// Actor presentation is suppressed while the hyperspace clip starts.
    Suppressed,
    /// A presentation value owned by another coordinator is being preserved.
    Other(u16),
}

impl CameraApproachPresentation {
    /// Interpret a recovered presentation identifier.
    pub const fn from_id(value: u16) -> Self {
        match value {
            0 => Self::Inactive,
            1 => Self::Active,
            u16::MAX => Self::Suppressed,
            other => Self::Other(other),
        }
    }

    /// Return the recovered presentation identifier represented by this mode.
    pub const fn id(self) -> u16 {
        match self {
            Self::Inactive => 0,
            Self::Active => 1,
            Self::Suppressed => u16::MAX,
            Self::Other(value) => value,
        }
    }
}

/// Mutable state of the approach, clip handoff, and camera easing sequence.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CameraApproachState {
    /// Raw phase byte; upper bits are retained because the original dispatcher
    /// only masked them for its initialization test.
    pub phase: u8,
    /// Navigation actor presentation mode.
    pub actor_presentation: CameraApproachPresentation,
    /// Whether the interactive navigation camera is currently active.
    pub camera_view_active: bool,
    /// Whether a camera transition is in progress.
    pub transition_pending: bool,
    /// Whether the camera-approach UI mode is active.
    pub ui_active: bool,
    /// Flat signed ship-camera coordinates.
    pub camera: [i16; 3],
    /// Wrapping forward acceleration used during approach phase two.
    pub forward_acceleration: u16,
    /// Current projection angle in the original angular units.
    pub projection_angle: u16,
    /// Wrapping selector for the eight authored hyperspace clips.
    pub hyperspace_sequence_index: u16,
    /// Selected hyperspace resource name, without a C string terminator.
    pub hyperspace_resource: Box<[u8]>,
    /// Active script/presentation line.
    pub active_line: u16,
    /// Whether the scene presentation callback is still busy.
    pub presentation_pending: bool,
}

impl Default for CameraApproachState {
    fn default() -> Self {
        Self {
            phase: u8::MIN,
            actor_presentation: CameraApproachPresentation::Inactive,
            camera_view_active: false,
            transition_pending: false,
            ui_active: false,
            camera: [CAMERA_X_RESET, CAMERA_Y_RESET, i16::MIN],
            forward_acceleration: u16::MIN,
            projection_angle: u16::MIN,
            hyperspace_sequence_index: u16::MIN,
            hyperspace_resource: Box::default(),
            active_line: NO_PRESENTATION_LINE,
            presentation_pending: false,
        }
    }
}

/// Read-only resources required by one camera-approach update.
#[derive(Clone, Copy, Debug)]
pub struct CameraApproachContext<'a, SceneLink> {
    /// Scene dispatched after the hyperspace clip has been queued.
    pub scene_link: &'a SceneLink,
    /// Complete authored cycle of hyperspace resource names.
    pub hyperspace_resources: &'a [Box<[u8]>; HYPERSPACE_SEQUENCE_COUNT],
}

/// Rendering and scene services called by the camera-approach coordinator.
pub trait CameraApproachHost<SceneLink> {
    /// Host-specific scene dispatch failure.
    type Error;

    /// Publish a write to the navigation actor/MANU3 selector alias.
    fn publish_actor_presentation(&mut self, presentation: CameraApproachPresentation);

    /// Rebuild shared screen flags after entering or leaving the transition.
    fn initialize_screen_flags(&mut self, transition_pending: bool);

    /// Mark the inclusive ship entity range for a state transition.
    fn mark_entity_range_dirty(&mut self, first: u16, last: u16);

    /// Apply one entity state transition.
    fn transition_entity(&mut self, entity: u16);

    /// Dispatch the linked scene, which may complete the presentation gate.
    fn dispatch_scene(
        &mut self,
        scene_link: &SceneLink,
        presentation_pending: &mut bool,
    ) -> Result<(), Self::Error>;

    /// Draw the ship HUD, capture its palette, and reset the camera position.
    fn snapshot_ship_hud_and_reset_camera(&mut self, camera: &mut [i16; 3]);

    /// Clear the ship projection row before rendering a transition frame.
    fn clear_projection_row(&mut self, color: u8);

    /// Rebuild the ship projection matrix from the coordinator's flat camera state.
    fn build_projection_matrix(&mut self, camera: [i16; 3], projection_angle: u16);

    /// Project the ship point cloud.
    fn project_point_cloud(&mut self);

    /// Project ship object sprites.
    fn project_object_sprites(&mut self);
}

/// Terminal path taken by one camera-approach update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraApproachOutcome {
    /// The camera changed and a transition frame was rendered.
    FrameRendered,
    /// A hyperspace clip was selected for presentation.
    HyperspaceQueued,
    /// The linked scene remains busy.
    WaitingForPresentation,
    /// Scene presentation completed and final camera easing can begin.
    PresentationCompleted,
    /// Camera easing reached zero and the transition was closed.
    TransitionCompleted,
}

/// Advance the ship-camera approach and hyperspace transition by one frame.
///
/// This translates `camera_fsm_state_gate` at BLOODPRG routine offset
/// `0x008A4E`. Typed presentation modes, owned resource names, booleans, flat
/// camera coordinates, and explicit host operations replace packed global
/// words, a writable C-string suffix, and segment-owned renderer state. The
/// source's signed-X and unsigned-Z comparisons and wrapping word arithmetic
/// are retained because they affect authored transition timing.
pub fn update_camera_approach<SceneLink, Host: CameraApproachHost<SceneLink>>(
    state: &mut CameraApproachState,
    context: CameraApproachContext<'_, SceneLink>,
    host: &mut Host,
) -> Result<CameraApproachOutcome, Host::Error> {
    if state.phase & PHASE_MASK == 0 {
        state.actor_presentation = CameraApproachPresentation::Active;
        host.publish_actor_presentation(state.actor_presentation);
        state.camera_view_active = false;
        state.transition_pending = true;
        host.initialize_screen_flags(state.transition_pending);
        state.phase = state.phase.wrapping_add(1);
        state.ui_active = true;
    }

    match state.phase {
        1 => {
            if state.camera[0] >= CAMERA_X_LIMIT {
                state.camera[0] = state.camera[0].wrapping_sub(CAMERA_X_STEP);
                let angle = state.projection_angle.wrapping_sub(1);
                state.projection_angle = if angle as i16 >= 0 {
                    angle
                } else {
                    PROJECTION_ANGLE_WRAP
                };
            } else {
                state.phase = state.phase.wrapping_add(1);
            }
        }
        2 => {
            let camera_z = state.camera[2] as u16;
            if camera_z <= CAMERA_Z_CRUISE {
                state.camera[2] = camera_z.wrapping_add(state.forward_acceleration) as i16;
                state.forward_acceleration = state
                    .forward_acceleration
                    .wrapping_add(CAMERA_Z_ACCELERATION_STEP);
            } else {
                host.mark_entity_range_dirty(FIRST_DIRTY_ENTITY, LAST_DIRTY_ENTITY);
                state.phase = state.phase.wrapping_add(1);
            }
        }
        3 => {
            state.actor_presentation = CameraApproachPresentation::Suppressed;
            host.publish_actor_presentation(state.actor_presentation);
            host.transition_entity(TRANSITION_ENTITY);
            state.camera[2] = CAMERA_Z_CRUISE as i16;
            state.projection_angle = 0;
            state.camera[0] = CAMERA_X_RESET;

            let resource_index = usize::from(state.hyperspace_sequence_index & PHASE_MASK as u16);
            state.hyperspace_sequence_index = state.hyperspace_sequence_index.wrapping_add(1);
            state.hyperspace_resource = context.hyperspace_resources[resource_index].clone();
            state.active_line = HYPERSPACE_ACTIVE_LINE;
            state.phase = state.phase.wrapping_add(1);
            return Ok(CameraApproachOutcome::HyperspaceQueued);
        }
        4 => {
            host.dispatch_scene(context.scene_link, &mut state.presentation_pending)?;
            if state.presentation_pending {
                return Ok(CameraApproachOutcome::WaitingForPresentation);
            }

            state.actor_presentation = CameraApproachPresentation::Inactive;
            host.publish_actor_presentation(state.actor_presentation);
            host.transition_entity(TRANSITION_ENTITY);
            host.snapshot_ship_hud_and_reset_camera(&mut state.camera);
            host.initialize_screen_flags(state.transition_pending);
            state.phase = state.phase.wrapping_add(1);
            state.camera[2] = CAMERA_Z_FINAL;
            return Ok(CameraApproachOutcome::PresentationCompleted);
        }
        _ => {
            let camera_z = state.camera[2] as u16;
            let easing_step = 0_u16.wrapping_sub(camera_z) >> 2;
            if easing_step != 0 {
                state.camera[2] = camera_z.wrapping_add(easing_step) as i16;
            } else {
                state.forward_acceleration = COMPLETED_ACCELERATION;
                state.camera[2] = 0;
                state.transition_pending = false;
                state.phase = 0;
                state.ui_active = false;
                host.initialize_screen_flags(state.transition_pending);
                state.actor_presentation = CameraApproachPresentation::Active;
                host.publish_actor_presentation(state.actor_presentation);
                return Ok(CameraApproachOutcome::TransitionCompleted);
            }
        }
    }

    host.clear_projection_row(RENDER_CLEAR_COLOR);
    host.build_projection_matrix(state.camera, state.projection_angle);
    host.project_point_cloud();
    host.project_object_sprites();
    Ok(CameraApproachOutcome::FrameRendered)
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use serde::Deserialize;

    use super::*;

    const ORACLE_VECTOR_COUNT: usize = 14;

    #[derive(Debug, Deserialize)]
    struct OracleVector {
        name: String,
        initial: OracleState,
        final_state: OracleFinalState,
        calls: Vec<OracleCall>,
    }

    #[derive(Debug, Deserialize)]
    struct OracleState {
        phase: u8,
        presentation: u16,
        camera_active: u8,
        transition: u8,
        ui: u16,
        camera_x: u16,
        camera_y: u16,
        camera_z: u16,
        acceleration: u16,
        angle: u16,
        sequence_index: u16,
        presentation_gate: u8,
        active_line: u16,
        scene_link_target: u16,
    }

    #[derive(Debug, Deserialize)]
    struct OracleFinalState {
        phase: u8,
        presentation: u16,
        camera_active: u8,
        transition: u8,
        ui: u16,
        camera_x: u16,
        camera_y: u16,
        camera_z: u16,
        acceleration: u16,
        angle: u16,
        sequence_index: u16,
        presentation_gate: u8,
        active_line: u16,
        filename_hex: String,
    }

    #[derive(Debug, Deserialize)]
    struct OracleCall {
        call: String,
        #[serde(default)]
        first_object_id: Option<u16>,
        #[serde(default)]
        last_object_id: Option<u16>,
        #[serde(default)]
        object_id: Option<u16>,
        #[serde(default)]
        link_target: Option<u16>,
        #[serde(default)]
        color: Option<u8>,
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    enum HostCall {
        InitializeScreenFlags,
        MarkEntityRange(u16, u16),
        TransitionEntity(u16),
        DispatchScene(u16),
        SnapshotHud,
        ClearProjectionRow(u8),
        BuildProjectionMatrix,
        ProjectPointCloud,
        ProjectObjectSprites,
    }

    struct OracleHost {
        calls: Vec<HostCall>,
        completed_gate: Option<bool>,
    }

    impl CameraApproachHost<u16> for OracleHost {
        type Error = Infallible;

        fn publish_actor_presentation(&mut self, _presentation: CameraApproachPresentation) {}

        fn initialize_screen_flags(&mut self, _transition_pending: bool) {
            self.calls.push(HostCall::InitializeScreenFlags);
        }

        fn mark_entity_range_dirty(&mut self, first: u16, last: u16) {
            self.calls.push(HostCall::MarkEntityRange(first, last));
        }

        fn transition_entity(&mut self, entity: u16) {
            self.calls.push(HostCall::TransitionEntity(entity));
        }

        fn dispatch_scene(
            &mut self,
            scene_link: &u16,
            presentation_pending: &mut bool,
        ) -> Result<(), Self::Error> {
            self.calls.push(HostCall::DispatchScene(*scene_link));
            if let Some(completed_gate) = self.completed_gate {
                *presentation_pending = completed_gate;
            }
            Ok(())
        }

        fn snapshot_ship_hud_and_reset_camera(&mut self, camera: &mut [i16; 3]) {
            self.calls.push(HostCall::SnapshotHud);
            *camera = [10_000, 12_000, 0];
        }

        fn clear_projection_row(&mut self, color: u8) {
            self.calls.push(HostCall::ClearProjectionRow(color));
        }

        fn build_projection_matrix(&mut self, _camera: [i16; 3], _projection_angle: u16) {
            self.calls.push(HostCall::BuildProjectionMatrix);
        }

        fn project_point_cloud(&mut self) {
            self.calls.push(HostCall::ProjectPointCloud);
        }

        fn project_object_sprites(&mut self) {
            self.calls.push(HostCall::ProjectObjectSprites);
        }
    }

    #[test]
    fn every_original_camera_approach_vector_matches() {
        let json = include_str!("../../../../../re/tools/oracle_vectors/func_8a4e_natural.json")
            .replace("\"final\":", "\"final_state\":");
        let vectors: Vec<OracleVector> = serde_json::from_str(&json).unwrap();
        assert_eq!(vectors.len(), ORACLE_VECTOR_COUNT);
        let resources = hyperspace_resources();

        for vector in vectors {
            let expected_calls = vector
                .calls
                .iter()
                .map(expected_host_call)
                .collect::<Vec<_>>();
            let expected_resource = filename_bytes(&vector.final_state.filename_hex);
            let initial_resource = Box::<[u8]>::from([0_u8; 16]);
            let mut state = CameraApproachState {
                phase: vector.initial.phase,
                actor_presentation: CameraApproachPresentation::from_id(
                    vector.initial.presentation,
                ),
                camera_view_active: vector.initial.camera_active != 0,
                transition_pending: vector.initial.transition != 0,
                ui_active: vector.initial.ui & 4 != 0,
                camera: [
                    vector.initial.camera_x as i16,
                    vector.initial.camera_y as i16,
                    vector.initial.camera_z as i16,
                ],
                forward_acceleration: vector.initial.acceleration,
                projection_angle: vector.initial.angle,
                hyperspace_sequence_index: vector.initial.sequence_index,
                hyperspace_resource: initial_resource,
                active_line: vector.initial.active_line,
                presentation_pending: vector.initial.presentation_gate & 1 != 0,
            };
            let mut host = OracleHost {
                calls: Vec::new(),
                completed_gate: match vector.name.as_str() {
                    "phase_four_callback_completes" => Some(false),
                    _ => None,
                },
            };

            let outcome = update_camera_approach(
                &mut state,
                CameraApproachContext {
                    scene_link: &vector.initial.scene_link_target,
                    hyperspace_resources: &resources,
                },
                &mut host,
            )
            .unwrap();

            assert_eq!(state.phase, vector.final_state.phase, "{}", vector.name);
            assert_eq!(
                state.actor_presentation.id(),
                vector.final_state.presentation,
                "{}",
                vector.name
            );
            assert_eq!(
                state.camera_view_active,
                vector.final_state.camera_active != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.transition_pending,
                vector.final_state.transition != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.ui_active,
                vector.final_state.ui & 4 != 0,
                "{}",
                vector.name
            );
            assert_eq!(
                state.camera.map(|coordinate| coordinate as u16),
                [
                    vector.final_state.camera_x,
                    vector.final_state.camera_y,
                    vector.final_state.camera_z,
                ],
                "{}",
                vector.name
            );
            assert_eq!(
                state.forward_acceleration, vector.final_state.acceleration,
                "{}",
                vector.name
            );
            assert_eq!(
                state.projection_angle, vector.final_state.angle,
                "{}",
                vector.name
            );
            assert_eq!(
                state.hyperspace_sequence_index, vector.final_state.sequence_index,
                "{}",
                vector.name
            );
            assert_eq!(
                state.active_line, vector.final_state.active_line,
                "{}",
                vector.name
            );
            assert_eq!(
                state.presentation_pending,
                vector.final_state.presentation_gate & 1 != 0,
                "{}",
                vector.name
            );
            if vector.name == "phase_three_selects_wrapped_hyperspace_name" {
                assert_eq!(
                    state.hyperspace_resource.as_ref(),
                    expected_resource.as_slice()
                );
            } else {
                assert_eq!(state.hyperspace_resource.as_ref(), &[0_u8; 16]);
            }
            assert_eq!(host.calls, expected_calls, "{}", vector.name);
            assert_eq!(outcome, expected_outcome(&vector.name));
        }
    }

    fn hyperspace_resources() -> [Box<[u8]>; HYPERSPACE_SEQUENCE_COUNT] {
        std::array::from_fn(|index| {
            format!("hyper_{index:02}.hnm")
                .into_bytes()
                .into_boxed_slice()
        })
    }

    fn filename_bytes(encoded: &str) -> Vec<u8> {
        encoded
            .as_bytes()
            .chunks_exact(2)
            .map(|digits| u8::from_str_radix(std::str::from_utf8(digits).unwrap(), 16).unwrap())
            .take_while(|byte| *byte != 0)
            .collect()
    }

    fn expected_host_call(call: &OracleCall) -> HostCall {
        match call.call.as_str() {
            "screen_flags_init" => HostCall::InitializeScreenFlags,
            "sprite_slot_range_mark_dirty" => HostCall::MarkEntityRange(
                call.first_object_id.unwrap(),
                call.last_object_id.unwrap(),
            ),
            "entity_flag_state_transition" => HostCall::TransitionEntity(call.object_id.unwrap()),
            "dlg_line_id_scene_dispatch" => HostCall::DispatchScene(call.link_target.unwrap()),
            "ship_3d_hud_palette_snapshot_and_camera_reset" => HostCall::SnapshotHud,
            "blit_fill_row_5221" => HostCall::ClearProjectionRow(call.color.unwrap()),
            "ship_3d_projection_matrix_build" => HostCall::BuildProjectionMatrix,
            "ship_3d_point_cloud_project" => HostCall::ProjectPointCloud,
            "ship_3d_object_sprite_project" => HostCall::ProjectObjectSprites,
            unknown => panic!("unknown oracle call {unknown}"),
        }
    }

    fn expected_outcome(name: &str) -> CameraApproachOutcome {
        match name {
            "phase_three_selects_wrapped_hyperspace_name" => {
                CameraApproachOutcome::HyperspaceQueued
            }
            "phase_four_waits_for_presentation" => CameraApproachOutcome::WaitingForPresentation,
            "phase_four_callback_completes" | "phase_four_high_gate_bit_does_not_wait" => {
                CameraApproachOutcome::PresentationCompleted
            }
            "default_zero_finishes_transition" | "masked_phase_eight_initializes_then_finishes" => {
                CameraApproachOutcome::TransitionCompleted
            }
            _ => CameraApproachOutcome::FrameRendered,
        }
    }
}
