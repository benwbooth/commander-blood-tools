//! Circular motion-history coordination for alien behavior models.

use std::fmt;

use commander_blood_formats::alien::AXIS_COUNT;

use super::{AlienModelPose, AlienSpecies};

const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const FIRST_NODE: usize = 0;
const FIRST_FOLLOWER_NODE: usize = 1;
const SINGLE_NODE_COUNT: usize = 1;
const RING_ENTRY_COUNT: usize = 128;
const INITIAL_TIMER: u16 = 7;
const INITIAL_POSITION: i32 = 1_700;
const INITIAL_COURSE_FRAMES: i16 = 25;
const INITIAL_FEEDBACK_PHASE: u16 = 0;
const FOLLOWING_PHASE_STEP: u16 = 256;
const INITIAL_BEHAVIOR_SEED: u16 = 0xa957;
const INITIAL_RADIAL_OFFSET: i16 = 70;
const ZERO_POSITION_COMPONENT: i32 = 0;
const ZERO_MOTION_COMPONENT: i16 = 0;
const RESTART_RADIAL_OFFSET: i16 = 8;
const RESTART_COURSE_FRAMES: i16 = 30;
const RESUME_COUNTDOWN: u16 = 18;
const RESUME_COMMAND_FLAGS: u16 = 2;
const RANDOM_BORROW_BIT: u16 = 1;
const RANDOM_BORROW_SHIFT: u32 = 2;
const RANDOM_ROTATION: u32 = 3;

/// One motion-history sample consumed by the recovered slot-3 callbacks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienRingEntry {
    /// Pitch increment applied when this sample becomes current.
    pub pitch_step: i16,
    /// Pan increment applied when this sample becomes current.
    pub pan_step: i16,
    /// Radial displacement applied when this sample becomes current.
    pub radial_offset: i16,
    /// Command bits consumed by the concrete callback implementation.
    pub command_flags: u16,
}

/// Concrete behavior callback selected for one model node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlienRingCallback {
    /// Generate the leading node's initial motion course.
    #[default]
    InitialCourse,
    /// Follow the shared history produced by the leading node.
    FollowCourse,
    /// Clear successive history entries while a captured node resumes.
    ClearHistory,
}

/// Per-node state used by the circular motion-history behavior.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienRingNodeState {
    /// Callback selected for this node's current behavior stage.
    pub callback: AlienRingCallback,
    /// Frames remaining in the leading node's generated course.
    pub course_frames_remaining: i16,
    /// Cyclic phase used by the callback's feedback sample.
    pub feedback_phase: u16,
    /// Index of this node's current motion-history entry.
    pub ring_slot: usize,
    /// Per-node deterministic seed or callback-stage marker.
    pub behavior_seed: u16,
}

/// Timer policy selected by the recovered slot-3 coordinator.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum AlienRingLifecycle {
    /// The node and history state have not been initialized.
    #[default]
    Uninitialized,
    /// The shared history timer advances once per coordinator call.
    TimerRunning,
    /// The shared history timer remains unchanged while callbacks run.
    TimerSuspended,
}

/// Owned flat-memory state for one model's circular motion history.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienRingAnimationState {
    /// Current initialization and timer policy.
    pub lifecycle: AlienRingLifecycle,
    /// Countdown shared by every node callback in this model.
    pub timer: u16,
    /// Wrapping generation counter used when constructing follower chains.
    pub generation: u16,
    /// Ring slot reserved for the next initialized model.
    pub next_ring_slot: usize,
    /// Fixed-size motion-history ring.
    pub entries: [AlienRingEntry; RING_ENTRY_COUNT],
    /// Behavior metadata parallel to the model pose's node vector.
    pub nodes: Vec<AlienRingNodeState>,
}

impl AlienRingAnimationState {
    /// Allocate ring behavior state for a typed model hierarchy.
    pub fn new(node_count: usize) -> Self {
        Self {
            lifecycle: AlienRingLifecycle::Uninitialized,
            timer: u16::MIN,
            generation: u16::MIN,
            next_ring_slot: usize::MIN,
            entries: [AlienRingEntry::default(); RING_ENTRY_COUNT],
            nodes: vec![AlienRingNodeState::default(); node_count],
        }
    }
}

/// Scene state published while a ring node is captured for resumption.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienRingResumeState {
    /// Frames remaining before the resume sequence advances.
    pub countdown: u16,
    /// Typed model-node index selected for the resume sequence.
    pub selected_node: Option<usize>,
}

