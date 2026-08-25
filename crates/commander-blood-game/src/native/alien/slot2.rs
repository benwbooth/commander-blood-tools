//! Initialization and typed callback dispatch for alien slot-2 animation.

use std::fmt;

use commander_blood_formats::alien::AXIS_COUNT;

use super::{
    AlienCallbackSceneState, AlienCameraTransform, AlienModelPose, AlienSpecies, AlienWaveSelection,
};

const PRIMARY_NODE: usize = 0;
const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const INITIAL_DURATION: i16 = 50;
const INITIAL_AMER_RADIAL_TARGET: u16 = 20;
const INITIAL_AMER_SELECTION_RADIAL_TARGET: u16 = 40;
const ANGLE_MASK: u16 = 0x0ffc;
const CROOLIS_SEED_STEP: u16 = 250;
const SCRUT_SEED_STEP: u16 = 300;
const RANDOM_ROTATION: u32 = 7;
const RANDOM_BORROW_SHIFT: u32 = 6;
const RANDOM_BORROW_MASK: u16 = 1;
const RESET_SIGNED_VALUE: i16 = 0;
const RESET_ANGLE: u16 = 0;
const PRIMARY_AND_FOLLOWER_NODE_COUNT: usize = 2;
const AMER_RETURN_PAN_STEP: u16 = 128;
const AMER_RETURN_ROLL_STEP: i16 = 117;
const AMER_RESTART_COUNTDOWN: i16 = 32;
const AMER_SIGNED_ANGLE_SHIFT: u32 = 4;
const AMER_VELOCITY_SHIFT: u32 = 5;
const AMER_STEERING_DISTANCE: i32 = 1_000;
const AMER_STEERING_TURN_STEP: u16 = 32;
const AMER_FINISH_COUNTDOWN: i16 = 64;
const AMER_RESET_RADIAL_OFFSET: i16 = 60;
const AMER_CAMERA_HEIGHT_MINIMUM: i16 = -768;
const AMER_CAMERA_HEIGHT_MAXIMUM: i16 = 768;
const AMER_CAMERA_HEIGHT_EASING_SHIFT: u32 = 1;
const AMER_FINISH_RADIAL_STEP: i16 = 10;
const AMER_FINISH_RADIAL_TARGET: u16 = 500;
const AMER_COMMON_NODE_COUNT: usize = 5;
const AMER_RADIAL_EASING_SHIFT: u32 = 3;
const AMER_CENTER_X_MINIMUM: i16 = -40;
const AMER_CENTER_X_MAXIMUM_EXCLUSIVE: i16 = 40;
const AMER_CENTER_Y_MINIMUM: i16 = -40;
const AMER_CENTER_Y_MAXIMUM: i16 = 40;
const AMER_CENTER_Z_MINIMUM_EXCLUSIVE: i16 = -80;
const AMER_CENTER_Z_MAXIMUM: i16 = 40;
const AMER_CAMERA_HALF_TURN: u16 = 0x0800;
const AMER_ANIMATION_PHASE_STEP: u16 = 132;
const AMER_ANIMATION_PHASE_MASK: u16 = 0x03ff;
const AMER_ROLL_TO_PAN_SHIFT: u32 = 3;
const AMER_ROLL_REGION_BIAS: i16 = 32;
const AMER_ROLL_REGION_WIDTH: i16 = 64;
const AMER_FOLLOWER_PITCH: i16 = 256;
const AMER_FOLLOWER_PITCH_DOUBLE: i16 = 512;
const AMER_CAMERA_PLACEMENT_DISTANCE: i32 = -160;
const AMER_MINIMUM_RETURN_DEPTH: i16 = 40;
const AMER_RETURN_DEPTH_SHIFT: u32 = 1;
const AMER_RETURN_TIMER_SHIFT: u32 = 2;
const AMER_RETURN_TIMER_BIAS: u16 = 20;
const AMER_RETURN_VELOCITY_SHIFT: u32 = 18;
const AMER_CAMERA_DEPTH_RESET: i16 = -64;
const AMER_RETURN_CALLBACK_COUNTDOWN: u16 = 1;
const UNREFERENCED_STEERING_RADIAL_OFFSET: i16 = 10;
const UNREFERENCED_STEERING_TURN_STEP: i16 = 16;
const AMER_MOTION_RANDOM_ROTATION: u32 = 3;
const AMER_MOTION_RANDOM_BORROW_SHIFT: u32 = 2;
const AMER_MOTION_RANDOM_BORROW_MASK: u16 = 1;
const AMER_MOTION_TARGET_MASK: u16 = 0x07ff;
const AMER_MOTION_TARGET_CENTER: u16 = 1_023;
const AMER_MOTION_DURATION_SHIFT: u32 = 2;
const AMER_MOTION_DURATION_BIAS: u16 = 16;
const AMER_MOTION_CAMERA_X_MINIMUM: i16 = -1_500;
const AMER_MOTION_CAMERA_X_MAXIMUM: i16 = 1_500;
const AMER_MOTION_CAMERA_Z_MINIMUM: i16 = -1_000;
const AMER_MOTION_CAMERA_Z_MAXIMUM: i16 = 1_500;
const AMER_SELECTION_DEPTH_MAXIMUM: u16 = 3_000;
const AMER_SELECTION_CAMERA_X_MINIMUM: i16 = -1_000;
const AMER_SELECTION_CAMERA_X_MAXIMUM: i16 = 1_000;
const AMER_SELECTION_LATE_DEPTH: i16 = 800;
const AMER_SELECTION_LATE_RADIAL_TARGET: u16 = 80;
const AMER_SELECTION_TURN_STEP: u16 = 64;
const AMER_LATE_SELECTION_DEPTH_MAXIMUM: u16 = 1_000;
const AMER_LATE_SELECTION_CAMERA_X_MINIMUM: i16 = -500;
const AMER_LATE_SELECTION_CAMERA_X_MAXIMUM: i16 = 500;
const AMER_LATE_SELECTION_DEPTH_ORIGIN: i16 = 200;
const AMER_LATE_SELECTION_ROLL_VELOCITY: i16 = 48;

/// Callback stage selected for one slot-2 animation model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSlot2Callback {
    /// Advance the species-specific primary animation callback.
    Update,
    /// Move AMER back toward its ordinary autonomous state.
    AmerReturn,
    /// Apply AMER's camera-relative autonomous steering.
    AmerSteer,
    /// Complete AMER's camera-relative steering phase.
    AmerFinish,
    /// Wait for AMER's active selection-tracking callback.
    AmerSelectionWait,
    /// Track the active camera-relative selection target for AMER.
    AmerSelection,
    /// Track AMER after it crosses into the close selection phase.
    AmerSelectionLate,
}

/// Callback-owned state parallel to one animated model node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienSlot2NodeState {
    /// Species-specific motion parameter: AMER timer or follower velocity.
    pub motion_parameter: i16,
    /// Desired radial displacement approached by the callback family.
    pub radial_target: u16,
    /// SCRUT depth target retained independently from node ownership.
    pub depth_target: i16,
    /// Deterministic behavior seed used by AMER's reset path.
    pub behavior_seed: u16,
}

/// Persistent state for one model using the slot-2/4 behavior method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienSlot2AnimationState {
    /// Whether one-time model initialization has completed.
    pub initialized: bool,
    /// Callback selected for the next model update.
    pub callback: Option<AlienSlot2Callback>,
    /// Timer controlling the current species-specific callback phase.
    pub phase_timer: i16,
    /// CROOLIS-only signed motion accumulator.
    pub croolis_motion_accumulator: i16,
    /// Sign-extended species seed captured during initialization.
    pub species_seed_at_initialization: i32,
    /// Deterministic random value owned by this model.
    pub random_value: u16,
    /// Wrapped phase driving AMER's four follower-node poses.
    pub amer_animation_phase: u16,
    /// AMER-only signed per-axis velocity used during its return flight.
    pub amer_velocity: [i16; AXIS_COUNT],
    /// Callback state parallel to the model pose nodes.
    pub nodes: Vec<AlienSlot2NodeState>,
}

impl AlienSlot2AnimationState {
    /// Allocate flat callback state for a typed model hierarchy.
    pub fn new(node_count: usize) -> Self {
        Self {
            initialized: false,
            callback: None,
            phase_timer: i16::default(),
            croolis_motion_accumulator: i16::default(),
            species_seed_at_initialization: i32::default(),
            random_value: u16::default(),
            amer_animation_phase: u16::default(),
            amer_velocity: [i16::default(); AXIS_COUNT],
            nodes: vec![AlienSlot2NodeState::default(); node_count],
        }
    }
}

/// Scene-owned random state shared while slot-2 models initialize in order.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienSlot2SceneState {
    /// Deterministic random stream shared by all animation models.
    pub random_state: u16,
    /// CROOLIS/SCRUT initialization seed, initially zero in both overlays.
    pub species_seed: u16,
}

/// Isolated state for the steering sibling compiled but unreachable in all overlays.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienUnreferencedSteeringState {
    /// Turn selected by the preceding invocation for sign-change damping.
    pub previous_turn: i16,
}

/// Concrete callback boundary for the slot-2 coordinator.
pub trait AlienSlot2Callbacks {
    /// Invoke the currently selected typed callback.
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienSlot2Callback,
        pose: &mut AlienModelPose,
        animation: &mut AlienSlot2AnimationState,
        scene: &mut AlienSlot2SceneState,
    ) -> Result<(), AlienSlot2Error>;
}

