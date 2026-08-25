//! Typed state and callbacks for alien resume behavior.

use std::fmt;

use commander_blood_formats::alien::{AlienTrigonometryPair, TRIGONOMETRY_ENTRY_COUNT};

use super::{
    ALIEN_TRANSITION_QUEUE_CAPACITY, AlienCallbackSceneState, AlienNodePose, AlienRingCallback,
    AlienSceneNode, AlienSpecies,
};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const PITCH_AXIS: usize = 0;
const PAN_AXIS: usize = 1;
const AMER_DEPTH_BOUND: i32 = 100;
const OTHER_DEPTH_BOUND: i32 = 200;
const LATERAL_BOUND: i32 = 200;
const VERTICAL_MINIMUM: i16 = -200;
const VERTICAL_MAXIMUM_EXCLUSIVE: i16 = 200;
const VERTICAL_EASING_SHIFT: u32 = 3;
const PITCH_EASING_SHIFT: u32 = 1;
const ANGLE_MASK: u16 = 0x0ffc;
const ANGLE_INDEX_SHIFT: u32 = 2;
const NEGATIVE_DIRECTION_PAN_STEP: i16 = -32;
const NONNEGATIVE_DIRECTION_PAN_STEP: i16 = 16;
const TEXTURE_COMPONENT_COUNT: usize = 2;
const TEXTURE_U_COMPONENT: usize = 0;
const TEXTURE_V_COMPONENT: usize = 1;
const PHASE_HIGH_BYTE_SHIFT: u32 = 8;
const PHASE_HIGH_CLAMP: i8 = 22;
const PHASE_ZERO_STEP: u8 = 2;
const PHASE_REVERSE_STEP: u8 = 254;
const AMER_RESUME_TEXTURE_VERTEX_COUNT: usize = 54;
const CROOLIS_RESUME_TEXTURE_VERTEX_COUNT: usize = 26;
const SCRUT_RESUME_TEXTURE_VERTEX_COUNT: usize = 44;
const PAIRED_RESUME_COUNTDOWN: u16 = 24;
const FINAL_RETURN_RADIAL_OFFSET: i16 = 100;
/// Number of typed slots in the recovered resume queue.
pub const ALIEN_RESUME_QUEUE_CAPACITY: usize = ALIEN_TRANSITION_QUEUE_CAPACITY;
const IDLE_PITCH_STEP: u16 = 2_016;
const IDLE_PITCH_MASK: u16 = 0x0ffc;
const IDLE_PITCH_BIAS: u16 = 2_048;
const AMER_IDLE_PAN_STEP: u16 = 8;
const AMER_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 4] = [
    (0, TextureDirection::Add),
    (53, TextureDirection::Subtract),
    (35, TextureDirection::Add),
    (25, TextureDirection::Subtract),
];
const CROOLIS_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 2] =
    [(0, TextureDirection::Add), (25, TextureDirection::Subtract)];
const SCRUT_RESUME_TEXTURE_TARGETS: [(usize, TextureDirection); 4] = [
    (0, TextureDirection::Add),
    (43, TextureDirection::Subtract),
    (42, TextureDirection::Add),
    (25, TextureDirection::Subtract),
];

#[derive(Clone, Copy)]
enum TextureDirection {
    Add,
    Subtract,
}

/// Resume callback selected by the recovered slot-13 coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeCallback {
    /// Begin the species-specific resume state machine.
    Begin,
    /// Move the current node toward its queued partner.
    Pair,
    /// Continue texture motion while the resume delay expires.
    Timeout,
    /// Move the current node back toward the active queue anchor.
    Final,
}

/// Typed continuation state owned by one resumable behavior method.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienResumeMethodState {
    /// Callback selected for the next coordinator invocation.
    pub callback: Option<AlienResumeCallback>,
    /// Current phase within the resume state machine.
    pub phase: u16,
    /// Optional node paired with the currently resumed node.
    pub paired_node: Option<AlienSceneNode>,
    /// Optional node whose state is being resumed.
    pub resumed_node: Option<AlienSceneNode>,
}

/// Resolved partner borrowed by an occupied resume queue slot.
pub struct AlienResumePairContext<'a> {
    /// Exact scene-node identity stored in the queue.
    pub node: AlienSceneNode,
    /// Mutable pose already resolved by the scene owner.
    pub pose: &'a mut AlienNodePose,
    /// Ring callback owned by the resolved partner.
    pub callback: &'a mut AlienRingCallback,
}

/// Borrowed flat state needed when a resume queue slot is consumed.
pub struct AlienResumeQueueContext<'a> {
    /// Exact scene-node identity whose method is consuming the queue.
    pub current: AlienSceneNode,
    /// Mutable pose already resolved by the scene owner.
    pub current_pose: &'a mut AlienNodePose,
    /// Texture coordinates owned by the current node's model.
    pub texture_coordinates: &'a mut [[i16; TEXTURE_COMPONENT_COUNT]],
    /// Resolved partner for an occupied slot, or `None` for an empty slot.
    pub paired: Option<AlienResumePairContext<'a>>,
    /// Decoded trigonometry table used by pair steering.
    pub trigonometry: &'a [AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    /// Shared resume countdown updated by an immediate in-range pairing.
    pub countdown: &'a mut u16,
}

/// Callback boundary retained by the recovered resume coordinator.
pub trait AlienResumeCallbacks {
    /// Error returned by the concrete callback implementation.
    type Error;

    /// Invoke the selected resume callback.
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienResumeCallback,
        state: &mut AlienResumeMethodState,
    ) -> Result<(), Self::Error>;
}

/// Stage completed by one invocation of the resume coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeUpdate {
    /// The initial callback and empty pairing state were installed.
    Initialized,
    /// The previously selected callback was invoked.
    CallbackInvoked,
}

/// Spatial relationship found while steering a resumed model pair.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumePairUpdate {
    /// Both nodes are inside the species-specific pairing bounds.
    Inside,
    /// The current node was steered toward an outlying paired node.
    Outside,
}

/// Texture animation result produced by one resume-state update.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTextureUpdate {
    /// Signed low-byte displacement applied to the selected coordinates.
    pub delta: i16,
    /// Packed animation phase after its wrapping high-byte advance.
    pub phase: u16,
}

/// State produced by one resume timeout continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTimeoutUpdate {
    /// Texture-coordinate motion completed before the countdown update.
    pub texture: AlienResumeTextureUpdate,
    /// Wrapping post-decrement countdown value.
    pub countdown: u16,
    /// Whether the post-decrement sign selected the final continuation.
    pub final_stage_selected: bool,
}