/// Indirect callback boundary retained by the recovered coordinator.
///
/// Implementations may update the pose and animation state, but must preserve
/// the lengths of their node arrays while one coordinator pass is active.
pub trait AlienRingCallbacks {
    /// Invoke one node's currently selected callback.
    fn invoke(
        &mut self,
        species: AlienSpecies,
        callback: AlienRingCallback,
        node_index: usize,
        pose: &mut AlienModelPose,
        animation: &mut AlienRingAnimationState,
    ) -> Result<(), AlienRingError>;
}

/// Stage completed by one invocation of the ring coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRingUpdate {
    /// One-time node and history state was initialized.
    Initialized,
    /// Every node's selected callback was invoked in hierarchy order.
    CallbacksInvoked {
        /// Number of callbacks invoked.
        count: usize,
    },
}

/// Result of one history-clearing callback.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRingClearUpdate {
    /// The shared timer has not yet reached zero.
    Waiting,
    /// The node advanced and cleared the reported history slot.
    Cleared {
        /// Slot reset by this callback.
        slot: usize,
    },
}

/// Invalid typed state supplied to the recovered ring coordinator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienRingError {
    /// The original zero-node path walked the entire 16-bit address space.
    EmptyNodeList,
    /// Pose nodes and parallel behavior metadata must have identical lengths.
    NodeStateCountMismatch {
        /// Nodes available in the mutable model pose.
        pose: usize,
        /// Parallel ring-node states available to the coordinator.
        animation: usize,
    },
    /// A shared ring cursor must name one of the owned ring entries.
    InvalidNextRingSlot {
        /// Invalid slot supplied by the caller.
        slot: usize,
    },
    /// A node's callback state must name one of the owned ring entries.
    InvalidNodeRingSlot {
        /// Node containing the invalid slot.
        node_index: usize,
        /// Invalid slot supplied by the caller.
        slot: usize,
    },
    /// A callback selected a node outside its typed model hierarchy.
    InvalidNodeIndex {
        /// Invalid node supplied by the caller.
        node_index: usize,
        /// Number of nodes available in the hierarchy.
        node_count: usize,
    },
}

impl fmt::Display for AlienRingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "invalid alien ring-animation state: {self:?}")
    }
}

impl std::error::Error for AlienRingError {}

/// Restart one node's generated course using the recovered random transition.
pub fn restart_initial_course(
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
    random_state: &mut u16,
) -> Result<(), AlienRingError> {
    let slot = validate_node_pair(node_index, pose, animation)?;
    animation.entries[slot].command_flags = u16::MIN;
    animation.entries[slot].radial_offset = RESTART_RADIAL_OFFSET;
    animation.nodes[node_index].callback = AlienRingCallback::InitialCourse;
    animation.nodes[node_index].course_frames_remaining = RESTART_COURSE_FRAMES;
    pose.nodes[node_index].angles[Z_AXIS] = u16::MIN;
    pose.nodes[node_index].radial_offset = RESTART_RADIAL_OFFSET;

    let next_random = random_transition(*random_state);
    animation.nodes[node_index].behavior_seed = next_random;
    *random_state = next_random;
    Ok(())
}

/// Reset one node and begin clearing its circular history during resumption.
pub fn begin_resume_clear(
    species: AlienSpecies,
    node_index: usize,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
) -> Result<(), AlienRingError> {
    let slot = validate_node_pair(node_index, pose, animation)?;
    reset_pose_node(&mut pose.nodes[node_index], initial_position(species));
    animation.nodes[node_index].callback = AlienRingCallback::ClearHistory;
    animation.entries[slot] = AlienRingEntry {
        command_flags: RESUME_COMMAND_FLAGS,
        ..AlienRingEntry::default()
    };
    Ok(())
}

/// Capture one node for the resume sequence and reset its transform state.
pub fn capture_resume_state(
    species: AlienSpecies,
    node_index: usize,
    pose: &mut AlienModelPose,
    resume: &mut AlienRingResumeState,
) -> Result<(), AlienRingError> {
    let node_count = pose.nodes.len();
    let node = pose
        .nodes
        .get_mut(node_index)
        .ok_or(AlienRingError::InvalidNodeIndex {
            node_index,
            node_count,
        })?;
    resume.countdown = RESUME_COUNTDOWN;
    resume.selected_node = Some(node_index);
    reset_pose_node(node, initial_position(species));
    Ok(())
}

/// Advance and clear one node's history entry when the shared timer expires.
pub fn clear_next_ring_entry(
    node_index: usize,
    animation: &mut AlienRingAnimationState,
) -> Result<AlienRingClearUpdate, AlienRingError> {
    let node_count = animation.nodes.len();
    let node = animation
        .nodes
        .get_mut(node_index)
        .ok_or(AlienRingError::InvalidNodeIndex {
            node_index,
            node_count,
        })?;
    if node.ring_slot >= RING_ENTRY_COUNT {
        return Err(AlienRingError::InvalidNodeRingSlot {
            node_index,
            slot: node.ring_slot,
        });
    }
    if animation.timer != u16::MIN {
        return Ok(AlienRingClearUpdate::Waiting);
    }

    node.ring_slot = next_slot(node.ring_slot);
    let slot = node.ring_slot;
    animation.entries[slot] = AlienRingEntry::default();
    Ok(AlienRingClearUpdate::Cleared { slot })
}