/// Stage completed by one slot-2 coordinator invocation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSlot2Update {
    /// One-time model and per-node callback state was initialized.
    Initialized,
    /// The previously selected callback was invoked.
    CallbackInvoked,
}

/// Stage completed by one AMER return-flight callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerReturnUpdate {
    /// Position and orientation advanced while the return timer remains active.
    Returning,
    /// The return completed and the ordinary update callback was restored.
    Restarted,
}

/// Stage completed by one AMER autonomous-steering callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerSteeringUpdate {
    /// Steering advanced while the phase timer remains active.
    Steering,
    /// The phase timer expired and the finish callback was selected.
    FinishStarted,
}

/// Continuation selected by one AMER finish-callback pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerFinishUpdate {
    /// Continue immediately through the separately recovered reset routine.
    ResetRequested,
    /// Wait for the next frame before beginning selection tracking.
    SelectionWaitStarted,
    /// Continue camera-relative steering in the current finish phase.
    Steering,
}

/// Stage completed by AMER's common animation tail.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerCommonUpdate {
    /// Ordinary autonomous animation and follower poses were advanced.
    MotionUpdated,
    /// The model faced back toward the current camera pan.
    CameraFacing,
    /// The model was placed ahead of the camera and began its return flight.
    ReturnStarted,
}

/// Typed continuation chosen by AMER's ordinary slot-2 callback head.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerUpdateHead {
    /// Continue immediately through the selection-entry callback.
    SelectionRequested,
    /// Continue immediately through AMER's motion reset.
    ResetRequested,
    /// Continue immediately through AMER's shared animation tail.
    CommonRequested,
}

/// Typed continuation chosen by AMER's primary selection callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerSelectionUpdate {
    /// Continue immediately through the ordinary-update restart.
    RestartRequested,
    /// Continue immediately through AMER's motion reset.
    ResetRequested,
    /// The close selection callback was installed for a later frame.
    LateSelectionStarted,
    /// Continue immediately through AMER's shared animation tail.
    CommonRequested,
}

/// Typed continuation chosen by AMER's close selection callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienAmerLateSelectionUpdate {
    /// Continue immediately through the selection-entry callback.
    SelectionWaitRequested,
    /// Continue immediately through AMER's motion reset.
    ResetRequested,
    /// Continue immediately through AMER's shared animation tail.
    CommonRequested,
}

/// Invalid flat state supplied to the slot-2 coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSlot2Error {
    /// Every shipped animation model contains a primary node.
    EmptyNodeList,
    /// Pose nodes and parallel callback states must have identical lengths.
    NodeStateCountMismatch {
        /// Nodes available in the mutable pose.
        pose: usize,
        /// Parallel callback states available to the animation.
        animation: usize,
    },
    /// CROOLIS and SCRUT initialization requires at least one follower node.
    MissingFollowerNode {
        /// Nodes supplied by the caller.
        node_count: usize,
    },
    /// AMER's common animation tail requires its four follower nodes.
    MissingAmerAnimationNodes {
        /// Nodes supplied by the caller.
        node_count: usize,
    },
    /// Initialized state must retain a typed callback stage.
    MissingCallback,
}

impl fmt::Display for AlienSlot2Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien slot-2 state: {self:?}")
    }
}

impl std::error::Error for AlienSlot2Error {}

/// Initialize or dispatch one recovered slot-2/4 animation method.
pub fn initialize_or_dispatch_slot2(
    species: AlienSpecies,
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    scene: &mut AlienSlot2SceneState,
    callbacks: &mut impl AlienSlot2Callbacks,
) -> Result<AlienSlot2Update, AlienSlot2Error> {
    validate_state(species, pose, animation)?;
    if animation.initialized {
        let callback = animation.callback.ok_or(AlienSlot2Error::MissingCallback)?;
        callbacks.invoke(species, callback, pose, animation, scene)?;
        return Ok(AlienSlot2Update::CallbackInvoked);
    }

    let first_random = transform_random(scene.random_state);
    scene.random_state = first_random;
    animation.initialized = true;
    animation.callback = Some(AlienSlot2Callback::Update);
    if species == AlienSpecies::Amer {
        animation.phase_timer = RESET_SIGNED_VALUE;
        animation.random_value = first_random;
        pose.nodes[PRIMARY_NODE].angles[Y_AXIS] = first_random & ANGLE_MASK;
        animation.nodes[PRIMARY_NODE].radial_target = INITIAL_AMER_RADIAL_TARGET;
        return Ok(AlienSlot2Update::Initialized);
    }

    animation.phase_timer = INITIAL_DURATION;
    if species == AlienSpecies::Croolis {
        animation.croolis_motion_accumulator = RESET_SIGNED_VALUE;
    }
    animation.species_seed_at_initialization = i32::from(scene.species_seed as i16);
    scene.species_seed = scene.species_seed.wrapping_add(seed_step(species));
    animation.random_value = transform_random(first_random);
    let primary = &mut pose.nodes[PRIMARY_NODE];
    primary.angles[Y_AXIS] = animation.random_value & ANGLE_MASK;
    primary.angles[Z_AXIS] = RESET_ANGLE;
    primary.radial_offset = RESET_SIGNED_VALUE;
    animation.nodes[PRIMARY_NODE].motion_parameter = RESET_SIGNED_VALUE;
    animation.nodes[PRIMARY_NODE].radial_target = RESET_ANGLE;
    if species == AlienSpecies::Scrut {
        animation.nodes[PRIMARY_NODE].depth_target = RESET_SIGNED_VALUE;
    }

    for (node, callback_state) in pose.nodes[1..].iter().zip(&mut animation.nodes[1..]) {
        callback_state.motion_parameter = match species {
            AlienSpecies::Amer => unreachable!("AMER initialization returns before followers"),
            AlienSpecies::Croolis => node.local_position[Z_AXIS] as i16,
            AlienSpecies::Scrut => node.local_position[X_AXIS] as i16,
        };
        if species == AlienSpecies::Scrut {
            callback_state.depth_target = node.local_position[Z_AXIS] as i16;
        }
    }
    Ok(AlienSlot2Update::Initialized)
}

/// Advance AMER's camera-relative return flight using owned flat model state.
pub fn update_amer_return(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    slot2_active: &mut bool,
) -> Result<AlienAmerReturnUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    animation.phase_timer = animation.phase_timer.wrapping_sub(1);
    let primary = &mut pose.nodes[PRIMARY_NODE];
    primary.radial_offset = RESET_SIGNED_VALUE;
    if animation.phase_timer >= RESET_SIGNED_VALUE {
        primary.angles[Y_AXIS] = primary.angles[Y_AXIS].wrapping_add(AMER_RETURN_PAN_STEP);
        primary.angles[Z_AXIS] = primary.angles[Z_AXIS].wrapping_sub(AMER_RETURN_ROLL_STEP as u16);
        for (position, velocity) in primary
            .local_position
            .iter_mut()
            .zip(animation.amer_velocity)
        {
            *position = position.wrapping_add(i32::from(velocity));
        }
        return Ok(AlienAmerReturnUpdate::Returning);
    }

    animation.phase_timer = AMER_RESTART_COUNTDOWN;
    primary.angles[Y_AXIS] = normalize_signed_angle(primary.angles[Y_AXIS]) as u16;
    primary.angles[Z_AXIS] = normalize_signed_angle(primary.angles[Z_AXIS]) as u16;
    animation.amer_velocity[X_AXIS] =
        primary.angles[Z_AXIS].wrapping_neg().cast_signed() >> AMER_VELOCITY_SHIFT;
    animation.callback = Some(AlienSlot2Callback::Update);
    *slot2_active = false;
    Ok(AlienAmerReturnUpdate::Restarted)
}

/// Advance AMER's autonomous steering using the projected node transform.
pub fn update_amer_steering(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    camera_depth_step: u16,
) -> Result<AlienAmerSteeringUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    let primary = &mut pose.nodes[PRIMARY_NODE];
    let camera_x = transformed_component(primary, X_AXIS);
    let camera_z = transformed_component(primary, Z_AXIS);
    let forward_x = primary.transform.matrix[X_AXIS][Z_AXIS];
    let forward_z = primary.transform.matrix[Z_AXIS][Z_AXIS];
    let forward_distance = i32::from(camera_z)
        .wrapping_sub(i32::from(camera_depth_step))
        .wrapping_sub(AMER_STEERING_DISTANCE)
        .wrapping_neg();
    let score = forward_distance
        .wrapping_mul(forward_x)
        .wrapping_add(i32::from(camera_x).wrapping_mul(forward_z));
    primary.angles[Y_AXIS] = if score < i32::default() {
        primary.angles[Y_AXIS].wrapping_add(AMER_STEERING_TURN_STEP)
    } else {
        primary.angles[Y_AXIS].wrapping_sub(AMER_STEERING_TURN_STEP)
    };

    let node_state = &mut animation.nodes[PRIMARY_NODE];
    node_state.motion_parameter = node_state.motion_parameter.wrapping_sub(1);
    if node_state.motion_parameter >= RESET_SIGNED_VALUE {
        return Ok(AlienAmerSteeringUpdate::Steering);
    }

    node_state.motion_parameter = AMER_FINISH_COUNTDOWN;
    animation.callback = Some(AlienSlot2Callback::AmerFinish);
    Ok(AlienAmerSteeringUpdate::FinishStarted)
}

