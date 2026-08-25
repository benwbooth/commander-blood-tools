//! Bounds and camera-selection transitions shared by alien wave callbacks.

use std::fmt;

use commander_blood_formats::alien::{
    AXIS_COUNT, AlienNodeParent, AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT,
};

use super::{
    AlienCallbackSceneState, AlienControlLatch, AlienModelPose, AlienRingAnimationState,
    AlienRingCallback, AlienSceneNode, AlienSpecies, AlienWaveSelection,
};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const LATERAL_BOUND: i16 = 64;
const DEPTH_BOUND: u16 = 128;
const ACTIVE_CALLBACK_COUNTDOWN: u16 = 5;
const SELECTED_DEPTH_LOW_WORD: u16 = 32;
const ZERO_RADIAL_OFFSET: i16 = 0;
const LOW_WORD_MASK: u32 = u16::MAX as u32;
const HIGH_WORD_MASK: u32 = !LOW_WORD_MASK;
const AMER_AND_SCRUT_PULSE_ADVANCE: [i32; AXIS_COUNT] = [0, 30, 35];
const CROOLIS_PULSE_ADVANCE: [i32; AXIS_COUNT] = [25, 30, 35];
const WAVE_ANCHOR_MODEL_INDEX: usize = 0;
const WAVE_ANCHOR_NODE_INDEX: usize = 3;
const WAVE_PITCH: u16 = 0;
const WAVE_PAN: u16 = 0x0800;
const WAVE_SECONDARY_PAN_STEP: u16 = 53;
const METHOD_DELTA_STEP: u16 = 8;
const METHOD_DELTA_LIMIT: u16 = 128;
const METHOD_DELTA_MAXIMUM: u16 = 127;
const FINISH_CALLBACK_COUNTDOWN: u16 = 4;
const FINISH_SAMPLE_BIAS: u16 = 176;
const FINISH_ANGLE_ADVANCE: [u16; AXIS_COUNT] = [160, 208, 224];
const CAMERA_ANGLE_MASK: u16 = 0x0ffc;
const CAMERA_MOTION_SHIFT: u32 = 4;
const WAVE_MOTION_PHASE_STEP: i16 = 1;
const WAVE_MOTION_PHASE_LIMIT: i16 = 15;
const WAVE_RETURN_COUNTDOWN: i16 = 64;
const WAVE_RETURN_COUNTDOWN_STEP: i16 = 1;
const COMPLETED_RETURN_COUNTDOWN: i16 = 0;
const STEERING_RADIAL_OFFSET: i16 = 12;
const STEERING_COUNTDOWN_STEP: i16 = 1;
const STEERING_DISTANCE_LIMIT: i16 = 1_000;
const STEERING_HALF_TURN: u16 = 0x0800;
const STEERING_QUARTER_TURN: u16 = 0x0400;
const STEERING_ANGLE_MASK: u16 = 0x0ffc;
const FIXED_STEERING_COUNTDOWN: i16 = 16;
const FIXED_STEERING_DIVISOR: i16 = 32;
const FIXED_STEERING_SHIFT: u32 = 2;
const RANDOM_STEERING_MASK: u16 = 0x07ff;
const RANDOM_STEERING_BIAS: u16 = 0x03ff;
const RANDOM_STEERING_DIVISOR_SHIFT: u32 = 1;
const RANDOM_STEERING_DIVISOR_BIAS: u16 = 16;
const RANDOM_STEERING_ROTATION: u32 = 3;
const RANDOM_STEERING_BORROW_SHIFT: u32 = 2;
const RANDOM_STEERING_BORROW_MASK: u16 = 1;
const STEERING_PAN_SHIFT: u32 = 5;
const STEERING_SAMPLE_PHASE_STEP: u16 = 0x0080;
const STEERING_SAMPLE_PHASE_MASK: u16 = 0x0ffc;
const STEERING_SAMPLE_INDEX_SHIFT: u32 = 2;
const STEERING_SAMPLE_SHIFT: u32 = 5;

/// Typed continuation selected by the slot-1 bounds callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSelectionUpdate {
    /// Continue the separately recovered out-of-bounds motion callback.
    MotionContinuationRequested,
    /// Continue the separately recovered camera-update callback.
    CameraUpdateRequested,
    /// The node was prepared for its wave callback.
    WaveStarted,
}

/// Typed continuation selected by the active wave callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveCallbackUpdate {
    /// No model has completed the requested wave selection yet.
    Waiting,
    /// Continue the separately recovered wave-finish callback.
    FinishRequested,
    /// Continue the separately recovered camera-update callback.
    CameraUpdateRequested,
}

/// Stage completed by one per-frame wave camera-motion callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveMotionUpdate {
    /// Camera-relative angular motion remains active.
    Moving,
    /// Motion completed and the delayed return callback was selected.
    ReturnDelayStarted,
}

/// Stage completed by one delayed wave-return callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienWaveReturnUpdate {
    /// The return countdown remains active.
    Waiting,
    /// The countdown completed and wave selection was requested.
    SelectionRequested,
}