/// Initialize or advance the recovered slot-3 motion-history coordinator.
pub fn update_or_initialize_ring(
    species: AlienSpecies,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
    callbacks: &mut impl AlienRingCallbacks,
) -> Result<AlienRingUpdate, AlienRingError> {
    validate_state(pose, animation)?;
    if animation.lifecycle == AlienRingLifecycle::Uninitialized {
        initialize(species, pose, animation);
        return Ok(AlienRingUpdate::Initialized);
    }

    if animation.lifecycle == AlienRingLifecycle::TimerRunning {
        animation.timer = animation.timer.wrapping_sub(1);
        if (animation.timer as i16).is_negative() {
            animation.timer = INITIAL_TIMER;
        }
    }

    let node_count = pose.nodes.len();
    for node_index in usize::MIN..node_count {
        let callback = animation.nodes[node_index].callback;
        callbacks.invoke(species, callback, node_index, pose, animation)?;
        if pose.nodes.len() != node_count || animation.nodes.len() != node_count {
            return Err(AlienRingError::NodeStateCountMismatch {
                pose: pose.nodes.len(),
                animation: animation.nodes.len(),
            });
        }
    }
    Ok(AlienRingUpdate::CallbacksInvoked { count: node_count })
}

fn validate_state(
    pose: &AlienModelPose,
    animation: &AlienRingAnimationState,
) -> Result<(), AlienRingError> {
    if pose.nodes.is_empty() {
        return Err(AlienRingError::EmptyNodeList);
    }
    if pose.nodes.len() != animation.nodes.len() {
        return Err(AlienRingError::NodeStateCountMismatch {
            pose: pose.nodes.len(),
            animation: animation.nodes.len(),
        });
    }
    if animation.next_ring_slot >= RING_ENTRY_COUNT {
        return Err(AlienRingError::InvalidNextRingSlot {
            slot: animation.next_ring_slot,
        });
    }
    if animation.lifecycle != AlienRingLifecycle::Uninitialized {
        for (node_index, node) in animation.nodes.iter().enumerate() {
            if node.ring_slot >= RING_ENTRY_COUNT {
                return Err(AlienRingError::InvalidNodeRingSlot {
                    node_index,
                    slot: node.ring_slot,
                });
            }
        }
    }
    Ok(())
}

fn initialize(
    species: AlienSpecies,
    pose: &mut AlienModelPose,
    animation: &mut AlienRingAnimationState,
) {
    animation.lifecycle = AlienRingLifecycle::TimerRunning;
    animation.timer = INITIAL_TIMER;
    let initial_position = initial_position(species);
    let mut current_slot = animation.next_ring_slot;

    reset_pose_node(&mut pose.nodes[FIRST_NODE], initial_position);
    animation.nodes[FIRST_NODE] = AlienRingNodeState {
        callback: AlienRingCallback::InitialCourse,
        course_frames_remaining: INITIAL_COURSE_FRAMES,
        feedback_phase: INITIAL_FEEDBACK_PHASE,
        ring_slot: current_slot,
        behavior_seed: INITIAL_BEHAVIOR_SEED,
    };
    animation.entries[current_slot] = AlienRingEntry {
        pitch_step: ZERO_MOTION_COMPONENT,
        pan_step: ZERO_MOTION_COMPONENT,
        radial_offset: INITIAL_RADIAL_OFFSET,
        command_flags: u16::MIN,
    };

    if pose.nodes.len() == SINGLE_NODE_COUNT {
        animation.next_ring_slot = previous_slot(current_slot);
        return;
    }

    current_slot = previous_slot(current_slot);
    animation.generation = animation.generation.wrapping_add(1);
    if animation.generation != u16::MIN {
        animation.lifecycle = AlienRingLifecycle::TimerSuspended;
        animation.nodes[FIRST_NODE].callback = AlienRingCallback::FollowCourse;
        animation.entries[current_slot].radial_offset = ZERO_MOTION_COMPONENT;
        pose.nodes[FIRST_NODE].angles.fill(u16::MIN);
        pose.nodes[FIRST_NODE].local_position = initial_position;
    }

    let mut phase = u16::MIN;
    for node_index in FIRST_FOLLOWER_NODE..pose.nodes.len() {
        current_slot = previous_slot(current_slot);
        phase = phase.wrapping_add(FOLLOWING_PHASE_STEP);
        let course_frames_remaining = animation.nodes[node_index].course_frames_remaining;
        animation.nodes[node_index] = AlienRingNodeState {
            callback: AlienRingCallback::FollowCourse,
            course_frames_remaining,
            feedback_phase: phase,
            ring_slot: current_slot,
            behavior_seed: u16::MIN,
        };
        animation.entries[current_slot] = AlienRingEntry::default();
        reset_pose_node(&mut pose.nodes[node_index], initial_position);
    }
    animation.next_ring_slot = previous_slot(current_slot);
}

