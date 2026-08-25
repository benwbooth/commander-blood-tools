//! Initialization and typed callback dispatch for alien slot-2 animation.

use std::fmt;

use super::{AlienModelPose, AlienSpecies};

const PRIMARY_NODE: usize = 0;
const X_AXIS: usize = 0;
const Y_AXIS: usize = 1;
const Z_AXIS: usize = 2;
const INITIAL_DURATION: i16 = 50;
const INITIAL_AMER_SAMPLE_PHASE: u16 = 20;
const ANGLE_MASK: u16 = 0x0ffc;
const CROOLIS_SEED_STEP: u16 = 250;
const SCRUT_SEED_STEP: u16 = 300;
const RANDOM_ROTATION: u32 = 7;
const RANDOM_BORROW_SHIFT: u32 = 6;
const RANDOM_BORROW_MASK: u16 = 1;
const RESET_SIGNED_VALUE: i16 = 0;
const RESET_ANGLE: u16 = 0;

/// Callback stage selected for one slot-2 animation model.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AlienSlot2Callback {
    /// Advance the species-specific primary animation callback.
    Update,
}

/// Callback-owned state parallel to one animated model node.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct AlienSlot2NodeState {
    /// Signed velocity or source position used by the callback family.
    pub velocity: i16,
    /// Cyclic sample phase used by animation feedback.
    pub sample_phase: u16,
    /// SCRUT depth target retained independently from node ownership.
    pub depth_target: i16,
}

/// Persistent state for one model using the slot-2/4 behavior method.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AlienSlot2AnimationState {
    /// Whether one-time model initialization has completed.
    pub initialized: bool,
    /// Callback selected for the next model update.
    pub callback: Option<AlienSlot2Callback>,
    /// Species-specific animation duration.
    pub duration: i16,
    /// Model-level signed motion accumulator.
    pub motion_accumulator: i16,
    /// Sign-extended species seed captured during initialization.
    pub signed_seed: i32,
    /// Second deterministic random value owned by this model.
    pub random_value: u16,
    /// Callback state parallel to the model pose nodes.
    pub nodes: Vec<AlienSlot2NodeState>,
}

impl AlienSlot2AnimationState {
    /// Allocate flat callback state for a typed model hierarchy.
    pub fn new(node_count: usize) -> Self {
        Self {
            initialized: false,
            callback: None,
            duration: i16::default(),
            motion_accumulator: i16::default(),
            signed_seed: i32::default(),
            random_value: u16::default(),
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
        animation.motion_accumulator = RESET_SIGNED_VALUE;
        animation.random_value = first_random;
        pose.nodes[PRIMARY_NODE].angles[Y_AXIS] = first_random & ANGLE_MASK;
        animation.nodes[PRIMARY_NODE].sample_phase = INITIAL_AMER_SAMPLE_PHASE;
        return Ok(AlienSlot2Update::Initialized);
    }

    animation.duration = INITIAL_DURATION;
    if species == AlienSpecies::Croolis {
        animation.motion_accumulator = RESET_SIGNED_VALUE;
    }
    animation.signed_seed = i32::from(scene.species_seed as i16);
    scene.species_seed = scene.species_seed.wrapping_add(seed_step(species));
    animation.random_value = transform_random(first_random);
    let primary = &mut pose.nodes[PRIMARY_NODE];
    primary.angles[Y_AXIS] = animation.random_value & ANGLE_MASK;
    primary.angles[Z_AXIS] = RESET_ANGLE;
    primary.radial_offset = RESET_SIGNED_VALUE;
    animation.nodes[PRIMARY_NODE].velocity = RESET_SIGNED_VALUE;
    animation.nodes[PRIMARY_NODE].sample_phase = RESET_ANGLE;
    if species == AlienSpecies::Scrut {
        animation.nodes[PRIMARY_NODE].depth_target = RESET_SIGNED_VALUE;
    }

    for (node, callback_state) in pose.nodes[1..].iter().zip(&mut animation.nodes[1..]) {
        callback_state.velocity = match species {
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
    if species != AlienSpecies::Amer && pose.nodes.len() < 2 {
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

fn seed_step(species: AlienSpecies) -> u16 {
    match species {
        AlienSpecies::Amer => u16::MIN,
        AlienSpecies::Croolis => CROOLIS_SEED_STEP,
        AlienSpecies::Scrut => SCRUT_SEED_STEP,
    }
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
        sample_phase_after: u16,
        depth_target_after: u16,
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
                        velocity: expected.velocity_after as i16,
                        sample_phase: expected.sample_phase_after,
                        depth_target: expected.depth_target_after as i16,
                    };
                }
                pose.nodes[PRIMARY_NODE].angles[Y_AXIS] = TOUCHED_FIELD_SENTINEL;
                animation.nodes[PRIMARY_NODE].sample_phase = TOUCHED_FIELD_SENTINEL;
                if species != AlienSpecies::Amer {
                    pose.nodes[PRIMARY_NODE].angles[Z_AXIS] = TOUCHED_FIELD_SENTINEL;
                    pose.nodes[PRIMARY_NODE].radial_offset = TOUCHED_FIELD_SENTINEL as i16;
                    animation.nodes[PRIMARY_NODE].velocity = TOUCHED_FIELD_SENTINEL as i16;
                    if species == AlienSpecies::Scrut {
                        animation.nodes[PRIMARY_NODE].depth_target = TOUCHED_FIELD_SENTINEL as i16;
                    }
                    for node in &mut animation.nodes[1..] {
                        node.velocity = TOUCHED_FIELD_SENTINEL as i16;
                        if species == AlienSpecies::Scrut {
                            node.depth_target = TOUCHED_FIELD_SENTINEL as i16;
                        }
                    }
                }
                animation.duration = TOUCHED_FIELD_SENTINEL as i16;
                animation.motion_accumulator = TOUCHED_FIELD_SENTINEL as i16;
                animation.signed_seed = i32::from(TOUCHED_FIELD_SENTINEL);
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
                    assert_eq!(animation.duration as u16, duration);
                }
                if let Some(accumulator) = vector.motion_accumulator_after {
                    assert_eq!(animation.motion_accumulator as u16, accumulator);
                }
                if let Some(seed) = vector.signed_seed_after {
                    assert_eq!(animation.signed_seed, seed);
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
                        animation.nodes[node_index].velocity as u16,
                        expected.velocity_after
                    );
                    assert_eq!(
                        animation.nodes[node_index].sample_phase,
                        expected.sample_phase_after
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
            let mut pose = pose(
                &[Slot2NodeVector {
                    local_x_before: u16::MIN,
                    local_z_before: u16::MIN,
                    pan_after: u16::MIN,
                    roll_after: u16::MIN,
                    radial_after: u16::MIN,
                    velocity_after: u16::MIN,
                    sample_phase_after: u16::MIN,
                    depth_target_after: u16::MIN,
                }; 2],
            );
            let mut animation = AlienSlot2AnimationState::new(2);
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

        let node = Slot2NodeVector {
            local_x_before: u16::MIN,
            local_z_before: u16::MIN,
            pan_after: u16::MIN,
            roll_after: u16::MIN,
            radial_after: u16::MIN,
            velocity_after: u16::MIN,
            sample_phase_after: u16::MIN,
            depth_target_after: u16::MIN,
        };
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
}