/// Reset AMER's autonomous motion after a bounds or phase transition.
pub fn reset_amer_motion(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
) -> Result<(), AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    let random_value = transform_random(animation.random_value);
    animation.random_value = random_value;
    animation.amer_velocity[X_AXIS] = RESET_SIGNED_VALUE;
    animation.callback = Some(AlienSlot2Callback::AmerSteer);

    let primary = &mut pose.nodes[PRIMARY_NODE];
    primary.angles[X_AXIS] = (random_value.cast_signed() >> RANDOM_BORROW_SHIFT) as u16;
    primary.angles[Z_AXIS] = RESET_ANGLE;
    primary.radial_offset = AMER_RESET_RADIAL_OFFSET;
    let node_state = &mut animation.nodes[PRIMARY_NODE];
    node_state.motion_parameter = AMER_RESTART_COUNTDOWN;
    node_state.behavior_seed = u16::default();
    Ok(())
}

/// Restart AMER's ordinary update and request same-pass callback dispatch.
pub fn restart_amer_update(
    pose: &AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
) -> Result<AlienSlot2Callback, AlienSlot2Error> {
    prepare_amer_immediate_callback(
        pose,
        animation,
        INITIAL_AMER_RADIAL_TARGET,
        AlienSlot2Callback::Update,
    )
}

/// Begin AMER selection tracking and request same-pass callback dispatch.
pub fn begin_amer_selection(
    pose: &AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
) -> Result<AlienSlot2Callback, AlienSlot2Error> {
    prepare_amer_immediate_callback(
        pose,
        animation,
        INITIAL_AMER_SELECTION_RADIAL_TARGET,
        AlienSlot2Callback::AmerSelection,
    )
}

/// Advance AMER's finish phase up to its typed reset or selection transition.
pub fn update_amer_finish(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    camera_depth_step: u16,
) -> Result<AlienAmerFinishUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    let primary = &mut pose.nodes[PRIMARY_NODE];
    let node_state = &mut animation.nodes[PRIMARY_NODE];
    node_state.motion_parameter = node_state.motion_parameter.wrapping_sub(1);
    if node_state.motion_parameter < RESET_SIGNED_VALUE {
        return Ok(AlienAmerFinishUpdate::ResetRequested);
    }

    let camera_y = transformed_component(primary, Y_AXIS);
    primary.angles[X_AXIS] = camera_y
        .wrapping_add(primary.angles[X_AXIS] as i16)
        .wrapping_shr(AMER_CAMERA_HEIGHT_EASING_SHIFT)
        .clamp(AMER_CAMERA_HEIGHT_MINIMUM, AMER_CAMERA_HEIGHT_MAXIMUM)
        as u16;
    let camera_x = transformed_component(primary, X_AXIS);
    let camera_z = transformed_component(primary, Z_AXIS);
    let horizontal = i32::from(camera_z)
        .wrapping_sub(i32::from(camera_depth_step))
        .wrapping_sub(AMER_STEERING_DISTANCE);
    let vertical = i32::from(camera_x);
    if (i32::default()..=AMER_STEERING_DISTANCE).contains(&horizontal)
        && (-AMER_STEERING_DISTANCE..=AMER_STEERING_DISTANCE).contains(&vertical)
    {
        animation.callback = Some(AlienSlot2Callback::AmerSelectionWait);
        primary.radial_offset >>= 1;
        return Ok(AlienAmerFinishUpdate::SelectionWaitStarted);
    }

    primary.radial_offset = primary.radial_offset.wrapping_add(AMER_FINISH_RADIAL_STEP);
    node_state.radial_target = AMER_FINISH_RADIAL_TARGET;
    let score = horizontal
        .wrapping_neg()
        .wrapping_mul(primary.transform.matrix[X_AXIS][Z_AXIS])
        .wrapping_add(vertical.wrapping_mul(primary.transform.matrix[Z_AXIS][Z_AXIS]));
    primary.angles[Y_AXIS] = if score < i32::default() {
        primary.angles[Y_AXIS].wrapping_add(AMER_STEERING_TURN_STEP)
    } else {
        primary.angles[Y_AXIS].wrapping_sub(AMER_STEERING_TURN_STEP)
    };
    Ok(AlienAmerFinishUpdate::Steering)
}

/// Select and prepare AMER's next ordinary-update continuation.
pub fn update_amer_head(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    scene: &AlienCallbackSceneState,
) -> Result<AlienAmerUpdateHead, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    if scene.method_delta >= RESET_SIGNED_VALUE
        && scene.wave_selection == AlienWaveSelection::Requested
    {
        return Ok(AlienAmerUpdateHead::SelectionRequested);
    }

    animation.phase_timer = animation.phase_timer.wrapping_sub(1);
    if animation.phase_timer >= RESET_SIGNED_VALUE {
        return Ok(AlienAmerUpdateHead::CommonRequested);
    }

    let primary = &mut pose.nodes[PRIMARY_NODE];
    let camera_x = transformed_component(primary, X_AXIS);
    let camera_z = transformed_component(primary, Z_AXIS);
    let in_motion_bounds = (AMER_MOTION_CAMERA_X_MINIMUM..=AMER_MOTION_CAMERA_X_MAXIMUM)
        .contains(&camera_x)
        && (AMER_MOTION_CAMERA_Z_MINIMUM..=AMER_MOTION_CAMERA_Z_MAXIMUM).contains(&camera_z);
    if !in_motion_bounds {
        return Ok(AlienAmerUpdateHead::ResetRequested);
    }

    let random_value = transform_amer_motion_random(animation.random_value);
    let target_roll =
        ((random_value & AMER_MOTION_TARGET_MASK).wrapping_sub(AMER_MOTION_TARGET_CENTER)) as i16;
    let duration = ((target_roll.unsigned_abs() >> AMER_MOTION_DURATION_SHIFT)
        .wrapping_add(AMER_MOTION_DURATION_BIAS)) as i16;
    animation.phase_timer = duration;
    animation.random_value = random_value;
    animation.amer_velocity[X_AXIS] =
        target_roll.wrapping_sub(primary.angles[Z_AXIS] as i16) / duration;
    animation.nodes[PRIMARY_NODE].radial_target = INITIAL_AMER_RADIAL_TARGET;
    let camera_y = transformed_component(primary, Y_AXIS);
    primary.angles[X_AXIS] = camera_y
        .wrapping_add(primary.angles[X_AXIS] as i16)
        .wrapping_shr(AMER_CAMERA_HEIGHT_EASING_SHIFT)
        .clamp(AMER_CAMERA_HEIGHT_MINIMUM, AMER_CAMERA_HEIGHT_MAXIMUM)
        as u16;
    Ok(AlienAmerUpdateHead::CommonRequested)
}

/// Select and prepare AMER's primary selection continuation.
pub fn update_amer_selection(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    scene: &AlienCallbackSceneState,
    camera_view_y: i16,
) -> Result<AlienAmerSelectionUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    if scene.wave_selection != AlienWaveSelection::Requested {
        return Ok(AlienAmerSelectionUpdate::RestartRequested);
    }

    let primary = &mut pose.nodes[PRIMARY_NODE];
    let camera_depth = transformed_component(primary, Z_AXIS) as u16;
    let camera_x = transformed_component(primary, X_AXIS);
    if camera_depth > AMER_SELECTION_DEPTH_MAXIMUM
        || !(AMER_SELECTION_CAMERA_X_MINIMUM..=AMER_SELECTION_CAMERA_X_MAXIMUM).contains(&camera_x)
    {
        return Ok(AlienAmerSelectionUpdate::ResetRequested);
    }
    if (camera_depth as i16) < AMER_SELECTION_LATE_DEPTH {
        animation.nodes[PRIMARY_NODE].radial_target = AMER_SELECTION_LATE_RADIAL_TARGET;
        animation.callback = Some(AlienSlot2Callback::AmerSelectionLate);
        return Ok(AlienAmerSelectionUpdate::LateSelectionStarted);
    }

    let camera_z = camera_depth as i16;
    let score = i32::from(camera_z)
        .wrapping_neg()
        .wrapping_mul(primary.transform.matrix[X_AXIS][Z_AXIS])
        .wrapping_add(i32::from(camera_x).wrapping_mul(primary.transform.matrix[Z_AXIS][Z_AXIS]));
    animation.amer_velocity[X_AXIS] = RESET_SIGNED_VALUE;
    primary.angles[Y_AXIS] = if score < i32::default() {
        primary.angles[Y_AXIS].wrapping_add(AMER_SELECTION_TURN_STEP)
    } else {
        primary.angles[Y_AXIS].wrapping_sub(AMER_SELECTION_TURN_STEP)
    };
    primary.angles[Z_AXIS] = RESET_ANGLE;
    update_amer_selection_pitch(primary, camera_view_y);
    Ok(AlienAmerSelectionUpdate::CommonRequested)
}

/// Select and prepare AMER's close-selection continuation.
pub fn update_amer_late_selection(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    camera_view_y: i16,
) -> Result<AlienAmerLateSelectionUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    let primary = &mut pose.nodes[PRIMARY_NODE];
    let camera_depth = transformed_component(primary, Z_AXIS) as u16;
    if camera_depth > AMER_LATE_SELECTION_DEPTH_MAXIMUM {
        return Ok(AlienAmerLateSelectionUpdate::SelectionWaitRequested);
    }
    let camera_x = transformed_component(primary, X_AXIS);
    if !(AMER_LATE_SELECTION_CAMERA_X_MINIMUM..=AMER_LATE_SELECTION_CAMERA_X_MAXIMUM)
        .contains(&camera_x)
    {
        return Ok(AlienAmerLateSelectionUpdate::ResetRequested);
    }

    let depth_from_origin = (camera_depth as i16).wrapping_sub(AMER_LATE_SELECTION_DEPTH_ORIGIN);
    let score = i32::from(depth_from_origin)
        .wrapping_neg()
        .wrapping_mul(primary.transform.matrix[X_AXIS][Z_AXIS])
        .wrapping_add(i32::from(camera_x).wrapping_mul(primary.transform.matrix[Z_AXIS][Z_AXIS]));
    animation.amer_velocity[X_AXIS] = if score < i32::default() {
        AMER_LATE_SELECTION_ROLL_VELOCITY
    } else {
        -AMER_LATE_SELECTION_ROLL_VELOCITY
    };
    update_amer_selection_pitch(primary, camera_view_y);
    Ok(AlienAmerLateSelectionUpdate::CommonRequested)
}