fn reset_pose_node(node: &mut super::AlienNodePose, position: [i32; AXIS_COUNT]) {
    node.local_position[X_AXIS] = position[X_AXIS];
    node.local_position[Y_AXIS] = position[Y_AXIS];
    node.local_position[Z_AXIS] = position[Z_AXIS];
    node.angles.fill(u16::MIN);
    node.radial_offset = ZERO_MOTION_COMPONENT;
}

fn initial_position(species: AlienSpecies) -> [i32; AXIS_COUNT] {
    match species {
        AlienSpecies::Amer | AlienSpecies::Croolis => [
            ZERO_POSITION_COMPONENT,
            INITIAL_POSITION,
            ZERO_POSITION_COMPONENT,
        ],
        AlienSpecies::Scrut => [
            INITIAL_POSITION,
            ZERO_POSITION_COMPONENT,
            ZERO_POSITION_COMPONENT,
        ],
    }
}

fn validate_node_pair(
    node_index: usize,
    pose: &AlienModelPose,
    animation: &AlienRingAnimationState,
) -> Result<usize, AlienRingError> {
    if pose.nodes.len() != animation.nodes.len() {
        return Err(AlienRingError::NodeStateCountMismatch {
            pose: pose.nodes.len(),
            animation: animation.nodes.len(),
        });
    }
    if node_index >= pose.nodes.len() {
        return Err(AlienRingError::InvalidNodeIndex {
            node_index,
            node_count: pose.nodes.len(),
        });
    }
    let slot = animation.nodes[node_index].ring_slot;
    if slot >= RING_ENTRY_COUNT {
        return Err(AlienRingError::InvalidNodeRingSlot { node_index, slot });
    }
    Ok(slot)
}

fn random_transition(value: u16) -> u16 {
    value
        .rotate_right(RANDOM_ROTATION)
        .wrapping_sub((value >> RANDOM_BORROW_SHIFT) & RANDOM_BORROW_BIT)
}

fn previous_slot(slot: usize) -> usize {
    if slot == usize::MIN {
        RING_ENTRY_COUNT - 1
    } else {
        slot - 1
    }
}

