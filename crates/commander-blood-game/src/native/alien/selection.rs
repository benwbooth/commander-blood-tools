//! Bounds and camera-selection transitions shared by alien wave callbacks.

use std::fmt;

use commander_blood_formats::alien::AXIS_COUNT;

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

    let advances = match species {
        AlienSpecies::Amer | AlienSpecies::Scrut => AMER_AND_SCRUT_PULSE_ADVANCE,
        AlienSpecies::Croolis => CROOLIS_PULSE_ADVANCE,
    };
    for (pulse, advance) in scene.palette_pulses.iter_mut().zip(advances) {
        *pulse = pulse.wrapping_add(advance);
    }
    animation.nodes[node_index].callback = AlienRingCallback::Wave;
    scene.callback_countdown = ACTIVE_CALLBACK_COUNTDOWN;
    Ok(AlienSelectionUpdate::WaveStarted)
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
    if node_index >= pose.nodes.len() {
        return Err(AlienSelectionError::InvalidNodeIndex {
            node_index,
            node_count: pose.nodes.len(),
        });
    }
    Ok(())
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

#[cfg(test)]
mod tests {
    use commander_blood_formats::alien::{AlienFaceData, AlienNodeParent, AlienTransformData};
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
    const UNCHANGED_PULSE: i32 = 0x1357_9BDF;
    const FIXED_FRACTION_SAMPLE: i32 = 0x5678;

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

    fn fixtures() -> [&'static str; AXIS_COUNT] {
        [
            include_str!("../../../../../re/tools/oracle_vectors/xdb_amer_func_0bea_natural.json"),
            include_str!(
                "../../../../../re/tools/oracle_vectors/xdb_croolis_func_0c3e_natural.json"
            ),
            include_str!("../../../../../re/tools/oracle_vectors/xdb_scrut_func_0c32_natural.json"),
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

    fn pose(vector: &SelectionVector) -> AlienModelPose {
        AlienModelPose {
            root: AlienTransformData::default(),
            nodes: vec![AlienNodePose {
                parent: AlienNodeParent::SceneCamera,
                scene_parent: None,
                first_vertex: usize::MIN,
                vertex_count: SINGLE_NODE_COUNT,
                transform: AlienTransformData {
                    translation: vector
                        .translation_integer_words
                        .map(fixed_with_integer_word),
                    ..AlienTransformData::default()
                },
                local_position: vector.position_before.map(|value| value as i32),
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
                let mut pose = pose(&vector);
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
}