/// State produced by one paired-node approach continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumePairStageUpdate {
    /// Texture-coordinate motion completed before node steering.
    pub texture: AlienResumeTextureUpdate,
    /// Whether the paired nodes remain apart after radial averaging.
    pub relationship: AlienResumePairUpdate,
}

/// Result of consuming one resume queue slot.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeQueueUpdate {
    /// An empty slot advanced the read index and applied idle angle drift.
    Idle {
        /// Slot inspected by the next invocation.
        read_slot: usize,
    },
    /// An occupied slot was consumed and immediately ran the pair stage.
    PairDispatched {
        /// Typed scene-node identity removed from the queue.
        paired_node: AlienSceneNode,
        /// Result of the same-invocation pair continuation.
        pair: AlienResumePairStageUpdate,
    },
}

/// Invalid model data supplied to the recovered resume texture animation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AlienResumeTextureError {
    /// Number of texture coordinates required by the species animation.
    pub required: usize,
    /// Number of texture coordinates present in the decoded model.
    pub available: usize,
}

impl fmt::Display for AlienResumeTextureError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "resume texture animation requires {} vertices, but the model contains {}",
            self.required, self.available
        )
    }
}

impl std::error::Error for AlienResumeTextureError {}

/// Invalid typed state supplied to the paired-node resume continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumePairStageError {
    /// The method has no typed index for the node represented by `other`.
    MissingPairedNode,
    /// The model cannot satisfy the species-specific texture animation.
    Texture(AlienResumeTextureError),
}

impl fmt::Display for AlienResumePairStageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien resume pair state: {self:?}")
    }
}

impl std::error::Error for AlienResumePairStageError {}

impl From<AlienResumeTextureError> for AlienResumePairStageError {
    fn from(error: AlienResumeTextureError) -> Self {
        Self::Texture(error)
    }
}

/// Invalid typed state supplied to the final resume continuation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeFinalStageError {
    /// The method has no typed index for the node that must restart.
    MissingPairedNode,
}

impl fmt::Display for AlienResumeFinalStageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien resume final state: {self:?}")
    }
}

impl std::error::Error for AlienResumeFinalStageError {}

/// Invalid flat state supplied to the begin-stage queue owner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienResumeQueueError {
    /// The queue cursor does not select one of its typed slots.
    InvalidReadSlot {
        /// Invalid slot index.
        index: usize,
    },
    /// An empty queue slot was supplied with an unrelated partner borrow.
    UnexpectedPairContext,
    /// An occupied queue slot was not supplied with its resolved partner.
    MissingPairContext {
        /// Scene node that the caller must resolve.
        queued: AlienSceneNode,
    },
    /// The supplied partner does not match the identity stored in the queue.
    MismatchedPair {
        /// Scene node stored in the queue.
        queued: AlienSceneNode,
        /// Scene node resolved by the caller.
        supplied: AlienSceneNode,
    },
    /// A scene node cannot pair with itself.
    AliasedPair {
        /// Repeated scene-node identity.
        node: AlienSceneNode,
    },
    /// The immediate pair continuation rejected its typed inputs.
    PairStage(AlienResumePairStageError),
}

impl fmt::Display for AlienResumeQueueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien resume queue state: {self:?}")
    }
}

impl std::error::Error for AlienResumeQueueError {}

impl From<AlienResumePairStageError> for AlienResumeQueueError {
    fn from(error: AlienResumePairStageError) -> Self {
        Self::PairStage(error)
    }
}

/// Initialize or dispatch the recovered slot-13 resume method.
pub fn initialize_or_dispatch_resume<C: AlienResumeCallbacks>(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    callbacks: &mut C,
) -> Result<AlienResumeUpdate, C::Error> {
    if let Some(callback) = state.callback {
        callbacks.invoke(species, callback, state)?;
        return Ok(AlienResumeUpdate::CallbackInvoked);
    }

    state.callback = Some(AlienResumeCallback::Begin);
    state.phase = u16::MIN;
    state.paired_node = None;
    Ok(AlienResumeUpdate::Initialized)
}

/// Consume one queue slot and immediately dispatch an acquired pair.
pub fn update_resume_queue(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    scene: &mut AlienCallbackSceneState,
    context: AlienResumeQueueContext<'_>,
) -> Result<AlienResumeQueueUpdate, AlienResumeQueueError> {
    let AlienResumeQueueContext {
        current,
        current_pose,
        texture_coordinates,
        paired,
        trigonometry,
        countdown,
    } = context;
    if scene.transition_queue_read_slot >= ALIEN_RESUME_QUEUE_CAPACITY {
        return Err(AlienResumeQueueError::InvalidReadSlot {
            index: scene.transition_queue_read_slot,
        });
    }
    let Some(paired_node) = scene.transition_queue[scene.transition_queue_read_slot] else {
        if paired.is_some() {
            return Err(AlienResumeQueueError::UnexpectedPairContext);
        }
        scene.transition_queue_read_slot =
            (scene.transition_queue_read_slot + 1) % ALIEN_RESUME_QUEUE_CAPACITY;
        let pitch = match species {
            AlienSpecies::Amer => current_pose.angles[PITCH_AXIS].wrapping_add(IDLE_PITCH_STEP),
            AlienSpecies::Croolis | AlienSpecies::Scrut => {
                current_pose.angles[PITCH_AXIS].wrapping_sub(IDLE_PITCH_STEP)
            }
        };
        current_pose.angles[PITCH_AXIS] = (pitch & IDLE_PITCH_MASK).wrapping_sub(IDLE_PITCH_BIAS);
        if species == AlienSpecies::Amer {
            current_pose.angles[PAN_AXIS] =
                current_pose.angles[PAN_AXIS].wrapping_add(AMER_IDLE_PAN_STEP);
        }
        return Ok(AlienResumeQueueUpdate::Idle {
            read_slot: scene.transition_queue_read_slot,
        });
    };

    let AlienResumePairContext {
        node: supplied_node,
        pose: paired_pose,
        callback: paired_callback,
    } = paired.ok_or(AlienResumeQueueError::MissingPairContext {
        queued: paired_node,
    })?;
    if supplied_node != paired_node {
        return Err(AlienResumeQueueError::MismatchedPair {
            queued: paired_node,
            supplied: supplied_node,
        });
    }
    if paired_node == current {
        return Err(AlienResumeQueueError::AliasedPair { node: paired_node });
    }
    let required_textures = resume_texture_vertex_count(species);
    if texture_coordinates.len() < required_textures {
        return Err(AlienResumePairStageError::Texture(AlienResumeTextureError {
            required: required_textures,
            available: texture_coordinates.len(),
        })
        .into());
    }

    scene.active_node = None;
    scene.transition_queue[scene.transition_queue_read_slot] = None;
    state.callback = Some(AlienResumeCallback::Pair);
    state.paired_node = Some(paired_node);

    let pair = update_resume_pair_stage(
        species,
        state,
        current_pose,
        paired_pose,
        paired_callback,
        texture_coordinates,
        trigonometry,
        countdown,
    )?;
    Ok(AlienResumeQueueUpdate::PairDispatched { paired_node, pair })
}