fn next_slot(slot: usize) -> usize {
    if slot == RING_ENTRY_COUNT - 1 {
        usize::MIN
    } else {
        slot + 1
    }
}

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienNodeParent, AlienTransformData};
    use serde::Deserialize;

    use super::*;
    use crate::native::alien::{AlienNodePose, AlienProjectedVertex};

    const INITIAL_RING_CURSORS: [u16; 4] = [0x0180, 0x02a0, 0, 0];
    const INITIAL_GENERATIONS: [u16; 4] = [0x1234, 0, u16::MAX, 1];
    const INITIAL_NODE_COUNTS: [usize; 4] = [1, 3, 2, 2];
    const ORIGINAL_RING_ENTRY_BYTES: usize = 8;
    const PRESERVED_COURSE_FRAMES: i16 = 321;
    const CALLBACK_POSITION: [i32; AXIS_COUNT] = [0x1122_3344, 0x5566_7788, 0x99aa_bbcc_u32 as i32];
    const CALLBACK_ANGLES: [u16; AXIS_COUNT] = [0x1111, 0x2222, 0x3333];
    const CALLBACK_RADIAL_OFFSET: i16 = 0x4444;
    const CALLBACK_COURSE_FRAMES: i16 = 0x5555;
    const CALLBACK_BEHAVIOR_SEED: u16 = 0x6666;
    const CALLBACK_FEEDBACK_PHASE: u16 = 0x7777;
    const RESTART_RANDOM_INPUTS: [u16; 4] = [0, 0x1234, u16::MAX, 4];

    #[derive(Deserialize)]
    struct RingVector {
        name: String,
        module: String,
        path: String,
        state_count: usize,
        generation_after: Option<u16>,
        context_state_after: Option<u16>,
        ring_cursor_after: Option<u16>,
        method_state: Option<u16>,
        timer_before: Option<u16>,
        timer_after: Option<u16>,
        effective_callbacks: Option<usize>,
    }

    #[derive(Deserialize)]
    struct CallbackVector {
        name: String,
        module: String,
        kind: String,
        ring_before: u16,
        ring_after: u16,
        position_after: [u32; AXIS_COUNT],
        motion_after: [u16; 6],
        resume_countdown_after: u16,
        resume_state_after: u16,
    }

    #[derive(Default)]
    struct CallbackRecorder {
        calls: Vec<(AlienRingCallback, usize)>,
    }

    impl AlienRingCallbacks for CallbackRecorder {
        fn invoke(
            &mut self,
            _species: AlienSpecies,
            callback: AlienRingCallback,
            node_index: usize,
            _pose: &mut AlienModelPose,
            _animation: &mut AlienRingAnimationState,
        ) -> Result<(), AlienRingError> {
            self.calls.push((callback, node_index));
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

    fn pose(node_count: usize) -> AlienModelPose {
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: (usize::MIN..node_count)
                .map(|node_index| AlienNodePose {
                    parent: AlienNodeParent::Root,
                    first_vertex: node_index,
                    vertex_count: 1,
                    transform: AlienTransformData::default(),
                    local_position: [node_index as i32 + 11; AXIS_COUNT],
                    angles: [node_index as u16 + 21; AXIS_COUNT],
                    radial_offset: node_index as i16 + 31,
                })
                .collect(),
            projected_vertices: vec![AlienProjectedVertex::default(); node_count],
            texture_coordinates: vec![[i16::MIN; 2]; node_count],
            object_positions: vec![[i16::MIN; AXIS_COUNT]; node_count],
            authored_vertex_count: node_count,
            faces: Vec::<AlienFaceData>::new(),
            last_rotation_matrix: Default::default(),
            last_common_clip: u16::MIN,
        }
    }

    fn seeded_animation(node_count: usize) -> AlienRingAnimationState {
        let mut animation = AlienRingAnimationState::new(node_count);
        for (slot, entry) in animation.entries.iter_mut().enumerate() {
            *entry = AlienRingEntry {
                pitch_step: slot as i16 + 101,
                pan_step: slot as i16 + 201,
                radial_offset: slot as i16 + 301,
                command_flags: slot as u16 + 401,
            };
        }
        for node in &mut animation.nodes {
            node.course_frames_remaining = PRESERVED_COURSE_FRAMES;
        }
        animation
    }

    fn fixtures() -> [&'static str; 3] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_1286_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_12de_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_12cc_natural.json"),
        ]
    }

    fn callback_fixtures(kind: &str) -> [&'static str; 3] {
        match kind {
            "restart" => [
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_1558_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_15b0_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_scrut_func_159e_natural.json"
                ),
            ],
            "resume" => [
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_158a_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_15e2_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_scrut_func_15d0_natural.json"
                ),
            ],
            "capture" => [
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_15db_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_1633_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_scrut_func_1621_natural.json"
                ),
            ],
            "ring_zero" => [
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_amer_func_1614_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_croolis_func_166c_natural.json"
                ),
                include_str!(
                    "../../../../../re/tools/oracle_vectors/xdb_scrut_func_165a_natural.json"
                ),
            ],
            _ => panic!("unknown callback fixture kind {kind}"),
        }
    }

    fn callback_state(ring_cursor: u16) -> (AlienModelPose, AlienRingAnimationState) {
        let mut pose = pose(SINGLE_NODE_COUNT);
        pose.nodes[FIRST_NODE].local_position = CALLBACK_POSITION;
        pose.nodes[FIRST_NODE].angles = CALLBACK_ANGLES;
        pose.nodes[FIRST_NODE].radial_offset = CALLBACK_RADIAL_OFFSET;
        let mut animation = seeded_animation(SINGLE_NODE_COUNT);
        animation.nodes[FIRST_NODE] = AlienRingNodeState {
            callback: AlienRingCallback::FollowCourse,
            course_frames_remaining: CALLBACK_COURSE_FRAMES,
            feedback_phase: CALLBACK_FEEDBACK_PHASE,
            ring_slot: usize::from(ring_cursor) / ORIGINAL_RING_ENTRY_BYTES,
            behavior_seed: CALLBACK_BEHAVIOR_SEED,
        };
        (pose, animation)
    }

    fn callback_position(pose: &AlienModelPose) -> [u32; AXIS_COUNT] {
        pose.nodes[FIRST_NODE]
            .local_position
            .map(|component| component as u32)
    }

    fn callback_motion(pose: &AlienModelPose, animation: &AlienRingAnimationState) -> [u16; 6] {
        let node = &pose.nodes[FIRST_NODE];
        let behavior = &animation.nodes[FIRST_NODE];
        [
            node.angles[X_AXIS],
            node.angles[Y_AXIS],
            node.angles[Z_AXIS],
            node.radial_offset as u16,
            behavior.course_frames_remaining as u16,
            behavior.behavior_seed,
        ]
    }

    #[test]
    fn initialization_matches_every_well_formed_original_vector() {
        for fixture in fixtures() {
            let vectors: Vec<RingVector> = serde_json::from_str(fixture).unwrap();
            for (case_index, vector) in vectors.into_iter().take(4).enumerate() {
                assert_eq!(vector.path, "initialize");
                assert_eq!(vector.state_count, INITIAL_NODE_COUNTS[case_index]);
                let node_count = vector.state_count;
                let initial_slot =
                    usize::from(INITIAL_RING_CURSORS[case_index]) / ORIGINAL_RING_ENTRY_BYTES;
                let initial_generation = INITIAL_GENERATIONS[case_index];
                let mut pose = pose(node_count);
                let mut animation = seeded_animation(node_count);
                animation.next_ring_slot = initial_slot;
                animation.generation = initial_generation;
                let entries_before = animation.entries;
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    update_or_initialize_ring(
                        species(&vector.module),
                        &mut pose,
                        &mut animation,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienRingUpdate::Initialized,
                    "{}",
                    vector.name
                );
                assert!(callbacks.calls.is_empty(), "{}", vector.name);
                assert_eq!(animation.timer, INITIAL_TIMER, "{}", vector.name);
                assert_eq!(
                    animation.generation,
                    vector.generation_after.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.lifecycle,
                    match vector.context_state_after.unwrap() {
                        1 => AlienRingLifecycle::TimerRunning,
                        u16::MAX => AlienRingLifecycle::TimerSuspended,
                        value => panic!("unexpected original lifecycle {value}"),
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.next_ring_slot * ORIGINAL_RING_ENTRY_BYTES,
                    usize::from(vector.ring_cursor_after.unwrap()),
                    "{}",
                    vector.name
                );

                let initial_position = match species(&vector.module) {
                    AlienSpecies::Amer | AlienSpecies::Croolis => [
                        ZERO_POSITION_COMPONENT,
                        INITIAL_POSITION,
                        ZERO_POSITION_COMPONENT,
                    ],
                    AlienSpecies::Scrut => [
                        INITIAL_POSITION,
                        ZERO_POSITION_COMPONENT,
                        ZERO_POSITION_COMPONENT,
                    ],
                };
                for (node_index, (node, behavior)) in
                    pose.nodes.iter().zip(&animation.nodes).enumerate()
                {
                    assert_eq!(node.local_position, initial_position, "{}", vector.name);
                    assert_eq!(node.angles, [u16::MIN; AXIS_COUNT], "{}", vector.name);
                    assert_eq!(node.radial_offset, ZERO_MOTION_COMPONENT, "{}", vector.name);
                    assert_eq!(
                        behavior.feedback_phase,
                        (node_index as u16).wrapping_mul(FOLLOWING_PHASE_STEP),
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        behavior.course_frames_remaining,
                        if node_index == FIRST_NODE {
                            INITIAL_COURSE_FRAMES
                        } else {
                            PRESERVED_COURSE_FRAMES
                        },
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        behavior.behavior_seed,
                        if node_index == FIRST_NODE {
                            INITIAL_BEHAVIOR_SEED
                        } else {
                            u16::MIN
                        },
                        "{}",
                        vector.name
                    );
                }
                assert_eq!(
                    animation.entries[initial_slot],
                    AlienRingEntry {
                        pitch_step: ZERO_MOTION_COMPONENT,
                        pan_step: ZERO_MOTION_COMPONENT,
                        radial_offset: INITIAL_RADIAL_OFFSET,
                        command_flags: u16::MIN,
                    },
                    "{}",
                    vector.name
                );

                if node_count > 1 {
                    let generation_after = vector.generation_after.unwrap();
                    let pre_follower_slot = previous_slot(initial_slot);
                    if generation_after != u16::MIN {
                        let mut expected = entries_before[pre_follower_slot];
                        expected.radial_offset = ZERO_MOTION_COMPONENT;
                        assert_eq!(
                            animation.entries[pre_follower_slot], expected,
                            "{}",
                            vector.name
                        );
                    } else {
                        assert_eq!(
                            animation.entries[pre_follower_slot], entries_before[pre_follower_slot],
                            "{}",
                            vector.name
                        );
                    }
                    for behavior in animation.nodes.iter().skip(1) {
                        assert_eq!(
                            animation.entries[behavior.ring_slot],
                            AlienRingEntry::default(),
                            "{}",
                            vector.name
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn callback_order_and_timer_match_every_well_formed_original_vector() {
        for fixture in fixtures() {
            let vectors: Vec<RingVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors.into_iter().skip(4).take(4) {
                assert_eq!(vector.path, "callbacks");
                let node_count = vector.state_count;
                let mut pose = pose(node_count);
                let mut animation = AlienRingAnimationState::new(node_count);
                animation.lifecycle = match vector.method_state.unwrap() {
                    1 => AlienRingLifecycle::TimerRunning,
                    u16::MAX => AlienRingLifecycle::TimerSuspended,
                    value => panic!("unexpected original lifecycle {value}"),
                };
                animation.timer = vector.timer_before.unwrap();
                for (node_index, node) in animation.nodes.iter_mut().enumerate() {
                    node.callback = if node_index == FIRST_NODE {
                        AlienRingCallback::InitialCourse
                    } else {
                        AlienRingCallback::FollowCourse
                    };
                }
                let mut callbacks = CallbackRecorder::default();

                assert_eq!(
                    update_or_initialize_ring(
                        species(&vector.module),
                        &mut pose,
                        &mut animation,
                        &mut callbacks,
                    )
                    .unwrap(),
                    AlienRingUpdate::CallbacksInvoked { count: node_count },
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.timer,
                    vector.timer_after.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callbacks.calls.len(),
                    vector.effective_callbacks.unwrap(),
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callbacks
                        .calls
                        .iter()
                        .map(|(_, node_index)| *node_index)
                        .collect::<Vec<_>>(),
                    (usize::MIN..node_count).collect::<Vec<_>>(),
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn course_restart_matches_every_original_overlay_vector() {
        for fixture in callback_fixtures("restart") {
            let vectors: Vec<CallbackVector> = serde_json::from_str(fixture).unwrap();
            for (case_index, vector) in vectors.into_iter().enumerate() {
                assert_eq!(vector.kind, "restart");
                let (mut pose, mut animation) = callback_state(vector.ring_before);
                let slot = animation.nodes[FIRST_NODE].ring_slot;
                let entry_before = animation.entries[slot];
                let mut random_state = RESTART_RANDOM_INPUTS[case_index];

                restart_initial_course(FIRST_NODE, &mut pose, &mut animation, &mut random_state)
                    .unwrap();

                assert_eq!(
                    callback_position(&pose),
                    vector.position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callback_motion(&pose, &animation),
                    vector.motion_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    AlienRingCallback::InitialCourse,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].ring_slot,
                    usize::from(vector.ring_after) / ORIGINAL_RING_ENTRY_BYTES,
                    "{}",
                    vector.name
                );
                assert_eq!(random_state, vector.motion_after[5], "{}", vector.name);
                assert_eq!(animation.entries[slot].pitch_step, entry_before.pitch_step);
                assert_eq!(animation.entries[slot].pan_step, entry_before.pan_step);
                assert_eq!(animation.entries[slot].radial_offset, RESTART_RADIAL_OFFSET);
                assert_eq!(animation.entries[slot].command_flags, u16::MIN);
                assert_eq!(vector.resume_countdown_after, u16::MIN);
                assert_eq!(vector.resume_state_after, u16::MIN);
            }
        }
    }

    #[test]
    fn resume_clear_setup_matches_every_original_overlay_vector() {
        for fixture in callback_fixtures("resume") {
            let vectors: Vec<CallbackVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.kind, "resume");
                let (mut pose, mut animation) = callback_state(vector.ring_before);
                let slot = animation.nodes[FIRST_NODE].ring_slot;

                begin_resume_clear(
                    species(&vector.module),
                    FIRST_NODE,
                    &mut pose,
                    &mut animation,
                )
                .unwrap();

                assert_eq!(
                    callback_position(&pose),
                    vector.position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callback_motion(&pose, &animation),
                    vector.motion_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].callback,
                    AlienRingCallback::ClearHistory,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].ring_slot,
                    usize::from(vector.ring_after) / ORIGINAL_RING_ENTRY_BYTES,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.entries[slot],
                    AlienRingEntry {
                        command_flags: RESUME_COMMAND_FLAGS,
                        ..AlienRingEntry::default()
                    },
                    "{}",
                    vector.name
                );
                assert_eq!(vector.resume_countdown_after, u16::MIN);
                assert_eq!(vector.resume_state_after, u16::MIN);
            }
        }
    }

    #[test]
    fn resume_capture_matches_every_original_overlay_vector() {
        for fixture in callback_fixtures("capture") {
            let vectors: Vec<CallbackVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.kind, "capture");
                let (mut pose, animation) = callback_state(vector.ring_before);
                let animation_before = animation.clone();
                let mut resume = AlienRingResumeState {
                    countdown: u16::MAX,
                    selected_node: None,
                };

                capture_resume_state(species(&vector.module), FIRST_NODE, &mut pose, &mut resume)
                    .unwrap();

                assert_eq!(
                    callback_position(&pose),
                    vector.position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callback_motion(&pose, &animation),
                    vector.motion_after,
                    "{}",
                    vector.name
                );
                assert_eq!(animation, animation_before, "{}", vector.name);
                assert_eq!(
                    resume.countdown, vector.resume_countdown_after,
                    "{}",
                    vector.name
                );
                assert_eq!(resume.selected_node, Some(FIRST_NODE), "{}", vector.name);
                assert_ne!(vector.resume_state_after, u16::MIN);
                assert_eq!(
                    animation.nodes[FIRST_NODE].ring_slot,
                    usize::from(vector.ring_after) / ORIGINAL_RING_ENTRY_BYTES,
                    "{}",
                    vector.name
                );
            }
        }
    }

    #[test]
    fn history_clear_matches_every_original_overlay_vector() {
        for fixture in callback_fixtures("ring_zero") {
            let vectors: Vec<CallbackVector> = serde_json::from_str(fixture).unwrap();
            for vector in vectors {
                assert_eq!(vector.kind, "ring_zero");
                let (pose, mut animation) = callback_state(vector.ring_before);
                let pose_before = pose.clone();
                animation.timer = if vector.name == "timer_blocks" {
                    SINGLE_NODE_COUNT as u16
                } else {
                    u16::MIN
                };
                let animation_before = animation.clone();

                let result = clear_next_ring_entry(FIRST_NODE, &mut animation).unwrap();
                let expected_slot = usize::from(vector.ring_after) / ORIGINAL_RING_ENTRY_BYTES;
                if vector.name == "timer_blocks" {
                    assert_eq!(result, AlienRingClearUpdate::Waiting, "{}", vector.name);
                    assert_eq!(animation, animation_before, "{}", vector.name);
                } else {
                    assert_eq!(
                        result,
                        AlienRingClearUpdate::Cleared {
                            slot: expected_slot
                        },
                        "{}",
                        vector.name
                    );
                    assert_eq!(
                        animation.entries[expected_slot],
                        AlienRingEntry::default(),
                        "{}",
                        vector.name
                    );
                }
                assert_eq!(pose, pose_before, "{}", vector.name);
                assert_eq!(
                    callback_position(&pose),
                    vector.position_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    callback_motion(&pose, &animation),
                    vector.motion_after,
                    "{}",
                    vector.name
                );
                assert_eq!(
                    animation.nodes[FIRST_NODE].ring_slot, expected_slot,
                    "{}",
                    vector.name
                );
                assert_eq!(vector.resume_countdown_after, u16::MIN);
                assert_eq!(vector.resume_state_after, u16::MIN);
            }
        }
    }

    #[test]
    fn zero_count_address_walk_is_rejected_by_the_flat_model() {
        for fixture in fixtures() {
            let vectors: Vec<RingVector> = serde_json::from_str(fixture).unwrap();
            let vector = vectors.last().unwrap();
            assert_eq!(vector.name, "zero_count");
            assert_eq!(vector.effective_callbacks, Some(65_536));
            let mut pose = pose(usize::MIN);
            let mut animation = AlienRingAnimationState::new(usize::MIN);
            animation.lifecycle = AlienRingLifecycle::TimerSuspended;
            animation.timer = vector.timer_before.unwrap();
            let mut callbacks = CallbackRecorder::default();
            assert_eq!(
                update_or_initialize_ring(
                    species(&vector.module),
                    &mut pose,
                    &mut animation,
                    &mut callbacks,
                ),
                Err(AlienRingError::EmptyNodeList)
            );
            assert!(callbacks.calls.is_empty());
            assert_eq!(animation.timer, vector.timer_before.unwrap());
        }
    }

    #[test]
    fn invalid_flat_indices_and_shape_changes_are_rejected() {
        let mut pose = pose(1);
        let mut animation = AlienRingAnimationState::new(1);
        animation.next_ring_slot = RING_ENTRY_COUNT;
        let mut callbacks = CallbackRecorder::default();
        assert_eq!(
            update_or_initialize_ring(
                AlienSpecies::Amer,
                &mut pose,
                &mut animation,
                &mut callbacks,
            ),
            Err(AlienRingError::InvalidNextRingSlot {
                slot: RING_ENTRY_COUNT,
            })
        );

        animation.next_ring_slot = usize::MIN;
        animation.nodes.clear();
        assert_eq!(
            update_or_initialize_ring(
                AlienSpecies::Amer,
                &mut pose,
                &mut animation,
                &mut callbacks,
            ),
            Err(AlienRingError::NodeStateCountMismatch {
                pose: 1,
                animation: 0,
            })
        );
    }
}