/// Invalid flat state supplied to the slot-1 selection callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSelectionError {
    /// Pose nodes and parallel callback states must have identical lengths.
    NodeStateCountMismatch {
        /// Nodes available in the mutable model pose.
        pose: usize,
        /// Parallel callback states available in the animation state.
        animation: usize,
    },
    /// The callback selected a node outside the typed model hierarchy.
    InvalidNodeIndex {
        /// Invalid node supplied by the caller.
        node_index: usize,
        /// Number of nodes available in the hierarchy.
        node_count: usize,
    },
    /// A completed selection did not publish its typed scene node.
    MissingSelectedSceneNode,
}

impl fmt::Display for AlienSelectionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien selection state: {self:?}")
    }
}

impl std::error::Error for AlienSelectionError {}

/// Apply the recovered slot-1 bounds and wave-selection callback.
///
/// Tail transfers to the motion and camera callbacks are returned explicitly;
/// no executable address is retained in the modern runtime.
pub fn update_wave_selection(
    species: AlienSpecies,
    model_index: usize,
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
    scene: &mut AlienCallbackSceneState,
) -> Result<AlienSelectionUpdate, AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    if node_outside_selection_bounds(&pose.nodes[node_index]) {
        return Ok(AlienSelectionUpdate::MotionContinuationRequested);
    }

    scene.control_latch = match species {
        AlienSpecies::Amer => AlienControlLatch::Signal,
        AlienSpecies::Croolis | AlienSpecies::Scrut => AlienControlLatch::Model(model_index),
    };
    if scene.wave_selection != AlienWaveSelection::Disabled {
        return Ok(AlienSelectionUpdate::CameraUpdateRequested);
    }

    scene.wave_selection = AlienWaveSelection::Requested;
    let node = &mut pose.nodes[node_index];
    node.scene_parent = Some(AlienSceneNode {
        model_index: WAVE_ANCHOR_MODEL_INDEX,
        node_index: WAVE_ANCHOR_NODE_INDEX,
    });
    node.radial_offset = ZERO_RADIAL_OFFSET;
    node.local_position[X_AXIS] = replace_low_word(node.local_position[X_AXIS], u16::MIN);
    node.local_position[Y_AXIS] = replace_low_word(node.local_position[Y_AXIS], u16::MIN);
    node.local_position[Z_AXIS] =
        replace_low_word(node.local_position[Z_AXIS], SELECTED_DEPTH_LOW_WORD);

    for (pulse, advance) in scene
        .palette_pulses
        .iter_mut()
        .zip(palette_pulse_advances(species))
    {
        *pulse = pulse.wrapping_add(advance);
    }
    animation.nodes[node_index].callback = AlienRingCallback::Wave;
    scene.callback_countdown = ACTIVE_CALLBACK_COUNTDOWN;
    Ok(AlienSelectionUpdate::WaveStarted)
}

/// Apply one recovered slot-1 wave callback up to its typed continuation.
pub fn update_wave_callback(
    species: AlienSpecies,
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
    scene: &mut AlienCallbackSceneState,
    view: [i16; AXIS_COUNT],
) -> Result<AlienWaveCallbackUpdate, AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    let node = &mut pose.nodes[node_index];
    if !scene.slot2_active {
        node.angles[X_AXIS] = WAVE_PITCH;
        node.angles[Y_AXIS] = WAVE_PAN;
        node.angles[Z_AXIS] = node.angles[Z_AXIS].wrapping_add(WAVE_SECONDARY_PAN_STEP);
        if scene.wave_selection != AlienWaveSelection::Selected {
            return Ok(AlienWaveCallbackUpdate::Waiting);
        }

        let selected_node = scene
            .wave_selected_node
            .ok_or(AlienSelectionError::MissingSelectedSceneNode)?;
        let next_delta = (scene.method_delta as u16).wrapping_add(METHOD_DELTA_STEP);
        scene.method_delta = if next_delta >= METHOD_DELTA_LIMIT {
            METHOD_DELTA_MAXIMUM as i16
        } else {
            next_delta as i16
        };
        node.scene_parent = Some(selected_node);
        animation.nodes[node_index].callback = AlienRingCallback::WaveFinish;
        scene.wave_selection = AlienWaveSelection::Disabled;
        subtract_palette_pulses(species, &mut scene.palette_pulses);
        if species != AlienSpecies::Amer {
            scene.slot2_active = false;
        }
        scene.callback_countdown = FINISH_CALLBACK_COUNTDOWN;
        return Ok(AlienWaveCallbackUpdate::FinishRequested);
    }

    scene.wave_selection = AlienWaveSelection::Disabled;
    subtract_palette_pulses(species, &mut scene.palette_pulses);
    node.parent = AlienNodeParent::SceneCamera;
    node.scene_parent = None;
    node.local_position = view.map(|component| -i32::from(component));
    Ok(AlienWaveCallbackUpdate::CameraUpdateRequested)
}

/// Apply the recovered selected-wave pose update.
///
/// The original routine read a module-global sample and replaced only the low
/// word of the node's Y position. The flat runtime receives that sample as a
/// normal value while retaining the exact wrapping arithmetic.
pub fn update_wave_finish(
    node_index: usize,
    current_sample: u16,
    pose: &mut AlienModelPose,
) -> Result<(), AlienSelectionError> {
    validate_pose_node(node_index, pose)?;
    let node = &mut pose.nodes[node_index];
    node.local_position[Y_AXIS] = replace_low_word(
        node.local_position[Y_AXIS],
        current_sample.wrapping_sub(FINISH_SAMPLE_BIAS),
    );
    for (angle, advance) in node.angles.iter_mut().zip(FINISH_ANGLE_ADVANCE) {
        *angle = angle.wrapping_add(advance);
    }
    Ok(())
}