/// Run AMER's shared motion tail using flat model and camera state.
pub fn update_amer_common(
    pose: &mut AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    scene: &mut AlienCallbackSceneState,
    camera: &AlienCameraTransform,
    camera_pan: u16,
    camera_depth_step: &mut i16,
) -> Result<AlienAmerCommonUpdate, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    if pose.nodes.len() < AMER_COMMON_NODE_COUNT {
        return Err(AlienSlot2Error::MissingAmerAnimationNodes {
            node_count: pose.nodes.len(),
        });
    }

    let radial_target = animation.nodes[PRIMARY_NODE].radial_target as i16;
    let primary = &mut pose.nodes[PRIMARY_NODE];
    let radial_delta = radial_target.wrapping_sub(primary.radial_offset);
    primary.radial_offset = primary
        .radial_offset
        .wrapping_add(radial_delta >> AMER_RADIAL_EASING_SHIFT);
    let camera_x = transformed_component(primary, X_AXIS);
    let camera_y = transformed_component(primary, Y_AXIS);
    let camera_z = transformed_component(primary, Z_AXIS);
    let centered = (AMER_CENTER_X_MINIMUM..AMER_CENTER_X_MAXIMUM_EXCLUSIVE).contains(&camera_x)
        && (AMER_CENTER_Y_MINIMUM..=AMER_CENTER_Y_MAXIMUM).contains(&camera_y)
        && camera_z > AMER_CENTER_Z_MINIMUM_EXCLUSIVE
        && camera_z <= AMER_CENTER_Z_MAXIMUM;
    if centered && camera_z < RESET_SIGNED_VALUE {
        primary.angles[Y_AXIS] = camera_pan.wrapping_add(AMER_CAMERA_HALF_TURN) & ANGLE_MASK;
        return Ok(AlienAmerCommonUpdate::CameraFacing);
    }
    if centered {
        primary.radial_offset = RESET_SIGNED_VALUE;
        for axis in usize::default()..AXIS_COUNT {
            let camera_offset = camera.matrix[Z_AXIS][axis]
                .wrapping_mul(AMER_CAMERA_PLACEMENT_DISTANCE)
                .wrapping_add(camera.position[axis]);
            primary.local_position[axis] = (camera_offset >> u16::BITS).wrapping_neg();
        }
        let depth = (*camera_depth_step >> AMER_RETURN_DEPTH_SHIFT).max(AMER_MINIMUM_RETURN_DEPTH);
        animation.phase_timer = (((depth as u16) >> AMER_RETURN_TIMER_SHIFT)
            .wrapping_add(AMER_RETURN_TIMER_BIAS)) as i16;
        for axis in usize::default()..AXIS_COUNT {
            animation.amer_velocity[axis] = (camera.matrix[Z_AXIS][axis]
                .wrapping_mul(i32::from(depth))
                >> AMER_RETURN_VELOCITY_SHIFT) as i16;
        }
        *camera_depth_step = AMER_CAMERA_DEPTH_RESET;
        animation.callback = Some(AlienSlot2Callback::AmerReturn);
        scene.slot2_active = true;
        scene.callback_countdown = AMER_RETURN_CALLBACK_COUNTDOWN;
        return Ok(AlienAmerCommonUpdate::ReturnStarted);
    }

    primary.angles[Z_AXIS] =
        primary.angles[Z_AXIS].wrapping_add(animation.amer_velocity[X_AXIS] as u16);
    primary.angles[Y_AXIS] = primary.angles[Y_AXIS]
        .wrapping_add((primary.angles[Z_AXIS] as i16 >> AMER_ROLL_TO_PAN_SHIFT) as u16);
    animation.amer_animation_phase = animation
        .amer_animation_phase
        .wrapping_add(AMER_ANIMATION_PHASE_STEP)
        & AMER_ANIMATION_PHASE_MASK;
    let roll = primary.angles[Z_AXIS] as i16;
    update_amer_followers(
        &mut pose.nodes[1..AMER_COMMON_NODE_COUNT],
        roll,
        animation.amer_animation_phase,
    );
    Ok(AlienAmerCommonUpdate::MotionUpdated)
}

/// Preserve the observable behavior of the unreachable steering sibling.
///
/// No original alien method table or callback points at this routine. Keeping
/// it translated documents the complete overlays without retaining its DOS
/// context-pointer traversal in the modern runtime.
pub fn update_unreferenced_steering(
    pose: &mut AlienModelPose,
    steering: &mut AlienUnreferencedSteeringState,
) -> Result<i16, AlienSlot2Error> {
    let primary = pose
        .nodes
        .get_mut(PRIMARY_NODE)
        .ok_or(AlienSlot2Error::EmptyNodeList)?;
    let camera_x = transformed_component(primary, X_AXIS);
    let camera_z = transformed_component(primary, Z_AXIS);
    let score = i32::from(camera_x)
        .wrapping_mul(primary.transform.matrix[Z_AXIS][Z_AXIS])
        .wrapping_sub(i32::from(camera_z).wrapping_mul(primary.transform.matrix[X_AXIS][Z_AXIS]));
    let desired_turn = if score < i32::default() {
        UNREFERENCED_STEERING_TURN_STEP
    } else {
        -UNREFERENCED_STEERING_TURN_STEP
    };
    let direction_changed = (steering.previous_turn ^ desired_turn) < i16::default();
    let turn = if direction_changed {
        desired_turn >> 1
    } else {
        desired_turn
    };
    steering.previous_turn = turn;
    primary.radial_offset = UNREFERENCED_STEERING_RADIAL_OFFSET;
    primary.angles[Y_AXIS] = primary.angles[Y_AXIS].wrapping_add(turn as u16);
    Ok(turn)
}

fn validate_state(
    species: AlienSpecies,
    pose: &AlienModelPose,
    animation: &AlienSlot2AnimationState,
) -> Result<(), AlienSlot2Error> {
    if pose.nodes.is_empty() {
        return Err(AlienSlot2Error::EmptyNodeList);
    }
    if pose.nodes.len() != animation.nodes.len() {
        return Err(AlienSlot2Error::NodeStateCountMismatch {
            pose: pose.nodes.len(),
            animation: animation.nodes.len(),
        });
    }
    if species != AlienSpecies::Amer && pose.nodes.len() < PRIMARY_AND_FOLLOWER_NODE_COUNT {
        return Err(AlienSlot2Error::MissingFollowerNode {
            node_count: pose.nodes.len(),
        });
    }
    Ok(())
}

fn transform_random(value: u16) -> u16 {
    value
        .rotate_right(RANDOM_ROTATION)
        .wrapping_sub((value >> RANDOM_BORROW_SHIFT) & RANDOM_BORROW_MASK)
}

fn transform_amer_motion_random(value: u16) -> u16 {
    value
        .rotate_right(AMER_MOTION_RANDOM_ROTATION)
        .wrapping_sub((value >> AMER_MOTION_RANDOM_BORROW_SHIFT) & AMER_MOTION_RANDOM_BORROW_MASK)
}

fn seed_step(species: AlienSpecies) -> u16 {
    match species {
        AlienSpecies::Amer => u16::MIN,
        AlienSpecies::Croolis => CROOLIS_SEED_STEP,
        AlienSpecies::Scrut => SCRUT_SEED_STEP,
    }
}

fn normalize_signed_angle(value: u16) -> i16 {
    (value << AMER_SIGNED_ANGLE_SHIFT).cast_signed() >> AMER_SIGNED_ANGLE_SHIFT
}

fn transformed_component(node: &super::AlienNodePose, axis: usize) -> i16 {
    ((node.transform.translation[axis] as u32 >> u16::BITS) as u16) as i16
}

fn prepare_amer_immediate_callback(
    pose: &AlienModelPose,
    animation: &mut AlienSlot2AnimationState,
    radial_target: u16,
    callback: AlienSlot2Callback,
) -> Result<AlienSlot2Callback, AlienSlot2Error> {
    validate_state(AlienSpecies::Amer, pose, animation)?;
    animation.nodes[PRIMARY_NODE].radial_target = radial_target;
    animation.callback = Some(callback);
    Ok(callback)
}