fn resume_texture_vertex_count(species: AlienSpecies) -> usize {
    match species {
        AlienSpecies::Amer => AMER_RESUME_TEXTURE_VERTEX_COUNT,
        AlienSpecies::Croolis => CROOLIS_RESUME_TEXTURE_VERTEX_COUNT,
        AlienSpecies::Scrut => SCRUT_RESUME_TEXTURE_VERTEX_COUNT,
    }
}

/// Animate the species-specific texture vertices used by the resume sequence.
pub fn update_resume_texture_motion(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    texture_coordinates: &mut [[i16; TEXTURE_COMPONENT_COUNT]],
) -> Result<AlienResumeTextureUpdate, AlienResumeTextureError> {
    let (component, required, targets): (_, _, &[(usize, TextureDirection)]) = match species {
        AlienSpecies::Amer => (
            TEXTURE_V_COMPONENT,
            AMER_RESUME_TEXTURE_VERTEX_COUNT,
            &AMER_RESUME_TEXTURE_TARGETS,
        ),
        AlienSpecies::Croolis => (
            TEXTURE_U_COMPONENT,
            CROOLIS_RESUME_TEXTURE_VERTEX_COUNT,
            &CROOLIS_RESUME_TEXTURE_TARGETS,
        ),
        AlienSpecies::Scrut => (
            TEXTURE_V_COMPONENT,
            SCRUT_RESUME_TEXTURE_VERTEX_COUNT,
            &SCRUT_RESUME_TEXTURE_TARGETS,
        ),
    };
    if texture_coordinates.len() < required {
        return Err(AlienResumeTextureError {
            required,
            available: texture_coordinates.len(),
        });
    }

    let low = state.phase as u8;
    let delta = i16::from(low as i8);
    for &(vertex, direction) in targets {
        let coordinate = &mut texture_coordinates[vertex][component];
        *coordinate = match direction {
            TextureDirection::Add => coordinate.wrapping_add(delta),
            TextureDirection::Subtract => coordinate.wrapping_sub(delta),
        };
    }

    let high = ((state.phase >> PHASE_HIGH_BYTE_SHIFT) as u8).wrapping_add(low);
    let next_low = if (high as i8) >= PHASE_HIGH_CLAMP {
        PHASE_REVERSE_STEP
    } else if high == u8::default() {
        PHASE_ZERO_STEP
    } else {
        low
    };
    state.phase = (u16::from(high) << PHASE_HIGH_BYTE_SHIFT) | u16::from(next_low);
    Ok(AlienResumeTextureUpdate {
        delta,
        phase: state.phase,
    })
}

/// Advance resume texture motion and select the final stage after timeout.
pub fn update_resume_timeout(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    texture_coordinates: &mut [[i16; TEXTURE_COMPONENT_COUNT]],
    countdown: &mut u16,
) -> Result<AlienResumeTimeoutUpdate, AlienResumeTextureError> {
    let texture = update_resume_texture_motion(species, state, texture_coordinates)?;
    *countdown = countdown.wrapping_sub(1);
    let final_stage_selected = (*countdown as i16).is_negative();
    if final_stage_selected {
        state.callback = Some(AlienResumeCallback::Final);
    }
    Ok(AlienResumeTimeoutUpdate {
        texture,
        countdown: *countdown,
        final_stage_selected,
    })
}

/// Animate and steer one node toward its queued resume partner.
#[allow(clippy::too_many_arguments)]
pub fn update_resume_pair_stage(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    current: &mut AlienNodePose,
    other: &mut AlienNodePose,
    other_callback: &mut AlienRingCallback,
    texture_coordinates: &mut [[i16; TEXTURE_COMPONENT_COUNT]],
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
    countdown: &mut u16,
) -> Result<AlienResumePairStageUpdate, AlienResumePairStageError> {
    let paired_node = state
        .paired_node
        .ok_or(AlienResumePairStageError::MissingPairedNode)?;
    let texture = update_resume_texture_motion(species, state, texture_coordinates)?;
    let half_other = other.radial_offset >> 1;
    let average = other
        .radial_offset
        .wrapping_add(current.radial_offset)
        .wrapping_add(half_other);
    current.radial_offset = average >> 1;
    let relationship = update_resume_pair_steering(species, current, other, trigonometry);
    if relationship == AlienResumePairUpdate::Inside {
        state.callback = Some(AlienResumeCallback::Timeout);
        current.radial_offset = i16::default();
        *other_callback = AlienRingCallback::BeginResumeClear;
        state.resumed_node = Some(paired_node);
        *countdown = PAIRED_RESUME_COUNTDOWN;
    }
    Ok(AlienResumePairStageUpdate {
        texture,
        relationship,
    })
}