/// Prepare the recovered per-frame motion toward the camera orientation.
///
/// Executable callback addresses from the DOS overlay become a typed callback
/// variant, and the reused C structure words become named motion fields.
pub fn update_wave_camera(
    node_index: usize,
    camera_pan: u16,
    pose: &AlienModelPose,
    animation: &mut AlienRingAnimationState,
) -> Result<(), AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    let node = &pose.nodes[node_index];
    let masked_camera_pan = camera_pan & CAMERA_ANGLE_MASK;
    let masked_node_pan = node.angles[Y_AXIS] & CAMERA_ANGLE_MASK;
    let pan_delta = masked_camera_pan.wrapping_sub(masked_node_pan) as i16;
    let state = &mut animation.nodes[node_index];
    state.wave_pan_step = pan_delta >> CAMERA_MOTION_SHIFT;
    state.wave_roll_step = (node.angles[Z_AXIS] as i16) >> CAMERA_MOTION_SHIFT;
    state.callback = AlienRingCallback::WaveMotion;
    Ok(())
}

/// Integrate one recovered frame of camera-relative wave motion.
pub fn update_wave_motion(
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
) -> Result<AlienWaveMotionUpdate, AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    let state = &mut animation.nodes[node_index];
    let node = &mut pose.nodes[node_index];
    node.angles[Y_AXIS] = node.angles[Y_AXIS].wrapping_add(state.wave_pan_step as u16);
    node.angles[Z_AXIS] = node.angles[Z_AXIS].wrapping_sub(state.wave_roll_step as u16);
    node.radial_offset = node.radial_offset.wrapping_add(WAVE_MOTION_PHASE_STEP);
    if node.radial_offset <= WAVE_MOTION_PHASE_LIMIT {
        return Ok(AlienWaveMotionUpdate::Moving);
    }

    node.radial_offset = WAVE_RETURN_COUNTDOWN;
    state.callback = AlienRingCallback::WaveReturn;
    Ok(AlienWaveMotionUpdate::ReturnDelayStarted)
}

/// Advance the recovered delay before resuming wave target selection.
pub fn update_wave_return(
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
) -> Result<AlienWaveReturnUpdate, AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    let node = &mut pose.nodes[node_index];
    node.radial_offset = node.radial_offset.wrapping_sub(WAVE_RETURN_COUNTDOWN_STEP);
    if node.radial_offset != COMPLETED_RETURN_COUNTDOWN {
        return Ok(AlienWaveReturnUpdate::Waiting);
    }

    animation.nodes[node_index].callback = AlienRingCallback::WaveSelection;
    Ok(AlienWaveReturnUpdate::SelectionRequested)
}