fn update_amer_followers(followers: &mut [super::AlienNodePose], roll: i16, animation_phase: u16) {
    let negative_phase = animation_phase.wrapping_neg();
    let roll_region = roll.wrapping_add(AMER_ROLL_REGION_BIAS);
    let values = if roll_region < RESET_SIGNED_VALUE {
        [
            (-AMER_FOLLOWER_PITCH, animation_phase),
            (AMER_FOLLOWER_PITCH, negative_phase),
            (-AMER_FOLLOWER_PITCH_DOUBLE, RESET_ANGLE),
            (AMER_FOLLOWER_PITCH_DOUBLE, RESET_ANGLE),
        ]
    } else if roll_region.wrapping_sub(AMER_ROLL_REGION_WIDTH) >= RESET_SIGNED_VALUE {
        [
            (-AMER_FOLLOWER_PITCH_DOUBLE, RESET_ANGLE),
            (AMER_FOLLOWER_PITCH_DOUBLE, RESET_ANGLE),
            (-AMER_FOLLOWER_PITCH, negative_phase),
            (AMER_FOLLOWER_PITCH, animation_phase),
        ]
    } else {
        [
            (-AMER_FOLLOWER_PITCH, animation_phase),
            (AMER_FOLLOWER_PITCH, negative_phase),
            (-AMER_FOLLOWER_PITCH, negative_phase),
            (AMER_FOLLOWER_PITCH, animation_phase),
        ]
    };
    for (node, (pitch, node_roll)) in followers.iter_mut().zip(values) {
        node.angles[X_AXIS] = pitch as u16;
        node.angles[Z_AXIS] = node_roll;
    }
}