/// Steer the current node back to the queue anchor and restart its partner.
pub fn update_resume_final_stage(
    species: AlienSpecies,
    state: &mut AlienResumeMethodState,
    current: &mut AlienNodePose,
    anchor: &AlienNodePose,
    paired_callback: &mut AlienRingCallback,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> Result<AlienResumePairUpdate, AlienResumeFinalStageError> {
    state
        .paired_node
        .ok_or(AlienResumeFinalStageError::MissingPairedNode)?;
    current.radial_offset = FINAL_RETURN_RADIAL_OFFSET;
    let relationship = update_resume_pair_steering(species, current, anchor, trigonometry);
    if relationship == AlienResumePairUpdate::Inside {
        state.callback = Some(AlienResumeCallback::Begin);
        *paired_callback = AlienRingCallback::RestartInitialCourse;
        current.radial_offset = i16::default();
    }
    Ok(relationship)
}

/// Test a resumed node pair and steer the current node when they remain apart.
pub fn update_resume_pair_steering(
    species: AlienSpecies,
    current: &mut AlienNodePose,
    other: &AlienNodePose,
    trigonometry: &[AlienTrigonometryPair; TRIGONOMETRY_ENTRY_COUNT],
) -> AlienResumePairUpdate {
    let depth_delta = i32::from(position_word(other, Z_AXIS))
        .wrapping_sub(i32::from(position_word(current, Z_AXIS)));
    let lateral_delta = i32::from(position_word(other, X_AXIS))
        .wrapping_sub(i32::from(position_word(current, X_AXIS)));
    let vertical_delta = position_word(other, Y_AXIS).wrapping_sub(position_word(current, Y_AXIS));
    let depth_bound = match species {
        AlienSpecies::Amer => AMER_DEPTH_BOUND,
        AlienSpecies::Croolis | AlienSpecies::Scrut => OTHER_DEPTH_BOUND,
    };
    if (-depth_bound..=depth_bound).contains(&depth_delta)
        && (-LATERAL_BOUND..=LATERAL_BOUND).contains(&lateral_delta)
        && (VERTICAL_MINIMUM..VERTICAL_MAXIMUM_EXCLUSIVE).contains(&vertical_delta)
    {
        return AlienResumePairUpdate::Inside;
    }

    let vertical_step = vertical_delta >> VERTICAL_EASING_SHIFT;
    let pitch = (current.angles[PITCH_AXIS] as i16).wrapping_sub(vertical_step);
    current.angles[PITCH_AXIS] = (pitch >> PITCH_EASING_SHIFT) as u16;

    let sample_offset = current.angles[PAN_AXIS] & ANGLE_MASK;
    let sample = trigonometry[usize::from(sample_offset >> ANGLE_INDEX_SHIFT)];
    let direction = i32::from(sample.cosine)
        .wrapping_mul(lateral_delta)
        .wrapping_sub(i32::from(sample.sine).wrapping_mul(depth_delta));
    let pan_step = if direction < i32::default() {
        NEGATIVE_DIRECTION_PAN_STEP
    } else {
        NONNEGATIVE_DIRECTION_PAN_STEP
    };
    current.angles[PAN_AXIS] = sample_offset.wrapping_add(pan_step as u16);
    AlienResumePairUpdate::Outside
}

fn position_word(node: &AlienNodePose, axis: usize) -> i16 {
    node.local_position[axis] as i16
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use commander_blood_formats::alien::{AXIS_COUNT, AlienNodeParent, AlienTransformData};
    use serde::Deserialize;

    use super::*;

    const PRESERVED_RESUMED_NODE: usize = 37;
    const STATE_MODEL: usize = 41;

    fn state_node(node_index: usize) -> AlienSceneNode {
        AlienSceneNode {
            model_index: STATE_MODEL,
            node_index,
        }
    }

    #[derive(Deserialize)]
    struct ResumeVector {
        name: String,
        module: String,
        resume_before: u16,
        resume_after: u16,
        resume_step_before: u16,
        resume_step_after: u16,
        resume_value_before: u16,
        resume_value_after: u16,
        tail_dispatched: bool,
    }

    #[derive(Deserialize)]
    struct ResumePairVector {
        name: String,
        module: String,
        outside: bool,
        current_position: [u32; AXIS_COUNT],
        other_position: [u32; AXIS_COUNT],
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        cosine: u16,
        sine: u16,
        current_position_after: [u32; AXIS_COUNT],
        other_position_after: [u32; AXIS_COUNT],
    }

    #[derive(Deserialize)]
    struct ResumeTextureVector {
        name: String,
        module: String,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        targets: Vec<ResumeTextureTargetVector>,
    }

    #[derive(Deserialize)]
    struct ResumeTextureTargetVector {
        vertex: usize,
        direction: i16,
        before: u16,
        after: u16,
    }

    #[derive(Deserialize)]
    struct ResumeTimeoutVector {
        name: String,
        module: String,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        targets: Vec<ResumeTextureTargetVector>,
        countdown_before: u16,
        countdown_after: u16,
        final_selected: bool,
    }

    #[derive(Deserialize)]
    struct ResumePairStageVector {
        name: String,
        module: String,
        inside: bool,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        targets: Vec<ResumeTextureTargetVector>,
        current_position_before: [u32; AXIS_COUNT],
        current_position_after: [u32; AXIS_COUNT],
        other_position_before: [u32; AXIS_COUNT],
        other_position_after: [u32; AXIS_COUNT],
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        current_radial_before: u16,
        current_radial_after_average: u16,
        current_radial_after: u16,
        other_radial_before: u16,
        other_radial_after: u16,
        countdown_before: u16,
        countdown_after: u16,
    }

    #[derive(Deserialize)]
    struct ResumeFinalStageVector {
        name: String,
        module: String,
        inside: bool,
        current_position_before: [u32; AXIS_COUNT],
        current_position_after: [u32; AXIS_COUNT],
        anchor_position_before: [u32; AXIS_COUNT],
        anchor_position_after: [u32; AXIS_COUNT],
        pitch_before: u16,
        pitch_after: u16,
        pan_before: u16,
        pan_after: u16,
        radial_before: u16,
        radial_after: u16,
    }

    #[derive(Deserialize)]
    struct ResumeQueueVector {
        name: String,
        module: String,
        occupied: bool,
        pair_relationship: Option<String>,
        cursor_before: u16,
        cursor_after: u16,
        selected_slot: usize,
        primary_before: u16,
        secondary_before: u16,
        component: String,
        required_vertex_count: usize,
        phase_before: u16,
        phase_after: u16,
        signed_delta: i16,
        texture_targets: Vec<ResumeTextureTargetVector>,
        current_position_after: [u32; AXIS_COUNT],
        other_position_after: [u32; AXIS_COUNT],
        current_pitch_after: u16,
        current_pan_after: u16,
        current_radial_after: u16,
        other_radial_after: u16,
        countdown_before: u16,
        countdown_after: u16,
    }

    #[derive(Default)]
    struct CallbackRecorder {
        calls: Vec<(AlienSpecies, AlienResumeCallback)>,
    }

    impl AlienResumeCallbacks for CallbackRecorder {
        type Error = Infallible;

        fn invoke(
            &mut self,
            species: AlienSpecies,
            callback: AlienResumeCallback,
            _state: &mut AlienResumeMethodState,
        ) -> Result<(), Self::Error> {
            self.calls.push((species, callback));
            Ok(())
        }
    }

    fn species(module: &str) -> AlienSpecies {
        match module {
            "amer" => AlienSpecies::Amer,
            "croolis" => AlienSpecies::Croolis,
            "scrut" => AlienSpecies::Scrut,
            _ => panic!("unknown alien module {module}"),
        }
    }

    fn fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1bea_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b46_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1bfb_natural.json"),
        ]
    }

    fn pair_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1cfa_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1c46_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1d06_natural.json"),
        ]
    }

    fn texture_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1c03_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b5f_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1c14_natural.json"),
        ]
    }

    fn timeout_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1cbf_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1c0b_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1ccb_natural.json"),
        ]
    }

    fn pair_stage_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1c7d_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1bc9_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1c89_natural.json"),
        ]
    }

    fn final_stage_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1ccf_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1c1b_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1cdb_natural.json"),
        ]
    }

    fn queue_fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1c34_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1b85_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_1c45_natural.json"),
        ]
    }

    fn node(position: [u32; AXIS_COUNT], pitch: u16, pan: u16) -> AlienNodePose {
        AlienNodePose {
            parent: AlienNodeParent::Root,
            scene_parent: None,
            first_vertex: usize::default(),
            vertex_count: 1,
            transform: AlienTransformData::default(),
            local_position: position.map(|value| value as i32),
            angles: [pitch, pan, u16::default()],
            radial_offset: i16::default(),
        }
    }

    #[test]
    fn initialization_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().take(3) {
                assert_eq!(vector.resume_before, u16::MIN);
                assert!(!vector.tail_dispatched);
                let mut state = AlienResumeMethodState {
                    callback: None,
                    phase: vector.resume_step_before,
                    paired_node: Some(state_node(usize::from(vector.resume_value_before))),
                    resumed_node: Some(state_node(PRESERVED_RESUMED_NODE)),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::Initialized,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_ne!(vector.resume_after, u16::MIN);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(state.paired_node, None);
                assert_eq!(vector.resume_value_after, u16::MIN);
                assert_eq!(state.resumed_node, Some(state_node(PRESERVED_RESUMED_NODE)));
                assert!(callbacks.calls.is_empty());
            }
        }
    }

    #[test]
    fn dispatch_matches_every_original_overlay_vector() {
        for fixture in fixtures() {
            let vectors: Vec<ResumeVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().skip(3) {
                assert_ne!(vector.resume_before, u16::MIN);
                assert!(vector.tail_dispatched);
                let paired_node = Some(state_node(usize::from(vector.resume_value_before)));
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Begin),
                    phase: vector.resume_step_before,
                    paired_node,
                    resumed_node: Some(state_node(PRESERVED_RESUMED_NODE)),
                };
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    initialize_or_dispatch_resume(
                        species(&vector.module),
                        &mut state,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienResumeUpdate::CallbackInvoked,
                    "{}",
                    vector.name
                );
                assert_eq!(state.callback, Some(AlienResumeCallback::Begin));
                assert_eq!(vector.resume_after, vector.resume_before);
                assert_eq!(state.phase, vector.resume_step_after);
                assert_eq!(vector.resume_step_after, vector.resume_step_before);
                assert_eq!(state.paired_node, paired_node);
                assert_eq!(vector.resume_value_after, vector.resume_value_before);
                assert_eq!(state.resumed_node, Some(state_node(PRESERVED_RESUMED_NODE)));
                assert_eq!(
                    callbacks.calls,
                    vec![(species(&vector.module), AlienResumeCallback::Begin)]
                );
            }
        }
    }

    #[test]
    fn pair_steering_matches_every_original_overlay_vector() {
        for fixture in pair_fixtures() {
            let vectors: Vec<ResumePairVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let species = species(&vector.module);
                let mut current = node(
                    vector.current_position,
                    vector.pitch_before,
                    vector.pan_before,
                );
                let other = node(vector.other_position, u16::default(), u16::default());
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from((vector.pan_before & ANGLE_MASK) >> ANGLE_INDEX_SHIFT);
                trigonometry[sample_index] = AlienTrigonometryPair {
                    cosine: vector.cosine as i16,
                    sine: vector.sine as i16,
                };

                assert_eq!(
                    update_resume_pair_steering(species, &mut current, &other, &trigonometry,),
                    if vector.outside {
                        AlienResumePairUpdate::Outside
                    } else {
                        AlienResumePairUpdate::Inside
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PITCH_AXIS], vector.pitch_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PAN_AXIS], vector.pan_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.local_position.map(|value| value as u32),
                    vector.current_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.local_position.map(|value| value as u32),
                    vector.other_position_after,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn texture_motion_matches_every_original_overlay_vector() {
        for fixture in texture_fixtures() {
            let vectors: Vec<ResumeTextureVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut texture_coordinates =
                    vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.targets {
                    texture_coordinates[target.vertex][component] = target.before as i16;
                }
                let mut expected = texture_coordinates.clone();
                for target in &vector.targets {
                    expected[target.vertex][component] = target.after as i16;
                    assert!(matches!(target.direction, -1 | 1));
                }
                let preserved_callback = Some(AlienResumeCallback::Begin);
                let preserved_paired_node = Some(state_node(17));
                let preserved_resumed_node = Some(state_node(29));
                let mut state = AlienResumeMethodState {
                    callback: preserved_callback,
                    phase: vector.phase_before,
                    paired_node: preserved_paired_node,
                    resumed_node: preserved_resumed_node,
                };

                assert_eq!(
                    update_resume_texture_motion(
                        species(&vector.module),
                        &mut state,
                        &mut texture_coordinates,
                    ),
                    Ok(AlienResumeTextureUpdate {
                        delta: vector.signed_delta,
                        phase: vector.phase_after,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(texture_coordinates, expected, "{}", vector.name);
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(state.callback, preserved_callback, "{}", vector.name);
                assert_eq!(state.paired_node, preserved_paired_node, "{}", vector.name);
                assert_eq!(
                    state.resumed_node, preserved_resumed_node,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn texture_motion_rejects_truncated_model_data_without_mutation() {
        for species in [
            AlienSpecies::Amer,
            AlienSpecies::Croolis,
            AlienSpecies::Scrut,
        ] {
            let required = match species {
                AlienSpecies::Amer => AMER_RESUME_TEXTURE_VERTEX_COUNT,
                AlienSpecies::Croolis => CROOLIS_RESUME_TEXTURE_VERTEX_COUNT,
                AlienSpecies::Scrut => SCRUT_RESUME_TEXTURE_VERTEX_COUNT,
            };
            let mut state = AlienResumeMethodState {
                phase: 0x0102,
                ..AlienResumeMethodState::default()
            };
            let original_state = state;
            let mut coordinates = vec![[123, 456]; required - 1];
            let original_coordinates = coordinates.clone();

            assert_eq!(
                update_resume_texture_motion(species, &mut state, &mut coordinates),
                Err(AlienResumeTextureError {
                    required,
                    available: required - 1,
                })
            );
            assert_eq!(state, original_state);
            assert_eq!(coordinates, original_coordinates);
        }
    }

    #[test]
    fn timeout_continuation_matches_every_original_overlay_vector() {
        for fixture in timeout_fixtures() {
            let vectors: Vec<ResumeTimeoutVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut texture_coordinates =
                    vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.targets {
                    texture_coordinates[target.vertex][component] = target.before as i16;
                }
                let mut expected = texture_coordinates.clone();
                for target in &vector.targets {
                    expected[target.vertex][component] = target.after as i16;
                }
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Timeout),
                    phase: vector.phase_before,
                    paired_node: Some(state_node(17)),
                    resumed_node: Some(state_node(29)),
                };
                let mut countdown = vector.countdown_before;

                assert_eq!(
                    update_resume_timeout(
                        species(&vector.module),
                        &mut state,
                        &mut texture_coordinates,
                        &mut countdown,
                    ),
                    Ok(AlienResumeTimeoutUpdate {
                        texture: AlienResumeTextureUpdate {
                            delta: vector.signed_delta,
                            phase: vector.phase_after,
                        },
                        countdown: vector.countdown_after,
                        final_stage_selected: vector.final_selected,
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(texture_coordinates, expected, "{}", vector.name);
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(countdown, vector.countdown_after, "{}", vector.name);
                assert_eq!(
                    state.callback,
                    Some(if vector.final_selected {
                        AlienResumeCallback::Final
                    } else {
                        AlienResumeCallback::Timeout
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(state.paired_node, Some(state_node(17)), "{}", vector.name);
                assert_eq!(state.resumed_node, Some(state_node(29)), "{}", vector.name);
            }
        }
    }

    #[test]
    fn paired_node_continuation_matches_every_original_overlay_vector() {
        const PAIRED_NODE: usize = 17;
        const PRESERVED_RESUMED_NODE: usize = 29;

        for fixture in pair_stage_fixtures() {
            let vectors: Vec<ResumePairStageVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let species = species(&vector.module);
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut texture_coordinates =
                    vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.targets {
                    texture_coordinates[target.vertex][component] = target.before as i16;
                }
                let mut expected_textures = texture_coordinates.clone();
                for target in &vector.targets {
                    expected_textures[target.vertex][component] = target.after as i16;
                }
                let mut current = node(
                    vector.current_position_before,
                    vector.pitch_before,
                    vector.pan_before,
                );
                current.radial_offset = vector.current_radial_before as i16;
                let mut other = node(vector.other_position_before, u16::default(), u16::default());
                other.radial_offset = vector.other_radial_before as i16;
                let mut other_callback = AlienRingCallback::FollowCourse;
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from((vector.pan_before & ANGLE_MASK) >> ANGLE_INDEX_SHIFT);
                trigonometry[sample_index] = AlienTrigonometryPair {
                    cosine: 20_000,
                    sine: -8_000,
                };
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Pair),
                    phase: vector.phase_before,
                    paired_node: Some(state_node(PAIRED_NODE)),
                    resumed_node: Some(state_node(PRESERVED_RESUMED_NODE)),
                };
                let mut countdown = vector.countdown_before;

                assert_eq!(
                    update_resume_pair_stage(
                        species,
                        &mut state,
                        &mut current,
                        &mut other,
                        &mut other_callback,
                        &mut texture_coordinates,
                        &trigonometry,
                        &mut countdown,
                    ),
                    Ok(AlienResumePairStageUpdate {
                        texture: AlienResumeTextureUpdate {
                            delta: vector.signed_delta,
                            phase: vector.phase_after,
                        },
                        relationship: if vector.inside {
                            AlienResumePairUpdate::Inside
                        } else {
                            AlienResumePairUpdate::Outside
                        },
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(texture_coordinates, expected_textures, "{}", vector.name);
                assert_eq!(
                    current.local_position.map(|value| value as u32),
                    vector.current_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.local_position.map(|value| value as u32),
                    vector.other_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PITCH_AXIS], vector.pitch_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PAN_AXIS], vector.pan_after,
                    "{}",
                    vector.name
                );
                if !vector.inside {
                    assert_eq!(
                        current.radial_offset as u16, vector.current_radial_after_average,
                        "{}",
                        vector.name
                    );
                }
                assert_eq!(
                    current.radial_offset as u16, vector.current_radial_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.radial_offset as u16, vector.other_radial_after,
                    "{}",
                    vector.name
                );
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(
                    state.callback,
                    Some(if vector.inside {
                        AlienResumeCallback::Timeout
                    } else {
                        AlienResumeCallback::Pair
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other_callback,
                    if vector.inside {
                        AlienRingCallback::BeginResumeClear
                    } else {
                        AlienRingCallback::FollowCourse
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.resumed_node,
                    Some(if vector.inside {
                        state_node(PAIRED_NODE)
                    } else {
                        state_node(PRESERVED_RESUMED_NODE)
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(countdown, vector.countdown_after, "{}", vector.name);
            }
        }
    }

    #[test]
    fn final_continuation_matches_every_original_overlay_vector() {
        const PAIRED_NODE: usize = 17;
        const PRESERVED_RESUMED_NODE: usize = 29;

        for fixture in final_stage_fixtures() {
            let vectors: Vec<ResumeFinalStageVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let mut current = node(
                    vector.current_position_before,
                    vector.pitch_before,
                    vector.pan_before,
                );
                current.radial_offset = vector.radial_before as i16;
                let anchor = node(
                    vector.anchor_position_before,
                    u16::default(),
                    u16::default(),
                );
                let mut paired_callback = AlienRingCallback::FollowCourse;
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from((vector.pan_before & ANGLE_MASK) >> ANGLE_INDEX_SHIFT);
                trigonometry[sample_index] = AlienTrigonometryPair {
                    cosine: 20_000,
                    sine: -8_000,
                };
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Final),
                    phase: 0x1234,
                    paired_node: Some(state_node(PAIRED_NODE)),
                    resumed_node: Some(state_node(PRESERVED_RESUMED_NODE)),
                };

                assert_eq!(
                    update_resume_final_stage(
                        species(&vector.module),
                        &mut state,
                        &mut current,
                        &anchor,
                        &mut paired_callback,
                        &trigonometry,
                    ),
                    Ok(if vector.inside {
                        AlienResumePairUpdate::Inside
                    } else {
                        AlienResumePairUpdate::Outside
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.local_position.map(|value| value as u32),
                    vector.current_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    anchor.local_position.map(|value| value as u32),
                    vector.anchor_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PITCH_AXIS], vector.pitch_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PAN_AXIS], vector.pan_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.radial_offset as u16, vector.radial_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.callback,
                    Some(if vector.inside {
                        AlienResumeCallback::Begin
                    } else {
                        AlienResumeCallback::Final
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    paired_callback,
                    if vector.inside {
                        AlienRingCallback::RestartInitialCourse
                    } else {
                        AlienRingCallback::FollowCourse
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(state.phase, 0x1234, "{}", vector.name);
                assert_eq!(
                    state.paired_node,
                    Some(state_node(PAIRED_NODE)),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.resumed_node,
                    Some(state_node(PRESERVED_RESUMED_NODE)),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn queue_owner_matches_every_well_formed_original_overlay_vector() {
        const CURRENT_MODEL: usize = 3;
        const PAIRED_MODEL: usize = 11;
        const CURRENT_NODE: usize = 0;
        const PAIRED_NODE: usize = 0;
        const ACTIVE_NODE: usize = 2;
        const PRESERVED_PAIRED_NODE: usize = 17;
        const PRESERVED_RESUMED_NODE: usize = 29;

        for fixture in queue_fixtures() {
            let vectors: Vec<ResumeQueueVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                let species = species(&vector.module);
                assert_eq!(usize::from(vector.cursor_before) / 2, vector.selected_slot);
                let component = match vector.component.as_str() {
                    "u" => TEXTURE_U_COMPONENT,
                    "v" => TEXTURE_V_COMPONENT,
                    value => panic!("unknown texture component {value}"),
                };
                let mut textures = vec![[12_345_i16, -23_456_i16]; vector.required_vertex_count];
                for target in &vector.texture_targets {
                    textures[target.vertex][component] = target.before as i16;
                }
                let mut expected_textures = textures.clone();
                for target in &vector.texture_targets {
                    expected_textures[target.vertex][component] = target.after as i16;
                }

                let mut current = node(
                    [u32::default(); AXIS_COUNT],
                    vector.primary_before,
                    vector.secondary_before,
                );
                current.radial_offset = 40;
                let other_position = if vector.pair_relationship.as_deref() == Some("inside") {
                    [u32::default(); AXIS_COUNT]
                } else {
                    [300, 0, 0]
                };
                let mut other = node(other_position, u16::default(), u16::default());
                other.radial_offset = 20;
                let current_node = AlienSceneNode {
                    model_index: CURRENT_MODEL,
                    node_index: CURRENT_NODE,
                };
                let paired_node = AlienSceneNode {
                    model_index: PAIRED_MODEL,
                    node_index: PAIRED_NODE,
                };
                let active_node = state_node(ACTIVE_NODE);
                let mut paired_callback = AlienRingCallback::FollowCourse;
                let mut scene = AlienCallbackSceneState {
                    transition_queue_read_slot: vector.selected_slot,
                    active_node: Some(active_node),
                    transition_queue: std::array::from_fn(|slot| Some(state_node(100 + slot))),
                    ..AlienCallbackSceneState::default()
                };
                scene.transition_queue[vector.selected_slot] =
                    vector.occupied.then_some(paired_node);
                let mut expected_entries = scene.transition_queue;
                if vector.occupied {
                    expected_entries[vector.selected_slot] = None;
                }
                let mut state = AlienResumeMethodState {
                    callback: Some(AlienResumeCallback::Begin),
                    phase: vector.phase_before,
                    paired_node: Some(state_node(PRESERVED_PAIRED_NODE)),
                    resumed_node: Some(state_node(PRESERVED_RESUMED_NODE)),
                };
                let mut countdown = vector.countdown_before;
                let mut trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
                let sample_index =
                    usize::from((vector.secondary_before & ANGLE_MASK) >> ANGLE_INDEX_SHIFT);
                trigonometry[sample_index] = AlienTrigonometryPair {
                    cosine: 20_000,
                    sine: -8_000,
                };

                let result = update_resume_queue(
                    species,
                    &mut state,
                    &mut scene,
                    AlienResumeQueueContext {
                        current: current_node,
                        current_pose: &mut current,
                        texture_coordinates: &mut textures,
                        paired: vector.occupied.then_some(AlienResumePairContext {
                            node: paired_node,
                            pose: &mut other,
                            callback: &mut paired_callback,
                        }),
                        trigonometry: &trigonometry,
                        countdown: &mut countdown,
                    },
                );
                let expected_result = if vector.occupied {
                    Ok(AlienResumeQueueUpdate::PairDispatched {
                        paired_node,
                        pair: AlienResumePairStageUpdate {
                            texture: AlienResumeTextureUpdate {
                                delta: vector.signed_delta,
                                phase: vector.phase_after,
                            },
                            relationship: match vector.pair_relationship.as_deref() {
                                Some("inside") => AlienResumePairUpdate::Inside,
                                Some("outside") => AlienResumePairUpdate::Outside,
                                value => panic!("unknown pair relationship {value:?}"),
                            },
                        },
                    })
                } else {
                    Ok(AlienResumeQueueUpdate::Idle {
                        read_slot: usize::from(vector.cursor_after) / 2,
                    })
                };
                assert_eq!(result, expected_result, "{}", vector.name);
                assert_eq!(scene.transition_queue, expected_entries, "{}", vector.name);
                assert_eq!(
                    scene.transition_queue_read_slot,
                    usize::from(vector.cursor_after) / 2,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    scene.active_node,
                    if vector.occupied {
                        None
                    } else {
                        Some(active_node)
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.local_position.map(|value| value as u32),
                    vector.current_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.local_position.map(|value| value as u32),
                    vector.other_position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PITCH_AXIS], vector.current_pitch_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.angles[PAN_AXIS], vector.current_pan_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    current.radial_offset as u16, vector.current_radial_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    other.radial_offset as u16, vector.other_radial_after,
                    "{}",
                    vector.name
                );
                assert_eq!(textures, expected_textures, "{}", vector.name);
                let pair_inside = vector.pair_relationship.as_deref() == Some("inside");
                assert_eq!(
                    state.callback,
                    Some(if pair_inside {
                        AlienResumeCallback::Timeout
                    } else if vector.occupied {
                        AlienResumeCallback::Pair
                    } else {
                        AlienResumeCallback::Begin
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(state.phase, vector.phase_after, "{}", vector.name);
                assert_eq!(
                    state.paired_node,
                    Some(if vector.occupied {
                        paired_node
                    } else {
                        state_node(PRESERVED_PAIRED_NODE)
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    state.resumed_node,
                    Some(if pair_inside {
                        paired_node
                    } else {
                        state_node(PRESERVED_RESUMED_NODE)
                    }),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    paired_callback,
                    if pair_inside {
                        AlienRingCallback::BeginResumeClear
                    } else {
                        AlienRingCallback::FollowCourse
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(countdown, vector.countdown_after, "{}", vector.name);
            }
        }
    }

    #[test]
    fn queue_owner_rejects_invalid_typed_state_without_mutation() {
        let current_node = state_node(0);
        let other_node = state_node(1);
        let species = AlienSpecies::Amer;
        let mut current = node([0; AXIS_COUNT], 0, 0);
        let original_current = current.clone();
        let mut other = node([0; AXIS_COUNT], 0, 0);
        let original_other = other.clone();
        let mut textures = vec![[0; TEXTURE_COMPONENT_COUNT]; AMER_RESUME_TEXTURE_VERTEX_COUNT];
        let original_textures = textures.clone();
        let mut paired_callback = AlienRingCallback::FollowCourse;
        let mut state = AlienResumeMethodState::default();
        let original_state = state;
        let mut countdown = 7;
        let trigonometry = [AlienTrigonometryPair::default(); TRIGONOMETRY_ENTRY_COUNT];
        let mut scene = AlienCallbackSceneState {
            transition_queue_read_slot: ALIEN_RESUME_QUEUE_CAPACITY,
            active_node: Some(current_node),
            ..AlienCallbackSceneState::default()
        };
        let original_scene = scene;

        let result = update_resume_queue(
            species,
            &mut state,
            &mut scene,
            AlienResumeQueueContext {
                current: current_node,
                current_pose: &mut current,
                texture_coordinates: &mut textures,
                paired: None,
                trigonometry: &trigonometry,
                countdown: &mut countdown,
            },
        );

        assert_eq!(
            result,
            Err(AlienResumeQueueError::InvalidReadSlot {
                index: ALIEN_RESUME_QUEUE_CAPACITY,
            })
        );
        assert_eq!(scene, original_scene);
        assert_eq!(current, original_current);
        assert_eq!(other, original_other);
        assert_eq!(textures, original_textures);
        assert_eq!(state, original_state);
        assert_eq!(countdown, 7);

        scene.transition_queue_read_slot = 0;
        scene.transition_queue[0] = Some(current_node);
        let original_scene = scene;
        let result = update_resume_queue(
            species,
            &mut state,
            &mut scene,
            AlienResumeQueueContext {
                current: current_node,
                current_pose: &mut current,
                texture_coordinates: &mut textures,
                paired: Some(AlienResumePairContext {
                    node: current_node,
                    pose: &mut other,
                    callback: &mut paired_callback,
                }),
                trigonometry: &trigonometry,
                countdown: &mut countdown,
            },
        );

        assert_eq!(
            result,
            Err(AlienResumeQueueError::AliasedPair { node: current_node })
        );
        assert_eq!(scene, original_scene);
        assert_eq!(current, original_current);
        assert_eq!(other, original_other);
        assert_eq!(textures, original_textures);
        assert_eq!(state, original_state);
        assert_eq!(countdown, 7);

        scene.transition_queue[0] = Some(other_node);
        let original_scene = scene;
        let result = update_resume_queue(
            species,
            &mut state,
            &mut scene,
            AlienResumeQueueContext {
                current: current_node,
                current_pose: &mut current,
                texture_coordinates: &mut textures,
                paired: None,
                trigonometry: &trigonometry,
                countdown: &mut countdown,
            },
        );
        assert_eq!(
            result,
            Err(AlienResumeQueueError::MissingPairContext { queued: other_node })
        );
        assert_eq!(scene, original_scene);
        assert_eq!(current, original_current);
        assert_eq!(other, original_other);
        assert_eq!(textures, original_textures);
        assert_eq!(state, original_state);
        assert_eq!(countdown, 7);

        scene.transition_queue[0] = None;
        let original_scene = scene;
        let result = update_resume_queue(
            species,
            &mut state,
            &mut scene,
            AlienResumeQueueContext {
                current: current_node,
                current_pose: &mut current,
                texture_coordinates: &mut textures,
                paired: Some(AlienResumePairContext {
                    node: other_node,
                    pose: &mut other,
                    callback: &mut paired_callback,
                }),
                trigonometry: &trigonometry,
                countdown: &mut countdown,
            },
        );
        assert_eq!(result, Err(AlienResumeQueueError::UnexpectedPairContext));
        assert_eq!(scene, original_scene);
        assert_eq!(current, original_current);
        assert_eq!(other, original_other);
        assert_eq!(textures, original_textures);
        assert_eq!(state, original_state);
        assert_eq!(countdown, 7);

        scene.transition_queue[0] = Some(other_node);
        let supplied_node = state_node(2);
        let original_scene = scene;
        let result = update_resume_queue(
            species,
            &mut state,
            &mut scene,
            AlienResumeQueueContext {
                current: current_node,
                current_pose: &mut current,
                texture_coordinates: &mut textures,
                paired: Some(AlienResumePairContext {
                    node: supplied_node,
                    pose: &mut other,
                    callback: &mut paired_callback,
                }),
                trigonometry: &trigonometry,
                countdown: &mut countdown,
            },
        );
        assert_eq!(
            result,
            Err(AlienResumeQueueError::MismatchedPair {
                queued: other_node,
                supplied: supplied_node,
            })
        );
        assert_eq!(scene, original_scene);
        assert_eq!(current, original_current);
        assert_eq!(other, original_other);
        assert_eq!(textures, original_textures);
        assert_eq!(state, original_state);
        assert_eq!(countdown, 7);
    }
}