/// Continue autonomous steering for a node outside wave-selection bounds.
///
/// The decoded cosine table and view vector replace the original process-wide
/// data references. All callback state is owned by the typed model animation.
pub fn continue_wave_steering(
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
    view: [i16; AXIS_COUNT],
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Result<(), AlienSelectionError> {
    validate_node(node_index, pose, animation)?;
    let node = &mut pose.nodes[node_index];
    let steering = &mut animation.nodes[node_index].steering;
    node.radial_offset = STEERING_RADIAL_OFFSET;
    steering.turn_countdown = steering
        .turn_countdown
        .wrapping_sub(STEERING_COUNTDOWN_STEP);

    if steering.turn_countdown < COMPLETED_RETURN_COUNTDOWN {
        let z_distance = (node.local_position[Z_AXIS] as i16).wrapping_add(view[Z_AXIS]);
        let mut target = node.angles[Y_AXIS].wrapping_add(STEERING_HALF_TURN);
        let generated_turn;
        let divisor;
        if z_distance < -STEERING_DISTANCE_LIMIT {
            (generated_turn, divisor) = fixed_steering_target(target);
            steering.turn_countdown = FIXED_STEERING_COUNTDOWN;
        } else {
            target = target.wrapping_add(STEERING_HALF_TURN);
            if z_distance > STEERING_DISTANCE_LIMIT {
                (generated_turn, divisor) = fixed_steering_target(target);
                steering.turn_countdown = FIXED_STEERING_COUNTDOWN;
            } else {
                let x_distance = (node.local_position[X_AXIS] as i16).wrapping_add(view[X_AXIS]);
                target = target.wrapping_add(STEERING_QUARTER_TURN);
                if x_distance < -STEERING_DISTANCE_LIMIT {
                    (generated_turn, divisor) = fixed_steering_target(target);
                    steering.turn_countdown = FIXED_STEERING_COUNTDOWN;
                } else {
                    target = target.wrapping_add(STEERING_HALF_TURN);
                    if x_distance >= STEERING_DISTANCE_LIMIT {
                        (generated_turn, divisor) = fixed_steering_target(target);
                        steering.turn_countdown = FIXED_STEERING_COUNTDOWN;
                    } else {
                        steering.random_seed = random_steering_step(steering.random_seed);
                        generated_turn = (steering.random_seed & RANDOM_STEERING_MASK)
                            .wrapping_sub(RANDOM_STEERING_BIAS)
                            as i16;
                        let magnitude = generated_turn.unsigned_abs();
                        let generated_divisor = (magnitude >> RANDOM_STEERING_DIVISOR_SHIFT)
                            .wrapping_add(RANDOM_STEERING_DIVISOR_BIAS);
                        steering.turn_countdown = generated_divisor as i16;
                        divisor = generated_divisor as i16;
                    }
                }
            }
        }
        steering.turn_step = generated_turn.wrapping_sub(steering.turn_offset) / divisor;
    }

    steering.turn_offset = steering.turn_step.wrapping_add(steering.turn_offset);
    node.angles[Y_AXIS] =
        node.angles[Y_AXIS].wrapping_add((steering.turn_offset >> STEERING_PAN_SHIFT) as u16);
    steering.sample_phase = steering
        .sample_phase
        .wrapping_add(STEERING_SAMPLE_PHASE_STEP)
        & STEERING_SAMPLE_PHASE_MASK;
    let sample_index = usize::from(steering.sample_phase >> STEERING_SAMPLE_INDEX_SHIFT);
    let roll_feedback = trigonometry[sample_index].cosine >> STEERING_SAMPLE_SHIFT;
    node.angles[Z_AXIS] = steering.turn_offset.wrapping_add(roll_feedback) as u16;
    Ok(())
}

fn fixed_steering_target(target: u16) -> (i16, i16) {
    let centered = (target & STEERING_ANGLE_MASK).wrapping_sub(STEERING_HALF_TURN) as i16;
    (
        (centered >> FIXED_STEERING_SHIFT).wrapping_neg(),
        FIXED_STEERING_DIVISOR,
    )
}

fn random_steering_step(value: u16) -> u16 {
    value
        .rotate_right(RANDOM_STEERING_ROTATION)
        .wrapping_sub((value >> RANDOM_STEERING_BORROW_SHIFT) & RANDOM_STEERING_BORROW_MASK)
}

fn validate_node(
    node_index: usize,
    pose: &AlienModelPose,
    animation: &AlienRingAnimationState,
) -> Result<(), AlienSelectionError> {
    if pose.nodes.len() != animation.nodes.len() {
        return Err(AlienSelectionError::NodeStateCountMismatch {
            pose: pose.nodes.len(),
            animation: animation.nodes.len(),
        });
    }
    validate_pose_node(node_index, pose)
}

fn validate_pose_node(node_index: usize, pose: &AlienModelPose) -> Result<(), AlienSelectionError> {
    (node_index < pose.nodes.len())
        .then_some(())
        .ok_or(AlienSelectionError::InvalidNodeIndex {
            node_index,
            node_count: pose.nodes.len(),
        })
}

fn node_outside_selection_bounds(node: &super::AlienNodePose) -> bool {
    let x = fixed_integer_word(node.transform.translation[X_AXIS]);
    let y = fixed_integer_word(node.transform.translation[Y_AXIS]);
    let z = (node.transform.translation[Z_AXIS] as u32 >> u16::BITS) as u16;
    z > DEPTH_BOUND
        || !(-LATERAL_BOUND..=LATERAL_BOUND).contains(&x)
        || !(-LATERAL_BOUND..=LATERAL_BOUND).contains(&y)
}

fn fixed_integer_word(value: i32) -> i16 {
    (value >> u16::BITS) as i16
}

fn replace_low_word(value: i32, low_word: u16) -> i32 {
    ((value as u32 & HIGH_WORD_MASK) | u32::from(low_word)) as i32
}

fn palette_pulse_advances(species: AlienSpecies) -> [i32; AXIS_COUNT] {
    match species {
        AlienSpecies::Amer | AlienSpecies::Scrut => AMER_AND_SCRUT_PULSE_ADVANCE,
        AlienSpecies::Croolis => CROOLIS_PULSE_ADVANCE,
    }
}

fn subtract_palette_pulses(species: AlienSpecies, pulses: &mut [i32; AXIS_COUNT]) {
    for (pulse, advance) in pulses.iter_mut().zip(palette_pulse_advances(species)) {
        *pulse = pulse.wrapping_sub(advance);
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienTransformData};
    use serde::Deserialize;

    use super::*;
    use crate::native::alien::{AlienNodePose, AlienProjectedVertex};

    const FIRST_NODE: usize = 0;
    const SINGLE_NODE_COUNT: usize = 1;
    const MODEL_INDEX: usize = 7;
    const ORIGINAL_CONTEXT_OFFSET: u16 = 0x3000;
    const ORIGINAL_WAVE_ANCHOR_OFFSET: u16 = 0x25A8;
    const ORIGINAL_CALLBACK_COUNTDOWN: u16 = 0x7777;
    const ORIGINAL_PARENT_SENTINEL: u16 = 0x4444;
    const ORIGINAL_WAVE_PARENT_SENTINEL: u16 = 0x1111;
    const ORIGINAL_WAVE_CALLBACK_SENTINEL: u16 = 0x2222;
    const ORIGINAL_LEAF_CALLBACK_SENTINEL: u16 = 0x5555;
    const ORIGINAL_STEERING_ROLL_SENTINEL: u16 = 0x6666;
    const ORIGINAL_STEERING_RADIAL_SENTINEL: u16 = 0x7777;
    const ORIGINAL_SELECTED_NODE: u16 = 0x3456;
    const ORIGINAL_SCENE_CAMERA_OFFSET: u16 = 0x22A8;
    const UNCHANGED_PULSE: i32 = 0x1357_9BDF;
    const FIXED_FRACTION_SAMPLE: i32 = 0x5678;
    const SELECTED_MODEL_INDEX: usize = 6;
    const SELECTED_NODE_INDEX: usize = 2;

    #[derive(Deserialize)]
    struct SelectionVector {
        name: String,
        module: String,
        translation_integer_words: [i32; AXIS_COUNT],
        selection_before: u16,
        selection_after: u16,
        control_latch_after: u16,
        callback_countdown_after: u16,
        parent_after: u16,
        radial_after: u16,
        position_before: [u32; AXIS_COUNT],
        position_after: [u32; AXIS_COUNT],
        pulse_before: Vec<u32>,
        pulse_after: Vec<u32>,
        expected_action: String,
    }

    #[derive(Deserialize)]
    struct WaveCallbackVector {
        name: String,
        module: String,
        active_before: u16,
        active_after: u16,
        selection_before: u16,
        selection_after: u16,
        delta_before: u16,
        delta_after: u16,
        selected_state: u16,
        view: [i16; AXIS_COUNT],
        motion_before: [u32; 6],
        motion_after: [u32; 6],
        owner_after: u16,
        callback_after: u16,
        countdown_after: u16,
        pulse_before: Vec<u32>,
        pulse_after: Vec<u32>,
        expected_action: String,
    }

    #[derive(Deserialize)]
    struct WaveFinishVector {
        name: String,
        module: String,
        current_sample: u16,
        position_y_before: u32,
        position_y_after: u32,
        angles_before: [u16; AXIS_COUNT],
        angles_after: [u16; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct WaveCameraVector {
        name: String,
        module: String,
        camera_pan: u16,
        node_pan: u16,
        secondary_pan: u16,
        pan_step: u16,
        secondary_pan_step: u16,
        callback_after: u16,
    }

    #[derive(Deserialize)]
    struct WaveMotionVector {
        name: String,
        module: String,
        pan_before: u16,
        roll_before: u16,
        pan_step: u16,
        roll_step: u16,
        phase_before: u16,
        pan_after: u16,
        roll_after: u16,
        phase_after: u16,
        callback_before: u16,
        callback_after: u16,
        transitioned: bool,
    }

    #[derive(Deserialize)]
    struct WaveReturnVector {
        name: String,
        module: String,
        countdown_before: u16,
        countdown_after: u16,
        callback_before: u16,
        callback_after: u16,
        transitioned: bool,
    }

    #[derive(Deserialize)]
    struct WaveSteeringVector {
        name: String,
        module: String,
        position: [i16; AXIS_COUNT],
        view: [i16; AXIS_COUNT],
        pan_before: u16,
        pan_after: u16,
        roll_after: u16,
        countdown_before: u16,
        countdown_after: u16,
        turn_step_before: u16,
        turn_step_after: u16,
        turn_offset_before: u16,
        turn_offset_after: u16,
        sample_phase_before: u16,
        sample_phase_after: u16,
        sample: u16,
        random_seed_before: u16,
        random_seed_after: u16,
    }

    fn fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0bea_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0c3e_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0c32_natural.json"),
        ]
    }

    fn wave_callback_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0b37_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0b78_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0b78_natural.json"),
        ]
    }

    fn wave_finish_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0bd0_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0c24_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0c18_natural.json"),
        ]
    }

    fn wave_camera_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0c5d_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0cb5_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0ca3_natural.json"),
        ]
    }

    fn wave_motion_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0c81_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0cd9_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0cc7_natural.json"),
        ]
    }

    fn wave_return_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0ca1_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0cf9_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0ce7_natural.json"),
        ]
    }

    fn wave_steering_fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0cac_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0d04_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0cf2_natural.json"),
        ]
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn wave_selection(value: u16) -> AlienWaveSelection {
        match value {
            0 => AlienWaveSelection::Disabled,
            1 => AlienWaveSelection::Requested,
            2 => AlienWaveSelection::Selected,
            _ => panic!("unknown wave selection {value}"),
        }
    }

    fn selection_update(value: &str) -> AlienSelectionUpdate {
        match value {
            "motion" => AlienSelectionUpdate::MotionContinuationRequested,
            "camera" => AlienSelectionUpdate::CameraUpdateRequested,
            "selected" => AlienSelectionUpdate::WaveStarted,
            _ => panic!("unknown selection action {value}"),
        }
    }

    fn wave_callback_update(value: &str) -> AlienWaveCallbackUpdate {
        match value {
            "waiting" => AlienWaveCallbackUpdate::Waiting,
            "finish" => AlienWaveCallbackUpdate::FinishRequested,
            "camera" => AlienWaveCallbackUpdate::CameraUpdateRequested,
            _ => panic!("unknown wave callback action {value}"),
        }
    }

    fn finish_callback(module: &str) -> u16 {
        match module {
            "amer" => 0x0BD0,
            "croolis" => 0x0C24,
            "scrut" => 0x0C18,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn motion_callback(module: &str) -> u16 {
        match module {
            "amer" => 0x0C81,
            "croolis" => 0x0CD9,
            "scrut" => 0x0CC7,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn return_callback(module: &str) -> u16 {
        match module {
            "amer" => 0x0CA1,
            "croolis" => 0x0CF9,
            "scrut" => 0x0CE7,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn selection_callback(module: &str) -> u16 {
        match module {
            "amer" => 0x0BEA,
            "croolis" => 0x0C3E,
            "scrut" => 0x0C32,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn control_latch(value: u16) -> AlienControlLatch {
        match value {
            u16::MIN => AlienControlLatch::Inactive,
            1 => AlienControlLatch::Signal,
            ORIGINAL_CONTEXT_OFFSET => AlienControlLatch::Model(MODEL_INDEX),
            _ => panic!("unknown control latch {value:#06x}"),
        }
    }

    fn palette_pulses(module: &str, values: &[u32]) -> [i32; AXIS_COUNT] {
        match module {
            "croolis" => [values[0] as i32, values[1] as i32, values[2] as i32],
            "amer" | "scrut" => [UNCHANGED_PULSE, values[0] as i32, values[1] as i32],
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn fixed_with_integer_word(integer: i32) -> i32 {
        ((integer as i16 as i32) << u16::BITS) | FIXED_FRACTION_SAMPLE
    }

    fn pose(
        translation_integer_words: [i32; AXIS_COUNT],
        local_position: [u32; AXIS_COUNT],
    ) -> AlienModelPose {
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: vec![AlienNodePose {
                parent: AlienNodeParent::SceneCamera,
                scene_parent: None,
                first_vertex: usize::MIN,
                vertex_count: SINGLE_NODE_COUNT,
                transform: AlienTransformData {
                    translation: translation_integer_words.map(fixed_with_integer_word),
                    ..AlienTransformData::default()
                },
                local_position: local_position.map(|value| value as i32),
                angles: [u16::MIN; AXIS_COUNT],
                radial_offset: 0x6666,
            }],
            projected_vertices: vec![AlienProjectedVertex::default()],
            texture_coordinates: vec![[i16::MIN; 2]],
            object_positions: vec![[i16::MIN; AXIS_COUNT]],
            authored_vertex_count: SINGLE_NODE_COUNT,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    #[test]
    fn selection_state_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<SelectionVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let species = species(&vector.module);
                let mut pose = pose(vector.translation_integer_words, vector.position_before);
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);
                animation.nodes[FIRST_NODE].callback = AlienRingCallback::FollowCourse;
                let mut scene = AlienCallbackSceneState {
                    callback_countdown: ORIGINAL_CALLBACK_COUNTDOWN,
                    wave_selection: wave_selection(vector.selection_before),
                    palette_pulses: palette_pulses(&vector.module, &vector.pulse_before),
                    ..AlienCallbackSceneState::default()
                };

                let update = update_wave_selection(
                    species,
                    MODEL_INDEX,
                    FIRST_NODE,
                    &mut pose,
                    &mut animation,
                    &mut scene,
                )
                .unwrap();

                assert_eq!(
                    update,
                    selection_update(&vector.expected_action),
                    "{}",
                    vector.name
                );
                assert_eq!(scene.wave_selection, wave_selection(vector.selection_after));
                assert_eq!(
                    scene.control_latch,
                    control_latch(vector.control_latch_after)
                );
                assert_eq!(scene.callback_countdown, vector.callback_countdown_after);
                assert_eq!(pose.nodes[FIRST_NODE].parent, AlienNodeParent::SceneCamera);
                assert_eq!(
                    pose.nodes[FIRST_NODE].scene_parent,
                    if vector.parent_after == ORIGINAL_WAVE_ANCHOR_OFFSET {
                        Some(AlienSceneNode {
                            model_index: WAVE_ANCHOR_MODEL_INDEX,
                            node_index: WAVE_ANCHOR_NODE_INDEX,
                        })
                    } else {
                        assert_eq!(vector.parent_after, ORIGINAL_PARENT_SENTINEL);
                        None
                    }
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].radial_offset as u16,
                    vector.radial_after
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE]
                        .local_position
                        .map(|value| value as u32),
                    vector.position_after
                );
                assert_eq!(
                    scene.palette_pulses,
                    palette_pulses(&vector.module, &vector.pulse_after)
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    if update == AlienSelectionUpdate::WaveStarted {
                        AlienRingCallback::Wave
                    } else {
                        AlienRingCallback::FollowCourse
                    }
                );
            }
        }
    }

    #[test]
    fn wave_callback_matches_every_original_overlay_vector() {
        let selected_node = AlienSceneNode {
            model_index: SELECTED_MODEL_INDEX,
            node_index: SELECTED_NODE_INDEX,
        };
        for fixture in wave_callback_fixtures() {
            let vectors: Vec<WaveCallbackVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.selected_state, ORIGINAL_SELECTED_NODE);
                let species = species(&vector.module);
                let mut pose = pose(
                    [i32::MIN; AXIS_COUNT],
                    [
                        vector.motion_before[3],
                        vector.motion_before[4],
                        vector.motion_before[5],
                    ],
                );
                pose.nodes[FIRST_NODE].parent = AlienNodeParent::Root;
                pose.nodes[FIRST_NODE].angles = [
                    vector.motion_before[0] as u16,
                    vector.motion_before[1] as u16,
                    vector.motion_before[2] as u16,
                ];
                pose.nodes[FIRST_NODE].local_position = [
                    vector.motion_before[3] as i32,
                    vector.motion_before[4] as i32,
                    vector.motion_before[5] as i32,
                ];
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);
                animation.nodes[FIRST_NODE].callback = AlienRingCallback::Wave;
                let mut scene = AlienCallbackSceneState {
                    callback_countdown: ORIGINAL_CALLBACK_COUNTDOWN,
                    wave_selection: wave_selection(vector.selection_before),
                    palette_pulses: palette_pulses(&vector.module, &vector.pulse_before),
                    method_delta: vector.delta_before as i16,
                    slot2_active: vector.active_before != u16::MIN,
                    wave_selected_node: Some(selected_node),
                    ..AlienCallbackSceneState::default()
                };

                let update = update_wave_callback(
                    species,
                    FIRST_NODE,
                    &mut pose,
                    &mut animation,
                    &mut scene,
                    vector.view,
                )
                .unwrap();

                assert_eq!(
                    update,
                    wave_callback_update(&vector.expected_action),
                    "{}",
                    vector.name
                );
                assert_eq!(scene.slot2_active, vector.active_after != u16::MIN);
                assert_eq!(scene.wave_selection, wave_selection(vector.selection_after));
                assert_eq!(scene.method_delta as u16, vector.delta_after);
                assert_eq!(scene.callback_countdown, vector.countdown_after);
                assert_eq!(
                    scene.palette_pulses,
                    palette_pulses(&vector.module, &vector.pulse_after)
                );
                assert_eq!(
                    [
                        u32::from(pose.nodes[FIRST_NODE].angles[X_AXIS]),
                        u32::from(pose.nodes[FIRST_NODE].angles[Y_AXIS]),
                        u32::from(pose.nodes[FIRST_NODE].angles[Z_AXIS]),
                        pose.nodes[FIRST_NODE].local_position[X_AXIS] as u32,
                        pose.nodes[FIRST_NODE].local_position[Y_AXIS] as u32,
                        pose.nodes[FIRST_NODE].local_position[Z_AXIS] as u32,
                    ],
                    vector.motion_after
                );
                match vector.owner_after {
                    ORIGINAL_WAVE_PARENT_SENTINEL => {
                        assert_eq!(pose.nodes[FIRST_NODE].parent, AlienNodeParent::Root);
                        assert_eq!(pose.nodes[FIRST_NODE].scene_parent, None);
                    }
                    ORIGINAL_SELECTED_NODE => {
                        assert_eq!(pose.nodes[FIRST_NODE].parent, AlienNodeParent::Root);
                        assert_eq!(pose.nodes[FIRST_NODE].scene_parent, Some(selected_node));
                    }
                    ORIGINAL_SCENE_CAMERA_OFFSET => {
                        assert_eq!(pose.nodes[FIRST_NODE].parent, AlienNodeParent::SceneCamera);
                        assert_eq!(pose.nodes[FIRST_NODE].scene_parent, None);
                    }
                    value => panic!("unknown wave parent {value:#06x}"),
                }
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    if vector.callback_after == ORIGINAL_WAVE_CALLBACK_SENTINEL {
                        AlienRingCallback::Wave
                    } else {
                        assert_eq!(vector.callback_after, finish_callback(&vector.module));
                        AlienRingCallback::WaveFinish
                    }
                );
            }
        }
    }

    #[test]
    fn wave_finish_matches_every_original_overlay_vector() {
        for fixture in wave_finish_fixtures() {
            let vectors: Vec<WaveFinishVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut pose = pose(
                    [i32::MIN; AXIS_COUNT],
                    [u32::MIN, vector.position_y_before, u32::MIN],
                );
                pose.nodes[FIRST_NODE].angles = vector.angles_before;

                update_wave_finish(FIRST_NODE, vector.current_sample, &mut pose).unwrap();

                assert_eq!(
                    pose.nodes[FIRST_NODE].local_position[Y_AXIS] as u32, vector.position_y_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].angles, vector.angles_after,
                    "{} {}",
                    vector.module, vector.name
                );
            }
        }
    }

    #[test]
    fn wave_camera_matches_every_original_overlay_vector() {
        for fixture in wave_camera_fixtures() {
            let vectors: Vec<WaveCameraVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.callback_after, motion_callback(&vector.module));
                let mut pose = pose([i32::MIN; AXIS_COUNT], [u32::MIN; AXIS_COUNT]);
                pose.nodes[FIRST_NODE].angles[Y_AXIS] = vector.node_pan;
                pose.nodes[FIRST_NODE].angles[Z_AXIS] = vector.secondary_pan;
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);

                update_wave_camera(FIRST_NODE, vector.camera_pan, &pose, &mut animation).unwrap();

                let state = animation.nodes[FIRST_NODE];
                assert_eq!(
                    state.wave_pan_step as u16, vector.pan_step,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    state.wave_roll_step as u16, vector.secondary_pan_step,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(state.callback, AlienRingCallback::WaveMotion);
            }
        }
    }

    #[test]
    fn wave_motion_matches_every_original_overlay_vector() {
        for fixture in wave_motion_fixtures() {
            let vectors: Vec<WaveMotionVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.callback_before, ORIGINAL_LEAF_CALLBACK_SENTINEL);
                let mut pose = pose([i32::MIN; AXIS_COUNT], [u32::MIN; AXIS_COUNT]);
                pose.nodes[FIRST_NODE].angles[Y_AXIS] = vector.pan_before;
                pose.nodes[FIRST_NODE].angles[Z_AXIS] = vector.roll_before;
                pose.nodes[FIRST_NODE].radial_offset = vector.phase_before as i16;
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);
                animation.nodes[FIRST_NODE].callback = AlienRingCallback::WaveMotion;
                animation.nodes[FIRST_NODE].wave_pan_step = vector.pan_step as i16;
                animation.nodes[FIRST_NODE].wave_roll_step = vector.roll_step as i16;

                let update = update_wave_motion(FIRST_NODE, &mut pose, &mut animation).unwrap();

                assert_eq!(
                    pose.nodes[FIRST_NODE].angles[Y_AXIS], vector.pan_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].angles[Z_AXIS], vector.roll_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].radial_offset as u16, vector.phase_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    update,
                    if vector.transitioned {
                        AlienWaveMotionUpdate::ReturnDelayStarted
                    } else {
                        AlienWaveMotionUpdate::Moving
                    }
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    if vector.callback_after == ORIGINAL_LEAF_CALLBACK_SENTINEL {
                        AlienRingCallback::WaveMotion
                    } else {
                        assert_eq!(vector.callback_after, return_callback(&vector.module));
                        AlienRingCallback::WaveReturn
                    }
                );
            }
        }
    }

    #[test]
    fn wave_return_matches_every_original_overlay_vector() {
        for fixture in wave_return_fixtures() {
            let vectors: Vec<WaveReturnVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.callback_before, ORIGINAL_LEAF_CALLBACK_SENTINEL);
                let mut pose = pose([i32::MIN; AXIS_COUNT], [u32::MIN; AXIS_COUNT]);
                pose.nodes[FIRST_NODE].radial_offset = vector.countdown_before as i16;
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);
                animation.nodes[FIRST_NODE].callback = AlienRingCallback::WaveReturn;

                let update = update_wave_return(FIRST_NODE, &mut pose, &mut animation).unwrap();

                assert_eq!(
                    pose.nodes[FIRST_NODE].radial_offset as u16, vector.countdown_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    update,
                    if vector.transitioned {
                        AlienWaveReturnUpdate::SelectionRequested
                    } else {
                        AlienWaveReturnUpdate::Waiting
                    }
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    if vector.callback_after == ORIGINAL_LEAF_CALLBACK_SENTINEL {
                        AlienRingCallback::WaveReturn
                    } else {
                        assert_eq!(vector.callback_after, selection_callback(&vector.module));
                        AlienRingCallback::WaveSelection
                    }
                );
            }
        }
    }

    #[test]
    fn wave_steering_matches_every_original_overlay_vector() {
        for fixture in wave_steering_fixtures() {
            let vectors: Vec<WaveSteeringVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut pose = pose([i32::MIN; AXIS_COUNT], [u32::MIN; AXIS_COUNT]);
                pose.nodes[FIRST_NODE].local_position = vector.position.map(i32::from);
                pose.nodes[FIRST_NODE].angles[Y_AXIS] = vector.pan_before;
                pose.nodes[FIRST_NODE].angles[Z_AXIS] = ORIGINAL_STEERING_ROLL_SENTINEL;
                pose.nodes[FIRST_NODE].radial_offset = ORIGINAL_STEERING_RADIAL_SENTINEL as i16;
                let mut animation = AlienRingAnimationState::new(SINGLE_NODE_COUNT);
                animation.nodes[FIRST_NODE].callback = AlienRingCallback::WaveSelection;
                animation.nodes[FIRST_NODE].steering = super::super::AlienWaveSteeringState {
                    turn_countdown: vector.countdown_before as i16,
                    turn_step: vector.turn_step_before as i16,
                    turn_offset: vector.turn_offset_before as i16,
                    sample_phase: vector.sample_phase_before,
                    random_seed: vector.random_seed_before,
                };
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from(vector.sample_phase_after >> STEERING_SAMPLE_INDEX_SHIFT);
                trigonometry[sample_index].cosine = vector.sample as i16;

                continue_wave_steering(
                    FIRST_NODE,
                    &mut pose,
                    &mut animation,
                    vector.view,
                    &trigonometry,
                )
                .unwrap();

                assert_eq!(
                    pose.nodes[FIRST_NODE].angles[Y_AXIS], vector.pan_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].angles[Z_AXIS], vector.roll_after,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    pose.nodes[FIRST_NODE].radial_offset, STEERING_RADIAL_OFFSET,
                    "{} {}",
                    vector.module, vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].steering,
                    super::super::AlienWaveSteeringState {
                        turn_countdown: vector.countdown_after as i16,
                        turn_step: vector.turn_step_after as i16,
                        turn_offset: vector.turn_offset_after as i16,
                        sample_phase: vector.sample_phase_after,
                        random_seed: vector.random_seed_after,
                    },
                    "{} {}",
                    vector.module,
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    AlienRingCallback::WaveSelection
                );
            }
        }
    }
}