fn update_amer_selection_pitch(primary: &mut super::AlienNodePose, camera_view_y: i16) {
    let position_y = primary.local_position[Y_AXIS] as i16;
    primary.angles[X_AXIS] = position_y
        .wrapping_add(camera_view_y)
        .wrapping_add(primary.angles[X_AXIS] as i16)
        .wrapping_shr(AMER_CAMERA_HEIGHT_EASING_SHIFT)
        .clamp(AMER_CAMERA_HEIGHT_MINIMUM, AMER_CAMERA_HEIGHT_MAXIMUM)
        as u16;
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{
        AXIS_COUNT, AlienFaceData, AlienNodeParent, AlienTransformData,
    };
    use serde::Deserialize;

    use super::*;
    use crate::native::alien::{AlienNodePose, AlienProjectedVertex};

    const SINGLE_VERTEX_COUNT: usize = 1;
    const UNCHANGED_PITCH: u16 = 0x4444;
    const TOUCHED_FIELD_SENTINEL: u16 = 0x5555;
    const RANDOM_STATE_SENTINEL: u16 = 0xa55a;
    const TRANSFORM_LOW_WORD_SENTINEL: u16 = 0x6a5a;

    #[derive(Deserialize)]
    struct Slot2Vector {
        name: String,
        module: String,
        path: String,
        state_count: Option<usize>,
        random_before: Option<u16>,
        random_after: Option<u16>,
        context_random: Option<u16>,
        seed_before: Option<u16>,
        seed_after: Option<u16>,
        duration_after: Option<u16>,
        motion_accumulator_after: Option<u16>,
        signed_seed_after: Option<i32>,
        node_states: Option<Vec<Slot2NodeVector>>,
    }

    #[derive(Clone, Copy, Deserialize)]
    struct Slot2NodeVector {
        local_x_before: u16,
        local_z_before: u16,
        pan_after: u16,
        roll_after: u16,
        radial_after: u16,
        velocity_after: u16,
        radial_target_after: u16,
        depth_target_after: u16,
    }

    const EMPTY_NODE_VECTOR: Slot2NodeVector = Slot2NodeVector {
        local_x_before: u16::MIN,
        local_z_before: u16::MIN,
        pan_after: u16::MIN,
        roll_after: u16::MIN,
        radial_after: u16::MIN,
        velocity_after: u16::MIN,
        radial_target_after: u16::MIN,
        depth_target_after: u16::MIN,
    };

    #[derive(Deserialize)]
    struct AmerReturnVector {
        name: String,
        path: String,
        countdown_before: u16,
        countdown_after: u16,
        velocity_before: [i16; AXIS_COUNT],
        velocity_after: [i16; AXIS_COUNT],
        position_before: [u32; AXIS_COUNT],
        position_after: [u32; AXIS_COUNT],
        pan_before: u16,
        pan_after: u16,
        roll_before: u16,
        roll_after: u16,
        radial_before: u16,
        radial_after: u16,
        active_before: u16,
        active_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerSteeringVector {
        name: String,
        path: String,
        camera_x_before: u16,
        camera_z_before: u16,
        camera_depth_step: u16,
        forward_x_before: u32,
        forward_z_before: u32,
        pan_before: u16,
        field_050_after: u16,
        countdown_before: u16,
        countdown_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerResetVector {
        name: String,
        random_before: u16,
        random_after: u16,
        velocity_x_after: i16,
        pitch_after: u16,
        roll_after: u16,
        radial_after: u16,
        countdown_after: u16,
        behavior_seed_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerSetupVector {
        name: String,
        next_stage: String,
        radial_target_before: u16,
        radial_target_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerFinishVector {
        name: String,
        path: String,
        countdown_before: u16,
        countdown_after: u16,
        camera_before: [u16; AXIS_COUNT],
        camera_depth_step: u16,
        forward_x_before: u32,
        forward_z_before: u32,
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        radial_before: u16,
        radial_after: u16,
        radial_target_before: u16,
        radial_target_after: u16,
    }

    #[derive(Clone, Copy, Deserialize)]
    struct AmerFollowerVector {
        pitch: u16,
        roll: u16,
    }

    #[derive(Deserialize)]
    struct AmerCommonVector {
        name: String,
        path: String,
        camera_translation_before: [u16; AXIS_COUNT],
        camera_pan: u16,
        camera_matrix: [u32; AXIS_COUNT],
        camera_position: [u32; AXIS_COUNT],
        camera_depth_step_before: u16,
        camera_depth_step_after: u16,
        phase_timer_before: u16,
        phase_timer_after: u16,
        velocity_before: [i16; AXIS_COUNT],
        velocity_after: [i16; AXIS_COUNT],
        animation_phase_before: u16,
        animation_phase_after: u16,
        position_before: [u32; AXIS_COUNT],
        position_after: [u32; AXIS_COUNT],
        pan_before: u16,
        pan_after: u16,
        roll_before: u16,
        roll_after: u16,
        radial_before: u16,
        radial_target: u16,
        radial_after: u16,
        followers_before: [AmerFollowerVector; AMER_COMMON_NODE_COUNT - 1],
        followers_after: [AmerFollowerVector; AMER_COMMON_NODE_COUNT - 1],
        active_before: u16,
        active_after: u16,
        callback_countdown_before: u16,
        callback_countdown_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerUpdateHeadVector {
        name: String,
        transfer: String,
        method_delta: u16,
        selection_state: u16,
        countdown_before: u16,
        countdown_after: u16,
        camera_before: [u16; AXIS_COUNT],
        pitch_before: u16,
        pitch_after: u16,
        roll_before: u16,
        random_before: u16,
        random_after: u16,
        velocity_x_before: u16,
        velocity_x_after: u16,
        radial_target_before: u16,
        radial_target_after: u16,
    }

    #[derive(Deserialize)]
    struct AmerSelectionVector {
        name: String,
        variant: String,
        continuation: String,
        selection_state: u16,
        camera_x_before: u16,
        camera_z_before: u16,
        position_y_low: u16,
        view_y: u16,
        forward_x_before: u32,
        forward_z_before: u32,
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        roll_before: u16,
        roll_after: u16,
        velocity_x_before: u16,
        velocity_x_after: u16,
        radial_target_before: u16,
        radial_target_after: u16,
    }

    #[derive(Deserialize)]
    struct UnreferencedSteeringVector {
        name: String,
        module: String,
        camera_x_before: u16,
        camera_z_before: u16,
        forward_x_before: u32,
        forward_z_before: u32,
        radial_before: u16,
        radial_after: u16,
        pan_before: u16,
        pan_after: u16,
        previous_turn_before: u16,
        turn_after: u16,
        direction_changed: bool,
    }

    #[derive(Default)]
    struct CallbackRecorder {
        calls: Vec<(AlienSpecies, AlienSlot2Callback)>,
    }

    impl AlienSlot2Callbacks for CallbackRecorder {
        fn invoke(
            &mut self,
            species: AlienSpecies,
            callback: AlienSlot2Callback,
            _pose: &mut AlienModelPose,
            _animation: &mut AlienSlot2AnimationState,
            _scene: &mut AlienSlot2SceneState,
        ) -> Result<(), AlienSlot2Error> {
            self.calls.push((species, callback));
            Ok(())
        }
    }

    fn fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_164c_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_16a4_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1692_natural.json"),
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

    fn pose(vectors: &[Slot2NodeVector]) -> AlienModelPose {
        let nodes = vectors
            .iter()
            .map(|vector| AlienNodePose {
                parent: AlienNodeParent::Root,
                scene_parent: None,
                first_vertex: usize::MIN,
                vertex_count: SINGLE_VERTEX_COUNT,
                transform: AlienTransformData::default(),
                local_position: [
                    i32::from(vector.local_x_before as i16),
                    i32::MIN,
                    i32::from(vector.local_z_before as i16),
                ],
                angles: [UNCHANGED_PITCH, vector.pan_after, vector.roll_after],
                radial_offset: vector.radial_after as i16,
            })
            .collect::<Vec<_>>();
        let node_count = nodes.len();
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes,
            projected_vertices: vec![AlienProjectedVertex::default(); node_count],
            texture_coordinates: vec![[i16::MIN; 2]; node_count],
            object_positions: vec![[i16::MIN; AXIS_COUNT]; node_count],
            authored_vertex_count: node_count,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    #[test]
    fn initialization_matches_typed_original_overlay_vectors() {
        for fixture in fixtures() {
            let vectors: Vec<Slot2Vector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors
                .into_iter()
                .filter(|vector| vector.path == "initialize")
            {
                let species = species(&vector.module);
                let node_vectors = vector.node_states.as_deref().unwrap();
                let node_count = vector.state_count.unwrap();
                if node_vectors.is_empty() || (species != AlienSpecies::Amer && node_count < 2) {
                    continue;
                }
                assert_eq!(node_vectors.len(), node_count);
                let mut pose = pose(node_vectors);
                let mut animation = AlienSlot2AnimationState::new(node_count);
                for (node_index, expected) in node_vectors.iter().enumerate() {
                    animation.nodes[node_index] = AlienSlot2NodeState {
                        motion_parameter: expected.velocity_after as i16,
                        radial_target: expected.radial_target_after,
                        depth_target: expected.depth_target_after as i16,
                        behavior_seed: u16::default(),
                    };
                }
                pose.nodes[PRIMARY_NODE].angles[Y_AXIS] = TOUCHED_FIELD_SENTINEL;
                animation.nodes[PRIMARY_NODE].radial_target = TOUCHED_FIELD_SENTINEL;
                if species != AlienSpecies::Amer {
                    pose.nodes[PRIMARY_NODE].angles[Z_AXIS] = TOUCHED_FIELD_SENTINEL;
                    pose.nodes[PRIMARY_NODE].radial_offset = TOUCHED_FIELD_SENTINEL as i16;
                    animation.nodes[PRIMARY_NODE].motion_parameter = TOUCHED_FIELD_SENTINEL as i16;
                    if species == AlienSpecies::Scrut {
                        animation.nodes[PRIMARY_NODE].depth_target = TOUCHED_FIELD_SENTINEL as i16;
                    }
                    for node in &mut animation.nodes[1..] {
                        node.motion_parameter = TOUCHED_FIELD_SENTINEL as i16;
                        if species == AlienSpecies::Scrut {
                            node.depth_target = TOUCHED_FIELD_SENTINEL as i16;
                        }
                    }
                }
                animation.phase_timer = TOUCHED_FIELD_SENTINEL as i16;
                animation.croolis_motion_accumulator = TOUCHED_FIELD_SENTINEL as i16;
                animation.species_seed_at_initialization = i32::from(TOUCHED_FIELD_SENTINEL);
                animation.random_value = TOUCHED_FIELD_SENTINEL;
                let mut scene = AlienSlot2SceneState {
                    random_state: vector.random_before.unwrap(),
                    species_seed: vector.seed_before.unwrap_or(u16::MIN),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_slot2(
                        species,
                        &mut pose,
                        &mut animation,
                        &mut scene,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienSlot2Update::Initialized,
                    "{}",
                    vector.name
                );

                assert_eq!(scene.random_state, vector.random_after.unwrap());
                assert_eq!(scene.species_seed, vector.seed_after.unwrap_or(u16::MIN));
                assert_eq!(animation.callback, Some(AlienSlot2Callback::Update));
                assert_eq!(animation.random_value, vector.context_random.unwrap());
                if let Some(duration) = vector.duration_after {
                    assert_eq!(animation.phase_timer as u16, duration);
                }
                if let Some(accumulator) = vector.motion_accumulator_after {
                    if species == AlienSpecies::Amer {
                        assert_eq!(animation.phase_timer as u16, accumulator);
                    } else {
                        assert_eq!(animation.croolis_motion_accumulator as u16, accumulator);
                    }
                }
                if let Some(seed) = vector.signed_seed_after {
                    assert_eq!(animation.species_seed_at_initialization, seed);
                }
                for (node_index, expected) in node_vectors.iter().enumerate() {
                    assert_eq!(pose.nodes[node_index].angles[0], UNCHANGED_PITCH);
                    assert_eq!(pose.nodes[node_index].angles[Y_AXIS], expected.pan_after);
                    assert_eq!(pose.nodes[node_index].angles[Z_AXIS], expected.roll_after);
                    assert_eq!(
                        pose.nodes[node_index].radial_offset as u16,
                        expected.radial_after
                    );
                    assert_eq!(
                        animation.nodes[node_index].motion_parameter as u16,
                        expected.velocity_after
                    );
                    assert_eq!(
                        animation.nodes[node_index].radial_target,
                        expected.radial_target_after
                    );
                    assert_eq!(
                        animation.nodes[node_index].depth_target as u16,
                        expected.depth_target_after
                    );
                }
                assert!(callbacks.calls.is_empty());
            }
        }
    }

    #[test]
    fn initialized_models_dispatch_their_typed_callback() {
        for fixture in fixtures() {
            let vectors: Vec<Slot2Vector> = serde_json::from_str(fixture).unwrap();
            let vector = vectors
                .into_iter()
                .find(|vector| vector.path == "callback")
                .unwrap();
            let species = species(&vector.module);
            let mut pose = pose(&[EMPTY_NODE_VECTOR; PRIMARY_AND_FOLLOWER_NODE_COUNT]);
            let mut animation = AlienSlot2AnimationState::new(PRIMARY_AND_FOLLOWER_NODE_COUNT);
            animation.initialized = true;
            animation.callback = Some(AlienSlot2Callback::Update);
            let mut scene = AlienSlot2SceneState::default();
            let mut callbacks = CallbackRecorder::default();

            assert_eq!(
                initialize_or_dispatch_slot2(
                    species,
                    &mut pose,
                    &mut animation,
                    &mut scene,
                    &mut callbacks,
                )
                .unwrap(),
                AlienSlot2Update::CallbackInvoked
            );
            assert_eq!(callbacks.calls, vec![(species, AlienSlot2Callback::Update)]);
        }
    }

    #[test]
    fn amer_return_matches_every_original_overlay_vector() {
        let vectors: Vec<AmerReturnVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_18d3_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            primary.local_position = vector.position_before.map(|value| value as i32);
            primary.angles = [UNCHANGED_PITCH, vector.pan_before, vector.roll_before];
            primary.radial_offset = vector.radial_before as i16;
            let mut animation = AlienSlot2AnimationState::new(SINGLE_VERTEX_COUNT);
            animation.callback = Some(AlienSlot2Callback::AmerReturn);
            animation.phase_timer = vector.countdown_before as i16;
            animation.amer_velocity = vector.velocity_before;
            let mut active = vector.active_before != u16::default();

            let expected_stage = if vector.path == "transition" {
                AlienAmerReturnUpdate::Restarted
            } else {
                AlienAmerReturnUpdate::Returning
            };
            assert_eq!(
                update_amer_return(&mut pose, &mut animation, &mut active).unwrap(),
                expected_stage,
                "{}",
                vector.name
            );

            let primary = &pose.nodes[PRIMARY_NODE];
            assert_eq!(animation.phase_timer as u16, vector.countdown_after);
            assert_eq!(animation.amer_velocity, vector.velocity_after);
            assert_eq!(
                primary.local_position.map(|value| value as u32),
                vector.position_after
            );
            assert_eq!(primary.angles[X_AXIS], UNCHANGED_PITCH);
            assert_eq!(primary.angles[Y_AXIS], vector.pan_after);
            assert_eq!(primary.angles[Z_AXIS], vector.roll_after);
            assert_eq!(primary.radial_offset as u16, vector.radial_after);
            assert_eq!(active, vector.active_after != u16::default());
            assert_eq!(
                animation.callback,
                Some(if vector.path == "transition" {
                    AlienSlot2Callback::Update
                } else {
                    AlienSlot2Callback::AmerReturn
                })
            );
        }
    }

    #[test]
    fn amer_steering_matches_every_original_overlay_vector() {
        let vectors: Vec<AmerSteeringVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_1a5c_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            primary.angles[Y_AXIS] = vector.pan_before;
            primary.transform.translation[X_AXIS] =
                join_words(vector.camera_x_before, TRANSFORM_LOW_WORD_SENTINEL);
            primary.transform.translation[Z_AXIS] =
                join_words(vector.camera_z_before, TRANSFORM_LOW_WORD_SENTINEL);
            primary.transform.matrix[X_AXIS][Z_AXIS] = vector.forward_x_before as i32;
            primary.transform.matrix[Z_AXIS][Z_AXIS] = vector.forward_z_before as i32;
            let mut animation = AlienSlot2AnimationState::new(SINGLE_VERTEX_COUNT);
            animation.callback = Some(AlienSlot2Callback::AmerSteer);
            animation.nodes[PRIMARY_NODE].motion_parameter = vector.countdown_before as i16;

            let expected_stage = if vector.path == "transition" {
                AlienAmerSteeringUpdate::FinishStarted
            } else {
                AlienAmerSteeringUpdate::Steering
            };
            assert_eq!(
                update_amer_steering(&mut pose, &mut animation, vector.camera_depth_step,).unwrap(),
                expected_stage,
                "{}",
                vector.name
            );

            assert_eq!(
                pose.nodes[PRIMARY_NODE].angles[Y_AXIS],
                vector.field_050_after
            );
            assert_eq!(
                animation.nodes[PRIMARY_NODE].motion_parameter as u16,
                vector.countdown_after
            );
            assert_eq!(
                animation.callback,
                Some(if vector.path == "transition" {
                    AlienSlot2Callback::AmerFinish
                } else {
                    AlienSlot2Callback::AmerSteer
                })
            );
        }
    }

    #[test]
    fn amer_reset_matches_every_original_overlay_vector() {
        let vectors: Vec<AmerResetVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_1a2b_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            primary.angles[X_AXIS] = TOUCHED_FIELD_SENTINEL;
            primary.angles[Z_AXIS] = TOUCHED_FIELD_SENTINEL;
            primary.radial_offset = TOUCHED_FIELD_SENTINEL as i16;
            let mut animation = AlienSlot2AnimationState::new(SINGLE_VERTEX_COUNT);
            animation.callback = Some(AlienSlot2Callback::Update);
            animation.random_value = vector.random_before;
            animation.amer_velocity[X_AXIS] = TOUCHED_FIELD_SENTINEL as i16;
            animation.nodes[PRIMARY_NODE].motion_parameter = TOUCHED_FIELD_SENTINEL as i16;
            animation.nodes[PRIMARY_NODE].behavior_seed = TOUCHED_FIELD_SENTINEL;

            reset_amer_motion(&mut pose, &mut animation).unwrap();

            let primary = &pose.nodes[PRIMARY_NODE];
            assert_eq!(
                animation.random_value, vector.random_after,
                "{}",
                vector.name
            );
            assert_eq!(animation.amer_velocity[X_AXIS], vector.velocity_x_after);
            assert_eq!(primary.angles[X_AXIS], vector.pitch_after);
            assert_eq!(primary.angles[Z_AXIS], vector.roll_after);
            assert_eq!(primary.radial_offset as u16, vector.radial_after);
            assert_eq!(
                animation.nodes[PRIMARY_NODE].motion_parameter as u16,
                vector.countdown_after
            );
            assert_eq!(
                animation.nodes[PRIMARY_NODE].behavior_seed,
                vector.behavior_seed_after
            );
            assert_eq!(animation.callback, Some(AlienSlot2Callback::AmerSteer));
        }
    }

    #[test]
    fn amer_immediate_transitions_match_every_original_overlay_vector() {
        for fixture in [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1688_natural.json"),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_193e_natural.json"),
        ] {
            let vectors: Vec<AmerSetupVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let pose = pose(&[EMPTY_NODE_VECTOR]);
                let mut animation = AlienSlot2AnimationState::new(SINGLE_VERTEX_COUNT);
                animation.callback = Some(AlienSlot2Callback::AmerReturn);
                animation.nodes[PRIMARY_NODE].radial_target = vector.radial_target_before;

                let expected_callback = match vector.next_stage.as_str() {
                    "update" => AlienSlot2Callback::Update,
                    "selection" => AlienSlot2Callback::AmerSelection,
                    stage => panic!("unknown AMER immediate stage {stage}"),
                };
                let callback = match expected_callback {
                    AlienSlot2Callback::Update => {
                        restart_amer_update(&pose, &mut animation).unwrap()
                    }
                    AlienSlot2Callback::AmerSelection => {
                        begin_amer_selection(&pose, &mut animation).unwrap()
                    }
                    _ => unreachable!("fixture only covers immediate AMER transitions"),
                };

                assert_eq!(callback, expected_callback, "{}", vector.name);
                assert_eq!(animation.callback, Some(expected_callback));
                assert_eq!(
                    animation.nodes[PRIMARY_NODE].radial_target,
                    vector.radial_target_after
                );
            }
        }
    }

    #[test]
    fn amer_finish_matches_every_original_overlay_vector() {
        let vectors: Vec<AmerFinishVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_1aa0_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            primary.angles = [vector.pitch_before, vector.pan_before, UNCHANGED_PITCH];
            primary.radial_offset = vector.radial_before as i16;
            for axis in usize::default()..AXIS_COUNT {
                primary.transform.translation[axis] =
                    join_words(vector.camera_before[axis], TRANSFORM_LOW_WORD_SENTINEL);
            }
            primary.transform.matrix[X_AXIS][Z_AXIS] = vector.forward_x_before as i32;
            primary.transform.matrix[Z_AXIS][Z_AXIS] = vector.forward_z_before as i32;
            let mut animation = AlienSlot2AnimationState::new(SINGLE_VERTEX_COUNT);
            animation.callback = Some(AlienSlot2Callback::AmerFinish);
            animation.nodes[PRIMARY_NODE].motion_parameter = vector.countdown_before as i16;
            animation.nodes[PRIMARY_NODE].radial_target = vector.radial_target_before;

            let expected_stage = match vector.path.as_str() {
                "reset" => AlienAmerFinishUpdate::ResetRequested,
                "selection" => AlienAmerFinishUpdate::SelectionWaitStarted,
                "steering" => AlienAmerFinishUpdate::Steering,
                path => panic!("unknown AMER finish path {path}"),
            };
            assert_eq!(
                update_amer_finish(&mut pose, &mut animation, vector.camera_depth_step,).unwrap(),
                expected_stage,
                "{}",
                vector.name
            );

            let primary = &pose.nodes[PRIMARY_NODE];
            assert_eq!(
                animation.nodes[PRIMARY_NODE].motion_parameter as u16,
                vector.countdown_after
            );
            assert_eq!(primary.angles[X_AXIS], vector.pitch_after);
            assert_eq!(primary.angles[Y_AXIS], vector.pan_after);
            assert_eq!(primary.radial_offset as u16, vector.radial_after);
            assert_eq!(
                animation.nodes[PRIMARY_NODE].radial_target,
                vector.radial_target_after
            );
            assert_eq!(
                animation.callback,
                Some(if vector.path == "selection" {
                    AlienSlot2Callback::AmerSelectionWait
                } else {
                    AlienSlot2Callback::AmerFinish
                })
            );
        }
    }

    #[test]
    fn amer_common_update_matches_every_original_overlay_vector() {
        let vectors: Vec<AmerCommonVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_171d_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR; AMER_COMMON_NODE_COUNT]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            primary.local_position = vector.position_before.map(|value| value as i32);
            primary.angles = [UNCHANGED_PITCH, vector.pan_before, vector.roll_before];
            primary.radial_offset = vector.radial_before as i16;
            for axis in usize::default()..AXIS_COUNT {
                primary.transform.translation[axis] = join_words(
                    vector.camera_translation_before[axis],
                    TRANSFORM_LOW_WORD_SENTINEL,
                );
            }
            for (node, follower) in pose.nodes[1..].iter_mut().zip(vector.followers_before) {
                node.angles[X_AXIS] = follower.pitch;
                node.angles[Z_AXIS] = follower.roll;
            }
            let mut animation = AlienSlot2AnimationState::new(AMER_COMMON_NODE_COUNT);
            animation.callback = Some(AlienSlot2Callback::Update);
            animation.phase_timer = vector.phase_timer_before as i16;
            animation.amer_velocity = vector.velocity_before;
            animation.random_value = RANDOM_STATE_SENTINEL;
            animation.amer_animation_phase = vector.animation_phase_before;
            animation.nodes[PRIMARY_NODE].radial_target = vector.radial_target;
            let mut scene = AlienCallbackSceneState {
                slot2_active: vector.active_before != u16::default(),
                callback_countdown: vector.callback_countdown_before,
                ..AlienCallbackSceneState::default()
            };
            let mut camera = AlienCameraTransform {
                position: vector.camera_position.map(|value| value as i32),
                ..AlienCameraTransform::default()
            };
            for axis in usize::default()..AXIS_COUNT {
                camera.matrix[Z_AXIS][axis] = vector.camera_matrix[axis] as i32;
            }
            let mut camera_depth_step = vector.camera_depth_step_before as i16;

            let expected_stage = match vector.path.as_str() {
                "motion" => AlienAmerCommonUpdate::MotionUpdated,
                "camera_facing" => AlienAmerCommonUpdate::CameraFacing,
                "return_started" => AlienAmerCommonUpdate::ReturnStarted,
                path => panic!("unknown AMER common path {path}"),
            };
            assert_eq!(
                update_amer_common(
                    &mut pose,
                    &mut animation,
                    &mut scene,
                    &camera,
                    vector.camera_pan,
                    &mut camera_depth_step,
                )
                .unwrap(),
                expected_stage,
                "{}",
                vector.name
            );

            let primary = &pose.nodes[PRIMARY_NODE];
            assert_eq!(primary.angles[X_AXIS], UNCHANGED_PITCH);
            assert_eq!(primary.angles[Y_AXIS], vector.pan_after);
            assert_eq!(primary.angles[Z_AXIS], vector.roll_after);
            assert_eq!(primary.radial_offset as u16, vector.radial_after);
            assert_eq!(
                primary.local_position.map(|value| value as u32),
                vector.position_after
            );
            assert_eq!(animation.phase_timer as u16, vector.phase_timer_after);
            assert_eq!(animation.amer_velocity, vector.velocity_after);
            assert_eq!(animation.random_value, RANDOM_STATE_SENTINEL);
            assert_eq!(animation.amer_animation_phase, vector.animation_phase_after);
            assert_eq!(camera_depth_step as u16, vector.camera_depth_step_after);
            assert_eq!(scene.slot2_active, vector.active_after != u16::default());
            assert_eq!(scene.callback_countdown, vector.callback_countdown_after);
            assert_eq!(
                animation.callback,
                Some(if vector.path == "return_started" {
                    AlienSlot2Callback::AmerReturn
                } else {
                    AlienSlot2Callback::Update
                })
            );
            for (node, expected) in pose.nodes[1..].iter().zip(vector.followers_after) {
                assert_eq!(node.angles[X_AXIS], expected.pitch);
                assert_eq!(node.angles[Z_AXIS], expected.roll);
            }
        }
    }

    #[test]
    fn amer_update_head_matches_every_isolated_original_overlay_vector() {
        let vectors: Vec<AmerUpdateHeadVector> = serde_json::from_str(include_str!(
            "../../../../../re/tools/oracle_vectors/xdb_amer_func_1692_head_natural.json"
        ))
        .unwrap();
        for vector in vectors {
            let mut pose = pose(&[EMPTY_NODE_VECTOR]);
            let primary = &mut pose.nodes[PRIMARY_NODE];
            for axis in usize::default()..AXIS_COUNT {
                primary.transform.translation[axis] =
                    join_words(vector.camera_before[axis], TRANSFORM_LOW_WORD_SENTINEL);
            }
            primary.angles[X_AXIS] = vector.pitch_before;
            primary.angles[Z_AXIS] = vector.roll_before;
            let mut animation = AlienSlot2AnimationState::new(1);
            animation.callback = Some(AlienSlot2Callback::Update);
            animation.phase_timer = vector.countdown_before as i16;
            animation.random_value = vector.random_before;
            animation.amer_velocity[X_AXIS] = vector.velocity_x_before as i16;
            animation.nodes[PRIMARY_NODE].radial_target = vector.radial_target_before;
            let scene = AlienCallbackSceneState {
                method_delta: vector.method_delta as i16,
                wave_selection: match vector.selection_state {
                    0 => AlienWaveSelection::Disabled,
                    1 => AlienWaveSelection::Requested,
                    2 => AlienWaveSelection::Selected,
                    state => panic!("unknown wave-selection state {state}"),
                },
                ..AlienCallbackSceneState::default()
            };
            let expected = match vector.transfer.as_str() {
                "selection" => AlienAmerUpdateHead::SelectionRequested,
                "reset" => AlienAmerUpdateHead::ResetRequested,
                "common" => AlienAmerUpdateHead::CommonRequested,
                transfer => panic!("unknown AMER head transfer {transfer}"),
            };

            assert_eq!(
                update_amer_head(&mut pose, &mut animation, &scene).unwrap(),
                expected,
                "{}",
                vector.name
            );
            assert_eq!(animation.phase_timer as u16, vector.countdown_after);
            assert_eq!(animation.random_value, vector.random_after);
            assert_eq!(
                animation.amer_velocity[X_AXIS] as u16,
                vector.velocity_x_after
            );
            assert_eq!(
                animation.nodes[PRIMARY_NODE].radial_target,
                vector.radial_target_after
            );
            assert_eq!(pose.nodes[PRIMARY_NODE].angles[X_AXIS], vector.pitch_after);
            assert_eq!(animation.callback, Some(AlienSlot2Callback::Update));
        }
    }

    #[test]
    fn amer_selection_callbacks_match_every_isolated_original_overlay_vector() {
        for input in [
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_amer_func_1948_head_natural.json"
            ),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_amer_func_19cb_head_natural.json"
            ),
        ] {
            let vectors: Vec<AmerSelectionVector> = serde_json::from_str(input).unwrap();
            for vector in vectors {
                assert_amer_selection_vector(vector);
            }
        }
    }

    #[test]
    fn unreferenced_steering_matches_all_three_original_overlays() {
        for input in [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1b1a_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1a86_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1b3b_natural.json"),
        ] {
            let vectors: Vec<UnreferencedSteeringVector> = serde_json::from_str(input).unwrap();
            for vector in vectors {
                let mut pose = pose(&[EMPTY_NODE_VECTOR]);
                let primary = &mut pose.nodes[PRIMARY_NODE];
                primary.transform.translation[X_AXIS] =
                    join_words(vector.camera_x_before, TRANSFORM_LOW_WORD_SENTINEL);
                primary.transform.translation[Z_AXIS] =
                    join_words(vector.camera_z_before, TRANSFORM_LOW_WORD_SENTINEL);
                primary.transform.matrix[X_AXIS][Z_AXIS] = vector.forward_x_before as i32;
                primary.transform.matrix[Z_AXIS][Z_AXIS] = vector.forward_z_before as i32;
                primary.radial_offset = vector.radial_before as i16;
                primary.angles[Y_AXIS] = vector.pan_before;
                let mut steering = AlienUnreferencedSteeringState {
                    previous_turn: vector.previous_turn_before as i16,
                };

                let direction_changed = (steering.previous_turn
                    ^ if vector.turn_after as i16 >= i16::default() {
                        UNREFERENCED_STEERING_TURN_STEP
                    } else {
                        -UNREFERENCED_STEERING_TURN_STEP
                    })
                    < i16::default();
                assert_eq!(
                    direction_changed, vector.direction_changed,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    update_unreferenced_steering(&mut pose, &mut steering).unwrap() as u16,
                    vector.turn_after,
                    "{}:{}",
                    vector.module,
                    vector.name
                );
                assert_eq!(steering.previous_turn as u16, vector.turn_after);
                assert_eq!(
                    pose.nodes[PRIMARY_NODE].radial_offset as u16,
                    vector.radial_after
                );
                assert_eq!(pose.nodes[PRIMARY_NODE].angles[Y_AXIS], vector.pan_after);
            }
        }
    }

    #[test]
    fn invalid_flat_node_shapes_are_rejected() {
        let mut empty_pose = pose(&[]);
        let mut empty_animation = AlienSlot2AnimationState::new(usize::MIN);
        let mut scene = AlienSlot2SceneState::default();
        let mut callbacks = CallbackRecorder::default();
        assert_eq!(
            initialize_or_dispatch_slot2(
                AlienSpecies::Amer,
                &mut empty_pose,
                &mut empty_animation,
                &mut scene,
                &mut callbacks,
            ),
            Err(AlienSlot2Error::EmptyNodeList)
        );

        let node = EMPTY_NODE_VECTOR;
        let mut single_pose = pose(&[node]);
        let mut single_animation = AlienSlot2AnimationState::new(1);
        assert_eq!(
            initialize_or_dispatch_slot2(
                AlienSpecies::Croolis,
                &mut single_pose,
                &mut single_animation,
                &mut scene,
                &mut callbacks,
            ),
            Err(AlienSlot2Error::MissingFollowerNode { node_count: 1 })
        );
    }

    fn join_words(high: u16, low: u16) -> i32 {
        (u32::from(high) << u16::BITS | u32::from(low)) as i32
    }

    fn assert_amer_selection_vector(vector: AmerSelectionVector) {
        let mut pose = pose(&[EMPTY_NODE_VECTOR]);
        let primary = &mut pose.nodes[PRIMARY_NODE];
        primary.transform.translation[X_AXIS] =
            join_words(vector.camera_x_before, TRANSFORM_LOW_WORD_SENTINEL);
        primary.transform.translation[Z_AXIS] =
            join_words(vector.camera_z_before, TRANSFORM_LOW_WORD_SENTINEL);
        primary.transform.matrix[X_AXIS][Z_AXIS] = vector.forward_x_before as i32;
        primary.transform.matrix[Z_AXIS][Z_AXIS] = vector.forward_z_before as i32;
        primary.local_position[Y_AXIS] = join_words(0xA5A5, vector.position_y_low);
        primary.angles[X_AXIS] = vector.pitch_before;
        primary.angles[Y_AXIS] = vector.pan_before;
        primary.angles[Z_AXIS] = vector.roll_before;
        let mut animation = AlienSlot2AnimationState::new(1);
        animation.callback = Some(if vector.variant == "late" {
            AlienSlot2Callback::AmerSelectionLate
        } else {
            AlienSlot2Callback::AmerSelection
        });
        animation.amer_velocity[X_AXIS] = vector.velocity_x_before as i16;
        animation.nodes[PRIMARY_NODE].radial_target = vector.radial_target_before;
        let scene = AlienCallbackSceneState {
            wave_selection: match vector.selection_state {
                0 => AlienWaveSelection::Disabled,
                1 => AlienWaveSelection::Requested,
                2 => AlienWaveSelection::Selected,
                state => panic!("unknown wave-selection state {state}"),
            },
            ..AlienCallbackSceneState::default()
        };

        if vector.variant == "late" {
            let expected = match vector.continuation.as_str() {
                "selection_wait" => AlienAmerLateSelectionUpdate::SelectionWaitRequested,
                "reset" => AlienAmerLateSelectionUpdate::ResetRequested,
                "common" => AlienAmerLateSelectionUpdate::CommonRequested,
                continuation => panic!("unknown late-selection continuation {continuation}"),
            };
            assert_eq!(
                update_amer_late_selection(&mut pose, &mut animation, vector.view_y as i16,)
                    .unwrap(),
                expected,
                "{}",
                vector.name
            );
        } else {
            let expected = match vector.continuation.as_str() {
                "restart" => AlienAmerSelectionUpdate::RestartRequested,
                "reset" => AlienAmerSelectionUpdate::ResetRequested,
                "late" => AlienAmerSelectionUpdate::LateSelectionStarted,
                "common" => AlienAmerSelectionUpdate::CommonRequested,
                continuation => panic!("unknown selection continuation {continuation}"),
            };
            assert_eq!(
                update_amer_selection(&mut pose, &mut animation, &scene, vector.view_y as i16,)
                    .unwrap(),
                expected,
                "{}",
                vector.name
            );
        }

        let primary = &pose.nodes[PRIMARY_NODE];
        assert_eq!(primary.angles[X_AXIS], vector.pitch_after);
        assert_eq!(primary.angles[Y_AXIS], vector.pan_after);
        assert_eq!(primary.angles[Z_AXIS], vector.roll_after);
        assert_eq!(
            animation.amer_velocity[X_AXIS] as u16,
            vector.velocity_x_after
        );
        assert_eq!(
            animation.nodes[PRIMARY_NODE].radial_target,
            vector.radial_target_after
        );
        assert_eq!(
            animation.callback,
            Some(
                if vector.continuation == "late" || vector.variant == "late" {
                    AlienSlot2Callback::AmerSelectionLate
                } else {
                    AlienSlot2Callback::AmerSelection
                }
            )
        );
    }
}
